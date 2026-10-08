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
import time


ROOT = Path(__file__).resolve().parents[2]
CODE = Path(__file__).resolve().parent
TASK = 'fleet-container-supervisor'
PURPOSE = 'real-hermes-loaded-configuration'
PIN = 'bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3'
CONTROL_FILES = ('scripts/runtime_boundary.py', 'scripts/runtime_bootstrap.py',
                 'scripts/runtime_control.py')
BASE_FILES = (*CONTROL_FILES, 'deploy/fleet-hermes-container-launch.py',
              'deploy/runtime-boundary-qa/hermes_source.Dockerfile')
BASELINE_TEST = 'real_container_supervisor_isolates_chat_and_drains_configuration_replacement'
CUSTODY_TEST = 'controller_recovery::real_controller_startup_maintains_custody_without_granting_execution'
ACTIVATION_TEST = 'activation_recovery::real_candidate_running_crash_restores_effective_configuration'


def test_command(helper, phase=None):
    command = helper.command + ['exec', '-T']
    if phase is not None:
        if phase not in ('prepare', 'recover-1', 'expired', 'recover-2', 'freeze-1', 'freeze-2', 'stop'):
            raise RuntimeError('Unknown own custody phase')
        command += ['-e', 'FLEET_CONTAINER_RECOVERY_PHASE=' + phase,
                    '-e', 'RUST_LOG=infra::runtime::controller_recovery_worker=warn']
    return command + ['fleet-backend', '/out/fleet-container-live',
                      CUSTODY_TEST if phase else BASELINE_TEST, '--exact', '--ignored',
                      '--nocapture', '--test-threads=1']


def custody_snapshot(helper, checked, stopped=False):
    """Keep only immutable physical identity; never persist Docker env or mounts."""
    ids = helper.resources('container')
    items = json.loads(checked(helper.docker + ['container', 'inspect', *ids])) if ids else []
    result = {}
    for item in items:
        labels = item['Config']['Labels']
        if (labels.get('com.docker.compose.project') != helper.project
                or labels.get('sdlc.task') != TASK or labels.get('sdlc.purpose') != PURPOSE):
            raise RuntimeError('Foreign custody resource; no adoption')
        service = labels.get('com.docker.compose.service')
        if service == 'fleet-backend' or re.fullmatch(r'agent[12]-runtime-[a-f0-9]{32}', service or ''):
            state = item['State']
            is_stopped_agent = stopped and service != 'fleet-backend'
            if (service in result or type(state['Pid']) is not int
                    or (is_stopped_agent and (state['Running'] is not False
                                             or state['Pid'] != 0 or state['Status'] != 'exited'))
                    or (not is_stopped_agent and (state['Running'] is not True or state['Pid'] <= 0))):
                raise RuntimeError('Custody process identity does not match the requested state')
            result[service] = {key: item[key] for key in ('Id', 'Image')}
            result[service].update(pid=item['State']['Pid'], started_at=item['State']['StartedAt'])
    if len(result) != 3 or 'fleet-backend' not in result:
        raise RuntimeError('Custody requires exactly two real agent containers and Fleet')
    return result


def validate_stopped(before, after):
    if before.keys() != after.keys() or before['fleet-backend'] != after['fleet-backend']:
        raise RuntimeError('Namespace stop changed the current Fleet controller')
    for service, original in before.items():
        if service != 'fleet-backend' and (type(after[service].get('pid')) is not int
                                          or after[service] != dict(original, pid=0)):
            raise RuntimeError('Namespace stop replaced or restarted an original Hermes')


