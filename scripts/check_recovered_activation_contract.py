"""Fleet's sealed four-module loader and Base4 CLI under the original private fake Engine.

This exercises utility transport/custody, not Rust/PG phases or real API readiness.
"""
import argparse
import copy
from contextlib import closing
import io
import json
from pathlib import Path
import re
import sqlite3
import subprocess
import sys
import types
import unittest
import uuid
from unittest.mock import patch

from verify_container_utilities import BASE_REVISION, NAMES, verify


SELECTORS = (
    "test_positive_prepare_attach_start_readback_and_exact_rollback_generation",
    "test_unknown_accepted_create_reconciles_without_second_namespace",
    "test_unknown_absent_create_never_resends_or_forks",
    "test_crash_after_command_claim_before_native_claim_is_held",
    "test_unknown_attachment_accepted_and_absent_never_repeat_connect",
    "test_unknown_start_without_original_ack_stays_held",
    "test_unknown_stop_running_blocks_successor_then_original_exit_readback_recovers",
    "test_changed_payload_credentials_image_mapping_and_owner_never_dispatch",
    "test_engine_controller_volume_and_process_only_restart_cannot_gain_custody",
    "test_next_proven_controller_restart_resumes_original_command_and_attachment",
    "test_original_stop_ack_before_phase_cas_readback_has_no_effects_or_writes",
    "test_original_stop_missing_or_wrong_claim_never_creates_a_permit",
    "test_original_stop_without_start_ack_or_exit_is_held",
    "test_original_stop_foreign_or_unacknowledged_lease_is_held",
    "test_terminal_child_stop_known_and_unknown_ack_reconcile_without_repeat",
    "test_committed_child_next_revision_uses_original_scope_and_credentials",
    "test_rolled_back_child_next_revision_remains_linear",
    "test_next_revision_expired_original_lease_has_no_effects",
    "test_next_revision_changed_epoch_requires_new_original_proof",
    "test_duplicate_next_plan_conflicts_without_new_effects",
    "test_next_failure_rollback_uses_current_effective_predecessor",
)


