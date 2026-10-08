#!/usr/bin/env python3
"""Managed Fleet/native gateway gate in an owned, disposable Compose project."""
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import tempfile
import uuid
import re
import shutil


ROOT = Path(__file__).resolve().parents[2]
PIN = 'bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3'
TASK = 'fleet-native-supervisor'
SWAGGER_ARCHIVE_SHA256 = '481244d0812097b11fbaeef79f71d942b171617f9c9f9514e63acbe13e71ccdc'
OBSERVER_REVISION = '43b365fd97e8955821312cbc2e68b4d83e5bd240'
OBSERVER_FIXTURE_REVISION = 'dd9ce31a6b97e31e2467662658e38dd7a6485f45'
OBSERVER_GIT_PATH = '/qa/observer-base.git'
OBSERVER_GIT_CONFIG_PATH = '/qa/observer-git-config'
OBSERVER_FILES = {
    '__init__.py': '6a5ea8b412a75dcf3803843be89409c931471c07188370e06a684443d44789a3',
    'plugin.py': '1e21d8c0e6fcac136b966b09d4e5c433bc9ff1e4ed3be6cb667fee9718d3f6de',
    'store.py': 'c33cd12cf59d6c9bf1d4e2aade98743d56de4d4e9a924df00e7a0cb302f20170',
    'plugin.yaml': 'ed39d83e75c2ecfeab0182beb1eee1e7ce69623680fe3d4b6b76240a13ba985a',
}
TEST_NAMES = {
    'lifecycle':'managed_native_gateway_isolates_home_soul_messages_and_restart_history',
    'recovery':'managed_native_lost_ack_recovers_original_run_across_fleet_processes',
    'controls':'managed_native_run_steer_and_stop_require_native_ack_and_terminal_readback',
    'control-outcomes':'managed_native_original_control_outcomes_recover_lost_http_ack',
    'control-restart':'native_control_restart::managed_native_control_outcomes_survive_fleet_process_death',
    'approvals':'native_approvals::managed_native_approval_decisions_are_exact_once_and_unknown_ack_is_held',
    'approval-recovery':'native_approvals::native_approval_recovery::managed_native_waiting_approval_recovers_across_fleet_processes',
    'approval-outcomes':'native_approvals::managed_native_original_approval_outcomes_recover_lost_http_ack',
    'approval-restart':'native_approvals::native_approval_restart::managed_native_approval_outcomes_survive_fleet_process_death',
    'combined-recovery':'native_approvals::native_approval_restart::managed_native_combined_run_and_approval_outcomes_survive_fleet_process_death',
    'combined-controls':'native_control_restart::managed_native_combined_run_and_control_outcomes_survive_fleet_process_death',
    'observer':'native_request_observer::managed_native_observer_activation_reads_two_original_runs_with_existing_extensions',
}
PLUGIN_FILES = ('__init__.py', 'plugin.py', 'store.py', 'plugin.yaml')


def scenario_plugins(scenario):
    kinds = []
    if scenario in {'control-outcomes', 'control-restart', 'approval-outcomes', 'approval-restart', 'combined-recovery', 'combined-controls', 'observer'}:
        kinds.append('control')
    if scenario in {'recovery', 'combined-recovery', 'combined-controls', 'observer'}:
        kinds.append('recovery')
    return tuple(kinds)


def git(repo, *args):
    return subprocess.check_output(['git', '--no-replace-objects', '-C', str(repo), *args], timeout=30)