def validate_stop_evidence(value, native=False):
    positive = ('original_namespaces_exited', 'original_snapshots_unchanged',
                'native_journals_unchanged', 'read_only_exit_proof') if native else (
        'actual_startup_worker', 'current_owner_namespace_stop', 'competing_logical_controller_denied',
        'original_launch_identity_retained', 'immutable_stop_delivery', 'single_outcome_audit',
        'original_dispatch_transcript_content_unchanged', 'accepted_run_cancelled',
        'pending_approval_cancelled_without_grant', 'terminal_events_once', 'approval_target_unchanged')
    counts = {'agents': 2} if native else {'agents': 2, 'epoch': 3}
    if (not isinstance(value, dict) or value.get('state') != 'passed'
            or any(value.get(key) is not True for key in positive)
            or any(value.get(key) is not False for key in (
                'raw_receipts_persisted', 'resumed_execution', 'sdlc_acceptance'))
            or (not native and (type(value.get('uncertain_stop_readbacks')) is not int
                                or not 0 <= value['uncertain_stop_readbacks'] <= 2))
            or any(type(value.get(key)) is not int or value[key] != count for key, count in counts.items())):
        raise RuntimeError('Recovered namespace stop evidence is incomplete')


def validate_restart(before, after):
    if before.keys() != after.keys():
        raise RuntimeError('Controller restart replaced an agent generation')
    for service, original in before.items():
        current = after[service]
        if service != 'fleet-backend':
            if current != original:
                raise RuntimeError('Controller restart changed a surviving Hermes')
        elif (current['Id'] != original['Id'] or current['Image'] != original['Image']
              or current['pid'] == original['pid'] or current['started_at'] == original['started_at']):
            raise RuntimeError('An actual same-container controller restart was not proven')


def validate_custody_evidence(value, epoch=None, native=False, expired=False):
    positive = ('historical_receipt_unchanged', 'same_version_heartbeat_replay_read_only',
                'native_journals_unchanged') if native else (
        ('same_physical_start_cannot_take_over', 'expired_db_lease_not_revived',
         'native_run_waiting_for_approval') if expired else (
        'actual_startup_worker', 'native_live_observation', 'original_launches_unchanged',
        'native_run_waiting_for_approval', 'new_owner_execution_held',
        'competing_logical_controller_denied', 'message_replay_did_not_dispatch'))
    expected = {'sdlc_acceptance': False}
    counts = {}
    if native:
        counts['agents'] = 2
        expected.update(resumed_execution=False, raw_receipts_persisted=False,
                        native_expiry_checked=expired, native_live_observation=not expired)
    elif not expired:
        counts.update(agents=2, epoch=epoch, minimum_lease_version=4)
        expected['resumed_execution'] = False
    if (not isinstance(value, dict) or value.get('state') != 'passed'
            or any(value.get(key) is not True for key in positive)
            or any(value.get(key) is not result for key, result in expected.items())
            or any(type(value.get(key)) is not int or value[key] != result for key, result in counts.items())):
        raise RuntimeError('Actual controller custody evidence is incomplete')


def wait_for_custody_ready(helper, prepare, activation=False, crash_point='candidate-running'):
    if crash_point not in ('candidate-running', 'before-create'):
        raise RuntimeError('Unknown activation crash point')
    deadline = time.monotonic() + 240
    ready_path = '/controller/activation-ready.json' if activation else '/controller/custody-ready.json'
    candidate_running = crash_point == 'candidate-running'
    assertion = (f'assert v["state"]=="ready" and v["crash_point"]=={crash_point!r} '
                 f'and v["candidate_running"] is {candidate_running} '
                 f'and v["settlement_paused"] is {candidate_running} '
                 f'and v["preparation_paused"] is {not candidate_running} '
                 'and type(v["actual_model_calls"]) is int '
                 'and v["actual_model_calls"]==0') if activation else (
                 'assert v["state"]=="ready" and type(v["actual_model_calls"]) is int '
                 'and v["actual_model_calls"]==1 and v["native_waiting_for_approval"] is True')
    probe = (f'import json,pathlib,sys; p=pathlib.Path("{ready_path}"); '
             'sys.exit(3) if not p.exists() else None; v=json.loads(p.read_bytes()); '
             + assertion)
    timeouts = 0
    while True:
        if prepare.poll() is not None:
            raise RuntimeError('Custody preparation terminated before actual controller restart')
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise RuntimeError('Real Hermes approval-wait preparation deadline expired')
        try:
            result = subprocess.run(helper.command + ['exec', '-T', 'fleet-backend',
                'python3', '-I', '-B', '-c', probe], capture_output=True, timeout=min(15, remaining))
        except subprocess.TimeoutExpired:
            # Only the read-only probe repeats; the live preparation is never redispatched.
            timeouts += 1
            continue
        if prepare.poll() is not None:
            raise RuntimeError('Custody preparation terminated before actual controller restart')
        if time.monotonic() >= deadline:
            raise RuntimeError('Real Hermes approval-wait preparation deadline expired')
        if result.returncode == 0:
            return timeouts
        if result.returncode != 3:
            raise RuntimeError('Real Hermes approval-wait preparation was not proven')
        time.sleep(min(1, max(0, deadline - time.monotonic())))


