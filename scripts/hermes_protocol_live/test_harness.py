"""Host-side safety tests; no Docker, runtime credentials or native imports."""
import importlib.util
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


if __name__ == "__main__":
    unittest.main()
