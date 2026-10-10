import importlib.util
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('observer_start_fault', Path(__file__).with_name('observer_start_fault.py'))
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)


class ObserverStartupTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.home = self.root / 'agent1' / 'config'
        self.home.mkdir(parents=True)
        self.marker = self.home / '.qa-fail-next-launch'
        self.opt_in = self.root / 'readonly-fixture-opt-in'
        self.opt_in.write_bytes(fixture.OPT_IN_CONTENT)
        self.environment = {'HERMES_HOME': str(self.home)}

    def run_fixture(self, environment=None):
        with patch.object(fixture, 'AGENTS_ROOT', self.root), patch.object(fixture, 'OPT_IN', self.opt_in), patch.dict(os.environ, environment or self.environment, clear=True):
            fixture.main()

    def test_opt_in_and_owned_home_are_required_without_exec(self):
        self.opt_in.unlink()
        with patch.object(fixture.os, 'execv') as execute:
            with self.assertRaises(RuntimeError):
                self.run_fixture()
            execute.assert_not_called()
        self.opt_in.write_bytes(b'foreign')
        with patch.object(fixture.os, 'execv') as execute:
            with self.assertRaises(RuntimeError):
                self.run_fixture()
            execute.assert_not_called()
        self.opt_in.write_bytes(fixture.OPT_IN_CONTENT)
        for environment in ({'FLEET_NATIVE_SUPERVISOR_TEST': '1'},
                            {'HERMES_HOME': str(self.root)}):
            with patch.object(fixture.os, 'execv') as execute:
                with self.assertRaises(RuntimeError):
                    self.run_fixture(environment)
                execute.assert_not_called()

    def test_cleared_child_environment_does_not_require_qa_env_leakage(self):
        with patch.object(fixture.os, 'execv') as execute:
            self.run_fixture({'HERMES_HOME': str(self.home), 'LANG': 'C.UTF-8'})
            execute.assert_called_once()

    def test_without_marker_forwards_original_launcher_and_arguments(self):
        with patch.object(fixture.os, 'execv') as execute, patch.object(fixture.sys, 'argv', ['fixture', 'serve', '--host', '127.0.0.1', '--port', '29705']):
            self.run_fixture()
            execute.assert_called_once_with(fixture.LAUNCHER, [fixture.LAUNCHER, 'serve', '--host', '127.0.0.1', '--port', '29705'])

    def test_owned_marker_is_consumed_once_before_exec(self):
        self.marker.write_bytes(b'owned startup fault')
        with patch.object(fixture.os, 'execv') as execute:
            with self.assertRaises(SystemExit) as error:
                self.run_fixture()
            self.assertEqual(error.exception.code, 71)
            self.assertFalse(self.marker.exists())
            execute.assert_not_called()
            self.run_fixture()
            execute.assert_called_once()

    def test_foreign_marker_bytes_are_preserved_without_exec(self):
        self.marker.write_bytes(b'foreign')
        with patch.object(fixture.os, 'execv') as execute:
            with self.assertRaises(RuntimeError):
                self.run_fixture()
            self.assertEqual(self.marker.read_bytes(), b'foreign')
            execute.assert_not_called()

    def test_non_regular_marker_is_preserved_without_exec(self):
        self.marker.mkdir()
        with patch.object(fixture.os, 'execv') as execute:
            with self.assertRaises(RuntimeError):
                self.run_fixture()
            self.assertTrue(self.marker.is_dir())
            execute.assert_not_called()
