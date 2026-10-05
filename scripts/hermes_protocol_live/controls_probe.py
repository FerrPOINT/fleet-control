#!/usr/bin/env python3
"""Actual pinned API/AIAgent control ACK readback; not Fleet integration or OS-safe stop."""
import hashlib
import http.client
import json
import os
from pathlib import Path
import threading
import types
import unittest
import urllib.parse
import uuid

import native_protocol as native

CAPS = "/fleet/v1/controls/capabilities"
LOOKUP = "/fleet/v1/controls/lookup"


def command(process, run_id, operation, raw, key, epoch):
    connection = http.client.HTTPConnection("127.0.0.1", process.port, timeout=15)
    try:
        connection.request("POST", f"/v1/runs/{run_id}/{operation}", body=raw,
            headers={"Authorization": "Bearer " + process.token, "Content-Type": "application/json",
                     "Idempotency-Key": key, "X-Fleet-Control-Store-Id": epoch})
        response = connection.getresponse()
        value = response.read(65_537)
        if len(value) > 65_536:
            raise RuntimeError("native command ACK exceeds fixture bound")
        return response.status, json.loads(value)
    finally:
        connection.close()


class LostControlAck(native.BaseHTTPRequestHandler):
    """Consume an actual accepted response, then close without sending it downstream."""
    def log_message(self, *_args):
        pass

    def do_POST(self):
        raw = self.rfile.read(int(self.headers["Content-Length"]))
        status, ack = command(self.server.upstream, self.server.run_id, self.server.operation,
                              raw, self.headers["Idempotency-Key"], self.headers["X-Fleet-Control-Store-Id"])
        self.server.received = (status, ack)
        self.server.forwarded.set()
        self.close_connection = True
        self.connection.close()


class ControlTests(native.NativeProtocolTests):
    test_native_sse_and_two_isolated_agents = None
    test_lost_ack_concurrent_replay_and_process_restart = None
    test_rotated_credential_is_not_recovery_scope = None
    test_unknown_ack_crash_during_inference_never_reexecutes = None

    def native(self, name):
        process = native.NativeProcess(Path(self.directory.name) / name, self.model.server_port,
                                       control_plugin=Path(__file__).with_name("plugin"))
        self.addCleanup(process.stop)
        process.start()
        return process

    def caps(self, process):
        code, result = native.json_request(process.port, CAPS, token=process.token)
        self.assertEqual(code, 200)
        self.assertEqual(result["native_source_revision"], os.environ["HERMES_PROTOCOL_SOURCE_SHA"])
        self.assertEqual(result["scope_fingerprint"], hashlib.sha256(("default\0" + process.token).encode()).hexdigest())
        self.assertIs(result["single_send"], True)
        self.assertIs(result["non_dispatch_lookup"], True)
        return result

    def lookup(self, process, caps, run, operation, raw, key, token=None):
        query = urllib.parse.urlencode({"store_id": caps["store_id"], "command_id": key,
            "run_id": run, "operation": operation, "request_sha256": hashlib.sha256(raw).hexdigest()})
        return native.json_request(process.port, LOOKUP + "?" + query,
                                    token=process.token if token is None else token)

    def lost(self, process, run, operation, raw, key, epoch):
        proxy = native.ThreadingHTTPServer(("127.0.0.1", 0), LostControlAck)
        proxy.upstream, proxy.run_id, proxy.operation = process, run, operation
        proxy.forwarded = threading.Event()
        thread = threading.Thread(target=proxy.serve_forever, daemon=True)
        thread.start()
        try:
            with self.assertRaises(OSError):
                command(types.SimpleNamespace(port=proxy.server_port, token=process.token),
                        run, operation, raw, key, epoch)
            self.assertTrue(proxy.forwarded.wait(5))
            self.assertEqual(proxy.received[0], 200)
            return proxy.received[1]
        finally:
            proxy.shutdown()
            proxy.server_close()
            thread.join(timeout=10)

    def test_real_steer_and_stop_ack_survive_transport_loss_and_gateway_restart(self):
        process = self.native("native-control-effects")
        caps = self.caps(process)
        entered, release = threading.Event(), threading.Event()
        self.addCleanup(release.set)
        prompt = "native-control-real-blocked-model"
        with self.model.count_lock:
            self.model.barriers[prompt] = (entered, release)
        code, accepted = native.json_request(process.port, "/v1/runs", token=process.token,
            body={"input": prompt, "session_id": "fleet:control:live"}, key=str(uuid.uuid4()))
        self.assertEqual(code, 202)
        run = accepted["run_id"]
        self.assertTrue(entered.wait(30))
        steer = b'{"input":"native-control-guidance"}'
        keys = {"steer": str(uuid.uuid4()), "stop": str(uuid.uuid4())}
        for operation, raw in (("steer", steer), ("stop", b"")):
            ack = self.lost(process, run, operation, raw, keys[operation], caps["store_id"])
            code, found = self.lookup(process, caps, run, operation, raw, keys[operation])
            self.assertEqual(code, 200)
            self.assertEqual(found["state"], "acknowledged")
            self.assertEqual(found["ack"], ack)
            self.assertEqual(command(process, run, operation, raw, keys[operation], caps["store_id"])[0], 409)
        release.set()
        terminal = process.terminal(run)
        self.assertIsNot(terminal.get("completed"), True)
        before = process.process.pid
        process.stop(crash=True)
        process.start()
        self.assertNotEqual(before, process.process.pid)
        self.assertEqual(self.caps(process), caps)
        for operation, raw in (("steer", steer), ("stop", b"")):
            code, found = self.lookup(process, caps, run, operation, raw, keys[operation])
            self.assertEqual(code, 200)
            self.assertEqual(found["state"], "acknowledged")
            self.assertEqual(command(process, run, operation, raw, keys[operation], caps["store_id"])[0], 409)
        self.assertEqual(self.model.counts.get(prompt), 1)
        self.assertEqual(self.lookup(process, caps, run, "steer", steer, keys["steer"], "foreign")[0], 401)
        self.evidence["cases"].append("real-AIAgent-steer-interrupt-ACK-original-key-readback-restart-no-native-reexecution")

    def test_native_rejected_command_remains_uncertain_and_never_reaches_handler_again(self):
        process = self.native("native-control-rejected")
        caps = self.caps(process)
        run, key = "run_" + "d" * 32, str(uuid.uuid4())
        raw = b'{"input":"never-dispatch-model"}'
        self.assertEqual(command(process, run, "steer", raw, key, caps["store_id"])[0], 503)
        code, found = self.lookup(process, caps, run, "steer", raw, key)
        self.assertEqual(code, 200)
        self.assertEqual(found["state"], "uncertain")
        self.assertNotIn("ack", found)
        self.assertEqual(command(process, run, "steer", raw, key, caps["store_id"])[0], 409)
        self.assertEqual(command(process, run, "steer", b'{"input":"changed"}', key, caps["store_id"])[0], 409)
        process.stop(crash=True)
        process.start()
        self.assertEqual(self.lookup(process, caps, run, "steer", raw, key)[1]["state"], "uncertain")
        self.assertEqual(command(process, run, "steer", raw, key, caps["store_id"])[0], 409)
        self.assertNotIn("never-dispatch-model", self.model.counts)
        self.evidence["cases"].append("native-unknown-command-persisted-hold-restart-conflict-no-handler-or-model-retry")


if __name__ == "__main__":
    unittest.main(verbosity=2)