def activation_snapshot(helper, checked):
    ids = helper.resources('container')
    items = json.loads(checked(helper.docker + ['container', 'inspect', *ids])) if ids else []
    result = {}
    for item in items:
        labels = item['Config']['Labels']
        if (labels.get('com.docker.compose.project') != helper.project
                or labels.get('sdlc.task') != TASK or labels.get('sdlc.purpose') != PURPOSE):
            raise RuntimeError('Foreign activation resource; no adoption')
        service = labels.get('com.docker.compose.service')
        if service == 'fleet-backend' or re.fullmatch(r'agent[12]-runtime-[a-f0-9]{32}', service or ''):
            state = item['State']
            if (service in result or type(state['Pid']) is not int
                    or (state['Running'] and state['Pid'] <= 0)
                    or (not state['Running'] and (state['Pid'] != 0 or state['Status'] != 'exited'))):
                raise RuntimeError('Activation physical identity is invalid')
            result[service] = {'id': item['Id'], 'image': item['Image'], 'pid': state['Pid'],
                               'started_at': state['StartedAt'], 'running': state['Running']}
    return result


def verify_activation_recovery(helper, directory, report, logged, checked, crash_point='candidate-running'):
    if crash_point not in ('candidate-running', 'before-create'):
        raise RuntimeError('Unknown activation crash point')
    report['state'] = 'failed'
    candidate_running = crash_point == 'candidate-running'
    generations = 3 if candidate_running else 2
    def command(phase):
        return helper.command + ['exec', '-T', '-e', 'FLEET_CONTAINER_ACTIVATION_PHASE=' + phase,
            '-e', 'FLEET_CONTAINER_ACTIVATION_CRASH_POINT=' + crash_point,
            '-e', 'RUST_LOG=infra::runtime::controller_recovery_worker=warn',
            'fleet-backend', '/out/fleet-container-live', ACTIVATION_TEST, '--exact', '--ignored',
            '--nocapture', '--test-threads=1']

    prepare = None
    try:
        with (directory / 'activation-prepare.log').open('wb') as stream:
            prepare = subprocess.Popen(command('prepare'), stdout=stream, stderr=subprocess.STDOUT)
            report['activation_probe_timeouts'] = wait_for_custody_ready(
                helper, prepare, activation=True, crash_point=crash_point)
            original = activation_snapshot(helper, checked)
            agents = {key: value for key, value in original.items() if key != 'fleet-backend'}
            if (len(agents) != generations
                    or sum(value['running'] for value in agents.values()) != generations - 1
                    or 'fleet-backend' not in original):
                raise RuntimeError('Activation crash requires the exact original/candidate/peer generations')
            logged(helper.command + ['restart', '-t', '1', 'fleet-backend'], 'activation-restart.log', 60)
            prepare.wait(timeout=30)
            if prepare.returncode == 0:
                raise RuntimeError('Activation process was not physically interrupted')
        restarted = activation_snapshot(helper, checked)
        if original.keys() != restarted.keys() or any(restarted[key] != value for key, value in agents.items()):
            raise RuntimeError('Fleet crash replaced or restarted an agent')
        before, after = original['fleet-backend'], restarted['fleet-backend']
        if (before['id'] != after['id'] or before['image'] != after['image']
                or before['pid'] == after['pid'] or before['started_at'] == after['started_at']):
            raise RuntimeError('Actual same-container Fleet restart was not proven')
        logged(command('recover'), 'activation-recover.log', 300)
        evidence = json.loads((directory / 'evidence/activation-report.json').read_bytes())
        positive = ('actual_rust_supervisor', 'actual_docker_hermes',
                    'effective_preserved', 'backup_bytes_restored', 'fresh_rollback_generation',
                    'loaded_previous_soul', 'peer_unchanged', 'original_namespaces_exited', 'rollback_audit_once')
        if (evidence.get('state') != 'passed' or any(evidence.get(key) is not True for key in positive)
                or evidence.get('crash_point') != crash_point
                or any(evidence.get(key) is not expected for key, expected in (
                    ('actual_candidate_running_crash', candidate_running),
                    ('actual_before_create_crash', not candidate_running),
                    ('lost_preparation_ack_readback', not candidate_running),
                    ('qa_settlement_barrier_released', candidate_running),
                    ('qa_preparation_barrier_released', not candidate_running)))
                or evidence.get('sdlc_acceptance') is not False
                or evidence.get('raw_credentials_persisted_in_evidence') is not False
                or type(evidence.get('initial_preparation_readbacks')) is not int
                or not 0 <= evidence['initial_preparation_readbacks'] <= 120
                or type(evidence.get('model_prompts')) is not int or evidence['model_prompts'] != 1):
            raise RuntimeError('Actual activation recovery evidence is incomplete')
        final = activation_snapshot(helper, checked)
        if (len(final) != generations + 2
                or not restarted.keys() <= final.keys()
                or any(final[key] != dict(value, pid=0, running=False) for key, value in agents.items())
                or any(value['running'] for key, value in final.items() if key != 'fleet-backend')
                or final['fleet-backend'] != after):
            raise RuntimeError('Original and rollback namespaces did not remain stopped')
        report.update(state='passed', actual_rust_supervisor=True, actual_docker_hermes=True,
                      activation_recovery=evidence, activation_physical_snapshots=[original, restarted, final],
                      activation_log_sha256=sha(directory / 'activation-recover.log'))
    finally:
        if prepare is not None and prepare.poll() is None:
            prepare.terminate()
            try:
                prepare.wait(timeout=10)
            except subprocess.TimeoutExpired:
                prepare.kill()
                prepare.wait(timeout=10)


