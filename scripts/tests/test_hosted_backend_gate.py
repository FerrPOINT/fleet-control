"""Static/in-memory and owned Linux process tests; no compiler, Docker, PG or API calls."""
from collections import Counter
import copy
import importlib.util
import io
import json
import os
import re
from pathlib import Path
import subprocess
import sys
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
        self.assertEqual(flow["permissions"], {"contents": "read", "actions": "read"})
        self.assertEqual(flow["concurrency"]["cancel-in-progress"], "false")
        self.assertEqual(set(flow["jobs"]), {"backend"})
        job = flow["jobs"]["backend"]
        self.assertEqual(job["runs-on"], "ubuntu-24.04")
        self.assertEqual(job["container"], dict(image="rust:1.88.0-bookworm", options="--cpus 2 --memory 4g"))
        self.assertEqual(job["env"]["CARGO_BUILD_JOBS"], "1")
        self.assertIn("github.event.deleted == false", job["if"])
        self.assertIn("github.ref == 'refs/heads/" + gate.BRANCH + "'", job["if"])

    def test_container_run_steps_explicitly_use_bash(self):
        job = self.workflow()["jobs"]["backend"]
        self.assertEqual(job["defaults"], {"run": {"shell": "bash"}})
        for step in job["steps"]:
            if "run" in step:
                self.assertEqual(step.get("shell", job["defaults"]["run"]["shell"]), "bash")

    def test_precheckout_failure_skips_fallback_postcheckout_cleanup_stays_fail_closed(self):
        steps = self.workflow()["jobs"]["backend"]["steps"]
        controls = next(step for step in steps if step.get("id") == "controls")
        self.assertTrue(controls["uses"].startswith("actions/checkout@"))
        self.assertEqual(controls["with"]["path"], "controls")
        self.assertNotIn("continue-on-error", controls)
        execute = next(step for step in steps if step.get("run", "").endswith(" execute"))
        self.assertNotIn("if", execute)
        self.assertNotIn("continue-on-error", execute)
        cleanup = next(step for step in steps if step.get("run", "").endswith(" cleanup"))
        self.assertEqual(cleanup["if"], "always() && steps.controls.outcome == 'success'")
        self.assertEqual(cleanup["run"], "python3 -B controls/scripts/hosted_backend_gate.py cleanup")
        self.assertNotIn("continue-on-error", cleanup)
        self.assertLess(steps.index(controls), steps.index(execute))
        self.assertLess(steps.index(execute), steps.index(cleanup))

    def test_managed_synthetic_pg_no_ports_no_production_credentials(self):
        service = self.workflow()["jobs"]["backend"]["services"]["postgres"]
        self.assertEqual(service["image"], "postgres:17.6-alpine")
        self.assertEqual(service["env"], dict(POSTGRES_USER="fleet_test", POSTGRES_DB="fleet_foundation_test",
                                               POSTGRES_HOST_AUTH_METHOD="trust"))
        self.assertNotIn("ports", service)
        urls = gate.database_environment()
        self.assertEqual(urls["FLEET_REAL_AUTH_TEST_DATABASE_URL"], "postgres://fleet_test@postgres:5432/fleet_real_auth_test")
        self.assertEqual(len(urls), 19)
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
        self.assertEqual({step["ref"] for step in private}, {gate.BASE_SHA, gate.AUTH_SHA, gate.UTILITY_SHA})
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

    def test_all_162_ignored_exactly_once_and_290_default_declarations(self):
        records = REVIEWED["ignored"]
        self.assertEqual(len(records), 162)
        self.assertEqual(len({(x["source"], x["name"]) for x in records}), 162)
        self.assertEqual(Counter(x["gate"] for x in records)["runtime_controls"], 30)
        self.assertEqual(len(REVIEWED["groups"]["runtime_terminal"]), 14)
        self.assertEqual(len(REVIEWED["groups"]["foundation"]), 44)
        self.assertEqual(REVIEWED["default_foundation_ignored"], 115)
        self.assertEqual(len(REVIEWED["workspace_default_declarations"]), 290)
        self.assertEqual(REVIEWED["authority"]["old_ignored"], 130)
        for x in records:
            self.assertIn(x["name"], REVIEWED["groups"][x["gate"]])

    def test_required_full_commands_migrations_and_real_auth_remain(self):
        text = (ROOT / gate.GATE).read_text()
        for command in ("cargo fmt --all -- --check", "cargo check --locked --workspace --all-targets",
                        "cargo clippy --locked --workspace --all-targets --message-format=json -- -D warnings",
                        "cargo test --locked --workspace -- --test-threads=1",
                        "cargo test --locked -p migration --lib lineage_tests -- --include-ignored --test-threads=1",
                        "cargo run --locked -p migration -- down -n 1", "cargo run --locked -p migration -- down -n 20",
                        "cargo build --locked -p auth-server --bin auth-server",
                        "cargo test --locked -p infra --test pm_credentials_real_auth -- --ignored --test-threads=1",
                        "cmp ../openapi/openapi.json ${QA_OUTPUT}/openapi.json"):
            self.assertIn(command, text)
        self.assertEqual(len(gate.GATES), 70)
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

    def test_compiler_ignored_list_must_be_exact_162_no_missing_extra_duplicate(self):
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
        text = "test first ... ok\ntest second ... ok\ntest result: ok. 2 passed; 0 failed; 162 ignored;\n"
        result = gate.verify_test_log("workspace", text, REVIEWED, listing)
        self.assertEqual(result["passed"], 2)
        for bad in (text.replace("test second ... ok\n", ""), text.replace("162 ignored", "130 ignored"), ""):
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
            root = parent / "fleet-backend-b249bc8"
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
        process = SimpleNamespace(pid=12345, returncode=None, wait=mock.Mock())
        with mock.patch.object(gate, "leader_status") as held, \
                mock.patch.object(gate.os, "getpgid", return_value=12345, create=True), \
                mock.patch.object(gate.os, "getsid", return_value=12345, create=True), \
                mock.patch.object(gate, "live_group", return_value=False), \
                mock.patch.object(gate.os, "killpg", create=True) as kill, \
                mock.patch.object(gate.signal, "SIGKILL", 9, create=True):
            gate.stop_owned_group(process)
            self.assertEqual([call.args[0] for call in kill.call_args_list], [12345, 12345])
            self.assertEqual(held.call_count, 2)
            process.wait.assert_called_once_with(timeout=10)
            self.assertTrue(process._fleet_group_drained)

    def test_reaped_leader_is_never_signalled(self):
        process = SimpleNamespace(pid=12345, returncode=0, wait=mock.Mock())
        with mock.patch.object(gate.os, "killpg", create=True) as kill, \
                mock.patch.object(gate.signal, "SIGKILL", 9, create=True):
            with self.assertRaises(ValueError):
                gate.stop_owned_group(process)
            kill.assert_not_called()
            process.wait.assert_not_called()

    def test_lost_waitid_custody_never_signals(self):
        process = SimpleNamespace(pid=12345, returncode=None, wait=mock.Mock())
        with mock.patch.object(gate, "leader_status", side_effect=ChildProcessError), \
                mock.patch.object(gate.os, "killpg", create=True) as kill, \
                mock.patch.object(gate.signal, "SIGKILL", 9, create=True):
            with self.assertRaises(ChildProcessError):
                gate.stop_owned_group(process)
            kill.assert_not_called()
            process.wait.assert_not_called()

    def test_group_and_session_custody_checked_before_each_signal(self):
        for pgids, sids, signals in (([3], [12345], 0), ([12345], [3], 0),
                                     ([12345, 3], [12345], 1), ([12345, 12345], [12345, 3], 1)):
            process = SimpleNamespace(pid=12345, returncode=None, wait=mock.Mock())
            with mock.patch.object(gate, "leader_status"), \
                    mock.patch.object(gate.os, "getpgid", side_effect=pgids, create=True), \
                    mock.patch.object(gate.os, "getsid", side_effect=sids, create=True), \
                    mock.patch.object(gate.os, "killpg", create=True) as kill, \
                    mock.patch.object(gate.signal, "SIGKILL", 9, create=True):
                with self.assertRaises(ValueError):
                    gate.stop_owned_group(process)
                self.assertEqual(kill.call_count, signals)
                process.wait.assert_not_called()

    def test_drain_precedes_reap_and_cleanup_is_idempotent_after_interrupt(self):
        events = []
        process = SimpleNamespace(pid=12345, returncode=None)
        def reap(**_):
            self.assertTrue(process._fleet_group_drained)
            events.append("reap")
            process.returncode = 0
            raise TimeoutError("interrupted after reap")
        process.wait = mock.Mock(side_effect=reap)
        with mock.patch.object(gate, "leader_status"), \
                mock.patch.object(gate.os, "getpgid", return_value=12345, create=True), \
                mock.patch.object(gate.os, "getsid", return_value=12345, create=True), \
                mock.patch.object(gate, "live_group", side_effect=lambda _: events.append("drain") or False), \
                mock.patch.object(gate.os, "killpg", side_effect=lambda *_: events.append("signal"), create=True), \
                mock.patch.object(gate.signal, "SIGKILL", 9, create=True):
            with self.assertRaises(TimeoutError):
                gate.stop_owned_group(process)
            gate.stop_owned_group(process)
        self.assertEqual(events, ["signal", "signal", "drain", "reap"])
        process.wait.assert_called_once()

    def test_group_drain_failure_never_reaps(self):
        process = SimpleNamespace(pid=12345, returncode=None, wait=mock.Mock())
        with mock.patch.object(gate, "leader_status"), \
                mock.patch.object(gate.os, "getpgid", return_value=12345, create=True), \
                mock.patch.object(gate.os, "getsid", return_value=12345, create=True), \
                mock.patch.object(gate.os, "killpg", create=True), \
                mock.patch.object(gate.signal, "SIGKILL", 9, create=True), \
                mock.patch.object(gate, "live_group", return_value=True), \
                mock.patch.object(gate.time, "monotonic", side_effect=[0, 10]):
            with self.assertRaises(ValueError):
                gate.stop_owned_group(process)
            process.wait.assert_not_called()
            self.assertFalse(getattr(process, "_fleet_group_drained", False))

    @unittest.skipUnless(sys.platform == "linux", "Linux WNOWAIT/procfs process custody")
    def test_linux_exited_leader_held_until_real_descendant_drains(self):
        program = ("import os,signal,time\n"
                   "pid=os.fork()\n"
                   "if pid==0:\n signal.signal(signal.SIGTERM,signal.SIG_IGN)\n time.sleep(20)\n os._exit(0)\n"
                   "print(pid,flush=True)\nos._exit(0)\n")
        process = subprocess.Popen([sys.executable, "-c", program], stdout=subprocess.PIPE,
                                   stderr=subprocess.DEVNULL, start_new_session=True)
        try:
            self.assertEqual(gate.wait_owned_exit(process, 5), 0)
            self.assertIsNone(process.returncode)
            self.assertIsNotNone(gate.leader_status(process))
            self.assertEqual(os.getsid(process.pid), process.pid)
            gate.stop_owned_group(process)
            self.assertEqual(process.returncode, 0)
            self.assertFalse(gate.live_group(process))
            with mock.patch.object(gate.os, "killpg") as kill:
                gate.stop_owned_group(process)
                kill.assert_not_called()
        finally:
            gate.stop_owned_group(process)
            process.stdout.close()

    @unittest.skipUnless(sys.platform == "linux", "Linux WNOWAIT command custody")
    def test_linux_command_drains_descendant_holding_output_pipe(self):
        program = ("import os,time\n"
                   "if os.fork()==0:\n time.sleep(20)\n os._exit(0)\n"
                   "print('owned-output',flush=True)\nos._exit(0)\n")
        self.assertEqual(gate.command([sys.executable, "-c", program]), b"owned-output\n")

    def test_gate_verification_commands_do_not_start_an_escaping_session(self):
        with mock.patch.object(gate, "VERIFY_IN_GATE", True), \
                mock.patch.object(gate.subprocess, "run", return_value=SimpleNamespace(returncode=0, stdout=b"ok")) as run, \
                mock.patch.object(gate.subprocess, "Popen") as spawn:
            self.assertEqual(gate.command(["git", "read-only-probe"]), b"ok")
            self.assertEqual(run.call_args.kwargs, dict(capture_output=True, timeout=300))
            spawn.assert_not_called()
        if sys.platform == "linux":
            with mock.patch.object(gate, "VERIFY_IN_GATE", True):
                actual = gate.command([sys.executable, "-c", "import os;print(os.getpgrp())"])
                self.assertEqual(int(actual), os.getpgrp())

    def test_workflow_all_steps_bounded_and_metadata_token_not_job_global(self):
        job = self.workflow()["jobs"]["backend"]
        self.assertEqual(job["timeout-minutes"], "120")
        self.assertNotIn("FLEET_GATE_READ_TOKEN", job["env"])
        for step in job["steps"]:
            self.assertGreater(int(step["timeout-minutes"]), 0)
            if step.get("run", "").endswith((" preflight", " execute", " cleanup")):
                self.assertEqual(step["env"], {"FLEET_GATE_READ_TOKEN": "${{ github.token }}"})
            else:
                self.assertNotIn("FLEET_GATE_READ_TOKEN", step.get("env", {}))

    def job_metadata(self):
        return dict(total_count=1, jobs=[dict(id=11, name="backend", status="in_progress", run_id=12,
            run_attempt=2, head_sha="a" * 40, runner_name="GitHub Actions 123",
            started_at="2026-10-09T17:00:00Z")])

    def metadata_environment(self):
        return mock.patch.dict(os.environ, dict(GITHUB_RUN_ID="12", GITHUB_RUN_ATTEMPT="2",
            GITHUB_SHA="a" * 40, RUNNER_NAME="GitHub Actions 123"))

    def test_metadata_accounts_for_setup_request_and_rounding_not_fresh_6600(self):
        with self.metadata_environment():
            remaining = gate.metadata_remaining(self.job_metadata(), "Fri, 09 Oct 2026 17:10:01 GMT", 3)
        self.assertEqual(remaining, 7200 - 601 - 3 - 2)
        with mock.patch.object(gate.time, "monotonic", return_value=100):
            budget = gate.JobBudget(remaining)
            with mock.patch.object(gate, "BUDGET", budget):
                self.assertEqual(gate.bounded_timeout(6600), 6054)
                self.assertEqual(budget.job_end - budget.work_end, 540)

    def test_metadata_rejects_wrong_job_attempt_sha_runner_and_clock(self):
        with self.metadata_environment():
            for field, value in (("id", False), ("name", "other"), ("status", "completed"),
                                  ("run_id", 13), ("run_attempt", 3), ("head_sha", "b" * 40),
                                  ("runner_name", "foreign")):
                payload = self.job_metadata()
                payload["jobs"][0][field] = value
                with self.assertRaises(ValueError):
                    gate.metadata_remaining(payload, "Fri, 09 Oct 2026 17:10:01 GMT", 0)
            for date in ("Fri, 09 Oct 2026 16:59:59 GMT", "Fri, 09 Oct 2026 19:00:00 GMT"):
                with self.assertRaises(ValueError):
                    gate.metadata_remaining(self.job_metadata(), date, 0)
            for count in (0, 2, True):
                with self.assertRaises(ValueError):
                    gate.metadata_remaining(dict(self.job_metadata(), total_count=count),
                                            "Fri, 09 Oct 2026 17:10:01 GMT", 0)

    def test_expired_budget_sticky_and_cannot_renew_on_next_command(self):
        with mock.patch.object(gate.time, "monotonic", return_value=0):
            budget = gate.JobBudget(600)
        with mock.patch.object(gate, "BUDGET", budget), \
                mock.patch.object(gate.time, "monotonic", return_value=61):
            with self.assertRaises(TimeoutError):
                gate.bounded_timeout(300)
        with mock.patch.object(gate.time, "monotonic", return_value=1):
            with self.assertRaises(TimeoutError):
                budget.check()

    def test_cleanup_and_fallback_have_separate_reserves_without_job_extension(self):
        with mock.patch.object(gate.time, "monotonic", return_value=0):
            budget = gate.JobBudget(600)
        with mock.patch.object(gate.time, "monotonic", return_value=60), \
                mock.patch.object(gate.signal, "setitimer", create=True), \
                mock.patch.object(gate.signal, "signal"), \
                mock.patch.object(gate.signal, "SIGALRM", 14, create=True), \
                mock.patch.object(gate.signal, "ITIMER_REAL", 0, create=True):
            budget.expired = True
            budget.cleanup()
            self.assertEqual(budget.end, 360)
            self.assertEqual(budget.job_end, 600)
        with mock.patch.object(gate.time, "monotonic", return_value=375), \
                mock.patch.object(gate.signal, "setitimer", create=True), \
                mock.patch.object(gate.signal, "signal"), \
                mock.patch.object(gate.signal, "SIGALRM", 14, create=True), \
                mock.patch.object(gate.signal, "ITIMER_REAL", 0, create=True):
            budget.cleanup(fallback=True)
            self.assertEqual(budget.end, 420)
            self.assertEqual(budget.job_end - budget.end, 180)

    def test_cleanup_cannot_start_after_artifact_reserve_consumed(self):
        with mock.patch.object(gate.time, "monotonic", return_value=0):
            budget = gate.JobBudget(600)
        with mock.patch.object(gate.time, "monotonic", return_value=421), \
                mock.patch.object(gate.signal, "setitimer", create=True), \
                mock.patch.object(gate.signal, "SIGALRM", 14, create=True), \
                mock.patch.object(gate.signal, "ITIMER_REAL", 0, create=True):
            with self.assertRaises(TimeoutError):
                budget.cleanup(fallback=True)

    def test_local_budget_fails_before_network_or_token_read(self):
        with mock.patch.dict(os.environ, {}, clear=True), \
                mock.patch.object(gate.urllib.request, "build_opener") as opener:
            with self.assertRaises(ValueError):
                gate.hosted_budget()
            opener.assert_not_called()

    def test_gate_exit_observed_without_wait_poll_or_communicate(self):
        source = (ROOT / gate.HELPER).read_text()
        body = source[source.index("def execute():"):source.index("def cleanup_fallback():")]
        self.assertIn("wait_owned_exit(process, GATE_SECONDS)", body)
        self.assertNotIn("process.wait(", body)
        self.assertNotIn("process.poll(", body)
        self.assertNotIn("process.communicate(", body)

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
        self.assertEqual(cleanup["if"], "always() && steps.controls.outcome == 'success'")
        helper = (ROOT / gate.HELPER).read_text()
        self.assertIn("finally:", helper)
        self.assertIn("start_new_session=True", helper)
        self.assertIn('stdout=diagnostics, stderr=diagnostics', helper)
        self.assertNotIn('dict(os.environ,', helper)

    def compiler_line(self, file="api/src/routes/sessions.rs", code="E0308", line=17, column=9, primary=True):
        return (json.dumps(dict(reason="compiler-message", package_id="PRIVATE_SENTINEL_PACKAGE",
             message=dict(level="error", code=dict(code=code, explanation="PRIVATE_SENTINEL_EXPLANATION"),
                          message="PRIVATE_SENTINEL_MESSAGE", rendered="PRIVATE_SENTINEL_RENDERED",
                          children=[dict(message="PRIVATE_SENTINEL_CHILD", spans=[])],
                          spans=[dict(file_name=file, line_start=line, column_start=column, is_primary=primary,
                                      text=[dict(text="PRIVATE_SENTINEL_SOURCE")], label="PRIVATE_SENTINEL_LABEL")]))) + "\n").encode()

    def diagnostics(self, out, err=b""):
        return gate.safe_compiler_diagnostics((io.BytesIO(out), io.BytesIO(err)), REVIEWED["rust_source_sha256"], "/owned/src/fleet-control/backend")

    def test_compiler_json_only_code_allowlisted_primary_location_no_context(self):
        result = self.diagnostics(self.compiler_line())
        self.assertEqual(result, dict(diagnostics=[dict(error_code="E0308", file="backend/api/src/routes/sessions.rs", line=17, column=9)],
                                      categories=[], truncated=False))
        self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))

    def test_compiler_paths_only_exact_fleet_allowlist_no_private_prefix_or_traversal(self):
        for file in ("api/src/routes/sessions.rs", "backend/api/src/routes/sessions.rs", "/owned/src/fleet-control/backend/api/src/routes/sessions.rs"):
            self.assertEqual(self.diagnostics(self.compiler_line(file))["diagnostics"][0]["file"], "backend/api/src/routes/sessions.rs")
        for file in ("../services-base/crates/private.rs", "/private/api/src/routes/sessions.rs", "C:/private/x.rs",
                     "api\\src\\routes\\sessions.rs", "/owned/src/fleet-control/backend/../private.rs",
                     "api//src/routes/sessions.rs", "api/src/routes/./sessions.rs", "api/.local/private.rs",
                     "target/private.rs", "api/src/routes/sessions.rs\nPRIVATE_SENTINEL", "api/src/private.rs"):
            self.assertEqual(self.diagnostics(self.compiler_line(file))["diagnostics"], [])

    def test_crate_relative_span_requires_allowlisted_target_never_guesses(self):
        value = json.loads(self.compiler_line("src/routes/sessions.rs"))
        self.assertEqual(self.diagnostics((json.dumps(value) + "\n").encode())["diagnostics"], [])
        value["target"] = dict(src_path="/owned/src/fleet-control/backend/api/src/lib.rs")
        self.assertEqual(self.diagnostics((json.dumps(value) + "\n").encode())["diagnostics"][0]["file"], "backend/api/src/routes/sessions.rs")
        for entry in ("/private/base/src/lib.rs", "../api/src/lib.rs", "api/src/private.rs"):
            value["target"]["src_path"] = entry
            self.assertEqual(self.diagnostics((json.dumps(value) + "\n").encode())["diagnostics"], [])

    def test_compiler_untrusted_codes_and_numeric_boundaries(self):
        for code in (None, "clippy::private_token", "E1234 PRIVATE_SENTINEL", "E１２３４", ["E0308"], "PRIVATE_SENTINEL"):
            result = self.diagnostics(self.compiler_line(code=code))
            self.assertIsNone(result["diagnostics"][0]["error_code"])
            self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))
        for line, column in ((True, 9), (1, True), ("17", 9), (0, 9), (-1, 9), (1000001, 9), (17, 10001), (17, 0)):
            self.assertEqual(self.diagnostics(self.compiler_line(line=line, column=column))["diagnostics"], [])

    def test_compiler_only_error_primary_spans_not_rendered_children_or_artifacts(self):
        value = json.loads(self.compiler_line())
        for level in ("warning", "note", "help", "PRIVATE_SENTINEL"):
            value["message"]["level"] = level
            self.assertEqual(self.diagnostics((json.dumps(value) + "\n").encode())["diagnostics"], [])
        self.assertEqual(self.diagnostics(self.compiler_line(primary=False))["diagnostics"], [])
        value["reason"], value["message"]["level"] = "compiler-artifact", "error"
        self.assertEqual(self.diagnostics((json.dumps(value) + "\n").encode())["diagnostics"], [])

    def test_nonjson_categories_fixed_allowlist_never_echo_raw(self):
        for category, patterns in gate.CATEGORY_PATTERNS.items():
            result = self.diagnostics(b"", (patterns[0] + " PRIVATE_SENTINEL /private/base/token\n").encode())
            self.assertEqual(result, dict(diagnostics=[], categories=[category], truncated=False))
            self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))
        self.assertEqual(self.diagnostics(b"PRIVATE_SENTINEL\n")["categories"], ["unknown"])

    def test_malformed_json_wrong_shapes_and_nested_context_fail_closed(self):
        for raw in (b"{malformed PRIVATE_SENTINEL\n", b"[]\n", b"null\n", b'{"reason":"compiler-message","message":"PRIVATE_SENTINEL"}\n',
                    b'{"reason":"compiler-message","message":{"level":"error","spans":"PRIVATE_SENTINEL"}}\n'):
            self.assertEqual(self.diagnostics(raw), dict(diagnostics=[], categories=["unknown"], truncated=False))

    def test_compiler_deduplication_and_record_limit_are_bounded(self):
        same = self.compiler_line()
        self.assertEqual(len(self.diagnostics(same + same)["diagnostics"]), 1)
        result = self.diagnostics(b"".join(self.compiler_line(line=line) for line in range(1, 80)))
        self.assertEqual(len(result["diagnostics"]), gate.DIAGNOSTIC_LIMIT)
        self.assertTrue(result["truncated"])
        self.assertIn("unknown", result["categories"])

    def test_compiler_input_and_line_limits_discard_oversized_context(self):
        with mock.patch.object(gate, "DIAGNOSTIC_LINE_LIMIT", 64), mock.patch.object(gate, "DIAGNOSTIC_INPUT_LIMIT", 256):
            result = self.diagnostics(b"PRIVATE_SENTINEL" * 100 + b"\n" + self.compiler_line())
        self.assertTrue(result["truncated"])
        self.assertEqual(result["diagnostics"], [])
        self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))

    def test_compiler_log_io_failure_does_not_echo_or_lose_recorded_exit(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "private").mkdir()
            (root / "private/check.exit").write_bytes(b"101\n")
            result = gate.compiler_failure_logs(root, "check", REVIEWED)
            self.assertEqual(result, dict(diagnostics=[], categories=["unknown"], truncated=False, command_exit_code=101))
            (root / "private/check.exit").write_bytes(b"PRIVATE_SENTINEL")
            self.assertIsNone(gate.compiler_failure_logs(root, "check", REVIEWED)["command_exit_code"])

    def test_compiler_json_commands_keep_locked_all_targets_clippy_exit_and_finally(self):
        text = (ROOT / gate.GATE).read_text()
        for command in ("run_compiler check cargo check --locked --workspace --all-targets --message-format=json",
                        "run_compiler clippy cargo clippy --locked --workspace --all-targets --message-format=json -- -D warnings"):
            self.assertIn(command, text)
        self.assertIn('2> "$QA_OUTPUT/$stage.stderr" || code=$?', text)
        self.assertIn('if ((code != 0)); then exit "$code"; fi', text)
        helper = (ROOT / gate.HELPER).read_text()
        block = helper[helper.index("    finally:\n", helper.index("def execute()")):]
        self.assertLess(block.index("stop_owned_group(process)"), block.index("compiler_failure_logs("))
        self.assertLess(block.index("compiler_failure_logs("), block.index("drop_databases(owned_dbs)"))
        self.assertLess(block.index("remove_scratch(root, temporary, identity)"), block.index("failure_root.mkdir"))
        self.assertIn("return 0 if success else code if type(code) is int and 1 <= code <= 255 else 1", helper)

    def test_compiler_capture_preserves_exit_without_running_any_cargo(self):
        text = (ROOT / gate.GATE).read_text()
        start = text.index("run_compiler() {")
        function = text[start:text.index("\n}\n", start) + 3]
        bash = "C:/Program Files/Git/bin/bash.exe" if os.name == "nt" else "bash"
        for code in (0, 101):
            with self.subTest(code=code), tempfile.TemporaryDirectory() as temporary:
                script = "set -euo pipefail\nQA_OUTPUT=.\n" + function + \
                    '\npassed() { printf passed > passed-marker; }\n' + \
                    f'fake_compiler() {{ printf PRIVATE_SENTINEL; printf PRIVATE_SENTINEL >&2; return {code}; }}\n' + \
                    'run_compiler check fake_compiler\n'
                result = subprocess.run([bash, "-c", script], cwd=temporary, capture_output=True, timeout=10)
                self.assertEqual(result.returncode, code)
                self.assertEqual(result.stdout + result.stderr, b"")
                self.assertEqual((Path(temporary) / "check.exit").read_text().strip(), str(code))
                self.assertEqual((Path(temporary) / "passed-marker").exists(), code == 0)

    def test_failure_artifact_explicit_separate_never_pass_artifact(self):
        steps = self.workflow()["jobs"]["backend"]["steps"]
        failed = next(step for step in steps if step.get("with", {}).get("name", "").startswith("fleet-backend-failure-"))
        self.assertEqual(failed["if"], "failure() && steps.gate.outcome == 'failure'")
        self.assertEqual(failed["with"]["path"], "${{ runner.temp }}/fleet-backend-failure-evidence/compiler-diagnostics.json")
        self.assertEqual(failed["with"]["retention-days"], "14")
        self.assertEqual(failed["with"]["if-no-files-found"], "warn")
        self.assertEqual(failed["with"]["overwrite"], "false")
        self.assertEqual(next(step for step in steps if step.get("id") == "gate")["run"], "python3 -B controls/scripts/hosted_backend_gate.py execute")
        self.assertLess(next(i for i, step in enumerate(steps) if step.get("run", "").endswith(" cleanup")), steps.index(failed))

    def failure_value(self):
        return dict(version=1, kind="safe_compiler_failure", status="failure", repository=gate.REPOSITORY, branch=gate.BRANCH,
                    workflow_sha="a" * 40, workflow_path=gate.WORKFLOW, run_id=123, run_attempt=1,
                    source_sha=gate.SOURCE_SHA, base_sha=gate.BASE_SHA, auth_sha=gate.AUTH_SHA,
                    utility_sha=gate.UTILITY_SHA, utility_inventory_sha256=gate.UTILITY_INVENTORY_SHA,
                    source_inventory_sha256=gate.SOURCE_INVENTORY_SHA,
                    control_sha256={name: gate.digest((ROOT / name).read_bytes()) for name in sorted(gate.WRITE_SET)},
                    stage="check", gate_failed_stage="check", gate_exit_code=101, command_exit_code=101,
                    **self.diagnostics(self.compiler_line()), cleanup=dict(scratch=True, synthetic_databases=True),
                    backend_quality_gate=False, all_quality_gate=False, sdlc_acceptance=False)

    def validate_failure(self, value):
        return gate.validate_failure_evidence(value, workflow_sha="a" * 40, run_id=123, attempt=1)

    def test_failure_schema_strict_no_private_fields_or_fake_pass(self):
        value = self.failure_value()
        self.validate_failure(value)
        for key, item in (("message", "PRIVATE_SENTINEL"), ("rendered", "PRIVATE_SENTINEL"), ("backend_quality_gate", True),
                          ("all_quality_gate", True), ("sdlc_acceptance", True), ("source_sha", "bf27"),
                          ("source_inventory_sha256", "0" * 64), ("command_exit_code", "PRIVATE_SENTINEL"),
                          ("categories", ["PRIVATE_SENTINEL"]), ("control_sha256", {})):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(value, **{key: item}))
        for key, item in (("message", "PRIVATE_SENTINEL"), ("file", "../services-base/private.rs"), ("error_code", "E0308 TOKEN"), ("line", True)):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(value, diagnostics=[dict(value["diagnostics"][0], **{key: item})]))

    def failure_artifact(self, extra=None):
        run, artifact, _, args = self.artifact()
        run["conclusion"] = "failure"
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w") as archive:
            archive.writestr(gate.FAILURE_FILE, gate.canonical(self.failure_value()))
            for name in extra or ():
                archive.writestr(name, b"PRIVATE_SENTINEL")
        payload = buffer.getvalue()
        artifact.update(name="fleet-backend-failure-b249bc8-123-1", digest="sha256:" + gate.digest(payload))
        args["artifact_digest"] = gate.digest(payload)
        return run, artifact, payload, args

    def test_failure_readback_authenticates_failure_run_attempt_sha_artifact_digest(self):
        run, artifact, payload, args = self.failure_artifact()
        self.assertEqual(set(gate.validate_failure_readback(run, artifact, payload, **args)), {gate.FAILURE_FILE})
        for key, item in (("id", 456), ("run_attempt", 2), ("conclusion", "success"), ("event", "pull_request"), ("head_sha", "b" * 40)):
            with self.assertRaises(ValueError):
                gate.validate_failure_readback(dict(run, **{key: item}), artifact, payload, **args)
        for key, item in (("expired", True), ("name", "fleet-backend-b249bc8-123-1"), ("digest", "sha256:" + "0" * 64)):
            with self.assertRaises(ValueError):
                gate.validate_failure_readback(run, dict(artifact, **{key: item}), payload, **args)
        with self.assertRaises(ValueError):
            gate.validate_readback(run, artifact, payload, **args)

    def test_failure_zip_rejects_raw_extra_traversal_duplicate_and_size(self):
        for extra in (["../private"], ["cargo-stderr.log"], [gate.FAILURE_FILE]):
            with self.subTest(extra=extra):
                if extra == [gate.FAILURE_FILE]:
                    with self.assertWarns(UserWarning):
                        run, artifact, payload, args = self.failure_artifact(extra)
                else:
                    run, artifact, payload, args = self.failure_artifact(extra)
                with self.assertRaises(ValueError):
                    gate.validate_failure_readback(run, artifact, payload, **args)
        run, artifact, payload, args = self.failure_artifact()
        payload = b"x" * (2 * gate.FAILURE_SIZE_LIMIT + 1)
        artifact["digest"] = "sha256:" + gate.digest(payload)
        args["artifact_digest"] = gate.digest(payload)
        with self.assertRaises(ValueError):
            gate.validate_failure_readback(run, artifact, payload, **args)

    def test_failure_cleanup_false_remains_failure_and_never_asserts_acceptance(self):
        value = self.failure_value()
        value.update(gate_failed_stage="cleanup", cleanup=dict(scratch=False, synthetic_databases=False))
        self.validate_failure(value)
        self.assertFalse(value["backend_quality_gate"])
        self.assertFalse(value["all_quality_gate"])
        self.assertFalse(value["sdlc_acceptance"])

    def artifact(self, report_changes=None, provenance_changes=None, extra=None):
        workflow_sha = "a" * 40
        focused = {name: dict(passed=len(names), failed=0, ignored=115 if name == "foundation" else 0,
                              tests=sorted(names)) for name, names in REVIEWED["groups"].items()}
        focused["workspace"] = dict(passed=1, failed=0, ignored=162, tests=["normal"])
        report = dict(backend_quality_gate=True, all_quality_gate=False, sdlc_acceptance=False, status="success",
                      gates=[dict(stage=name, status="passed") for name in gate.GATES], focused=focused,
                      cleanup=dict(scratch=True, synthetic_databases=True), ignored_required=162, foundation_ignored=115,
                      contracts={stage: dict(passed=len(names), failed=0, ignored=0, tests=sorted(names))
                                 for stage, names in REVIEWED["python_contracts"].items()},
                      migration_ledger=gate.expected_migration_receipt(REVIEWED),
                      runtime_inventory=dict(ignored=162, listed_default_count=1, ignored_names_sha256=gate.digest(gate.canonical(
                          sorted(item["name"] for item in REVIEWED["ignored"])))))
        report.update(report_changes or {})
        provenance = dict(version=1, repository=gate.REPOSITORY, branch=gate.BRANCH, source_sha=gate.SOURCE_SHA,
                          base_sha=gate.BASE_SHA, auth_sha=gate.AUTH_SHA, workflow_sha=workflow_sha, workflow_path=gate.WORKFLOW,
                          utility_sha=gate.UTILITY_SHA, utility_tree=REVIEWED["utility_tree"],
                          utility_inventory_sha256=gate.UTILITY_INVENTORY_SHA, source_inventory_sha256=gate.SOURCE_INVENTORY_SHA,
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
                        name="fleet-backend-b249bc8-123-1", digest="sha256:" + gate.digest(payload))
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

    def python_log(self, stage):
        suite = unittest.TestSuite()
        for identity in REVIEWED["python_contracts"][stage]:
            cls_path, method = identity.rsplit(".", 1)
            module, cls_name = cls_path.rsplit(".", 1)
            cls = type(cls_name, (unittest.TestCase,), {"__module__": module, method: lambda self: None})
            suite.addTest(cls(method))
        output = io.StringIO()
        result = unittest.TextTestRunner(stream=output, verbosity=2).run(suite)
        self.assertTrue(result.wasSuccessful())
        return output.getvalue()

    def test_python_actual_unittest_formatter_has_exact_5_15_21_16_receipts(self):
        for stage, names in REVIEWED["python_contracts"].items():
            with self.subTest(stage=stage):
                result = gate.verify_python_log(stage, self.python_log(stage), REVIEWED)
                self.assertEqual(result, dict(passed=len(names), failed=0, ignored=0, tests=sorted(names)))

    def test_python_receipt_rejects_missing_duplicate_skipped_and_count_only_logs(self):
        for stage in REVIEWED["python_contracts"]:
            text = self.python_log(stage)
            first = next(line for line in text.splitlines() if " ... ok" in line)
            for bad in (text.replace(first + "\n", "", 1), text + first + "\n",
                        text.replace(" ... ok", " ... skipped 'private'", 1),
                        re.sub(r"Ran \d+ tests", "Ran 0 tests", text),
                        "Ran 21 tests in 0.001s\n\nOK\n", text.replace("\nOK\n", "\nOK (skipped=1)\n")):
                with self.subTest(stage=stage), self.assertRaises(ValueError):
                    gate.verify_python_log(stage, bad, REVIEWED)

    def test_each_new_stage_has_real_invocation_and_correct_order(self):
        text = (ROOT / gate.GATE).read_text()
        names = gate.GATES[43:-3]
        self.assertEqual(len(names), 24)
        positions = []
        for name in names:
            command = re.search(r"^(?:run_tests|run_ignored_target|run_python_contract) " + name + r"\b.*$", text, re.M)
            self.assertIsNotNone(command, name)
            positions.append(command.start())
        self.assertEqual(positions, sorted(positions))
        self.assertLess(positions[-1], text.index("stage=migration_smoke"))
        self.assertIn('verify_python_log(stage, (root / (stage + ".log")).read_text(), reviewed)', (ROOT / gate.HELPER).read_text())

    def test_new_rust_ignored_targets_each_require_actual_exact_names(self):
        for stage in ("container_controller_pg", "container_preparation_pg", "container_activation_pg",
                      "container_controller_migration", "mapped_controller_migration", "container_preparation_migration",
                      "container_activation_migration", "recovered_activation_migration"):
            text = self.log(stage)
            self.assertEqual(gate.verify_test_log(stage, text, REVIEWED)["passed"], len(REVIEWED["groups"][stage]))
            with self.assertRaises(ValueError):
                gate.verify_test_log(stage, text.replace(" ... ok", " ... ignored", 1), REVIEWED)

    def test_all_19_databases_init_environment_and_finally_match_exactly(self):
        created = re.findall(r"^CREATE DATABASE (\w+)", (ROOT / gate.INIT).read_text(), re.M)
        self.assertEqual(set(created) | {"fleet_foundation_test"}, set(gate.DATABASES))
        self.assertEqual(len(created), 18)
        self.assertEqual(set(gate.URLS.values()), set(gate.DATABASES))
        shell = (ROOT / gate.GATE).read_text()
        for name in gate.URLS:
            self.assertIn(name, shell)
        with mock.patch.object(gate, "psql") as psql, mock.patch.object(gate, "database_catalog", return_value=["postgres"]):
            gate.drop_databases(list(gate.DATABASES))
            self.assertEqual(psql.call_count, 20)
            self.assertTrue(all(call.args[0].startswith("DROP DATABASE IF EXISTS") for call in psql.call_args_list[:-1]))

    def test_migration_smoke_exact20_19_20_0_20_not_count_waiver(self):
        expected = gate.expected_migration_receipt(REVIEWED)
        self.assertEqual([len(v) for v in expected.values()], [20, 19, 20, 0, 20])
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for stage, names in expected.items():
                (root / ("migration-" + stage + ".txt")).write_text("".join(n + "\n" for n in names), newline="\n")
            self.assertEqual(gate.verify_migration_snapshots(root, REVIEWED), expected)
            (root / "migration-down_all.txt").write_text("foreign_history\n", newline="\n")
            with self.assertRaises(ValueError):
                gate.verify_migration_snapshots(root, REVIEWED)
        text = (ROOT / gate.GATE).read_text()
        self.assertIn("cargo run --locked -p migration -- down -n 20", text)
        for name in expected:
            self.assertIn("migration_snapshot " + name, text)
        self.assertEqual(len(REVIEWED["migration_registries"]["split"]), 23)
        self.assertIn("split_down_one_and_reapply_preserves_other_history", "\n".join(REVIEWED["groups"]["lineage10"]))

    def test_readback_requires_each_python_stage_exact_cases_and_migration_ledger(self):
        contracts = {stage: dict(passed=len(names), failed=0, ignored=0, tests=sorted(names))
                     for stage, names in REVIEWED["python_contracts"].items()}
        for stage in contracts:
            bad = copy.deepcopy(contracts)
            bad[stage]["tests"].pop()
            run, artifact, payload, args = self.artifact(report_changes=dict(contracts=bad))
            with self.assertRaises(ValueError):
                gate.validate_readback(run, artifact, payload, **args)
        for changes in (dict(contracts={}), dict(migration_ledger={}), dict(ignored_required=135)):
            run, artifact, payload, args = self.artifact(report_changes=changes)
            with self.assertRaises(ValueError):
                gate.validate_readback(run, artifact, payload, **args)

    def test_readback_rejects_wrong_utility_and_compiled_fingerprints(self):
        for field in ("utility_sha", "utility_tree", "utility_inventory_sha256", "source_inventory_sha256"):
            run, artifact, payload, args = self.artifact(provenance_changes={field: "0" * 40})
            with self.assertRaises(ValueError):
                gate.validate_readback(run, artifact, payload, **args)

    def test_utility_clean_exact_worktree_git_blobs_and_hashes_all_required(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            content = b"synthetic immutable utility\n"
            path = root / "scripts/runtime_boundary.py"
            path.parent.mkdir()
            path.write_bytes(content)
            reviewed = dict(utility_source_sha256={"scripts/runtime_boundary.py": gate.digest(content)})
            aggregate = gate.digest(gate.canonical(reviewed["utility_source_sha256"]))
            with mock.patch.object(gate, "UTILITY_INVENTORY_SHA", aggregate), mock.patch.object(gate, "clean_head") as clean, \
                    mock.patch.object(gate, "git", return_value=content):
                self.assertEqual(gate.qualify_utility(root, reviewed), reviewed["utility_source_sha256"])
                clean.assert_called_once_with(root, gate.UTILITY_SHA)
                path.write_bytes(content + b"changed")
                with self.assertRaises(ValueError):
                    gate.qualify_utility(root, reviewed)

    def test_utility_checkout_and_no_secret_child_environment_wiring(self):
        flow = self.workflow()["jobs"]["backend"]["steps"]
        utility = next(x for x in flow if x.get("with", {}).get("path") == "fleet-runtime-contract-base")
        self.assertEqual(utility["with"]["ref"], gate.UTILITY_SHA)
        self.assertEqual(utility["with"]["fetch-depth"], "0")
        helper = (ROOT / gate.HELPER).read_text()
        self.assertEqual(helper.count("qualify_utility(checkouts[3][0], reviewed)"), 2)
        self.assertIn('QA_UTILITY_CHECKOUT=str(checkouts[3][0]), PYTHONDONTWRITEBYTECODE="1"', helper)
        environment = helper.split('environment = {key:', 1)[1].split('with (root / "private/driver.log")', 1)[0]
        for secret in ("GITHUB_TOKEN", "SERVICES_BASE_TOKEN", "ACTIONS_RUNTIME_TOKEN"):
            self.assertNotIn(secret, environment)

    def test_fallback_drops_only_persisted_exact_owned19_before_scratch_removal(self):
        with tempfile.TemporaryDirectory() as temporary:
            parent = Path(temporary).resolve()
            root = parent / "fleet-backend-b249bc8"
            root.mkdir()
            identity = dict(workflow_sha="a" * 40, run_id=123, run_attempt=1)
            (root / "owner.json").write_bytes(gate.canonical(identity))
            marker = root / "owned-databases.json"
            marker.write_bytes(gate.canonical(list(gate.DATABASES)))
            with mock.patch.object(gate, "hosted_identity", return_value=(parent, "a" * 40)), \
                    mock.patch.dict(os.environ, dict(RUNNER_TEMP=str(parent), GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="1")), \
                    mock.patch.object(gate, "drop_databases") as drop, mock.patch.object(gate, "remove_scratch") as remove:
                gate.cleanup_fallback()
                drop.assert_called_once_with(list(gate.DATABASES))
                remove.assert_called_once_with(root, parent, identity)
                marker.write_bytes(gate.canonical(["foreign"]))
                drop.reset_mock()
                with self.assertRaises(ValueError):
                    gate.cleanup_fallback()
                drop.assert_not_called()

    def test_fleet_allowlist_only_required_scripts_no_cache_or_runtime_input(self):
        self.assertEqual(len(REVIEWED["compiled_source_sha256"]), 367)
        self.assertEqual(len(REVIEWED["utility_source_sha256"]), 10)
        for name in REVIEWED["compiled_source_sha256"]:
            self.assertFalse({".local", ".git", "target", "node_modules", "__pycache__", ".venv"}.intersection(Path(name).parts))
        self.assertIn("scripts/check_recovered_activation_contract.py", gate.FLEET_ROOTS)
        self.assertNotIn("scripts", gate.FLEET_ROOTS)


if __name__ == "__main__":
    unittest.main(verbosity=2)