def observer_cache(repo):
    """Require a self-contained read-only cache; linked Windows worktrees cannot be mounted."""
    root = Path(repo)
    if root.is_symlink() or not root.is_dir():
        raise RuntimeError('Observer requires an existing self-contained bare Git cache')
    root = root.resolve()
    if git(root, 'rev-parse', '--is-bare-repository').strip() != b'true':
        raise RuntimeError('Observer requires a bare cache, not a linked checkout')
    git_dir = Path(git(root, 'rev-parse', '--absolute-git-dir').decode().strip()).resolve()
    common = Path(git(root, 'rev-parse', '--git-common-dir').decode().strip())
    common = common.resolve() if common.is_absolute() else (root / common).resolve()
    if git_dir != root or common != root or (root / 'objects/info/alternates').exists():
        raise RuntimeError('Observer cache must not depend on shared Git paths')
    origin = git(root, 'config', '--get', 'remote.origin.url').decode().strip()
    if origin not in ('https://github.com/FerrPOINT/services-base.git',
                      'git@github.com:FerrPOINT/services-base.git'):
        raise RuntimeError('Observer cache has a foreign source repository')
    prefix = 'deploy/hermes-request-observer/'
    if git(root, 'cat-file', '-t', OBSERVER_REVISION).strip() != b'commit':
        raise RuntimeError('Observer source commit is unavailable')
    files = archive_files(git(root, 'archive', '--format=tar', OBSERVER_REVISION,
                              *[prefix + name for name in PLUGIN_FILES]))
    expected = {prefix + name: digest for name, digest in OBSERVER_FILES.items()}
    if {name: hashlib.sha256(body).hexdigest() for name, body in files.items()} != expected:
        raise RuntimeError('Observer cache bytes differ from the pinned producer')
    return root


def observer_git_config(directory):
    """Trust only the validated read-only QA cache, never every repository."""
    path = directory / 'observer-git-config'
    path.write_bytes(('[safe]\n\tdirectory = ' + OBSERVER_GIT_PATH + '\n').encode('ascii'))
    return path


def fixture_revision(scenario, requested, checkout):
    if scenario == 'observer' and requested != OBSERVER_FIXTURE_REVISION:
        raise RuntimeError('Observer requires its explicit committed recovery/control fixture revision')
    if requested is not None and not re.fullmatch('[a-f0-9]{40}', requested):
        raise RuntimeError('Base fixture revision must be a full commit ID')
    revision = requested or git(checkout, 'rev-parse', 'HEAD').decode().strip()
    if git(checkout, 'cat-file', '-t', revision).strip() != b'commit':
        raise RuntimeError('Base fixture revision is not a commit')
    return revision


def archive_files(archive):
    result = {}
    with tarfile.open(fileobj=io.BytesIO(archive)) as source:
        for member in source:
            path = PurePosixPath(member.name)
            if path.is_absolute() or '..' in path.parts or '\\' in member.name or ':' in member.name:
                raise RuntimeError('Unsafe source archive path')
            if path.parts and path.parts[0] == '.venv':
                raise RuntimeError('Source archive must not replace dependency environment')
            if member.isfile():
                result[member.name] = source.extractfile(member).read()
            elif not member.isdir():
                raise RuntimeError('Non-regular source archive member')
    return result


def source_dockerfile(alias):
    if not re.fullmatch('sdlc-qa-fleet-native-[a-f0-9]{12}-deps:qa', alias):
        raise RuntimeError('QA source layer requires its owned dependency alias')
    return f'FROM {alias}\nADD source.tar /opt/hermes/\n'


def build_target_directory(project):
    if not re.fullmatch('sdlc-qa-fleet-native-[a-f0-9]{12}', project):
        raise RuntimeError('Native compilation requires its owned Compose project')
    return '/cache/' + project


def verify_source_layer(dependency, staged):
    original = dependency['RootFS']['Layers']
    layers = staged['RootFS']['Layers']
    if (staged['Config'].get('User') != dependency['Config']['User']
            or len(layers) != len(original)+1 or layers[:-1] != original):
        raise RuntimeError('QA source layer changed its dependency or runtime identity')


def bind(source, target):
    return {'type':'bind', 'source':str(source), 'target':target, 'read_only':True,
            'bind':{'create_host_path':False}}


def snapshot_fleet(root, destination):
    destination.mkdir(exist_ok=False)
    paths = git(root, 'ls-files', '-z', '--cached', '--others', '--exclude-standard',
                '--', 'backend', 'scripts', '.base-revision').decode().split('\0')
    hashes = {}
    for name in sorted(set(paths) - {''}):
        path = PurePosixPath(name)
        if path.is_absolute() or '..' in path.parts or '\\' in name or ':' in name:
            raise RuntimeError('Unsafe Fleet build input path')
        source = root / name
        if source.is_symlink() or not source.resolve().is_relative_to(root.resolve()):
            raise RuntimeError('Fleet build input is a link or outside the repository')
        if not source.is_file():
            continue
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        digest = hashlib.sha256(source.read_bytes()).hexdigest()
        if hashlib.sha256(target.read_bytes()).hexdigest() != digest:
            raise RuntimeError('Fleet source changed during snapshot capture')
        hashes[name] = digest
    if not hashes or 'backend/Cargo.lock' not in hashes or '.base-revision' not in hashes:
        raise RuntimeError('Fleet snapshot is incomplete')
    return hashes