def verify_controller_recovery(helper, directory, report, logged, checked, controller_stop=False):
    report['state'] = 'failed'
    prepare = None
    try:
        with (directory / 'custody-prepare.log').open('wb') as stream:
            prepare = subprocess.Popen(test_command(helper, 'prepare'), stdout=stream, stderr=subprocess.STDOUT)
            report['custody_readiness_probe_timeouts'] = wait_for_custody_ready(helper, prepare)
            original = custody_snapshot(helper, checked)
            logged(helper.command + ['restart', '-t', '1', 'fleet-backend'], 'custody-restart1.log', 60)
            prepare.wait(timeout=30)
            if prepare.returncode == 0:
                raise RuntimeError('Original Fleet process was not interrupted')
        first = custody_snapshot(helper, checked)
        validate_restart(original, first)
        evidence = {}
        for phase in ('recover-1', 'expired', 'recover-2'):
            if phase == 'expired':
                # The recovered CLI (and its Tokio workers) has exited. Use actual lease time.
                time.sleep(31)
            elif phase == 'recover-2':
                logged(helper.command + ['restart', '-t', '1', 'fleet-backend'], 'custody-restart2.log', 60)
                validate_restart(first, custody_snapshot(helper, checked))
            logged(test_command(helper, phase), 'custody-' + phase + '.log',
                   180 if phase.startswith('recover-') else 120)
            epoch = 2 if phase == 'recover-2' else 1
            suffix = 'expired' if phase == 'expired' else 'epoch' + str(epoch)
            value = json.loads((directory / 'evidence' / ('custody-' + suffix + '.json')).read_bytes())
            validate_custody_evidence(value, epoch=epoch, expired=phase == 'expired')
            evidence[suffix] = value
            if phase != 'expired':
                # Read the final exact DB version only after its worker has exited.
                freeze = 'freeze-' + str(epoch)
                logged(test_command(helper, freeze), 'custody-' + freeze + '.log', 30)
            command = helper.command + ['exec', '-T', 'fleet-backend', 'python3', '-I', '-B',
                '/qa-fixtures/custody_probe.py', '--epoch', str(epoch)]
            if phase == 'expired':
                command += ['--expired']
            logged(command, 'custody-native-' + suffix + '.log', 90)
            value = json.loads((directory / 'evidence' / ('custody-native-' + suffix + '.json')).read_bytes())
            validate_custody_evidence(value, native=True, expired=phase == 'expired')
            evidence['native-' + suffix] = value
        final = custody_snapshot(helper, checked)
        for service in original:
            if service != 'fleet-backend' and final[service] != original[service]:
                raise RuntimeError('Custody changed an original agent process')
        report.update(state='passed', actual_rust_supervisor=True, actual_docker_hermes=True,
                      controller_recovery=evidence, physical_snapshots=[original, first, final],
                      resumed_execution=False)
        if controller_stop:
            report['state'] = 'failed'
            time.sleep(31)
            logged(helper.command + ['restart', '-t', '1', 'fleet-backend'], 'custody-restart3.log', 60)
            third = custody_snapshot(helper, checked)
            validate_restart(final, third)
            logged(test_command(helper, 'stop'), 'custody-stop.log', 240)
            value = json.loads((directory / 'evidence/custody-stop.json').read_bytes())
            validate_stop_evidence(value)
            stopped = custody_snapshot(helper, checked, stopped=True)
            validate_stopped(third, stopped)
            logged(helper.command + ['exec', '-T', 'fleet-backend', 'python3', '-B',
                                    '/qa-fixtures/stop_probe.py'], 'custody-native-stop.log', 90)
            native = json.loads((directory / 'evidence/custody-native-stop.json').read_bytes())
            validate_stop_evidence(native, native=True)
            report.update(state='passed', controller_stop=value, controller_stop_native=native,
                          stop_physical_snapshots=[third, stopped])
    finally:
        if prepare is not None and prepare.poll() is None:
            # Stop only the already-owned Compose service, not an arbitrary host PID.
            checked(helper.command + ['stop', '-t', '1', 'fleet-backend'], timeout=60)
            prepare.wait(timeout=30)


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


