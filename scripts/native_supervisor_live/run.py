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


ROOT = Path(__file__).resolve().parents[2]
PIN = 'bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3'
TASK = 'fleet-native-supervisor'
TEST_NAMES = {
    'lifecycle':'managed_native_gateway_isolates_home_soul_messages_and_restart_history',
    'recovery':'managed_native_lost_ack_recovers_original_run_across_fleet_processes',
    'controls':'managed_native_run_steer_and_stop_require_native_ack_and_terminal_readback',
    'approvals':'native_approvals::managed_native_approval_decisions_are_exact_once_and_unknown_ack_is_held',
}
PLUGIN_FILES = ('__init__.py', 'plugin.py', 'store.py', 'plugin.yaml')


def git(repo, *args):
    return subprocess.check_output(['git', '-C', str(repo), *args])


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


def verify_source_layer(dependency, staged):
    original = dependency['RootFS']['Layers']
    layers = staged['RootFS']['Layers']
    if (staged['Config'].get('User') != dependency['Config']['User']
            or len(layers) != len(original)+1 or layers[:-1] != original):
        raise RuntimeError('QA source layer changed its dependency or runtime identity')


def bind(source, target):
    return {'type':'bind', 'source':str(source), 'target':target, 'read_only':True,
            'bind':{'create_host_path':False}}


def service(purpose):
    return {'labels':{'sdlc.task':TASK,'sdlc.purpose':purpose}, 'networks':['qa'],
            'cpus':2, 'mem_limit':'3g', 'pids_limit':256}


def recovery_files(repo, revision):
    prefix = 'deploy/hermes-recovery-plugin/'
    entries = archive_files(git(repo, 'archive', '--format=tar', revision,
                                *[prefix+name for name in PLUGIN_FILES]))
    if set(entries) != {prefix+name for name in PLUGIN_FILES}:
        raise RuntimeError('Committed recovery plugin inventory is incomplete')
    return {name:entries[prefix+name] for name in PLUGIN_FILES}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--hermes', type=Path, required=True)
    parser.add_argument('--base-sdk', type=Path, required=True)
    parser.add_argument('--base-checkout', type=Path, required=True)
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
    base_head = git(args.base_checkout, 'rev-parse','HEAD').decode().strip()
    launcher = archive_files(git(args.base_checkout,'archive','--format=tar',base_head,'deploy/fleet-hermes-launch.py'))['deploy/fleet-hermes-launch.py']
    plugin = recovery_files(args.base_checkout, base_head) if args.scenario == 'recovery' else None
    image = json.loads(subprocess.check_output(['docker','image','inspect',args.image]))[0]
    if image['Config'].get('Labels',{}).get('sdlc.hermes.revision') != PIN:
        raise RuntimeError('Dependency image revision label is incompatible')
    if image['Config'].get('User','').split(':')[0] in ['', '0', 'root']:
        raise RuntimeError('Managed gateway requires a non-root dependency image user')
    args.artifacts.mkdir(parents=True, exist_ok=True)
    project = 'sdlc-qa-fleet-native-' + uuid.uuid4().hex[:12]
    directory = Path(tempfile.mkdtemp(prefix=project+'-',dir=args.artifacts)).resolve()
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
    report = {'project':project,'result':'failed','native_source_sha':PIN,'source_archive_sha256':hashlib.sha256(archive).hexdigest(),
              'scenario':args.scenario, 'test_name':TEST_NAMES[args.scenario],
              'dependency_image_id':image['Id'],'image_user':image['Config']['User'],'base_sdk_sha':sdk_pin,'launcher_base_sha':base_head,
              'launcher_sha256':hashlib.sha256(launcher).hexdigest(),
              'fleet_head':git(ROOT,'rev-parse','HEAD').decode().strip(),
              'fleet_worktree_clean':not bool(git(ROOT,'status','--porcelain')),
              'fleet_test_source_sha256':hashlib.sha256((ROOT/'backend/infra/tests/native_supervisor_live.rs').read_bytes()).hexdigest(),
              'fleet_test_sources_sha256':{name:hashlib.sha256((ROOT/name).read_bytes()).hexdigest()
                  for name in ['backend/infra/tests/native_supervisor_live.rs',
                               'backend/infra/tests/support/native_approvals.rs']},
              'harness_sha256':{name:hashlib.sha256((scripts/name).read_bytes()).hexdigest() for name in ['run.py','build.sh','native.sh','preflight.py','discard_ack_plugin.py','approval_fault_plugin.py']}}
    if plugin is not None:
        plugin_dir = directory/'recovery-plugin'
        plugin_dir.mkdir()
        for name, body in plugin.items():
            (plugin_dir/name).write_bytes(body)
        plugin_hashes = {name:hashlib.sha256(body).hexdigest() for name,body in plugin.items()}
        (directory/'recovery-hashes.json').write_text(json.dumps(plugin_hashes)+'\n',encoding='utf-8')
        report['recovery_plugin_base_sha'] = base_head
        report['recovery_plugin_hashes'] = plugin_hashes
    postgres = service('disposable-native-postgresql')
    postgres.update(image='postgres:17.6-alpine',tmpfs=['/var/lib/postgresql/data:rw,size=512m'],
        environment={'POSTGRES_USER':'native_qa','POSTGRES_DB':'fleet_native_test','POSTGRES_HOST_AUTH_METHOD':'trust'},
        healthcheck={'test':['CMD','pg_isready','-U','native_qa','-d','fleet_native_test'],'interval':'1s','timeout':'2s','retries':30})
    build = service('rust-native-test-build')
    build.update(image='rust:1.88.0-bookworm',working_dir='/work/fleet-control/backend',entrypoint=['bash','/qa/build.sh'],
        environment={'RUSTUP_TOOLCHAIN':'1.88.0','CARGO_TARGET_DIR':'/cache/final','CARGO_BUILD_JOBS':'2',
            'CARGO_INCREMENTAL':'0','CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0'},
        volumes=[dict(bind(ROOT,'/work/fleet-control'),read_only=False),bind(args.base_sdk.resolve(),'/work/services-base'),
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
    if plugin is not None:
        native['environment']['FLEET_NATIVE_FAULT_ROOT'] = '/tmp/fleet-native-supervisor/recovery-fault'
        native['volumes'].extend([bind(plugin_dir,'/qa/recovery-plugin'),
            bind(directory/'recovery-hashes.json','/qa/recovery-hashes.json')])
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
        if (not re.search(rb'test result: ok\. 1 passed; 0 failed; 0 ignored;',result.stdout)
                or TEST_NAMES[args.scenario].encode() not in result.stdout
                or ('Exact pinned native tracked source verified: '+str(len(hashes))+' files').encode() not in result.stdout):
            raise RuntimeError('Native evidence is incomplete or no test ran')
        if plugin is not None and b'Exact committed recovery plugin verified: 4 files' not in result.stdout:
            raise RuntimeError('Native recovery plugin preflight is missing')
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
