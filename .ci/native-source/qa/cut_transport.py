#!/usr/bin/env python3
"""QA-only byte-transparent Base transport with one ACK-before-return latch."""
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import time
import uuid

sys.dont_write_bytecode = True
from cut_contract import SOURCE, HASHES, action, arm, closed, cut_stop, decode, digest, projection, response, selected, validate_loader, validate_sources
from cut_files import atomic_json, blocking

ROOT = Path("/controller")
EVIDENCE = Path("/evidence")
LIMIT = 4 * 1024 * 1024 + 65536


def private(path, limit=8192):
    if path.parent != ROOT or ROOT.is_symlink():
        raise ValueError("Exact private QA parent required")
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW)
    try:
        info = os.fstat(fd)
        if (not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) != 0o600
                or info.st_uid != os.geteuid() or info.st_nlink != 1 or info.st_size > limit):
            raise ValueError("Private original evidence required")
        with os.fdopen(fd, "rb", closefd=False) as stream:
            raw = stream.read(limit + 1)
        after = os.fstat(fd)
        if len(raw) > limit or (info.st_size,info.st_mtime_ns,info.st_ino) != (after.st_size,after.st_mtime_ns,after.st_ino):
            raise ValueError("Evidence changed while reading")
        return raw
    finally:
        os.close(fd)


def create(path, value):
    raw = (json.dumps(value,sort_keys=True,separators=(',',':')) + '\n').encode()
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    try:
        with os.fdopen(fd, "wb", closefd=False) as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(fd)
    finally:
        os.close(fd)
    parent = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(parent)
    finally:
        os.close(parent)


def journal_witness(path):
    path = Path(path)
    raw = private(path, 8 * 1024 * 1024)
    info = path.lstat()
    return dict(sha256=hashlib.sha256(raw).hexdigest(),device=info.st_dev,inode=info.st_ino,
                size=len(raw),mode=stat.S_IMODE(info.st_mode),uid=info.st_uid)


def delegate(args, raw):
    # subprocess.run kills and waits ONLY for its own child on a timeout.
    result = subprocess.run([sys.executable,*args], input=raw,stdout=subprocess.PIPE,
                            stderr=subprocess.DEVNULL,timeout=55,check=False)
    if len(result.stdout) > 65536:
        raise ValueError("Bounded original ACK required")
    return result.returncode,result.stdout


def relay(args, raw, armed, call=delegate, persist=create, witness=journal_witness, pause=time.sleep,
          publish=atomic_json, blocked=blocking, clock=time.monotonic):
    payload = decode(raw)
    validate_loader(args,payload,armed)
    audited = selected(payload,armed)
    cutting = cut_stop(payload,armed)
    if audited and action(payload) == "stop":
        request = payload["request"]
        if (not cutting or type(request.get("protocol_version")) is not int or request["protocol_version"] != 2
                or digest(request["registration"]) != armed["registration_sha256"]
                or Path(request["stop_journal"]).parent != ROOT):
            raise ValueError("Only exact armed original mapped stop may reach Base")
    call_id = str(uuid.uuid4())
    request = projection(payload,armed) if audited else None
    if cutting:
        # Durable QA latch, NOT a Base permit. Missing/unknown native ACK never retries.
        persist(ROOT / "qa-cut-once.json",dict(call_id=call_id,request=request))
    if audited:
        if len(list(EVIDENCE.glob("dispatch-*.json"))) >= 128:
            raise ValueError("QA trace budget exhausted")
        persist(EVIDENCE / ("dispatch-" + call_id + ".json"),dict(call_id=call_id,request=request))
    status, output = call(args,raw)
    if audited:
        ack = response(payload,output,status,armed)
        event = dict(call_id=call_id,request=request,ack=ack,ack_sha256=digest(ack),
                     raw_ack_sha256=hashlib.sha256(output).hexdigest())
        persist(EVIDENCE / ("ack-" + call_id + ".json"),event)
        if cutting:
            proof = dict(event=event,journal=witness(payload["request"]["stop_journal"]),
                         stdout_forwarded=False)
            live = blocked(event)
            publish(ROOT / "qa-cut-blocked.json",live)
            publish(EVIDENCE / "cut-stop-ack.json",proof)
            # Host must physically restart the original controller before Rust's unchanged
            # 60s deadline. Never let this call return success/advance a phase on timeout.
            pause(max(0, live["deadline"] - clock()))
            raise ValueError("ACK latch was not interrupted by physical controller restart")
    return status,output


def main():
    try:
        raw = sys.stdin.buffer.read(LIMIT + 1)
        if len(raw) > LIMIT:
            raise ValueError("Bounded original payload required")
        proof = decode(Path("/proof/cut.json").read_bytes())
        closed(proof,("source_commit","loader_sha256","utility_hashes"))
        if proof["source_commit"] != SOURCE or proof["utility_hashes"] != HASHES:
            raise ValueError("Exact protected cut qualification required")
        validate_sources(sys.argv[1:],decode(raw),proof["loader_sha256"])
        try:
            armed = arm(decode(private(ROOT / "qa-cut-arm.json")))
        except FileNotFoundError:
            if os.path.lexists(ROOT / "qa-cut-once.json"):
                raise ValueError("Lost armed QA identity after native attempt")
            status,output = delegate(sys.argv[1:],raw)
            sys.stdout.buffer.write(output)
            sys.stdout.buffer.flush()
            return status
        status,output = relay(sys.argv[1:],raw,armed)
        sys.stdout.buffer.write(output)
        sys.stdout.buffer.flush()
        return status
    except Exception:
        # No raw args, requests, env, private paths or native stderr escape the QA wrapper.
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