def cleanup_nested(helper, directory, write_json, checked, remaining_passes=2):
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
            expected_image = spec['services'].get('source-check', {}).get('image')
            if expected_image is not None and item['Image'] != expected_image:
                raise RuntimeError('Generated agent image differs; cleanup refused')
            agent_services.add(service)
            spec['services'][service] = {'image': item['Image'], 'networks': [service]}
            # Late create may survive after its bridge was already removed. Down
            # still needs a declared network, but never creates the missing bridge.
            spec['networks'].setdefault(service, {'name': helper.project + '-' + service, 'internal': True})
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
    try:
        helper.close()
    except RuntimeError:
        # A timed-out native create can appear between the union snapshot and down.
        # Reinspect every identity before another bounded, cleanup-only Compose down.
        if remaining_passes and (helper.resources('container') or helper.resources('network')):
            return cleanup_nested(helper, directory, write_json, checked, remaining_passes - 1)
        raise
    for item in items['volume']:
        name = item['Name']
        if checked(helper.docker + ['ps', '-aq', '--filter', 'volume=' + name], text=True).strip():
            raise RuntimeError('Own disposable volume still in use; not removed')
        checked(helper.docker + ['volume', 'rm', name])
    if any(helper.resources(kind) for kind in ('container', 'network', 'volume')):
        raise RuntimeError('Own Compose resources remain after cleanup')


