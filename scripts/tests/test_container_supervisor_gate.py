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
