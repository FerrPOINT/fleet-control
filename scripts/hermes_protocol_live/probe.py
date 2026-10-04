#!/usr/bin/env python3
"""Native Hermes HTTP/SQLite acceptance with a local deterministic model only."""
import argparse
import asyncio
from concurrent.futures import ThreadPoolExecutor
import hashlib
import http.client
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import secrets
import signal
import socket
import sqlite3
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import urllib.error


MODEL = "fleet-native-protocol-fixture"
MAX_BODY = 1024 * 1024


def request(port, path, *, token=None, body=None, key=None, timeout=30):
    deadline = time.monotonic() + timeout
    headers = {"Accept": "application/json"}
    if token is not None:
        headers["Authorization"] = "Bearer " + token
    if key is not None:
        headers["Idempotency-Key"] = key
    if body is not None:
        headers["Content-Type"] = "application/json"
        body = json.dumps(body, sort_keys=True, separators=(",", ":")).encode()
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=timeout)
    timer = None
    try:
        connection.connect()
        transport = connection.sock
        def remaining():
            budget = deadline - time.monotonic()
            if budget <= 0:
                raise TimeoutError("native response absolute deadline exceeded")
            return budget
        def expire():
            try:
                transport.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass
        timer = threading.Timer(remaining(), expire)
        timer.daemon = True
        timer.start()
        transport.settimeout(remaining())
        connection.request("POST" if body is not None else "GET", path, body=body, headers=headers)
        with connection.getresponse() as response:
            payload = bytearray()
            while True:
                transport.settimeout(remaining())
                chunk = response.read1(min(4096, MAX_BODY + 1 - len(payload)))
                remaining()
                payload.extend(chunk)
                if len(payload) > MAX_BODY:
                    raise RuntimeError("native response exceeds acceptance bound")
                if not chunk:
                    break
            return response.status, payload
    finally:
        if timer is not None:
            timer.cancel()
        connection.close()


def json_request(*args, **kwargs):
    status, payload = request(*args, **kwargs)
    return status, json.loads(payload)


def free_port():
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def parse_sse(payload):
    events = []
    for frame in payload.decode("utf-8").replace("\r\n", "\n").split("\n\n"):
        lines = [line[5:].lstrip() for line in frame.splitlines() if line.startswith("data:")]
        if lines:
            events.append(json.loads("\n".join(lines)))
    return events


class ModelHandler(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_GET(self):
        self.send_json({"object": "list", "data": [{"id": MODEL, "object": "model"}]})

    def send_json(self, value):
        encoded = json.dumps(value).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(encoded)))
        self.end_headers()
        self.wfile.write(encoded)

    def do_POST(self):
        size = int(self.headers.get("Content-Length", "0"))
        if not 0 < size <= MAX_BODY or self.path != "/v1/chat/completions":
            self.send_error(400)
            return
        body = json.loads(self.rfile.read(size))
        users = [m.get("content", "") for m in body.get("messages", []) if m.get("role") == "user"]
        prompt = users[-1] if users else "no-user-prompt"
        if not isinstance(prompt, str):
            prompt = json.dumps(prompt, sort_keys=True)
        with self.server.count_lock:
            self.server.counts[prompt] = self.server.counts.get(prompt, 0) + 1
            barrier = self.server.barriers.get(prompt)
        if barrier is not None:
            entered, release = barrier
            entered.set()
            if not release.wait(90):
                self.send_error(503)
                return
        output = "native-model-reply"
        choice = {"index": 0, "message": {"role": "assistant", "content": output}, "finish_reason": "stop"}
        base = {"id": "chatcmpl-native-fixture", "model": MODEL, "created": int(time.time())}
        if not body.get("stream"):
            self.send_json({**base, "object": "chat.completion", "choices": [choice],
                            "usage": {"prompt_tokens": 10, "completion_tokens": 3, "total_tokens": 13}})
            return
        self.send_response(200)
        self.send_header("Content-Type", "text/event-stream")
        self.send_header("Connection", "close")
        self.end_headers()
        for delta, finish in [({"role": "assistant", "content": output}, None), ({}, "stop")]:
            chunk = {**base, "object": "chat.completion.chunk",
                     "choices": [{"index": 0, "delta": delta, "finish_reason": finish}]}
            self.wfile.write(("data: " + json.dumps(chunk) + "\n\n").encode())
        self.wfile.write(b"data: [DONE]\n\n")
        self.wfile.flush()
        self.close_connection = True