def verify_log_readback(helper, directory, report, logged):
    report['state'] = 'failed'
    logged(helper.command + ['exec', '-T', 'fleet-backend', 'python3', '-I', '-B',
                            '/qa-fixtures/log_readback.py'], 'log-readback.log', 180)
    logs = json.loads((directory / 'evidence/log-readback.json').read_bytes())
    if (logs.get('state') != 'passed' or logs.get('actual_base_log_readback') is not True
            or logs.get('fleet_log_ingestion') is not False or logs.get('raw_logs_persisted') is not False
            or logs.get('sdlc_acceptance') is not False or logs.get('agents') != 2
            or type(logs.get('original_exited_generations')) is not int
            or logs['original_exited_generations'] < 4
            or type(logs.get('nonempty_generations')) is not int
            or not 2 <= logs['nonempty_generations'] <= logs['original_exited_generations']):
        raise RuntimeError('Private Base log readback evidence is incomplete')
    report.update(state='passed', log_readback=logs,
                  log_readback_log_sha256=sha(directory / 'log-readback.log'))


def validate_live_evidence(live, readiness_rollback):
    positive = ('actual_rust_supervisor', 'actual_docker_hermes', 'controlled_model',
        'isolated_soul_and_mirror', 'cross_agent_token_denied', 'idempotent_messages',
        'drain_before_file_effects', 'loaded_replacement_soul', 'peer_unchanged',
        'fresh_restart_generation', 'confirmed_namespace_stop', 'native_provider_rotation',
        'peer_provider_unchanged', 'original_environment_custody')
    counts = {'agents': 2, 'controller_uid': 999,
              'model_prompts': 6 if readiness_rollback else 5,
              'original_environment_intents': 6 if readiness_rollback else 4}
    if (not isinstance(live, dict) or live.get('state') != 'passed'
            or any(live.get(key) is not True for key in positive)
            or live.get('sdlc_acceptance') is not False
            or live.get('raw_credentials_persisted_in_evidence') is not False
            or live.get('readiness_rollback') is not readiness_rollback
            or any(type(live.get(key)) is not int or live[key] != value
                   for key, value in counts.items())):
        raise RuntimeError('Actual Rust/Hermes evidence is incomplete')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('base-sdk', 'base-control', 'hermes-source', 'artifacts'):
        parser.add_argument('--' + name, type=Path, required=True)
    for name in ('rust-image', 'docker-image', 'hermes-image', 'postgres-image', 'context'):
        parser.add_argument('--' + name, required=True)
    parser.add_argument('--readiness-rollback', action='store_true',
                        help='Inject an owned candidate boot delay and verify actual Docker rollback')
    parser.add_argument('--log-readback', action='store_true',
                        help='Verify private Base log reads from the stopped original generations')
    parser.add_argument('--controller-recovery', action='store_true',
                        help='Prove actual startup custody across two controller restarts during approval-wait')
    parser.add_argument('--controller-stop', action='store_true',
                        help='After custody proof, restart a third time and stop both original agent namespaces')
    parser.add_argument('--activation-recovery', action='store_true',
                        help='Crash the real controller after candidate start, before configuration settlement')
    parser.add_argument('--activation-crash-point', choices=('candidate-running', 'before-create'),
                        help='Select the exact physical activation boundary (requires --activation-recovery)')
    args = parser.parse_args()
    if args.activation_crash_point is not None and not args.activation_recovery:
        parser.error('Activation crash point requires --activation-recovery')
    args.activation_crash_point = args.activation_crash_point or 'candidate-running'
    if args.controller_stop and not args.controller_recovery:
        parser.error('Recovered namespace stop requires the complete controller custody gate')
    if args.controller_recovery and (args.readiness_rollback or args.log_readback):
        parser.error('Controller custody requires a separate owned gate, not rollback/log mode')
    if args.activation_recovery and (args.controller_recovery or args.controller_stop
                                     or args.readiness_rollback or args.log_readback):
        parser.error('Activation recovery requires its own crash gate')
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
              'readiness_rollback_requested': args.readiness_rollback,
              'log_readback_requested': args.log_readback,
              'controller_recovery_requested': args.controller_recovery,
              'controller_stop_requested': args.controller_stop,
              'activation_recovery_requested': args.activation_recovery,
              'activation_crash_point': args.activation_crash_point if args.activation_recovery else None,
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
                    'FLEET_CONTROL_SECRET__LOCAL_MODEL_ROTATED': 'owned-local-model-rotated-fixture',
                    'FLEET_TEST_DATABASE_URL': f'postgresql://fleet_qa:{password}@postgres:5432/fleet_container'},
                'volumes': [volume('agents', '/agents'), volume('controller', '/controller'), volume('compiled', '/out', True),
                    bind(directory / 'input/base-control', '/base-control'), bind(directory / 'proof', '/qa'),
                    bind(frozen, '/qa-fixtures'),
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
        # Component fixtures use a distinct database; never consume the live gate's ordinals.
        logged(helper.command + ['exec', '-T', 'postgres', 'psql', '-v', 'ON_ERROR_STOP=1',
            '-U', 'fleet_qa', '-d', 'fleet_container', '-c', 'CREATE DATABASE fleet_components'],
            'component-database.log')
        logged(helper.command + ['exec', '-T', '-e',
            f'FLEET_TEST_DATABASE_URL=postgresql://fleet_qa:{password}@postgres:5432/fleet_components',
            'fleet-backend', '/out/fleet-runtime-tests', 'runtime::', '--nocapture', '--test-threads=1'],
            'runtime-components.log', 600)
        component_log = (directory / 'runtime-components.log').read_text(encoding='utf-8')
        if (not re.search(r'test result: ok\. [1-9][0-9]* passed; 0 failed;', component_log)
                or 'test runtime::container_lifecycle_tests::pending_preparation_does_not_adopt_foreign_owner_or_replace_lost_claim ... ok'
                not in component_log):
            raise RuntimeError('Real PostgreSQL runtime component coverage is missing')
        report['runtime_components_log_sha256'] = sha(directory / 'runtime-components.log')
        cid = checked(helper.command + ['ps', '-q', 'fleet-backend'], text=True).strip()
        item = json.loads(checked(docker + ['container', 'inspect', cid]))[0]
        if item['Image'] != controller_image or not item['State']['Running']:
            raise RuntimeError('Live Fleet controller identity differs')
        write_json(directory / 'proof/controller-proof.json', {
            'engine_id': helper.identity, 'agents_volume': project + '_agents', 'model_host': item['Name'].lstrip('/'),
            'readiness_rollback': args.readiness_rollback,
            'control': {'python': 'python3', 'base_root': '/base-control', 'context': 'default',
                'source_sha256': [sha(directory / 'input/base-control' / name) for name in CONTROL_FILES],
                'provisioning': {'project': project, 'image_id': hermes_image, 'user': '999:999',
                    'entrypoint': ['/opt/hermes/.venv/bin/python', '/runtime/readiness-fault.py'
                                   if args.readiness_rollback else '/runtime/hermes-container.py'],
                    'pids_limit': 128, 'memory_bytes': 1073741824, 'nano_cpus': 1000000000,
                    'network_internal': True, 'task': TASK, 'purpose': PURPOSE},
                'bridge_controller': {'container_id': cid, 'image_id': controller_image, 'service': 'fleet-backend'}}})
        if args.activation_recovery:
            verify_activation_recovery(helper, directory, report, logged, checked, args.activation_crash_point)
            report['build_log_sha256'] = sha(directory / 'build.log')
        elif args.controller_recovery:
            verify_controller_recovery(helper, directory, report, logged, checked, args.controller_stop)
            report['build_log_sha256'] = sha(directory / 'build.log')
        else:
            logged(test_command(helper), 'live.log', 1200)
            live = json.loads((directory / 'evidence/live-report.json').read_bytes())
            validate_live_evidence(live, args.readiness_rollback)
            report.update(state='passed', actual_rust_supervisor=True, actual_docker_hermes=True,
                          live=live, build_log_sha256=sha(directory / 'build.log'), live_log_sha256=sha(directory / 'live.log'))
            if args.log_readback:
                verify_log_readback(helper, directory, report, logged)
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
