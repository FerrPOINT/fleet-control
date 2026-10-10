"""Raw Git packet primitives. No container, network or build operations on import."""
import hashlib
import json
from pathlib import PurePosixPath
import re
import subprocess

FORBIDDEN = {".local", "target", "node_modules", ".venv", ".git", "backups", "__pycache__"}


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_json(path, value):
    temporary = path.with_suffix(".tmp")
    with temporary.open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")
    temporary.replace(path)


def checked(args, cwd=None, timeout=300):
    result = subprocess.run(args, cwd=cwd, capture_output=True, timeout=timeout)
    if result.returncode:
        raise RuntimeError("Command failed; raw arguments/output are not public evidence")
    return result.stdout


def git(root, *args):
    return checked(["git", "--no-replace-objects", "-C", str(root), *args])


def exact_sha(value):
    if not re.fullmatch("[a-f0-9]{40}", value):
        raise ValueError("Exact Git commit required")
    return value


def inventory(root):
    result = {}
    for path in root.rglob("*"):
        if path.is_symlink():
            raise ValueError("Source link is not permitted")
        if path.is_file():
            result[path.relative_to(root).as_posix()] = sha(path)
    return dict(sorted(result.items()))


def member(name, mode):
    path = PurePosixPath(name)
    if (str(path) != name or path.is_absolute() or ".." in path.parts
            or "\\" in name or ":" in name or FORBIDDEN.intersection(path.parts)
            or path.suffix == ".pyc" or mode not in ("100644", "100755")):
        raise ValueError("Unsafe Git blob export member")
    return path


def export(root, revision, target, paths):
    exact_sha(revision)
    entries = git(root, "ls-tree", "-r", "-z", revision, "--", *paths).split(b"\0")
    target.mkdir(parents=True, exist_ok=False)
    process = subprocess.Popen(["git", "--no-replace-objects", "-C", str(root), "cat-file", "--batch"],
                               stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    total = count = 0
    try:
        for entry in filter(None, entries):
            meta, name = entry.split(b"\t", 1)
            mode, kind, oid = meta.decode().split()
            path = member(name.decode("utf-8"), mode)
            if kind != "blob":
                raise ValueError("Non-blob Git source")
            process.stdin.write(oid.encode() + b"\n")
            process.stdin.flush()
            header = process.stdout.readline().decode("ascii").split()
            if len(header) != 3 or header[:2] != [oid, "blob"]:
                raise ValueError("Raw blob header drift")
            size = int(header[2])
            total += size
            if not 0 <= size <= 128 * 1024 ** 2 or total > 512 * 1024 ** 2:
                raise ValueError("Source export budget exceeded")
            identity = hashlib.sha1(f"blob {size}\0".encode())
            destination = target.joinpath(*path.parts)
            destination.parent.mkdir(parents=True, exist_ok=True)
            with destination.open("xb") as stream:
                remaining = size
                while remaining:
                    raw = process.stdout.read(min(remaining, 1024 * 1024))
                    if not raw:
                        raise ValueError("Truncated Git blob")
                    identity.update(raw)
                    stream.write(raw)
                    remaining -= len(raw)
            if identity.hexdigest() != oid or process.stdout.read(1) != b"\n":
                raise ValueError("Raw blob identity mismatch")
            destination.chmod(0o755 if mode == "100755" else 0o644)
            count += 1
        process.stdin.close()
        if process.wait(timeout=30) or not count:
            raise ValueError("Empty or failed raw Git export")
    finally:
        process.stdout.close()
        if not process.stdin.closed:
            process.stdin.close()
        if process.poll() is None:
            process.kill()
            process.wait()
