"""Only explicit Cargo test outputs may enter the disposable compiled volume."""
import importlib.util
import json
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import patch


SOURCE = Path(__file__).resolve().parents[1] / 'container_supervisor_live/select_artifact.py'
SPEC = importlib.util.spec_from_file_location('container_test_artifact', SOURCE)
selector = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(selector)


class ArtifactTests(unittest.TestCase):
    def invoke(self, output, name='infra', kind=None, profile=True, binary='/tmp/container-target/debug/infra-test',
               duplicate=False, exists=False):
        entry = dict(reason='compiler-artifact', target=dict(name=name, kind=kind or ['lib']),
                     profile=dict(test=profile), executable=binary)
        entries = [entry, entry] if duplicate else [entry]
        contents = '\n'.join(json.dumps(value) for value in entries)
        actual_path = Path

        def path(value):
            if value == '/input/artifacts.jsonl':
                return SimpleNamespace(read_text=lambda: contents)
            return actual_path(value)

        with patch.object(selector, 'Path', side_effect=path), \
                patch.object(selector.sys, 'argv', ['selector', '/input/artifacts.jsonl', output]), \
                patch.object(actual_path, 'exists', return_value=exists), \
                patch.object(actual_path, 'is_file', return_value=True), \
                patch.object(actual_path, 'resolve', lambda self: self), \
                patch.object(actual_path, 'chmod') as chmod, \
                patch.object(selector.shutil, 'copyfile') as copy:
            selector.main()
            copy.assert_called_once_with(actual_path(binary), actual_path(output))
            chmod.assert_called_once_with(0o755)

    def test_exact_library_and_integration_test_bindings(self):
        self.invoke('/out/fleet-runtime-tests')
        self.invoke('/out/fleet-container-live', name='container_supervisor_live', kind=['test'])

    def test_wrong_artifact_identity_profile_or_count_is_rejected(self):
        for invalid in (dict(name='api'), dict(kind=['test']), dict(profile=False), dict(duplicate=True)):
            with self.subTest(invalid=invalid), self.assertRaises(RuntimeError):
                self.invoke('/out/fleet-runtime-tests', **invalid)

    def test_unknown_output_existing_file_and_outside_binary_are_rejected(self):
        for invalid in (dict(output='/out/other'), dict(output='/out/fleet-runtime-tests', exists=True),
                        dict(output='/out/fleet-runtime-tests', binary='/tmp/foreign/infra-test')):
            with self.subTest(invalid=invalid), self.assertRaises(RuntimeError):
                self.invoke(**invalid)


if __name__ == '__main__':
    unittest.main()
