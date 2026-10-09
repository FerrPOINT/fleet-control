"""Read-only verification of Fleet's utility hashes against canonical Base Git blobs."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess


BASE_REVISION = "9b53de7b23593949a9e6c05bd5a4f94b930e50a0"
NAMES = ("runtime_boundary.py", "runtime_bootstrap.py", "runtime_control.py", "runtime_replacement.py")
SOURCE = Path(__file__).resolve().parents[1] / "backend/infra/src/runtime/container_lifecycle.rs"


def utility_hashes():
    blocks = re.findall(r'const CONTROL_SHA256: \[&str; 4\] = \[(.*?)\];',
                        SOURCE.read_text(encoding="utf-8"), re.S)
    if len(blocks) != 1:
        raise ValueError("expected one sealed utility hash array")
    hashes = re.findall(r'"([a-f0-9]{64})"', blocks[0])
    if len(hashes) != len(NAMES):
        raise ValueError("expected exactly four utility hashes")
    return dict(zip(NAMES, hashes))


def verify(base_checkout, revision=BASE_REVISION):
    if not re.fullmatch(r"[a-f0-9]{40}", revision):
        raise ValueError("an exact Git object SHA is required")
    expected = utility_hashes()
    for name in NAMES:
        blob = subprocess.run(
            ["git", "-c", f"safe.directory={Path(base_checkout).resolve().as_posix()}", "show", f"{revision}:scripts/{name}"], cwd=base_checkout,
            capture_output=True, check=True, timeout=10,
        ).stdout
        if hashlib.sha256(blob).hexdigest() != expected[name]:
            raise ValueError(f"{name}: canonical Git blob does not match the compiled hash")
        blob.decode("utf-8")
    return {"revision": revision, "sources": expected}


def verify_contract_checkout(base_checkout, revision=BASE_REVISION):
    """Fake tests use Python's newline-equivalent text; native bytes stay strict."""
    result = verify(base_checkout, revision)
    for name, expected in result["sources"].items():
        source = (Path(base_checkout) / "scripts" / name).read_bytes()
        if hashlib.sha256(source.replace(b"\r\n", b"\n")).hexdigest() != expected:
            raise ValueError(f"{name}: fake-contract checkout differs from canonical Base")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("base_checkout", type=Path)
    parser.add_argument("--revision", default=BASE_REVISION)
    args = parser.parse_args()
    try:
        result = verify(args.base_checkout, args.revision)
    except (OSError, ValueError, subprocess.SubprocessError):
        parser.exit(1, "Sealed Base utility Git blob verification failed\n")
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
