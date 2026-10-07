"""Owned live-gate safety tests; no Docker daemon, runtime or credentials."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch


ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('container_gate', ROOT / 'scripts/container_supervisor_live/run.py')
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)


class SafetyTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.home = Path(self.temporary.name)
        self.project = 'sdlc-qa-fleet-container-live-aaaaaaaaaaaa'
        self.path = self.home / 'compose.json'
        self.path.write_text(json.dumps({'services': {'fleet-backend': {'image': 'sha256:' + '1' * 64}},
                                        'networks': {'fleet': {'name': self.project + '_fleet'}},
                                        'volumes': {'agents': {'name': self.project + '_agents'}}}))
        self.labels = {'sdlc.task': runner.TASK, 'sdlc.purpose': runner.PURPOSE,
                       'com.docker.compose.project': self.project}
        self.items = {kind: [] for kind in ('container', 'network', 'volume')}
        self.effects = []
        self.helper = SimpleNamespace(project=self.project, docker=['docker', '--context', 'owned-test'],
            path=self.path, manifest_hash=runner.sha(self.path),
            definitions={'network': {'fleet': {'name': self.project + '_fleet'}},
                         'volume': {'agents': {'name': self.project + '_agents'}}},
            check_endpoint=lambda: self.effects.append('endpoint-checked'),
            resources=lambda kind: [item.get('Id', item.get('Name')) for item in self.items[kind]],
            close=lambda: self.effects.append('helper-closed'))

    def checked(self, command, **kwargs):
        args = command[3:]
        if args[0] in self.items and args[1] == 'inspect':
            return json.dumps(self.items[args[0]]).encode()
        if args[0] == 'compose':
            self.effects.append(command)
            self.items['container'] = []
            self.items['network'] = []
            return b''
        if args[:2] == ['volume', 'rm']:
            self.effects.append(command)
            self.items['volume'] = [item for item in self.items['volume'] if item['Name'] != args[2]]
            return b''
        if args[:2] == ['ps', '-aq']:
            return ''
        raise AssertionError('Unexpected maintenance command')

    def cleanup(self):
        runner.cleanup_nested(self.helper, self.home,
            lambda path, value: path.write_text(json.dumps(value)), self.checked)

    def container(self, service, purpose=None):
        labels = dict(self.labels, **{'com.docker.compose.service': service})
        if purpose:
            labels['sdlc.purpose'] = purpose
        if service == 'fleet-backend':
            labels['com.docker.compose.project.config_files'] = str(self.path)
        return {'Id': 'c1', 'Image': 'sha256:' + '1' * 64, 'Config': {'Labels': labels}}

    def test_union_cleanup_accepts_only_exact_owned_agent_generation(self):
        service = 'agent1-runtime-' + 'a' * 32
        self.items['container'] = [self.container('fleet-backend'), self.container(service)]
        self.items['network'] = [{'Id': 'n1', 'Name': self.project + '-' + service,
                                  'Labels': dict(self.labels, **{'com.docker.compose.network': service})}]
        self.items['volume'] = [{'Name': self.project + '_agents', 'Labels': self.labels}]
        self.cleanup()
        manifest = json.loads((self.home / 'cleanup.json').read_text())
        self.assertIn(service, manifest['services'])
        self.assertIn(service, manifest['networks'])
        self.assertFalse(any(self.items.values()))
        self.assertIn('helper-closed', self.effects)
        maintenance = [effect for effect in self.effects if isinstance(effect, list)]
        self.assertEqual(maintenance[0][-2:], ['down', '--remove-orphans'])
        self.assertEqual(maintenance[1][-3:], ['volume', 'rm', self.project + '_agents'])

    def test_foreign_purpose_refuses_cleanup_before_side_effects(self):
        self.items['container'] = [self.container('fleet-backend', 'some-other-task')]
        with self.assertRaisesRegex(RuntimeError, 'Foreign resource'):
            self.cleanup()
        self.assertEqual(self.effects, ['endpoint-checked'])

    def test_orphan_owned_network_is_referenced_by_cleanup_manifest(self):
        service = 'agent2-runtime-' + 'b' * 32
        self.items['network'] = [{'Id': 'n2', 'Name': self.project + '-' + service,
                                  'Labels': dict(self.labels, **{'com.docker.compose.network': service})}]
        self.cleanup()
        manifest = json.loads((self.home / 'cleanup.json').read_text())
        self.assertEqual(manifest['services'][service]['networks'], [service])

    def test_unknown_agent_service_refuses_cleanup(self):
        self.items['container'] = [self.container('agent3-runtime-' + 'a' * 32)]
        with self.assertRaisesRegex(RuntimeError, 'Unexpected container'):
            self.cleanup()
        self.assertEqual(self.effects, ['endpoint-checked'])

    def test_changed_manifest_refuses_all_effects(self):
        self.path.write_text('{}')
        with self.assertRaisesRegex(RuntimeError, 'manifest changed'):
            self.cleanup()
        self.assertEqual(self.effects, ['endpoint-checked'])

    def test_undeclared_volume_is_not_adopted(self):
        self.items['volume'] = [{'Name': self.project + '_protected', 'Labels': self.labels}]
        with self.assertRaisesRegex(RuntimeError, 'Unexpected volume'):
            self.cleanup()
        self.assertEqual(self.effects, ['endpoint-checked'])

    def test_volume_still_used_is_not_deleted(self):
        self.items['volume'] = [{'Name': self.project + '_agents', 'Labels': self.labels}]
        original = self.checked

        def checked(command, **kwargs):
            return 'some-container' if command[3:5] == ['ps', '-aq'] else original(command, **kwargs)

        with self.assertRaisesRegex(RuntimeError, 'still in use'):
            runner.cleanup_nested(self.helper, self.home,
                lambda path, value: path.write_text(json.dumps(value)), checked)
        self.assertEqual(len(self.items['volume']), 1)
        self.assertFalse(any(isinstance(effect, list) and effect[3:5] == ['volume', 'rm']
                             for effect in self.effects))

    def test_capture_uses_frozen_exact_bytes_and_filters_scope(self):
        original = self.home / 'source'
        original.mkdir()
        (original / 'keep.rs').write_bytes(b'owned source\n')
        (original / 'unrelated.txt').write_bytes(b'unrelated')
        manifest, paths = {}, {}
        with patch.object(runner, 'git', return_value=b'keep.rs\0unrelated.txt\0keep.rs\0'):
            runner.capture(original, ('keep.rs',), self.home / 'frozen', manifest, paths)
        self.assertEqual(manifest, {'frozen/keep.rs': hashlib.sha256(b'owned source\n').hexdigest()})
        self.assertEqual((self.home / 'frozen/keep.rs').read_bytes(), b'owned source\n')
        self.assertFalse((self.home / 'frozen/unrelated.txt').exists())


class LiveEvidenceTests(unittest.TestCase):
    def evidence(self, rollback=False):
        result = {key: True for key in ('actual_rust_supervisor', 'actual_docker_hermes',
            'controlled_model', 'isolated_soul_and_mirror', 'cross_agent_token_denied',
            'idempotent_messages', 'drain_before_file_effects', 'loaded_replacement_soul',
            'peer_unchanged', 'fresh_restart_generation', 'confirmed_namespace_stop',
            'native_provider_rotation', 'peer_provider_unchanged', 'original_environment_custody')}
        result.update(state='passed', sdlc_acceptance=False, readiness_rollback=rollback,
            raw_credentials_persisted_in_evidence=False, agents=2, controller_uid=999,
            model_prompts=6 if rollback else 5, original_environment_intents=6 if rollback else 4)
        return result

    def test_baseline_and_rollback_require_distinct_exact_custody_and_prompt_counts(self):
        for rollback in (False, True):
            runner.validate_live_evidence(self.evidence(rollback), rollback)
            with self.assertRaisesRegex(RuntimeError, 'evidence is incomplete'):
                runner.validate_live_evidence(self.evidence(not rollback), rollback)

    def test_missing_false_mistyped_counts_or_overclaimed_evidence_cannot_pass(self):
        for key, value in self.evidence().items():
            for invalid in (None, 'true', not value if type(value) is bool else 0):
                with self.subTest(key=key, value=invalid):
                    evidence = self.evidence()
                    evidence[key] = invalid
                    with self.assertRaisesRegex(RuntimeError, 'evidence is incomplete'):
                        runner.validate_live_evidence(evidence, False)
        for key in ('agents', 'controller_uid', 'model_prompts', 'original_environment_intents'):
            evidence = self.evidence()
            evidence[key] = True
            with self.assertRaisesRegex(RuntimeError, 'evidence is incomplete'):
                runner.validate_live_evidence(evidence, False)
        for invalid in (None, [], 'passed'):
            with self.assertRaisesRegex(RuntimeError, 'evidence is incomplete'):
                runner.validate_live_evidence(invalid, False)


class CustodyTests(unittest.TestCase):
    def evidence(self, native=False, expired=False, epoch=1):
        value = dict(state='passed', sdlc_acceptance=False)
        if native:
            value.update(agents=2, historical_receipt_unchanged=True,
                same_version_heartbeat_replay_read_only=True, native_journals_unchanged=True,
                native_expiry_checked=expired, native_live_observation=not expired,
                raw_receipts_persisted=False, resumed_execution=False)
        elif expired:
            value.update(same_physical_start_cannot_take_over=True,
                expired_db_lease_not_revived=True, native_run_waiting_for_approval=True)
        else:
            value.update(agents=2, epoch=epoch, minimum_lease_version=4, actual_startup_worker=True,
                native_live_observation=True, original_launches_unchanged=True,
                native_run_waiting_for_approval=True, new_owner_execution_held=True,
                competing_logical_controller_denied=True, message_replay_did_not_dispatch=True,
                resumed_execution=False)
        return value

    def test_each_selected_case_is_exact_and_cannot_run_all_ignored_cases(self):
        helper = SimpleNamespace(command=['docker', 'compose', '-p', 'sdlc-qa-owned'])
        for phase in (None, 'prepare', 'recover-1', 'expired', 'recover-2', 'freeze-1', 'freeze-2', 'stop'):
            command = runner.test_command(helper, phase)
            self.assertEqual(command[-5:], [runner.CUSTODY_TEST if phase else runner.BASELINE_TEST,
                '--exact', '--ignored', '--nocapture', '--test-threads=1'])
            if phase:
                self.assertIn('FLEET_CONTAINER_RECOVERY_PHASE=' + phase, command)
        with self.assertRaisesRegex(RuntimeError, 'Unknown own custody phase'):
            runner.test_command(helper, 'foreign')

    def test_all_phase_evidence_is_required_and_never_claims_resumed_execution(self):
        for native, expired, epoch in ((False, False, 1), (False, False, 2),
                                      (False, True, 1), (True, False, 1), (True, True, 1)):
            value = self.evidence(native, expired, epoch)
            runner.validate_custody_evidence(value, epoch, native, expired)
            for key in value:
                for invalid in (None, 'invalid', True if type(value[key]) is int else not value[key]):
                    with self.subTest(native=native, expired=expired, epoch=epoch, key=key):
                        broken = dict(value, **{key: invalid})
                        with self.assertRaisesRegex(RuntimeError, 'custody evidence is incomplete'):
                            runner.validate_custody_evidence(broken, epoch, native, expired)

    def test_same_controller_container_must_actually_restart_and_agents_must_survive(self):
        original = {'fleet-backend': dict(Id='fleet', Image='image', pid=10, started_at='first'),
                    'agent1-runtime-a': dict(Id='one', Image='hermes', pid=20, started_at='agent-one'),
                    'agent2-runtime-b': dict(Id='two', Image='hermes', pid=30, started_at='agent-two')}
        current = {key: dict(value) for key, value in original.items()}
        current['fleet-backend'].update(pid=11, started_at='second')
        runner.validate_restart(original, current)
        for service, key, value in (('fleet-backend', 'Id', 'replacement'),
                                    ('fleet-backend', 'Image', 'candidate'),
                                    ('fleet-backend', 'pid', 10),
                                    ('fleet-backend', 'started_at', 'first'),
                                    ('agent1-runtime-a', 'pid', 21),
                                    ('agent2-runtime-b', 'started_at', 'restarted')):
            broken = {name: dict(item) for name, item in current.items()}
            broken[service][key] = value
            with self.assertRaises(RuntimeError):
                runner.validate_restart(original, broken)

    def test_prepare_failure_cannot_retain_success_and_closes_only_own_service(self):
        with tempfile.TemporaryDirectory() as folder:
            helper = SimpleNamespace(command=['docker', 'compose', '-p', 'sdlc-qa-owned'])
            report = dict(state='passed')
            process = SimpleNamespace(poll=lambda: None, wait=lambda **_: None)
            result = SimpleNamespace(returncode=1)
            with patch.object(runner.subprocess, 'Popen', return_value=process), \
                    patch.object(runner.subprocess, 'run', return_value=result), \
                    patch.object(runner.time, 'sleep'), \
                    patch.object(runner, 'custody_snapshot') as snapshot:
                effects = []
                with self.assertRaisesRegex(RuntimeError, 'preparation was not proven'):
                    runner.verify_controller_recovery(helper, Path(folder), report,
                        lambda *_: self.fail('Unexpected phase'),
                        lambda command, **_: effects.append(command))
                self.assertEqual(report['state'], 'failed')
                self.assertEqual(effects, [helper.command + ['stop', '-t', '1', 'fleet-backend']])
                snapshot.assert_not_called()

    def test_success_requires_both_real_restarts_and_every_rust_and_native_phase(self):
        with tempfile.TemporaryDirectory() as folder:
            home = Path(folder)
            (home / 'evidence').mkdir()
            helper = SimpleNamespace(command=['docker', 'compose', '-p', 'sdlc-qa-owned'])
            alive = [True]
            process = SimpleNamespace(poll=lambda: None if alive[0] else 137,
                                      wait=lambda **_: None, returncode=137)
            original = {'fleet-backend': dict(Id='fleet', Image='image', pid=10, started_at='first'),
                        'agent1-runtime-a': dict(Id='one', Image='hermes', pid=20, started_at='agent-one'),
                        'agent2-runtime-b': dict(Id='two', Image='hermes', pid=30, started_at='agent-two')}
            first = {key: dict(item) for key, item in original.items()}
            first['fleet-backend'].update(pid=11, started_at='second')
            second = {key: dict(item) for key, item in first.items()}
            second['fleet-backend'].update(pid=12, started_at='third')
            effects, report = [], dict(state='passed')

            def logged(command, name, timeout):
                self.assertEqual(report['state'], 'failed')
                effects.append(command)
                if 'restart' in command:
                    alive[0] = False
                    return
                if any(item in command for item in ('FLEET_CONTAINER_RECOVERY_PHASE=freeze-1',
                                                    'FLEET_CONTAINER_RECOVERY_PHASE=freeze-2')):
                    return
                native = '/qa-fixtures/custody_probe.py' in command
                expired = '--expired' in command if native else 'FLEET_CONTAINER_RECOVERY_PHASE=expired' in command
                epoch = int(command[-1]) if native and not expired else (
                    2 if 'FLEET_CONTAINER_RECOVERY_PHASE=recover-2' in command else 1)
                suffix = 'expired' if expired else 'epoch' + str(epoch)
                prefix = 'custody-native-' if native else 'custody-'
                (home / 'evidence' / (prefix + suffix + '.json')).write_text(
                    json.dumps(self.evidence(native, expired, epoch)))

            with patch.object(runner.subprocess, 'Popen', return_value=process), \
                    patch.object(runner.subprocess, 'run', return_value=SimpleNamespace(returncode=0)), \
                    patch.object(runner.time, 'sleep') as sleep, \
                    patch.object(runner, 'custody_snapshot', side_effect=[original, first, second, second]):
                runner.verify_controller_recovery(helper, home, report, logged,
                    lambda *_: self.fail('Unexpected forced stop'))
            self.assertEqual(report['state'], 'passed')
            self.assertFalse(report['resumed_execution'])
            self.assertEqual(len(report['controller_recovery']), 6)
            self.assertEqual(sum('restart' in command for command in effects), 2)
            self.assertEqual(len(effects), 10)
            self.assertEqual(sum(any(item in command for item in (
                'FLEET_CONTAINER_RECOVERY_PHASE=freeze-1',
                'FLEET_CONTAINER_RECOVERY_PHASE=freeze-2')) for command in effects), 2)
            sleep.assert_called_once_with(31)


class RecoveredStopTests(unittest.TestCase):
    def evidence(self, native=False):
        positive = ('original_namespaces_exited', 'original_snapshots_unchanged',
            'native_journals_unchanged', 'read_only_exit_proof') if native else (
            'actual_startup_worker', 'current_owner_namespace_stop', 'competing_logical_controller_denied',
            'original_launch_identity_retained', 'immutable_stop_delivery', 'single_outcome_audit',
            'session_dispatch_transcript_approval_unchanged', 'approval_target_unchanged')
        value = dict.fromkeys(positive, True)
        value.update(state='passed', agents=2, raw_receipts_persisted=False,
                     resumed_execution=False, sdlc_acceptance=False)
        if not native:
            value['epoch'] = 3
            value['uncertain_stop_readbacks'] = 0
        return value

    def test_stop_evidence_cannot_overclaim_execution_or_omit_any_assertion(self):
        for native in (False, True):
            value = self.evidence(native)
            runner.validate_stop_evidence(value, native)
            for key in value:
                for invalid in (None, 'true', True if type(value[key]) is int else not value[key]):
                    with self.subTest(native=native, key=key, invalid=invalid):
                        with self.assertRaisesRegex(RuntimeError, 'stop evidence is incomplete'):
                            runner.validate_stop_evidence(dict(value, **{key: invalid}), native)
        for valid in (1, 2):
            runner.validate_stop_evidence(dict(self.evidence(), uncertain_stop_readbacks=valid))
        for invalid in (-1, 3):
            with self.assertRaisesRegex(RuntimeError, 'stop evidence is incomplete'):
                runner.validate_stop_evidence(dict(self.evidence(), uncertain_stop_readbacks=invalid))

    def test_stop_requires_original_container_image_and_start_with_zero_pid(self):
        before = {'fleet-backend': dict(Id='fleet', Image='image', pid=12, started_at='third'),
                  'agent1-runtime-a': dict(Id='one', Image='hermes', pid=20, started_at='one'),
                  'agent2-runtime-b': dict(Id='two', Image='hermes', pid=30, started_at='two')}
        after = {key: dict(item, pid=0) if key != 'fleet-backend' else dict(item)
                 for key, item in before.items()}
        runner.validate_stopped(before, after)
        for service, key, value in (('fleet-backend', 'pid', 13),
                                   ('agent1-runtime-a', 'Id', 'replacement'),
                                   ('agent1-runtime-a', 'Image', 'different'),
                                   ('agent2-runtime-b', 'started_at', 'restarted'),
                                   ('agent2-runtime-b', 'pid', False),
                                   ('agent2-runtime-b', 'pid', 30)):
            broken = {name: dict(item) for name, item in after.items()}
            broken[service][key] = value
            with self.assertRaises(RuntimeError):
                runner.validate_stopped(before, broken)

    def test_stopped_snapshot_requires_engine_exit_not_a_zero_pid_running_namespace(self):
        helper = SimpleNamespace(project='owned', docker=['docker'], resources=lambda _: ['fleet', 'one', 'two'])
        items = []
        for service in ('fleet-backend', 'agent1-runtime-' + 'a' * 32, 'agent2-runtime-' + 'b' * 32):
            items.append(dict(Id=service, Image='image', Config={'Labels': {
                'com.docker.compose.project': 'owned', 'sdlc.task': runner.TASK,
                'sdlc.purpose': runner.PURPOSE, 'com.docker.compose.service': service}},
                State=dict(Running=service == 'fleet-backend', Pid=10 if service == 'fleet-backend' else 0,
                           Status='running' if service == 'fleet-backend' else 'exited', StartedAt='original')))
        checked = lambda *_: json.dumps(items)
        runner.custody_snapshot(helper, checked, stopped=True)
        for key, value in (('Running', True), ('Pid', False), ('Pid', 1), ('Status', 'dead')):
            original = dict(items[1]['State'])
            items[1]['State'][key] = value
            with self.assertRaisesRegex(RuntimeError, 'requested state'):
                runner.custody_snapshot(helper, checked, stopped=True)
            items[1]['State'] = original

    def test_stop_gate_requires_third_restart_and_both_stop_proofs_after_all_custody_phases(self):
        for native_failure in (False, True):
            with self.subTest(native_failure=native_failure), tempfile.TemporaryDirectory() as folder:
                home = Path(folder)
                (home / 'evidence').mkdir()
                helper = SimpleNamespace(command=['docker', 'compose', '-p', 'sdlc-qa-owned'])
                alive = [True]
                process = SimpleNamespace(poll=lambda: None if alive[0] else 137,
                                          wait=lambda **_: None, returncode=137)
                original = {'fleet-backend': dict(Id='fleet', Image='image', pid=10, started_at='zero'),
                            'agent1-runtime-a': dict(Id='one', Image='hermes', pid=20, started_at='one'),
                            'agent2-runtime-b': dict(Id='two', Image='hermes', pid=30, started_at='two')}
                snapshots = [original]
                for epoch in (1, 2):
                    current = {key: dict(item) for key, item in original.items()}
                    current['fleet-backend'].update(pid=10 + epoch, started_at=str(epoch))
                    snapshots.append(current)
                third = {key: dict(item) for key, item in original.items()}
                third['fleet-backend'].update(pid=13, started_at='three')
                stopped = {key: dict(item, pid=0) if key != 'fleet-backend' else dict(item)
                           for key, item in third.items()}
                effects, report = [], dict(state='passed')

                def logged(command, name, timeout):
                    self.assertEqual(report['state'], 'failed')
                    effects.append(command)
                    if 'restart' in command:
                        alive[0] = False
                        return
                    if 'FLEET_CONTAINER_RECOVERY_PHASE=stop' in command:
                        value = self.evidence()
                        target = 'custody-stop.json'
                    elif '/qa-fixtures/stop_probe.py' in command:
                        value = self.evidence(native=True)
                        value['native_journals_unchanged'] = not native_failure
                        target = 'custody-native-stop.json'
                    else:
                        if any(item.startswith('FLEET_CONTAINER_RECOVERY_PHASE=freeze-') for item in command):
                            return
                        native = '/qa-fixtures/custody_probe.py' in command
                        expired = '--expired' in command if native else 'FLEET_CONTAINER_RECOVERY_PHASE=expired' in command
                        epoch = int(command[-1]) if native and not expired else (
                            2 if 'FLEET_CONTAINER_RECOVERY_PHASE=recover-2' in command else 1)
                        value = CustodyTests().evidence(native, expired, epoch)
                        suffix = 'expired' if expired else 'epoch' + str(epoch)
                        target = ('custody-native-' if native else 'custody-') + suffix + '.json'
                    (home / 'evidence' / target).write_text(json.dumps(value))

                with patch.object(runner.subprocess, 'Popen', return_value=process), \
                        patch.object(runner.subprocess, 'run', return_value=SimpleNamespace(returncode=0)), \
                        patch.object(runner.time, 'sleep') as sleep, \
                        patch.object(runner, 'custody_snapshot', side_effect=[
                            *snapshots, snapshots[-1], third, stopped]) as snapshot:
                    if native_failure:
                        with self.assertRaisesRegex(RuntimeError, 'stop evidence is incomplete'):
                            runner.verify_controller_recovery(helper, home, report, logged,
                                lambda *_: self.fail('Unexpected forced stop'), controller_stop=True)
                    else:
                        runner.verify_controller_recovery(helper, home, report, logged,
                            lambda *_: self.fail('Unexpected forced stop'), controller_stop=True)
                    self.assertEqual(sleep.call_args_list, [unittest.mock.call(31), unittest.mock.call(31)])
                    self.assertEqual(snapshot.call_args_list[-1].kwargs, {'stopped': True})
                self.assertEqual(len(report['controller_recovery']), 6)
                self.assertEqual(sum('restart' in command for command in effects), 3)
                self.assertEqual(len(effects), 13)
                self.assertEqual(report['state'], 'failed' if native_failure else 'passed')
                if native_failure:
                    self.assertNotIn('controller_stop', report)
                else:
                    self.assertEqual(report['stop_physical_snapshots'], [third, stopped])
                    self.assertFalse(report['controller_stop']['sdlc_acceptance'])


class LogReadbackTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.home = Path(self.temporary.name)
        (self.home / 'evidence').mkdir()
        self.helper = SimpleNamespace(command=['docker', '--context', 'owned-test',
            'compose', '-p', 'sdlc-qa-owned', '-f', '/owned/compose.json'])
        self.report = {'state': 'passed', 'actual_rust_supervisor': True}
        self.evidence = {'state': 'passed', 'actual_base_log_readback': True, 'agents': 2,
            'original_exited_generations': 4, 'nonempty_generations': 2,
            'raw_logs_persisted': False, 'fleet_log_ingestion': False, 'sdlc_acceptance': False}

    def logged(self, command, name, timeout):
        self.assertEqual(self.report['state'], 'failed')
        self.assertEqual(command, self.helper.command + ['exec', '-T', 'fleet-backend',
            'python3', '-I', '-B', '/qa-fixtures/log_readback.py'])
        self.assertEqual((name, timeout), ('log-readback.log', 180))
        (self.home / name).write_bytes(b'bounded private probe\n')
        (self.home / 'evidence/log-readback.json').write_text(json.dumps(self.evidence))

    def test_probe_uses_existing_compose_command_and_publishes_only_verified_evidence(self):
        runner.verify_log_readback(self.helper, self.home, self.report, self.logged)
        self.assertEqual(self.report['state'], 'passed')
        self.assertEqual(self.report['log_readback'], self.evidence)
        self.assertEqual(self.report['log_readback_log_sha256'],
            hashlib.sha256(b'bounded private probe\n').hexdigest())

    def test_probe_failure_cannot_retain_baseline_success(self):
        with self.assertRaisesRegex(RuntimeError, 'probe failed'):
            runner.verify_log_readback(self.helper, self.home, self.report,
                lambda *_: (_ for _ in ()).throw(RuntimeError('probe failed')))
        self.assertEqual(self.report['state'], 'failed')
        self.assertNotIn('log_readback', self.report)

    def test_missing_or_insufficient_or_overclaimed_evidence_never_passes(self):
        for key, value in (('state', 'failed'), ('actual_base_log_readback', False),
                           ('agents', 1), ('original_exited_generations', 3),
                           ('nonempty_generations', 0), ('nonempty_generations', True),
                           ('raw_logs_persisted', True), ('fleet_log_ingestion', True),
                           ('sdlc_acceptance', True)):
            with self.subTest(key=key, value=value):
                original = dict(self.evidence)
                self.evidence[key] = value
                with self.assertRaisesRegex(RuntimeError, 'evidence is incomplete'):
                    runner.verify_log_readback(self.helper, self.home, self.report, self.logged)
                self.assertEqual(self.report['state'], 'failed')
                self.assertNotIn('log_readback', self.report)
                self.evidence = original


class ReadinessFaultTests(unittest.TestCase):
    def setUp(self):
        source = ROOT / 'scripts/container_supervisor_live/readiness_fault.py'
        spec = importlib.util.spec_from_file_location('readiness_fault', source)
        self.fault = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.fault)

    def test_ordinary_boot_delegates_without_delay_and_preserves_arguments(self):
        args = ['qa.py', 'serve', '--host', '0.0.0.0', '--port', '29105']
        with patch.object(self.fault.Path, 'read_text', return_value='Normal SOUL'), \
                patch.object(self.fault.time, 'sleep') as sleep, \
                patch.object(self.fault.os, 'execv') as execute, \
                patch.object(self.fault.sys, 'argv', args):
            self.fault.main()
        sleep.assert_not_called()
        execute.assert_called_once_with(self.fault.sys.executable,
            [self.fault.sys.executable, '/runtime/hermes-container.py', *args[1:]])

    def test_marker_delays_boot_before_original_launcher_exec(self):
        effects = []
        with patch.object(self.fault.Path, 'read_text', return_value='CONTAINER_QA_READINESS_DELAY'), \
                patch.object(self.fault.time, 'sleep', side_effect=lambda seconds: effects.append(seconds)), \
                patch.object(self.fault.os, 'execv', side_effect=lambda *_: effects.append('exec')), \
                patch('builtins.print'):
            self.fault.main()
        self.assertEqual(effects, [120, 'exec'])

    def test_unreadable_fixture_configuration_never_executes(self):
        with patch.object(self.fault.Path, 'read_text', side_effect=OSError('unreadable')), \
                patch.object(self.fault.os, 'execv') as execute:
            with self.assertRaises(OSError):
                self.fault.main()
        execute.assert_not_called()


if __name__ == '__main__':
    unittest.main()
