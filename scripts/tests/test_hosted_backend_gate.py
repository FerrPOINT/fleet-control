"""Pure/static/in-memory tests only; no compiler, Docker, PG, exports or API calls."""
from collections import Counter
import copy
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock
import zipfile

import yaml

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("hosted_gate", ROOT / "scripts/hosted_backend_gate.py")
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)
REVIEWED = gate.reviewed_inventory(ROOT)


class HostedBackendTests(unittest.TestCase):
    def workflow(self):
        return yaml.load((ROOT / gate.WORKFLOW).read_text(), Loader=yaml.BaseLoader)

    def test_exact_push_only_branch_permissions_and_one_bounded_job(self):
        flow = self.workflow()
        self.assertEqual(flow["on"], {"push": {"branches": [gate.BRANCH]}})
        self.assertEqual(flow["permissions"], {"contents": "read"})
        self.assertEqual(flow["concurrency"]["cancel-in-progress"], "false")
        self.assertEqual(set(flow["jobs"]), {"backend"})
        job = flow["jobs"]["backend"]
        self.assertEqual(job["runs-on"], "ubuntu-24.04")
        self.assertEqual(job["container"], dict(image="rust:1.88.0-bookworm", options="--cpus 2 --memory 4g"))
        self.assertEqual(job["env"]["CARGO_BUILD_JOBS"], "1")
        self.assertIn("github.event.deleted == false", job["if"])
        self.assertIn("github.ref == 'refs/heads/" + gate.BRANCH + "'", job["if"])

    def test_managed_synthetic_pg_no_ports_no_production_credentials(self):
        service = self.workflow()["jobs"]["backend"]["services"]["postgres"]
        self.assertEqual(service["image"], "postgres:17.6-alpine")
        self.assertEqual(service["env"], dict(POSTGRES_USER="fleet_test", POSTGRES_DB="fleet_foundation_test",
                                               POSTGRES_HOST_AUTH_METHOD="trust"))
        self.assertNotIn("ports", service)
        urls = gate.database_environment()
        self.assertEqual(urls["FLEET_REAL_AUTH_TEST_DATABASE_URL"], "postgres://fleet_test@postgres:5432/fleet_real_auth_test")
        self.assertEqual(len(urls), 11)
        for url in urls.values():
            self.assertTrue(url.startswith(("postgres://fleet_test@postgres:5432/fleet_",
                                            "postgres://fleet_approval_events_test@postgres:5432/fleet_")))

    def test_action_pins_and_restricted_preflight_before_both_Base_tokens(self):
        steps = self.workflow()["jobs"]["backend"]["steps"]
        preflight = next(i for i, step in enumerate(steps) if step.get("run", "").endswith(" preflight"))
        private = []
        for i, step in enumerate(steps):
            if "uses" in step:
                self.assertRegex(step["uses"], r"^actions/(checkout|upload-artifact)@[0-9a-f]{40}$")
                self.assertNotIn("rust-cache", step["uses"])
            if "token" in step.get("with", {}):
                private.append(step["with"])
                self.assertGreater(i, preflight)
                self.assertEqual(step["with"]["token"], "${{ secrets.SERVICES_BASE_TOKEN }}")
            if step.get("uses", "").startswith("actions/checkout"):
                self.assertEqual(step["with"]["persist-credentials"], "false")
        self.assertEqual({step["ref"] for step in private}, {gate.BASE_SHA, gate.AUTH_SHA})
        source = next(step["with"] for step in steps if step.get("with", {}).get("path") == "fleet-control")
        self.assertEqual(source["ref"], gate.SOURCE_SHA)

    def test_delta_only_six_additions_and_source_ci_never_modified(self):
        good = "\n".join("A\t" + name for name in gate.WRITE_SET)
        gate.validate_delta(good)
        for bad in (good.replace("A\t", "M\t", 1), good + "\nA\tbackend/api/src/lib.rs",
                    good + "\nA\tbackend/migration/src/fake.rs", good + "\nA\topenapi/openapi.json", ""):
            with self.assertRaises(ValueError):
                gate.validate_delta(bad)
        original = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "show",
                                   gate.SOURCE_SHA + ":.github/workflows/ci.yml"], capture_output=True, check=True).stdout
        self.assertEqual(original, (ROOT / ".github/workflows/ci.yml").read_bytes())

    def test_all_135_ignored_exactly_once_and_236_default_declarations(self):
        records = REVIEWED["ignored"]
        self.assertEqual(len(records), 135)
        self.assertEqual(len({(x["source"], x["name"]) for x in records}), 135)
        self.assertEqual(Counter(x["gate"] for x in records)["runtime_controls"], 30)
        self.assertEqual(len(REVIEWED["groups"]["runtime_terminal"]), 14)
        self.assertEqual(len(REVIEWED["groups"]["foundation"]), 44)
        self.assertEqual(REVIEWED["default_foundation_ignored"], 115)
        self.assertEqual(len(REVIEWED["workspace_default_declarations"]), 236)
        self.assertEqual(REVIEWED["authority"]["old_ignored"], 130)
        for x in records:
            self.assertIn(x["name"], REVIEWED["groups"][x["gate"]])

    def test_required_full_commands_migrations_and_real_auth_remain(self):
        text = (ROOT / gate.GATE).read_text()
        for command in ("cargo fmt --all -- --check", "cargo check --locked --workspace --all-targets",
                        "cargo clippy --locked --workspace --all-targets -- -D warnings",
                        "cargo test --locked --workspace -- --test-threads=1",
                        "cargo test --locked -p migration --lib lineage_tests -- --include-ignored --test-threads=1",
                        "cargo run --locked -p migration -- down -n 1", "cargo run --locked -p migration -- down -n 15",
                        "cargo build --locked -p auth-server --bin auth-server",
                        "cargo test --locked -p infra --test pm_credentials_real_auth -- --ignored --test-threads=1",
                        "cmp ../openapi/openapi.json ${QA_OUTPUT}/openapi.json"):
            self.assertIn(command, text)
        self.assertEqual(len(gate.GATES), 46)
        self.assertEqual(len(REVIEWED["groups"]["lineage10"]), 10)
        self.assertEqual(len([x for x in REVIEWED["ignored"] if x["gate"] == "lineage10"]), 9)
        for stage in ("lookup_header", "lookup_openapi", "lookup_route"):
            self.assertEqual(len(REVIEWED["groups"][stage]), 1)
            self.assertIn("run_tests " + stage, text)
        self.assertIn("exact=(--exact)", text)
        self.assertIn("CARGO_BUILD_JOBS=1", text)

    def log(self, stage, ignored=0):
        names = REVIEWED["groups"][stage]
        return "\n".join("test " + name + " ... ok" for name in names) + \
            f"\ntest result: ok. {len(names)} passed; 0 failed; {ignored} ignored; 0 measured; 0 filtered out;\n"

    def test_exact_focused_log_requires_each_name_and_count_no_zero(self):
        text = self.log("runtime_controls")
        gate.verify_test_log("runtime_controls", text, REVIEWED)
        variants = [text.replace(" ... ok", " ... ignored", 1), text.replace("30 passed", "0 passed"),
                    text + "test unexpected ... ok\n", text.replace("0 ignored", "1 ignored"),
                    text + "PostgreSQL tests skipped\n", text + "test result: FAILED.\n", ""]
        for bad in variants:
            with self.assertRaises(ValueError):
                gate.verify_test_log("runtime_controls", bad, REVIEWED)

    def test_foundation_44_passed_and_115_ignored_never_lowered(self):
        gate.verify_test_log("foundation", self.log("foundation", 115), REVIEWED)
        with self.assertRaises(ValueError):
            gate.verify_test_log("foundation", self.log("foundation", 110), REVIEWED)

    def test_compiler_ignored_list_must_be_exact_135_no_missing_extra_duplicate(self):
        names = [item["name"] for item in REVIEWED["ignored"]]
        ignored = "\n".join(name + ": test" for name in names)
        ordinary = ignored + "\nnormal: test\n"
        result = gate.verify_runtime_inventory(ordinary, ignored, REVIEWED)
        self.assertEqual(result["listed_default_count"], 1)
        for bad in ("", ignored + "\nextra: test", ignored + "\n" + names[0] + ": test", "\n".join(ignored.splitlines()[1:])):
            with self.assertRaises(ValueError):
                gate.verify_runtime_inventory(ordinary, bad, REVIEWED)

    def test_workspace_actual_cases_match_compiler_list_not_static_count(self):
        listing = "\n".join(x["name"] + ": test" for x in REVIEWED["ignored"]) + "\nfirst: test\nsecond: test\n"
        text = "test first ... ok\ntest second ... ok\ntest result: ok. 2 passed; 0 failed; 135 ignored;\n"
        result = gate.verify_test_log("workspace", text, REVIEWED, listing)
        self.assertEqual(result["passed"], 2)
        for bad in (text.replace("test second ... ok\n", ""), text.replace("135 ignored", "130 ignored"), ""):
            with self.assertRaises(ValueError):
                gate.verify_test_log("workspace", bad, REVIEWED, listing)

    def test_archive_path_guards(self):
        gate.safe_member("backend/api/src/lib.rs", ("backend",))
        for name in ("../private", "/backend/x", "C:/backend/x", "backend\\x", "backend/../private",
                     "backend/target/x", "backend/.env", "backend/node_modules/x", "backend/.local/x", "backend/a b"):
            with self.assertRaises(ValueError):
                gate.safe_member(name, ("backend",))

    def test_archive_allows_only_directory_ancestors_of_nested_allowlist(self):
        roots = ("docs/TESTING.md", "frontend/src/lib/theme-preference.js")
        for name in ("docs", "frontend", "frontend/src", "frontend/src/lib"):
            gate.safe_member(name, roots, directory=True)
            with self.assertRaises(ValueError):
                gate.safe_member(name, roots)
        for name in ("docs/private", "frontend/src/private", "docs/../private", "front"):
            with self.assertRaises(ValueError):
                gate.safe_member(name, roots, directory=True)

    def test_local_execution_fails_before_any_heavy_or_network_effect(self):
        with mock.patch.dict(os.environ, {}, clear=True), mock.patch.object(gate, "command") as command:
            with self.assertRaises(ValueError):
                gate.execute()
            command.assert_not_called()

    def test_hosted_disk_memory_guards_are_measured_and_fail_closed(self):
        data = {"/proc/meminfo": "MemAvailable: 5000000 kB\nCommitLimit: 7000000 kB\nCommitted_AS: 1000000 kB\n",
                "/sys/fs/cgroup/memory.max": str(4 * 1024 ** 3), "/sys/fs/cgroup/memory.current": str(512 * 1024 ** 2)}
        with mock.patch.object(Path, "read_text", autospec=True, side_effect=lambda path: data[path.as_posix()]), \
                mock.patch.object(gate.shutil, "disk_usage", return_value=SimpleNamespace(free=5 * 1024 ** 3)) as disk:
            self.assertEqual(gate.resource_guard(Path("owned"))["hosted_disk_guard_bytes"], 5 * 1024 ** 3)
            disk.return_value.free -= 1
            with self.assertRaises(ValueError):
                gate.resource_guard(Path("owned"))
            disk.return_value.free += 1
            data["/sys/fs/cgroup/memory.current"] = str(2 * 1024 ** 3)
            with self.assertRaises(ValueError):
                gate.resource_guard(Path("owned"))

    def test_cleanup_only_exact_owned_scratch_and_verified_identity(self):
        identity = dict(workflow_sha="a" * 40, run_id=1, run_attempt=1)
        with tempfile.TemporaryDirectory() as temporary:
            parent = Path(temporary).resolve()
            root = parent / "fleet-backend-98d950e"
            root.mkdir()
            (root / "owner.json").write_bytes(gate.canonical(identity))
            with self.assertRaises(ValueError):
                gate.remove_scratch(root, parent, dict(identity, run_id=2))
            self.assertTrue(root.exists())
            with self.assertRaises(ValueError):
                gate.remove_scratch(root, parent.parent, identity)
            gate.remove_scratch(root, parent, identity)
            self.assertFalse(root.exists())

    def test_cleanup_foreign_db_rejected_before_psql(self):
        with mock.patch.object(gate, "psql") as psql:
            with self.assertRaises(ValueError):
                gate.drop_databases(["production"])
            psql.assert_not_called()

    def test_timeout_stage_is_inferred_without_private_log_read(self):
        rows = [[name, "passed"] for name in gate.GATES[:4]]
        self.assertEqual(gate.failure_stage(rows), "auth_binary")
        self.assertEqual(gate.failure_stage(rows + [["auth_binary", "failed"]]), "auth_binary")
        self.assertEqual(gate.failure_stage([["foreign", "failed"]]), "preflight")
        self.assertEqual(gate.failure_stage([["fmt", "passed"]]), "gate_contract")

    def test_only_the_created_child_process_group_is_stopped(self):
        process = mock.Mock(pid=12345)
        with mock.patch.object(gate.os, "killpg", create=True) as kill, \
                mock.patch.object(gate.signal, "SIGKILL", 9, create=True):
            gate.stop_owned_group(process)
            self.assertEqual([call.args[0] for call in kill.call_args_list], [12345, 12345])
            process.wait.assert_called_once_with(timeout=30)

    def test_private_logs_never_uploaded_cleanup_always_runs(self):
        steps = self.workflow()["jobs"]["backend"]["steps"]
        artifact = next(step for step in steps if step.get("id") == "artifact")
        self.assertNotIn("if", artifact)
        self.assertEqual(artifact["with"]["retention-days"], "14")
        self.assertEqual(artifact["with"]["archive"], "true")
        self.assertEqual(artifact["with"]["overwrite"], "false")
        paths = artifact["with"]["path"].splitlines()
        self.assertEqual({Path(path).name for path in paths}, gate.ARTIFACT_FILES)
        cleanup = next(step for step in steps if step.get("run", "").endswith(" cleanup"))
        self.assertEqual(cleanup["if"], "always()")
        helper = (ROOT / gate.HELPER).read_text()
        self.assertIn("finally:", helper)
        self.assertIn("start_new_session=True", helper)
        self.assertIn('stdout=diagnostics, stderr=diagnostics', helper)
        self.assertNotIn('dict(os.environ,', helper)

    def artifact(self, report_changes=None, provenance_changes=None, extra=None):
        workflow_sha = "a" * 40
        focused = {name: dict(passed=len(names), failed=0, ignored=115 if name == "foundation" else 0,
                              tests=sorted(names)) for name, names in REVIEWED["groups"].items()}
        focused["workspace"] = dict(passed=1, failed=0, ignored=135, tests=["normal"])
        report = dict(backend_quality_gate=True, all_quality_gate=False, sdlc_acceptance=False, status="success",
                      gates=[dict(stage=name, status="passed") for name in gate.GATES], focused=focused,
                      cleanup=dict(scratch=True, synthetic_databases=True), ignored_required=135, foundation_ignored=115,
                      runtime_inventory=dict(ignored=135, listed_default_count=1, ignored_names_sha256=gate.digest(gate.canonical(
                          sorted(item["name"] for item in REVIEWED["ignored"])))))
        report.update(report_changes or {})
        provenance = dict(version=1, repository=gate.REPOSITORY, branch=gate.BRANCH, source_sha=gate.SOURCE_SHA,
                          base_sha=gate.BASE_SHA, auth_sha=gate.AUTH_SHA, workflow_sha=workflow_sha, workflow_path=gate.WORKFLOW,
                          run_id=123, run_attempt=1, rust="1.88.0", postgres="17.6", cargo_build_jobs=1,
                          swagger_sha256=gate.SWAGGER_SHA, openapi_sha256=gate.OPENAPI_SHA, backend_quality_gate=True,
                          all_quality_gate=False, sdlc_acceptance=False, auth_binary_sha256="b" * 64,
                          control_sha256={name: gate.digest((ROOT / name).read_bytes()) for name in sorted(gate.WRITE_SET)})
        provenance.update(provenance_changes or {})
        files = {"report.json": gate.canonical(report), "provenance.json": gate.canonical(provenance)}
        files["SHA256SUMS"] = "".join(gate.digest(files[name]) + "  " + name + "\n"
                                     for name in ("report.json", "provenance.json")).encode()
        files.update(extra or {})
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w") as archive:
            for name, data in files.items():
                archive.writestr(name, data)
        payload = buffer.getvalue()
        run = dict(id=123, run_attempt=1, status="completed", conclusion="success", event="push", head_sha=workflow_sha,
                   head_branch=gate.BRANCH, path=gate.WORKFLOW, repository={"full_name": gate.REPOSITORY})
        artifact = dict(expired=False, workflow_run={"id": 123, "head_sha": workflow_sha},
                        name="fleet-backend-98d950e-123-1", digest="sha256:" + gate.digest(payload))
        args = dict(run_id=123, attempt=1, workflow_sha=workflow_sha, artifact_digest=gate.digest(payload))
        return run, artifact, payload, args

    def test_authenticated_readback_binds_exact_run_attempt_sha_digest(self):
        run, artifact, payload, args = self.artifact()
        self.assertEqual(set(gate.validate_readback(run, artifact, payload, **args)), gate.ARTIFACT_FILES)
        for key, value in (("id", 999), ("run_attempt", 2), ("head_sha", "b" * 40), ("event", "pull_request"),
                           ("conclusion", "failure"), ("head_branch", "main")):
            with self.assertRaises(ValueError):
                gate.validate_readback(dict(run, **{key: value}), artifact, payload, **args)
        with self.assertRaises(ValueError):
            gate.validate_readback(run, artifact, payload + b"tampered", **args)

    def test_readback_rejects_weakened_gate_and_wrong_source_base_auth(self):
        for change in (dict(backend_quality_gate=False), dict(cleanup=dict(scratch=False, synthetic_databases=True)),
                       dict(ignored_required=130), dict(gates=[]), dict(focused={})):
            run, artifact, payload, args = self.artifact(report_changes=change)
            with self.assertRaises(ValueError):
                gate.validate_readback(run, artifact, payload, **args)
        for change in (dict(source_sha="bad"), dict(base_sha="bad"), dict(auth_sha="bad"), dict(all_quality_gate=True),
                       dict(sdlc_acceptance=True), dict(control_sha256={}), dict(auth_binary_sha256="")):
            run, artifact, payload, args = self.artifact(provenance_changes=change)
            with self.assertRaises(ValueError):
                gate.validate_readback(run, artifact, payload, **args)

    def test_readback_rejects_unsafe_or_private_artifact_members(self):
        for name in ("../private", "cargo-stderr.log", "src/base-auth-source/private.rs"):
            run, artifact, payload, args = self.artifact(extra={name: b"private"})
            with self.assertRaises(ValueError):
                gate.validate_readback(run, artifact, payload, **args)


if __name__ == "__main__":
    unittest.main(verbosity=2)
