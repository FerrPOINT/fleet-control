"""Read-only original namespace exit proof after the real recovered Fleet stop."""
import hashlib
import json
from pathlib import Path
import sys

from custody_probe import private


def main():
    proof = json.loads(Path('/qa/controller-proof.json').read_bytes())
    root = Path(proof['control']['base_root'])
    for name, expected in zip(('runtime_boundary.py', 'runtime_bootstrap.py', 'runtime_control.py'),
                              proof['control']['source_sha256'], strict=True):
        assert hashlib.sha256((root / 'scripts' / name).read_bytes()).hexdigest() == expected
    sys.path.insert(0, str(root))
    from scripts.runtime_boundary import digest
    from scripts.runtime_control import execute
    context, stops = private('custody-context.json'), private('custody-stops.json')
    assert len(stops) == len(context['launches']) == 2
    assert len({launch['id'] for launch in context['launches']}) == 2
    for launch, saved in zip(context['launches'], stops, strict=True):
        binding, intent, command = launch['container'], saved['intent'], saved['command']
        assert binding['source_sha256'] == proof['control']['source_sha256']
        assert intent['launch_id'] == launch['id'] == intent['operation_id']
        assert intent['launch_sha256'] == digest(launch)
        assert command['epoch'] == 3 and command['request']['launch_id'] == launch['id']
        outcome = saved['outcome']
        assert outcome['kind'] in ('stop', 'observe')
        assert outcome['receipt']['observation'] == 'namespace_exited'
        snapshot_sha256 = (outcome['receipt']['snapshot_sha256'] if outcome['kind'] == 'stop'
                           else digest(outcome['receipt']['snapshot']))
        assert snapshot_sha256 == intent['snapshot_sha256']
        paths = [Path(binding[key]) for key in ('journal', 'stop_journal')]
        paths.append(paths[0].parent / (launch['id'] + '.controller-epochs.sqlite'))
        before = [path.read_bytes() for path in paths]
        request = {'protocol_version': 2, 'action': 'observe_controller_restart',
                   'context': binding['context'], 'policy': binding['policy'],
                   'compose': binding['compose'], 'journal': binding['journal'],
                   'registration': binding['registration'], 'mount_mapping': binding['mount_mapping'],
                   'mapping_file': binding['mapping_file']}
        witness = execute(request)['result']
        receipt = witness['receipt']
        assert witness['state'] == 'controller_restart_observed'
        assert receipt['state'] == 'observed' and receipt['observation'] == 'namespace_exited'
        assert receipt['container_id'] == outcome['receipt']['container_id']
        assert receipt['generation'] == outcome['receipt']['generation']
        assert receipt['resource_id'] == outcome['receipt']['resource_id']
        assert digest(receipt['snapshot']) == intent['snapshot_sha256']
        assert [path.read_bytes() for path in paths] == before
    result = dict(state='passed', agents=2, original_namespaces_exited=True,
                  original_snapshots_unchanged=True, native_journals_unchanged=True,
                  read_only_exit_proof=True, raw_receipts_persisted=False,
                  resumed_execution=False, sdlc_acceptance=False)
    Path('/evidence/custody-native-stop.json').write_text(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    main()
