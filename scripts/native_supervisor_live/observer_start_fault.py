#!/opt/hermes/.venv/bin/python
"""Owned native QA launcher: one requested startup failure, never production."""
import os
from pathlib import Path
import re
import stat
import sys

AGENTS_ROOT = Path('/tmp/fleet-native-supervisor/observer-agents')
LAUNCHER = '/opt/fleet-hermes/bin/hermes'
OPT_IN = Path('/qa/observer-start-fault-opt-in')
OPT_IN_CONTENT = b'fleet-native-observer-start-fault/v1\n'


def main():
    if OPT_IN.is_symlink():
        raise RuntimeError('Observer startup fixture requires its readonly QA opt-in')
    try:
        with OPT_IN.open('rb') as source:
            opt_in = source.read(len(OPT_IN_CONTENT) + 1)
    except OSError:
        raise RuntimeError('Observer startup fixture requires its readonly QA opt-in') from None
    if opt_in != OPT_IN_CONTENT:
        raise RuntimeError('Observer startup fixture requires its readonly QA opt-in')
    home = Path(os.environ.get('HERMES_HOME', ''))
    if (not home.is_absolute() or home.parent.parent != AGENTS_ROOT
            or home.name != 'config' or not re.fullmatch(r'agent[1-9][0-9]*', home.parent.name)
            or any(path.is_symlink() or not path.is_dir()
                   for path in (AGENTS_ROOT, home.parent, home))):
        raise RuntimeError('Observer startup fixture requires its owned agent config')
    marker = home / '.qa-fail-next-launch'
    try:
        metadata = marker.lstat()
    except FileNotFoundError:
        os.execv(LAUNCHER, [LAUNCHER, *sys.argv[1:]])
        return
    if not stat.S_ISREG(metadata.st_mode) or metadata.st_nlink != 1:
        raise RuntimeError('Observer startup marker must be an owned regular file')
    descriptor = os.open(marker, os.O_RDONLY | getattr(os, 'O_NOFOLLOW', 0))
    try:
        opened = os.fstat(descriptor)
        if ((opened.st_dev, opened.st_ino) != (metadata.st_dev, metadata.st_ino)
                or os.read(descriptor, 64) != b'owned startup fault'):
            raise RuntimeError('Observer startup marker identity or content changed')
    finally:
        os.close(descriptor)
    marker.unlink()
    raise SystemExit(71)


if __name__ == '__main__':
    main()
