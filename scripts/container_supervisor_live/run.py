#!/usr/bin/env python3
"""Real Fleet Rust supervisor, Docker Hermes and loaded-config acceptance in owned Compose."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import secrets
import shutil
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
CODE = Path(__file__).resolve().parent
TASK = 'fleet-container-supervisor'
PURPOSE = 'real-hermes-loaded-configuration'
PIN = 'bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3'
CONTROL_FILES = ('scripts/runtime_boundary.py', 'scripts/runtime_bootstrap.py',
                 'scripts/runtime_control.py')
BASE_FILES = (*CONTROL_FILES, 'deploy/fleet-hermes-container-launch.py',
              'deploy/runtime-boundary-qa/hermes_source.Dockerfile')


def git(home, *args):
    return subprocess.check_output(['git', '-C', str(home), *args])


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def bind(source, target, read_only=True):
    return {'type': 'bind', 'source': str(source), 'target': target, 'read_only': read_only}


def volume(source, target, read_only=False):
    return {'type': 'volume', 'source': source, 'target': target, 'read_only': read_only}


def capture(home, prefixes, target, manifest, originals):
    names = git(home, 'ls-files', '-z', '--cached', '--others', '--exclude-standard').decode().split('\0')
    for name in sorted(set(names)):
        if not name or not any(name.startswith(prefix) for prefix in prefixes):
            continue
        source = home / name
        if not source.is_file():
            continue
        if source.is_symlink() or home.resolve() not in source.resolve().parents:
            raise RuntimeError('Source capture cannot follow links or escape its repository')
        destination = target / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
        key = target.name + '/' + name
        manifest[key] = sha(source)
        originals[key] = source
        if sha(destination) != manifest[key]:
            raise RuntimeError('Frozen source capture differs from the input')


def cleanup_nested(helper, directory, write_json, checked):
    """Admit only this project's original services and exact generated agent namespaces."""
    helper.check_endpoint()
    if sha(helper.path) != helper.manifest_hash:
        raise RuntimeError('Initial Compose manifest changed; cleanup refused')
    spec = json.loads(helper.path.read_bytes())
    original_services = set(spec['services'])
    originals = {kind: helper.resources(kind) for kind in ('container', 'network', 'volume')}
    items = {}
    for kind, ids in originals.items():
        items[kind] = json.loads(checked(helper.docker + [kind, 'inspect', *ids])) if ids else []
        for item in items[kind]:
            labels = (item.get('Config', {}).get('Labels') if kind == 'container' else item.get('Labels')) or {}
            if (labels.get('com.docker.compose.project') != helper.project
                    or labels.get('sdlc.task') != TASK or labels.get('sdlc.purpose') != PURPOSE):
                raise RuntimeError('Foreign resource in own project; no cleanup adoption')
    agent_services = set()
    for item in items['container']:
        labels = item['Config']['Labels']
        service = labels['com.docker.compose.service']
        if service in original_services:
            if labels.get('com.docker.compose.project.config_files') != str(helper.path):
                raise RuntimeError('Original service manifest differs; cleanup refused')
        elif re.fullmatch(r'agent[12]-runtime-[a-f0-9]{32}', service):
            agent_services.add(service)
            spec['services'][service] = {'image': item['Image'], 'networks': [service]}
        else:
            raise RuntimeError('Unexpected container service; cleanup refused')
    for item in items['network']:
        logical = item['Labels'].get('com.docker.compose.network')
        if logical in helper.definitions['network']:
            if item['Name'] != helper.definitions['network'][logical]['name']:
                raise RuntimeError('Initial network differs; cleanup refused')
        else:
            # A failed prepare can leave its network before the agent container exists.
            service = item['Name'].removeprefix(helper.project + '-')
            if not re.fullmatch(r'agent[12]-runtime-[a-f0-9]{32}', service) or logical != service:
                raise RuntimeError('Unexpected generated agent network; cleanup refused')
            spec['networks'][logical] = {'name': item['Name'], 'internal': True}
            # Compose does not remove unreferenced declared networks on every
            # version. Down never creates this exact cleanup-only service.
            spec['services'].setdefault(service, {'image': spec['services']['fleet-backend']['image'],
                                                 'networks': [logical]})
    expected_volumes = {definition['name'] for definition in helper.definitions['volume'].values()}
    if any(item['Name'] not in expected_volumes for item in items['volume']):
        raise RuntimeError('Unexpected volume; cleanup refused')
    cleanup = directory / 'cleanup.json'
    write_json(cleanup, spec)
    checked(helper.docker + ['compose', '-p', helper.project, '-f', str(cleanup),
                             'down', '--remove-orphans'], timeout=180)
    helper.close()
    for item in items['volume']:
        name = item['Name']
        if checked(helper.docker + ['ps', '-aq', '--filter', 'volume=' + name], text=True).strip():
            raise RuntimeError('Own disposable volume still in use; not removed')
        checked(helper.docker + ['volume', 'rm', name])
    if any(helper.resources(kind) for kind in ('container', 'network', 'volume')):
        raise RuntimeError('Own Compose resources remain after cleanup')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('base-sdk', 'base-control', 'hermes-source', 'artifacts'):
        parser.add_argument('--' + name, type=Path, required=True)
    for name in ('rust-image', 'docker-image', 'hermes-image', 'postgres-image', 'context'):
        parser.add_argument('--' + name, required=True)
    args = parser.parse_args()
    sdk = args.base_sdk.resolve()
    base = args.base_control.resolve()
    sdk_pin = (ROOT / '.base-revision').read_text().strip()
    if git(sdk, 'rev-parse', 'HEAD').decode().strip() != sdk_pin or git(sdk, 'status', '--porcelain'):
        raise RuntimeError('SDK checkout must be clean and match .base-revision')
    if git(base, 'status', '--porcelain', '--untracked-files=no'):
        raise RuntimeError('Base control tracked source must be clean')
    sys.path.insert(0, str(base))
    from scripts.compose_helpers import ComposeHelper, checked, permanent_state
    from scripts.platform_postgres import private_directory, write_json

    # Reuse the existing strict archive parser instead of extracting an unchecked tar.
    archive_spec = importlib.util.spec_from_file_location('fleet_native_source',
                                                        ROOT / 'scripts/native_supervisor_live/run.py')
    archive_module = importlib.util.module_from_spec(archive_spec)
    archive_spec.loader.exec_module(archive_module)
    archive = git(args.hermes_source, 'archive', '--format=tar', PIN)
    hermes_files = archive_module.archive_files(archive)
    if not hermes_files:
        raise RuntimeError('Pinned Hermes source archive is empty')
    docker = ['docker', '--context', args.context]
    for image in (args.rust_image, args.docker_image, args.hermes_image, args.postgres_image):
        if not re.fullmatch(r'sha256:[a-f0-9]{64}', image):
            raise RuntimeError('Existing immutable image IDs are required')
        if checked(docker + ['image', 'inspect', image, '--format', '{{.Id}}'], text=True).strip() != image:
            raise RuntimeError('Image identity differs')
    hermes_dependency = json.loads(checked(docker + ['image', 'inspect', args.hermes_image]))[0]
    if hermes_dependency['Config']['Labels'].get('sdlc.hermes.revision') != PIN:
        raise RuntimeError('Hermes dependency image pin differs')
    project = 'sdlc-qa-fleet-container-live-' + secrets.token_hex(6)
    args.artifacts.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix=project + '-', dir=args.artifacts)).resolve()
    private_directory(directory)
    for name in ('input', 'build', 'hermes', 'evidence', 'proof'):
        (directory / name).mkdir()
    manifest, originals = {}, {}
    capture(ROOT, ('backend/', 'scripts/container_supervisor_live/', 'scripts/native_supervisor_live/'),
            directory / 'input/fleet-control', manifest, originals)
    capture(sdk, ('crates/', 'Cargo.toml', 'Cargo.lock', 'LICENSE'),
            directory / 'input/services-base', manifest, originals)
    capture(base, BASE_FILES, directory / 'input/base-control', manifest, originals)
    write_json(directory / 'source-manifest.json', manifest)
    (directory / 'manifest.sha256').write_text(''.join(
        f'{digest}  /tmp/src/{name}\n' for name, digest in manifest.items()), encoding='ascii')
    source_hashes = {name: hashlib.sha256(body).hexdigest() for name, body in hermes_files.items()}
    write_json(directory / 'proof/hermes-source.json', source_hashes)
    (directory / 'hermes/source.tar').write_bytes(archive)
    shutil.copyfile(directory / 'input/base-control/deploy/runtime-boundary-qa/hermes_source.Dockerfile',
                    directory / 'hermes/Dockerfile')
    shutil.copyfile(directory / 'input/fleet-control/scripts/container_supervisor_live/controller.Dockerfile',
                    directory / 'build/Dockerfile')
    tags = [project + suffix for suffix in ('-rust:qa', '-docker:qa', '-deps:qa', '-controller:qa', '-hermes:qa')]
    tagged, helper = [], None
    report = {'state': 'failed', 'project': project, 'fleet_head': git(ROOT, 'rev-parse', 'HEAD').decode().strip(),
              'base_sdk_sha': sdk_pin, 'base_control_sha': git(base, 'rev-parse', 'HEAD').decode().strip(),
              'hermes_revision': PIN, 'source_files': len(manifest), 'source_sha256': sha(directory / 'source-manifest.json'),
              'actual_rust_supervisor': False, 'actual_docker_hermes': False, 'sdlc_acceptance': False}
    before = permanent_state(docker)

    def logged(command, name, timeout=180):
        with (directory / name).open('wb') as stream:
            result = subprocess.run(command, stdout=stream, stderr=subprocess.STDOUT, timeout=timeout)
        if result.returncode:
            raise RuntimeError('Owned live gate failed; see private ' + name)

    try:
        for tag in tags:
            if subprocess.run(docker + ['image', 'inspect', tag], capture_output=True, timeout=15).returncode == 0:
                raise RuntimeError('Own unique image tag already exists; no adoption')
        for image, tag in zip((args.rust_image, args.docker_image, args.hermes_image), tags[:3], strict=True):
            checked(docker + ['image', 'tag', image, tag])
            tagged.append(tag)
        for target, folder, build_args in ((tags[3], 'build', {'RUST_IMAGE': tags[0], 'DOCKER_IMAGE': tags[1]}),
                                           (tags[4], 'hermes', {'DEPENDENCY_IMAGE': tags[2]})):
            command = docker + ['build', '--pull=false', '--network', 'none', '--label', 'sdlc.task=' + TASK,
                                '--label', 'sdlc.purpose=' + PURPOSE, '-t', target]
            for key, value in build_args.items():
                command += ['--build-arg', key + '=' + value]
            logged(command + [str(directory / folder)], folder + '-image.log', 600)
            tagged.append(target)
        controller_image = checked(docker + ['image', 'inspect', tags[3], '--format', '{{.Id}}'], text=True).strip()
        hermes_image = checked(docker + ['image', 'inspect', tags[4], '--format', '{{.Id}}'], text=True).strip()
        report.update(controller_image=controller_image, hermes_image=hermes_image)
        helper = ComposeHelper(project=project, task=TASK, purpose=PURPOSE, docker=docker,
                               directory=directory / 'compose')
        password = secrets.token_hex(24)
        frozen = directory / 'input/fleet-control/scripts/container_supervisor_live'
        init = ('import os; os.chmod("/controller",0o700); '
                '[os.chown(p,999,999) for p in ("/agents","/controller")]')
        source_probe = ('import hashlib,json; from pathlib import Path; '
                        'm=json.loads(Path("/qa/hermes-source.json").read_text()); '
                        'assert all(hashlib.sha256((Path("/opt/hermes")/p).read_bytes()).hexdigest()==h for p,h in m.items()); '
                        'print("Exact Hermes source:",len(m),"files")')
        helper.write({
            'volume-init': {'image': controller_image, 'network_mode': 'none', 'read_only': True,
                'user': '0:0', 'cap_drop': ['ALL'], 'cap_add': ['CHOWN'],
                'entrypoint': ['python3', '-B', '-c', init],
                'volumes': [volume('agents', '/agents'), volume('controller', '/controller')]},
            'source-check': {'image': hermes_image, 'network_mode': 'none', 'read_only': True,
                'user': '999:999', 'cap_drop': ['ALL'], 'entrypoint': ['/opt/hermes/.venv/bin/python', '-B', '-c', source_probe],
                'volumes': [bind(directory / 'proof', '/qa')]},
            'build': {'image': args.rust_image, 'network_mode': 'none', 'read_only': True,
                'user': '0:0', 'cap_drop': ['ALL'], 'cpus': 2, 'mem_limit': '4g', 'pids_limit': 512,
                'environment': {'RUSTUP_TOOLCHAIN': '1.88.0', 'HOME': '/tmp'},
                'entrypoint': ['bash', '/qa/build.sh'], 'tmpfs': ['/tmp:rw,exec,nosuid,size=6g,mode=1777'],
                'volumes': [bind(directory / 'input', '/input'), bind(frozen / 'build.sh', '/qa/build.sh'),
                    bind(frozen / 'select_artifact.py', '/qa/select_artifact.py'),
                    bind(directory / 'manifest.sha256', '/qa/manifest.sha256'), volume('compiled', '/out')]},
            'postgres': {'image': args.postgres_image, 'networks': ['fleet'], 'cpus': 1, 'mem_limit': '1g',
                'pids_limit': 128, 'tmpfs': ['/var/lib/postgresql/data:rw,size=512m'],
                'environment': {'POSTGRES_USER': 'fleet_qa', 'POSTGRES_DB': 'fleet_container', 'POSTGRES_PASSWORD': password},
                'healthcheck': {'test': ['CMD', 'pg_isready', '-U', 'fleet_qa', '-d', 'fleet_container'],
                    'interval': '1s', 'timeout': '2s', 'retries': 30}},
            'fleet-backend': {'image': controller_image, 'networks': ['fleet'], 'read_only': True,
                'user': '999:999', 'group_add': ['0'], 'cap_drop': ['ALL'], 'cpus': 2, 'mem_limit': '2g',
                'pids_limit': 256, 'security_opt': ['no-new-privileges:true'], 'working_dir': '/tmp',
                'entrypoint': ['python3', '-B', '-c', 'import time; time.sleep(3600)'],
                'tmpfs': ['/tmp:rw,size=128m,mode=1777'],
                'environment': {'FLEET_CONTAINER_SUPERVISOR_TEST': '1', 'HOME': '/tmp', 'PYTHONDONTWRITEBYTECODE': '1',
                    'FLEET_CONTROL_SECRET__LOCAL_MODEL': 'owned-local-model-fixture',
                    'FLEET_TEST_DATABASE_URL': f'postgresql://fleet_qa:{password}@postgres:5432/fleet_container'},
                'volumes': [volume('agents', '/agents'), volume('controller', '/controller'), volume('compiled', '/out', True),
                    bind(directory / 'input/base-control', '/base-control'), bind(directory / 'proof', '/qa'),
                    bind(directory / 'evidence', '/evidence', False),
                    bind('/var/run/docker.sock', '/var/run/docker.sock')]},
        }, volumes={'agents': {}, 'controller': {}, 'compiled': {}}, networks={'fleet': {'internal': True}},
            daemon_bind_sources=('/var/run/docker.sock',))
        logged(helper.run('source-check'), 'source-check.log')
        logged(helper.run('volume-init'), 'volume-init.log')
        logged(helper.run('build'), 'build.log', 1800)
        helper.check_endpoint()
        helper.check_ownership()
        logged(helper.command + ['up', '-d', '--wait', '--pull', 'never', 'postgres', 'fleet-backend'], 'startup.log')
        cid = checked(helper.command + ['ps', '-q', 'fleet-backend'], text=True).strip()
        item = json.loads(checked(docker + ['container', 'inspect', cid]))[0]
        if item['Image'] != controller_image or not item['State']['Running']:
            raise RuntimeError('Live Fleet controller identity differs')
        write_json(directory / 'proof/controller-proof.json', {
            'engine_id': helper.identity, 'agents_volume': project + '_agents', 'model_host': item['Name'].lstrip('/'),
            'control': {'python': 'python3', 'base_root': '/base-control', 'context': 'default',
                'source_sha256': [sha(directory / 'input/base-control' / name) for name in CONTROL_FILES],
                'provisioning': {'project': project, 'image_id': hermes_image, 'user': '999:999',
                    'entrypoint': ['/opt/hermes/.venv/bin/python', '/runtime/hermes-container.py'],
                    'pids_limit': 128, 'memory_bytes': 1073741824, 'nano_cpus': 1000000000,
                    'network_internal': True, 'task': TASK, 'purpose': PURPOSE},
                'bridge_controller': {'container_id': cid, 'image_id': controller_image, 'service': 'fleet-backend'}}})
        logged(helper.command + ['exec', '-T', 'fleet-backend', '/out/fleet-container-live', '--ignored',
                                 '--nocapture', '--test-threads=1'], 'live.log', 1200)
        live = json.loads((directory / 'evidence/live-report.json').read_bytes())
        if (live.get('state') != 'passed' or not live.get('actual_rust_supervisor')
                or not live.get('actual_docker_hermes') or live.get('sdlc_acceptance') is not False):
            raise RuntimeError('Actual Rust/Hermes evidence is incomplete')
        report.update(state='passed', actual_rust_supervisor=True, actual_docker_hermes=True,
                      live=live, build_log_sha256=sha(directory / 'build.log'), live_log_sha256=sha(directory / 'live.log'))
    finally:
        try:
            if helper is not None:
                cleanup_nested(helper, directory, write_json, checked)
                report['cleaned'] = True
            for tag in reversed(tagged):
                checked(docker + ['image', 'rm', tag])
            report['own_image_tags_removed'] = True
        finally:
            report['sources_unchanged'] = all(sha(path) == manifest[name] for name, path in originals.items())
            report['permanent_runtime_unchanged'] = permanent_state(docker) == before
            write_json(directory / 'report.json', report)
            print(json.dumps({'state': report['state'], 'project': project, 'report': str(directory / 'report.json')}))
    if not all(report.get(name) is True for name in ('cleaned', 'own_image_tags_removed', 'sources_unchanged',
                                                     'permanent_runtime_unchanged')):
        raise RuntimeError('Owned cleanup or unchanged-runtime/source evidence is incomplete')


if __name__ == '__main__':
    main()
