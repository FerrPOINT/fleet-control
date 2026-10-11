"""Pure/read-only Git qualification. No daemon, Rust compiler, PG or network."""
import ast
import copy
import hashlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib
import types
import unittest
from unittest.mock import Mock, patch
import uuid

sys.dont_write_bytecode = True
import hermes_fixture as fixture
import compile_proof
import packet
import run

PROJECT = "sdlc-qa-fleet-native4-0123456789ab"


def original_creation():
    # Execute only pinned pure Base functions; never instantiate its Engine.
    scripts = types.ModuleType("scripts")
    scripts.__path__ = []
    boundary = types.ModuleType("scripts.runtime_boundary")
    bootstrap = types.ModuleType("scripts.runtime_bootstrap")
    with patch.dict(sys.modules, {"scripts":scripts, "scripts.runtime_boundary":boundary}):
        exec(compile(packet.git(run.BASE,"show",run.BASE_SHA + ":scripts/runtime_boundary.py"),
                     "pinned/runtime_boundary.py","exec"),boundary.__dict__)
        exec(compile(packet.git(run.BASE,"show",run.BASE_SHA + ":scripts/runtime_bootstrap.py"),
                     "pinned/runtime_bootstrap.py","exec"),bootstrap.__dict__)
    generation = str(uuid.UUID(int=42))
    name = "agent1-runtime-" + generation.replace("-","")
    policy = dict(contract_version=3,project=PROJECT,service=name,resource_id=str(uuid.UUID(int=1)),
        generation=generation,image_id=run.HERMES_IMAGE,task=run.TASK,purpose=run.PURPOSE,
        network=dict(id="0" * 64,name=PROJECT + "-" + name,internal=True),
        mounts=[dict(type="volume",source=PROJECT + "_agents",destination="/" + area,
                     read_only=area == "runtime",subpath="agent1/" + area)
                for area in ("runtime","config","workspace","logs")])
    process = dict(user="999:999",entrypoint=["/opt/hermes/.venv/bin/python","/runtime/hermes_fixture.py"],
        command=["serve","--host","0.0.0.0","--port","29100"],
        environment={"HOME":"/config","HERMES_HOME":"/config","API_SERVER_KEY":"synthetic-$-\u043a\u043b\u044e\u0447"},
        working_dir="/workspace",pids_limit=128,memory_bytes=1073741824,nano_cpus=1000000000)
    return bootstrap.creation_spec(policy,process)


class DriverTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.creation = original_creation()

    def test_base169_original_creation_accepted_without_custody_relabel(self):
        native, networks = run.native_manifests([self.creation],PROJECT)
        self.assertEqual(set(native),set(self.creation["services"]))
        self.assertEqual(networks,self.creation["networks"])
        service = next(iter(native.values()))
        self.assertNotIn("sdlc.cleanup-id",service["labels"])
        self.assertEqual(service["environment"]["API_SERVER_KEY"],"synthetic-$$-\u043a\u043b\u044e\u0447")

    def test_native_cleanup_rejects_foreign_project(self):
        changed = copy.deepcopy(self.creation)
        changed["name"] = "sdlc1"
        with self.assertRaises(ValueError):
            run.native_manifests([changed],PROJECT)

    def test_native_cleanup_rejects_foreign_owner_and_image(self):
        for change in ("task","purpose","image"):
            with self.subTest(change=change):
                changed = copy.deepcopy(self.creation)
                service = next(iter(changed["services"].values()))
                if change == "image":
                    service["image"] = "sha256:" + "0" * 64
                else:
                    service["labels"]["sdlc." + change] = "foreign"
                with self.assertRaises(ValueError):
                    run.native_manifests([changed],PROJECT)

    def test_native_cleanup_rejects_shared_volume_and_foreign_network(self):
        for change in ("volume","network","internal","custody"):
            with self.subTest(change=change):
                changed = copy.deepcopy(self.creation)
                definition = next(iter(changed["networks"].values()))
                if change == "volume":
                    changed["volumes"]["agent_storage"]["name"] = "runtime-protected"
                elif change == "network":
                    definition["name"] = "sdlc-common"
                elif change == "internal":
                    definition["internal"] = False
                else:
                    definition["labels"] = dict(definition["labels"], foreign="owner")
                with self.assertRaises(ValueError):
                    run.native_manifests([changed],PROJECT)

    def test_native_cleanup_rejects_duplicate_service_and_generation_drift(self):
        with self.assertRaises(ValueError):
            run.native_manifests([self.creation,self.creation],PROJECT)
        changed = copy.deepcopy(self.creation)
        next(iter(changed["services"].values()))["labels"]["sdlc.boundary.generation"] = str(uuid.UUID(int=43))
        with self.assertRaises(ValueError):
            run.native_manifests([changed],PROJECT)

    def test_original_canonical_utility_hashes(self):
        for name, expected in run.UTILITY_HASHES.items():
            raw = packet.git(run.BASE,"show",run.BASE_SHA + ":scripts/" + name)
            self.assertNotIn(b"\r\n",raw)
            self.assertEqual(hashlib.sha256(raw).hexdigest(),expected)

    def test_exact_merged_source_and_all_ancestors_required(self):
        self.assertTrue(all(run.prerequisites(run.SOURCE_MERGED).values()))
        with self.assertRaises(ValueError):
            run.prerequisites(run.SOURCE18)

    def test_raw_git_export_ignores_checkout_crlf_and_export_ignore(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            repo = root / "repo"
            repo.mkdir()
            packet.checked(["git","init","-q",str(repo)])
            packet.git(repo,"config","core.autocrlf","true")
            packet.git(repo,"config","user.email","qa@example.invalid")
            packet.git(repo,"config","user.name","Pure fixture")
            (repo / ".gitattributes").write_bytes(b"hidden.txt export-ignore\n*.txt text eol=lf\n")
            (repo / "hidden.txt").write_bytes("canonical \u0442\u0435\u043a\u0441\u0442\n".encode())
            (repo / "binary.bin").write_bytes(bytes(range(256)))
            packet.git(repo,"add",".")
            packet.git(repo,"commit","-qm","pure export fixture")
            revision = packet.git(repo,"rev-parse","HEAD").decode().strip()
            (repo / "hidden.txt").write_bytes(b"dirty\r\n")
            packet.export(repo,revision,root / "out",["."])
            self.assertEqual((root / "out/hidden.txt").read_bytes(),"canonical \u0442\u0435\u043a\u0441\u0442\n".encode())
            self.assertEqual((root / "out/binary.bin").read_bytes(),bytes(range(256)))

    def test_raw_export_rejects_forbidden_members(self):
        for name,mode in (("../escape","100644"),(".venv/key","100644"),("target/a","100644"),
                          ("link","120000"),("submodule","160000"),("C:/key","100644")):
            with self.subTest(name=name), self.assertRaises(ValueError):
                packet.member(name,mode)

    def test_qa_lock_preserves_frozen_product_records_and_string_specs(self):
        lock = run.FLEET / "backend/Cargo.lock"
        raw = run.derive_qa_lock(lock,run.HERE / "live/Cargo.toml")
        original = tomllib.loads(lock.read_text())["package"]
        derived = tomllib.loads(raw.decode())["package"]
        self.assertEqual(derived[:-1],original)
        self.assertEqual(derived[-1]["name"],"fleet-native-acceptance")
        expected = set(tomllib.loads((run.HERE / "live/Cargo.toml").read_text())["dependencies"])
        self.assertEqual({name.split()[0] for name in derived[-1]["dependencies"]},expected)

    def test_compose_is_explicit_isolated_no_ports_and_socket_controller_only(self):
        definitions = run.services(Path("C:/owned-packet"),run.SOURCE18)
        sockets = []
        for name, service in definitions.items():
            self.assertNotIn("ports",service)
            self.assertNotIn("build",service)
            self.assertNotIn("privileged",service)
            self.assertNotIn("container_name",service)
            self.assertIn(service["image"],run.services_images().values())
            for mount in service.get("volumes",[]):
                if mount["source"] == "/var/run/docker.sock":
                    sockets.append((name,mount["read_only"]))
                if mount["type"] == "volume":
                    self.assertIn(mount["source"],run.VOLUMES)
        self.assertEqual(sockets,[("fleet-backend",True)])
        self.assertEqual(definitions["build"]["networks"],["build"])
        self.assertEqual(definitions["postgres"]["networks"],["fleet"])

    def test_first_failure_survives_cleanup_error_without_raw_details(self):
        report = {}
        run.remember_failure(report,"initial",ValueError("secret-path-or-arg"))
        run.remember_failure(report,"native_cleanup",RuntimeError("other-secret"))
        self.assertEqual(report,dict(failure_phase="initial",failure_class="ValueError"))

    def test_failure_projection_accepts_only_fixed_phase_and_class(self):
        phases = ("maintenance", "source-check", "volume-init", "build", "compile_qualification",
                  "startup", "scenario", "initial", "physical_controller_restart", "recover", "native_cleanup")
        classes = ("ValueError", "RuntimeError", "AssertionError", "KeyError", "TypeError", "OSError",
                   "FileNotFoundError", "PermissionError", "TimeoutExpired", "CalledProcessError")
        for phase in phases:
            for kind in classes:
                self.assertEqual(run.failure_projection(dict(failure_phase=phase, failure_class=kind)),
                                 dict(state="failed", failure_phase=phase, failure_class=kind, cleanup_verified=None))
        for value in (None, True, 1, [], {}, "", "private-value", "initial/private", "ValueError: secret"):
            self.assertEqual(run.failure_projection(dict(failure_phase=value, failure_class=value)),
                             dict(state="failed", failure_phase="unknown", failure_class="OtherError", cleanup_verified=None))

    def test_failure_projection_cleanup_requires_exact_matrix_parity(self):
        for value, expected in (("passed", True), ("failed", False), (None, None), (True, None),
                                (1, None), ([], None), ({}, None), ("cleaned", None), ("passed private", None)):
            report = dict(parity=dict(cleanup_inventory=value), cleanup=dict(journal_phase="cleaned"))
            self.assertIs(run.failure_projection(report)["cleanup_verified"], expected)
        for parity in (None, True, [], "passed", {}, {"candidate_alias_cleanup": "passed"}):
            self.assertIsNone(run.failure_projection(dict(parity=parity))["cleanup_verified"])

    def test_failure_projection_retains_first_failure_without_private_values(self):
        report = dict(state="failed", parity=dict(cleanup_inventory="failed"),
                      artifact_hashes={"private-path": "private-hash"}, scenario={"body": "private-body"},
                      cleanup={"containers": ["private-container"]}, terminal_report="private-report")
        run.remember_failure(report, "initial", ValueError("private-error"))
        run.remember_failure(report, "native_cleanup", RuntimeError("private-cleanup-error"))
        result = run.failure_projection(report)
        self.assertEqual(result, dict(state="failed", failure_phase="initial", failure_class="ValueError", cleanup_verified=False))
        raw = json.dumps(result)
        self.assertNotIn("private", raw)
        self.assertLess(len(raw), 512)

    def test_execute_failure_and_save_failure_share_closed_projection(self):
        manifest = dict(execute_eligible=True, fleet_commit=run.SOURCE_MERGED, source_count=1,
                        prerequisite_checks={"ready": True})
        for save_fails in (False, True):
            with self.subTest(save_fails=save_fails), tempfile.TemporaryDirectory() as temp:
                saved = []
                def write(path, value):
                    if path.name == "terminal-report.json":
                        saved.append(copy.deepcopy(value))
                        if save_fails:
                            raise OSError("private-save-error")
                with patch.object(run, "require_candidates"), \
                     patch.object(run, "verify", return_value=manifest), \
                     patch.object(run, "prerequisites", return_value={"ready": True}), \
                     patch.object(run.hosted_policy, "phase_preflight"), \
                     patch.object(run, "sha", return_value="a" * 64), \
                     patch.object(run, "write_json", side_effect=write), \
                     patch.object(run, "checked", side_effect=ValueError("private-command-error")), \
                     patch.object(run, "maintenance"), patch.object(run, "services_images", return_value={}), \
                     patch.object(run, "inventory", return_value={"private-path": "private-hash"}), \
                     patch("builtins.print") as output:
                    code = run.execute(Path(temp), "exclusive native4 " + run.SOURCE_MERGED[:12], "unused")
                self.assertEqual(code, 1)
                output.assert_called_once()
                line = output.call_args.args[0]
                prefix = "TERMINAL_REPORT_UNSAVED " if save_fails else ""
                self.assertTrue(line.startswith(prefix))
                self.assertEqual(json.loads(line.removeprefix(prefix)),
                                 dict(state="failed", failure_phase="maintenance", failure_class="ValueError", cleanup_verified=None))
                self.assertNotIn("private", line)
                self.assertNotIn(temp, line)
                self.assertEqual(saved[0]["failure_phase"], "maintenance")
                self.assertEqual(saved[0]["failure_class"], "ValueError")

    def test_successful_terminal_output_is_unchanged(self):
        tree = ast.parse((run.HERE / "run.py").read_bytes())
        execute = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == "execute")
        output = execute.body[-2].value
        self.assertEqual(output.func.id, "print")
        report = dict(state="scoped_native_matrix_passed_not_sdlc_acceptance")
        packet_path = Path("owned-packet")
        raw = eval(compile(ast.Expression(output.args[0]), "terminal-output", "eval"),
                   dict(json=json, report=report, packet=packet_path, failure_projection=run.failure_projection))
        self.assertEqual(json.loads(raw), dict(state=report["state"], terminal_report=str(packet_path / "terminal-report.json")))

    def test_physical_generation_inventory_is_closed_without_raw_environment(self):
        operation = Mock(project=PROJECT)
        operation.resources.return_value = ["cid"]
        service = next(iter(self.creation["services"]))
        labels = dict(self.creation["services"][service]["labels"],
                      **{"com.docker.compose.project":PROJECT,"com.docker.compose.service":service})
        physical = dict(Id="cid",Image=run.HERMES_IMAGE,State=dict(StartedAt="original"),
                        Config=dict(Labels=labels,Env=["PRIVATE=never-report"]))
        with patch.object(run,"checked",return_value=json.dumps([physical]).encode()):
            evidence = run.native_inventory(operation,["docker","--context","unused"])
        self.assertEqual(evidence["cid"]["started_at"],"original")
        self.assertNotIn("PRIVATE",json.dumps(evidence))
        physical["Image"] = "foreign"
        with patch.object(run,"checked",return_value=json.dumps([physical]).encode()), self.assertRaises(ValueError):
            run.native_inventory(operation,["docker","--context","unused"])

    def test_execute_ack_denied_before_packet_docker_or_maintenance_io(self):
        with patch.object(run,"verify",side_effect=AssertionError("unexpected I/O")):
            with self.assertRaises(ValueError):
                run.execute(Path("unused"),"","desktop-linux")

    def test_unmerged_packet_denied_before_maintenance(self):
        with patch.object(run,"verify",return_value=dict(execute_eligible=False)), \
             patch.object(run,"maintenance",side_effect=AssertionError("unexpected maintenance")):
            with self.assertRaises(ValueError):
                run.execute(Path("unused"),"exclusive-native4-reviewed","desktop-linux")

    def report(self):
        return dict(state="scenario_passed",source_commit=run.SOURCE_MERGED,matrix=copy.deepcopy(run.EXPECTED_MATRIX),
            actual_rust_supervisor=True,genuine_hermes=True,controlled_model_prompts=6,
            no_second_native_post_or_generation=True,runtime_ready=False,sdlc_completion=False,
            isolation=[dict(agent_id="one",container_id="cid1"),dict(agent_id="two",container_id="cid2")])

    def test_closed_matrix_keeps_unknown_acceptance_held(self):
        run.validate_live(self.report(),run.SOURCE_MERGED)
        for key,value in (("state","held_controller_recovery"),("runtime_ready",True),
                          ("sdlc_completion",True),("controlled_model_prompts",5)):
            changed = self.report()
            changed[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                run.validate_live(changed,run.SOURCE_MERGED)
        changed = self.report()
        changed["matrix"]["unknown_ack_after_restart"] = "accepted"
        with self.assertRaises(ValueError):
            run.validate_live(changed,run.SOURCE_MERGED)

    def test_duplicate_container_cannot_qualify_two_agents(self):
        changed = self.report()
        changed["isolation"][1]["container_id"] = "cid1"
        with self.assertRaises(ValueError):
            run.validate_live(changed,run.SOURCE_MERGED)

    def test_review_subpath_counterexample_and_source_closure(self):
        """Static/model closure of P2, not actual Docker/Rust execution."""
        main = (run.HERE / "live/src/main.rs").read_text(encoding="utf-8")
        block = main.split("async fn isolation(",1)[1].split("async fn send(",1)[0]
        compact = "".join(block.split())
        self.assertIn('physical["HostConfig"]["Mounts"]',block)
        for field in ("Type","Source","Target","VolumeOptions"):
            self.assertIn('specification["' + field + '"]',block)
        self.assertIn('specification.get("ReadOnly")',compact)
        self.assertIn('"NoCopy":true,"Subpath":format!("{}/{area}",a.name)',compact)
        self.assertIn('mount["Name"]',block)
        self.assertIn('mount["RW"]',block)
        self.assertNotIn('.ends_with(&format!("/{}/{area}", a.name))',block)
        original = packet.git(run.BASE,"show",run.BASE_SHA + ":scripts/tests/test_runtime_control.py").decode()
        self.assertIn("'Source': '/daemon/owned_agents'",original)
        self.assertIn("'VolumeOptions': {'NoCopy': True, 'Subpath': m['subpath']}",original)
        for area in ("runtime","config","workspace","logs"):
            with self.subTest(area=area):
                actual = dict(Type="volume",Name="owned_agents",Source="/daemon/owned_agents",
                              Destination="/" + area,RW=area != "runtime")
                expected = dict(Type="volume",Source="owned_agents",Target="/" + area,ReadOnly=area == "runtime",
                                VolumeOptions=dict(NoCopy=True,Subpath="agent1/" + area))
                accepts = lambda a,s: (a["Type"] == "volume" and a["Name"] == "owned_agents"
                    and a["Destination"] == "/" + area and a["RW"] is (area != "runtime")
                    and type(s.get("ReadOnly",False)) is bool
                    and dict(s,ReadOnly=s.get("ReadOnly",False)) == expected)
                self.assertTrue(accepts(actual,expected))
                if area != "runtime":
                    omitted = {k:v for k,v in expected.items() if k != "ReadOnly"}
                    self.assertTrue(accepts(actual,omitted))
                self.assertFalse(actual["Source"].endswith("/agent1/" + area))
                for key,value in (("Source","foreign"),("Target","/foreign"),("ReadOnly",area != "runtime")):
                    self.assertFalse(accepts(actual,dict(expected,**{key:value})))
                for options in (dict(NoCopy=False,Subpath="agent1/" + area),dict(NoCopy=True,Subpath="agent2/" + area)):
                    self.assertFalse(accepts(actual,dict(expected,VolumeOptions=options)))
                self.assertFalse(accepts(dict(actual,RW=area == "runtime"),expected))

    def test_review_duplicate_mirror_counterexample_and_source_closure(self):
        """After replay, actual Rust helper rereads count/body and checks original ID."""
        main = (run.HERE / "live/src/main.rs").read_text(encoding="utf-8")
        answer = main.split("async fn answer(",1)[1].split("async fn request(",1)[0]
        self.assertIn("repo.list_session_messages(session)",answer)
        self.assertIn("assert_eq!(answers.len(), 1)",answer)
        self.assertIn("(*answers[0]).clone()",answer)
        block = main.split("for (index, a) in agents.iter().enumerate() {",1)[1].split("let a = &agents[0];",1)[0]
        self.assertLess(block.index("let acknowledged = answer("),block.index("repo.create_session_message"))
        after = block.split("repo.create_session_message",1)[1]
        self.assertIn("let replayed = answer(",after)
        self.assertIn("assert_eq!(replayed.id, acknowledged.id)",after)
        self.assertIn("assert_eq!(replayed.body, acknowledged.body)",after)
        original = dict(id="one",body="Container answer: fixture")
        accepts = lambda mirrors: len(mirrors) == 1 and mirrors[0] == original
        self.assertTrue(accepts([original]))
        self.assertFalse(accepts([original,dict(original,id="duplicate")]))
        self.assertFalse(accepts([dict(original,id="changed")]))
        self.assertFalse(accepts([dict(original,body="changed")]))

    def relay(self, unknown, status=200):
        body = json.dumps(dict(input="unknown-live-fixture" if unknown else "normal-live-fixture")).encode()
        handler = object.__new__(fixture.Proxy)
        handler.gateway_port = 29101
        handler.command,handler.path = "POST","/v1/runs"
        handler.headers = {"Content-Length":str(len(body)),"Authorization":"Bearer synthetic-private","Host":"proxy"}
        handler.rfile,handler.wfile = io.BytesIO(body),io.BytesIO()
        handler.connection = Mock()
        handler.send_response,handler.send_header,handler.end_headers = Mock(),Mock(),Mock()
        response = Mock(status=status)
        response.getheaders.return_value = [("content-type","text/event-stream")]
        response.read1.side_effect = [b"genuine response bytes",b""]
        connection = Mock()
        connection.getresponse.return_value = response
        with patch.object(fixture.http.client,"HTTPConnection",return_value=connection), \
             patch.object(fixture,"count_post") as count, patch.object(fixture,"count_ack_loss") as loss:
            handler.relay()
        count.assert_called_once_with(body)
        connection.request.assert_called_once_with("POST","/v1/runs",body=body,
            headers={"Content-Length":str(len(body)),"Authorization":"Bearer synthetic-private"})
        connection.close.assert_called_once()
        if unknown and 200 <= status < 300:
            loss.assert_called_once_with(body)
        else:
            loss.assert_not_called()
        return handler,response

    def test_fault_consumes_actual_ack_once_without_fabricated_response(self):
        handler,response = self.relay(True)
        response.read.assert_called_once()
        handler.send_response.assert_not_called()
        handler.connection.shutdown.assert_called_once_with(2)
        self.assertEqual(handler.wfile.getvalue(),b"")

    def test_normal_and_rejected_requests_forward_actual_bytes(self):
        for unknown,status in ((False,200),(True,409)):
            with self.subTest(status=status):
                handler,response = self.relay(unknown,status)
                handler.send_response.assert_called_once_with(status)
                handler.connection.shutdown.assert_not_called()
                self.assertEqual(handler.wfile.getvalue(),b"genuine response bytes")
                response.read.assert_not_called()

    def test_native_post_counter_exposes_duplicates_without_raw_prompts(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(fixture,"COUNTS",Path(temp) / "counts.json"):
            fixture.count_post(b"private synthetic prompt")
            fixture.count_post(b"private synthetic prompt")
            raw = fixture.COUNTS.read_bytes()
            self.assertNotIn(b"private",raw)
            self.assertNotIn(b"\r\n",raw)
            self.assertEqual(list(json.loads(raw).values()),[2])

    def test_json_control_files_are_lf_and_atomic(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "manifest.json"
            packet.write_json(path,dict(value="\u0442\u0435\u043a\u0441\u0442"))
            self.assertNotIn(b"\r\n",path.read_bytes())
            self.assertFalse(path.with_suffix(".tmp").exists())
            self.assertEqual(json.loads(path.read_bytes()),dict(value="\u0442\u0435\u043a\u0441\u0442"))

    def synthetic_packet(self, root):
        owned = root / PROJECT
        inputs = owned / "input"
        (inputs / "hermes").mkdir(parents=True)
        (inputs / "hermes/entry.py").write_bytes(b"print('synthetic seal fixture')\n")
        for name in run.HELPERS:
            (root / name).parent.mkdir(parents=True,exist_ok=True)
            (owned / name).parent.mkdir(parents=True,exist_ok=True)
            shutil.copyfile(run.HERE / name,root / name)
            shutil.copyfile(run.HERE / name,owned / name)
        files = packet.inventory(inputs)
        (inputs / "sources.sha256").write_bytes("".join(f"{h}  {p}\n" for p,h in files.items()).encode())
        packet.write_json(owned / "hermes-source.json",packet.inventory(inputs / "hermes"))
        manifest = dict(project=PROJECT,task=run.TASK,purpose=run.PURPOSE,expected_matrix=run.EXPECTED_MATRIX,
            held_unqualified=run.HELD_UNQUALIFIED,fleet_commit=run.SOURCE_MERGED,
            fleet_source_paths=list(run.SOURCE_PATHS),
            utility_hashes=run.UTILITY_HASHES,recovered_activation=True,
            sdk_commit=run.SDK_SHA,base_utility_commit=run.BASE_SHA,hermes_commit=run.HERMES_SHA,
            maintenance_helper_sha256=run.MAINTENANCE_SHA,images=run.services_images(),
            exact_disposable_volumes=[PROJECT + "_" + n for n in run.VOLUMES],
            exact_disposable_networks=[PROJECT + "_" + n for n in run.NETWORKS],
            source_files=packet.inventory(inputs),source_count=len(files) + 1,
            helper_files={n:packet.sha(owned / n) for n in run.HELPERS},
            hermes_manifest_sha256=packet.sha(owned / "hermes-source.json"))
        packet.write_json(owned / "manifest.json",manifest)
        packet.write_json(owned / "seal.json",dict(version=1,manifest_sha256=packet.sha(owned / "manifest.json")))
        return owned

    def test_seal_rejects_input_or_helper_mutation(self):
        for mutated in ("input/hermes/entry.py","run.py"):
            with tempfile.TemporaryDirectory() as temp, self.subTest(mutated=mutated):
                root = Path(temp)
                owned = self.synthetic_packet(root)
                with patch.object(run,"HERE",root):
                    run.verify(owned)
                    with (owned / mutated).open("ab") as stream:
                        stream.write(b"# mutation\n")
                    with self.assertRaises(ValueError):
                        run.verify(owned)

    def test_failed_preparation_packet_never_verifies(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            owned = self.synthetic_packet(root)
            packet.write_json(owned / "prepare-failure.json",dict(state="failed"))
            with patch.object(run,"HERE",root), self.assertRaises(ValueError):
                run.verify(owned)

    def test_selected_export_excludes_tracked_bytecode_without_relaxing_guard(self):
        raw = packet.git(run.FLEET,"ls-tree","-r","-z",run.SOURCE_MERGED,"--",*run.SOURCE_PATHS)
        names = [line.split(b"\t",1)[1].decode() for line in raw.split(b"\0") if line]
        self.assertTrue(any(name.startswith("frontend/") for name in names))
        self.assertIn("backend/infra/tests/container_controller.rs",names)
        self.assertFalse(any("__pycache__" in name or name.endswith(".pyc") for name in names))
        with self.assertRaises(ValueError):
            packet.member("scripts/__pycache__/verify_readme.cpython-311.pyc","100644")


class CompileProofTests(unittest.TestCase):
    """Synthetic metadata parser tests are not actual compilation receipts."""

    def records(self):
        return [dict(reason="compiler-artifact",target=dict(name=compile_proof.NAME,kind=["bin"],
                    crate_types=["bin"],src_path=str(Path("/scratch/src/live/src/main.rs"))),
                    executable=str(Path("/target/debug/fleet-native-acceptance")),profile=dict(test=False)),
                dict(reason="build-finished",success=True)]

    def qualify(self, records):
        return compile_proof.artifact(records,Path("/scratch/src"),Path("/target/debug/fleet-native-acceptance"))

    def test_exact_cargo_binary_artifact(self):
        records = self.records()
        records[0]["target"]["src_path"] = str(Path("/scratch/src/live/src/main.rs"))
        records[0]["executable"] = str(Path("/target/debug/fleet-native-acceptance"))
        self.assertEqual(self.qualify(records),records[0])

    def test_missing_failed_or_duplicate_completion_denied(self):
        records = self.records()
        for changed in (records[:-1],records + [records[-1]],
                        [records[0],dict(reason="build-finished",success=False)]):
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                self.qualify(changed)

    def test_duplicate_or_wrong_source_binary_and_test_profile_denied(self):
        records = self.records()
        with self.assertRaises(ValueError):
            self.qualify(records + [records[0]])
        for target,key,value in (("target","kind",["lib"]),("target","crate_types",["lib"]),
                                 ("target","src_path","/foreign/main.rs"),("profile","test",True),
                                 (None,"executable","/foreign/binary")):
            changed = self.records()
            if target:
                changed[0][target][key] = value
            else:
                changed[0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.qualify(changed)

    def test_host_compile_proof_binds_sealed_sources_and_artifact_bytes(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "output").mkdir()
            artifacts = root / "output/build-artifacts.jsonl"
            artifacts.write_bytes(b"synthetic metadata; not a compile\n")
            paths = {"source_main_sha256":"live/src/main.rs","qa_lock_sha256":"live/Cargo.lock",
                     "product_lock_sha256":"fleet-control/backend/Cargo.lock"}
            files = {path:hashlib.sha256(path.encode()).hexdigest() for path in paths.values()}
            proof = dict(state="actual_locked_compile_verified",native_executed=False,
                binary_sha256="1" * 64,compiler_artifacts=1,cargo_artifacts_sha256=packet.sha(artifacts),
                **{key:files[path] for key,path in paths.items()})
            packet.write_json(root / "output/compile-proof.json",proof)
            run.verify_compile_proof(root,dict(source_files=files))
            for key,value in (("qa_lock_sha256","0" * 64),("native_executed",True),
                              ("compiler_artifacts",0),("binary_sha256","not-a-hash")):
                changed = dict(proof,**{key:value})
                packet.write_json(root / "output/compile-proof.json",changed)
                with self.subTest(key=key), self.assertRaises(ValueError):
                    run.verify_compile_proof(root,dict(source_files=files))

    def test_build_failure_codes_only_and_constant_main_coordinates(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "cargo.jsonl"
            records = []
            for name, code in (("/scratch/src/live/src/main.rs", "E0308"),
                               ("/private/base/secret.rs", "E0369"),
                               ("live/src/main.rs", "E0277")):
                records.append(dict(reason="compiler-message", message=dict(level="error", code=dict(code=code),
                    message="PRIVATE_TOKEN", rendered="PRIVATE_BODY", spans=[dict(file_name=name, is_primary=True,
                    line_start=42, column_start=9, text=["PRIVATE_SOURCE"])])))
            path.write_text("\n".join(json.dumps(r) for r in records))
            result = compile_proof.failure("cargo", 101, path)
            self.assertEqual(result, dict(step="cargo", exit_code=101, diagnostics=[
                dict(code="E0308", line=42, column=9), dict(code="E0369"), dict(code="E0277")]))
            self.assertNotIn("PRIVATE", json.dumps(result))
            self.assertNotIn("file", json.dumps(result))
            self.assertLess(len(json.dumps(result)), 2048)
            records[0]["message"]["spans"][0]["line_start"] = True
            records[1]["message"]["code"]["code"] = "E0369 PRIVATE"
            records[2]["message"]["level"] = "warning"
            path.write_text("\n".join(json.dumps(r) for r in records))
            self.assertEqual(compile_proof.failure("cargo", 101, path)["diagnostics"], [dict(code="E0308")])

    def test_build_failure_input_and_record_bounds_fail_closed(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "cargo.jsonl"
            for raw in (b"private-not-json", b"{}\n" * 16385, b" " * (8 * 1024 ** 2 + 1),
                        b"[" * 1100 + b"]" * 1100, b"[]", b'{"reason":"compiler-message","message":true}'):
                path.write_bytes(raw)
                self.assertEqual(compile_proof.failure("cargo", 101, path),
                                 dict(step="cargo", exit_code=101, diagnostics=[]))
            path.unlink()
            self.assertEqual(compile_proof.failure("toolchain", 7, path)["diagnostics"], [])
            self.assertIsNone(compile_proof.read_failure(path))
            self.assertIsNone(compile_proof.read_failure(Path(temp)))
            record = dict(step="cargo", exit_code=101, diagnostics=[dict(code="E0308")])
            for changed in ({**record, "step":"private"}, {**record, "exit_code":True},
                            {**record, "exit_code":0}, {**record, "exit_code":256}, {**record, "body":"private"},
                            {**record, "diagnostics":[dict(code="E0308", file="private")]},
                            {**record, "diagnostics":[dict(code="E0308", line=0, column=1)]},
                            {**record, "diagnostics":record["diagnostics"] * 9}):
                path.write_text(json.dumps(changed))
                self.assertIsNone(compile_proof.read_failure(path))
            path.write_bytes(b" " * 2049)
            self.assertIsNone(compile_proof.read_failure(path))
            path.write_bytes(b"[" * 1000 + b"]" * 1000)
            self.assertIsNone(compile_proof.read_failure(path))
            path.write_text(json.dumps(record))
            self.assertEqual(compile_proof.read_failure(path), record)
            with patch.object(compile_proof.os, "open", side_effect=OSError("PRIVATE_READ")):
                self.assertIsNone(compile_proof.read_failure(path))
            with patch.object(Path, "is_symlink", return_value=True):
                self.assertIsNone(compile_proof.read_failure(path))

    def test_build_failure_eight_codes_and_original_success_cli(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            path = root / "cargo.jsonl"
            records = [dict(reason="compiler-message", message=dict(level="error", code=dict(code=f"E{i:04}")))
                       for i in range(12)]
            path.write_text("\n".join(json.dumps(r) for r in records + records))
            self.assertEqual(compile_proof.failure("cargo", 101, path)["diagnostics"],
                             [dict(code=f"E{i:04}") for i in range(8)])
            args = ["compile_proof.py", "--artifacts", str(path), "--root", str(root),
                    "--executable", str(root / "binary"), "--copied", str(root / "copy"),
                    "--output", str(root / "proof.json")]
            proof = dict(state="actual_locked_compile_verified")
            with patch.object(sys, "argv", args), patch.object(compile_proof, "qualify", return_value=proof) as qualify, \
                 patch.object(compile_proof, "write_json") as write:
                self.assertEqual(compile_proof.main(), 0)
                qualify.assert_called_once_with(path, root, root / "binary", root / "copy")
                write.assert_called_once_with(root / "proof.json", proof)
            with patch.object(sys, "argv", args + ["--failure-step", "cargo", "--failure-exit", "101"]), \
                 patch.object(compile_proof, "qualify") as qualify, patch.object(compile_proof, "write_json") as write:
                self.assertEqual(compile_proof.main(), 0)
                qualify.assert_not_called()
                self.assertEqual(write.call_args.args[1]["exit_code"], 101)
            with patch.object(sys, "argv", args + ["--failure-step", "cargo", "--failure-exit", "0"]), \
                 patch.object(compile_proof, "write_json") as write:
                self.assertEqual(compile_proof.main(), 1)
                write.assert_not_called()

    def test_build_failure_public_projection_and_capture_precede_cleanup(self):
        record = dict(step="cargo", exit_code=101, diagnostics=[dict(code="E0308", line=3, column=2)])
        result = run.failure_projection(dict(failure_phase="build", failure_class="RuntimeError", build_failure=record))
        self.assertEqual(result["build_failure"], record)
        self.assertEqual(run.failure_projection(dict(failure_phase="build", build_failure={"body":"PRIVATE"}))["build_failure"], None)
        self.assertNotIn("build_failure", run.failure_projection(dict(failure_phase="initial", build_failure=record)))
        tree = ast.parse((run.HERE / "run.py").read_bytes())
        execute = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == "execute")
        reads = [n for n in ast.walk(execute) if isinstance(n, ast.Call) and isinstance(n.func, ast.Attribute)
                 and n.func.attr == "read_failure"]
        cleanup = [n.lineno for n in ast.walk(execute) if isinstance(n, ast.Call) and isinstance(n.func, ast.Name)
                   and n.func.id == "cleanup_native"]
        self.assertEqual(len(reads), 1)
        self.assertLess(reads[0].lineno, min(cleanup))

    def test_build_shell_trap_preserves_original_exit_and_every_old_command(self):
        script = (run.HERE / "build.sh").read_text()
        original = "\n".join(line for line in script.splitlines() if not line.startswith(("build_step=", "  build_step=", "trap ")))
        self.assertEqual(hashlib.sha256((original + "\n").encode()).hexdigest(),
                         "d9ebe12483d8579d7566d2807e57ce4bb6a8ac4968a12b2b384fe745b3106ecc")
        steps = {line.strip().split("=", 1)[1] for line in script.splitlines() if line.strip().startswith("build_step=")}
        self.assertEqual(steps, compile_proof.BUILD_STEPS)
        bash = shutil.which("bash") if sys.platform != "win32" else "C:/Program Files/Git/bin/bash.exe"
        if not bash or not Path(bash).is_file():
            self.skipTest("Bash unavailable for pure trap execution")
        trap = next(line for line in script.splitlines() if line.startswith("trap "))
        for step in ("toolchain", "cargo", "compile_proof"):
            for writer_code in (0, 17):
                command = "set -euo pipefail\npython3() { printf '%s\\n' \"$*\"; return " + str(writer_code) + "; }\n"
                command += trap.replace(">/dev/null 2>&1", "") + f"\nbuild_step={step}\n(exit 23)\n"
                result = subprocess.run([bash, "-c", command], capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode, 23)
                self.assertIn(f"--failure-step {step} --failure-exit 23", result.stdout)


if __name__ == "__main__":
    unittest.main()
