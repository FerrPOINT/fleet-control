"""Pure injection/receipt counterexamples; NOT native/Fleet/PG execution."""
import ast
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch
import uuid

import cut_compile_proof
import cut_contract as c
import cut_run
import cut_transport as t
import packet
import run


def identity(n):
    return str(uuid.UUID(int=n))


def fixture():
    resource, generation, old_controller, new_controller = map(identity,(1,2,3,4))
    snapshot = dict(init_pid=42,started_at="2026-10-09T00:00:00Z")
    registration = dict(resource_id=resource,generation=generation,container_id="a" * 64,operation_id=identity(5))
    original = dict(prepared=dict(container=dict(registration=registration)),controller_id=old_controller,
                    stop_id=identity(6),snapshot=snapshot)
    armed = dict(source_commit=c.SOURCE,resource_id=resource,generation=generation,container_id="a" * 64,
        stop_id=original["stop_id"],registration_sha256=c.digest(registration),snapshot_sha256=c.digest(snapshot),loader_sha256="b" * 64)
    stop = dict(contract_version=3,operation_id=armed["stop_id"],resource_id=resource,generation=generation,
                container_id=armed["container_id"],snapshot_sha256=armed["snapshot_sha256"],state="observed",observation="namespace_exited")
    lease = dict(request=dict(id=identity(7),launch_id=generation,agent_id=resource,original_controller_id=old_controller,
                             controller_id=new_controller),epoch=1,lease_version=2,lease_expires_at="2026-10-09T00:01:00Z")
    records = []
    for i in range(3):
        candidate = dict(generation=identity(10+i*10),operation_id=identity(11+i*10),stop_id=identity(12+i*10))
        rollback = dict(generation=identity(13+i*10),operation_id=identity(14+i*10),stop_id=identity(15+i*10))
        previous = original if i == 0 else records[-1]["activation"]["candidate"]
        claim = dict(id=identity(50+i),revision=i+1,controller_id=old_controller,previous=previous,
            candidate=candidate,rollback=rollback,previous_revision=None if i == 0 else i,
            intent_sha256=str(i+1)*64,files_sha256=str(i+5)*64,previous_files_sha256="0"*64 if i == 0 else str(i+4)*64)
        if i:
            claim["lineage"] = dict(anchor=original,family_id=records[0]["activation"]["claim"]["id"],
                predecessor_activation_id=records[-1]["activation"]["claim"]["id"],
                predecessor_intent_sha256=records[-1]["activation"]["claim"]["intent_sha256"])
        launch = lambda cmd: dict(prepared=dict(container=dict(registration=dict(generation=cmd["generation"],operation_id=cmd["operation_id"]))),stop_id=cmd["stop_id"])
        published = rollback if i == 2 else candidate
        managed = claim["previous_files_sha256"] if i == 2 else claim["files_sha256"]
        activation = dict(claim=claim,phase="rolled_back" if i == 2 else "committed",previous_stop=stop if i == 0 else {},
            candidate=launch(candidate),candidate_stop={} if i == 2 else None,rollback=launch(rollback) if i == 2 else None,
            readiness=dict(generation=published["generation"],files_sha256=managed,capabilities_sha256="c"*64))
        records.append(dict(activation=activation,effective_revision=2 if i == 2 else i+1,launch_generation=published["generation"],
            plan_sha256="d"*64,managed_sha256=managed,lease=lease,
            lease_receipt=dict(ack=dict(state="controller_heartbeat",recovery_id=lease["request"]["id"],lease_version=2,lease_expires_at=lease["lease_expires_at"]),
                receipt=dict(state="observed",observation="namespace_exited",generation=generation,container_id=armed["container_id"],snapshot=snapshot)),
            lease_valid=True,health_running=True,authority_count=1))
    cut = dict(phase="stopping_previous",previous_stop=None,candidate=None,rollback=None,readiness=None,
        claim=records[0]["activation"]["claim"],plan_sha256="d"*64,physical_exit=True,draining=True)
    report = dict(state="scoped_cut_matrix_passed",source_commit=c.SOURCE,matrix=c.MATRIX,cut=cut,records=records,
                  peer_before=dict(pid=99),peer_after=dict(pid=99),runtime_ready=False,sdlc_completion=False)
    events = []
    projected_lease = dict(id=lease["request"]["id"],controller_id=new_controller,launch_id=generation,
        epoch=1,version=2,expires_at=lease["lease_expires_at"])
    def event(name,ack,command=None):
        events.append(dict(call_id=identity(100+len(events)),request=dict(action=name,request_sha256="e"*64,
            anchor_sha256=None if name == "stop" else "8"*64,command=command,
            lease=None if name == "stop" else projected_lease),ack=ack,ack_sha256=c.digest(ack),raw_ack_sha256="f"*64))
    event("stop",dict(protocol_version=2,action="stop",result=stop))
    event("read_original_stop",dict(protocol_version=4,action="read_original_stop",result=stop,request_sha256="e"*64))
    for i, record in enumerate(records):
        claim = record["activation"]["claim"]
        for reserved in ([claim["candidate"],claim["rollback"]] if i == 2 else [claim["candidate"]]):
            command = dict(reserved,predecessor_generation=(claim["candidate"]["generation"] if reserved == claim["rollback"] else claim["previous"]["prepared"]["container"]["registration"]["generation"]),
                intent_sha256=claim["intent_sha256"],command_sha256="9"*64)
            for name in ("prepare_replacement","attach_replacement","start_replacement","stop_replacement"):
                result = dict(state={"prepare_replacement":"prepared","attach_replacement":"attached"}.get(name,"observed"))
                if name in ("start_replacement","stop_replacement"):
                    result["observation"] = "running" if name == "start_replacement" else "namespace_exited"
                event(name,dict(protocol_version=4,action=name,result=result,
                    command_sha256=command["command_sha256"],intent_sha256=command["intent_sha256"],anchor_sha256="8"*64),command)
    return report,armed,events


