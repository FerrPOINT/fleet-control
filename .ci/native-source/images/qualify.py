"""Read-only image qualification. Invoked only in owned, offline Compose services."""
import argparse
import hashlib
import importlib.metadata
import json
from pathlib import Path
import subprocess
import sys


def checked(args):
    r = subprocess.run(args, capture_output=True, text=True, timeout=120)
    if r.returncode:
        raise ValueError("Qualification command failed; no installation/retry")
    return r.stdout.strip()


def file_sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def qualify(kind):
    proof = dict(kind=kind, state="qualified", native_executed=False)
    if kind in ("rust", "controller"):
        rust = checked(["rustc", "--version"])
        cargo = checked(["cargo", "--version"])
        if not rust.startswith("rustc 1.88.0 ") or not cargo.startswith("cargo 1.88.0 "):
            raise ValueError("Exact Rust/Cargo1.88 required")
        proof.update(rust=rust, cargo=cargo, python=sys.version.split()[0], git=checked(["git", "--version"]))
        checked(["rustfmt", "--version"])
        checked(["cargo", "clippy", "--version"])
        if kind == "controller":
            proof.update(docker=checked(["docker", "--version"]), compose=checked(["docker", "compose", "version", "--short"]))
    else:
        root = Path("/opt/hermes")
        contract = Path("/qa/parent/evidence")
        for filename, evidence in (("pyproject.toml", "hermes-pyproject.toml"), ("uv.lock", "hermes-uv.lock")):
            if file_sha(root / filename) != file_sha(contract / evidence):
                raise ValueError("Inherited source/lock drift")
        uv = checked(["uv", "--version"])
        if not uv.startswith("uv 0.11.6"):
            raise ValueError("Pinned uv0.11.6 required")
        # uv owns dependency resolution/markers/extras. Check only; never sync or repair.
        checked(["uv", "sync", "--check", "--frozen", "--offline", "--no-dev", "--extra", "web",
                 "--extra", "messaging", "--python", "/usr/bin/python3", "--no-python-downloads"])
        packages = sorted((d.metadata["Name"], d.version) for d in importlib.metadata.distributions())
        if not packages or any(not n or not v for n, v in packages):
            raise ValueError("Installed package inventory incomplete")
        proof.update(uv=uv, python=sys.version.split()[0], packages=packages,
                     uv_lock_sha256=file_sha(root / "uv.lock"), installed_closure_check="uv_frozen_offline_check")
        if kind == "hermes":
            entries = json.loads(Path("/qa/parent/hermes-git-inventory.json").read_bytes())
            for name, e in entries.items():
                path = root / name
                if path.is_symlink() or not path.is_file() or path.stat().st_size != e["size"]:
                    raise ValueError("Hermes source filesystem drift")
                h = hashlib.sha1(b"blob " + str(e["size"]).encode() + b"\0")
                with path.open("rb") as stream:
                    for block in iter(lambda: stream.read(1024 * 1024), b""):
                        h.update(block)
                if h.hexdigest() != e["git_blob"]:
                    raise ValueError("Hermes Git blob drift")
            proof["source_files_verified"] = len(entries)
    return proof


def main():
    p = argparse.ArgumentParser(__doc__)
    p.add_argument("kind", choices=("rust", "controller", "dependency", "hermes"))
    args = p.parse_args()
    try:
        print(json.dumps(qualify(args.kind), sort_keys=True))
        return 0
    except Exception as e:
        print(json.dumps(dict(state="withheld", failure_class=type(e).__name__, native_executed=False)))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
