"""Copy only an explicitly bound Cargo test artifact to the disposable volume."""
import json
from pathlib import Path
import shutil
import sys


def main():
    output = Path(sys.argv[2])
    target = {
        Path('/out/fleet-container-live'): ('container_supervisor_live', ['test']),
        Path('/out/fleet-runtime-tests'): ('infra', ['lib']),
    }.get(output)
    if target is None or output.exists():
        raise RuntimeError('No unknown output adoption or replacement is allowed')
    entries = [json.loads(line) for line in Path(sys.argv[1]).read_text().splitlines()]
    candidates = [Path(item['executable']) for item in entries
                  if item.get('reason') == 'compiler-artifact'
                  and item.get('target', {}).get('name') == target[0]
                  and item.get('target', {}).get('kind') == target[1]
                  and item.get('profile', {}).get('test') is True and item.get('executable')]
    if len(candidates) != 1 or not candidates[0].is_file():
        raise RuntimeError('Exactly one bound compiled test is required')
    binary = candidates[0].resolve()
    if Path('/tmp/container-target') not in binary.parents:
        raise RuntimeError('Artifact is outside this invocation target directory')
    shutil.copyfile(binary, output)
    output.chmod(0o755)
    print(json.dumps({'compiled_test': output.name, 'cargo_profile_test': True}))


if __name__ == '__main__':
    main()