class ReceiptTests(unittest.TestCase):
    def test_closed_positive_shape_is_only_a_parser_fixture(self):
        c.validate_report(*fixture())

    def test_duplicate_keys_nonfinite_or_truncated_receipt_bytes_refused(self):
        self.assertEqual(c.decode(b'{"known":true}'),dict(known=True))
        for raw in (b'{"state":"held","state":"observed"}',b'{"result":{"x":1,"x":2}}',
                    b'{"result":NaN}',b'{"result":Infinity}',b'{"result":-Infinity}',b'{'):
            with self.subTest(raw=raw),self.assertRaises(ValueError): c.decode(raw)

    def test_gate_requires_exact_actual_ack_and_private_journal_witness(self):
        _,armed,events = fixture()
        gate = dict(event=events[0],stdout_forwarded=False,
            journal=dict(sha256="1"*64,device=1,inode=2,size=32768,mode=0o600,uid=999))
        c.validate_gate(gate,armed)
        for kind in ("forwarded","missingjournal","mode","owner","zero","ackhash"):
            value = copy.deepcopy(gate)
            if kind == "forwarded": value["stdout_forwarded"] = True
            if kind == "missingjournal": value["journal"] = {}
            if kind == "mode": value["journal"]["mode"] = 0o644
            if kind == "owner": value["journal"]["uid"] = 0
            if kind == "zero": value["journal"]["size"] = 0
            if kind == "ackhash": value["event"]["ack_sha256"] = "0"*64
            with self.subTest(kind=kind),self.assertRaises(ValueError): c.validate_gate(value,armed)

    def test_inventory_cannot_hide_duplicate_or_unreserved_generation(self):
        report,armed,_ = fixture()
        report["peer_before"]["launch"] = dict(prepared=dict(container=dict(registration=dict(generation=identity(90)))))
        generations = [armed["generation"],identity(90),identity(10),identity(20),identity(30),identity(33)]
        inventory = {str(i):dict(generation=g) for i,g in enumerate(generations)}
        c.validate_inventory(inventory,report,armed)
        for kind in ("extra","missing","duplicate","foreign"):
            value = copy.deepcopy(inventory)
            if kind == "extra": value["extra"] = dict(generation=identity(999))
            if kind == "missing": value.pop("0")
            if kind == "duplicate": value["0"]["generation"] = identity(10)
            if kind == "foreign": value["0"]["generation"] = identity(999)
            with self.subTest(kind=kind),self.assertRaises(ValueError): c.validate_inventory(value,report,armed)

    def test_missing_extra_or_promoted_matrix_refused(self):
        for key,value in (("runtime_ready",True),("sdlc_completion",True),("state","accepted"),("matrix",{}),("foreign",1)):
            report,armed,events = fixture()
            report[key] = value
            with self.subTest(key=key),self.assertRaises(ValueError): c.validate_report(report,armed,events)

    def test_wrong_phase_or_existing_phase_cas_receipt_refused(self):
        for key,value in (("phase","previous_stopped"),("previous_stop",{}),("candidate",{}),("physical_exit",False),("draining",False)):
            report,armed,events = fixture()
            report["cut"][key] = value
            with self.subTest(key=key),self.assertRaises(ValueError): c.validate_report(report,armed,events)

    def test_original_claim_or_plan_hash_change_refused(self):
        for key in ("claim","plan_sha256"):
            report,armed,events = fixture()
            report["cut"][key] = {} if key == "claim" else "a"*64
            with self.subTest(key=key),self.assertRaises(ValueError): c.validate_report(report,armed,events)

    def test_missing_readback_or_duplicate_original_stop_refused(self):
        for which in ("missing","duplicate"):
            report,armed,events = fixture()
            if which == "missing": events.pop(1)
            else:
                duplicate = copy.deepcopy(events[0]); duplicate["call_id"] = identity(999); events.append(duplicate)
            with self.subTest(which=which),self.assertRaises(ValueError): c.validate_report(report,armed,events)

    def test_wrong_original_stop_identity_or_unknown_observation_refused(self):
        for key,value in (("operation_id",identity(999)),("generation",identity(999)),("container_id","f"*64),
                          ("snapshot_sha256","f"*64),("observation","running"),("state","held")):
            report,armed,events = fixture()
            events[1] = copy.deepcopy(events[1]); events[1]["ack"]["result"][key] = value
            events[1]["ack_sha256"] = c.digest(events[1]["ack"])
            with self.subTest(key=key),self.assertRaises(ValueError): c.validate_report(report,armed,events)

    def test_expired_unknown_old_epoch_and_foreign_custody_refused(self):
        for kind in ("expiry","heartbeat","epoch","controller","anchor","noauthority"):
            report,armed,events = copy.deepcopy(fixture())
            record = report["records"][1]
            if kind == "expiry": record["lease_valid"] = False
            if kind == "heartbeat": record["lease_receipt"] = None
            if kind == "epoch": record["lease"]["epoch"] += 1
            if kind == "controller": record["lease"]["request"]["controller_id"] = record["lease"]["request"]["original_controller_id"]
            if kind == "anchor": record["lease"]["request"]["launch_id"] = identity(888)
            if kind == "noauthority": record["authority_count"] = 0
            with self.subTest(kind=kind),self.assertRaises((ValueError,KeyError,TypeError)): c.validate_report(report,armed,events)

    def test_next_activation_cannot_mint_anchor_or_change_predecessor(self):
        for key,value in (("family_id",identity(999)),("predecessor_activation_id",identity(999)),
                          ("predecessor_intent_sha256","f"*64),("anchor",{})):
            report,armed,events = copy.deepcopy(fixture())
            report["records"][1]["activation"]["claim"]["lineage"][key] = value
            with self.subTest(key=key),self.assertRaises(ValueError): c.validate_report(report,armed,events)

    def test_failed_next_must_restore_current_effective_not_root(self):
        for key,value in (("effective_revision",1),("managed_sha256","5"*64),("launch_generation",identity(20)),("health_running",False)):
            report,armed,events = fixture(); report["records"][2][key] = value
            with self.subTest(key=key),self.assertRaises(ValueError): c.validate_report(report,armed,events)

    def test_missing_duplicate_effect_wrong_hash_or_peer_drift_refused(self):
        for kind in ("missing","duplicate","hash","generation","peer"):
            report,armed,events = copy.deepcopy(fixture())
            if kind == "missing": events.pop()
            if kind == "duplicate":
                value = copy.deepcopy(events[-1]); value["call_id"] = identity(999); events.append(value)
            if kind == "hash": events[-1]["ack_sha256"] = "0"*64
            if kind == "generation": events[-1]["request"]["command"]["generation"] = identity(999)
            if kind == "peer": report["peer_after"]["pid"] = 0
            with self.subTest(kind=kind),self.assertRaises(ValueError): c.validate_report(report,armed,events)

    def test_native_command_anchor_and_physical_ack_cannot_drift_between_phases(self):
        for kind in ("command","anchor","physical","state"):
            report,armed,events = copy.deepcopy(fixture())
            event = events[-1]
            if kind == "command":
                event["request"]["command"]["command_sha256"] = event["ack"]["command_sha256"] = "6"*64
            if kind == "anchor":
                event["request"]["anchor_sha256"] = event["ack"]["anchor_sha256"] = "7"*64
            if kind == "physical": event["ack"]["result"]["observation"] = "running"
            if kind == "state": event["ack"]["result"]["state"] = "prepared"
            event["ack_sha256"] = c.digest(event["ack"])
            with self.subTest(kind=kind),self.assertRaises(ValueError): c.validate_report(report,armed,events)


class TransportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.qualification = cut_run.qualification()
        text = packet.git(run.FLEET,"show",run.SOURCE_MERGED + ":backend/infra/src/runtime/container_control.rs").decode()
        cls.loader = text.split('const BOOTSTRAP: &str = r#"',1)[1].split('"#;',1)[0]
        cls.sources = [packet.git(run.BASE,"show",run.BASE_SHA + ":scripts/" + name).decode() for name in run.UTILITY_HASHES]

    def payload(self):
        report,armed,_ = fixture()
        armed["loader_sha256"] = self.qualification["loader_sha256"]
        request = dict(action="stop",protocol_version=2,registration=report["records"][0]["activation"]["claim"]["previous"]["prepared"]["container"]["registration"],
            operation_id=armed["stop_id"],stop_journal="/controller/stop.sqlite")
        payload = dict(request=request,sources=self.sources,entry="main")
        output = json.dumps(dict(protocol_version=2,action="stop",result=fixture()[2][0]["ack"]["result"])).encode()
        return ["-I","-c",self.loader,"/base-runtime"],json.dumps(payload).encode(),armed,output

    def test_real_ack_latched_after_one_unchanged_delegate_before_return(self):
        args,raw,armed,output = self.payload(); saved = {}
        call = Mock(return_value=(0,output)); pause = Mock(side_effect=InterruptedError("simulated physical interruption"))
        def persist(path,value):
            if path in saved: raise FileExistsError()
            saved[path] = value
        with self.assertRaises(InterruptedError):
            t.relay(args,raw,armed,call,persist,Mock(return_value=dict(sha256="1"*64)),pause,
                    publish=persist,blocked=Mock(return_value=dict(deadline=55)),clock=lambda:0)
        call.assert_called_once_with(args,raw)
        pause.assert_called_once_with(55)
        self.assertFalse(saved[t.EVIDENCE / "cut-stop-ack.json"]["stdout_forwarded"])
        self.assertEqual(saved[t.EVIDENCE / "cut-stop-ack.json"]["event"]["ack"]["result"],json.loads(output)["result"])
        with self.assertRaises(FileExistsError): t.relay(args,raw,armed,call,persist)
        self.assertEqual(call.call_count,1)

    def test_unknown_failed_or_malformed_ack_never_opens_gate_or_retries(self):
        for status,output in ((2,b'{}'),(0,b'{}'),(0,b'not-json')):
            args,raw,armed,_ = self.payload(); saved = {}
            def persist(path,value):
                if path in saved: raise FileExistsError()
                saved[path] = value
            call = Mock(return_value=(status,output))
            with self.subTest(status=status,output=output),self.assertRaises(ValueError):
                t.relay(args,raw,armed,call,persist)
            self.assertNotIn(t.EVIDENCE / "cut-stop-ack.json",saved)
            with self.assertRaises(FileExistsError): t.relay(args,raw,armed,call,persist)
            self.assertEqual(call.call_count,1)

    def test_withholding_deadline_includes_durable_publication_time(self):
        args,raw,armed,output = self.payload()
        clock = [0]
        pause = Mock(side_effect=InterruptedError("simulated physical interruption"))
        def publish(path,value):
            clock[0] += 7  # Actual relay control flow, deterministic slow filesystem seam.
        with self.assertRaises(InterruptedError):
            t.relay(args,raw,armed,Mock(return_value=(0,output)),Mock(),
                    Mock(return_value=dict(sha256="1"*64)),pause,publish=publish,
                    blocked=Mock(return_value=dict(deadline=55)),clock=lambda:clock[0])
        pause.assert_called_once_with(41)

    def test_noncut_known_or_held_bytes_pass_through_unchanged(self):
        args,raw,armed,_ = self.payload(); payload = json.loads(raw); payload["request"]["action"] = "observe"
        raw = json.dumps(payload).encode()
        for status,output in ((0,b'original observed bytes\n'),(2,b'original held bytes\n')):
            call,persist = Mock(return_value=(status,output)),Mock()
            self.assertEqual(t.relay(args,raw,armed,call,persist),(status,output))
            call.assert_called_once_with(args,raw); persist.assert_not_called()

    def test_changed_loader_root_module_or_identity_denied_before_delegate(self):
        for kind in ("loader","root","module","identity"):
            args,raw,armed,_ = self.payload()
            if kind == "loader": args[2] += '\n'
            if kind == "root": args[3] = "/foreign"
            if kind == "module":
                payload = json.loads(raw); payload["sources"][3] += '\n'; raw = json.dumps(payload).encode()
            if kind == "identity": armed["source_commit"] = "0"*40
            call = Mock(side_effect=AssertionError("no native effects"))
            with self.subTest(kind=kind),self.assertRaises(ValueError): t.relay(args,raw,armed,call,Mock())
            call.assert_not_called()

    def test_missing_journal_proof_does_not_open_gate(self):
        args,raw,armed,output = self.payload(); persist = Mock()
        with self.assertRaises(ValueError):
            t.relay(args,raw,armed,Mock(return_value=(0,output)),persist,Mock(side_effect=ValueError("missing original journal")))
        self.assertNotIn(t.EVIDENCE / "cut-stop-ack.json",[args[0] for args,_ in persist.call_args_list])

    def test_wrong_original_stop_id_generation_registration_or_path_has_no_effects(self):
        for kind in ("stopid","generation","registration","path"):
            args,raw,armed,_ = self.payload(); payload = json.loads(raw)
            if kind == "stopid": payload["request"]["operation_id"] = identity(999)
            if kind == "generation": payload["request"]["registration"]["generation"] = identity(999)
            if kind == "registration": payload["request"]["registration"]["container_id"] = "f"*64
            if kind == "path": payload["request"]["stop_journal"] = "/foreign/stop.sqlite"
            call = Mock(side_effect=AssertionError("no effects"))
            with self.subTest(kind=kind),self.assertRaises(ValueError):
                t.relay(args,json.dumps(payload).encode(),armed,call,Mock())
            call.assert_not_called()

    def test_delegate_timeout_propagates_and_only_owns_its_subprocess(self):
        import subprocess
        with patch.object(t.subprocess,"run",side_effect=subprocess.TimeoutExpired("owned",55)) as process:
            with self.assertRaises(subprocess.TimeoutExpired): t.delegate(["-I","-c","owned"],b"private")
        self.assertEqual(process.call_args.kwargs["timeout"],55)
        self.assertEqual(process.call_args.args[0],[t.sys.executable,"-I","-c","owned"])


