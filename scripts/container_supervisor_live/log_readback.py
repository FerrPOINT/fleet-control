"""Owned post-stop Base log proof. Never print or persist raw agent output."""
import hashlib
import json
from pathlib import Path
import sys


def main():
    proof = json.loads(Path('/qa/controller-proof.json').read_bytes())
    root = Path(proof['control']['base_root'])
    for name, expected in zip(('runtime_boundary.py', 'runtime_bootstrap.py', 'runtime_control.py'),
                              proof['control']['source_sha256'], strict=True):
        assert hashlib.sha256((root / 'scripts' / name).read_bytes()).hexdigest() == expected
    sys.path.insert(0, str(root))
    from scripts.runtime_control import execute
    documents = sorted(Path('/controller').glob('*.container-prepared.json'))
    assert len(documents) >= 4
    resources, count, nonempty = set(), 0, 0
    for path in documents:
        document = json.loads(path.read_bytes())
        boundary = document['container']
        assert boundary['source_sha256'] == proof['control']['source_sha256']
        request = {'protocol_version': 2 if boundary.get('mount_mapping') else 1,
                   'action': 'logs', 'context': boundary['context'], 'policy': boundary['policy'],
                   'compose': boundary['compose'], 'journal': boundary['journal'],
                   'registration': boundary['registration'], 'tail': 200}
        if request['protocol_version'] == 2:
            request.update(mount_mapping=boundary['mount_mapping'], mapping_file=boundary['mapping_file'])
        result = execute(request)['result']
        assert result['receipt']['state'] == 'observed'
        assert result['receipt']['observation'] == 'namespace_exited'
        assert result['receipt']['generation'] == boundary['registration']['generation']
        assert set(result) == {'receipt', 'stdout_base64', 'stderr_base64'}
        nonempty += bool(result['stdout_base64'] or result['stderr_base64'])
        resources.add(result['receipt']['resource_id'])
        count += 1
    assert len(resources) == 2 and nonempty >= 2
    report = {'state': 'passed', 'actual_base_log_readback': True, 'agents': 2,
              'original_exited_generations': count, 'nonempty_generations': nonempty,
              'raw_logs_persisted': False, 'fleet_log_ingestion': False, 'sdlc_acceptance': False}
    Path('/evidence/log-readback.json').write_text(json.dumps(report, sort_keys=True))


if __name__ == '__main__':
    main()
