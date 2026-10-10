"""Static/in-memory and owned Linux process tests; no compiler, Docker, PG or API calls."""
from collections import Counter
import copy
import importlib.util
import io
import json
import os
import re
import socket
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
    def source_blob(self, path, pin=None):
        return subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "show",
            (pin or gate.SOURCE_SHA) + ":" + path], capture_output=True, check=True, timeout=30).stdout

    def test_C11_source_fingerprints_and_complete_declarations_match_canonical_git_blobs(self):
        listing = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "ls-tree", "-r", "-z",
            gate.SOURCE_SHA, "--", *gate.FLEET_ROOTS], capture_output=True, check=True, timeout=30).stdout
        entries = []
        for row in listing.rstrip(b"\0").split(b"\0"):
            header, path = row.split(b"\t")
            mode, kind, oid = header.split()
            self.assertIn(mode, (b"100644", b"100755"))
            self.assertEqual(kind, b"blob")
            entries.append((path.decode(), oid))
        batch = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "cat-file", "--batch"],
            input=b"".join(oid + b"\n" for _, oid in entries), capture_output=True, check=True, timeout=30).stdout
        frames, actual, rust, sources = io.BytesIO(batch), {}, {}, {}
        for path, oid in entries:
            parts = frames.readline().split()
            self.assertEqual(parts[:2], [oid, b"blob"])
            body = frames.read(int(parts[2]))
            self.assertEqual(frames.read(1), b"\n")
            actual["fleet-control/" + path] = gate.digest(body)
            if path.startswith("backend/") and path.endswith(".rs"):
                rust[path] = gate.digest(body)
                sources[path] = body.decode()
        self.assertEqual(frames.read(), b"")
        self.assertEqual(actual, {k: v for k, v in REVIEWED["compiled_source_sha256"].items() if k.startswith("fleet-control/")})
        self.assertEqual(rust, REVIEWED["rust_source_sha256"])
        self.assertEqual(gate.digest(gate.canonical(REVIEWED["compiled_source_sha256"])), gate.SOURCE_INVENTORY_SHA)
        for record in REVIEWED["ignored"] + REVIEWED["workspace_default_declarations"]:
            text = sources[record["source"]]
            lines = text.splitlines(keepends=True)
            line = record["line"]
            name = record["name"].rsplit("::", 1)[-1]
            with self.subTest(source=record["source"], name=name, line=line):
                self.assertRegex(lines[line - 1], r"\bfn " + re.escape(name) + r"\(")
                prefix = "".join(lines[:line - 1])
                markers = list(re.finditer(r"#\[(?:tokio::)?test\]", prefix))
                self.assertTrue(markers)
                attrs = prefix[markers[-1].start():]
                self.assertEqual("#[ignore" in attrs, record.get("ignored", True))
        declarations = set()
        for path, text in sources.items():
            for match in re.finditer(r"#\[(?:tokio::)?test\](?:(?!\bfn\b).)*\b(?:async\s+)?fn\s+(\w+)\(", text, re.S):
                line = text.count("\n", 0, match.end()) + 1
                declarations.add((path, line, match.group(1), "#[ignore" in match.group()))
        expected = {(item["source"], item["line"], item["name"].rsplit("::", 1)[-1], item.get("ignored", True))
                    for item in REVIEWED["ignored"] + REVIEWED["workspace_default_declarations"]}
        self.assertEqual(declarations, expected)

    def test_committed_schema_requires_authentic_parity_not_claimed_prior_acceptance(self):
        self.assertEqual(gate.digest(self.source_blob("openapi/openapi.json")), gate.OPENAPI_SHA)
        self.assertEqual(self.source_blob("openapi/openapi.json"), self.source_blob("openapi/openapi.json", "820a1afe7bc830e2491dff7fffad2bb51f47faf5"))
        self.assertEqual(REVIEWED["openapi_binding"], dict(status="committed_input_parity_required", sha256=gate.OPENAPI_SHA))
        gate.require_codegen_binding(REVIEWED)
        for status in ("verified", "pending_final_source_binding"):
            pending = dict(REVIEWED, openapi_binding=dict(status=status, sha256=gate.OPENAPI_SHA))
            with mock.patch.object(gate, "hosted_identity", return_value=(Path("owned"), "a" * 40)), \
                    mock.patch.object(gate, "clean_head"), mock.patch.object(gate, "reviewed_inventory", return_value=pending), \
                    mock.patch.object(gate, "git") as git, mock.patch.object(gate.subprocess, "Popen") as spawn:
                with self.assertRaisesRegex(ValueError, "binding is pending"):
                    gate.preflight()
                git.assert_not_called()
                spawn.assert_not_called()
        with mock.patch.object(gate, "hosted_identity", return_value=(Path("owned"), "a" * 40)), \
                mock.patch.object(gate, "clean_head"), mock.patch.object(gate, "reviewed_inventory", return_value=REVIEWED), \
                mock.patch.object(gate, "git", side_effect=ValueError("Unreconciled source ancestry")) as git:
            with self.assertRaisesRegex(ValueError, "Unreconciled source ancestry"):
                gate.preflight()
            git.assert_called_once_with(Path("owned") / "controls", "merge-base", "--is-ancestor", gate.SOURCE_SHA, "a" * 40)

    def test_prepared_readback_rejects_forged_success_and_never_calls_GitHub(self):
        pending = dict(REVIEWED, openapi_binding=dict(status="pending_source_freeze", sha256=gate.OPENAPI_SHA))
        with mock.patch.object(gate, "command") as command, \
                mock.patch.object(gate, "reviewed_inventory", return_value=pending):
            with self.assertRaises(ValueError):
                gate.readback(SimpleNamespace())
            command.assert_not_called()
        run, artifact, payload, args = self.artifact(provenance_changes=dict(openapi_sha256=None))
        with mock.patch.object(gate, "OPENAPI_SHA", None):
            with self.assertRaisesRegex(ValueError, "Exact committed OpenAPI/source binding is pending"):
                gate.validate_readback(run, artifact, payload, **args)

    def test_credentials_each_exact_group_rejects_missing_duplicate_skip_or_count_only(self):
        for stage in ("credentials_unit", "credentials_pg", "credentials_migration", "real_auth"):
            text = self.log(stage)
            self.assertEqual(gate.verify_test_log(stage, text, REVIEWED)["passed"], len(REVIEWED["groups"][stage]))
            first = text.splitlines()[0]
            for bad in (text.replace(first + "\n", "", 1), text + first + "\n",
                        text.replace(" ... ok", " ... ignored", 1), text.replace("0 failed", "1 failed"),
                        text.replace("0 ignored", "1 ignored"), text.splitlines()[-1],
                        text + "PostgreSQL tests skipped\n"):
                with self.subTest(stage=stage), self.assertRaises(ValueError):
                    gate.verify_test_log(stage, bad, REVIEWED)

    def test_migration_smoke_preserves_surviving_timestamps_at_credential_down_and_reapply(self):
        expected = gate.expected_migration_receipt(REVIEWED)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for stage in ("down_one", "reapply"):
                self.migration_files(root, expected)
                path = root / ("migration-" + stage + "-ledger.tsv")
                path.write_text(path.read_text().replace("\t1000", "\t1001", 1), newline="\n")
                with self.subTest(stage=stage), self.assertRaisesRegex(ValueError, "Surviving migration applied-at history changed"):
                    gate.verify_migration_snapshots(root, REVIEWED)
            self.migration_files(root, expected)
            (root / "migration-up-ledger.tsv").write_text("invalid\t1000\n", newline="\n")
            with self.assertRaisesRegex(ValueError, "ledger shape drift"):
                gate.verify_migration_snapshots(root, REVIEWED)

    def workflow(self):
        return yaml.load((ROOT / gate.WORKFLOW).read_text(), Loader=yaml.BaseLoader)

    def test_workflow_summary_source_matches_checkout_and_helper_pin(self):
        steps = self.workflow()["jobs"]["backend"]["steps"]
        checkout = next(step for step in steps if step.get("with", {}).get("path") == "fleet-control")
        summary = next(step["run"] for step in steps if "GITHUB_STEP_SUMMARY" in step.get("run", ""))
        pins = re.findall(r"printf 'Source: `%s`\\n\\n' ([0-9a-f]{40})", summary)
        self.assertEqual(pins, [gate.SOURCE_SHA])
        self.assertEqual(checkout["with"]["ref"], gate.SOURCE_SHA)

    def test_exact_push_only_branch_permissions_and_one_bounded_job(self):
        flow = self.workflow()
        self.assertEqual(flow["on"], {"push": {"branches": [gate.BRANCH]}})
        self.assertEqual(flow["permissions"], {"contents": "read", "actions": "read"})
        self.assertEqual(flow["concurrency"]["cancel-in-progress"], "false")
        self.assertEqual(set(flow["jobs"]), {"backend"})
        job = flow["jobs"]["backend"]
        self.assertEqual(job["runs-on"], "ubuntu-24.04")
        self.assertEqual(job["container"], dict(image="public.ecr.aws/docker/library/rust@sha256:af306cfa71d987911a781c37b59d7d67d934f49684058f96cf72079c3626bfe0", options="--cpus 2 --memory 4g"))
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
        self.assertEqual(service["image"], "public.ecr.aws/docker/library/postgres@sha256:ef257d85f76e48da1c64832459b59fcaba1a4dac97bf5d7450c77753542eee94")
        self.assertEqual(service["env"], dict(POSTGRES_USER="fleet_test", POSTGRES_DB="fleet_foundation_test",
                                               POSTGRES_HOST_AUTH_METHOD="trust"))
        self.assertNotIn("ports", service)
        urls = gate.database_environment()
        self.assertEqual(urls["FLEET_REAL_AUTH_TEST_DATABASE_URL"], "postgres://fleet_test@postgres:5432/fleet_real_auth_test")
        self.assertEqual(len(urls), 9)
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
        self.assertEqual({step["ref"] for step in private}, {gate.BASE_SHA, gate.AUTH_SHA, gate.PACKAGE_SHA})
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

    def test_all17_ignored_once_and242_default_declarations_no_integration_tail(self):
        # Keep the prior selector; exact435 adds six default and one ignored test.
        self.assertEqual(len(REVIEWED["ignored"]), 18)
        self.assertEqual(len({(x["source"], x["name"]) for x in REVIEWED["ignored"]}), 18)
        self.assertEqual(len(REVIEWED["workspace_default_declarations"]), 248)
        self.assertEqual(len(REVIEWED["groups"]["foundation"]), 61)
        self.assertEqual(REVIEWED["default_foundation_ignored"], 0)
        self.assertEqual(len(gate.GATES), 28)
        for x in REVIEWED["ignored"]:
            self.assertIn(x["name"], REVIEWED["groups"][x["gate"]])
        self.assertEqual(REVIEWED["python_contracts"], {})
        self.assertFalse(any("container" in stage or "clarification" in stage for stage in gate.GATES))
        self.assertEqual(len(REVIEWED["migration_registries"]["canonical"]), 12)
        self.assertEqual(len(REVIEWED["migration_registries"]["split"]), 15)

    def test_required_full_workspace_and_C11_commands_remain(self):
        text = (ROOT / gate.GATE).read_text()
        for command in ("cargo fmt --all -- --check", "cargo check --locked --workspace --all-targets",
                        "cargo clippy --locked --workspace --all-targets --message-format=json -- -D warnings",
                        "cargo test --locked --workspace -- --test-threads=1",
                        "cargo test --locked -p migration --lib lineage_tests -- --include-ignored --test-threads=1",
                        "cargo run --locked -p migration -- down -n 1", "cargo run --locked -p migration -- down -n 12",
                        "cargo build --locked -p auth-server --bin auth-server",
                        "cargo test --locked -p infra --test pm_credentials_real_auth -- --ignored --test-threads=1",
                        "cmp ../openapi/openapi.json ${QA_OUTPUT}/openapi.json"):
            self.assertIn(command, text)
        self.assertEqual(len(REVIEWED["groups"]["lineage10"]), 10)
        self.assertEqual(sum(x["gate"] == "lineage10" for x in REVIEWED["ignored"]), 9)
        self.assertIn("CARGO_BUILD_JOBS=1", text)
        self.assertNotIn("QA_UTILITY_CHECKOUT", text)

    def test_standard_six_source_jobs_preserved_backend_and_minimum_commands_covered(self):
        source = yaml.load(self.source_blob(".github/workflows/ci.yml"), Loader=yaml.BaseLoader)
        self.assertEqual(set(source["jobs"]), {"containers", "docs", "backend", "real-base-auth", "frontend", "minimum-rust"})
        self.assertEqual(source, yaml.load((ROOT / ".github/workflows/ci.yml").read_bytes(), Loader=yaml.BaseLoader))
        shell = (ROOT / gate.GATE).read_text()
        for name in ("backend", "minimum-rust"):
            for step in source["jobs"][name]["steps"]:
                command = step.get("run", "")
                if command.startswith(("cargo fmt", "cargo check", "cargo clippy", "cargo test --locked --workspace")):
                    self.assertIn(command.replace(" -- -D warnings", " --message-format=json -- -D warnings"), shell)
        self.assertIn("frontend, native runtime and SDLC acceptance remain separate", self.workflow()["jobs"]["backend"]["steps"][-1]["run"])

    def test_exact28_shell_stage_order_and_C11_migration_registries_match_source(self):
        stages = []
        for line in (ROOT / gate.GATE).read_text().splitlines():
            match = re.match(r"^(?:stage=|run_tests |run_compiler )(\w+)(?:$| )", line.lstrip())
            if match:
                stages.append(match.group(1))
            elif line.startswith("for entry in "):
                stages.extend(line.removeprefix("for entry in ").split(";")[0].split())
        self.assertEqual(tuple(stages), gate.GATES)
        source = self.source_blob("backend/migration/src/lib.rs").decode()
        common = source.split("fn common_migrations()", 1)[1].split("struct CanonicalMigrator;", 1)[0]
        canonical = source.split("impl MigratorTrait for CanonicalMigrator", 1)[1].split("struct LegacyMigrator;", 1)[0]
        split = source.split("impl MigratorTrait for LegacyMigrator", 1)[1]
        names = lambda text: re.findall(r"Box::new\((\w+)::Migration\)", text)
        self.assertEqual(names(common) + names(canonical), REVIEWED["migration_registries"]["canonical"])
        self.assertEqual(names(common) + names(split), REVIEWED["migration_registries"]["split"])

    def test_socket_path_budget_refuses_before_any_owned_creation_or_child(self):
        with tempfile.TemporaryDirectory() as temporary:
            parent = Path(temporary).resolve()
            self.assertFalse((parent / "fc11").exists())
            with self.assertRaisesRegex(ValueError, "socket fixture budget"):
                gate.scratch_root(parent / ("x" * 128))
            self.assertFalse((parent / "fc11").exists())
            with mock.patch.object(gate, "preflight", return_value=(Path("owned"), ROOT, "a" * 40)), \
                    mock.patch.object(gate, "clean_head"), mock.patch.object(gate, "command", return_value=b"rustc 1.88.0"), \
                    mock.patch.object(Path, "read_text", return_value=gate.BASE_SHA), \
                    mock.patch.object(gate, "reviewed_inventory", return_value=REVIEWED), \
                    mock.patch.object(gate, "qualify_package"), mock.patch.object(gate, "resource_guard"), \
                    mock.patch.dict(os.environ, RUNNER_TEMP=temporary), \
                    mock.patch.object(gate, "scratch_root", side_effect=ValueError("Owned TMPDIR exceeds source Unix socket fixture budget")), \
                    mock.patch.object(Path, "mkdir") as mkdir, mock.patch.object(gate.subprocess, "Popen") as spawn:
                with self.assertRaisesRegex(ValueError, "socket fixture budget"):
                    gate.execute()
                mkdir.assert_not_called()
                spawn.assert_not_called()

    @unittest.skipUnless(sys.platform == "linux", "Linux AF_UNIX filesystem path budget")
    def test_linux_actual_source_socket_fixture_fits_short_root_and_rejects_long_root(self):
        source = self.source_blob("backend/infra/src/effective_configuration.rs").decode()
        self.assertIn('format!("fleet-skill-readback-{}", uuid::Uuid::new_v4())', source)
        self.assertIn('root.join("agent1/config/skills")', source)
        parent = Path("/tmp") / os.urandom(3).hex()
        parent.mkdir(mode=0o700, exist_ok=False)
        try:
            root = gate.scratch_root(parent)
            for root, succeeds in ((root, True), (parent / ("x" * 64) / "fc11", False)):
                path = root / "tmp" / ("fleet-skill-readback-" + "0" * 36) / "agent1/config/skills/socket"
                path.parent.mkdir(parents=True)
                with socket.socket(socket.AF_UNIX) as listener:
                    if succeeds:
                        self.assertLessEqual(len(os.fsencode(path)), 107)
                        listener.bind(str(path))
                    else:
                        self.assertGreater(len(os.fsencode(path)), 107)
                        with self.assertRaises(OSError):
                            listener.bind(str(path))
                        with self.assertRaises(ValueError):
                            gate.scratch_root(root.parent)
        finally:
            gate.shutil.rmtree(parent)

    def test_removed_integration_hints_are_never_public_C11_diagnostics(self):
        self.assertEqual(gate.TEST_CUSTOM_HINTS, {})
        self.assertEqual(gate.TEST_ACTIVATION_HINTS, frozenset())
        for detail in ("activation_probe_request_scope_exact", "activation_probe_private_suffix",
                       'Custom("Hermes guard is missing")', "PRIVATE_SENTINEL"):
            result = self.hint_diagnostics(detail)
            self.assertEqual(result["categories"], ["test_failure"])
            self.assertNotIn(detail, gate.canonical(result).decode())
        value = self.failure_test_value()
        for hint in ("activation_probe_request_scope_exact", "hermes_guard_missing"):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(value, categories=["test_failure", hint]))

    def log(self, stage, ignored=0):
        names = REVIEWED["groups"][stage]
        return "\n".join("test " + name + " ... ok" for name in names) + \
            f"\ntest result: ok. {len(names)} passed; 0 failed; {ignored} ignored; 0 measured; 0 filtered out;\n"

    def test_exact_focused_log_requires_each_name_and_count_no_zero(self):
        text = self.log("credentials_pg")
        gate.verify_test_log("credentials_pg", text, REVIEWED)
        variants = [text.replace(" ... ok", " ... ignored", 1), text.replace("15 passed", "0 passed"),
                    text + "test unexpected ... ok\n", text.replace("0 ignored", "1 ignored"),
                    text + "PostgreSQL tests skipped\n", text + "test result: FAILED.\n", ""]
        for bad in variants:
            with self.assertRaises(ValueError):
                gate.verify_test_log("credentials_pg", bad, REVIEWED)

    def test_foundation_exact56_passed_zero_ignored_never_lowered(self):
        gate.verify_test_log("foundation", self.log("foundation", 0), REVIEWED)
        with self.assertRaises(ValueError):
            gate.verify_test_log("foundation", self.log("foundation", 1), REVIEWED)

    def test_compiler_inventory_requires_exact17_ignored_and242_defaults(self):
        names = [x["name"] for x in REVIEWED["ignored"]]
        ignored = "\n".join(name + ": test" for name in names)
        defaults = "\n".join(x["name"] + ": test" for x in REVIEWED["workspace_default_declarations"])
        ordinary = ignored + "\n" + defaults
        result = gate.verify_runtime_inventory(ordinary, ignored, REVIEWED)
        self.assertEqual(result["listed_default_count"], 248)
        for bad in ("", ignored + "\nextra: test", ignored + "\n" + names[0] + ": test", "\n".join(ignored.splitlines()[1:])):
            with self.assertRaises(ValueError):
                gate.verify_runtime_inventory(ordinary, bad, REVIEWED)
        for bad in (ordinary + "\nextra: test", ignored, ordinary.replace(defaults.splitlines()[0], "wrong: test", 1)):
            with self.assertRaises(ValueError):
                gate.verify_runtime_inventory(bad, ignored, REVIEWED)

    def test_workspace_actual_cases_match_compiler_list_not_static_count(self):
        listing = "\n".join(x["name"] + ": test" for x in REVIEWED["ignored"]) + "\nfirst: test\nsecond: test\n"
        text = "test first ... ok\ntest second ... ok\ntest result: ok. 2 passed; 0 failed; 18 ignored;\n"
        result = gate.verify_test_log("workspace", text, REVIEWED, listing)
        self.assertEqual(result["passed"], 2)
        for bad in (text.replace("test second ... ok\n", ""), text.replace("18 ignored", "162 ignored"), ""):
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

    def test_export_reads_canonical_blobs_despite_crlf_archive_attributes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = root / "source"
            repo.mkdir()
            hooks = root / "empty-hooks"
            hooks.mkdir()
            def git(*args):
                return subprocess.run(["git", "-c", "core.autocrlf=false", "-c", "commit.gpgsign=false",
                    "-c", "user.name=Owned Source Fixture", "-c", "user.email=fixture@example.invalid",
                    "-c", "core.hooksPath=" + str(hooks), "-C", str(repo), *args],
                    capture_output=True, timeout=30, check=True).stdout
            git("init", "--quiet")
            (repo / "sql").mkdir()
            original = b"CREATE TABLE owned_fixture(id integer);\n"
            (repo / "sql/migration.sql").write_bytes(original)
            (repo / ".gitattributes").write_bytes(b"*.sql text eol=crlf\n")
            git("add", ".gitattributes", "sql/migration.sql")
            git("commit", "--quiet", "-m", "owned canonical byte fixture")
            revision = git("rev-parse", "HEAD").decode().strip()
            legacy = git("archive", "--format=tar", revision, "--", "sql")
            with gate.tarfile.open(fileobj=io.BytesIO(legacy)) as archive:
                altered = archive.extractfile("sql/migration.sql").read()
            self.assertNotEqual(altered, original)
            self.assertEqual(altered, original.replace(b"\n", b"\r\n"))
            destination = root / "exact-blobs"
            gate.export(repo, revision, destination, ("sql",))
            self.assertEqual((destination / "sql/migration.sql").read_bytes(), original)
            self.assertEqual(gate.inventory(destination), {"sql/migration.sql": gate.digest(original)})

    def test_export_rejects_symlink_submodule_duplicate_and_missing_root(self):
        oid = b"a" * 40
        for tree, roots in ((b"120000 blob " + oid + b"\tsql/x\0", ("sql",)),
                            (b"160000 commit " + oid + b"\tsql/x\0", ("sql",)),
                            ((b"100644 blob " + oid + b"\tsql/x\0") * 2, ("sql",)),
                            (b"100644 blob " + oid + b"\tsql/x\0", ("sql", "missing")),
                            (b"100644 blob " + oid + b"\t../x\0", ("sql",)),
                            (b"", ("sql",))):
            with tempfile.TemporaryDirectory() as directory, mock.patch.object(gate, "git", return_value=tree) as git:
                target = Path(directory) / "export"
                with self.assertRaises(ValueError):
                    gate.export(Path(directory), "b" * 40, target, roots)
                self.assertFalse(target.exists())
                git.assert_called_once()

    def test_export_rejects_wrong_truncated_oversized_and_extra_blob_frames(self):
        oid = b"a" * 40
        tree = b"100644 blob " + oid + b"\tsql/x\0"
        for batch in (b"b" * 40 + b" blob 1\nx\n", oid + b" missing\n", oid + b" blob 2\nx\n",
                      oid + b" blob 134217729\n", oid + b" blob 1\nx?", oid + b" blob 1\nx\nextra"):
            with tempfile.TemporaryDirectory() as directory, mock.patch.object(gate, "git", side_effect=[tree, batch]):
                with self.assertRaises(ValueError):
                    gate.export(Path(directory), "b" * 40, Path(directory) / "export", ("sql",))

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
            root = parent / "fc11"
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

    def test_failure_kind_never_exposes_exception_arguments_or_private_context(self):
        sentinel = "PRIVATE_SENTINEL_TOKEN_AND_SOURCE"
        for error, category in ((ValueError(sentinel), "validation"), (OSError(sentinel), "io"),
                                (TimeoutError(sentinel), "timeout"),
                                (subprocess.TimeoutExpired(sentinel, 12, output=sentinel), "timeout"),
                                (gate.tarfile.ReadError(sentinel), "archive"), (RuntimeError(sentinel), "unexpected")):
            actual = gate.failure_kind(error)
            self.assertEqual(actual, dict(category=category, exit_code=None))
            self.assertNotIn(sentinel, json.dumps(actual))

    def test_command_failure_retains_only_bounded_numeric_exit_code(self):
        for code in (1, 22, 128, -9):
            self.assertEqual(gate.failure_kind(gate.CommandFailed(code)), dict(category="command", exit_code=code))
        for code in (True, "PRIVATE_SENTINEL", 256, -256, None):
            self.assertEqual(gate.failure_kind(gate.CommandFailed(code)), dict(category="command", exit_code=None))

    def test_preflight_phase_diagnostics_use_only_static_literals(self):
        import ast
        module = ast.parse((ROOT / gate.HELPER).read_text())
        function = next(node for node in module.body if isinstance(node, ast.FunctionDef) and node.name == "execute")
        phases = set()
        for node in ast.walk(function):
            if isinstance(node, ast.Assign):
                if any(isinstance(target, ast.Name) and target.id == "phase" for target in node.targets):
                    self.assertIsInstance(node.value, ast.Constant)
                    self.assertIsInstance(node.value.value, str)
                    phases.add(node.value.value)
        self.assertEqual(phases, {"source_export_sdk", "source_export_auth", "source_inventory", "source_declarations",
            "expectations", "swagger_download", "swagger_hash", "postgres_qualification", "postgres_initialization",
            "gate_execution", "gate_receipts"})

    def test_final_failure_metadata_survives_compiler_artifact_and_cleanup_failure(self):
        import ast
        from contextlib import redirect_stdout
        module = ast.parse((ROOT / gate.HELPER).read_text())
        function = next(node for node in module.body if isinstance(node, ast.FunctionDef) and node.name == "execute")
        start = next(index for index, node in enumerate(function.body) if isinstance(node, ast.If)
                     and "compiler_failure is not None" in ast.unparse(node.test))
        function.name, function.body = "tail_probe", function.body[start:]
        module = ast.fix_missing_locations(ast.Module(body=[function], type_ignores=[]))
        for compiler_failure in (None, {"synthetic_safe_field": True}):
            for phase, category in (("gate_receipts", "validation"), ("cleanup", "io")):
                with self.subTest(compiler_failure=compiler_failure, phase=phase):
                    metadata = dict(category=category, exit_code=None)
                    environment = dict(vars(gate), success=False, failure=metadata, compiler_failure=compiler_failure,
                        identity=dict(workflow_sha="a" * 40, run_id=1, run_attempt=1), workflow_sha="a" * 40,
                        before={}, provenance={"control_sha256": {}}, compiler_stage="check", failed_stage=phase,
                        code=101, cleanup=dict(scratch=phase != "cleanup", synthetic_databases=True), phase=phase,
                        temporary=mock.MagicMock(), validate_failure_evidence=mock.Mock(), check_budget=mock.Mock())
                    exec(compile(module, "<execute-tail-regression>", "exec"), environment)
                    output = io.StringIO()
                    with redirect_stdout(output):
                        self.assertEqual(environment["tail_probe"](), 101)
                    records = [json.loads(line) for line in output.getvalue().splitlines()]
                    self.assertEqual(records[-1]["failure"], metadata)
                    self.assertEqual(records[-1]["control_phase"], phase)
                    self.assertFalse(records[-1]["all_quality_gate"])
                    if compiler_failure is not None:
                        self.assertEqual(records[0]["kind"], "safe_compiler_failure")
                        self.assertEqual(environment["validate_failure_evidence"].call_count, 1)
                    else:
                        self.assertEqual(len(records), 1)

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
                    package_sha=gate.PACKAGE_SHA, package_tree=gate.PACKAGE_TREE,
                    package_inventory_sha256=gate.PACKAGE_INVENTORY_SHA,
                    source_inventory_sha256=gate.SOURCE_INVENTORY_SHA,
                    control_sha256={name: gate.digest((ROOT / name).read_bytes()) for name in sorted(gate.WRITE_SET)},
                    stage="check", gate_failed_stage="check", gate_exit_code=101, command_exit_code=101,
                    **self.diagnostics(self.compiler_line()), cleanup=dict(scratch=True, synthetic_databases=True),
                    backend_quality_gate=False, all_quality_gate=False, sdlc_acceptance=False)

    def validate_failure(self, value):
        return gate.validate_failure_evidence(value, workflow_sha="a" * 40, run_id=123, attempt=1)

    def inventory_texts(self):
        defaults = ["tests::" + item["name"] for item in REVIEWED["workspace_default_declarations"]]
        ignored = [item["name"] for item in REVIEWED["ignored"]]
        return ("".join(name + ": test\n" for name in defaults + ignored),
                "".join(name + ": test\n" for name in ignored))

    def inventory_result(self, ordinary, ignored):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "private").mkdir()
            for name, text in (("workspace-list.log", ordinary), ("ignored-list.log", ignored)):
                if text is not None:
                    (root / "private" / name).write_bytes(text.encode() if isinstance(text, str) else text)
            return gate.inventory_failure_logs(root, REVIEWED)

    def inventory_value(self, result):
        value = self.failure_value()
        value.update(kind="safe_inventory_failure", stage="runtime_inventory", gate_failed_stage="runtime_inventory",
                     gate_exit_code=1, **result)
        return value

    def test_inventory_matching_lists_are_failed_stage_evidence_not_mismatch_or_pass(self):
        ordinary, ignored = self.inventory_texts()
        gate.verify_runtime_inventory(ordinary, ignored, REVIEWED)
        result = self.inventory_result(ordinary, ignored)
        self.assertEqual(result["inventory"]["reason"], "listing_matches")
        self.assertEqual(result["inventory"]["observed"], dict(ordinary=266, default=248, ignored=18))
        self.assertEqual(result["categories"], ["inventory_unavailable"])
        self.assertEqual(result["inventory"]["samples"], [])
        value = self.inventory_value(result)
        self.validate_failure(value)
        self.assertTrue(all(value[key] is False for key in ("backend_quality_gate", "all_quality_gate", "sdlc_acceptance")))

    def test_inventory_missing_extra_and_duplicate_counts_preserve_strict_verifier(self):
        ordinary, ignored = self.inventory_texts()
        first = ordinary.splitlines(keepends=True)[0]
        known = ignored.splitlines(keepends=True)[0]
        vectors = ((ordinary.replace(first, "", 1), ignored, "default", "missing", 1),
                   (ordinary + first, ignored, "default", "extra", 1),
                   (ordinary, ignored.replace(known, "", 1), "ignored", "missing", 1),
                   (ordinary, ignored + known, "ignored", "extra", 1),
                   (ordinary + "PRIVATE_SENTINEL: test\n", ignored, "default", "unallowlisted_extra", 1))
        for actual, actual_ignored, scope, field, count in vectors:
            with self.subTest(scope=scope, field=field):
                with self.assertRaises(ValueError):
                    gate.verify_runtime_inventory(actual, actual_ignored, REVIEWED)
                result = self.inventory_result(actual, actual_ignored)
                self.assertEqual(result["inventory"]["reason"], "listing_mismatch")
                self.assertEqual(result["inventory"]["differences"][scope][field], count)
                self.assertEqual(result["categories"], ["inventory_mismatch"])
                self.validate_failure(self.inventory_value(result))
                self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_inventory_private_lines_foreign_names_paths_and_hashes_never_emitted(self):
        ordinary, ignored = self.inventory_texts()
        known = REVIEWED["ignored"][0]["name"]
        private = "PRIVATE_SENTINEL_" + "a" * 64
        ordinary += private + ": test\n/credentials/" + private + ": test\n"
        ordinary += "error: PRIVATE_SENTINEL_BODY\n" + "PRIVATE_SENTINEL: test suffix\n"
        ignored += "foreign::" + known + ": test\n" + known + "_PRIVATE_SENTINEL: test\n"
        result = self.inventory_result(ordinary, ignored)
        self.assertEqual(result["inventory"]["differences"]["default"]["unallowlisted_extra"], 2)
        self.assertEqual(result["inventory"]["differences"]["ignored"]["unallowlisted_extra"], 2)
        self.assertEqual(result["inventory"]["samples"], [])
        self.validate_failure(self.inventory_value(result))
        self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
        self.assertNotIn("a" * 64, gate.canonical(result).decode())

    def test_inventory_samples_bounded_by_eight_canonical_identities(self):
        _, ignored = self.inventory_texts()
        result = self.inventory_result(ignored, "")
        inventory = result["inventory"]
        self.assertEqual(inventory["differences"]["default"]["missing"], 248)
        self.assertEqual(inventory["differences"]["ignored"]["missing"], 18)
        self.assertEqual(len(inventory["samples"]), 8)
        self.assertTrue(inventory["samples_truncated"])
        self.validate_failure(self.inventory_value(result))
        self.assertLess(len(gate.canonical(self.inventory_value(result))), gate.FAILURE_SIZE_LIMIT)

    def test_inventory_bounded_byte_line_and_record_reads_fail_closed(self):
        ordinary, ignored = self.inventory_texts()
        for constant, bound in (("INVENTORY_INPUT_LIMIT", 64), ("INVENTORY_LINE_LIMIT", 32),
                                ("INVENTORY_RECORD_LIMIT", 1)):
            with self.subTest(constant=constant), mock.patch.object(gate, constant, bound):
                result = self.inventory_result(ordinary, ignored)
                self.assertEqual(result["inventory"]["reason"], "listing_limit")
                self.assertTrue(result["truncated"])
                self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
        with mock.patch.object(gate, "INVENTORY_LINE_LIMIT", 16):
            result = self.inventory_result(b"PRIVATE_SENTINEL" * 100, ignored)
        self.validate_failure(self.inventory_value(result))
        self.assertEqual(result["inventory"]["observed"]["ordinary"], 0)

    def test_inventory_missing_invalid_utf8_and_nonregular_files_are_unavailable(self):
        ordinary, ignored = self.inventory_texts()
        for actual in (None, b"\xffPRIVATE_SENTINEL"):
            result = self.inventory_result(actual, ignored)
            self.assertEqual(result["inventory"]["reason"], "listing_unavailable")
            self.assertEqual(result["inventory"]["input_status"]["ordinary"], "unavailable")
            self.validate_failure(self.inventory_value(result))
            self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
        with tempfile.TemporaryDirectory() as directory:
            self.assertEqual(gate.inventory_listing(Path(directory)), (Counter(), "unavailable"))

    @unittest.skipUnless(sys.platform == "linux", "Linux nofollow/nonblocking special-file proof")
    def test_inventory_actual_fifo_and_symlink_fail_without_blocking_or_reading_target(self):
        import time
        ordinary, ignored = self.inventory_texts()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "private").mkdir()
            (root / "private/ignored-list.log").write_text(ignored)
            path = root / "private/workspace-list.log"
            os.mkfifo(path)
            started = time.monotonic()
            result = gate.inventory_failure_logs(root, REVIEWED)
            self.assertLess(time.monotonic() - started, 2)
            self.validate_failure(self.inventory_value(result))
            self.assertEqual(result["inventory"]["reason"], "listing_unavailable")
            path.unlink()
            target = root / "private-target"
            target.write_text(ordinary + "PRIVATE_SENTINEL: test\n")
            path.symlink_to(target)
            result = gate.inventory_failure_logs(root, REVIEWED)
            self.assertEqual(result["inventory"]["observed"]["ordinary"], 0)
            self.validate_failure(self.inventory_value(result))
            self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_inventory_closed_schema_rejects_private_fields_names_and_noninteger_counts(self):
        ordinary, ignored = self.inventory_texts()
        result = self.inventory_result(ordinary.split("\n", 1)[1], ignored)
        valid = result["inventory"]
        self.validate_failure(self.inventory_value(result))
        mutations = [dict(valid, raw="PRIVATE_SENTINEL"), dict(valid, expected=dict(default=249, ignored=18)),
                     dict(valid, observed=dict(valid["observed"], default=True)),
                     dict(valid, observed=dict(valid["observed"], ordinary=4097)),
                     dict(valid, reason="listing_matches"), dict(valid, samples_truncated=True),
                     dict(valid, input_status=dict(ordinary="PRIVATE_SENTINEL", ignored="complete"))]
        for change in (dict(name="PRIVATE_SENTINEL"), dict(name="foreign::" + valid["samples"][0]["name"]),
                       dict(name=valid["samples"][0]["name"] + "_suffix"), dict(count=True), dict(count=0),
                       dict(count=4097), dict(raw="PRIVATE_SENTINEL"), dict(scope="private"), dict(difference="unknown")):
            mutations.append(dict(valid, samples=[dict(valid["samples"][0], **change)]))
        mutations.append(dict(valid, samples=valid["samples"] * 2))
        mutations.append(dict(valid, samples=[]))
        mutations.append(dict(valid, differences=dict(valid["differences"], default=dict(missing=1, extra=0, unallowlisted_extra=1))))
        mutations.append(dict(valid, input_status=dict(ordinary="unavailable", ignored="complete"), reason="listing_unavailable"))
        mutations.append(dict(valid, observed=dict(valid["observed"], ordinary=300)))
        for inventory in mutations:
            with self.subTest(inventory=inventory), self.assertRaises(ValueError):
                self.validate_failure(self.inventory_value(dict(result, inventory=inventory)))

    def test_inventory_failure_framing_rejects_wrong_kind_stage_categories_and_acceptance(self):
        value = self.inventory_value(self.inventory_result(*self.inventory_texts()))
        for change in (dict(kind="safe_test_failure"), dict(kind="safe_compiler_failure"), dict(stage="check"),
                       dict(command_exit_code=101), dict(categories=["inventory_mismatch"]), dict(truncated=True),
                       dict(diagnostics=[dict(error_code=None, file="backend/api/src/routes/sessions.rs", line=1, column=1)]),
                       dict(backend_quality_gate=True), dict(all_quality_gate=True), dict(sdlc_acceptance=True),
                       dict(source_sha="a" * 40), dict(control_sha256={}), dict(private="PRIVATE_SENTINEL")):
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.validate_failure(dict(value, **change))

    def test_inventory_authenticated_readback_retains_failure_identity_and_rejects_raw_members(self):
        value = self.inventory_value(self.inventory_result(None, None))
        run, artifact, payload, args = self.failure_artifact(value=value)
        files = gate.validate_failure_readback(run, artifact, payload, **args)
        self.assertEqual(json.loads(files[gate.FAILURE_FILE])["kind"], "safe_inventory_failure")
        for change in (dict(id=456), dict(run_attempt=2), dict(head_sha="b" * 40), dict(conclusion="success")):
            with self.subTest(change=change), self.assertRaises(ValueError):
                gate.validate_failure_readback(dict(run, **change), artifact, payload, **args)
        with self.assertRaises(ValueError):
            gate.validate_failure_readback(run, artifact, payload + b"PRIVATE_SENTINEL", **args)
        with self.assertRaises(ValueError):
            gate.validate_readback(run, artifact, payload, **args)
        run, artifact, payload, args = self.failure_artifact(extra=["workspace-list.log"], value=value)
        with self.assertRaises(ValueError):
            gate.validate_failure_readback(run, artifact, payload, **args)

    def test_inventory_readback_cli_authenticates_before_creating_output(self):
        value = self.inventory_value(self.inventory_result(None, None))
        run, artifact, payload, arguments = self.failure_artifact(value=value)
        artifact.update(id=42, size_in_bytes=len(payload))
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "readback"
            args = SimpleNamespace(mode="readback-failure", artifact_id=42, output=output, **arguments)
            with mock.patch.object(gate, "command", side_effect=[gate.canonical(run), gate.canonical(artifact), payload]), \
                    mock.patch("sys.stdout", new_callable=io.StringIO) as stdout:
                gate.readback(args)
            self.assertEqual(set(path.name for path in output.iterdir()), {gate.FAILURE_FILE})
            self.assertEqual(json.loads(stdout.getvalue())["state"], "verified_safe_inventory_failure")
            self.assertFalse(json.loads(stdout.getvalue())["backend_quality_gate"])
            output = Path(directory) / "rejected"
            args.output = output
            with mock.patch.object(gate, "command", side_effect=[gate.canonical(dict(run, head_sha="b" * 40)),
                                                                 gate.canonical(artifact), payload]), self.assertRaises(ValueError):
                gate.readback(args)
            self.assertFalse(output.exists())

    def test_inventory_capture_precedes_actual_scratch_removal_and_publishes_only_safe_failure(self):
        import ast
        import shutil
        from contextlib import redirect_stdout
        source = ast.parse((ROOT / gate.HELPER).read_text())
        original = next(node for node in source.body if isinstance(node, ast.FunctionDef) and node.name == "execute")
        cleanup_try = next(node for node in original.body if isinstance(node, ast.Try) and node.finalbody)
        cleanup_function = copy.deepcopy(original)
        cleanup_function.name = "cleanup_probe"
        cleanup_function.body = ast.parse(
            "success = False\nfailed_stage = 'runtime_inventory'\ncompiler_stage = compiler_failure = None").body + copy.deepcopy(cleanup_try.finalbody) + ast.parse(
            "return compiler_stage, compiler_failure, cleanup, failed_stage").body
        tail_function = copy.deepcopy(original)
        start = next(index for index, node in enumerate(original.body) if isinstance(node, ast.If)
                     and "compiler_failure is not None" in ast.unparse(node.test))
        tail_function.name, tail_function.body = "tail_probe", copy.deepcopy(original.body[start:])
        module = ast.fix_missing_locations(ast.Module(body=[cleanup_function, tail_function], type_ignores=[]))
        with tempfile.TemporaryDirectory() as directory:
            temporary = Path(directory)
            root = temporary / "fc11"
            (root / "private").mkdir(parents=True)
            (root / "private/workspace-list.log").write_text("PRIVATE_SENTINEL: test\n")
            (root / "private/ignored-list.log").write_text("")
            identity = dict(workflow_sha="a" * 40, run_id=123, run_attempt=1)
            environment = dict(vars(gate), BUDGET=None, success=False, process=None, root=root, temporary=temporary,
                reviewed=REVIEWED, compiler_stage=None, compiler_failure=None, failed_stage="runtime_inventory",
                owned_dbs=[], cleanup={}, identity=identity, stop_owned_group=mock.Mock(),
                remove_scratch=mock.Mock(side_effect=lambda *args: shutil.rmtree(root)),
                check_budget=mock.Mock(), workflow_sha="a" * 40, before=REVIEWED["compiled_source_sha256"],
                provenance=dict(control_sha256=self.failure_value()["control_sha256"]),
                code=1, phase="gate_receipts", failure=dict(category="validation", exit_code=None))
            exec(compile(module, "<inventory-cleanup-regression>", "exec"), environment)
            stage, result, cleanup, failed = environment["cleanup_probe"]()
            self.assertFalse(root.exists())
            self.assertEqual(stage, "runtime_inventory")
            self.assertEqual(result["inventory"]["differences"]["default"]["unallowlisted_extra"], 1)
            environment.update(compiler_stage=stage, compiler_failure=result, cleanup=cleanup, failed_stage=failed)
            output = io.StringIO()
            with redirect_stdout(output):
                self.assertEqual(environment["tail_probe"](), 1)
            data = (temporary / "fleet-backend-failure-evidence" / gate.FAILURE_FILE).read_bytes()
            self.validate_failure(json.loads(data))
            self.assertEqual(json.loads(data)["kind"], "safe_inventory_failure")
            self.assertNotIn("PRIVATE_SENTINEL", output.getvalue())
            self.assertNotIn("PRIVATE_SENTINEL", data.decode())
            self.assertEqual(json.loads(output.getvalue().splitlines()[-1])["state"], "backend_quality_gate_failed")

    def test_inventory_diagnostics_do_not_change_frozen_verifier_counts_gates_or_inputs(self):
        import ast
        predecessor = self.source_blob(gate.HELPER, "42447728419b69011efbd69f9f7ff5935c1e271f").decode()
        current = (ROOT / gate.HELPER).read_text()
        predecessor = predecessor.replace("credentials_unit=7, credentials_pg=10, foundation=56",
                                          "credentials_unit=8, credentials_pg=15, foundation=61").replace("real_auth=1)", "real_auth=2)")
        for name in ("verify_runtime_inventory", "verify_log_cli", "reviewed_inventory", "preflight"):
            definitions = [next(node for node in ast.parse(text).body if isinstance(node, ast.FunctionDef) and node.name == name)
                           for text in (predecessor, current)]
            self.assertEqual(ast.dump(definitions[0], include_attributes=False), ast.dump(definitions[1], include_attributes=False))
        constants = lambda text: {node.targets[0].id: ast.dump(node, include_attributes=False)
            for node in ast.parse(text).body if isinstance(node, ast.Assign) and len(node.targets) == 1
            and isinstance(node.targets[0], ast.Name) and node.targets[0].id.isupper()}
        old, new = constants(predecessor), constants(current)
        rebound = {"SOURCE_SHA", "SOURCE_TREE", "SOURCE_INVENTORY_SHA", "DEFAULT_COUNT", "IGNORED_COUNT"}
        self.assertTrue(all(new.get(name) == value for name, value in old.items() if name not in rebound))
        self.assertEqual((ROOT / gate.INIT).read_bytes(), self.source_blob(gate.INIT, "42447728419b69011efbd69f9f7ff5935c1e271f"))
        prior_shell = self.source_blob(gate.GATE, "42447728419b69011efbd69f9f7ff5935c1e271f").decode()
        self.assertEqual((ROOT / gate.GATE).read_text(), prior_shell.replace(
            "1 passed; 0 failed; 0 ignored;' ${QA_OUTPUT}/real_auth.log",
            "2 passed; 0 failed; 0 ignored;' ${QA_OUTPUT}/real_auth.log"))
        prior_workflow = yaml.load(self.source_blob(gate.WORKFLOW, "42447728419b69011efbd69f9f7ff5935c1e271f").decode().replace(
            "994f29d93c6c35d1fc43329b29175cd6a4b0ad87", gate.SOURCE_SHA).replace(
            "Exact isolated C11 product source; no integration tail.",
            "Exact C11 regressions with the test-only iterator correction."), Loader=yaml.BaseLoader)
        current_workflow = self.workflow()
        self.assertEqual(current_workflow, prior_workflow)
        self.assertEqual(len(gate.GATES), 28)
        self.assertEqual((gate.DEFAULT_COUNT, gate.IGNORED_COUNT), (248, 18))
        self.assertEqual(len(REVIEWED["compiled_source_sha256"]), 309)
        self.assertEqual(gate.SOURCE_SHA, "0e494309d2f94e11598e6e7562c57de423d044e9")
        self.assertEqual(gate.SOURCE_TREE, "934f69f7f8d667a242531cf122917d8e74ca5cf9")
        self.assertEqual(gate.SOURCE_INVENTORY_SHA, "2df727b43f468da1c0919fd6a004dd3836d6a548bbabeef807a608c8435de9c8")

    def test_ci_only_product_successor_preserves_exact_compiled_and_declaration_inventory(self):
        prior = json.loads(self.source_blob(gate.INVENTORY, "bdd1da4e97497f13d0cfd17d1e28c087b52c2051"))
        prior.update(source_commit=gate.SOURCE_SHA, source_tree=gate.SOURCE_TREE)
        for path in ("backend/shared/src/id.rs", "docs/TESTING.md", "backend/infra/src/pm_credentials/coordinator.rs"):
            prior["compiled_source_sha256"]["fleet-control/" + path] = gate.digest(self.source_blob(path))
        prior["rust_source_sha256"]["backend/shared/src/id.rs"] = gate.digest(self.source_blob("backend/shared/src/id.rs"))
        prior["rust_source_sha256"]["backend/infra/src/pm_credentials/coordinator.rs"] = gate.digest(self.source_blob("backend/infra/src/pm_credentials/coordinator.rs"))
        self.assertEqual(REVIEWED, prior)
        changed = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "diff", "--name-only",
            "4358dea9d62f6d26cafcd6da8de7b533ac65fe56", "2a1b20b2db022391025540074f53034270eca3a2"],
            capture_output=True, check=True, timeout=30).stdout.decode().splitlines()
        self.assertEqual(changed, [".github/workflows/ci.yml"])
        self.assertEqual(self.source_blob(".github/workflows/ci.yml"), (ROOT / ".github/workflows/ci.yml").read_bytes())

    def test_orphan_id_declaration_is_retained_and_registered_in_final_product_source(self):
        self.assertRegex(self.source_blob("backend/shared/src/id.rs").decode(), r"#\[cfg\(test\)\]\s+mod tests;")
        self.assertEqual(self.source_blob("backend/shared/src/id/tests.rs"),
                         self.source_blob("backend/shared/src/id/tests.rs", "994f29d93c6c35d1fc43329b29175cd6a4b0ad87"))
        records = [item for item in REVIEWED["workspace_default_declarations"] if item["name"] == "new_ids_are_plain_uuids"]
        self.assertEqual(len(records), 1)
        self.assertEqual(records[0]["source"], "backend/shared/src/id/tests.rs")
        self.assertFalse(records[0]["ignored"])

    def test_clippy_successor_changes_only_scope_variant_iteration_and_one_compiled_blob(self):
        previous_source = "3d1a10836ba1d2d6d136a3656201a4831e216118"
        path = "backend/infra/src/pm_credentials/coordinator.rs"
        previous = self.source_blob(path, previous_source)
        self.assertEqual(self.source_blob(path), previous.replace(
            b"for index in 0..scopes.len() {", b"for (index, scope) in scopes.iter().enumerate() {").replace(
            b".push(json!(scopes[index]));", b".push(json!(scope));"))
        changed = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "diff", "--name-only",
            previous_source, gate.SOURCE_SHA], capture_output=True, check=True, timeout=30).stdout.decode().splitlines()
        self.assertEqual(changed, [path])
        prior = json.loads(self.source_blob(gate.INVENTORY, "cfe7805dfeff3c37558044b35c1454c27662bc8c"))
        prior.update(source_commit=gate.SOURCE_SHA, source_tree=gate.SOURCE_TREE)
        prior["compiled_source_sha256"]["fleet-control/" + path] = gate.digest(self.source_blob(path))
        prior["rust_source_sha256"][path] = gate.digest(self.source_blob(path))
        self.assertEqual(REVIEWED, prior)

    def test_regression_source_preserves_every_prior_declaration_and_adds_exact_seven(self):
        prior = json.loads(self.source_blob(gate.INVENTORY, "42447728419b69011efbd69f9f7ff5935c1e271f"))
        defaults = lambda value: {(item["source"], item["name"]) for item in value["workspace_default_declarations"]}
        ignored = lambda value: {(item["source"], item["name"]) for item in value["ignored"]}
        self.assertEqual(len(defaults(prior)), 242)
        self.assertEqual(len(ignored(prior)), 17)
        self.assertLessEqual(defaults(prior), defaults(REVIEWED))
        self.assertLessEqual(ignored(prior), ignored(REVIEWED))
        pg_names = {
            "pm_credentials_pg_changed_valid_child_uuid_replay_conflicts_without_context_or_mutation",
            "pm_credentials_pg_changed_valid_child_expiry_replay_conflicts_without_context_or_mutation",
            "pm_credentials_pg_parent_principal_mismatch_prevents_first_post",
            "pm_credentials_pg_parent_principal_mismatch_retains_ack_without_another_post",
            "pm_credentials_pg_child_principal_mismatch_retains_ack_without_usable_credential"}
        unit_name = "introspection_rejects_wrong_subject_and_non_exact_parent_or_child_scopes"
        real_name = "real_base_expired_children_replay_without_minting_and_are_rejected_by_fleet"
        additions = {("backend/infra/tests/support/pm_credential_creation.rs", name) for name in pg_names}
        additions.add(("backend/infra/src/pm_credentials/coordinator.rs", unit_name))
        self.assertEqual(defaults(REVIEWED) - defaults(prior), additions)
        self.assertEqual(ignored(REVIEWED) - ignored(prior), {("backend/infra/tests/pm_credentials_real_auth.rs", real_name)})
        expected_groups = {"credentials_unit": 8, "credentials_pg": 15, "foundation": 61, "real_auth": 2}
        for stage, names in prior["groups"].items():
            self.assertLessEqual(set(names), set(REVIEWED["groups"][stage]))
            self.assertEqual(len(REVIEWED["groups"][stage]), expected_groups.get(stage, len(names)))
        for key in ("migration_registries", "openapi_binding", "package_input", "python_contracts"):
            self.assertEqual(REVIEWED[key], prior[key])
        self.assertEqual(set(REVIEWED["compiled_source_sha256"]), set(prior["compiled_source_sha256"]))
        changed = {path for path, sha in REVIEWED["compiled_source_sha256"].items()
                   if sha != prior["compiled_source_sha256"][path]}
        self.assertEqual(changed, {"fleet-control/" + path for path, _ in additions} |
                         {"fleet-control/backend/infra/tests/pm_credentials_real_auth.rs",
                          "fleet-control/backend/shared/src/id.rs", "fleet-control/docs/TESTING.md"})

    def test_regression_successor_refuses_old_inventory_and_reports_only_exact_missing_additions(self):
        prior = json.loads(self.source_blob(gate.INVENTORY, "42447728419b69011efbd69f9f7ff5935c1e271f"))
        old_ignored = "".join(item["name"] + ": test\n" for item in prior["ignored"])
        old_defaults = "".join("tests::" + item["name"] + ": test\n" for item in prior["workspace_default_declarations"])
        with self.assertRaises(ValueError):
            gate.verify_runtime_inventory(old_defaults + old_ignored, old_ignored, REVIEWED)
        result = self.inventory_result(old_defaults + old_ignored, old_ignored)
        self.assertEqual(result["inventory"]["reason"], "listing_mismatch")
        self.assertEqual(result["inventory"]["observed"], dict(ordinary=259, default=242, ignored=17))
        self.assertEqual(result["inventory"]["differences"], dict(
            default=dict(missing=6, extra=0, unallowlisted_extra=0),
            ignored=dict(missing=1, extra=0, unallowlisted_extra=0)))
        self.assertEqual(len(result["inventory"]["samples"]), 7)
        self.assertFalse(result["inventory"]["samples_truncated"])
        self.validate_failure(self.inventory_value(result))

    def test_regression_new_cases_are_mandatory_and_prior_failure_receipts_cannot_bind(self):
        prior = json.loads(self.source_blob(gate.INVENTORY, "42447728419b69011efbd69f9f7ff5935c1e271f"))
        for stage in ("credentials_unit", "credentials_pg", "real_auth"):
            additions = set(REVIEWED["groups"][stage]) - set(prior["groups"][stage])
            self.assertTrue(additions)
            for name in additions:
                text = self.log(stage).replace("test " + name + " ... ok\n", "")
                text = text.replace(f'{len(REVIEWED["groups"][stage])} passed',
                                    f'{len(REVIEWED["groups"][stage]) - 1} passed')
                with self.subTest(stage=stage, name=name), self.assertRaises(ValueError):
                    gate.verify_test_log(stage, text, REVIEWED)
        value = self.inventory_value(self.inventory_result(None, None))
        for change in (dict(source_sha=prior["source_commit"]),
                       dict(source_inventory_sha256=gate.digest(gate.canonical(prior["compiled_source_sha256"]))),
                       dict(inventory=dict(value["inventory"], expected=dict(default=242, ignored=17)))):
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.validate_failure(dict(value, **change))

    def test_regression_successor_preserves_112_selectors_and_all_safe_parser_functions(self):
        import ast
        functions = lambda tree: {node.name: ast.dump(node, include_attributes=False)
            for node in tree.body if isinstance(node, ast.FunctionDef)}
        old = functions(ast.parse(self.source_blob(gate.HELPER, "42447728419b69011efbd69f9f7ff5935c1e271f")))
        new = functions(ast.parse((ROOT / gate.HELPER).read_bytes()))
        self.assertEqual(set(old), set(new))
        self.assertTrue(all(new[name] == body for name, body in old.items() if name != "reviewed_inventory"))
        selectors = lambda tree: {node.name for cls in tree.body if isinstance(cls, ast.ClassDef)
            for node in cls.body if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        previous = selectors(ast.parse(self.source_blob("scripts/tests/test_hosted_backend_gate.py", "42447728419b69011efbd69f9f7ff5935c1e271f")))
        self.assertEqual(len(previous), 112)
        self.assertLessEqual(previous, selectors(ast.parse(Path(__file__).read_bytes())))

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

    def failure_test_value(self):
        value = self.failure_value()
        name = REVIEWED["groups"]["credentials_pg"][0]
        value.update(kind="safe_test_failure", stage="credentials_pg", gate_failed_stage="credentials_pg",
                     failed_tests=[name], categories=["test_failure"],
                     diagnostics=[dict(error_code=None, file="backend/infra/tests/support/pm_credential_creation.rs", line=300, column=5)])
        return value

    def test_test_failure_parser_emits_only_reviewed_names_and_fleet_locations(self):
        name = REVIEWED["groups"]["credentials_pg"][0]
        path = "infra/tests/support/pm_credential_creation.rs"
        data = (f"test {name} ... FAILED\nthread 'PRIVATE_SENTINEL' panicked at {path}:300:5:\n"
                "Database error PRIVATE_SENTINEL\n"
                "thread 'secret' panicked at /qa/src/services-base/private.rs:10:2:\n"
                "test PRIVATE_SENTINEL ... FAILED\n").encode()
        result = gate.safe_test_diagnostics(io.BytesIO(data), {name}, REVIEWED["rust_source_sha256"], Path("/qa/src/fleet-control/backend"))
        self.assertEqual(result, dict(failed_tests=[name], categories=["test_failure"], truncated=False,
                                     diagnostics=[dict(error_code=None, file="backend/" + path, line=300, column=5)]))
        self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_test_failure_parser_rejects_unsafe_locations_and_bounds_private_input(self):
        data = ("thread 'secret' panicked at ../services-base/private.rs:10:2:\n"
                "thread 'secret' panicked at infra/tests/support/pm_credential_creation.rs:0:1:\n"
                "thread 'secret' panicked at infra/tests/support/pm_credential_creation.rs:1:10001:\n").encode()
        result = gate.safe_test_diagnostics(io.BytesIO(data), set(), REVIEWED["rust_source_sha256"], Path("/qa/src/fleet-control/backend"))
        self.assertEqual(result["diagnostics"], [])
        self.assertEqual(result["categories"], ["unknown"])
        with mock.patch.object(gate, "DIAGNOSTIC_INPUT_LIMIT", 16):
            result = gate.safe_test_diagnostics(io.BytesIO(b"PRIVATE_SENTINEL" * 100), set(), {}, Path("/qa/backend"))
        self.assertTrue(result["truncated"])
        self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_test_failure_schema_rejects_private_names_extra_fields_and_compiler_claims(self):
        value = self.failure_test_value()
        self.validate_failure(value)
        for change in (dict(failed_tests=["PRIVATE_SENTINEL"]), dict(stage="preflight"),
                       dict(stage=[]), dict(message="PRIVATE_SENTINEL"), dict(categories=["PRIVATE_SENTINEL"]),
                       dict(kind="safe_compiler_failure"), dict(backend_quality_gate=True),
                       dict(failed_tests=value["failed_tests"] * 2),
                       dict(diagnostics=[dict(value["diagnostics"][0], error_code="E0308")])):
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.validate_failure(dict(value, **change))

    def hint_diagnostics(self, detail, path="infra/tests/support/pm_credential_creation.rs"):
        name = REVIEWED["groups"]["credentials_pg"][0]
        data = (f"test {name} ... FAILED\nthread 'PRIVATE_SENTINEL' panicked at {path}:300:5:\n" + detail + "\n").encode()
        return gate.safe_test_diagnostics(io.BytesIO(data), {name}, REVIEWED["rust_source_sha256"], Path("/qa/src/fleet-control/backend"))

    def test_null_decode_hints_require_exact_definition_or_body_field_and_null_error(self):
        for category, field in gate.TEST_NULL_HINTS.items():
            for detail in (
                'Query(SqlxError(ColumnDecode { index: "\\"' + field + '\\"", source: UnexpectedNullError }))',
                'Type("A null value was encountered while decoding \\"' + field + '\\"")',
            ):
                with self.subTest(category=category, detail=detail):
                    result = self.hint_diagnostics(detail + " PRIVATE_SENTINEL")
                    self.assertEqual(result["categories"], sorted([category, "test_failure"]))
                    self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
        for detail in (
            'ColumnDecode { index: "\\"PRIVATE_SENTINEL\\"", source: UnexpectedNullError }',
            'ColumnDecode { index: "\\"definition\\"", source: PRIVATE_SENTINEL }',
            'Type("A null value was encountered while decoding \\"private\\"")',
            "definition body UnexpectedNullError PRIVATE_SENTINEL",
        ):
            self.assertEqual(self.hint_diagnostics(detail)["categories"], ["test_failure"])

    def test_panic_hints_require_immediate_allowlisted_location_not_arbitrary_private_text(self):
        detail = 'ColumnDecode { index: "\\\"definition\\\"", source: UnexpectedNullError }'
        for path in ("../services-base/private.rs", "/qa/src/services-base/private.rs"):
            self.assertEqual(self.hint_diagnostics(detail, path)["categories"], ["test_failure"])
        for text in (detail, "blank\n" + detail, 'Custom("Hermes guard is missing PRIVATE_SENTINEL")',
                     'PrivateCustom("Hermes guard is missing")', "Hermes guard is missing"):
            if text == detail:
                result = gate.safe_test_diagnostics(io.BytesIO(text.encode()), set(), {}, Path("/qa/backend"))
                self.assertEqual(result["categories"], ["unknown"])
            else:
                self.assertEqual(self.hint_diagnostics(text)["categories"], ["test_failure"])

    def test_panic_hints_do_not_mine_new_headers_or_test_result_boundaries(self):
        detail = r'ColumnDecode { index: "\"definition\"", source: UnexpectedNullError }'
        owned = "infra/tests/support/pm_credential_creation.rs"
        for boundary in (
            f"thread '{detail}' panicked at /qa/src/services-base/private.rs:10:2:",
            f"thread '{detail}' panicked at {owned}:301:5:",
            f"thread '{detail}' panicked at malformed PRIVATE_SENTINEL",
            f"test {detail} ... FAILED",
            f"test result: FAILED {detail}",
        ):
            with self.subTest(boundary=boundary):
                result = self.hint_diagnostics(boundary + "\nunrelated PRIVATE_SENTINEL")
                self.assertEqual(result["categories"], ["test_failure"])
                self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
        result = self.hint_diagnostics(
            f"thread '{detail}' panicked at {owned}:301:5:\n" + detail)
        self.assertEqual(result["categories"], ["null_definition_decode", "test_failure"])
        self.assertEqual(len(result["diagnostics"]), 2)

    def test_panic_hints_discard_oversized_line_and_its_continuation(self):
        with mock.patch.object(gate, "DIAGNOSTIC_LINE_LIMIT", 128):
            result = self.hint_diagnostics("PRIVATE_SENTINEL" * 20 + 'Custom("Hermes guard is missing")')
        self.assertTrue(result["truncated"])
        self.assertEqual(result["categories"], ["test_failure"])
        self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_panic_hints_respect_total_input_and_diagnostic_count_bounds(self):
        name = REVIEWED["groups"]["credentials_pg"][0]
        prefix = f"test {name} ... FAILED\nthread 't' panicked at infra/tests/support/pm_credential_creation.rs:300:5:\n"
        with mock.patch.object(gate, "DIAGNOSTIC_INPUT_LIMIT", len(prefix.encode())):
            result = self.hint_diagnostics(r'ColumnDecode { index: "\"definition\"", source: UnexpectedNullError }')
        self.assertTrue(result["truncated"])
        self.assertNotIn("null_definition_decode", result["categories"])
        data = "".join(f"thread 't' panicked at infra/tests/support/pm_credential_creation.rs:{i + 1}:5:\n"
                       r'ColumnDecode { index: "\"definition\"", source: UnexpectedNullError }' + '\n' for i in range(gate.DIAGNOSTIC_LIMIT + 1))
        result = gate.safe_test_diagnostics(io.BytesIO(data.encode()), set(), REVIEWED["rust_source_sha256"], Path("/qa/src/fleet-control/backend"))
        self.assertEqual(len(result["diagnostics"]), gate.DIAGNOSTIC_LIMIT)
        self.assertTrue(result["truncated"])
        self.assertEqual(result["categories"], ["null_definition_decode", "test_failure"])
        self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_panic_hint_schema_rejects_unknown_values_unanchored_or_compiler_categories(self):
        value = self.failure_test_value()
        for categories in (["PRIVATE_SENTINEL"], ["null_definition_decode=PRIVATE_SENTINEL", "test_failure"],
                           ["hermes_guard_missing", "null_definition_decode", "test_failure"],
                           ["test_failure", "hermes_guard_missing"], ["hermes_guard_missing"],
                           [True], [{"PRIVATE_SENTINEL": "body"}], ["network", "test_failure"]):
            with self.subTest(categories=categories), self.assertRaises(ValueError):
                self.validate_failure(dict(value, categories=categories))
        with self.assertRaises(ValueError):
            self.validate_failure(dict(value, categories=["null_definition_decode", "test_failure"], diagnostics=[]))
        with self.assertRaises(ValueError):
            self.validate_failure(dict(self.failure_value(), categories=["hermes_guard_missing"]))

    def test_panic_hint_authenticated_readback_accepts_old_schema_and_fixed_hints_only(self):
        for categories in (["test_failure"], ["unknown"], ["null_body_decode", "test_failure"]):
            value = dict(self.failure_test_value(), categories=categories)
            run, artifact, payload, args = self.failure_artifact(value=value)
            result = json.loads(gate.validate_failure_readback(run, artifact, payload, **args)[gate.FAILURE_FILE])
            self.assertEqual(result["categories"], categories)
            for flag in ("backend_quality_gate", "all_quality_gate", "sdlc_acceptance"):
                self.assertFalse(result[flag])
        value = dict(self.failure_test_value(), categories=["PRIVATE_SENTINEL"])
        run, artifact, payload, args = self.failure_artifact(value=value)
        with self.assertRaises(ValueError):
            gate.validate_failure_readback(run, artifact, payload, **args)

    def test_test_failure_readback_is_authenticated_failure_not_acceptance(self):
        run, artifact, payload, args = self.failure_artifact(value=self.failure_test_value())
        files = gate.validate_failure_readback(run, artifact, payload, **args)
        value = json.loads(files[gate.FAILURE_FILE])
        self.assertEqual(value["kind"], "safe_test_failure")
        self.assertFalse(value["backend_quality_gate"])
        with self.assertRaises(ValueError):
            gate.validate_readback(run, artifact, payload, **args)

    def test_credentials_groups_use_safe_test_failure_without_cross_group_identity_or_acceptance(self):
        for stage in ("credentials_unit", "credentials_pg", "credentials_migration", "real_auth"):
            name = REVIEWED["groups"][stage][0]
            foreign = REVIEWED["groups"]["config_api"][0]
            with self.subTest(stage=stage):
                parsed = gate.safe_test_diagnostics(io.BytesIO(
                    f"test {name} ... FAILED\ntest {foreign} ... FAILED\nPRIVATE_SENTINEL\n".encode()),
                    gate.failure_test_names(REVIEWED, stage), REVIEWED["rust_source_sha256"], Path("/qa/src/fleet-control/backend"))
                self.assertEqual(parsed["failed_tests"], [name])
                value = dict(self.failure_test_value(), stage=stage, gate_failed_stage=stage, **parsed)
                self.validate_failure(value)
                with self.assertRaises(ValueError):
                    self.validate_failure(dict(value, failed_tests=[foreign]))
                run, artifact, payload, args = self.failure_artifact(value=value)
                gate.validate_failure_readback(run, artifact, payload, **args)
                with self.assertRaises(ValueError):
                    gate.validate_readback(run, artifact, payload, **args)

    def failure_artifact(self, extra=None, value=None):
        run, artifact, _, args = self.artifact()
        run["conclusion"] = "failure"
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w") as archive:
            archive.writestr(gate.FAILURE_FILE, gate.canonical(value or self.failure_value()))
            for name in extra or ():
                archive.writestr(name, b"PRIVATE_SENTINEL")
        payload = buffer.getvalue()
        artifact.update(name="fleet-backend-failure-c11-123-1", digest="sha256:" + gate.digest(payload))
        args["artifact_digest"] = gate.digest(payload)
        return run, artifact, payload, args

    def test_failure_readback_authenticates_failure_run_attempt_sha_artifact_digest(self):
        run, artifact, payload, args = self.failure_artifact()
        self.assertEqual(set(gate.validate_failure_readback(run, artifact, payload, **args)), {gate.FAILURE_FILE})
        for key, item in (("id", 456), ("run_attempt", 2), ("conclusion", "success"), ("event", "pull_request"), ("head_sha", "b" * 40)):
            with self.assertRaises(ValueError):
                gate.validate_failure_readback(dict(run, **{key: item}), artifact, payload, **args)
        for key, item in (("expired", True), ("name", "fleet-backend-c11-123-1"), ("digest", "sha256:" + "0" * 64)):
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
        focused = {name: dict(passed=len(names), failed=0, ignored=0 if name == "foundation" else 0,
                              tests=sorted(names)) for name, names in REVIEWED["groups"].items()}
        focused["workspace"] = dict(passed=248, failed=0, ignored=18, tests=["case" + str(i) for i in range(248)])
        report = dict(backend_quality_gate=True, all_quality_gate=False, sdlc_acceptance=False, status="success",
                      gates=[dict(stage=name, status="passed") for name in gate.GATES], focused=focused,
                      cleanup=dict(scratch=True, synthetic_databases=True), ignored_required=18, foundation_ignored=0,
                      contracts={stage: dict(passed=len(names), failed=0, ignored=0, tests=sorted(names))
                                 for stage, names in REVIEWED["python_contracts"].items()},
                      migration_ledger=dict(snapshots=gate.expected_migration_receipt(REVIEWED), applied_at_preserved=True),
                      runtime_inventory=dict(ignored=18, listed_default_count=248, ignored_names_sha256=gate.digest(gate.canonical(
                          sorted(item["name"] for item in REVIEWED["ignored"])))))
        report.update(report_changes or {})
        provenance = dict(version=1, repository=gate.REPOSITORY, branch=gate.BRANCH, source_sha=gate.SOURCE_SHA,
                          base_sha=gate.BASE_SHA, auth_sha=gate.AUTH_SHA, workflow_sha=workflow_sha, workflow_path=gate.WORKFLOW,
                          source_inventory_sha256=gate.SOURCE_INVENTORY_SHA,
                          package_sha=gate.PACKAGE_SHA, package_tree=gate.PACKAGE_TREE,
                          package_inventory_sha256=gate.PACKAGE_INVENTORY_SHA,
                          run_id=123, run_attempt=1, rust="1.88.0", postgres="17.6", cargo_build_jobs=1,
                          source_tree=gate.SOURCE_TREE, swagger_sha256=gate.SWAGGER_SHA, openapi_sha256=gate.OPENAPI_SHA, backend_quality_gate=True,
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
                        name="fleet-backend-c11-123-1", digest="sha256:" + gate.digest(payload))
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

    def test_all9_databases_init_environment_and_finally_match(self):
        text = (ROOT / gate.INIT).read_text()
        created = re.findall(r"CREATE DATABASE (\w+)", text)
        self.assertEqual(set(created) | {"fleet_foundation_test"}, set(gate.DATABASES))
        self.assertEqual(len(created), 8)
        self.assertEqual(set(gate.URLS.values()), set(gate.DATABASES))
        self.assertEqual(len(gate.URLS), 9)
        self.assertIn("CREATE ROLE fleet_approval_events_test LOGIN;", text)
        helper = (ROOT / gate.HELPER).read_text()
        self.assertIn("drop_databases(owned_dbs)", helper)
        self.assertIn('owned_dbs = list(DATABASES)', helper)

    def migration_files(self, root, expected):
        for stage, names in expected.items():
            (root / ("migration-" + stage + ".txt")).write_text("".join(n + "\n" for n in names), newline="\n")
            (root / ("migration-" + stage + "-ledger.tsv")).write_text(
                "".join(n + "\t1000\n" for n in names), newline="\n")

    def test_migration_smoke_exact12_11_12_0_12_no_count_waiver(self):
        expected = gate.expected_migration_receipt(REVIEWED)
        self.assertEqual([len(x) for x in expected.values()], [12, 11, 12, 0, 12])
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.migration_files(root, expected)
            gate.verify_migration_snapshots(root, REVIEWED)
            (root / "migration-down_one.txt").write_text("\n".join(expected["up"][:11]) + "\n")
            # Equal counts with a substituted name must not satisfy the ledger guard.
            (root / "migration-down_one.txt").write_text("\n".join(expected["down_one"][:-1] + ["foreign"]) + "\n")
            with self.assertRaises(ValueError):
                gate.verify_migration_snapshots(root, REVIEWED)

    def test_fallback_drops_only_persisted_exact_owned9_before_scratch_removal(self):
        with tempfile.TemporaryDirectory() as temporary:
            parent = Path(temporary).resolve()
            root = parent / "fc11"
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

    def test_C11_allowlist_exact_compiled_inputs_no_runtime_cache_or_extra_schema(self):
        self.assertEqual(len(REVIEWED["compiled_source_sha256"]), 309)
        self.assertEqual(len(REVIEWED["rust_source_sha256"]), 105)
        self.assertEqual(gate.FLEET_ROOTS, ("backend", ".base-revision", "openapi", "docs/TESTING.md"))
        for name in REVIEWED["compiled_source_sha256"]:
            self.assertFalse({".local", ".git", "target", "node_modules", "__pycache__", ".venv"}.intersection(Path(name).parts))
        self.assertNotIn("utility_source_sha256", REVIEWED)
        self.assertEqual(set(self.workflow()["jobs"]), {"backend"})

    def test_PR64_commands_have_exact_noncolliding_filters_and_scoped_database(self):
        shell = (ROOT / gate.GATE).read_text()
        commands = (
            "run_tests config_api -p api --lib routes::sdlc_configuration::tests::",
            "run_tests base_package_unit -p infra --lib base_package::tests::",
            "run_tests config_files_unit -p infra --lib effective_configuration::tests::",
            "run_tests package_effective_unit -p infra --lib tests::base_package_effective_",
            "run_tests config_shared_unit -p shared --lib config::tests::",
            "run_tests base_package_pg -p infra --test sdlc_foundation base_package::",
            "run_tests config_revision_pg -p infra --test sdlc_foundation config_revision_",
        )
        positions = [shell.index(command) for command in commands]
        self.assertEqual(positions, sorted(positions))
        self.assertLess(shell.index("run_tests foundation"), positions[0])
        self.assertLess(positions[-1], shell.index("stage=workspace"))
        candidates = sum((REVIEWED["groups"][stage] for stage in (
            "base_package_unit", "config_files_unit", "package_effective_unit")), [])
        self.assertEqual(sorted(name for name in candidates if "tests::base_package_effective_" in name),
                         REVIEWED["groups"]["package_effective_unit"])
        for stage in ("base_package_pg", "config_revision_pg"):
            self.assertIn('FLEET_TEST_DATABASE_URL="$FLEET_CONFIGURATION_TEST_DATABASE_URL" \\\n  run_tests ' + stage, shell)
        self.assertNotEqual(gate.database_environment()["FLEET_TEST_DATABASE_URL"],
                            gate.database_environment()["FLEET_CONFIGURATION_TEST_DATABASE_URL"])

    def test_PR64_named_groups_reject_missing_duplicate_and_count_only(self):
        for stage in ("config_api", "base_package_unit", "config_files_unit", "package_effective_unit",
                      "config_shared_unit", "base_package_pg", "config_revision_pg"):
            text = self.log(stage)
            self.assertEqual(gate.verify_test_log(stage, text, REVIEWED)["passed"], len(REVIEWED["groups"][stage]))
            first = text.splitlines()[0]
            for bad in (text.replace(first + "\n", "", 1), text + first + "\n", text.splitlines()[-1],
                        text.replace(" ... ok", " ... ignored", 1)):
                with self.subTest(stage=stage), self.assertRaises(ValueError):
                    gate.verify_test_log(stage, bad, REVIEWED)

    def package_git(self, args, *, origin=None, tree=None, metadata=None):
        if args == ("config", "--get", "remote.origin.url"):
            return (origin or "https://github.com/FerrPOINT/services-base.git").encode()
        if args == ("rev-parse", "HEAD^{tree}"):
            return (tree or gate.PACKAGE_TREE).encode()
        if args[:1] == ("ls-tree",):
            return metadata
        if args == ("cat-file", "-e", gate.PACKAGE_SHA + ":agent-skills/manifest.json"):
            return b""
        self.fail(str(args))

    def test_package_qualification_uses_exact_separate_pin_tree_and_git_metadata(self):
        metadata = b"synthetic metadata\0" * 22
        proof = dict(REVIEWED["package_input"], inventory_sha256=gate.digest(metadata))
        with mock.patch.object(gate, "clean_head") as clean, \
                mock.patch.object(gate, "git", side_effect=lambda root, *args: self.package_git(args, metadata=metadata)) as git:
            self.assertEqual(gate.qualify_package(Path("owned"), dict(package_input=proof)), proof)
            clean.assert_called_once_with(Path("owned"), gate.PACKAGE_SHA)
            git.assert_any_call(Path("owned"), "ls-tree", "-r", "-l", "-z", gate.PACKAGE_SHA, "--",
                                "agent-skills/manifest.json", "agent-skills/roles", "agent-skills/skills")
            git.assert_any_call(Path("owned"), "cat-file", "-e", gate.PACKAGE_SHA + ":agent-skills/manifest.json")
        self.assertEqual(len({gate.PACKAGE_SHA, gate.BASE_SHA, gate.AUTH_SHA}), 3)

    def test_package_qualification_rejects_foreign_origin_tree_metadata_and_absent_manifest(self):
        metadata = b"synthetic metadata\0" * 22
        proof = dict(REVIEWED["package_input"], inventory_sha256=gate.digest(metadata))
        for changes in (dict(origin="https://github.com/foreign/services-base.git"), dict(tree="0" * 40),
                        dict(metadata=metadata + b"extra\0"), dict(metadata=metadata.replace(b"synthetic", b"changed"))):
            options = dict(metadata=metadata, **{})
            options.update(changes)
            with mock.patch.object(gate, "clean_head"), \
                    mock.patch.object(gate, "git", side_effect=lambda root, *args: self.package_git(args, **options)), \
                    self.subTest(changes=changes), self.assertRaises(ValueError):
                gate.qualify_package(Path("owned"), dict(package_input=proof))
        for error in ("Wrong HEAD", "Dirty checkout", "Missing manifest"):
            def git(root, *args):
                if args[:1] == ("cat-file",):
                    raise ValueError(error)
                return self.package_git(args, metadata=metadata)
            with mock.patch.object(gate, "clean_head", side_effect=None if error == "Missing manifest" else ValueError(error)), \
                    mock.patch.object(gate, "git", side_effect=git), self.subTest(error=error), self.assertRaises(ValueError):
                gate.qualify_package(Path("owned"), dict(package_input=proof))

    def test_package_inventory_missing_wrong_pin_and_sdk_substitution_fail_closed(self):
        for proof in (None, {}, dict(REVIEWED["package_input"], commit=gate.BASE_SHA),
                      dict(REVIEWED["package_input"], commit=gate.AUTH_SHA),
                      dict(REVIEWED["package_input"], inventory_sha256="0" * 64)):
            value = dict(REVIEWED, package_input=proof)
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                (root / gate.INVENTORY).parent.mkdir(parents=True)
                (root / gate.INVENTORY).write_bytes(gate.canonical(value))
                with self.assertRaisesRegex(ValueError, "Package input drift"):
                    gate.reviewed_inventory(root)

    def test_package_mandatory_wiring_and_pre_post_proof_never_inherits_ambient_checkout(self):
        shell = (ROOT / gate.GATE).read_text()
        helper = (ROOT / gate.HELPER).read_text()
        pre_cargo = shell[:shell.index("cargo fmt")]
        self.assertIn("FLEET_TEST_BASE_PACKAGE_CHECKOUT", pre_cargo)
        self.assertIn('test -n "${!name}"', pre_cargo)
        self.assertIn('git -C "$FLEET_TEST_BASE_PACKAGE_CHECKOUT" rev-parse HEAD', pre_cargo)
        self.assertIn(gate.PACKAGE_SHA + ":agent-skills/manifest.json", pre_cargo)
        self.assertEqual(helper.count("qualify_package(checkouts[3][0], reviewed)"), 2)
        self.assertIn("FLEET_TEST_BASE_PACKAGE_CHECKOUT=str(checkouts[3][0])", helper)
        steps = self.workflow()["jobs"]["backend"]["steps"]
        checkout = next(x for x in steps if x.get("with", {}).get("path") == "base-role-package")
        self.assertEqual(checkout["with"]["ref"], gate.PACKAGE_SHA)
        self.assertEqual(checkout["with"]["persist-credentials"], "false")
        self.assertLess(next(i for i, x in enumerate(steps) if "before Base tokens" in x.get("name", "")), steps.index(checkout))

    def test_package_refusal_happens_before_private_scratch_or_native_effects(self):
        with mock.patch.object(gate, "preflight", return_value=(Path("owned"), ROOT, "a" * 40)), \
                mock.patch.object(gate, "clean_head"), mock.patch.object(gate, "command", return_value=b"rustc 1.88.0"), \
                mock.patch.object(Path, "read_text", return_value=gate.BASE_SHA), \
                mock.patch.object(gate, "reviewed_inventory", return_value=REVIEWED), \
                mock.patch.object(gate, "qualify_package", side_effect=ValueError("Package missing")), \
                mock.patch.object(gate, "resource_guard") as resources, mock.patch.object(gate.subprocess, "Popen") as spawn:
            with self.assertRaisesRegex(ValueError, "Package missing"):
                gate.execute()
            resources.assert_not_called()
            spawn.assert_not_called()

    def test_package_provenance_is_required_by_success_and_failure_receipts(self):
        for key in ("package_sha", "package_tree", "package_inventory_sha256"):
            run, artifact, payload, args = self.artifact(provenance_changes={key: "wrong"})
            with self.subTest(key=key), self.assertRaises(ValueError):
                gate.validate_readback(run, artifact, payload, **args)
            with self.assertRaises(ValueError):
                self.validate_failure(dict(self.failure_value(), **{key: "wrong"}))


if __name__ == "__main__":
    unittest.main()