class SourceTests(unittest.TestCase):
    def test_new_namespace_does_not_mutate_original_helper_or_pins(self):
        self.assertEqual(run.PROJECT_PREFIX,"sdlc-qa-fleet-native4-")
        self.assertEqual(cut_run.driver.PROJECT_PREFIX,"sdlc-qa-fleet-native4cut-")
        self.assertEqual(cut_run.driver.SOURCE_MERGED,run.SOURCE_MERGED)
        self.assertEqual(cut_run.driver.SDK_SHA,run.SDK_SHA)
        self.assertEqual(cut_run.driver.UTILITY_HASHES,run.UTILITY_HASHES)
        self.assertEqual(cut_run.driver.VOLUMES,run.VOLUMES)
        self.assertEqual(cut_run.driver.NETWORKS,run.NETWORKS)
        self.assertEqual(cut_run.driver.EXPECTED_MATRIX,run.EXPECTED_MATRIX)

    def test_old_grant_refused_before_new_packet_or_native_io(self):
        with patch.object(cut_run.driver,"verify",side_effect=AssertionError("no I/O")),self.assertRaises(ValueError):
            cut_run.driver.execute(Path("unused"),"exclusive-native4-b249bc895e51","desktop-linux")

    def test_new_extra_lock_preserves_every_product_record_and_dependency(self):
        product = run.FLEET / "backend/Cargo.lock"
        self.assertEqual(run.derive_qa_lock(product,run.HERE / "live/Cargo.toml"),
                         run.derive_qa_lock(product,run.HERE / "cut-live/Cargo.toml"))

    def test_cut_services_preserve_socket_ownership_and_resource_budgets(self):
        before,after = run.services(Path("/owned"),run.SOURCE_MERGED),cut_run.services(Path("/owned"),run.SOURCE_MERGED)
        for name in before:
            if name != "build": self.assertEqual(before[name],after[name])
        expected = copy.deepcopy(before["build"]); expected["entrypoint"] = after["build"]["entrypoint"]
        expected["volumes"] = after["build"]["volumes"]
        self.assertEqual(expected,after["build"])
        self.assertTrue(all(m["read_only"] for m in after["build"]["volumes"] if m["type"] == "bind" and m["target"].startswith("/qa/")))

    def test_no_sql_receipt_or_custody_mutation_and_original_assertions_retained(self):
        source = (run.HERE / "cut-live/src/scenario.rs").read_text()
        self.assertNotIn("UPDATE ",source)
        self.assertNotIn("INSERT INTO runtime_",source)
        self.assertNotIn("INSERT INTO agent_config",source)
        self.assertIn("INSERT INTO users",source)  # Original synthetic fixture owner only.
        self.assertIn("SELECT count(*)::bigint",source)
        before = packet.git(run.HERE.parent,"show","df6574bfe5d26445b9ce31a8f396d42914c8c653:qa/live/src/main.rs")
        current = (run.HERE / "live/src/main.rs").read_bytes().replace(b'\r\n',b'\n')
        current = current.replace(b"    assert_eq!(session.task_bound, Some(false));\n", b"")
        current = current.replace(b"    assert_eq!(\n        message.request_payload_hash,\n        Some(hex::encode(Sha256::digest(\n            serde_json::to_vec(&serde_json::to_value(&request).unwrap()).unwrap()\n        )))\n    );\n", b"")
        self.assertEqual(current,before)
        build = (run.HERE / "build_cuts.sh").read_text()
        self.assertIn("bash /qa/build.sh",build)
        self.assertIn("CARGO_HOME=/cargo CARGO_TARGET_DIR=/target",build)
        self.assertIn("--locked --manifest-path cut-live/Cargo.toml",build)

    def test_actual_cut_artifact_missing_wrong_source_or_test_profile_refused(self):
        root,exe = Path("/scratch/src"),Path("/target/debug/fleet-native-acceptance")
        record = dict(reason="compiler-artifact",target=dict(name="fleet-native-acceptance",kind=["bin"],crate_types=["bin"],
            src_path=str(root / "cut-live/src/main.rs")),executable=str(exe),profile=dict(test=False))
        records = [record,dict(reason="build-finished",success=True)]
        cut_compile_proof.artifact(records,root,exe)
        for kind in ("missing","wrongsource","test"):
            changed = copy.deepcopy(records)
            if kind == "missing": changed.pop()
            if kind == "wrongsource": changed[0]["target"]["src_path"] = str(root / "live/src/main.rs")
            if kind == "test": changed[0]["profile"]["test"] = True
            with self.subTest(kind=kind),self.assertRaises(ValueError): cut_compile_proof.artifact(changed,root,exe)

    def test_both_compile_proofs_gate_postgres_and_native_startup(self):
        import inspect
        source = inspect.getsource(run.execute)
        original = source.index('report["compile_proof"] = verify_compile_proof')
        cut = source.index('report["cut_compile_proof"] = compile_validator')
        startup = source.index('phase = "startup"')
        self.assertLess(original,cut)
        self.assertLess(cut,startup)
        self.assertIn("compile_validator=cut_compile_proof.verify",inspect.getsource(cut_run.main))


if __name__ == "__main__":
    unittest.main()
