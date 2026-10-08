#!/usr/bin/env python3
"""Actual initialized native request observation, not SDLC admission or full inventory."""
import hashlib
import hmac
import io
import json
import os
from pathlib import Path
import threading
import unittest
import urllib.parse
import uuid

import native_protocol as native

CAPS = "/fleet/v1/request-observations/capabilities"
READ = "/fleet/v1/request-observations/"


def digest(token, value):
    key = hmac.digest(token.encode(), b"fleet-request-observation/v1", "sha256")
    raw = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()
    return hmac.new(key, raw, hashlib.sha256).hexdigest()


class RecordingModel(native.ModelHandler):
    def do_POST(self):
        size = int(self.headers.get("Content-Length", "0"))
        if not 0 < size <= native.MAX_BODY:
            self.send_error(400)
            return
        raw = self.rfile.read(size)
        body = json.loads(raw)
        users = [m.get("content", "") for m in body["messages"] if m["role"] == "user"]
        with self.server.count_lock:
            self.server.observed_requests[users[-1]] = body
        original = self.rfile
        self.rfile = io.BytesIO(raw)
        try:
            super().do_POST()
        finally:
            self.rfile.close()
            self.rfile = original


class ObservationTests(native.NativeProtocolTests):
    test_native_sse_and_two_isolated_agents = None
    test_lost_ack_concurrent_replay_and_process_restart = None
    test_rotated_credential_is_not_recovery_scope = None
    test_unknown_ack_crash_during_inference_never_reexecutes = None

    @classmethod
    def setUpClass(cls):
        super().setUpClass()
        cls.model.observed_requests = {}
        cls.model.RequestHandlerClass = RecordingModel

    def process(self, name):
        process = native.NativeProcess(Path(self.directory.name) / name, self.model.server_port,
            observer_plugin=Path(__file__).with_name("plugin"))
        self.addCleanup(process.stop)
        process.start()
        return process

    def caps(self, process):
        status, value = native.json_request(process.port, CAPS, token=process.token)
        self.assertEqual(status, 200)
        self.assertEqual(value["native_source_revision"], os.environ["HERMES_PROTOCOL_SOURCE_SHA"])
        self.assertIs(value["observational_read"], True)
        self.assertIs(value["runtime_ready"], False)
        return value

    def observation(self, process, run, incarnation, token=None):
        return native.json_request(process.port, READ + run + "?" + urllib.parse.urlencode({"incarnation": incarnation}),
            token=process.token if token is None else token)

    def run_held(self, process, prompt):
        entered, release = threading.Event(), threading.Event()
        self.addCleanup(release.set)
        with self.model.count_lock:
            self.model.barriers[prompt] = (entered, release)
        status, result = native.json_request(process.port, "/v1/runs", token=process.token,
            body={"input": prompt, "session_id": "observation-" + uuid.uuid4().hex})
        self.assertEqual(status, 202)
        self.assertTrue(entered.wait(30))
        return result["run_id"], release

    def compare(self, process, prompt, observation):
        with self.model.count_lock:
            body = self.model.observed_requests[prompt]
            count = self.model.counts[prompt]
        self.assertEqual(count, 1)
        self.assertEqual(observation["system_hmac_sha256"], digest(process.token,
            [m for m in body["messages"] if m["role"] == "system"]))
        self.assertEqual(observation["tools_hmac_sha256"], digest(process.token, body.get("tools", [])))
        self.assertEqual(observation["model_hmac_sha256"], digest(process.token, body["model"]))
        self.assertEqual(observation["home_hmac_sha256"], digest(process.token, str(process.home)))
        self.assertEqual(observation["cwd_hmac_sha256"], digest(process.token, str(process.workspace)))
        self.assertIs(observation["complete"], False)
        self.assertIs(observation["runtime_ready"], False)
        self.assertEqual(observation["request_sequence"], 1)

    def test_real_initialized_owner_reads_match_model_and_isolate_peer(self):
        first, peer = self.process("owner-one"), self.process("owner-peer")
        caps, peer_caps = self.caps(first), self.caps(peer)
        self.assertNotEqual(caps["incarnation"], peer_caps["incarnation"])
        run, release = self.run_held(first, "observe-owner-one")
        status, observed = self.observation(first, run, caps["incarnation"])
        self.assertEqual(status, 200)
        self.compare(first, "observe-owner-one", observed)
        for _ in range(5):
            self.assertEqual(self.observation(first, run, caps["incarnation"]), (200, observed))
        self.assertEqual(self.observation(peer, run, peer_caps["incarnation"])[0], 503)
        self.assertEqual(self.observation(first, run, caps["incarnation"], token="wrong")[0], 401)
        other, other_release = self.run_held(peer, "observe-owner-peer")
        status, other_observed = self.observation(peer, other, peer_caps["incarnation"])
        self.assertEqual(status, 200)
        self.compare(peer, "observe-owner-peer", other_observed)
        self.assertNotEqual(observed["system_hmac_sha256"], other_observed["system_hmac_sha256"])
        raw = json.dumps(observed)
        for secret in (first.token, "Answer the fixture prompt", str(first.home)):
            self.assertNotIn(secret, raw)
        release.set()
        other_release.set()
        self.assertEqual(first.terminal(run)["status"], "completed")
        self.assertEqual(peer.terminal(other)["status"], "completed")
        self.assertEqual(self.observation(first, run, caps["incarnation"]), (200, observed))

    def test_restart_does_not_adopt_old_capture_or_replay_prompt(self):
        process = self.process("restart-owner")
        old = self.caps(process)
        run, release = self.run_held(process, "observe-before-restart")
        self.assertEqual(self.observation(process, run, old["incarnation"])[0], 200)
        release.set()
        self.assertEqual(process.terminal(run)["status"], "completed")
        process.stop()
        process.start()
        current = self.caps(process)
        self.assertNotEqual(old["incarnation"], current["incarnation"])
        for incarnation in (old["incarnation"], current["incarnation"]):
            self.assertEqual(self.observation(process, run, incarnation)[0], 503)
        with self.model.count_lock:
            self.assertEqual(self.model.counts["observe-before-restart"], 1)
        new_run, new_release = self.run_held(process, "observe-after-restart")
        status, observed = self.observation(process, new_run, current["incarnation"])
        self.assertEqual(status, 200)
        self.compare(process, "observe-after-restart", observed)
        new_release.set()
        self.assertEqual(process.terminal(new_run)["status"], "completed")


if __name__ == "__main__":
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(ObservationTests)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    Path(os.environ["HERMES_PROTOCOL_RESULT"]).write_text(json.dumps({
        "native_source_sha": os.environ["HERMES_PROTOCOL_SOURCE_SHA"],
        "cases": ["initialized-digest-owner-isolation", "restart-does-not-adopt-capture"] if result.wasSuccessful() else [],
        "model": "deterministic-loopback", "full_inventory": False, "sdlc_acceptance": False,
    }) + "\n")
    raise SystemExit(0 if result.wasSuccessful() else 1)
