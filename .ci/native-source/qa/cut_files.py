"""Cut-owned Linux publication/probe seam. No native permits or producer evidence."""
import json
import os
from pathlib import Path
import stat
import sys
import time
import uuid

from cut_contract import arm, closed, decode, sha, uid, validate_gate

ROOT = Path("/controller")
EVIDENCE = Path("/evidence")
WITHHOLD = 55


def atomic_json(path, value):
    # A permanent exclusive attempt latch also forbids republishing after deletion/crash.
    raw = (json.dumps(value, sort_keys=True, separators=(',', ':')) + '\n').encode()
    directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    temporary = "." + path.name + "." + uuid.uuid4().hex + ".tmp"
    created = False
    try:
        claim = os.open("." + path.name + ".once", os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                        0o600, dir_fd=directory)
        try:
            os.fsync(claim)
        finally:
            os.close(claim)
        os.fsync(directory)
        fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW,
                     0o600, dir_fd=directory)
        created = True
        try:
            info = os.fstat(fd)
            if (not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) != 0o600
                    or info.st_uid != os.geteuid() or info.st_nlink != 1):
                raise ValueError("Private unique publication temporary required")
            with os.fdopen(fd, "wb", closefd=False) as stream:
                stream.write(raw)
                stream.flush()
                os.fsync(fd)
        finally:
            os.close(fd)
        # link is atomic NO-REPLACE: readers see all bytes or no final name at all.
        os.link(temporary, path.name, src_dir_fd=directory, dst_dir_fd=directory, follow_symlinks=False)
        os.unlink(temporary, dir_fd=directory)
        created = False
        os.fsync(directory)
    finally:
        try:
            if created:
                os.unlink(temporary, dir_fd=directory)
                os.fsync(directory)
        finally:
            os.close(directory)


def private(path, limit=8192):
    directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        owner = os.fstat(directory)
        if owner.st_uid != os.geteuid() or stat.S_IMODE(owner.st_mode) != 0o700:
            raise ValueError("Original private controller directory required")
        fd = os.open(path.name, os.O_RDONLY | os.O_NOFOLLOW, dir_fd=directory)
        try:
            info = os.fstat(fd)
            if (not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) != 0o600
                    or info.st_uid != os.geteuid() or info.st_nlink != 1 or info.st_size > limit):
                raise ValueError("Original private cut witness required")
            with os.fdopen(fd, "rb", closefd=False) as stream:
                raw = stream.read(limit + 1)
            after = os.fstat(fd)
            if len(raw) != info.st_size or (info.st_ino, info.st_mtime_ns, info.st_size) != (after.st_ino, after.st_mtime_ns, after.st_size):
                raise ValueError("Cut witness changed during read")
            return decode(raw)
        finally:
            os.close(fd)
    finally:
        os.close(directory)


def process(pid):
    raw = Path(f"/proc/{pid}/stat").read_text()
    fields = raw[raw.rindex(')') + 2:].split()
    if fields[0] in ("Z", "X", "x") or Path(f"/proc/{pid}").stat().st_uid != os.geteuid():
        raise ValueError("Live original owned wrapper required")
    return int(fields[19])  # proc stat field 22; comm may itself contain spaces/parentheses.


def blocking(event):
    start = time.monotonic()
    return dict(call_id=event["call_id"], ack_sha256=event["ack_sha256"], pid=os.getpid(),
                start_ticks=process(os.getpid()), boot_id=Path("/proc/sys/kernel/random/boot_id").read_text().strip(),
                began=start, deadline=start + WITHHOLD)


def probe(call_id, ack_sha256):
    uid(call_id)
    sha(ack_sha256)
    value = private(ROOT / "qa-cut-blocked.json")
    closed(value, ("call_id", "ack_sha256", "pid", "start_ticks", "boot_id", "began", "deadline"))
    once = private(ROOT / "qa-cut-once.json")
    closed(once, ("call_id", "request"))
    for key in ("pid", "start_ticks"):
        if type(value[key]) is not int or value[key] <= 0:
            raise ValueError("Original process identity required")
    if (value["call_id"] != call_id or once["call_id"] != call_id or value["ack_sha256"] != ack_sha256
            or value["boot_id"] != Path("/proc/sys/kernel/random/boot_id").read_text().strip()
            or value["start_ticks"] != process(value["pid"])):
        raise ValueError("Original blocked wrapper epoch/ACK mismatch")
    now = time.monotonic()
    result = dict(witness=value, observed=now, remaining=value["deadline"] - now)
    validate_probe(result, call_id, ack_sha256)
    return result


def validate_probe(value, call_id, ack_sha256):
    import math
    closed(value, ("witness", "observed", "remaining"))
    w = value["witness"]
    closed(w, ("call_id", "ack_sha256", "pid", "start_ticks", "boot_id", "began", "deadline"))
    uid(w["call_id"])
    sha(w["ack_sha256"])
    uid(w["boot_id"])
    if any(type(w[k]) is not int or w[k] <= 0 for k in ("pid", "start_ticks")):
        raise ValueError("Exact wrapper PID/start identity required")
    numbers = (w["began"], w["deadline"], value["observed"], value["remaining"])
    if any(type(n) not in (int, float) or not math.isfinite(n) for n in numbers):
        raise ValueError("Finite monotonic cut budget required")
    if (w["call_id"] != call_id or w["ack_sha256"] != ack_sha256 or w["began"] < 0
            or w["deadline"] - w["began"] != WITHHOLD or not w["began"] <= value["observed"] < w["deadline"]
            or value["remaining"] != w["deadline"] - value["observed"]):
        raise ValueError("Live original 55s withholding budget required")


def publish_ready(raw):
    value = decode(raw)
    closed(value, ("cut", "arm", "peer"))
    armed = arm(private(ROOT / "qa-cut-arm.json"))
    gate = decode((EVIDENCE / "cut-stop-ack.json").read_bytes())
    validate_gate(gate, armed)
    if value["arm"] != armed or value["cut"]["phase"] != "stopping_previous" or value["cut"]["previous_stop"] is not None:
        raise ValueError("Unchanged original cut identity/phase required")
    live = probe(gate["event"]["call_id"], gate["event"]["ack_sha256"])
    if private(ROOT / "qa-cut-once.json")["request"] != gate["event"]["request"]:
        raise ValueError("Original native attempt binding required")
    atomic_json(EVIDENCE / "cut-ready.json", value)
    return live


if __name__ == "__main__":
    try:
        if sys.argv[1:] == ["publish-ready"]:
            raw = sys.stdin.buffer.read(1_048_577)
            if len(raw) > 1_048_576:
                raise ValueError("Bounded ready receipt required")
            publish_ready(raw)
        elif len(sys.argv) == 4 and sys.argv[1] == "probe":
            print(json.dumps(probe(sys.argv[2], sys.argv[3]), sort_keys=True))
        else:
            raise ValueError("Closed cut file operation required")
    except Exception:
        raise SystemExit(2)  # Never print private paths/native payloads.
