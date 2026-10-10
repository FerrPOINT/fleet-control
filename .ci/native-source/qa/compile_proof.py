"""Qualify actual Cargo output before native execution; import is pure."""
import argparse
import json
from pathlib import Path

from packet import sha, write_json


NAME = "fleet-native-acceptance"


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
    args = parser.parse_args()
    try:
        write_json(args.output,qualify(args.artifacts,args.root,args.executable,args.copied))
    except Exception as error:
        print("Compile qualification withheld: " + type(error).__name__)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
