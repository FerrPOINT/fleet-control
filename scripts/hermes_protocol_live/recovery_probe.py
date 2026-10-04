#!/usr/bin/env python3
"""Opt-in native hook + real SQLite/AIAgent recovery with no resubmission."""
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import threading
import unittest
import uuid
import native_protocol as native

CAPS = "/fleet/v1/recovery/capabilities"
LOOKUP = "/fleet/v1/recovery/lookup"


class RecoveryTests(native.NativeProtocolTests):
    # Reuse only fixtures/assertions, not the legacy POST-replay scenarios.
    test_native_sse_and_two_isolated_agents = None
    test_lost_ack_concurrent_replay_and_process_restart = None
    test_rotated_credential_is_not_recovery_scope = None
    test_unknown_ack_crash_during_inference_never_reexecutes = None

    def native(self, name):
        process = native.NativeProcess(Path(self.directory.name) / name, self.model.server_port,
            recovery_plugin=Path(__file__).with_name("plugin"))
        self.addCleanup(process.stop)
        process.start()
        return process

    def caps(self, process):
        code, result = native.json_request(process.port, CAPS, token=process.token)
        self.assertEqual(code, 200)
        self.assertEqual(set(result), {"object","contract_version","store_id","profile",
            "scope_fingerprint","native_source_revision","lookup","non_dispatch","durable_witness"})
        self.assertEqual(result["scope_fingerprint"], hashlib.sha256(("default\0" + process.token).encode()).hexdigest())
        self.assertEqual(result["native_source_revision"], os.environ["HERMES_PROTOCOL_SOURCE_SHA"])
        self.assertIs(result["non_dispatch"], True)
        self.assertIs(result["durable_witness"], True)
        self.assertEqual(str(uuid.UUID(result["store_id"])), result["store_id"])
        return result

    def packet(self, caps, key, body):
        raw = json.dumps(body, sort_keys=True, separators=(",", ":"))
        return {"contract_version":1, "store_id":caps["store_id"], "idempotency_key":key,
                "request_json":raw, "request_sha256":hashlib.sha256(raw.encode()).hexdigest()}

    def lost(self, process, body, key, store_id):
        proxy = native.ThreadingHTTPServer(("127.0.0.1", 0), native.LostAckHandler)
        proxy.upstream_port, proxy.token = process.port, process.token
        proxy.forwarded = threading.Event()
        thread = threading.Thread(target=proxy.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(thread.join, 10)
        self.addCleanup(proxy.server_close)
        self.addCleanup(proxy.shutdown)
        with self.assertRaises(OSError):
            native.request(proxy.server_port, "/v1/runs", body=body, key=key, store_id=store_id)
        self.assertTrue(proxy.forwarded.wait(5))
        self.assertEqual(proxy.upstream_status, 202)
        self.assertIs(proxy.accepted["replayed"], False)
        return proxy.accepted["run_id"]

    def test_recovery_lost_ack_restart_lookup_does_not_call_model(self):
        process = self.native("recovery-lost")
        caps = self.caps(process)
        body = {"input":"native-recovery-lost-ack", "session_id":"fleet:recovery:lost"}
        key = str(uuid.uuid4())
        self.assertEqual(native.request(process.port, "/v1/runs", token=process.token, body=body, key=key)[0],409)
        self.assertEqual(native.request(process.port, CAPS, token="wrong-native-fixture")[0],401)
        self.assertEqual(native.request(process.port, LOOKUP, token="wrong-native-fixture", body={})[0],401)
        run_id = self.lost(process, body, key, caps["store_id"])
        result = process.terminal(run_id)
        self.completed(result,run_id,body["session_id"])
        old_pid = process.process.pid
        process.stop(crash=True)
        process.start()
        self.assertNotEqual(process.process.pid, old_pid)
        self.assertEqual(self.caps(process),caps)
        packet = self.packet(caps,key,body)
        with ThreadPoolExecutor(max_workers=8) as pool:
            replies = list(pool.map(lambda _: native.json_request(process.port,LOOKUP,token=process.token,body=packet),range(8)))
        self.assertTrue(all(code==200 and found["run_id"]==run_id and found["found"] is True for code,found in replies))
        code, recovered = native.json_request(process.port,"/v1/runs/"+run_id,token=process.token)
        self.assertEqual(code,200)
        self.completed(recovered,run_id,body["session_id"])
        self.assertEqual(self.model.counts.get(body["input"]),1)
        self.transcript(process,body["session_id"],body["input"])
        self.evidence["cases"].append("opt-in-native-hook-dropped-202-crash-eight-non-dispatch-lookups-original-id-one-inference")

    def test_recovery_prune_tombstone_and_reset_never_authorize_replacement(self):
        process = self.native("recovery-prune")
        caps = self.caps(process)
        body = {"input":"native-recovery-prune", "session_id":"fleet:recovery:prune"}
        key = str(uuid.uuid4())
        code, ack = native.json_request(process.port,"/v1/runs",token=process.token,body=body,key=key,store_id=caps["store_id"])
        self.assertEqual(code,202)
        run_id = ack["run_id"]
        process.terminal(run_id)
        packet = self.packet(caps,key,body)
        changed = self.packet(caps,key,{**body,"input":"changed-native-recovery-body"})
        self.assertEqual(native.request(process.port,LOOKUP,token=process.token,body=changed)[0],409)
        missing = self.packet(caps,str(uuid.uuid4()),body)
        self.assertEqual(native.request(process.port,LOOKUP,token=process.token,body=missing)[0],404)
        process.stop(crash=True)
        # Controlled producer failure injection. Not Fleet opening runtime SQLite.
        database = process.home / "runs_idempotency.db"
        with sqlite3.connect(database) as db:
            db.execute("DELETE FROM run_idempotency WHERE scope=? AND idempotency_key=?",(caps["scope_fingerprint"],key))
        process.start()
        code, found = native.json_request(process.port,LOOKUP,token=process.token,body=packet)
        self.assertEqual(code,200)
        self.assertEqual(found["run_id"],run_id)
        self.assertNotEqual(native.request(process.port,"/v1/runs",token=process.token,body=body,key=key,store_id=caps["store_id"])[0],202)
        self.assertEqual(self.model.counts.get(body["input"]),1)
        process.stop(crash=True)
        database.unlink()
        for suffix in ("-wal","-shm","-journal"):
            Path(str(database)+suffix).unlink(missing_ok=True)
        process.start()
        self.assertNotEqual(self.caps(process)["store_id"],caps["store_id"])
        self.assertEqual(native.request(process.port,LOOKUP,token=process.token,body=packet)[0],409)
        self.assertEqual(native.request(process.port,"/v1/runs",token=process.token,body=body,key=key,store_id=caps["store_id"])[0],409)
        self.assertEqual(self.model.counts.get(body["input"]),1)
        self.evidence["cases"].append("native-prune-witness-original-id-key-tombstone-and-reset-old-epoch-denied-no-inference")


if __name__ == "__main__":
    unittest.main(verbosity=2)
