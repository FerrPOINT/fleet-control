"""Actual second binary/source proof; never substitutes synthetic parser fixtures."""
import argparse
import json
from pathlib import Path

from packet import sha, write_json

SOURCES = ("cut-live/src/main.rs","cut-live/src/scenario.rs","cut-live/build.rs","cut-live/Cargo.toml",
           "cut-live/Cargo.lock","live/src/main.rs","fleet-control/backend/Cargo.lock",
           "cut_transport.py","cut_contract.py","cut_files.py")


def artifact(records, root, executable):
    finished = [r for r in records if r.get("reason") == "build-finished"]
    items = [r for r in records if r.get("reason") == "compiler-artifact"
             and r.get("target",{}).get("name") == "fleet-native-acceptance"]
    if len(finished) != 1 or finished[0].get("success") is not True or len(items) != 1:
        raise ValueError("One actual cut binary and successful locked build required")
    item = items[0]
    if (item["target"].get("kind") != ["bin"] or item["target"].get("crate_types") != ["bin"]
            or item["target"].get("src_path") != str(root / "cut-live/src/main.rs")
            or item.get("executable") != str(executable) or item.get("profile",{}).get("test") is not False):
        raise ValueError("Exact actual cut binary source/profile required")


def qualify(artifacts, root, executable, copied):
    artifact([json.loads(s) for s in artifacts.read_text().splitlines() if s],root,executable)
    for path in (executable,copied):
        if path.is_symlink() or not path.is_file() or not path.stat().st_mode & 0o111:
            raise ValueError("Actual executable required")
        with path.open("rb") as stream:
            if stream.read(4) != b"\x7fELF":
                raise ValueError("Actual ELF required")
    if sha(executable) != sha(copied):
        raise ValueError("Exact actual binary copy required")
    return dict(state="actual_cut_locked_compile_verified",native_executed=False,
                binary_sha256=sha(copied),artifact_sha256=sha(artifacts),sources={p:sha(root / p) for p in SOURCES})


def verify(packet, manifest):
    value = json.loads((packet / "output/cut-compile-proof.json").read_bytes())
    if (set(value) != {"state","native_executed","binary_sha256","artifact_sha256","sources"}
            or value["state"] != "actual_cut_locked_compile_verified" or value["native_executed"] is not False
            or value["artifact_sha256"] != sha(packet / "output/cut-build-artifacts.jsonl")
            or value["sources"] != {p:manifest["source_files"][p] for p in SOURCES}):
        raise ValueError("Actual sealed cut compile proof required before native")
    from cut_contract import sha as require_hash
    require_hash(value["binary_sha256"])
    return value


if __name__ == "__main__":
    parser = argparse.ArgumentParser(__doc__)
    for name in ("artifacts","root","executable","copied","output"):
        parser.add_argument("--" + name,type=Path,required=True)
    args = parser.parse_args()
    write_json(args.output,qualify(args.artifacts,args.root,args.executable,args.copied))