def sealed_modules(base):
    verify(base)
    source = Path(__file__).resolve().parents[1] / "backend/infra/src/runtime/container_control.rs"
    blocks = re.findall(r'const BOOTSTRAP: &str = r#"(.*?)"#;', source.read_text(), re.S)
    if len(blocks) != 1 or not blocks[0].endswith("raise SystemExit(package.runtime_control.main())\n"):
        raise ValueError("sealed loader entry point changed")
    sources = [subprocess.run(["git", "-c", f"safe.directory={base.as_posix()}", "show", f"{BASE_REVISION}:scripts/{name}"],
                             cwd=base, capture_output=True, check=True).stdout.decode("utf-8")
               for name in NAMES]
    # Pause immediately before main only to install the fake Engine; utility bytes stay exact.
    prefix = blocks[0].rsplit("raise SystemExit", 1)[0]
    with patch.object(sys, "stdin", io.StringIO(json.dumps({"sources": sources, "request": {}}))), \
            patch.object(sys, "argv", ["fleet-sealed-contract", str(base)]):
        namespace = {}
        exec(compile(prefix, "fleet-sealed-loader", "exec"), namespace)
    tests = types.ModuleType("scripts.tests")
    tests.__path__ = [str(base / "scripts/tests")]
    sys.modules["scripts.tests"] = tests
    sys.modules["scripts"].tests = tests
    return namespace["original_stop_readback"]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", required=True, type=Path)
    args = parser.parse_args()
    if sys.platform != "linux":
        parser.exit(2, "Linux private-filesystem/boot-clock gate required; skips are not PASS\n")
    original_stop_readback = sealed_modules(args.base.resolve())
    from scripts import runtime_control as control
    from scripts.tests.test_runtime_replacement import ReplacementTests

    class FleetSealedCLI(ReplacementTests):
        def recovery_command(self, seconds=30):
            # Use the supported 30-second bound, not a fake clock or relaxed lease guard.
            return super().recovery_command(seconds=seconds)

        def original_stop(self, request=None):
            return original_stop_readback(request or {"anchor":self.anchor,
                "operation_id":self.stop_id,"stop_journal":self.stop_path}, lambda _:self.engine)

        def test_original_stop_ack_before_phase_cas_readback_has_no_effects_or_writes(self):
            # Fixture stops the original, but Fleet's hypothetical PG phase is still StoppingPrevious.
            before = self.effects()
            evidence = {str(p):p.read_bytes() for p in self.root.iterdir() if p.is_file()}
            envelope = self.original_stop()
            self.assertEqual(envelope["result"],self.previous)
            self.assertEqual(self.original_stop(),envelope)
            from scripts.runtime_boundary import digest
            self.assertEqual(envelope["request_sha256"],digest({"anchor":self.anchor,
                "operation_id":self.stop_id,"stop_journal":self.stop_path}))
            self.assertEqual(self.effects(),before)
            self.assertEqual(evidence,{str(p):p.read_bytes() for p in self.root.iterdir() if p.is_file()})

        def test_original_stop_missing_or_wrong_claim_never_creates_a_permit(self):
            from scripts.runtime_boundary import BoundaryError
            before = self.effects()
            evidence = Path(self.stop_path).read_bytes()
            wrong = {"anchor":self.anchor,"operation_id":str(uuid.uuid4()),"stop_journal":self.stop_path}
            with self.assertRaises((BoundaryError,OSError)): self.original_stop(wrong)
            self.assertEqual(Path(self.stop_path).read_bytes(),evidence)
            Path(self.stop_path).unlink()
            with self.assertRaises((BoundaryError,OSError)): self.original_stop()
            self.assertFalse(Path(self.stop_path).exists())
            self.assertEqual(self.effects(),before)

        def test_original_stop_without_start_ack_or_exit_is_held(self):
            from scripts.runtime_boundary import BoundaryError
            before = self.effects()
            with closing(sqlite3.connect(self.anchor["journal"])) as db:
                snapshot=json.loads(db.execute("SELECT snapshot_json FROM launches").fetchone()[0])
            self.engine.original.item["State"].update(Status="running",Running=True,Pid=snapshot["init_pid"],ExitCode=0)
            self.assertEqual(self.original_stop()["result"]["state"],"held")
            self.engine.original.exited()
            with closing(sqlite3.connect(self.anchor["journal"])) as db:
                db.execute("UPDATE launches SET snapshot_json=NULL")
                db.commit()
            with self.assertRaises(BoundaryError): self.original_stop()
            self.assertEqual(self.effects(),before)

        def test_original_stop_foreign_or_unacknowledged_lease_is_held(self):
            from scripts.runtime_boundary import BoundaryError
            before = self.effects()
            for key in ("controller_id","lease_version"):
                request={"anchor":copy.deepcopy(self.anchor),"operation_id":self.stop_id,"stop_journal":self.stop_path}
                if key=="controller_id": request["anchor"]["recovery"]["request"][key]=str(uuid.uuid4())
                else: request["anchor"]["recovery"][key]+=1
                with self.assertRaises(BoundaryError): self.original_stop(request)
            self.assertEqual(self.effects(),before)

        def test_terminal_child_stop_known_and_unknown_ack_reconcile_without_repeat(self):
            self.assertEqual(self.run_action()["state"],"prepared")
            self.assertEqual(self.run_action("attach_replacement")["state"],"attached")
            self.assertEqual(self.run_action("start_replacement")["observation"],"running")
            self.child.stay_running=self.child.lost_ack=True
            self.assertEqual(self.run_action("stop_replacement")["state"],"held")
            self.assertEqual(self.run_action("reconcile_replacement_stop")["state"],"held")
            self.child.exited()
            self.assertEqual(self.run_action("reconcile_replacement_stop")["observation"],"namespace_exited")
            self.assertEqual(self.child.kills,1)

        def run_action(self, action="prepare_replacement", request=None):
            request = dict(request or self.request, action=action)
            output = io.StringIO()
            execute = control.execute
            with patch.object(control, "execute", side_effect=lambda value: execute(value, lambda _: self.engine)):
                code = control.main(io.BytesIO(json.dumps(request).encode()), output)
            envelope = json.loads(output.getvalue())
            if code == 0:
                self.assertEqual(envelope["protocol_version"], 4)
                self.assertEqual(envelope["action"], action)
                from scripts.runtime_boundary import digest
                self.assertEqual(envelope["command_sha256"], digest(request["command"]))
                self.assertEqual(envelope["intent_sha256"], request["command"]["intent_sha256"])
            else:
                self.assertIn(code, (1, 2))
            return envelope.get("result", envelope)

        def live(self, request):
            prepared=self.run_action(request=request)
            self.assertEqual(prepared["state"],"prepared",prepared)
            self.assertEqual(self.run_action("attach_replacement",request)["state"],"attached")
            started=self.run_action("start_replacement",request)
            self.assertEqual(started.get("observation"),"running",started)
            return prepared

        def successor(self, request):
            stopped=self.run_action("stop_replacement",request)
            self.assertEqual(stopped["observation"],"namespace_exited",stopped)
            return self.new_command(request["command"]["policy"]["generation"],stopped,request["command"]["stop_journal"])

        def test_committed_child_next_revision_uses_original_scope_and_credentials(self):
            self.live(self.request)
            original=self.retained_files()
            successor,child=self.successor(self.request)
            prepared=self.live(successor)
            self.assertEqual(successor["anchor"],self.request["anchor"])
            self.assertEqual(successor["replacement_journal"],self.request["replacement_journal"])
            self.assertEqual(successor["command"]["process"],self.request["command"]["process"])
            self.assertEqual(child.item["Config"]["Env"],self.child.item["Config"]["Env"])
            self.assertNotEqual(prepared["registration"]["generation"],self.request["command"]["policy"]["generation"])
            # Original immutable receipts/credentials survive; only stop and root scope append.
            for path,body in original.items():
                if path not in (Path(self.request["replacement_journal"]).name,Path(self.request["command"]["stop_journal"]).name):
                    self.assertEqual((self.root/path).read_bytes(),body)
            before=self.effects()
            self.assertEqual(self.run_action("reconcile_replacement",successor),prepared)
            self.assertEqual(self.effects(),before)
            self.assert_held("start_replacement",self.request)

        def test_rolled_back_child_next_revision_remains_linear(self):
            self.live(self.request)
            restored,_=self.successor(self.request)
            self.live(restored)
            successor,_=self.successor(restored)
            self.live(successor)
            self.assertEqual(successor["anchor"],self.anchor)
            with closing(sqlite3.connect(successor["replacement_journal"])) as db:
                chain=db.execute("SELECT generation,predecessor FROM commands ORDER BY rowid").fetchall()
            self.assertEqual(chain,[(self.request["command"]["policy"]["generation"],self.anchor["policy"]["generation"]),
                (restored["command"]["policy"]["generation"],self.request["command"]["policy"]["generation"]),
                (successor["command"]["policy"]["generation"],restored["command"]["policy"]["generation"])])

        def test_next_revision_expired_original_lease_has_no_effects(self):
            self.live(self.request)
            successor,child=self.successor(self.request)
            with closing(sqlite3.connect(self.anchor["recovery_journal"])) as db:
                db.execute("UPDATE owner_lease SET deadline_ns=0")
                db.commit()
            self.assert_held(request=successor)
            self.assertEqual(child.creates,0)
            with closing(sqlite3.connect(successor["replacement_journal"])) as db:
                self.assertEqual(db.execute("SELECT count(*) FROM commands").fetchone(),(1,))

        def test_next_revision_changed_epoch_requires_new_original_proof(self):
            from datetime import datetime,timedelta,timezone
            self.live(self.request)
            successor,child=self.successor(self.request)
            unknown=copy.deepcopy(successor)
            unknown["anchor"]["recovery"]["lease_version"]+=1
            self.assert_held(request=unknown)
            with closing(sqlite3.connect(self.anchor["recovery_journal"])) as db:
                db.execute("UPDATE owner_lease SET deadline_ns=0")
                db.commit()
            self.item["State"].update(Pid=777,StartedAt="2026-10-06T12:00:00Z")
            witness=self.call("observe_controller_restart",registration=self.anchor["registration"])
            fresh=copy.deepcopy(self.anchor)
            r=fresh["recovery"]
            r["request"].update(id=str(uuid.uuid4()),controller_id=str(uuid.uuid4()),predecessor_id=self.anchor["recovery"]["request"]["id"],
                controller_snapshot=witness["current_controller_snapshot"])
            r.update(epoch=2,lease_version=1,lease_expires_at=(datetime.now(timezone.utc)+timedelta(seconds=20)).isoformat())
            self.assertEqual(self.owner_call(fresh,"recover_controller")["state"],"controller_recovered")
            self.assert_held(request=successor)
            self.assertEqual(child.creates,0)
            successor["anchor"]=fresh
            self.live(successor)

        def test_duplicate_next_plan_conflicts_without_new_effects(self):
            self.live(self.request)
            successor,_=self.successor(self.request)
            prepared=self.live(successor)
            before=self.effects()
            self.assertEqual(self.run_action(request=successor),prepared)
            self.assertEqual(self.effects(),before)
            conflict=copy.deepcopy(successor)
            conflict["command"]["intent_sha256"]="f"*64
            self.assert_held(request=conflict)
            fork,_=self.new_command(successor["command"]["predecessor_generation"],
                successor["command"]["previous_stop"]["receipt"],successor["command"]["previous_stop"]["journal"])
            self.assert_held(request=fork)

        def test_next_failure_rollback_uses_current_effective_predecessor(self):
            self.live(self.request)  # Existing effective child, not the exited root.
            candidate,child=self.successor(self.request)
            self.live(candidate)
            child.stay_running=child.lost_ack=True
            unknown=self.run_action("stop_replacement",candidate)
            self.assertEqual(unknown["state"],"held")
            expected=copy.deepcopy(unknown)
            expected.update(state="observed",observation="namespace_exited")
            guessed,_=self.new_command(candidate["command"]["policy"]["generation"],
                expected,candidate["command"]["stop_journal"])
            self.assert_held(request=guessed)
            child.exited()
            stopped=self.run_action("reconcile_replacement_stop",candidate)
            self.assertEqual(stopped["observation"],"namespace_exited")
            self.assertEqual(stopped,expected)
            restored=guessed  # Same reserved rollback operation/generation, never rekey on unknown.
            self.live(restored)
            self.assertEqual(restored["command"]["process"],self.request["command"]["process"])
            self.assertEqual(restored["anchor"],self.anchor)
            self.assertEqual(child.kills,1)

    # Explicit selectors run only these cases; do not silently inherit the Base suite count.
    suite = unittest.TestSuite(FleetSealedCLI(name) for name in SELECTORS)
    if suite.countTestCases()!=21:
        raise SystemExit("Mandatory recovered activation selectors changed")
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() and result.testsRun == 21 and not result.skipped else 1


if __name__ == "__main__":
    sys.exit(main())
