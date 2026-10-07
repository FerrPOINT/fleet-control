"""Actual native lease/readback checks; never print commands, receipts or runtime output."""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import uuid
from datetime import datetime, timedelta, timezone


def private(name):
    path = Path('/controller') / name
    stat = path.lstat()
    assert path.is_file() and not path.is_symlink() and stat.st_uid == 999
    assert stat.st_mode & 0o777 == 0o600 and stat.st_size < 1024 * 1024
    return json.loads(path.read_bytes())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--epoch', type=int, choices=(1, 2), required=True)
    parser.add_argument('--expired', action='store_true')
    args = parser.parse_args()
    proof = json.loads(Path('/qa/controller-proof.json').read_bytes())
    root = Path(proof['control']['base_root'])
    for name, expected in zip(('runtime_boundary.py', 'runtime_bootstrap.py', 'runtime_control.py'),
                              proof['control']['source_sha256'], strict=True):
        assert hashlib.sha256((root / 'scripts' / name).read_bytes()).hexdigest() == expected
    sys.path.insert(0, str(root))
    from scripts.runtime_boundary import BoundaryError, canonical, digest
    from scripts.runtime_control import execute
    context = private('custody-context.json')
    owners = private(f'custody-epoch{args.epoch}.json')
    assert len(owners) == len(context['launches']) == 2

    def call(request, action, command):
        return execute(dict(request, action=action, recovery=command))['result']

    def denied(request, action, command):
        try:
            call(request, action, command)
        except BoundaryError:
            return
        raise AssertionError('Native custody hold was bypassed')

    for launch, owner in zip(context['launches'], owners, strict=True):
        original, current = owner['initial'], owner['current']
        assert current['epoch'] == args.epoch and current['lease_version'] >= 4
        assert current['request'] == original['request'] and original['lease_version'] == 1
        binding = launch['container']
        assert binding['source_sha256'] == proof['control']['source_sha256']
        journal = Path(binding['journal']).parent / (launch['id'] + '.controller-epochs.sqlite')
        request = {'protocol_version': 3, 'context': binding['context'], 'policy': binding['policy'],
                   'compose': binding['compose'], 'journal': binding['journal'],
                   'registration': binding['registration'], 'mount_mapping': binding['mount_mapping'],
                   'mapping_file': binding['mapping_file'], 'recovery_journal': str(journal)}
        before = journal.read_bytes()
        receipt = call(request, 'read_controller_recovery', original)
        assert digest(receipt) == owner['receipt_sha256'] and receipt['recovery'] == original
        ack = call(request, 'heartbeat_controller', current)
        assert ack == {'state': 'controller_heartbeat', 'recovery_id': current['request']['id'],
                       'lease_version': current['lease_version'], 'lease_expires_at': current['lease_expires_at']}
        if args.expired:
            denied(request, 'observe', current)
            newer = json.loads(canonical(current))
            newer['lease_version'] += 1
            newer['lease_expires_at'] = (datetime.now(timezone.utc) + timedelta(seconds=30)).isoformat()
            denied(request, 'heartbeat_controller', newer)
        else:
            observed = call(request, 'observe', current)
            assert observed == receipt['witness']['receipt'] and observed['observation'] == 'running'
            competitor = json.loads(canonical(current))
            competitor['request'].update(id=str(uuid.uuid4()), controller_id=str(uuid.uuid4()),
                predecessor_id=original['request']['id'])
            competitor.update(epoch=args.epoch + 1, lease_version=1)
            denied(request, 'recover_controller', competitor)
            denied(request, 'observe', competitor)
        assert journal.read_bytes() == before, 'Read-only proof or denied contender changed native journal'
    suffix = 'expired' if args.expired else f'epoch{args.epoch}'
    result = {'state': 'passed', 'agents': 2, 'historical_receipt_unchanged': True,
              'same_version_heartbeat_replay_read_only': True, 'native_journals_unchanged': True,
              'native_expiry_checked': args.expired, 'native_live_observation': not args.expired,
              'sdlc_acceptance': False, 'resumed_execution': False, 'raw_receipts_persisted': False}
    Path(f'/evidence/custody-native-{suffix}.json').write_text(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()