def service(purpose):
    return {'labels':{'sdlc.task':TASK,'sdlc.purpose':purpose}, 'networks':['qa'],
            'cpus':2, 'mem_limit':'3g', 'pids_limit':256}


def recovery_files(repo, revision, controls=False):
    prefix = 'deploy/hermes-control-plugin/' if controls else 'deploy/hermes-recovery-plugin/'
    entries = archive_files(git(repo, 'archive', '--format=tar', revision,
                                *[prefix+name for name in PLUGIN_FILES]))
    if set(entries) != {prefix+name for name in PLUGIN_FILES}:
        raise RuntimeError('Committed runtime plugin inventory is incomplete')
    return {name:entries[prefix+name] for name in PLUGIN_FILES}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--hermes', type=Path, required=True)
    parser.add_argument('--base-sdk', type=Path, required=True)
    parser.add_argument('--base-checkout', type=Path, required=True)
    parser.add_argument('--base-revision', help='exact committed launcher/recovery/control fixture revision')
    parser.add_argument('--observer-checkout', type=Path, help='self-contained bare Base observer Git cache')
    parser.add_argument('--image', required=True, help='existing immutable native dependency image')
    parser.add_argument('--scenario', choices=tuple(TEST_NAMES), default='lifecycle')
    parser.add_argument('--target-cache', required=True)
    parser.add_argument('--cargo-cache', required=True)
    parser.add_argument('--rustup-cache', required=True)
    parser.add_argument('--artifacts', type=Path, default=ROOT / 'tmp/native-supervisor-live')
    args = parser.parse_args()
    if git(args.hermes, 'rev-parse','HEAD').decode().strip() != PIN or git(args.hermes,'status','--porcelain'):
        raise RuntimeError('Hermes must be clean and match the exact source pin')
    archive = git(args.hermes,'archive','--format=tar',PIN)
    files = archive_files(archive)
    if not files:
        raise RuntimeError('Pinned native source archive is empty')
    sdk_pin = (ROOT / '.base-revision').read_text().strip()
    if (git(args.base_sdk, 'rev-parse','HEAD').decode().strip() != sdk_pin
            or git(args.base_sdk,'status','--porcelain')):
        raise RuntimeError('Base SDK must match Fleet .base-revision exactly')
    observer = None
    if args.scenario == 'observer':
        if args.observer_checkout is None:
            raise RuntimeError('Observer scenario requires its read-only producer Git cache')
        observer = observer_cache(args.observer_checkout)
    elif args.observer_checkout is not None:
        raise RuntimeError('Observer cache is only used by the observer scenario')
    base_head = fixture_revision(args.scenario, args.base_revision, args.base_checkout)
    launcher = archive_files(git(args.base_checkout,'archive','--format=tar',base_head,'deploy/fleet-hermes-launch.py'))['deploy/fleet-hermes-launch.py']
    plugins = {kind:recovery_files(args.base_checkout, base_head, controls=kind == 'control')
               for kind in scenario_plugins(args.scenario)}
    image = json.loads(subprocess.check_output(['docker','image','inspect',args.image]))[0]
    if image['Config'].get('Labels',{}).get('sdlc.hermes.revision') != PIN:
        raise RuntimeError('Dependency image revision label is incompatible')
    if image['Config'].get('User','').split(':')[0] in ['', '0', 'root']:
        raise RuntimeError('Managed gateway requires a non-root dependency image user')
    args.artifacts.mkdir(parents=True, exist_ok=True)
    project = 'sdlc-qa-fleet-native-' + uuid.uuid4().hex[:12]
    target_directory = build_target_directory(project)
    directory = Path(tempfile.mkdtemp(prefix=project+'-',dir=args.artifacts)).resolve()
    fleet_snapshot = directory / 'fleet-source'
    fleet_input_hashes = snapshot_fleet(ROOT, fleet_snapshot)
    scripts = Path(__file__).resolve().parent
    compose_path = directory / 'compose.json'
    hashes = {name:hashlib.sha256(body).hexdigest() for name,body in files.items()}
    (directory/'source-hashes.json').write_text(json.dumps(hashes,sort_keys=True)+'\n',encoding='utf-8')
    (directory/'hermes').write_bytes(launcher)
    (directory/'hermes').chmod(0o755)
    # BuildKit stages only the immutable archive, not Windows bind-mounted source.
    context = directory / 'source-layer'
    context.mkdir()
    (context/'source.tar').write_bytes(archive)
    dependency_alias = project + '-deps:qa'
    (context/'Dockerfile').write_text(source_dockerfile(dependency_alias),encoding='utf-8')
    qa_image = project + '-source:qa'
    report = {'project':project,'cargo_target_directory':target_directory,'swagger_archive_sha256':SWAGGER_ARCHIVE_SHA256,'result':'failed','native_source_sha':PIN,'source_archive_sha256':hashlib.sha256(archive).hexdigest(),
              'scenario':args.scenario, 'test_name':TEST_NAMES[args.scenario],
              'dependency_image_id':image['Id'],'image_user':image['Config']['User'],'base_sdk_sha':sdk_pin,'launcher_base_sha':base_head,
              'launcher_sha256':hashlib.sha256(launcher).hexdigest(),
              'fleet_head':git(ROOT,'rev-parse','HEAD').decode().strip(),
              'fleet_worktree_clean':not bool(git(ROOT,'status','--porcelain')),
              'fleet_snapshot_files_sha256':fleet_input_hashes,
              'fleet_test_source_sha256':hashlib.sha256((ROOT/'backend/infra/tests/native_supervisor_live.rs').read_bytes()).hexdigest(),
              'fleet_test_sources_sha256':{name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest()
                  for name in ['backend/infra/tests/native_supervisor_live.rs',
                               'backend/infra/tests/support/native_approvals.rs',
                               'backend/infra/tests/support/native_control_restart.rs',
                               'backend/infra/tests/support/native_prepared_launch.rs',
                               'backend/infra/tests/support/native_approval_restart.rs',
                               'backend/infra/tests/support/native_approval_recovery.rs',
                               'backend/infra/tests/support/native_request_observer.rs']},
              'fleet_runtime_sources_sha256':{name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest()
                  for name in ['backend/api/src/lib.rs', 'backend/api/src/routes/sessions.rs',
                               'backend/domain/src/lib.rs', 'backend/infra/src/base_package.rs',
                               'backend/infra/src/effective_configuration.rs',
                               'backend/api/src/routes/approvals.rs', 'backend/api/src/routes/agents.rs',
                               'backend/app/src/lib.rs', 'backend/infra/src/lib.rs',
                               'backend/app/src/runtime_launch.rs', 'backend/infra/src/runtime_launches.rs',
                               'backend/infra/src/message_dispatch.rs',
                               'backend/infra/src/runtime/launch_journal.rs', 'backend/infra/src/config_revisions.rs',
                               'backend/infra/src/runtime/prepared_dispatch.rs',
                               'backend/infra/src/runtime/request_observation.rs',
                               'backend/infra/src/request_observer_package.rs',
                               'backend/infra/src/configuration_disk.rs',
                               'backend/infra/src/pm_credentials.rs',
                               'backend/infra/src/runtime/activation_journal.rs',
                               'backend/infra/src/hermes_dispatch_journal.rs', 'backend/infra/src/hermes_approval_recovery.rs',
                               'backend/infra/src/runtime/approval_snapshot.rs', 'backend/infra/src/runtime/acceptance_readback.rs',
                               'backend/infra/src/runtime/mod.rs', 'backend/infra/src/runtime/native_context.rs',
                               'backend/infra/src/runtime/run_control.rs', 'backend/infra/src/runtime/targeted_approval.rs',
                               'backend/infra/src/runtime/approval_outcome.rs', 'backend/infra/src/approval_outcomes.rs',
                               'backend/infra/src/runtime/control_outcome_wire.rs', 'backend/infra/src/runtime/control_outcome_readback.rs',
                               'backend/infra/src/runtime_controls.rs', 'backend/shared/src/config.rs',
                               'backend/migration/src/lib.rs',
                               'backend/migration/src/m20261004_000012_hermes_dispatch_journal.rs',
                               'backend/migration/src/m20261005_000013_runtime_controls.rs',
                               'backend/migration/src/m20261005_000014_hermes_journal_time_order.rs',
                               'backend/migration/src/m20261005_000015_runtime_control_outcomes.rs',
                               'backend/migration/src/m20261005_000016_runtime_approval_outcomes.rs',
                               'backend/migration/src/m20261006_000017_runtime_launches.rs']},
              'harness_sha256':{name:hashlib.sha256((scripts/name).read_bytes()).hexdigest() for name in ['run.py','build.sh','native.sh','preflight.py','discard_ack_plugin.py','approval_fault_plugin.py','control_fault_plugin.py','observer_fault_plugin.py','observer_start_fault.py']}}
    for plugin_kind, plugin in plugins.items():
        plugin_dir = directory/(plugin_kind+'-plugin')
        plugin_dir.mkdir()
        for name, body in plugin.items():
            (plugin_dir/name).write_bytes(body)
        plugin_hashes = {name:hashlib.sha256(body).hexdigest() for name,body in plugin.items()}
        (directory/(plugin_kind+'-hashes.json')).write_text(json.dumps(plugin_hashes)+'\n',encoding='utf-8')
        report[plugin_kind+'_plugin_base_sha'] = base_head
        report[plugin_kind+'_plugin_hashes'] = plugin_hashes
    postgres = service('disposable-native-postgresql')
    postgres.update(image='postgres:17.6-alpine',tmpfs=['/var/lib/postgresql/data:rw,size=512m'],
        environment={'POSTGRES_USER':'native_qa','POSTGRES_DB':'fleet_native_test','POSTGRES_HOST_AUTH_METHOD':'trust'},
        healthcheck={'test':['CMD','pg_isready','-U','native_qa','-d','fleet_native_test'],'interval':'1s','timeout':'2s','retries':30})
    build = service('rust-native-test-build')
    build.update(image='rust:1.88.0-bookworm',working_dir='/work/fleet-control/backend',entrypoint=['bash','/qa/build.sh'],
        environment={'RUSTUP_TOOLCHAIN':'1.88.0','CARGO_TARGET_DIR':target_directory,'CARGO_BUILD_JOBS':'2',
            'FLEET_NATIVE_SWAGGER_SHA256':SWAGGER_ARCHIVE_SHA256,
            'CARGO_INCREMENTAL':'0','CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0'},
        volumes=[bind(fleet_snapshot,'/work/fleet-control'),bind(args.base_sdk.resolve(),'/work/services-base'),
            bind(scripts/'build.sh','/qa/build.sh'),'target:/cache','cargo:/usr/local/cargo','rustup:/usr/local/rustup'])
    native = service('real-fleet-gateways-native-pg-local-model')
    native.update(image=image['Id'],pull_policy='never',init=True,read_only=True,
        cap_drop=['ALL'],security_opt=['no-new-privileges:true'],working_dir='/tmp',entrypoint=['bash','/qa/native.sh'],
        tmpfs=['/tmp:rw,size=768m','/run:rw,size=32m'],environment={
            'FLEET_NATIVE_SUPERVISOR_TEST':'1','FLEET_TEST_DATABASE_URL':'postgres://native_qa@postgres:5432/fleet_native_test',
            'FLEET_NATIVE_TEST_NAME':TEST_NAMES[args.scenario],
            'FLEET_CONTROL_SECRET__LOCAL_MODEL':'owned-local-model-fixture','HOME':'/tmp/fleet-native-supervisor/parent',
            'PYTHONDONTWRITEBYTECODE':'1','PYTHONUNBUFFERED':'1'},
        volumes=['target:/cache:ro',bind(scripts/'native.sh','/qa/native.sh'),bind(scripts/'preflight.py','/qa/preflight.py'),
            bind(directory/'source-hashes.json','/qa/source-hashes.json'),bind(directory/'hermes','/opt/fleet-hermes/bin/hermes')])
    if observer is not None:
        startup_fixture = directory / 'observer-start-fault'
        startup_fixture.write_bytes((scripts / 'observer_start_fault.py').read_bytes())
        startup_fixture.chmod(0o755)
        startup_opt_in = directory / 'observer-start-fault-opt-in'
        startup_opt_in.write_bytes(b'fleet-native-observer-start-fault/v1\n')
        git_config = observer_git_config(directory)
        native['environment'].update(FLEET_OBSERVER_BASE_CHECKOUT=OBSERVER_GIT_PATH,
                                     GIT_CONFIG_GLOBAL=OBSERVER_GIT_CONFIG_PATH,
                                     GIT_CONFIG_NOSYSTEM='1')
        native['volumes'].append(bind(observer, OBSERVER_GIT_PATH))
        native['volumes'].append(bind(git_config, OBSERVER_GIT_CONFIG_PATH))
        native['volumes'].append(bind(startup_fixture, '/opt/fleet-hermes/bin/observer-hermes'))
        native['volumes'].append(bind(startup_opt_in, '/qa/observer-start-fault-opt-in'))
        report['observer_start_fault_opt_in_sha256'] = hashlib.sha256(startup_opt_in.read_bytes()).hexdigest()
        report['observer_git_config_sha256'] = hashlib.sha256(git_config.read_bytes()).hexdigest()
        report['observer_source_revision'] = OBSERVER_REVISION
        report['observer_source_hashes'] = OBSERVER_FILES
    for plugin_kind in plugins:
        plugin_dir = directory/(plugin_kind+'-plugin')
        if plugin_kind == 'recovery':
            native['environment']['FLEET_NATIVE_FAULT_ROOT'] = '/tmp/fleet-native-supervisor/recovery-fault'
        native['volumes'].extend([bind(plugin_dir,'/qa/'+plugin_kind+'-plugin'),
            bind(directory/(plugin_kind+'-hashes.json'),'/qa/'+plugin_kind+'-hashes.json')])
    compose = {'services':{'postgres':postgres,'build':build,'native':native},'networks':{'qa':{'internal':True}},
        'volumes':{key:{'external':True,'name':name} for key,name in [('target',args.target_cache),('cargo',args.cargo_cache),('rustup',args.rustup_cache)]}}
    def save():
        compose_path.write_text(json.dumps(compose,indent=2)+'\n',encoding='utf-8')
    def dc(*commands, timeout=120):
        return subprocess.run(['docker','compose','-p',project,'-f',str(compose_path),*commands],
                              stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=timeout,check=False)
    save()
    try:
        subprocess.run(['docker','image','tag',image['Id'],dependency_alias],check=True,timeout=30)
        alias_image = json.loads(subprocess.check_output(['docker','image','inspect',dependency_alias]))[0]
        if alias_image['Id'] != image['Id']:
            raise RuntimeError('Owned dependency alias identity changed')
        layer = subprocess.run(['docker','build','--network','none','--pull=false',
            '--label','sdlc.task='+TASK,'--label','sdlc.purpose=disposable-native-source-layer',
            '-t',qa_image,str(context)],stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=600,check=False)
        (directory/'source-layer.log').write_bytes(layer.stdout)
        if layer.returncode: raise RuntimeError('Owned native source layer build failed')
        staged_image = json.loads(subprocess.check_output(['docker','image','inspect',qa_image]))[0]
        verify_source_layer(image,staged_image)
        report['image_id'] = staged_image['Id']
        native['image'] = staged_image['Id']
        save()
        for commands in [('config','--quiet'),('up','-d','--wait','--pull','never','postgres')]:
            result = dc(*commands)
            if result.returncode: raise RuntimeError('Owned Compose preparation failed')
        result = dc('run','--rm','--no-deps','-T','build',timeout=900)
        (directory/'build.log').write_bytes(result.stdout)
        if result.returncode: raise RuntimeError('Native test Rust gates failed; see build.log')
        artifacts = [json.loads(line) for line in result.stdout.splitlines() if line.startswith(b'{')]
        executables = [item['executable'] for item in artifacts if item.get('reason')=='compiler-artifact'
                       and item.get('target',{}).get('name')=='native_supervisor_live' and item.get('executable')]
        if len(executables) != 1: raise RuntimeError('Expected one exact native test executable')
        native['environment']['FLEET_NATIVE_TEST_BINARY'] = executables[0]
        save()
        result = dc('run','--rm','--no-deps','-T','native',timeout=1500)
        (directory/'native.log').write_bytes(result.stdout)
        print(result.stdout.decode('utf-8',errors='replace'))
        report['native_exit_code'] = result.returncode
        report['native_log_sha256'] = hashlib.sha256(result.stdout).hexdigest()
        if result.returncode: raise RuntimeError('Managed native acceptance failed')
        for root in (ROOT, fleet_snapshot):
            if any(hashlib.sha256((root / path).read_bytes()).hexdigest() != expected
                   for path, expected in fleet_input_hashes.items()):
                raise RuntimeError('Fleet source differs from the frozen binary input')
        for field in ('fleet_test_sources_sha256', 'fleet_runtime_sources_sha256'):
            if any(hashlib.sha256((ROOT / path).read_bytes()).hexdigest() != expected
                   for path, expected in report[field].items()):
                raise RuntimeError('Fleet native source changed after the binary input capture')
        if any(hashlib.sha256((scripts / path).read_bytes()).hexdigest() != expected
               for path, expected in report['harness_sha256'].items()):
            raise RuntimeError('Native harness changed after the binary input capture')
        if (not re.search(rb'test result: ok\. 1 passed; 0 failed; 0 ignored;',result.stdout)
                or TEST_NAMES[args.scenario].encode() not in result.stdout
                or ('Exact pinned native tracked source verified: '+str(len(hashes))+' files').encode() not in result.stdout):
            raise RuntimeError('Native evidence is incomplete or no test ran')
        for plugin_kind in plugins:
            if ('Exact committed '+plugin_kind+' plugin verified: 4 files').encode() not in result.stdout:
                raise RuntimeError('Native runtime plugin preflight is missing')
        report['result'] = 'passed'
    except subprocess.TimeoutExpired as error:
        (directory/'timeout.log').write_bytes(error.output or b'')
        report['timeout'] = True
        raise RuntimeError('Owned acceptance deadline expired') from None
    finally:
        try:
            try:
                cleanup = dc('down','--remove-orphans')
                (directory/'cleanup.log').write_bytes(cleanup.stdout)
                report['cleanup_exit_code'] = cleanup.returncode
                if cleanup.returncode: report['result'] = 'cleanup_failed'
            except (subprocess.TimeoutExpired,OSError) as error:
                (directory/'cleanup.log').write_bytes(getattr(error,'output',None) or b'')
                report['cleanup_error'] = type(error).__name__
                report['result'] = 'cleanup_failed'
            try:
                removed = subprocess.run(['docker','image','rm',qa_image,dependency_alias],stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,timeout=120,check=False)
                (directory/'source-layer-cleanup.log').write_bytes(removed.stdout)
                # Build failures may have created no tag at all.
                for name in [qa_image,dependency_alias]:
                    present = subprocess.run(['docker','image','inspect',name],stdout=subprocess.PIPE,
                        stderr=subprocess.STDOUT,timeout=30,check=False)
                    removed_tag = present.returncode != 0
                    report.setdefault('qa_image_tags_removed',{})[name] = removed_tag
                    if not removed_tag: report['result'] = 'cleanup_failed'
            except (subprocess.TimeoutExpired,OSError) as error:
                report['image_cleanup_error'] = type(error).__name__
                report['result'] = 'cleanup_failed'
            try:
                remaining = dc('ps','-a','--format','json')
                report['post_cleanup_empty'] = remaining.returncode == 0 and remaining.stdout.strip() in [b'',b'[]']
                if not report['post_cleanup_empty']: report['result'] = 'cleanup_failed'
            except (subprocess.TimeoutExpired,OSError) as error:
                report['cleanup_ps_error'] = type(error).__name__
                report['result'] = 'cleanup_failed'
        finally:
            (directory/'evidence.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
            print('Evidence: '+str(directory/'evidence.json'))
    if report['result'] != 'passed': raise RuntimeError('Acceptance or exact cleanup failed')


if __name__ == '__main__':
    main()