class LostAckHandler(BaseHTTPRequestHandler):
    """Forward a real POST, consume its ACK, then drop it before the client sees it."""
    def log_message(self, *_args):
        pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        status, payload = request(self.server.upstream_port, "/v1/runs",
                                  token=self.server.token, body=body, key=self.headers["Idempotency-Key"])
        self.server.upstream_status = status
        self.server.accepted = json.loads(payload) if status == 202 else None
        self.server.forwarded.set()
        self.close_connection = True
        self.connection.shutdown(socket.SHUT_RDWR)


async def serve(home, port):
    os.environ["HERMES_HOME"] = str(home)
    from gateway.config import PlatformConfig
    from gateway.platforms.api_server import APIServerAdapter
    import gateway.platforms.api_server as native_api
    import run_agent
    source = Path(os.environ["PYTHONPATH"]).resolve()
    if any(not Path(module.__file__).resolve().is_relative_to(source) for module in [native_api, run_agent]):
        raise RuntimeError("native code was not imported from the pinned source snapshot")
    adapter = APIServerAdapter(PlatformConfig(enabled=True, extra={
        "host": "127.0.0.1", "port": port, "key": os.environ["API_SERVER_KEY"],
    }))
    if not await adapter.connect():
        raise RuntimeError("native adapter refused startup")
    stopping = asyncio.Event()
    asyncio.get_running_loop().add_signal_handler(signal.SIGTERM, stopping.set)
    try:
        await stopping.wait()
    finally:
        adapter.interrupt_active_runs("acceptance fixture shutdown")
        await adapter.disconnect()


class NativeProcess:
    def __init__(self, home, model_port):
        self.home = home
        home.mkdir()
        self.token = secrets.token_hex(32)
        self.port = free_port()
        self.process = None
        self.log = None
        config = {
            "model": {"default": MODEL, "provider": "custom", "api_mode": "chat_completions",
                      "base_url": f"http://127.0.0.1:{model_port}/v1", "api_key": "no-key-required",
                      "context_length": 131072},
            "platform_toolsets": {"api_server": []}, "mcp_servers": {},
            "security": {"tirith_enabled": False, "allow_lazy_installs": False},
            "memory": {"memory_enabled": False, "user_profile_enabled": False, "provider": ""},
            "auxiliary": {"title_generation": {"enabled": False}, "background_review": {"enabled": False}},
            "telemetry": {"shared_metrics": {"enabled": False}},
            "agent": {"max_turns": 2},
        }
        (home / "config.yaml").write_text(json.dumps(config), encoding="utf-8")
        (home / "SOUL.md").write_text("Answer the fixture prompt without using tools.\n", encoding="utf-8")
        self.workspace = home / "workspace"
        self.workspace.mkdir()

    def start(self):
        env = {key: value for key, value in os.environ.items()
               if key in {"PATH", "PYTHONPATH", "PYTHONDONTWRITEBYTECODE", "LANG"}}
        env.update({"HERMES_HOME": str(self.home), "HOME": str(self.home),
                    "API_SERVER_KEY": self.token, "HERMES_HEADLESS": "1",
                    "HERMES_DISABLE_LAZY_INSTALLS": "1",
                    "HERMES_AGENT_WORKDIR": str(self.workspace)})
        self.log = (self.home / "private-native.log").open("ab")
        self.process = subprocess.Popen([sys.executable, str(Path(__file__).resolve()),
            "--serve", "--home", str(self.home), "--port", str(self.port)],
            cwd=self.workspace, env=env, stdout=self.log, stderr=subprocess.STDOUT)
        deadline = time.monotonic() + 75
        while time.monotonic() < deadline:
            if self.process.poll() is not None:
                raise RuntimeError("native adapter process exited during startup")
            try:
                status, _ = request(self.port, "/health")
                if status == 200:
                    return
            except (OSError, urllib.error.URLError):
                pass
            time.sleep(0.2)
        raise RuntimeError("native adapter readiness deadline exceeded")

    def stop(self, *, crash=False):
        if self.process is not None:
            if self.process.poll() is None:
                self.process.kill() if crash else self.process.terminate()
                try:
                    self.process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait(timeout=10)
            self.process = None
        if self.log is not None:
            self.log.close()
            self.log = None

    def terminal(self, run_id):
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            status, result = json_request(self.port, "/v1/runs/" + run_id, token=self.token)
            if status != 200:
                raise RuntimeError("native status readback failed")
            if result.get("status") in {"completed", "failed", "cancelled", "interrupted"}:
                return result
            time.sleep(0.1)
        raise RuntimeError("native model run did not terminate")


class NativeProtocolTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory(prefix="fleet-native-protocol-")
        cls.model = ThreadingHTTPServer(("127.0.0.1", 0), ModelHandler)
        cls.model.count_lock = threading.Lock()
        cls.model.counts = {}
        cls.model.barriers = {}
        cls.thread = threading.Thread(target=cls.model.serve_forever, daemon=True)
        cls.thread.start()
        cls.evidence = {"model": "deterministic-local-openai", "native_agent": "real-AIAgent",
                        "native_source_sha": os.environ["HERMES_PROTOCOL_SOURCE_SHA"], "cases": []}

    @classmethod
    def tearDownClass(cls):
        cls.model.shutdown()
        cls.model.server_close()
        cls.thread.join(timeout=10)
        Path(os.environ["HERMES_PROTOCOL_RESULT"]).write_text(
            json.dumps(cls.evidence, indent=2) + "\n", encoding="utf-8")
        cls.directory.cleanup()

    def native(self, name):
        native = NativeProcess(Path(self.directory.name) / name, self.model.server_port)
        self.addCleanup(native.stop)
        native.start()
        return native

    def completed(self, status, run_id, session_id):
        self.assertEqual(status.get("object"), "hermes.run")
        self.assertEqual(status.get("run_id"), run_id)
        self.assertEqual(status.get("status"), "completed")
        self.assertIs(status.get("completed"), True)
        self.assertIs(status.get("partial"), False)
        self.assertIs(status.get("interrupted"), False)
        self.assertEqual(status.get("output"), "native-model-reply")
        self.assertEqual(status.get("session_id"), session_id)

    def transcript(self, native, session_id, prompt):
        code, document = json_request(native.port, f"/api/sessions/{session_id}/messages", token=native.token)
        self.assertEqual(code, 200)
        self.assertEqual(document["session_id"], session_id)
        self.assertTrue(any(m.get("role") == "user" and m.get("content") == prompt for m in document["data"]))
        self.assertTrue(any(m.get("role") == "assistant" and m.get("content") == "native-model-reply" for m in document["data"]))
        # Independent read-only observation; Fleet production never writes this database.
        with sqlite3.connect(f"file:{native.home / 'state.db'}?mode=ro", uri=True) as db:
            self.assertEqual(db.execute("SELECT id FROM sessions WHERE id=?", (session_id,)).fetchone(), (session_id,))

    def lose_ack(self, native, body, key):
        proxy = ThreadingHTTPServer(("127.0.0.1", 0), LostAckHandler)
        proxy.upstream_port, proxy.token = native.port, native.token
        proxy.forwarded = threading.Event()
        thread = threading.Thread(target=proxy.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(thread.join, 10)
        self.addCleanup(proxy.server_close)
        self.addCleanup(proxy.shutdown)
        with self.assertRaises((OSError, urllib.error.URLError)):
            request(proxy.server_port, "/v1/runs", body=body, key=key)
        self.assertTrue(proxy.forwarded.wait(5))
        self.assertEqual(proxy.upstream_status, 202)
        self.assertIs(proxy.accepted["replayed"], False)
        return proxy.accepted["run_id"]

    def test_native_sse_and_two_isolated_agents(self):
        one, two = self.native("agent1"), self.native("agent2")
        code, capabilities = json_request(one.port, "/v1/capabilities", token=one.token)
        self.assertEqual(code, 200)
        self.assertEqual(capabilities["features"]["runs_idempotency"],
                         {"supported": True, "durable": True, "retention_seconds": 86400})
        body = {"input": "native-stream-fixture", "session_id": "fleet:fixture:agent1"}
        code, ack = json_request(one.port, "/v1/runs", token=one.token, body=body, key="native-stream-key")
        self.assertEqual(code, 202)
        self.assertIs(ack["replayed"], False)
        run_id = ack["run_id"]
        code, raw = request(one.port, f"/v1/runs/{run_id}/events", token=one.token)
        self.assertEqual(code, 200)
        events = parse_sse(raw)
        self.assertTrue(events)
        self.assertTrue(all(event.get("run_id") == run_id for event in events))
        terminals = [event for event in events if event.get("event") == "run.completed"]
        self.assertEqual(len(terminals), 1)
        self.assertIs(terminals[0].get("completed"), True)
        self.assertIs(terminals[0].get("partial"), False)
        self.assertIs(terminals[0].get("interrupted"), False)
        result = one.terminal(run_id)
        self.completed(result, run_id, body["session_id"])
        self.transcript(one, body["session_id"], body["input"])
        self.assertEqual(json_request(one.port, f"/v1/runs/{run_id}", token=two.token)[0], 401)
        self.assertEqual(json_request(two.port, f"/v1/runs/{run_id}", token=two.token)[0], 404)
        self.assertEqual(json_request(one.port, f"/p/not-served/v1/runs/{run_id}", token=one.token)[0], 404)
        self.assertNotEqual(one.process.pid, two.process.pid)
        self.assertNotEqual(one.home, two.home)
        self.assertTrue((one.home / "runs_idempotency.db").is_file())
        self.assertEqual(self.model.counts.get(body["input"]), 1)
        self.evidence["cases"].append("native-sse-two-homes-token-and-profile-isolation")

    def test_lost_ack_concurrent_replay_and_process_restart(self):
        native = self.native("lost-ack")
        body = {"input": "native-lost-ack-fixture", "session_id": "fleet:fixture:lost"}
        key = "native-lost-ack-key"
        run_id = self.lose_ack(native, body, key)
        with ThreadPoolExecutor(max_workers=8) as pool:
            replies = list(pool.map(lambda _: json_request(native.port, "/v1/runs", token=native.token,
                                                          body=body, key=key), range(8)))
        self.assertTrue(all(code == 202 and ack["run_id"] == run_id and ack["replayed"] is True
                            for code, ack in replies))
        code, _ = json_request(native.port, "/v1/runs", token=native.token,
                               body={**body, "input": "changed-input"}, key=key)
        self.assertEqual(code, 409)
        result = native.terminal(run_id)
        self.completed(result, run_id, body["session_id"])
        self.transcript(native, body["session_id"], body["input"])
        self.assertEqual(self.model.counts.get(body["input"]), 1)
        old_pid = native.process.pid
        native.stop(crash=True)
        native.start()
        self.assertNotEqual(native.process.pid, old_pid)
        code, replay = json_request(native.port, "/v1/runs", token=native.token, body=body, key=key)
        self.assertEqual(code, 202)
        self.assertEqual(replay["run_id"], run_id)
        self.assertIs(replay["replayed"], True)
        self.assertEqual(replay["status"], "completed")
        code, recovered = json_request(native.port, f"/v1/runs/{run_id}", token=native.token)
        self.assertEqual(code, 200)
        self.completed(recovered, run_id, body["session_id"])
        self.transcript(native, body["session_id"], body["input"])
        self.assertEqual(recovered["session_id"], result["session_id"])
        self.assertEqual(request(native.port, f"/v1/runs/{run_id}/events", token=native.token)[0], 404)
        self.assertEqual(self.model.counts.get(body["input"]), 1)
        self.evidence["cases"].append("dropped-202-eight-replays-conflict-process-crash-status-recovery")

    def test_rotated_credential_is_not_recovery_scope(self):
        native = self.native("rotation")
        body = {"input": "native-rotation-fixture", "session_id": "fleet:fixture:rotation"}
        key = "native-rotation-key"
        code, ack = json_request(native.port, "/v1/runs", token=native.token, body=body, key=key)
        self.assertEqual(code, 202)
        self.completed(native.terminal(ack["run_id"]), ack["run_id"], body["session_id"])
        old_token = native.token
        native.stop(crash=True)
        native.token = secrets.token_hex(32)
        native.start()
        path = f"/v1/runs/{ack['run_id']}"
        self.assertEqual(request(native.port, path, token=old_token)[0], 401)
        self.assertEqual(request(native.port, path, token=native.token)[0], 404)
        for action, payload in [("stop", {}), ("steer", {"input": "fixture"}), ("approval", {"choice": "deny"})]:
            self.assertEqual(request(native.port, path + "/" + action, token=old_token, body=payload)[0], 401)
        code, replacement = json_request(native.port, "/v1/runs", token=native.token, body=body, key=key)
        self.assertEqual(code, 202)
        self.assertIs(replacement["replayed"], False)
        self.assertNotEqual(replacement["run_id"], ack["run_id"])
        self.completed(native.terminal(replacement["run_id"]), replacement["run_id"], body["session_id"])
        self.assertEqual(self.model.counts.get(body["input"]), 2)
        self.evidence["cases"].append("credential-rotation-new-scope-must-prohibit-fleet-auto-replay")

    def test_unknown_ack_crash_during_inference_never_reexecutes(self):
        native = self.native("unknown-active")
        body = {"input": "native-active-crash-fixture", "session_id": "fleet:fixture:active"}
        key = "native-active-crash-key"
        entered, release = threading.Event(), threading.Event()
        self.addCleanup(release.set)
        with self.model.count_lock:
            self.model.barriers[body["input"]] = (entered, release)
        run_id = self.lose_ack(native, body, key)
        self.assertTrue(entered.wait(30))
        code, status = json_request(native.port, f"/v1/runs/{run_id}", token=native.token)
        self.assertEqual(code, 200)
        self.assertEqual(status["status"], "running")
        old_pid = native.process.pid
        native.stop(crash=True)
        release.set()
        native.start()
        self.assertNotEqual(native.process.pid, old_pid)
        code, replay = json_request(native.port, "/v1/runs", token=native.token, body=body, key=key)
        self.assertEqual(code, 202)
        self.assertEqual(replay["run_id"], run_id)
        self.assertIs(replay["replayed"], True)
        code, recovered = json_request(native.port, f"/v1/runs/{run_id}", token=native.token)
        self.assertEqual(code, 200)
        self.assertEqual(recovered["run_id"], run_id)
        self.assertEqual(recovered["session_id"], body["session_id"])
        self.assertEqual(recovered["status"], "interrupted")
        self.assertEqual(replay["status"], "interrupted")
        self.assertEqual(recovered["object"], "hermes.run")
        self.assertEqual(recovered["last_event"], "run.interrupted")
        self.assertIsNot(recovered.get("completed"), True)
        self.assertNotIn("output", recovered)
        self.assertEqual(self.model.counts.get(body["input"]), 1)
        self.evidence["cases"].append("dropped-202-crash-during-real-inference-original-id-interrupted-no-reexecution")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--serve", action="store_true")
    parser.add_argument("--home", type=Path)
    parser.add_argument("--port", type=int)
    args = parser.parse_args()
    if args.serve:
        asyncio.run(serve(args.home, args.port))
        return
    source = Path(os.environ["PYTHONPATH"])
    for name in ["uv.lock", "pyproject.toml"]:
        if hashlib.sha256((source / name).read_bytes()).digest() != hashlib.sha256((Path("/opt/hermes") / name).read_bytes()).digest():
            raise RuntimeError("dependency image differs from pinned Hermes resolution")
    unittest.main(argv=[sys.argv[0]], verbosity=2)


if __name__ == "__main__":
    main()
