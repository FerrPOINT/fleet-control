"""Check every tracked source byte before the managed gateway/model test begins."""
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
from concurrent.futures import ThreadPoolExecutor


def verify(root, hashes):
    if not isinstance(hashes, dict) or not hashes:
        raise RuntimeError('Native source inventory is missing')
    canonical = root.resolve()
    def entry(item):
        name, digest = item
        path = PurePosixPath(name)
        if path.is_absolute() or '..' in path.parts or '\\' in name or not name or ':' in name:
            raise RuntimeError('Unsafe native source path')
        if not isinstance(digest, str) or not re.fullmatch('[a-f0-9]{64}', digest):
            raise RuntimeError('Invalid source digest')
        file = root / path
        if (not file.resolve().is_relative_to(canonical) or not file.is_file()
                or file.is_symlink() or hashlib.sha256(file.read_bytes()).hexdigest() != digest):
            raise RuntimeError('Native source differs from immutable archive: ' + name)
    with ThreadPoolExecutor(max_workers=8) as pool:
        for index, _ in enumerate(pool.map(entry, hashes.items()), 1):
            if index % 2000 == 0:
                print('Verified native source files: ' + str(index), flush=True)


if __name__ == '__main__':
    hashes = json.loads(Path('/qa/source-hashes.json').read_text())
    verify(Path('/opt/hermes'), hashes)
    print('Exact pinned native tracked source verified: ' + str(len(hashes)) + ' files', flush=True)
