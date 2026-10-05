"""Host-side safety tests; no Docker, runtime credentials or native imports."""
import importlib.util
import hashlib
import io
import json
from pathlib import Path
import socket
import subprocess
import sys
import threading
import time
import tarfile
import tempfile
import unittest
from unittest.mock import patch


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(file))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


probe = load("hermes_native_probe", "probe.py")
runner = load("hermes_native_runner", "run.py")
renderer = load("hermes_renderer_probe", "renderer_probe.py")


def archive(name, *, symlink=False):
    stream = io.BytesIO()
    with tarfile.open(fileobj=stream, mode="w") as tar:
        item = tarfile.TarInfo(name)
        if symlink:
            item.type = tarfile.SYMTYPE
            item.linkname = "outside"
            tar.addfile(item)
        else:
            payload = b"native-source\n"
            item.size = len(payload)
            tar.addfile(item, io.BytesIO(payload))
    return stream.getvalue()


class HarnessSafetyTests(unittest.TestCase):
    def test_controls_requires_complete_explicit_plugin_before_docker(self):
        with tempfile.TemporaryDirectory() as directory:
            for extra in [
                ['--scenario', 'controls'],
                ['--scenario', 'controls', '--control-plugin-root', directory],
                ['--control-plugin-root', directory],
            ]:
                with self.subTest(arguments=extra), \
                     patch.object(sys, 'argv', ['run.py', '--hermes', directory, '--image', 'fixture', *extra]), \
                     patch.object(runner.subprocess, 'check_output') as docker, self.assertRaises(SystemExit):
                    runner.main()
                docker.assert_not_called()

    def test_control_fixture_is_explicit_and_does_not_enable_recovery_plugin(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            plugin = root / 'plugin'
            plugin.mkdir()
            for name in ('__init__.py', 'plugin.py', 'store.py', 'plugin.yaml'):
                (plugin / name).write_bytes(b'fixture-only')
            process = probe.NativeProcess(root / 'home', 12345, control_plugin=plugin)
            config = json.loads((process.home / 'config.yaml').read_text())
            self.assertEqual(config['plugins']['enabled'], ['fleet-hermes-controls'])
            self.assertTrue((process.home / 'plugins/fleet-hermes-controls/plugin.py').is_file())
            self.assertFalse((process.home / 'plugins/fleet-hermes-recovery').exists())

    def test_native_yaml_exception_is_not_hidden_by_env_fallback(self):
        def failing_loader(_home, _data):
            raise TypeError('synthetic malformed native extra')
        with self.assertRaises(TypeError), patch.object(renderer, 'ROOT', Path('/renderer-evidence')):
            renderer.yaml_only_config(Path('/renderer-evidence/config'), failing_loader,
                                      lambda _data: self.fail('fallback constructor invoked'), 'api_server')

    def test_renderer_paths_cannot_escape_owned_root(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(renderer, 'ROOT', Path(directory)):
            for path in [Path(directory) / '..' / 'outside', Path('relative'), Path(directory).parent / 'outside']:
                with self.assertRaises(RuntimeError):
                    renderer.guarded(path)

    def test_renderer_inventory_and_tampered_bytes_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(renderer, 'ROOT', Path(directory)):
            home = Path(directory) / 'config'
            home.mkdir()
            hashes = {}
            for name in renderer.FILES:
                payload = ('fixture ' + name).encode()
                (home / name).write_bytes(payload)
                hashes[name] = hashlib.sha256(payload).hexdigest()
            agent = {'home':str(home), 'hashes':hashes}
            renderer.verify_files(agent)
            with self.assertRaises(RuntimeError):
                renderer.verify_files({'home':str(home), 'hashes':{'.env':hashes['.env']}})
            (home / '.env').write_bytes(b'tampered fixture')
            with self.assertRaises(RuntimeError):
                renderer.verify_files(agent)

    def test_renderer_scenario_requires_owned_evidence_before_docker(self):
        with patch.object(sys, 'argv', ['run.py', '--hermes','fixture', '--image','fixture', '--scenario','renderer']), \
             patch.object(runner.subprocess, 'check_output') as docker, self.assertRaises(SystemExit):
            runner.main()
        docker.assert_not_called()

    def test_recovery_requires_complete_explicit_plugin_before_docker(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'plugin.py').write_bytes(b'host-fixture-only')
            for extra in [
                ['--scenario', 'recovery'],
                ['--scenario', 'recovery', '--recovery-plugin-root', directory],
                ['--recovery-plugin-root', directory],
            ]:
                with self.subTest(arguments=extra), \
                     patch.object(sys, 'argv', ['run.py', '--hermes', directory, '--image', 'fixture', *extra]), \
                     patch.object(runner.subprocess, 'check_output') as docker, self.assertRaises(SystemExit):
                    runner.main()
                docker.assert_not_called()

    def test_recovery_snapshots_module_hashes_and_native_helper_read_only(self):
        with tempfile.TemporaryDirectory() as directory:
            plugin = Path(directory) / 'explicit-plugin'
            plugin.mkdir()
            hashes = {}
            for name in ('__init__.py', 'plugin.py', 'store.py', 'plugin.yaml'):
                payload = ('host-fixture-only:' + name + '\n').encode()
                (plugin / name).write_bytes(payload)
                hashes[name] = hashlib.sha256(payload).hexdigest()
            metadata = [{'Id': 'sha256:host-fixture-only', 'Config': {
                'Labels': {'sdlc.hermes.revision': runner.PIN},
            }}]

            def command(argv, **_kwargs):
                compose = Path(argv[argv.index('-f') + 1])
                action = argv[argv.index(str(compose)) + 1]
                if action == 'up':
                    document = json.loads(compose.read_text())
                    service = document['services']['probe']
                    self.assertTrue(service['read_only'])
                    self.assertNotIn('user', service)
                    self.assertNotIn('ports', service)
                    self.assertTrue(document['networks']['qa']['internal'])
                    mount = next(v for v in service['volumes'] if v['target'] == '/qa/probe')
                    self.assertTrue(mount['read_only'])
                    for name, digest in hashes.items():
                        copied = Path(mount['source']) / 'plugin' / name
                        self.assertEqual(hashlib.sha256(copied.read_bytes()).hexdigest(), digest)
                    copied_helper = Path(mount['source']) / 'native_protocol.py'
                    self.assertEqual(copied_helper.read_bytes(), Path(probe.__file__).read_bytes())
                    (compose.parent / 'output/native-result.json').write_text(json.dumps({
                        'native_source_sha': runner.PIN, 'cases': ['host-fixture-only'] * 2,
                    }), encoding='utf-8')
                return subprocess.CompletedProcess(argv, 0, stdout=b'[]' if action == 'ps' else b'fixture log')

            with patch.object(runner.subprocess, 'check_output', return_value=json.dumps(metadata).encode()), \
                 patch.object(runner.subprocess, 'run', side_effect=command), \
                 patch.object(runner, 'snapshot', return_value='host-fixture-source'), \
                 patch.object(sys, 'argv', [
                     'run.py', '--hermes', directory, '--image', 'fixture', '--artifacts', directory,
                     '--scenario', 'recovery', '--recovery-plugin-root', str(plugin),
                 ]):
                runner.main()
            evidence = json.loads(next(Path(directory).glob('*/evidence.json')).read_text())
            self.assertEqual(evidence['result'], 'passed')
            self.assertEqual(evidence['plugin_sha256'], hashes)
            self.assertEqual(evidence['native_helper_sha256'], hashlib.sha256(Path(probe.__file__).read_bytes()).hexdigest())
            self.assertEqual(evidence['cleanup_exit_code'], 0)
            self.assertTrue(evidence['post_cleanup_ps_empty'])

    def test_sse_comments_and_multiline_crlf(self):
        payload = b': keepalive\r\n\r\ndata: {"run_id":"run_test",\r\ndata: "event":"run.completed"}\r\n\r\n'
        self.assertEqual(probe.parse_sse(payload), [{"run_id": "run_test", "event": "run.completed"}])

    def test_sse_corrupt_frame_is_not_skipped(self):
        with self.assertRaises(json.JSONDecodeError):
            probe.parse_sse(b"data: invalid\n\n")

    def test_snapshot_exact_clean_revision(self):
        source = archive("gateway/module.py")
        with tempfile.TemporaryDirectory() as directory, patch.object(runner, "git", side_effect=[
            runner.PIN.encode(), b"", source,
        ]):
            destination = Path(directory) / "snapshot"
            digest = runner.snapshot(Path(directory), destination)
            self.assertEqual(len(digest), 64)
            self.assertEqual((destination / "gateway/module.py").read_bytes(), b"native-source\n")

    def test_snapshot_dirty_or_different_pin_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            for answers in [[b"0" * 40], [runner.PIN.encode(), b" M config.yaml"]]:
                with patch.object(runner, "git", side_effect=answers), self.assertRaises(RuntimeError):
                    runner.snapshot(Path(directory), Path(directory) / "snapshot")
            self.assertFalse((Path(directory) / "snapshot").exists())

    def test_snapshot_cannot_escape_owned_directory(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(runner, "git", side_effect=[
            runner.PIN.encode(), b"", archive("../outside.py"),
        ]), self.assertRaises(RuntimeError):
            runner.snapshot(Path(directory), Path(directory) / "snapshot")

    def test_snapshot_does_not_install_links(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(runner, "git", side_effect=[
            runner.PIN.encode(), b"", archive("gateway/link", symlink=True),
        ]), self.assertRaises(RuntimeError):
            runner.snapshot(Path(directory), Path(directory) / "snapshot")

    def test_keepalive_cannot_extend_response_deadline(self):
        stopping = threading.Event()
        class KeepaliveHandler(probe.BaseHTTPRequestHandler):
            def log_message(self, *_args):
                pass

            def do_GET(self):
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream")
                self.end_headers()
                try:
                    while not stopping.wait(0.02):
                        self.wfile.write(b": keepalive\n\n")
                        self.wfile.flush()
                except (BrokenPipeError, ConnectionResetError):
                    pass
        server = probe.ThreadingHTTPServer(("127.0.0.1", 0), KeepaliveHandler)
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        started = time.monotonic()
        try:
            with self.assertRaises((TimeoutError, socket.timeout)):
                probe.request(server.server_port, "/events", timeout=0.2)
            self.assertLess(time.monotonic() - started, 1.5)
        finally:
            stopping.set()
            server.shutdown()
            server.server_close()
            thread.join(timeout=5)

    def test_cleanup_timeouts_preserve_failed_evidence(self):
        for failing_action in ["down", "ps"]:
            with self.subTest(action=failing_action), tempfile.TemporaryDirectory() as directory:
                metadata = [{"Id": "sha256:host-fixture-only", "Config": {
                    "Labels": {"sdlc.hermes.revision": runner.PIN},
                }}]
                def command(argv, **_kwargs):
                    compose = Path(argv[argv.index("-f") + 1])
                    action = argv[argv.index(str(compose)) + 1]
                    if action == "up":
                        self.assertNotIn('user', json.loads(compose.read_text())['services']['probe'])
                        (compose.parent / "output/native-result.json").write_text(json.dumps({
                            "native_source_sha": runner.PIN, "cases": ["fixture"] * 4,
                        }), encoding="utf-8")
                    if action == failing_action:
                        raise subprocess.TimeoutExpired(argv, 90, output=b"partial cleanup log")
                    return subprocess.CompletedProcess(argv, 0, stdout=b"[]" if action == "ps" else b"fixture log")
                with patch.object(runner.subprocess, "check_output", return_value=json.dumps(metadata).encode()), \
                     patch.object(runner.subprocess, "run", side_effect=command), \
                     patch.object(runner, "snapshot", return_value="host-fixture-source"), \
                     patch.object(sys, "argv", [
                         "run.py", "--hermes", directory, "--image", "fixture", "--artifacts", directory,
                     ]), self.assertRaises(RuntimeError):
                    runner.main()
                paths = list(Path(directory).glob("*/evidence.json"))
                self.assertEqual(len(paths), 1)
                result = json.loads(paths[0].read_text())
                self.assertEqual(result["result"], "cleanup_failed")
                field = "cleanup_error" if failing_action == "down" else "post_cleanup_ps_error"
                self.assertEqual(result[field], "TimeoutExpired")

    def test_renderer_fixture_owner_override_is_read_only_and_scenario_specific(self):
        with tempfile.TemporaryDirectory() as directory:
            metadata = [{"Id": "sha256:host-fixture-only", "Config": {
                "Labels": {"sdlc.hermes.revision": runner.PIN},
            }}]
            def command(argv, **_kwargs):
                compose = Path(argv[argv.index('-f') + 1])
                action = argv[argv.index(str(compose)) + 1]
                if action == 'up':
                    service = json.loads(compose.read_text())['services']['probe']
                    self.assertEqual(service['user'], '0:0')
                    self.assertEqual(service['cap_drop'], ['ALL'])
                    self.assertEqual(service['security_opt'], ['no-new-privileges:true'])
                    self.assertTrue(service['read_only'])
                    mount = next(v for v in service['volumes'] if v['target'] == '/renderer-evidence')
                    self.assertTrue(mount['read_only'])
                    self.assertNotIn('ports', service)
                    (compose.parent / 'output/native-result.json').write_text(json.dumps({
                        'native_source_sha': runner.PIN, 'cases': ['synthetic-renderer-case'],
                    }), encoding='utf-8')
                return subprocess.CompletedProcess(argv, 0, stdout=b'[]' if action == 'ps' else b'fixture log')
            with patch.object(runner.subprocess, 'check_output', return_value=json.dumps(metadata).encode()), \
                 patch.object(runner.subprocess, 'run', side_effect=command), \
                 patch.object(runner, 'snapshot', return_value='host-fixture-source'), \
                 patch.object(sys, 'argv', [
                     'run.py', '--hermes', directory, '--image', 'fixture', '--artifacts', directory,
                     '--scenario', 'renderer', '--renderer-evidence-root', directory,
                 ]):
                runner.main()
            evidence = json.loads(next(Path(directory).glob('*/evidence.json')).read_text())
            self.assertEqual(evidence['result'], 'passed')


if __name__ == "__main__":
    unittest.main()
