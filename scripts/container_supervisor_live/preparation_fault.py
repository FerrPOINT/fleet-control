#!/usr/bin/python3
"""Owned QA only: lose one real preparation ACK, never fabricate native receipts."""
import json
import os
from pathlib import Path
import subprocess
import sys


ROOT = Path('/controller')


def private_identity(path, identity):
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, 'w') as output:
        json.dump(identity, output)
        output.flush()
        os.fsync(output.fileno())


def main():
    body = sys.stdin.buffer.read(4 * 1024 * 1024 + 1)
    if len(body) > 4 * 1024 * 1024:
        return 1
    request = json.loads(body)['request']
    result = subprocess.run(['/usr/bin/python3', *sys.argv[1:]], input=body,
        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=50, check=False)
    if len(result.stdout) > 64 * 1024:
        return 1
    target = (ROOT / 'preparation-fault-agent').read_text().strip()
    if (result.returncode == 0 and request['policy']['resource_id'] == target
            and request['action'] in ('prepare', 'reconcile_preparation')):
        actual = json.loads(result.stdout)['result']
        if actual['state'] != 'prepared':
            return 1
        marker = ROOT / 'preparation-fault-result.json'
        identity = dict(operation_id=request['operation_id'], generation=request['policy']['generation'])
        if request['action'] == 'prepare' and not marker.exists():
            private_identity(marker, identity)
            return 1
        if request['action'] == 'reconcile_preparation':
            if json.loads(marker.read_bytes()) != identity:
                return 1
            observed = ROOT / 'preparation-readback-observed.json'
            if observed.exists():
                if json.loads(observed.read_bytes()) != identity:
                    return 1
            else:
                private_identity(observed, identity)
    sys.stdout.buffer.write(result.stdout)
    sys.stdout.buffer.flush()
    return result.returncode


if __name__ == '__main__':
    raise SystemExit(main())
