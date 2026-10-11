"""Qualify actual Cargo output before native execution; import is pure."""
import argparse
import json
import os
import re
import stat
from pathlib import Path

from packet import sha, write_json


NAME = "fleet-native-acceptance"
BUILD_STEPS = frozenset("toolchain docker_cli compose_cli capacity empty_volumes scratch source_hash copy_source cargo executable copy_binary compile_proof final_hash".split())


def bounded(path, limit):
    if path.is_symlink():
        raise ValueError("Diagnostic link refused")
    with os.fdopen(os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)), "rb") as stream:
        if not stat.S_ISREG(os.fstat(stream.fileno()).st_mode):
            raise ValueError("Diagnostic regular file required")
        raw = stream.read(limit + 1)
    if len(raw) > limit:
        raise ValueError("Diagnostic bound exceeded")
    return raw


def failure_record(value):
    try:
        assert type(value) is dict and set(value) == {"step", "exit_code", "diagnostics"}
        assert type(value["step"]) is str and value["step"] in BUILD_STEPS
        assert type(value["exit_code"]) is int and 1 <= value["exit_code"] <= 255
        assert type(value["diagnostics"]) is list and len(value["diagnostics"]) <= 8
        for frame in value["diagnostics"]:
            assert type(frame) is dict and set(frame) in ({"code"}, {"code", "line", "column"})
            assert type(frame["code"]) is str and re.fullmatch(r"E[0-9]{4}", frame["code"])
            assert all(type(frame[k]) is int and 1 <= frame[k] <= 1000000 for k in frame if k != "code")
        return value
    except (AssertionError, KeyError, TypeError):
        return None


def read_failure(path):
    try:
        return failure_record(json.loads(bounded(path, 2048)))
    except (OSError, ValueError, RecursionError):
        return None


def failure(step, exit_code, artifacts):
    frames = []
    try:
        lines = bounded(artifacts, 8 * 1024 ** 2).splitlines()
        if len(lines) > 16384:
            raise ValueError("Diagnostic line bound exceeded")
        for line in lines:
            record = json.loads(line)
            if record.get("reason") != "compiler-message":
                continue
            message = record["message"]
            code = (message.get("code") or {}).get("code")
            if message.get("level") != "error" or type(code) is not str or not re.fullmatch(r"E[0-9]{4}", code):
                continue
            frame = dict(code=code)
            for span in message.get("spans", []):
                if span.get("is_primary") is True and span.get("file_name") == "/scratch/src/live/src/main.rs":
                    if all(type(span.get(k)) is int and 1 <= span[k] <= 1000000 for k in ("line_start", "column_start")):
                        frame.update(line=span["line_start"], column=span["column_start"])
                        break
            if frame not in frames and len(frames) < 8:
                frames.append(frame)
    except (OSError, ValueError, TypeError, KeyError, AttributeError, RecursionError):
        frames = []
    return failure_record(dict(step=step, exit_code=exit_code, diagnostics=frames))


def artifact(records, root, executable):
    finished = [r for r in records if r.get("reason") == "build-finished"]
    if len(finished) != 1 or finished[0].get("success") is not True:
        raise ValueError("One successful Cargo build completion is mandatory")
    matches = [r for r in records if r.get("reason") == "compiler-artifact"
               and r.get("target",{}).get("name") == NAME]
    if len(matches) != 1:
        raise ValueError("One exact compiled QA artifact is mandatory")
    item = matches[0]
    target = item["target"]
    if (target.get("kind") != ["bin"] or target.get("crate_types") != ["bin"]
            or target.get("src_path") != str(root / "live/src/main.rs")
            or item.get("executable") != str(executable) or item.get("profile",{}).get("test") is not False):
        raise ValueError("Cargo source, executable or profile drift")
    return item


def qualify(artifacts, root, executable, copied):
    records = [json.loads(line) for line in artifacts.read_text(encoding="utf-8").splitlines() if line]
    artifact(records,root,executable)
    for path in (executable,copied):
        if path.is_symlink() or not path.is_file() or not path.stat().st_mode & 0o111:
            raise ValueError("Actual executable binary is mandatory")
        with path.open("rb") as stream:
            if stream.read(4) != b"\x7fELF":
                raise ValueError("Native Linux ELF binary is mandatory")
    if sha(executable) != sha(copied):
        raise ValueError("Compiled binary copy drift")
    return dict(state="actual_locked_compile_verified",binary_sha256=sha(copied),
        source_main_sha256=sha(root / "live/src/main.rs"),
        qa_lock_sha256=sha(root / "live/Cargo.lock"),
        product_lock_sha256=sha(root / "fleet-control/backend/Cargo.lock"),
        cargo_artifacts_sha256=sha(artifacts),
        compiler_artifacts=sum(r.get("reason") == "compiler-artifact" for r in records),
        native_executed=False)


def main():
    parser = argparse.ArgumentParser(__doc__)
    for name in ("artifacts","root","executable","copied","output"):
        parser.add_argument("--" + name,type=Path,required=True)
    parser.add_argument("--failure-step", choices=sorted(BUILD_STEPS))
    parser.add_argument("--failure-exit", type=int)
    args = parser.parse_args()
    try:
        if args.failure_step:
            record = failure(args.failure_step, args.failure_exit, args.artifacts)
            if record is None:
                return 1
            write_json(args.output, record)
            return 0
        write_json(args.output,qualify(args.artifacts,args.root,args.executable,args.copied))
    except Exception as error:
        print("Compile qualification withheld: " + type(error).__name__)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
