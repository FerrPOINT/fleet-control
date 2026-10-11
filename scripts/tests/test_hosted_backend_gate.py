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
    def test_stop025_successor_keeps_all196_identities_and_exact_execution_guards(self):
        import ast
        frozen = "50cb550af47e06530997fc1f84a5562fd4503b9e"
        old = self.source_blob(gate.HELPER, frozen).decode()
        current = self.source_blob(gate.HELPER, "36fac86c7cdd945f8614fbaa6b7e302989b1a71d").decode()
        normalized = current.replace(gate.SOURCE_SHA, "5bc0fd3fd92a11a6957858525d9b124be00c1644")
        normalized = normalized.replace(gate.SOURCE_INVENTORY_SHA,
            "0f55274de1a4b602e0378eb47db48ee23aedd731b4e3d4782f01db811f48e5c0")
        normalized = re.sub(r"\b180\b", "179", normalized)
        normalized = re.sub(r"\b125\b", "124", normalized)
        for before, after in (
            ('len(value["workspace_default_declarations"]) == 401', 'len(value["workspace_default_declarations"]) == 400'),
            ('len(value["pm_workspace_required"]) == 50', 'len(value["pm_workspace_required"]) == 49'),
            ('len(set(value["pm_workspace_required"])) == 50', 'len(set(value["pm_workspace_required"])) == 49'),
            ('len(value["groups"]["pm_human_controls"]) == 6', 'len(value["groups"]["pm_human_controls"]) == 5'),
            ('len(set(value["groups"]["pm_human_controls"])) == 6', 'len(set(value["groups"]["pm_human_controls"])) == 5'),
            ('len(value["migration_registries"]["canonical"]) == 26', 'len(value["migration_registries"]["canonical"]) == 25'),
            ('len(value["migration_registries"]["split"]) == 29', 'len(value["migration_registries"]["split"]) == 28'),
        ):
            self.assertEqual(normalized.count(before), 1, before)
            normalized = normalized.replace(before, after)
        functions = lambda text: {node.name: ast.dump(node) for node in ast.parse(text).body
            if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
        before, after = functions(old), functions(normalized)
        self.assertEqual(before.keys(), after.keys())
        self.assertEqual({name for name in before if before[name] != after[name]},
                         {"expected_migration_receipt", "verify_migration_snapshots"})
        old_stages = next(ast.literal_eval(n.value) for n in ast.parse(old).body
            if isinstance(n, ast.Assign) and n.targets[0].id == "GATES")
        self.assertEqual(gate.GATES, old_stages)
        self.assertEqual(len(gate.GATES), 84)
        names = lambda data: {n.name for n in ast.walk(ast.parse(data))
            if isinstance(n, ast.FunctionDef) and n.name.startswith("test_")}
        previous = names(self.source_blob("scripts/tests/test_hosted_backend_gate.py", frozen))
        self.assertEqual(len(previous), 196)
        self.assertTrue(previous <= names(Path(__file__).read_bytes()))
        self.assertEqual((ROOT / gate.INIT).read_bytes(), self.source_blob(gate.INIT, frozen))
        self.assertEqual((ROOT / gate.WORKFLOW).read_bytes(), self.source_blob(gate.WORKFLOW, frozen).replace(
            b"5bc0fd3fd92a11a6957858525d9b124be00c1644", gate.SOURCE_SHA.encode()))
        shell = (ROOT / gate.GATE).read_bytes()
        shell = shell.replace(b"ignored=125", b"ignored=124")
        shell = shell.replace(b"  cargo run --locked -p migration -- down -n 1\n  migration_snapshot down_stop\n", b"")
        shell = shell.replace(b"  cargo run --locked -p migration -- up -n 1\n  migration_snapshot stop_reapply\n", b"")
        shell = shell.replace(b"down -n 26", b"down -n 25")
        self.assertEqual(shell, self.source_blob(gate.GATE, frozen))

    def test_stop025_inventory_adds_only_one_ordinary_one_ignored_and_one_rust_file(self):
        previous = json.loads(self.source_blob(gate.INVENTORY, "50cb550af47e06530997fc1f84a5562fd4503b9e"))
        identity = lambda row: (row["source"], row["name"])
        for key, added in (
            ("workspace_default_declarations", ("backend/infra/src/runtime/pm_readback.rs", "completed_requires_the_shared_terminal_flags")),
            ("ignored", ("backend/infra/tests/support/pm_human_controls.rs",
                "pm_human_controls::pm_stop_custody_migration_preserves_original_function_and_holds_unsafe_downgrade")),
        ):
            before, after = ({identity(row) for row in value[key]} for value in (previous, REVIEWED))
            self.assertEqual(after - before, {added})
            self.assertTrue(before <= after)
        migration = "backend/migration/src/m20261011_000025_pm_stop_custody.rs"
        self.assertEqual(set(REVIEWED["rust_source_sha256"]) - set(previous["rust_source_sha256"]), {migration})
        self.assertTrue(previous["rust_source_sha256"].keys() <= REVIEWED["rust_source_sha256"].keys())
        self.assertEqual((len(REVIEWED["compiled_source_sha256"]), len(REVIEWED["rust_source_sha256"])), (404, 191))
        self.assertEqual((len(REVIEWED["workspace_default_declarations"]), len(REVIEWED["ignored"])), (401, 180))
        self.assertEqual(REVIEWED["default_foundation_ignored"], 125)
        for stage, names in previous["groups"].items():
            actual = REVIEWED["groups"][stage]
            if stage == "pm_human_controls":
                self.assertEqual(set(actual) - set(names), {
                    "pm_human_controls::pm_stop_custody_migration_preserves_original_function_and_holds_unsafe_downgrade"})
                self.assertTrue(set(names) <= set(actual))
            else:
                self.assertEqual(actual, names, stage)
        for key in ("authority", "utility_source_sha256", "utility_tree", "package_input", "python_contracts"):
            self.assertEqual(REVIEWED[key], previous[key], key)

    def test_stop025_actual_pg_selector_and_both_new_ledger_edges_cannot_be_omitted(self):
        name = "pm_human_controls::pm_stop_custody_migration_preserves_original_function_and_holds_unsafe_downgrade"
        text = self.log("pm_human_controls")
        self.assertEqual(gate.verify_test_log("pm_human_controls", text, REVIEWED)["passed"], 6)
        for bad in (text.replace("test " + name + " ... ok\n", ""), text + "test " + name + " ... ok\n",
                    text.replace("test " + name + " ... ok", "test " + name + " ... ignored")):
            with self.assertRaises(ValueError):
                gate.verify_test_log("pm_human_controls", bad, REVIEWED)
        old = json.loads(self.source_blob(gate.INVENTORY, "50cb550af47e06530997fc1f84a5562fd4503b9e"))
        old_helper = self.source_blob(gate.HELPER, "50cb550af47e06530997fc1f84a5562fd4503b9e").decode()
        namespace = {}
        function = old_helper.split("def expected_migration_receipt(reviewed):", 1)[1].split("\n\ndef ", 1)[0]
        exec("def expected_migration_receipt(reviewed):" + function, {"require": gate.require}, namespace)
        expected = gate.expected_migration_receipt(REVIEWED)
        historical = namespace["expected_migration_receipt"](old)
        self.assertEqual(expected["down_stop"], historical["up"])
        self.assertEqual(expected["ack_reapply"], historical["ack_reapply"])
        for key in historical.keys() - {"up", "final_up"}:
            self.assertEqual(expected[key], historical[key], key)
        self.assertEqual(expected["stop_reapply"], expected["up"])
        for stage in ("down_stop", "stop_reapply"):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.migration_files(root, expected)
                ledger = root / ("migration-" + stage + "-ledger.tsv")
                ledger.write_text(ledger.read_text().replace("\t1000\n", "\t1001\n", 1), newline="\n")
                with self.assertRaises(ValueError):
                    gate.verify_migration_snapshots(root, REVIEWED)

    def test_runtime_stream_lifetime_fixture_inverse_matches_58e_exactly(self):
        path = "backend/infra/tests/support/runtime_stream_bounds.rs"
        name = "runtime_stream_bounds_invalid_json_utf8_and_foreign_control_payloads_never_mirror"
        before = self.source_blob(path, "58e0c18040e35f649bb1f3e5dd32e2190bfa94fa")
        current = self.source_blob(path)
        signature = ('#[test]\n#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]\nfn ' + name + '() {').encode()
        original = signature.replace(b"#[test]", b"#[tokio::test]").replace(b"\nfn ", b"\nasync fn ")
        block = (b'        let runtime = tokio::runtime::Builder::new_current_thread()\n'
                 b'            .enable_all()\n            .build()\n'
                 b'            .expect("isolated runtime stream fixture");\n'
                 b'        runtime.block_on(rejected(EventResponse::body(vec![frame])));')
        self.assertEqual(current.count(signature), 1)
        self.assertEqual(current.count(block), 1)
        self.assertEqual(current.replace(signature, original).replace(block,
            b"        rejected(EventResponse::body(vec![frame])).await;"), before)

    def test_additive024_keeps_all83_gates_and_exact_source_selected_cases(self):
        import ast
        donor = "8f78939ea6401d12060d85814390edf0c4cd7f0b"
        old = ast.parse(self.source_blob(gate.HELPER, donor))
        stages = next(ast.literal_eval(n.value) for n in old.body if isinstance(n, ast.Assign)
                      and n.targets[0].id == "GATES")
        self.assertEqual(len(stages), 83)
        self.assertEqual(tuple(s for s in gate.GATES if s != "pm_ack_migration"), stages)
        self.assertEqual(gate.GATES.count("pm_ack_migration"), 1)
        previous = json.loads(self.source_blob(gate.INVENTORY, donor))
        added = {
            "pm_events::pm_http_disabled_dispatch_denies_foreign_pins_and_unknown_ack",
            "pm_events::pm_http_disabled_dispatch_recovers_accepted_sse_and_terminal_once",
            "pm_dispatch::pm_publication_claim_requires_exact_run_instruction_receipt",
        }
        for stage, names in previous["groups"].items():
            expected = set(names) | added if stage == "foundation" else set(names)
            if stage == "pm_human_controls":
                expected.add("pm_human_controls::pm_stop_custody_migration_preserves_original_function_and_holds_unsafe_downgrade")
            if stage == "pm_recovery_pg":
                expected.add("runtime::pm_recovery::tests::pg::production_aaa_future_layout_without_constructing_or_polling_runtime")
            self.assertEqual(set(REVIEWED["groups"][stage]), expected, stage)
        self.assertEqual(set(REVIEWED["groups"]) - set(previous["groups"]), {"pm_ack_migration"})
        self.assertEqual([name for name in REVIEWED["groups"]["pm_recovery_pg"] if "production_aaa_future_layout" not in name],
                         previous["groups"]["pm_recovery_pg"])
        for kind, count in (("workspace_default_declarations", 401), ("ignored", 180)):
            identity = lambda row: (row["source"], row["name"])
            before = {identity(row) for row in previous[kind]}
            after = {identity(row) for row in REVIEWED[kind]}
            self.assertTrue(before <= after)
            self.assertEqual(len(after), count)
        source = self.source_blob("backend/infra/tests/support/pm_events.rs").decode()
        for name in added - {"pm_dispatch::pm_publication_claim_requires_exact_run_instruction_receipt"}:
            self.assertIn("#[tokio::test]\nasync fn " + name.split("::")[-1] + "()", source)
            self.assertNotIn(name, {row["name"] for row in REVIEWED["ignored"]})
        functions = lambda tree: {n.name: ast.dump(n) for n in tree.body
            if isinstance(n, (ast.FunctionDef, ast.ClassDef))}
        before, after = functions(old), functions(ast.parse(self.source_blob(gate.HELPER,
            "36fac86c7cdd945f8614fbaa6b7e302989b1a71d")))
        self.assertEqual(before.keys(), after.keys())
        self.assertEqual({name for name in before if before[name] != after[name]}, {
            "reviewed_inventory", "expected_migration_receipt", "verify_migration_snapshots",
            "verify_test_log", "verify_runtime_inventory", "execute", "validate_evidence_files",
            "safe_test_diagnostics", "test_failure_logs", "validate_failure_evidence", "main", "resource_guard"})

    def test_additive024_requires_exact_ignored_upgrade_selector_and_owned_cleanup(self):
        name = "pm_ack_bounds_repairs_installed_022_preserving_custody_and_empty_roundtrip"
        path = "backend/migration/tests/pm_ack_bounds.rs"
        self.assertEqual(REVIEWED["groups"]["pm_ack_migration"], [name])
        record = next(row for row in REVIEWED["ignored"] if row["source"] == path)
        self.assertEqual((record["package"], record["target_kind"], record["target"], record["gate"], record["name"]),
                         ("migration", "test", "pm_ack_bounds", "pm_ack_migration", name))
        source = self.source_blob(path).decode()
        self.assertIn('#[ignore = "requires empty disposable FLEET_PM_ACK_MIGRATION_TEST_DATABASE_URL"]', source)
        self.assertIn("async fn " + name + "()", source)
        text = self.log("pm_ack_migration")
        self.assertEqual(gate.verify_test_log("pm_ack_migration", text, REVIEWED)["passed"], 1)
        ignored = "\n".join(row["name"] + ": test" for row in REVIEWED["ignored"])
        ordinary = ignored + "\n" + "\n".join(n + ": test" for n in REVIEWED["pm_workspace_required"])
        gate.verify_runtime_inventory(ordinary, ignored, REVIEWED)
        for bad in (ordinary.replace(name + ": test", ""), ordinary + "\n" + name + ": test"):
            with self.assertRaises(ValueError):
                gate.verify_runtime_inventory(bad, ignored, REVIEWED)
        for bad in ("", text.replace(name, "foreign"), text.replace(" ... ok", " ... ignored"),
                    text + "test " + name + " ... ok\n", text.replace("0 failed", "1 failed")):
            with self.assertRaises(ValueError):
                gate.verify_test_log("pm_ack_migration", bad, REVIEWED)
        bad = copy.deepcopy(REVIEWED)
        bad["groups"]["pm_ack_migration"] = ["foreign"]
        with mock.patch.object(gate.json, "loads", return_value=bad), self.assertRaises(ValueError):
            gate.reviewed_inventory(ROOT)
        shell = (ROOT / gate.GATE).read_text()
        command = "run_ignored_target pm_ack_migration migration pm_ack_bounds\n"
        self.assertEqual(shell.count(command), 1)
        self.assertLess(shell.index(command), shell.index("stage=workspace\n"))
        self.assertIn("FLEET_PM_ACK_MIGRATION_TEST_DATABASE_URL", shell)
        self.assertEqual(gate.database_environment()["FLEET_PM_ACK_MIGRATION_TEST_DATABASE_URL"],
                         "postgres://fleet_test@postgres:5432/fleet_pm_ack_migration_test")
        with mock.patch.object(gate, "psql") as psql, mock.patch.object(gate, "database_catalog", return_value=["postgres"]):
            gate.drop_databases(["fleet_pm_ack_migration_test"])
            self.assertEqual(psql.call_args_list[0].args,
                             ('DROP DATABASE IF EXISTS "fleet_pm_ack_migration_test" WITH (FORCE)',))
        expected = gate.expected_migration_receipt(REVIEWED)
        for stage in ("down_ack", "ack_reapply"):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.migration_files(root, expected)
                ledger = root / ("migration-" + stage + "-ledger.tsv")
                ledger.write_text(ledger.read_text().replace("\t1000\n", "\t1001\n", 1), newline="\n")
                with self.assertRaises(ValueError):
                    gate.verify_migration_snapshots(root, REVIEWED)

    def test_additive024_cannot_relabel_ce_codegen_or_execute_pending_binding(self):
        REVIEWED = json.loads(self.source_blob(gate.INVENTORY, "d4c56055b74a1dd4efdb487f9fc5f6407852f075"))
        pending = json.loads(self.source_blob(gate.INVENTORY, "dab203a25290ec0ecacbdd0d1ee8fe0a9dc96aa3"))
        preparation = pending["config_union_preparation"]
        self.assertEqual(pending["openapi_binding"], dict(status="pending_authentic_codegen", sha256=None))
        self.assertIsNone(preparation["codegen_evidence"])
        self.assertTrue(preparation["authentic_union_codegen_pending"])
        old = preparation["prior_exact_source_codegen_evidence"]
        self.assertEqual((old["source_commit"], old["run_id"], old["artifact_id"]),
                         ("ce4153f453e730dad1e315131d65ca030243264c", 38058114502, 11671964606))
        self.assertNotEqual(old["artifact_bound_source_commit"], gate.SOURCE_SHA)
        with mock.patch.object(gate, "hosted_identity", return_value=(Path("owned"), "a" * 40)), \
                mock.patch.object(gate, "clean_head"), mock.patch.object(gate, "reviewed_inventory", return_value=pending), \
                mock.patch.object(gate, "git") as git, mock.patch.object(gate.subprocess, "Popen") as spawn:
            with self.assertRaisesRegex(ValueError, "binding is pending"):
                gate.preflight()
            git.assert_not_called()
            spawn.assert_not_called()
        gate.require_codegen_binding(REVIEWED)
        bound = REVIEWED["config_union_preparation"]
        self.assertEqual(bound["prior_exact_source_codegen_evidence"], old)
        self.assertFalse(bound["authentic_union_codegen_pending"])
        self.assertFalse(bound["compiled_inventory_includes_pre_regen_schema"])
        self.assertEqual(bound["codegen_evidence"], dict(
            source_commit="3445d922852026bb7ca08ea42186ca7028c7968b",
            artifact_bound_source_commit="3445d922852026bb7ca08ea42186ca7028c7968b",
            workflow_commit="4e499316f9d314b2f34867d32d0d0275bc42af1e",
            run_id=38063507780, run_attempt=1, artifact_id=11673994046,
            artifact_zip_sha256="d01dc1107122396080f1e3573e9b8a716ffdc200d1f03a5e00836e27f7ce2e0e",
            reported_by_parent=False,
            schema_sha256="afa46ac37b726232eda73df46c24d1d42c796f8873eefb68454fbe0f243df501",
            source_file_count=305,
            source_inventory_sha256="9afccf360d82e802eceddb5179ee17cfdd1627c876b854115e3ace62ac9df072",
            source_tree="7a9b7eb5ba13d4fad53add90cc6de07c95b838b4"))

    def test_recovery_successor_preserves_all143_selectors_82_stages_and_exact_additive_wiring(self):
        import ast
        donor = "9565ecc1c2d114d44132598d77f4c5942d440d1d"
        old = ast.parse(self.source_blob(gate.HELPER, donor))
        current = ast.parse(self.source_blob(gate.HELPER, "8f78939ea6401d12060d85814390edf0c4cd7f0b"))
        frozen_inventory = json.loads(self.source_blob(gate.INVENTORY, "8f78939ea6401d12060d85814390edf0c4cd7f0b"))
        frozen_constants = {node.targets[0].id: ast.literal_eval(node.value) for node in current.body
            if isinstance(node, ast.Assign) and node.targets[0].id in ("GATES", "DATABASES", "URLS")}
        frozen_gate = SimpleNamespace(**frozen_constants)
        constants = lambda tree: {node.targets[0].id: node.value for node in tree.body
            if isinstance(node, ast.Assign) and isinstance(node.targets[0], ast.Name)}
        before, after = constants(old), constants(current)
        self.assertEqual(before.keys(), after.keys())
        self.assertEqual(len(ast.literal_eval(before["GATES"])), 82)
        self.assertEqual(tuple(name for name in frozen_gate.GATES if name != "pm_recovery_pg"), ast.literal_eval(before["GATES"]))
        self.assertEqual(frozen_gate.GATES.count("pm_recovery_pg"), 1)
        self.assertEqual(ast.literal_eval(after["DATABASES"]), ast.literal_eval(before["DATABASES"]) + ("fleet_pm_recovery_test",))
        self.assertEqual(ast.literal_eval(after["URLS"]), dict(ast.literal_eval(before["URLS"]), FLEET_PM_RECOVERY_TEST_DATABASE_URL="fleet_pm_recovery_test"))
        self.assertEqual(ast.literal_eval(before["OPENAPI_SHA"]),
                         "e1b17e723abf43866c4f913c9fa4fba8b201bef5e3532b4a8f6cdc32ccbcce76")
        self.assertEqual(ast.literal_eval(after["OPENAPI_SHA"]),
                         "afa46ac37b726232eda73df46c24d1d42c796f8873eefb68454fbe0f243df501")
        for name in before.keys() - {"SOURCE_SHA", "SOURCE_INVENTORY_SHA", "OPENAPI_SHA", "GATES", "DATABASES", "URLS"}:
            self.assertEqual(ast.dump(before[name]), ast.dump(after[name]), name)
        selectors = lambda tree: {node.name for node in ast.walk(tree)
            if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        inherited = selectors(ast.parse(self.source_blob("scripts/tests/test_hosted_backend_gate.py", donor)))
        self.assertEqual(len(inherited), 143)
        self.assertTrue(inherited <= selectors(ast.parse(Path(__file__).read_bytes())))
        functions = lambda tree: {node.name: node for node in tree.body
            if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
        previous, actual = functions(old), functions(current)
        self.assertEqual(actual.keys() - previous.keys(), {"owned_private_ipv4"})
        self.assertFalse(previous.keys() - actual.keys())

        class RestoreCounts(ast.NodeTransformer):
            def visit_Constant(self, node):
                if type(node.value) is int:
                    node.value = {177: 174, 397: 382, 74: 73, 48: 34}.get(node.value, node.value)
                return node

        for name, node in previous.items():
            updated = copy.deepcopy(actual[name])
            if name in ("reviewed_inventory", "verify_runtime_inventory"):
                message = {"reviewed_inventory": "PM recovery selector coverage drift",
                           "verify_runtime_inventory": "PM recovery compiler selectors missing/duplicate/not ignored"}[name]
                additions = [child for child in updated.body if isinstance(child, ast.Expr)
                    and isinstance(child.value, ast.Call) and isinstance(child.value.func, ast.Name)
                    and child.value.func.id == "require" and len(child.value.args) == 2
                    and isinstance(child.value.args[1], ast.Constant) and child.value.args[1].value == message]
                self.assertEqual(len(additions), 1)
                updated.body.remove(additions[0])
            if name == "execute":
                additions = []
                for call in (child for child in ast.walk(updated) if isinstance(child, ast.Call)):
                    additions.extend(keyword.arg for keyword in call.keywords if keyword.arg in (
                        "FLEET_TEST_BASE_UTILITY_CHECKOUT", "FLEET_PM_RECOVERY_TEST_HOST"))
                    call.keywords = [keyword for keyword in call.keywords if keyword.arg not in (
                        "FLEET_TEST_BASE_UTILITY_CHECKOUT", "FLEET_PM_RECOVERY_TEST_HOST")]
                self.assertEqual(sorted(additions), ["FLEET_PM_RECOVERY_TEST_HOST", "FLEET_TEST_BASE_UTILITY_CHECKOUT"])
            self.assertEqual(ast.dump(node), ast.dump(RestoreCounts().visit(updated)), name)
        shell = self.source_blob(gate.GATE, "8f78939ea6401d12060d85814390edf0c4cd7f0b").decode()
        env = "  FLEET_PM_RECOVERY_TEST_DATABASE_URL FLEET_TEST_BASE_UTILITY_CHECKOUT FLEET_PM_RECOVERY_TEST_HOST \\\n"
        block = ('stage=pm_recovery_pg\nFLEET_TEST_DATABASE_URL="$FLEET_PM_RECOVERY_TEST_DATABASE_URL" \\\n'
                 '  cargo test --locked -p infra --lib runtime::pm_recovery::tests::pg:: -- --ignored --test-threads=1 2>&1 | tee "$QA_OUTPUT/$stage.log"\npassed\n')
        self.assertEqual(shell.count(env), 1)
        self.assertEqual(shell.count(block), 1)
        self.assertEqual(shell.replace(env, "").replace(block, ""), self.source_blob(gate.GATE, donor).decode())
        self.assertEqual(self.source_blob(gate.INIT, "8f78939ea6401d12060d85814390edf0c4cd7f0b"), self.source_blob(gate.INIT, donor) + b"CREATE DATABASE fleet_pm_recovery_test;\n")
        previous_inventory = json.loads(self.source_blob(gate.INVENTORY, donor))
        for stage, names in previous_inventory["groups"].items():
            expected = sorted(names + ["chat_controls_hold_each_bound_pending_identity"]) if stage == "foundation" else names
            self.assertEqual(frozen_inventory["groups"][stage], expected, stage)
        for kind, count in (("workspace_default_declarations", 15), ("ignored", 3)):
            identities = lambda value: {(row["source"], row["name"]) for row in value[kind]}
            before_names, after_names = identities(previous_inventory), identities(frozen_inventory)
            self.assertTrue(before_names <= after_names)
            self.assertEqual(len(after_names - before_names), count)
        self.assertEqual(frozen_inventory["migration_registries"], previous_inventory["migration_registries"])
        self.assertEqual(frozen_inventory["authority"], previous_inventory["authority"])

    def test_recovery_three_lib_pg_cases_are_explicit_source_declared_and_fail_closed(self):
        path = "backend/infra/src/runtime/pm_recovery_pg_tests.rs"
        prefix = "runtime::pm_recovery::tests::pg::"
        names = sorted(prefix + name for name in (
            "production_aaa_future_layout_without_constructing_or_polling_runtime",
            "production_lost_ack_reload_reuses_original_journal_and_persists_same_native_ack",
            "production_concurrent_submission_cas_and_ack_keep_one_native_effect",
            "production_unknown_replay_rejects_current_token_context_revocation_before_post"))
        self.assertEqual(REVIEWED["groups"]["pm_recovery_pg"], names)
        source = self.source_blob(path).decode()
        declarations = re.findall(r'#\[(?:tokio::)?test\]\n#\[ignore[^\n]*\]\n(?:async )?fn (\w+)\(', source)
        self.assertEqual(sorted(prefix + name for name in declarations), names)
        records = [row for row in REVIEWED["ignored"] if row["source"] == path]
        self.assertEqual(Counter(row["name"] for row in records), Counter(names))
        for row in records:
            self.assertEqual((row["package"], row["target_kind"], row["target"], row["gate"]),
                             ("infra", "lib", "infra", "pm_recovery_pg"))
        self.assertIn('#[cfg(target_os = "linux")]\n#[path = "pm_recovery_pg_tests.rs"]\nmod pg;',
                      self.source_blob("backend/infra/src/runtime/pm_recovery_tests.rs").decode())
        text = self.log("pm_recovery_pg")
        self.assertEqual(gate.verify_test_log("pm_recovery_pg", text, REVIEWED)["passed"], 4)
        for name in names:
            line = "test " + name + " ... ok\n"
            for bad in (text.replace(line, ""), text + line, text.replace(line, line.replace("ok", "ignored")),
                        text.replace("4 passed", "3 passed")):
                with self.subTest(name=name), self.assertRaises(ValueError):
                    gate.verify_test_log("pm_recovery_pg", bad, REVIEWED)
        ignored = "\n".join(row["name"] + ": test" for row in REVIEWED["ignored"])
        ordinary = ignored + "\n" + "\n".join(name + ": test" for name in REVIEWED["pm_workspace_required"])
        gate.verify_runtime_inventory(ordinary, ignored, REVIEWED)
        for name in names:
            for bad in (ordinary.replace(name + ": test", ""), ordinary + "\n" + name + ": test"):
                with self.subTest(name=name), self.assertRaises(ValueError):
                    gate.verify_runtime_inventory(bad, ignored, REVIEWED)

    def test_recovery_owned_private_ipv4_requires_unique_bindable_rfc1918_not_loopback_or_foreign(self):
        def addresses(*values):
            return [(gate.socket.AF_INET, gate.socket.SOCK_STREAM, 6, "", (value, 0)) for value in values]
        with mock.patch.object(gate.socket, "getaddrinfo", return_value=addresses("172.17.0.2", "172.17.0.2")), \
                mock.patch.object(gate.socket, "socket") as socket:
            self.assertEqual(gate.owned_private_ipv4(), "172.17.0.2")
            socket.return_value.__enter__.return_value.bind.assert_called_once_with(("172.17.0.2", 0))
        for values in (("127.0.0.1",), ("169.254.1.2",), ("8.8.8.8",), ("100.64.1.2",), (),
                       ("10.0.0.1", "192.168.1.2")):
            with self.subTest(values=values), mock.patch.object(gate.socket, "getaddrinfo", return_value=addresses(*values)), \
                    mock.patch.object(gate.socket, "socket"), self.assertRaises(ValueError):
                gate.owned_private_ipv4()
        with mock.patch.object(gate.socket, "getaddrinfo", return_value=addresses("10.0.0.1")), \
                mock.patch.object(gate.socket, "socket") as socket:
            socket.return_value.__enter__.return_value.bind.side_effect = OSError("unassigned address")
            with self.assertRaises(ValueError):
                gate.owned_private_ipv4()

    def test_recovery_runtime_env_uses_qualified_utility_alias_and_owned_database_cleanup(self):
        import ast
        tree = ast.parse((ROOT / gate.HELPER).read_bytes())
        execute = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == "execute")
        keywords = {keyword.arg: ast.unparse(keyword.value) for node in ast.walk(execute)
                    if isinstance(node, ast.Call) for keyword in node.keywords}
        self.assertEqual(keywords["FLEET_TEST_BASE_UTILITY_CHECKOUT"], keywords["QA_UTILITY_CHECKOUT"])
        self.assertEqual(keywords["FLEET_PM_RECOVERY_TEST_HOST"], "owned_private_ipv4()")
        self.assertEqual(gate.database_environment()["FLEET_PM_RECOVERY_TEST_DATABASE_URL"],
                         "postgres://fleet_test@postgres:5432/fleet_pm_recovery_test")
        self.assertEqual(gate.UTILITY_SHA, "9b53de7b23593949a9e6c05bd5a4f94b930e50a0")
        with mock.patch.object(gate, "psql") as psql, mock.patch.object(gate, "database_catalog", return_value=["postgres"]):
            gate.drop_databases(["fleet_pm_recovery_test"])
            self.assertEqual(psql.call_args_list[0].args, ('DROP DATABASE IF EXISTS "fleet_pm_recovery_test" WITH (FORCE)',))
            with self.assertRaises(ValueError):
                gate.drop_databases(["production"])

    def test_recovery_authentic4449_codegen_reuse_requires_exact_api_dependency_closure(self):
        import hashlib
        REVIEWED = json.loads(self.source_blob(gate.INVENTORY, "8f78939ea6401d12060d85814390edf0c4cd7f0b"))
        import tomllib
        frozen = json.loads(self.source_blob(gate.INVENTORY, "b6e933260e9327a29c5a80211411cf24cd04b88d"))
        evidence = frozen["config_union_preparation"]["codegen_evidence"]
        target = frozen["source_commit"]
        source = "4449a3b1cdd915e265543a24054506f15385393d"
        closure = evidence["api_dependency_closure"]
        self.assertEqual(evidence["source_commit"], source)
        self.assertEqual(evidence["artifact_bound_source_commit"], target)
        self.assertEqual(evidence["binding_kind"], "verified_api_dependency_closure_parity")
        self.assertEqual((evidence["run_id"], evidence["run_attempt"], evidence["artifact_id"]),
                         (38048577514, 1, 11667814381))
        self.assertEqual(evidence["workflow_commit"], "eed4bc54149e1b317c5eb3e3e7c628eeaffbdea4")
        self.assertEqual(evidence["artifact_zip_sha256"], "88460dd3e78b1eac14fe224d3c9b2bc1360618c0211d435cd9883c8ffd06b0c6")
        historical_schema = "e1b17e723abf43866c4f913c9fa4fba8b201bef5e3532b4a8f6cdc32ccbcce76"
        self.assertEqual(evidence["schema_sha256"], historical_schema)
        self.assertEqual(set(evidence["source_delta"]), {"backend/infra/src/runtime/pm_recovery_pg_tests.rs",
            "backend/infra/src/runtime/pm_recovery_tests.rs", "docs/CURRENT_STATE.md", "docs/GAP_REGISTER.md",
            "docs/TESTING.md", "docs/contracts/CHAT_CLARIFICATION_CONTRACT.md"})
        git = lambda *args: subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), *args],
            capture_output=True, check=True, timeout=30).stdout
        self.assertEqual(git("diff", "--name-only", source, target).decode().splitlines(), evidence["source_delta"])
        self.assertEqual(set(closure), {".base-revision", "backend/.cargo", "backend/Cargo.toml", "backend/Cargo.lock",
            "backend/api", "backend/app", "backend/domain", "backend/shared"} | {
            "backend/" + member + "/Cargo.toml" for member in ("api", "app", "domain", "shared", "infra", "migration", "server", "cli")})
        listings = [git("ls-tree", "-r", "-z", pin, "--", *closure) for pin in (source, target)]
        self.assertTrue(listings[0])
        self.assertEqual(listings[0], listings[1])
        self.assertEqual(hashlib.sha256(listings[0]).hexdigest(), evidence["api_dependency_closure_git_sha256"])
        self.assertEqual(gate.digest(self.source_blob("openapi/openapi.json", target)), evidence["schema_sha256"])
        reached, pending, base = set(), ["api"], set()
        while pending:
            package = pending.pop()
            if package in reached:
                continue
            reached.add(package)
            manifest = tomllib.loads(self.source_blob("backend/" + package + "/Cargo.toml", target).decode())
            self.assertNotIn("build", manifest["package"])
            for dependency in manifest.get("dependencies", {}).values():
                if isinstance(dependency, dict) and "path" in dependency:
                    path = dependency["path"]
                    if path.startswith("../../../services-base/crates/"):
                        base.add(path.rsplit("/", 1)[-1])
                    else:
                        self.assertRegex(path, r"^\.\./[a-z]+$")
                        pending.append(path[3:])
        self.assertEqual(reached, {"api", "app", "domain", "shared"})
        self.assertEqual(base, {"sdlc-shared", "sdlc-auth-core", "sdlc-telemetry"})
        for path in git("ls-tree", "-r", "--name-only", target, "--", "backend").decode().splitlines():
            if path.endswith("/build.rs") or "rust-toolchain" in path or path.endswith("/Cargo.toml"):
                self.assertIn(path, closure)
        with mock.patch.object(gate, "OPENAPI_SHA", historical_schema):
            gate.require_codegen_binding(frozen)
        pending = json.loads(self.source_blob(gate.INVENTORY, "e7a357f895617b057840a8c0d42d8552e2425d66"))
        self.assertEqual(pending["openapi_binding"], dict(status="pending_authentic_codegen", sha256=None))
        self.assertIsNone(pending["config_union_preparation"]["codegen_evidence"])
        with self.assertRaisesRegex(ValueError, "binding is pending"):
            gate.require_codegen_binding(pending)
        self.assertEqual(REVIEWED["config_union_preparation"]["prior_recovery_codegen_evidence"], evidence)
        qualified = json.loads(self.source_blob(gate.INVENTORY, "566a5db30842e43e6552d8da8c2138145cf7ed71"))
        self.assertEqual(qualified["config_union_preparation"]["codegen_evidence"], dict(
            source_commit=qualified["source_commit"], artifact_bound_source_commit=qualified["source_commit"],
            workflow_commit="91a1c48c7a3f0cfad33586fb55c45f47ff864bc2", run_id=38052082418,
            run_attempt=1, artifact_id=11669374937,
            artifact_zip_sha256="8283b599e66dc66d5a05e961ae2b59bc5dffc1c408f7d2f35352608198730701",
            reported_by_parent=True, schema_sha256=historical_schema,
            source_file_count=303,
            source_inventory_sha256="4ab1c70324a3b184096a69ed1dbdd72bcbf5f993fbdc1ecc6b01fb643f650457",
            source_tree="b975beb628bb68c03fb5bce45e7c43085837e41f"))
        self.assertEqual(REVIEWED["config_union_preparation"]["prior_exact_source_codegen_evidence"],
                         qualified["config_union_preparation"]["codegen_evidence"])
        self.assertEqual(REVIEWED["openapi_binding"], dict(status="verified", sha256=gate.OPENAPI_SHA))
        self.assertEqual(REVIEWED["config_union_preparation"]["codegen_evidence"], dict(
            source_commit=REVIEWED["source_commit"], artifact_bound_source_commit=REVIEWED["source_commit"],
            workflow_commit="e8fd29e4d45c749ac601d743a09400867f9635e6", run_id=38058114502,
            run_attempt=1, artifact_id=11671964606,
            artifact_zip_sha256="d209ef81e0f962dae8f1465852a0b06dcb6faacdd350ff85b5fda95a980c75cf",
            reported_by_parent=True, schema_sha256=gate.OPENAPI_SHA, source_file_count=303,
            source_inventory_sha256="95ab4d1ff7b20675b38dfdab686360ab6348bde52409b92849c60475b35a5546",
            source_tree="67ea7aa66bf66f803226abbd4893be7517ca3317"))
        self.assertFalse(REVIEWED["config_union_preparation"]["authentic_union_codegen_pending"])
        self.assertFalse(REVIEWED["config_union_preparation"]["compiled_inventory_includes_pre_regen_schema"])
        gate.require_codegen_binding(REVIEWED)

    def synthetic_bound_inventory(self):
        return dict(REVIEWED, openapi_binding=dict(status="verified", sha256=gate.OPENAPI_SHA))

    def source_blob(self, path, pin=None):
        return subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "show",
            (pin or gate.SOURCE_SHA) + ":" + path], capture_output=True, check=True, timeout=30).stdout


    def test_expansion_preserves_all_previous74_stages_and_case_identities(self):
        donor = "a7d7db205ee2ef7e480b5dc559156ad373135180"
        previous = json.loads(self.source_blob(gate.INVENTORY, donor))
        import ast
        tree = ast.parse(self.source_blob(gate.HELPER, donor))
        old_stages = next(ast.literal_eval(n.value) for n in tree.body if isinstance(n, ast.Assign)
            and any(isinstance(t, ast.Name) and t.id == "GATES" for t in n.targets))
        self.assertEqual(len(old_stages), 74)
        self.assertEqual(tuple(s for s in gate.GATES if s in old_stages), old_stages)
        rename = {"readiness_http_does_not_trust_database_only_effective_revision":
                  "config_revision_readiness_http_uses_exact_heads_without_trusting_database_only_files"}
        for stage, names in previous["groups"].items():
            expected = [rename.get(name, name) for name in names]
            if stage in ("foundation", "container_activation_pg", "container_activation_intent", "credentials_unit", "credentials_pg", "real_auth", "lineage10", "config_shared_unit"):
                self.assertTrue(set(expected) <= set(REVIEWED["groups"][stage]), stage)
            else:
                self.assertEqual(REVIEWED["groups"][stage], expected, stage)
        identity = lambda x: (x["package"], x["source"], x["name"])
        self.assertTrue({identity(x) for x in previous["ignored"]} <= {identity(x) for x in REVIEWED["ignored"]})
        self.assertTrue({identity(dict(x, name=rename.get(x["name"], x["name"]))) for x in previous["workspace_default_declarations"]}
            <= {identity(x) for x in REVIEWED["workspace_default_declarations"]})
        for path, value in previous["compiled_source_sha256"].items():
            if not path.startswith("fleet-control/"):
                self.assertEqual(REVIEWED["compiled_source_sha256"][path], value)
        self.assertEqual(REVIEWED["authority"], previous["authority"])

    def test_human_successor_preserves_all135_selectors_and_old81_ordered_stages(self):
        import ast
        donor = "0d1e5a4361610c0a0728137731f54fa5ab12c481"
        old = ast.parse(self.source_blob(gate.HELPER, donor))
        new = ast.parse(self.source_blob(gate.HELPER, "9565ecc1c2d114d44132598d77f4c5942d440d1d"))
        stages = next(ast.literal_eval(node.value) for node in old.body if isinstance(node, ast.Assign)
            and any(isinstance(target, ast.Name) and target.id == "GATES" for target in node.targets))
        self.assertEqual(len(stages), 81)
        self.assertEqual(tuple(stage for stage in gate.GATES if stage not in ("pm_human_controls", "pm_recovery_pg", "pm_ack_migration")), stages)
        self.assertEqual(gate.GATES.count("pm_human_controls"), 1)
        selectors = lambda tree: {node.name for node in ast.walk(tree)
            if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        inherited = selectors(ast.parse(self.source_blob("scripts/tests/test_hosted_backend_gate.py", donor)))
        self.assertEqual(len(inherited), 135)
        self.assertTrue(inherited <= selectors(ast.parse(Path(__file__).read_bytes())))
        functions = lambda tree: {node.name: node for node in tree.body
            if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
        before, after = functions(old), functions(new)
        self.assertEqual(before.keys(), after.keys())
        changed = {"reviewed_inventory", "expected_migration_receipt", "verify_migration_snapshots",
                   "verify_runtime_inventory", "verify_test_log", "execute", "validate_evidence_files"}
        for name in before.keys() - changed:
            self.assertEqual(ast.dump(before[name]), ast.dump(after[name]), name)
        previous = json.loads(self.source_blob(gate.INVENTORY, donor))
        for stage, names in previous["groups"].items():
            expected = sorted(names + ["chat_controls_hold_each_bound_pending_identity",
                "pm_dispatch::pm_publication_claim_requires_exact_run_instruction_receipt",
                "pm_events::pm_http_disabled_dispatch_denies_foreign_pins_and_unknown_ack",
                "pm_events::pm_http_disabled_dispatch_recovers_accepted_sse_and_terminal_once"]) if stage == "foundation" else names
            self.assertEqual(REVIEWED["groups"][stage], expected, stage)
        self.assertEqual((ROOT / gate.INIT).read_bytes().replace(b"CREATE DATABASE fleet_pm_recovery_test;\n", b"").replace(
            b"CREATE DATABASE fleet_pm_ack_migration_test;\n", b""), self.source_blob(gate.INIT, donor))

    def test_human_five_pg_http_selectors_are_once_ignored_and_mandatory(self):
        path = "backend/infra/tests/support/pm_human_controls.rs"
        source = self.source_blob(path).decode()
        declared = re.findall(r'#\[tokio::test\]\n#\[ignore = "requires isolated FLEET_TEST_DATABASE_URL"\]\nasync fn (\w+)\(', source)
        names = ["pm_human_controls::" + name for name in declared]
        self.assertEqual(len(names), 6)
        self.assertEqual(sorted(names), REVIEWED["groups"]["pm_human_controls"])
        records = [row for row in REVIEWED["ignored"] if row["source"] == path]
        self.assertEqual(Counter(row["name"] for row in records), Counter(names))
        for row in records:
            self.assertEqual((row["package"], row["target_kind"], row["target"], row["gate"]),
                             ("infra", "test", "sdlc_foundation", "pm_human_controls"))
        self.assertIn("mod pm_human_controls;", self.source_blob("backend/infra/tests/sdlc_foundation.rs").decode())
        text = self.log("pm_human_controls")
        gate.verify_test_log("pm_human_controls", text, REVIEWED)
        for name in names:
            line = "test " + name + " ... ok\n"
            for bad in (text.replace(line, ""), text + line, text.replace(line, line.replace("ok", "ignored")),
                        text.replace("6 passed", "5 passed")):
                with self.subTest(name=name), self.assertRaises(ValueError):
                    gate.verify_test_log("pm_human_controls", bad, REVIEWED)
        shell = (ROOT / gate.GATE).read_text()
        command = 'cargo test --locked -p infra --test sdlc_foundation pm_human_controls:: -- --ignored --test-threads=1'
        self.assertEqual(shell.count(command), 1)

    def test_human_compiler_inventory_requires_five_ignored_and_all34_ordinary_pm_cases(self):
        previous = json.loads(self.source_blob(gate.INVENTORY, "0d1e5a4361610c0a0728137731f54fa5ab12c481"))
        self.assertEqual(len(previous["pm_workspace_required"]), 33)
        inherited_and_human = set(previous["pm_workspace_required"]) | set(REVIEWED["groups"]["pm_human_controls"])
        self.assertEqual(len(inherited_and_human), 39)
        self.assertTrue(inherited_and_human <= (set(REVIEWED["pm_workspace_required"])
                                               | set(REVIEWED["groups"]["pm_human_controls"])))
        frozen = json.loads(self.source_blob(gate.INVENTORY, "9565ecc1c2d114d44132598d77f4c5942d440d1d"))
        self.assertEqual(set(frozen["pm_workspace_required"]) - set(previous["pm_workspace_required"]), {
            "runtime::pm_tools::tests::task_binding_matches_draft_identity_only_with_every_field_and_original_agent"})
        ignored = "\n".join(row["name"] + ": test" for row in REVIEWED["ignored"])
        ordinary = ignored + "\n" + "\n".join(name + ": test" for name in REVIEWED["pm_workspace_required"])
        gate.verify_runtime_inventory(ordinary, ignored, REVIEWED)
        for name in REVIEWED["groups"]["pm_human_controls"]:
            line = name + ": test"
            for bad in (ordinary.replace(line, ""), ordinary + "\n" + line):
                with self.subTest(name=name), self.assertRaises(ValueError):
                    gate.verify_runtime_inventory(bad, ignored, REVIEWED)

    def test_human023_down_empty_history_and_durable_custody_refusal_stay_required(self):
        migration = self.source_blob("backend/migration/src/m20261010_000023_pm_human_controls.rs").decode()
        tests = self.source_blob("backend/infra/tests/support/pm_human_controls.rs").decode()
        self.assertIn("PM custody history prevents human controls downgrade", migration)
        self.assertIn("continuation_state <> 'not_required'", migration)
        self.assertIn("hermes_dispatch_journal j WHERE j.run_id=c.session_run_id", migration)
        self.assertEqual(tests.count("assert_human_controls_downgrade_refused().await;"), 2)
        self.assertIn('assert!(error.contains("PM custody history prevents human controls downgrade"))', tests)
        self.assertIn("assert_eq!(before, after)", tests)
        dispatch = self.source_blob("backend/infra/tests/support/pm_dispatch.rs").decode()
        self.assertIn('"m20261010_000022_pm_dispatch"', dispatch)
        self.assertIn('error.contains("PM dispatch custody prevents downgrade")', dispatch)
        expected = gate.expected_migration_receipt(REVIEWED)
        for stage in ("down_human", "human_reapply"):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.migration_files(root, expected)
                ledger = root / ("migration-" + stage + "-ledger.tsv")
                ledger.write_text(ledger.read_text().replace("\t1000\n", "\t1001\n", 1), newline="\n")
                with self.assertRaises(ValueError):
                    gate.verify_migration_snapshots(root, REVIEWED)

    def test_human_current_codegen_and_final_source_pending_cannot_publish_success(self):
        pending = json.loads(self.source_blob(gate.INVENTORY, "6b3a8e67f95861cbad921ea0296f84e492554d63"))
        self.assertEqual(pending["openapi_binding"], dict(status="pending_authentic_codegen", sha256=None))
        preparation = pending["config_union_preparation"]
        self.assertIsNone(preparation["codegen_evidence"])
        for name in ("authentic_union_codegen_pending", "final_qualified_source_pending", "compiled_inventory_includes_pre_regen_schema"):
            self.assertIs(preparation[name], True)
        with mock.patch.object(gate, "hosted_identity", return_value=(Path("owned"), "a" * 40)), \
                mock.patch.object(gate, "clean_head"), mock.patch.object(gate, "reviewed_inventory", return_value=pending), \
                mock.patch.object(gate, "git") as git, mock.patch.object(gate.subprocess, "Popen") as spawn:
            with self.assertRaisesRegex(ValueError, "binding is pending"):
                gate.preflight()
            git.assert_not_called()
            spawn.assert_not_called()
        run, artifact, payload, args = self.artifact()
        with mock.patch.object(gate, "reviewed_inventory", return_value=pending):
            with self.assertRaisesRegex(ValueError, "binding is pending"):
                gate.validate_readback(run, artifact, payload, **args)

    def test_human_final_sql_source_delta_is_exact_seven_paths_and_preserves_codegen_api_inputs(self):
        previous = "31ab4e90b77f389b7bc5f6cfaf5f4b3d38f2d75e"
        sql_source = "dd5744e34cfd6d69dc7cffad882bdd635a887253"
        delta = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "diff", "--name-only",
            previous, sql_source], capture_output=True, check=True, timeout=30).stdout.decode().splitlines()
        self.assertEqual(set(delta), {"backend/infra/tests/support/pm_human_controls.rs",
            "backend/migration/src/m20261010_000023_pm_human_controls.rs", "docs/CURRENT_STATE.md",
            "docs/GAP_REGISTER.md", "docs/OPERATIONS.md", "docs/SECURITY.md", "docs/TESTING.md"})
        roots = ("backend/api", "backend/app", "backend/domain", "backend/shared", "backend/infra/src",
                 "backend/Cargo.lock", "backend/Cargo.toml", ".base-revision", "openapi")
        stable = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "diff", "--name-only",
            previous, sql_source, "--", *roots], capture_output=True, check=True, timeout=30).stdout
        self.assertEqual(stable, b"")
        self.assertEqual(self.source_blob("openapi/openapi.json", sql_source), self.source_blob("openapi/openapi.json", previous))
        # Historical API-input parity alone could not authenticate the pending producer artifact.
        with self.assertRaisesRegex(ValueError, "binding is pending"):
            gate.require_codegen_binding(json.loads(self.source_blob(gate.INVENTORY, "6b3a8e67f95861cbad921ea0296f84e492554d63")))

    def test_human_typed_api_successor_requires_new_codegen_and_exact_two_path_delta(self):
        previous = "dd5744e34cfd6d69dc7cffad882bdd635a887253"
        api_source = "f7d586be10a958f4f454c357831018250c779256"
        delta = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "diff", "--name-only",
            previous, api_source], capture_output=True, check=True, timeout=30).stdout.decode().splitlines()
        self.assertEqual(set(delta), {"backend/api/src/routes/task_chats.rs",
                                     "backend/infra/tests/support/pm_credential_creation.rs"})
        parents = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "rev-list", "--parents",
            "-n", "1", api_source], capture_output=True, check=True, timeout=30).stdout.decode().split()
        self.assertEqual(parents, [api_source, previous])
        owner = self.source_blob("backend/api/src/routes/task_chats.rs").decode().split(
            "pub(super) async fn require_pm_owner(", 1)[1].split("pub async fn ", 1)[0]
        for field in ("tracker_instance_id", "project_id", "task_id", "root_task_id", "owner_subject"):
            self.assertIn("identity." + field + " != binding." + field, owner)
        self.assertIn("pm.request.agent_id != binding.agent_id", owner)
        self.assertNotIn("pm.identity()? != binding", owner)
        credential = self.source_blob("backend/infra/tests/support/pm_credential_creation.rs").decode()
        self.assertIn(".get_task_chat_binding(op.session_id.unwrap())", credential)
        pending = json.loads(self.source_blob(gate.INVENTORY, "6b3a8e67f95861cbad921ea0296f84e492554d63"))
        self.assertEqual(pending["openapi_binding"], dict(status="pending_authentic_codegen", sha256=None))
        with self.assertRaisesRegex(ValueError, "binding is pending"):
            gate.require_codegen_binding(pending)

    def test_human_authentic_f7_codegen_binds_only_exact_doc_schema_successor_with_backend_parity(self):
        frozen = json.loads(self.source_blob(gate.INVENTORY, "9565ecc1c2d114d44132598d77f4c5942d440d1d"))
        frozen_source = frozen["source_commit"]
        source = "f7d586be10a958f4f454c357831018250c779256"
        delta = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "diff", "--name-only",
            source, frozen_source], capture_output=True, check=True, timeout=30).stdout.decode().splitlines()
        self.assertEqual(set(delta), {"docs/CURRENT_STATE.md", "docs/GAP_REGISTER.md", "openapi/openapi.json"})
        listings = [subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "ls-tree", "-r", "-z",
            pin, "--", "backend", ".base-revision"], capture_output=True, check=True, timeout=30).stdout
            for pin in (source, frozen_source)]
        self.assertTrue(listings[0])
        self.assertEqual(listings[0], listings[1])
        preparation = frozen["config_union_preparation"]
        for name in ("authentic_union_codegen_pending", "final_qualified_source_pending", "compiled_inventory_includes_pre_regen_schema"):
            self.assertIs(preparation[name], False)
        self.assertEqual(preparation["codegen_evidence"], dict(
            source_commit=source, workflow_commit="6f86fca225e4390847c99401f63b519e4dc34162",
            run_id=38044630680, run_attempt=1, artifact_id=11667646855,
            artifact_zip_sha256="1d7866097b944b8aa67abf25a16a970c20840fd1b9f2e0a3fba96ab5deceacac",
            schema_sha256="e1b17e723abf43866c4f913c9fa4fba8b201bef5e3532b4a8f6cdc32ccbcce76",
            artifact_bound_source_commit=frozen_source, reported_by_parent=True,
            binding_kind="verified_code_parity", source_delta=sorted(delta)))
        self.assertNotEqual(source, frozen_source)  # Artifact was produced on f7, not directly on d458.
        historical_schema = "e1b17e723abf43866c4f913c9fa4fba8b201bef5e3532b4a8f6cdc32ccbcce76"
        self.assertEqual(frozen["openapi_binding"], dict(status="verified", sha256=historical_schema))
        self.assertEqual(gate.digest(self.source_blob("openapi/openapi.json", frozen_source)), historical_schema)
        with mock.patch.object(gate, "OPENAPI_SHA", historical_schema):
            gate.require_codegen_binding(frozen)
        for old in ("ad980604beb2cff0890f4d1a07a185c97a444fda166985f2a6da465a222d129c",
                    "1167220ea9f3d65ddca4cce1112a26d53c77f8c1684ef958859f737f20210953"):
            with mock.patch.object(gate, "OPENAPI_SHA", old), self.assertRaises(ValueError):
                gate.require_codegen_binding(frozen)

    def test_successor_preserves_all_frozen_controls_gates_guards_and_input_pins(self):
        # Keep this historical transition bound to its reviewed frozen controls.
        historical = "50cb550af47e06530997fc1f84a5562fd4503b9e"
        REVIEWED = json.loads(self.source_blob(gate.INVENTORY, historical))
        import ast
        donor = "084d9f0f7b94953251b58a912b32b92cedbda020"
        old = ast.parse(self.source_blob(gate.HELPER, donor))
        current = ast.parse(self.source_blob(gate.HELPER, "9565ecc1c2d114d44132598d77f4c5942d440d1d"))
        constants = lambda tree: {node.targets[0].id: node.value for node in tree.body
            if isinstance(node, ast.Assign) and isinstance(node.targets[0], ast.Name)}
        before, after = constants(old), constants(current)
        self.assertEqual(before.keys(), after.keys())
        for name in before.keys() - {"SOURCE_SHA", "SOURCE_INVENTORY_SHA", "OPENAPI_SHA"}:
            actual = copy.deepcopy(after[name])
            if name == "GATES":
                actual.elts = [node for node in actual.elts if node.value != "pm_human_controls"]
            self.assertEqual(ast.dump(before[name]), ast.dump(actual), name)
        self.assertEqual(len(gate.GATES), 84)

        class Counts(ast.NodeTransformer):
            def visit_Constant(self, node):
                if type(node.value) is int and node.value == 174:
                    node.value = 168
                if type(node.value) is int and node.value == 124:
                    node.value = 119
                return node

        old_functions = {n.name: n for n in old.body if isinstance(n, (ast.FunctionDef, ast.ClassDef))}
        new_functions = {n.name: n for n in current.body if isinstance(n, (ast.FunctionDef, ast.ClassDef))}
        self.assertEqual(old_functions.keys(), new_functions.keys())
        for name, before_node in old_functions.items():
            actual = copy.deepcopy(new_functions[name])
            if name in ("reviewed_inventory", "verify_runtime_inventory"):
                new_guard = {"reviewed_inventory": "PM workspace selector coverage drift",
                             "verify_runtime_inventory": "PM compiler selectors missing/duplicate/ignored"}[name]
                human_guard = {"reviewed_inventory": "PM human selector coverage drift",
                               "verify_runtime_inventory": "PM human compiler selectors missing/duplicate/not ignored"}[name]
                added = [node for node in actual.body if isinstance(node, ast.Expr)
                         and isinstance(node.value, ast.Call) and isinstance(node.value.func, ast.Name)
                         and node.value.func.id == "require" and len(node.value.args) == 2
                         and isinstance(node.value.args[1], ast.Constant) and node.value.args[1].value in (new_guard, human_guard)]
                self.assertEqual(len(added), 2)
                for node in added:
                    actual.body.remove(node)  # Additive guards are independently exercised below.
            if name == "expected_migration_receipt":
                continue  # Independently tested old boundaries plus alias and PM boundaries.
            if name == "reviewed_inventory":
                for node in ast.walk(actual):
                    if isinstance(node, ast.Compare) and len(node.comparators) == 1:
                        spelling = ast.unparse(node.left)
                        changes = {"len(value['groups']['foundation'])": 58,
                                   "len(value['workspace_default_declarations'])": 344,
                                   "len(value['groups']['container_activation_pg'])": 13,
                                   "len(value['migration_registries']['canonical'])": 21,
                                   "len(value['migration_registries']['split'])": 24}
                        if spelling in changes:
                            node.comparators[0] = ast.Constant(changes[spelling])
                for node in ast.walk(actual):
                    if isinstance(node, ast.keyword) and node.arg in ("credentials_pg", "config_shared_unit"):
                        node.value = ast.Constant({"credentials_pg": 15, "config_shared_unit": 10}[node.arg])
            if name == "verify_migration_snapshots":
                old_loop = next(n for n in before_node.body if isinstance(n, ast.For)
                                and ast.unparse(n.target) == "(before, after)")
                new_loop = next(n for n in actual.body if isinstance(n, ast.For)
                                and ast.unparse(n.target) == "(before, after)")
                new_loop.iter = copy.deepcopy(old_loop.iter)
            self.assertEqual(ast.dump(before_node), ast.dump(Counts().visit(actual)), name)
        selectors = lambda tree: {n.name for n in ast.walk(tree)
            if isinstance(n, ast.FunctionDef) and n.name.startswith("test_")}
        for pin, count in (("6f648430a43ffbf021c804648fd05f27e2268bdb", 110),
                           ("1200321d90a914a0f809dc5fe19db328b8f813fb", 116),
                           ("9e7fb08a8d1bd72539d5b0b04c85f8714c91b048", 120),
                           ("084d9f0f7b94953251b58a912b32b92cedbda020", 126)):
            previous = selectors(ast.parse(self.source_blob("scripts/tests/test_hosted_backend_gate.py", pin)))
            self.assertEqual(len(previous), count)
            self.assertTrue(previous <= selectors(ast.parse(self.source_blob("scripts/tests/test_hosted_backend_gate.py", historical))))
        self.assertEqual(self.source_blob(gate.INIT, historical).replace(b"CREATE DATABASE fleet_pm_recovery_test;\n", b"").replace(
            b"CREATE DATABASE fleet_pm_ack_migration_test;\n", b""), self.source_blob(gate.INIT, donor))

    def test_successor_credential_additions_are_source_declared_and_never_silently_ignored(self):
        previous = json.loads(self.source_blob(gate.INVENTORY, "6f648430a43ffbf021c804648fd05f27e2268bdb"))
        identity = lambda row: (row["package"], row["source"], row["name"])
        for kind, added in (("workspace_default_declarations", 44), ("ignored", 7)):
            frozen = json.loads(self.source_blob(gate.INVENTORY, "9565ecc1c2d114d44132598d77f4c5942d440d1d"))
            before, after = ({identity(row) for row in value[kind]} for value in (previous, frozen))
            self.assertTrue(after <= {identity(row) for row in REVIEWED[kind]})
            self.assertTrue(before <= after)
            self.assertEqual(len(after - before), added)
        pg_path = "backend/infra/tests/support/pm_credential_creation.rs"
        before = set(previous["groups"]["credentials_pg"])
        new_pg = set(REVIEWED["groups"]["credentials_pg"]) - before
        self.assertEqual(len(new_pg), 6)
        for record in REVIEWED["workspace_default_declarations"]:
            if record["source"] == pg_path and "pm_credential_creation::" + record["name"] in new_pg:
                self.assertIs(record["ignored"], False)
                self.assertNotIn(record["name"], {row["name"].rsplit("::", 1)[-1] for row in REVIEWED["ignored"]})
        for stage, names in previous["groups"].items():
            expected = set(names)
            if stage in ("foundation", "credentials_pg"):
                expected |= new_pg
                if stage == "foundation":
                    expected.add("message_receipts_and_replay_are_independent_of_history_limit")
                    expected.add("chat_controls_hold_each_bound_pending_identity")
                    expected |= {"pm_dispatch::" + row["name"] for row in REVIEWED["workspace_default_declarations"]
                                 if row["source"] == "backend/infra/tests/support/pm_dispatch.rs"}
                    expected |= {"pm_events::" + row["name"] for row in REVIEWED["workspace_default_declarations"]
                                 if row["source"] == "backend/infra/tests/support/pm_events.rs"}
            elif stage == "lineage10":
                expected.add("lineage_tests::empty_pm_custody_downgrade_and_reupgrade_preserve_older_lineage")
            elif stage == "config_shared_unit":
                expected.add("config::tests::pm_dispatch_is_opt_in_and_does_not_serialize_machine_tokens")
            elif stage == "container_activation_pg":
                expected.add("recovered_authority_alias_repair_preserves_trigger_oid_and_exact_custody")
            elif stage == "credentials_unit":
                expected.add("pm_credentials::coordinator::tests::introspection_rejects_wrong_subject_and_non_exact_parent_or_child_scopes")
            elif stage == "real_auth":
                expected.add("real_base_expired_children_replay_without_minting_and_are_rejected_by_fleet")
            self.assertEqual(set(REVIEWED["groups"][stage]), expected, stage)
        self.assertEqual({stage: len(REVIEWED["groups"][stage]) for stage in (
            "foundation", "credentials_pg", "credentials_unit", "real_auth")},
            dict(foundation=77, credentials_pg=16, credentials_unit=8, real_auth=2))
        self.assertIn("#[cfg(test)]\nmod tests;", self.source_blob("backend/shared/src/id.rs").decode())
        self.assertEqual(sum(row["source"] == "backend/shared/src/id/tests.rs" and row["name"] == "new_ids_are_plain_uuids"
                             for row in REVIEWED["workspace_default_declarations"]), 1)
        for stage in ("credentials_pg", "credentials_unit", "real_auth", "foundation"):
            text = self.log(stage, 125 if stage == "foundation" else 0)
            gate.verify_test_log(stage, text, REVIEWED)
            for bad in (text.replace(" ... ok", " ... ignored", 1), "\n".join(text.splitlines()[1:]),
                        text + text.splitlines()[0] + "\n", text.replace("0 failed", "1 failed")):
                with self.subTest(stage=stage), self.assertRaises(ValueError):
                    gate.verify_test_log(stage, bad, REVIEWED)

    def test_successor_source_tree_and_six_lf_controls_remain_closed(self):
        self.assertEqual(gate.SOURCE_SHA, "8f8e69d637a64b2d7a3e8c2bc6dca00517539667")
        tree = subprocess.run(["git", "-C", str(ROOT), "rev-parse", gate.SOURCE_SHA + "^{tree}"],
            capture_output=True, check=True, timeout=30).stdout.decode().strip()
        self.assertEqual(tree, "8738d803e309b0bd0479e1dd624186b94c1704c7")
        delta = subprocess.run(["git", "-C", str(ROOT), "diff", "--name-status", gate.SOURCE_SHA],
            capture_output=True, check=True, timeout=30).stdout.decode()
        gate.validate_delta(delta)
        self.assertEqual(len(REVIEWED["compiled_source_sha256"]), 404)
        self.assertEqual(len(REVIEWED["rust_source_sha256"]), 191)
        previous3445 = json.loads(self.source_blob(gate.INVENTORY, "d4c56055b74a1dd4efdb487f9fc5f6407852f075"))
        identity = lambda row: (row["source"], row["name"])
        self.assertEqual({identity(row) for row in REVIEWED["workspace_default_declarations"]}
                         - {identity(row) for row in previous3445["workspace_default_declarations"]},
                         {('backend/infra/tests/support/pm_dispatch.rs', 'pm_publication_claim_requires_exact_run_instruction_receipt'),
                          ('backend/infra/src/runtime/pm_readback.rs', 'completed_requires_the_shared_terminal_flags')})
        groups = copy.deepcopy(previous3445["groups"])
        new_case = "pm_dispatch::pm_publication_claim_requires_exact_run_instruction_receipt"
        groups["foundation"] = sorted(groups["foundation"] + [new_case])
        groups["pm_recovery_pg"] = REVIEWED["groups"]["pm_recovery_pg"]
        groups["pm_human_controls"] = sorted(groups["pm_human_controls"] + [
            "pm_human_controls::pm_stop_custody_migration_preserves_original_function_and_holds_unsafe_downgrade"])
        self.assertEqual(REVIEWED["groups"], groups)
        self.assertEqual(set(REVIEWED["pm_workspace_required"])
                         - set(previous3445["pm_workspace_required"]), {new_case,
                            "runtime::pm_readback::tests::completed_requires_the_shared_terminal_flags"})
        self.assertEqual(REVIEWED["migration_registries"], {key: names + ["m20261011_000025_pm_stop_custody"]
            for key, names in previous3445["migration_registries"].items()})
        gate.require_codegen_binding(REVIEWED)
        preparation = REVIEWED["config_union_preparation"]
        self.assertEqual(preparation["prior_3445_codegen_evidence"],
                         previous3445["config_union_preparation"]["codegen_evidence"])
        # Original artifact identity remains aa11; only the attested target changes.
        original = json.loads(self.source_blob(gate.INVENTORY,
            "fe1fb3f5f863b8dad9dcd400ee2dccb68e69c4c4"))["config_union_preparation"]["codegen_evidence"]
        closure = [".base-revision", "backend/.cargo", "backend/Cargo.toml", "backend/Cargo.lock",
            "backend/api", "backend/app", "backend/domain", "backend/shared"] + [
            "backend/" + member + "/Cargo.toml"
            for member in ("api", "app", "domain", "shared", "infra", "migration", "server", "cli")]
        prior_controls = "78d3727e196ed17af3af371e3a56936e556cd7e8"
        prior = json.loads(self.source_blob(gate.INVENTORY, prior_controls))
        prior_source = prior["source_commit"]
        new_delta = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "diff",
            "--name-only", prior_source, gate.SOURCE_SHA], capture_output=True, check=True, timeout=30).stdout.decode().splitlines()
        changed_rust = {"backend/infra/src/runtime/pm_recovery_pg_tests.rs", "backend/infra/src/pm_controls.rs",
            "backend/infra/src/runtime/pm_readback.rs", "backend/infra/tests/sdlc_foundation.rs",
            "backend/infra/tests/support/pm_credential_creation.rs", "backend/infra/tests/support/pm_human_controls.rs",
            "backend/infra/tests/support/runtime_stream_bounds.rs", "backend/migration/src/lib.rs",
            "backend/migration/src/lineage_tests.rs", "backend/migration/src/m20261011_000025_pm_stop_custody.rs"}
        self.assertEqual({path for path in new_delta if path.startswith("backend/")}, changed_rust)
        self.assertTrue(all(path in changed_rust or path == "README.md" or path.startswith("docs/") for path in new_delta))
        source_delta = sorted(set(prior["config_union_preparation"]["codegen_evidence"]["source_delta"]) | set(new_delta))
        source = original["source_commit"]
        listing = lambda pin: subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT),
            "ls-tree", "-r", "-z", pin, "--", *closure], capture_output=True, check=True, timeout=30).stdout
        accepted, current = listing(source), listing(gate.SOURCE_SHA)
        self.assertTrue(accepted)
        self.assertEqual(accepted, current)
        self.assertEqual(gate.digest(accepted), "721d0adcd6a5c08d362ea5e25535173333aeac74173608009184818fc30c4159")
        delta = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "diff",
            "--name-only", source, gate.SOURCE_SHA], capture_output=True, check=True, timeout=30).stdout.decode().splitlines()
        self.assertEqual(delta, source_delta)
        self.assertEqual(preparation["codegen_evidence"], dict(original,
            artifact_bound_source_commit=gate.SOURCE_SHA, binding_kind="verified_api_dependency_closure_parity",
            api_dependency_closure=closure, api_dependency_closure_git_sha256=gate.digest(accepted),
            source_delta=source_delta))
        changed_compiled = {path for path, value in REVIEWED["compiled_source_sha256"].items()
                            if value != prior["compiled_source_sha256"].get(path)}
        changed_fingerprints = {path for path, value in REVIEWED["rust_source_sha256"].items()
                               if value != prior["rust_source_sha256"].get(path)}
        self.assertEqual(changed_compiled, {"fleet-control/" + path for path in changed_rust})
        self.assertEqual(changed_fingerprints, changed_rust)
        changed_rust = "backend/infra/src/runtime/pm_recovery_pg_tests.rs"
        body = self.source_blob(changed_rust, prior_source)
        self.assertEqual(body.count(b"let f = fixture(true, false).await;"), 2)
        self.assertEqual(body.count(b"let f = fixture(false, true).await;"), 1)
        expected = body.replace(b"let f = fixture(true, false).await;", b"let f = Box::pin(fixture(true, false)).await;")
        expected = expected.replace(b"let f = fixture(false, true).await;", b"let f = Box::pin(fixture(false, true)).await;")
        normalized = re.sub(rb'    eprintln!\("\\nFLEET_PM_RECOVERY_PHASE=[a-z_]+"\);\n', b"", self.source_blob(changed_rust))
        start = normalized.index(b"#[test]\n#[ignore")
        end = normalized.index(b"#[tokio::test]", start)
        normalized = normalized[:start] + normalized[end:]
        normalized = normalized.replace(b"Box::pin(submit(&f.supervisor, &f.agent, &f.intent, authorize()))",
                                        b"submit(&f.supervisor, &f.agent, &f.intent, authorize())")
        normalized = normalized.replace(b"Box::pin(submit(&other, &f.agent, &f.intent, authorize()))",
                                        b"submit(&other, &f.agent, &f.intent, authorize())")
        normalized = normalized.replace(b"Box::pin(submit(supervisor, &f.agent, intent, async { Ok(()) }))",
                                        b"submit(supervisor, &f.agent, intent, async { Ok(()) })")
        self.assertEqual(normalized, expected)
        expected_inventory = copy.deepcopy(prior)
        expected_inventory["source_commit"] = gate.SOURCE_SHA
        expected_inventory["config_union_preparation"]["source_commit"] = gate.SOURCE_SHA
        expected_inventory["config_union_preparation"]["codegen_evidence"] = preparation["codegen_evidence"]
        for path in changed_fingerprints:
            expected_inventory["compiled_source_sha256"]["fleet-control/" + path] = gate.digest(self.source_blob(path))
            expected_inventory["rust_source_sha256"][path] = gate.digest(self.source_blob(path))
        expected_inventory["workspace_default_declarations"] = REVIEWED["workspace_default_declarations"]
        expected_inventory["ignored"] = REVIEWED["ignored"]
        expected_inventory["default_foundation_ignored"] = 125
        expected_inventory["pm_workspace_required"] = sorted(prior["pm_workspace_required"] + [
            "runtime::pm_readback::tests::completed_requires_the_shared_terminal_flags"])
        expected_inventory["groups"]["pm_recovery_pg"] = REVIEWED["groups"]["pm_recovery_pg"]
        expected_inventory["groups"]["pm_human_controls"] = sorted(prior["groups"]["pm_human_controls"] + [
            "pm_human_controls::pm_stop_custody_migration_preserves_original_function_and_holds_unsafe_downgrade"])
        expected_inventory["migration_registries"] = {key: names + ["m20261011_000025_pm_stop_custody"]
            for key, names in prior["migration_registries"].items()}
        self.assertEqual(REVIEWED, expected_inventory)
        import ast
        old_helper = ast.parse(self.source_blob(gate.HELPER, prior_controls))
        current_helper = ast.parse(self.source_blob(gate.HELPER, "b9a81c8cb375bfe87a6531e001e16a7a1f4da4b7"))
        project = lambda tree: [ast.dump(node) for node in tree.body if not isinstance(node, ast.Assign)
            or not isinstance(node.targets[0], ast.Name) or node.targets[0].id not in {"SOURCE_SHA", "SOURCE_INVENTORY_SHA"}]
        self.assertEqual(project(old_helper), project(current_helper))
        old_tests = ast.parse(self.source_blob("scripts/tests/test_hosted_backend_gate.py", prior_controls))
        identities = lambda tree: {node.name for node in ast.walk(tree) if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        self.assertEqual(len(identities(old_tests)), 177)
        self.assertEqual(identities(old_tests), identities(ast.parse(self.source_blob(
            "scripts/tests/test_hosted_backend_gate.py", "b9a81c8cb375bfe87a6531e001e16a7a1f4da4b7"))))
        self.assertEqual((ROOT / gate.WORKFLOW).read_bytes(),
            self.source_blob(gate.WORKFLOW, prior_controls).replace(prior_source.encode(), gate.SOURCE_SHA.encode()))
        original_groups = json.loads(self.source_blob(gate.INVENTORY,
            "fe1fb3f5f863b8dad9dcd400ee2dccb68e69c4c4"))["groups"]
        original_groups["pm_human_controls"] = sorted(original_groups["pm_human_controls"] + [
            "pm_human_controls::pm_stop_custody_migration_preserves_original_function_and_holds_unsafe_downgrade"])
        self.assertEqual({k: v for k, v in REVIEWED["groups"].items() if k != "pm_recovery_pg"},
                         {k: v for k, v in original_groups.items() if k != "pm_recovery_pg"})
        for path in gate.WRITE_SET:
            data = (ROOT / path).read_bytes()
            canonical = subprocess.run(["git", "-C", str(ROOT), "show", ":" + path],
                capture_output=True, check=True, timeout=30).stdout
            self.assertEqual(data, canonical, path)
            self.assertNotIn(b"\r", data, path)
        source = next(step["with"] for step in self.workflow()["jobs"]["backend"]["steps"]
            if step.get("with", {}).get("path") == "fleet-control")
        self.assertEqual(source["ref"], gate.SOURCE_SHA)
        summary = self.workflow()["jobs"]["backend"]["steps"][-1]["run"]
        self.assertIn("printf 'Source: `%s`\\n\\n' " + gate.SOURCE_SHA, summary)
        previous = json.loads(self.source_blob(gate.INVENTORY, "566a5db30842e43e6552d8da8c2138145cf7ed71"))
        added = "chat_controls_hold_each_bound_pending_identity"
        self.assertEqual(set(REVIEWED["groups"]["foundation"]) - set(previous["groups"]["foundation"]), {added,
            "pm_dispatch::pm_publication_claim_requires_exact_run_instruction_receipt",
            "pm_events::pm_http_disabled_dispatch_denies_foreign_pins_and_unknown_ack",
            "pm_events::pm_http_disabled_dispatch_recovers_accepted_sse_and_terminal_once"})
        self.assertTrue(set(previous["groups"]["foundation"]) < set(REVIEWED["groups"]["foundation"]))
        source = self.source_blob("backend/infra/tests/sdlc_foundation.rs").decode()
        self.assertIn("#[tokio::test]\nasync fn " + added + "()", source)
        for name in (added, "message_receipts_and_replay_are_independent_of_history_limit",
                     "message_history_returns_latest_page_and_scopes_cursor",
                     "long_history_creation_replay_dispatch_and_terminal_mirror_return_exact_message",
                     "task_binding_is_immutable_unique_and_replays_concurrent_requests"):
            self.assertEqual(REVIEWED["groups"]["foundation"].count(name), 1)
            text = self.log("foundation", 125)
            line = "test " + name + " ... ok\n"
            for bad in (text.replace(line, ""), text.replace(line, line.replace("ok", "ignored")), text + line):
                with self.subTest(name=name), self.assertRaises(ValueError):
                    gate.verify_test_log("foundation", bad, REVIEWED)
        selectors = lambda text: {node.name for node in ast.walk(ast.parse(text))
            if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        import ast
        inherited = selectors(self.source_blob("scripts/tests/test_hosted_backend_gate.py", "566a5db30842e43e6552d8da8c2138145cf7ed71"))
        self.assertEqual(len(inherited), 148)
        self.assertTrue(inherited <= selectors(Path(__file__).read_bytes()))

    def test_union_additions_are_exact_three_unit_one_pg_and_one_foundation_case(self):
        frozen = json.loads(self.source_blob(gate.INVENTORY, "0d1e5a4361610c0a0728137731f54fa5ab12c481"))
        previous = json.loads(self.source_blob(gate.INVENTORY, "084d9f0f7b94953251b58a912b32b92cedbda020"))
        frozen_union = json.loads(self.source_blob(gate.INVENTORY, "6f23fd841f5e42fa0ea4a391cf296c3e9a93f25c"))
        identity = lambda row: (row["source"], row["name"])
        before = {identity(row) for row in previous["workspace_default_declarations"]}
        after = {identity(row) for row in frozen_union["workspace_default_declarations"]}
        self.assertTrue(before <= after)
        self.assertTrue(after <= {identity(row) for row in frozen["workspace_default_declarations"]})
        path = "backend/migration/src/m20261010_000021_activation_authority_alias.rs"
        self.assertEqual(after - before, {
            (path, "authority_alias_repair_changes_only_ambiguous_relation_references"),
            (path, "authority_alias_repair_is_repeatable_in_both_directions"),
            (path, "authority_alias_repair_rejects_missing_duplicate_and_mixed_guard"),
            ("backend/infra/tests/sdlc_foundation.rs", "message_receipts_and_replay_are_independent_of_history_limit")})
        before = {identity(row) for row in previous["ignored"]}
        after = {identity(row) for row in frozen["ignored"]}
        self.assertTrue(before <= after)
        added = (gate.ACTIVATION_PROBE_SOURCE, "recovered_authority_alias_repair_preserves_trigger_oid_and_exact_custody")
        self.assertEqual(after - before, {added})
        record = next(row for row in frozen["ignored"] if identity(row) == added)
        self.assertEqual((record["package"], record["target_kind"], record["target"], record["gate"]),
                         ("infra", "test", "container_activation", "container_activation_pg"))
        self.assertEqual(len(frozen["groups"]["container_activation_pg"]), 14)
        for stage in ("container_activation_pg", "foundation"):
            names = frozen["groups"][stage]
            ignored = 125 if stage == "foundation" else 0
            text = "".join("test " + name + " ... ok\n" for name in names)
            text += f"test result: ok. {len(names)} passed; 0 failed; {ignored} ignored;\n"
            gate.verify_test_log(stage, text, frozen)
            for bad in ("\n".join(text.splitlines()[1:]), text.replace(" ... ok", " ... ignored", 1),
                        text + text.splitlines()[0] + "\n"):
                with self.subTest(stage=stage), self.assertRaises(ValueError):
                    gate.verify_test_log(stage, bad, frozen)

    def test_union_migration_alias_boundary_keeps_exact_prior_prefixes_and_commands(self):
        previous = json.loads(self.source_blob(gate.INVENTORY, "084d9f0f7b94953251b58a912b32b92cedbda020"))
        for key in ("canonical", "split"):
            self.assertEqual(REVIEWED["migration_registries"][key], previous["migration_registries"][key]
                             + ["m20261010_000021_activation_authority_alias", "m20261010_000022_pm_dispatch",
                                "m20261010_000023_pm_human_controls", "m20261010_000024_pm_ack_bounds",
                                "m20261011_000025_pm_stop_custody"])
        expected = gate.expected_migration_receipt(REVIEWED)
        self.assertEqual(expected["down_alias"], sorted(previous["migration_registries"]["canonical"]))
        self.assertEqual(expected["down_one"], expected["down_alias"][:-1])
        self.assertEqual(expected["down_recovered"], expected["down_one"][:-1])
        self.assertEqual(expected["alias_reapply"], expected["down_pm"])
        self.assertEqual(expected["pm_reapply"], expected["down_human"])
        self.assertEqual(expected["human_reapply"], expected["down_ack"])
        self.assertEqual(expected["ack_reapply"], expected["down_stop"])
        self.assertEqual(expected["stop_reapply"], expected["up"])
        shell = (ROOT / gate.GATE).read_text()
        snapshots = re.findall(r"^  migration_snapshot (\w+)$", shell, re.M)
        self.assertEqual(snapshots, list(expected))
        source = (ROOT / gate.HELPER).read_text()
        self.assertIn('("up", "down_stop"), ("down_stop", "down_ack"), ("down_ack", "down_human"), ("down_human", "down_pm"), ("down_pm", "down_alias"), ("down_alias", "down_one"), ("down_one", "down_recovered")', source)
        self.assertIn('("reapply", "alias_reapply")', source)
        self.assertIn("cargo run --locked -p migration -- down -n 26", shell)
        segment = shell.split("stage=migration_smoke\n", 1)[1].split("} 2>&1", 1)[0]
        self.assertEqual(segment.count("cargo run --locked -p migration -- down -n 1\n"), 7)
        self.assertEqual(segment.count("cargo run --locked -p migration -- up -n 1\n"), 7)
        prior_shell = self.source_blob(gate.GATE, "084d9f0f7b94953251b58a912b32b92cedbda020").decode()
        historical_shell = self.source_blob(gate.GATE, "9565ecc1c2d114d44132598d77f4c5942d440d1d").decode()
        self.assertEqual(historical_shell.split("stage=migration_smoke\n", 1)[0].replace(
            "test result: ok. 11 passed;", "test result: ok. 10 passed;").replace(
            'stage=pm_human_controls\ncargo test --locked -p infra --test sdlc_foundation pm_human_controls:: -- --ignored --test-threads=1 2>&1 | tee "$QA_OUTPUT/$stage.log"\npassed\n', '').replace(
            'then ignored=124;', 'then ignored=119;'),
            prior_shell.split("stage=migration_smoke\n", 1)[0])
        self.assertEqual(shell.split("stage=openapi\n", 1)[1], prior_shell.split("stage=openapi\n", 1)[1])

    def test_union_pending_binding_rejects_old_receipt_before_network_or_execution(self):
        pending = dict(REVIEWED, openapi_binding=dict(status="pending_authentic_codegen", sha256=None))
        with mock.patch.object(gate, "OPENAPI_SHA", None), \
             mock.patch.object(gate, "reviewed_inventory", return_value=pending), \
             mock.patch.object(gate, "command") as command:
            with self.assertRaisesRegex(ValueError, "binding is pending"):
                gate.readback(SimpleNamespace())
            command.assert_not_called()
        old_hash = "1167220ea9f3d65ddca4cce1112a26d53c77f8c1684ef958859f737f20210953"
        with mock.patch.object(gate, "OPENAPI_SHA", old_hash):
            with self.assertRaisesRegex(ValueError, "binding is pending"):
                gate.require_codegen_binding(REVIEWED)

    def test_union_alias_snapshots_reject_changed_retained_ledger_timestamps(self):
        expected = gate.expected_migration_receipt(REVIEWED)
        for stage in ("down_pm", "down_alias", "alias_reapply", "pm_reapply"):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.migration_files(root, expected)
                path = root / ("migration-" + stage + "-ledger.tsv")
                path.write_text(path.read_text().replace("\t1000\n", "\t1001\n", 1), newline="\n")
                with self.assertRaises(ValueError):
                    gate.verify_migration_snapshots(root, REVIEWED)

    def test_pm_union_retains_all81_stages_and_adds_exact33_ordinary_cases(self):
        import ast
        predecessor = "6f23fd841f5e42fa0ea4a391cf296c3e9a93f25c"
        frozen = json.loads(self.source_blob(gate.INVENTORY, "0d1e5a4361610c0a0728137731f54fa5ab12c481"))
        previous = json.loads(self.source_blob(gate.INVENTORY, predecessor))
        tree = ast.parse(self.source_blob(gate.HELPER, predecessor))
        before_stages = next(ast.literal_eval(node.value) for node in tree.body
            if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == "GATES" for t in node.targets))
        self.assertEqual(tuple(stage for stage in gate.GATES if stage not in ("pm_human_controls", "pm_recovery_pg", "pm_ack_migration")), before_stages)
        identity = lambda row: (row["source"], row["name"])
        old = {identity(row) for row in previous["workspace_default_declarations"]}
        new = {identity(row) for row in frozen["workspace_default_declarations"]}
        self.assertTrue(old <= new)
        self.assertEqual(Counter(path for path, _ in new - old), {
            "backend/api/src/routes/pm_tools.rs": 2,
            "backend/domain/src/pm_dispatch.rs": 5,
            "backend/domain/src/pm_tools.rs": 2,
            "backend/infra/src/pm_tool_config.rs": 2,
            "backend/infra/src/runtime/pm_dispatch.rs": 4,
            "backend/infra/src/runtime/pm_tools.rs": 2,
            "backend/infra/tests/support/pm_credential_creation.rs": 1,
            "backend/infra/tests/support/pm_dispatch.rs": 6,
            "backend/infra/tests/support/pm_events.rs": 7,
            "backend/migration/src/lineage_tests.rs": 1,
            "backend/shared/src/config_tests.rs": 1})
        self.assertEqual({name.rsplit("::", 1)[-1] for name in frozen["pm_workspace_required"]},
                         {name for _, name in new - old})
        self.assertEqual(len(frozen["pm_workspace_required"]), 33)
        self.assertEqual({identity(row) for row in frozen["ignored"]},
                         {identity(row) for row in previous["ignored"]})
        selectors = lambda text: {node.name for node in ast.walk(ast.parse(text))
            if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        old_tests = selectors(self.source_blob("scripts/tests/test_hosted_backend_gate.py", predecessor))
        self.assertEqual(len(old_tests), 130)
        self.assertTrue(old_tests <= selectors(Path(__file__).read_bytes()))

    def test_pm_foundation_pg_cases_are_explicit_nonignored_and_cannot_be_omitted(self):
        for module, count in (("pm_dispatch", 7), ("pm_events", 9)):
            path = "backend/infra/tests/support/" + module + ".rs"
            names = {module + "::" + row["name"] for row in REVIEWED["workspace_default_declarations"]
                     if row["source"] == path}
            self.assertEqual(len(names), count)
            self.assertTrue(names <= set(REVIEWED["groups"]["foundation"]))
            self.assertIn('std::env::var("FLEET_TEST_DATABASE_URL").unwrap()', self.source_blob(path).decode())
        case = "pm_credential_creation::pm_mcp_publishes_tracker_receipts_then_resumes_only_after_saved_answer_and_terminal_proof"
        self.assertIn(case, REVIEWED["groups"]["credentials_pg"])
        self.assertIn(case, REVIEWED["groups"]["foundation"])
        shell = (ROOT / gate.GATE).read_text()
        self.assertIn("run_tests foundation -p infra --test sdlc_foundation\n", shell)
        self.assertIn("cargo test --locked --workspace -- --test-threads=1", shell)
        for stage in ("foundation", "credentials_pg", "lineage10", "config_shared_unit"):
            text = self.log(stage, 125 if stage == "foundation" else 0)
            gate.verify_test_log(stage, text, REVIEWED)
            for bad in ("\n".join(text.splitlines()[1:]), text.replace(" ... ok", " ... ignored", 1),
                        text + text.splitlines()[0] + "\n"):
                with self.subTest(stage=stage), self.assertRaises(ValueError):
                    gate.verify_test_log(stage, bad, REVIEWED)

    def test_pm_compiler_inventory_requires_all33_exact_new_cases_before_workspace(self):
        ignored = "\n".join(row["name"] + ": test" for row in REVIEWED["ignored"])
        required = REVIEWED["pm_workspace_required"]
        ordinary = ignored + "\n" + "\n".join(name + ": test" for name in required)
        gate.verify_runtime_inventory(ordinary, ignored, REVIEWED)
        for name in required:
            for bad in (ordinary.replace(name + ": test", ""), ordinary + "\n" + name + ": test"):
                with self.subTest(name=name), self.assertRaises(ValueError):
                    gate.verify_runtime_inventory(bad, ignored, REVIEWED)
        self.assertIn("verify_runtime_inventory", (ROOT / gate.HELPER).read_text())

    def test_pm022_has_empty_both_lineages_and_nonempty_custody_downgrade_coverage(self):
        case = "lineage_tests::empty_pm_custody_downgrade_and_reupgrade_preserve_older_lineage"
        self.assertIn(case, REVIEWED["groups"]["lineage10"])
        lineage = self.source_blob("backend/migration/src/lineage_tests.rs").decode()
        self.assertIn("for legacy in [false, true]", lineage)
        self.assertIn('assert_eq!(after[after.len() - 4].0, "m20261010_000022_pm_dispatch")', lineage)
        self.assertIn('"m20261010_000023_pm_human_controls"', lineage)
        name = "pm_dispatch::pm_downgrade_refuses_unknown_known_and_guidance_custody_without_changing_ledger"
        self.assertIn(name, REVIEWED["groups"]["foundation"])
        source = self.source_blob("backend/infra/tests/support/pm_dispatch.rs").decode()
        self.assertIn('for phase in ["unknown", "known", "guidance"]', source)
        self.assertIn('error.contains("PM dispatch custody prevents downgrade")', source)
        self.assertIn("assert_eq!(before, after)", source)
        expected = gate.expected_migration_receipt(REVIEWED)
        self.assertEqual({key: len(value) for key, value in expected.items()},
            dict(up=26, down_stop=25, down_ack=24, down_human=23, down_pm=22, down_alias=21, down_one=20, down_recovered=19,
                 recovered_reapply=20, reapply=21, alias_reapply=22, pm_reapply=23, human_reapply=24, ack_reapply=25, stop_reapply=26, down_all=0, final_up=26))
        for mutation in ("drop_pm", "reorder"):
            bad = copy.deepcopy(REVIEWED)
            if mutation == "drop_pm":
                bad["migration_registries"]["canonical"].pop()
            else:
                bad["migration_registries"]["canonical"][-1] = "m20261010_000023_private"
            with self.assertRaises(ValueError):
                gate.expected_migration_receipt(bad)

    def test_pm_codegen_binding_uses_authentic830_bytes_and_unchanged_production(self):
        source = "83091f055e3b34fcfe6a6d59b1703c117261c027"
        frozen = json.loads(self.source_blob(gate.INVENTORY, "0d1e5a4361610c0a0728137731f54fa5ab12c481"))
        delta = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "diff", "--name-only",
            source, "facb25b9eae66db0c8b762ab68a5963422edf58f"], capture_output=True, check=True, timeout=30).stdout.decode().splitlines()
        self.assertEqual(set(delta), {"backend/infra/tests/container_activation.rs", "docs/CURRENT_STATE.md",
                                    "docs/GAP_REGISTER.md", "openapi/openapi.json"})
        old = "ac545326e9b4ffca4378aee0aaaf9c2dd8c75faf02deb868cda0c87d7764b85a"
        self.assertEqual(gate.digest(self.source_blob("openapi/openapi.json", source)), old)
        self.assertNotEqual(gate.OPENAPI_SHA, old)
        with mock.patch.object(gate, "OPENAPI_SHA", old), self.assertRaises(ValueError):
            gate.require_codegen_binding(frozen)

    def test_successor_auth_two_case_summary_rejects_old_single_case_receipt(self):
        text = self.log("real_auth")
        gate.verify_test_log("real_auth", text, REVIEWED)
        for bad in (text.replace("2 passed", "1 passed"),
                    "\n".join(text.splitlines()[1:]), text.replace("0 ignored", "1 ignored")):
            with self.assertRaises(ValueError):
                gate.verify_test_log("real_auth", bad, REVIEWED)
        shell = (ROOT / gate.GATE).read_text().split("stage=real_auth\n", 1)[1].split("\npassed", 1)[0]
        self.assertIn("--ignored --test-threads=1", shell)
        self.assertIn("done < ${QA_EXPECTED}/real_auth.txt", shell)
        self.assertIn("grep -Fx \"test $name ... ok\"", shell)
        self.assertIn("grep -F 'test result: ok. 2 passed; 0 failed; 0 ignored;'", shell)
        self.assertNotIn("test result: ok. 1 passed", shell)

    def test_successor_adds_only_twelve_source_attested_readonly_probe_hints(self):
        import ast
        old = ast.parse(self.source_blob(gate.HELPER, "6f648430a43ffbf021c804648fd05f27e2268bdb"))
        before = next(ast.literal_eval(node.value.args[0]) for node in old.body if isinstance(node, ast.Assign)
            and any(isinstance(target, ast.Name) and target.id == "TEST_ACTIVATION_HINTS" for target in node.targets))
        expected = {"activation_probe_" + suffix for suffix in (
            "checked_query_ok", "checked_row_present", "checked_revision_decode", "checked_revision_predicate",
            "checked_snapshot_decode", "checked_snapshot_predicate", "current_query_ok", "current_row_present",
            "authority_query_ok", "authority_row_present", "authority_predicate_decode", "authority_predicate")}
        self.assertEqual(len(before), 25)
        self.assertEqual(gate.TEST_ACTIVATION_HINTS - before, expected)
        self.assertTrue(before <= gate.TEST_ACTIVATION_HINTS)
        source = self.source_blob(gate.ACTIVATION_PROBE_SOURCE)
        self.assertEqual(gate.digest(source), "6ba3f1f368a2bbe2e977fdb96a7a09dfe9a516b37c7052f32e1c36053df84e64")
        helper = source.decode().split("async fn assert_recovered_preconditions(", 1)[1].split("async fn recovered_step(", 1)[0]
        self.assertTrue(expected <= set(re.findall(r'"(activation_probe_[a-z_]+)"', helper)))
        for hint in expected:
            result = self.probe_diagnostics(hint)
            self.assertEqual(result["categories"], sorted([hint, "test_failure"]))
            self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_successor_new_probe_hint_readback_rejects_private_and_unanchored_values(self):
        for hint in sorted(value for value in gate.TEST_ACTIVATION_HINTS
                           if value.startswith(("activation_probe_checked_", "activation_probe_current_", "activation_probe_authority_"))):
            result = self.probe_diagnostics(hint)
            value = dict(self.failure_test_value(), stage="container_activation_pg",
                         gate_failed_stage="container_activation_pg", **result)
            run, artifact, payload, args = self.failure_artifact(value=value)
            files = gate.validate_failure_readback(run, artifact, payload, **args)
            retained = json.loads(files[gate.FAILURE_FILE])
            for flag in ("backend_quality_gate", "all_quality_gate", "sdlc_acceptance"):
                self.assertFalse(retained[flag])
            for detail in (hint + " PRIVATE_SENTINEL", "PRIVATE_SENTINEL " + hint, hint + "_unknown",
                           "SELECT '" + hint + "';", "blank\n" + hint):
                self.assertEqual(self.probe_diagnostics(detail)["categories"], ["test_failure"])
            for options in (dict(thread="foreign"), dict(path="/qa/src/services-base/private.rs")):
                expected = ["unknown"] if "path" in options else ["test_failure"]
                self.assertEqual(self.probe_diagnostics(hint, **options)["categories"], expected)
            for change in (dict(categories=[hint]), dict(categories=[hint, "PRIVATE_SENTINEL", "test_failure"]),
                           dict(diagnostics=[]), dict(backend_quality_gate=True)):
                with self.assertRaises(ValueError):
                    self.validate_failure(dict(value, **change))

    def test_journal_nine_identities_ignore_flags_and_lines_match_actual_git_source(self):
        specs = (
            ("backend/domain/src/clarification_commands.rs", "clarification_domain", "clarification_commands::tests::", False, 2),
            ("backend/api/src/routes/clarification_commands.rs", "clarification_api", "routes::clarification_commands::tests::", False, 2),
            ("backend/infra/tests/support/clarification_custody.rs", "clarification_pg", "clarification_custody::", True, 4),
            ("backend/migration/tests/clarification_commands.rs", "clarification_migration", "", True, 1),
        )
        for path, stage, prefix, ignored, count in specs:
            text = self.source_blob(path).decode()
            cases = [m for m in re.finditer(r"(?m)(?P<attrs>(?:[ \t]*#\[[^\n]*\]\n)+)[ \t]*(?:async )?fn (?P<name>\w+)\(", text)
                if re.search(r"#\[(?:tokio::)?test\]", m["attrs"])]
            self.assertEqual(len(cases), count)
            self.assertEqual(sorted(prefix + m["name"] for m in cases), REVIEWED["groups"][stage])
            for match in cases:
                self.assertEqual("#[ignore" in match["attrs"], ignored)
                records = REVIEWED["ignored"] if ignored else REVIEWED["workspace_default_declarations"]
                record = next(x for x in records if x["source"] == path and x["name"] == (prefix if ignored else "") + match["name"])
                self.assertEqual(record["line"], text[:match.start("name")].count("\n") + 1)
                if ignored:
                    self.assertEqual(record["target_kind"], "test")
                    self.assertEqual(record["target"], "sdlc_foundation" if stage == "clarification_pg" else "clarification_commands")
                    self.assertEqual(record["gate"], stage)

    def test_journal_source_fingerprints_match_canonical_git_blobs_without_materializing(self):
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

    def test_authentic_codegen_binding_and_missing_ancestry_fail_before_private_or_heavy_effects(self):
        self.assertEqual(gate.OPENAPI_SHA, "afa46ac37b726232eda73df46c24d1d42c796f8873eefb68454fbe0f243df501")
        prior_schema = "ad980604beb2cff0890f4d1a07a185c97a444fda166985f2a6da465a222d129c"
        frozen = json.loads(self.source_blob(gate.INVENTORY, "0d1e5a4361610c0a0728137731f54fa5ab12c481"))
        self.assertEqual(frozen["openapi_binding"], dict(status="verified", sha256=prior_schema))
        self.assertEqual(gate.digest(self.source_blob("openapi/openapi.json", frozen["source_commit"])), prior_schema)
        qualified = json.loads(self.source_blob(gate.INVENTORY, "566a5db30842e43e6552d8da8c2138145cf7ed71"))
        historical_schema = "e1b17e723abf43866c4f913c9fa4fba8b201bef5e3532b4a8f6cdc32ccbcce76"
        self.assertEqual(gate.digest(self.source_blob("openapi/openapi.json", qualified["source_commit"])), historical_schema)
        with mock.patch.object(gate, "OPENAPI_SHA", prior_schema):
            gate.require_codegen_binding(frozen)
        with mock.patch.object(gate, "OPENAPI_SHA", historical_schema):
            gate.require_codegen_binding(qualified)
        gate.require_codegen_binding(REVIEWED)
        gate.require_codegen_binding(self.synthetic_bound_inventory())
        self.assertEqual(gate.digest(self.source_blob("openapi/openapi.json", gate.SOURCE_SHA)), gate.OPENAPI_SHA)
        with mock.patch.object(gate, "OPENAPI_SHA", prior_schema), self.assertRaisesRegex(ValueError, "binding is pending"):
            gate.require_codegen_binding(REVIEWED)
        evidence = REVIEWED["config_union_preparation"]["prior_codegen_evidence"]
        self.assertEqual(evidence["run_id"], 38040550767)
        self.assertEqual(evidence["run_attempt"], 1)
        self.assertEqual(evidence["artifact_id"], 11666035358)
        self.assertEqual(evidence["artifact_zip_sha256"], "a6d16c4dc3fb47d3b6da5da22d42b1c91db9f83fc2e77d2bbd3a7aa8b1577daf")
        self.assertEqual(evidence["workflow_commit"], "b22971dbfbc0476b7789f94fa28999581e17da08")
        self.assertEqual(evidence["source_commit"], "83091f055e3b34fcfe6a6d59b1703c117261c027")
        self.assertEqual(evidence["schema_sha256"], prior_schema)
        self.assertEqual(evidence["artifact_bound_source_commit"], frozen["source_commit"])
        pending = dict(REVIEWED, openapi_binding=dict(status="pending_final_source_binding", sha256=None))
        with self.assertRaisesRegex(ValueError, "binding is pending"):
            gate.require_codegen_binding(pending)
        with mock.patch.object(gate, "hosted_identity", return_value=(Path("owned"), "a" * 40)), \
                mock.patch.object(gate, "clean_head"), mock.patch.object(gate, "reviewed_inventory", return_value=pending), \
                mock.patch.object(gate, "git") as git, mock.patch.object(gate.subprocess, "Popen") as spawn:
            with self.assertRaisesRegex(ValueError, "binding is pending"):
                gate.preflight()
            git.assert_not_called()
            spawn.assert_not_called()
        with mock.patch.object(gate, "hosted_identity", return_value=(Path("owned"), "a" * 40)), \
                mock.patch.object(gate, "clean_head"), mock.patch.object(gate, "reviewed_inventory", return_value=self.synthetic_bound_inventory()), \
                mock.patch.object(gate, "git", side_effect=ValueError("Unreconciled source ancestry")) as git, \
                mock.patch.object(gate.subprocess, "Popen") as spawn:
            with self.assertRaisesRegex(ValueError, "Unreconciled source ancestry"):
                gate.preflight()
            git.assert_called_once_with(Path("owned") / "controls", "merge-base", "--is-ancestor", gate.SOURCE_SHA, "a" * 40)
            spawn.assert_not_called()
        with mock.patch.object(gate, "OPENAPI_SHA", "e" * 64):
            with self.assertRaises(ValueError):
                gate.require_codegen_binding(frozen)
            bound = dict(REVIEWED, openapi_binding=dict(status="verified", sha256="e" * 64))
            gate.require_codegen_binding(bound)  # Synthetic unit contract, not codegen evidence.
            for binding in (dict(status="pending_authentic_codegen", sha256="e" * 64),
                            dict(status="verified", sha256="d" * 64), {}):
                with self.assertRaises(ValueError):
                    gate.require_codegen_binding(dict(REVIEWED, openapi_binding=binding))

    def test_prepared_readback_rejects_forged_success_and_never_calls_GitHub(self):
        pending = dict(REVIEWED, openapi_binding=dict(status="pending_source_freeze", sha256=gate.OPENAPI_SHA))
        with mock.patch.object(gate, "command") as command, \
                mock.patch.object(gate, "reviewed_inventory", return_value=pending):
            with self.assertRaises(ValueError):
                gate.readback(SimpleNamespace())
            command.assert_not_called()
        run, artifact, payload, args = self.artifact(provenance_changes=dict(openapi_sha256=None))
        with mock.patch.object(gate, "OPENAPI_SHA", None):
            with self.assertRaisesRegex(ValueError, "Authentic generated OpenAPI/source binding is pending"):
                gate.validate_readback(run, artifact, payload, **args)

    def test_journal_each_exact_group_rejects_missing_duplicate_skip_or_count_only(self):
        for stage in ("clarification_domain", "clarification_api", "clarification_pg", "clarification_migration"):
            text = self.log(stage)
            self.assertEqual(gate.verify_test_log(stage, text, REVIEWED)["passed"], len(REVIEWED["groups"][stage]))
            first = text.splitlines()[0]
            for bad in (text.replace(first + "\n", "", 1), text + first + "\n",
                        text.replace(" ... ok", " ... ignored", 1), text.replace("0 failed", "1 failed"),
                        text.replace("0 ignored", "1 ignored"), text.splitlines()[-1],
                        text + "PostgreSQL tests skipped\n"):
                with self.subTest(stage=stage), self.assertRaises(ValueError):
                    gate.verify_test_log(stage, bad, REVIEWED)

    def test_journal_commands_use_isolated_PG_URL_without_mutating_other_stage_environment(self):
        shell = (ROOT / gate.GATE).read_text()
        commands = (
            "run_tests clarification_domain -p domain --lib clarification_commands::tests::",
            "run_tests clarification_api -p api --lib routes::clarification_commands::tests::",
            'FLEET_TEST_DATABASE_URL="$FLEET_CLARIFICATION_TEST_DATABASE_URL" \\\n  cargo test --locked -p infra --test sdlc_foundation clarification_custody:: -- --ignored --test-threads=1',
            "run_ignored_target clarification_migration migration clarification_commands",
        )
        positions = [shell.index(command) for command in commands]
        self.assertEqual(positions, sorted(positions))
        self.assertLess(shell.index("run_tests foundation"), positions[0])
        self.assertLess(positions[-1], shell.index("stage=workspace"))
        self.assertNotIn('export FLEET_TEST_DATABASE_URL=', shell)
        env = gate.database_environment()
        self.assertNotEqual(env["FLEET_TEST_DATABASE_URL"], env["FLEET_CLARIFICATION_TEST_DATABASE_URL"])
        self.assertEqual(env["FLEET_CLARIFICATION_MIGRATION_TEST_DATABASE_URL"],
            "postgres://fleet_test@postgres:5432/fleet_clarification_migration_test")

    def test_migration_smoke_preserves_surviving_timestamps_at_both_20_and_19_boundaries(self):
        expected = gate.expected_migration_receipt(REVIEWED)
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for stage in ("down_one", "down_recovered", "recovered_reapply", "reapply"):
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
        self.assertEqual(job["container"], dict(image="public.ecr.aws/docker/library/rust@sha256:af306cfa71d987911a781c37b59d7d67d934f49684058f96cf72079c3626bfe0", options="--cpus 2 --memory 6g"))
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
        self.assertEqual(len(urls), 24)
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
        self.assertEqual({step["ref"] for step in private}, {gate.BASE_SHA, gate.AUTH_SHA, gate.UTILITY_SHA, gate.PACKAGE_SHA})
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

    def test_all_167_ignored_exactly_once_and_337_default_declarations(self):
        records = REVIEWED["ignored"]
        self.assertEqual(len(records), 180)
        self.assertEqual(len({(x["source"], x["name"]) for x in records}), 180)
        self.assertEqual(Counter(x["gate"] for x in records)["runtime_controls"], 30)
        self.assertEqual(len(REVIEWED["groups"]["runtime_terminal"]), 14)
        self.assertEqual(len(REVIEWED["groups"]["foundation"]), 77)
        self.assertEqual(REVIEWED["default_foundation_ignored"], 125)
        self.assertEqual(len(REVIEWED["workspace_default_declarations"]), 401)
        self.assertIn("activation_probe_hash_matches_base_unicode_snapshot",
                      {row["name"] for row in REVIEWED["workspace_default_declarations"]})
        self.assertEqual(REVIEWED["authority"]["old_ignored"], 130)
        for x in records:
            self.assertIn(x["name"], REVIEWED["groups"][x["gate"]])

    def test_required_full_commands_migrations_and_real_auth_remain(self):
        text = (ROOT / gate.GATE).read_text()
        for command in ("cargo fmt --all -- --check", "cargo check --locked --workspace --all-targets",
                        "cargo clippy --locked --workspace --all-targets --message-format=json -- -D warnings",
                        "cargo test --locked --workspace -- --test-threads=1",
                        "cargo test --locked -p migration --lib lineage_tests -- --include-ignored --test-threads=1",
                        "cargo run --locked -p migration -- down -n 1", "cargo run --locked -p migration -- down -n 26",
                        "cargo build --locked -p auth-server --bin auth-server",
                        "cargo test --locked -p infra --test pm_credentials_real_auth -- --ignored --test-threads=1",
                        "cmp ../openapi/openapi.json ${QA_OUTPUT}/openapi.json"):
            self.assertIn(command, text)
        self.assertEqual(len(gate.GATES), 84)
        self.assertEqual(len(REVIEWED["groups"]["lineage10"]), 11)
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

    def test_foundation_53_passed_and_119_ignored_never_lowered(self):
        gate.verify_test_log("foundation", self.log("foundation", 125), REVIEWED)
        with self.assertRaises(ValueError):
            gate.verify_test_log("foundation", self.log("foundation", 115), REVIEWED)

    def test_compiler_ignored_list_must_be_exact_167_no_missing_extra_duplicate(self):
        names = [item["name"] for item in REVIEWED["ignored"]]
        ignored = "\n".join(name + ": test" for name in names)
        ordinary = ignored + "\nnormal: test\n" + "\n".join(name + ": test" for name in REVIEWED["pm_workspace_required"])
        result = gate.verify_runtime_inventory(ordinary, ignored, REVIEWED)
        self.assertEqual(result["listed_default_count"], 51)
        for bad in ("", ignored + "\nextra: test", ignored + "\n" + names[0] + ": test", "\n".join(ignored.splitlines()[1:])):
            with self.assertRaises(ValueError):
                gate.verify_runtime_inventory(ordinary, bad, REVIEWED)

    def test_workspace_actual_cases_match_compiler_list_not_static_count(self):
        listing = "\n".join(x["name"] + ": test" for x in REVIEWED["ignored"]) + "\nfirst: test\nsecond: test\n"
        text = "test first ... ok\ntest second ... ok\ntest result: ok. 2 passed; 0 failed; 180 ignored;\n"
        result = gate.verify_test_log("workspace", text, REVIEWED, listing)
        self.assertEqual(result["passed"], 2)
        for bad in (text.replace("test second ... ok\n", ""), text.replace("180 ignored", "162 ignored"), ""):
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
                "/sys/fs/cgroup/memory.max": str(6 * 1024 ** 3), "/sys/fs/cgroup/memory.current": str(512 * 1024 ** 2)}
        with mock.patch.object(Path, "read_text", autospec=True, side_effect=lambda path: data[path.as_posix()]), \
                mock.patch.object(Path, "open", side_effect=FileNotFoundError), mock.patch.object(sys, "stdout", io.StringIO()), \
                mock.patch.object(gate.shutil, "disk_usage", return_value=SimpleNamespace(free=5 * 1024 ** 3)) as disk:
            self.assertEqual(gate.resource_guard(Path("owned"))["hosted_disk_guard_bytes"], 5 * 1024 ** 3)
            disk.return_value.free -= 1
            with self.assertRaises(ValueError):
                gate.resource_guard(Path("owned"))
            disk.return_value.free += 1
            data["/sys/fs/cgroup/memory.current"] = str(4 * 1024 ** 3)
            with self.assertRaises(ValueError):
                gate.resource_guard(Path("owned"))

    def resource_observation(self, body, *, current=4 * 1024 ** 3, available=5000000, event_body=b""):
        data = {"/proc/meminfo": f"MemAvailable: {available} kB\nCommitLimit: 7000000 kB\nCommitted_AS: 1000000 kB\n",
                "/sys/fs/cgroup/memory.max": str(6 * 1024 ** 3), "/sys/fs/cgroup/memory.current": str(current)}
        stream = mock.MagicMock()
        stream.__enter__.return_value = stream
        stream.read.return_value = body
        event_stream = mock.MagicMock()
        event_stream.__enter__.return_value = event_stream
        event_stream.read.return_value = event_body
        def open_counter(path, mode):
            self.assertEqual(mode, "rb")
            value, handle = {Path("/sys/fs/cgroup/memory.stat"): (body, stream),
                             Path("/sys/fs/cgroup/memory.events"): (event_body, event_stream)}[path]
            if isinstance(value, Exception):
                raise value
            return handle
        output = io.StringIO()
        with mock.patch.object(Path, "read_text", autospec=True, side_effect=lambda path: data[path.as_posix()]), \
                mock.patch.object(Path, "open", autospec=True, side_effect=open_counter) as opened, \
                mock.patch.object(gate.shutil, "disk_usage", return_value=SimpleNamespace(free=5 * 1024 ** 3)), \
                mock.patch.object(sys, "stdout", output), mock.patch.object(gate, "command") as command:
            with self.assertRaisesRegex(ValueError, "^Hosted initial cgroup headroom below 3 GiB$"):
                gate.resource_guard(Path("owned"))
        self.assertEqual(opened.call_args_list, [mock.call(Path("/sys/fs/cgroup/memory.stat"), "rb"),
                                                mock.call(Path("/sys/fs/cgroup/memory.events"), "rb")])
        if not isinstance(body, Exception):
            stream.read.assert_called_once_with(16 * 1024 + 1)
        if not isinstance(event_body, Exception):
            event_stream.read.assert_called_once_with(4 * 1024 + 1)
        command.assert_not_called()
        self.assertLess(len(output.getvalue()), 2048)
        self.assertNotIn("PRIVATE_SENTINEL", output.getvalue())
        self.assertEqual(len(output.getvalue().splitlines()), 1)
        value = json.loads(output.getvalue())
        self.assertEqual(set(value), {"state", "reason", "cgroup_limit_bytes", "cgroup_current_bytes",
            "cgroup_anon_bytes", "cgroup_file_bytes", "cgroup_shmem_bytes", "cgroup_kernel_bytes", "cgroup_inactive_file_bytes",
            "cgroup_slab_bytes", "cgroup_slab_reclaimable_bytes", "cgroup_slab_unreclaimable_bytes", "proc_mem_available_bytes", "cgroup_events",
            "backend_quality_gate", "all_quality_gate", "sdlc_acceptance"})
        self.assertEqual((value["state"], value["reason"]), ("backend_resource_observation", "cgroup_headroom"))
        for key in ("backend_quality_gate", "all_quality_gate", "sdlc_acceptance"):
            self.assertIs(value[key], False)
        for key in ("cgroup_limit_bytes", "cgroup_current_bytes", "cgroup_anon_bytes", "cgroup_file_bytes",
                    "cgroup_shmem_bytes", "cgroup_kernel_bytes", "cgroup_inactive_file_bytes", "cgroup_slab_bytes",
                    "cgroup_slab_reclaimable_bytes", "cgroup_slab_unreclaimable_bytes", "proc_mem_available_bytes"):
            self.assertTrue(value[key] is None or type(value[key]) is int and 0 <= value[key] <= 2 ** 63 - 1)
        self.assertEqual(set(value["cgroup_events"]), {"low", "high", "max", "oom", "oom_kill", "oom_group_kill"})
        self.assertTrue(all(item is None or type(item) is int and 0 <= item <= 2 ** 63 - 1 for item in value["cgroup_events"].values()))
        return value

    def test_resource_observation_reports_only_same_mount_bounded_byte_counters(self):
        value = self.resource_observation(b"file 123456\nPRIVATE_SENTINEL 999\nslab 789\nanon 456\n")
        self.assertEqual([value[key] for key in ("cgroup_anon_bytes", "cgroup_file_bytes", "cgroup_slab_bytes")], [456, 123456, 789])
        self.assertEqual((value["cgroup_limit_bytes"], value["cgroup_current_bytes"], value["proc_mem_available_bytes"]),
                         (6 * 1024 ** 3, 4 * 1024 ** 3, 5000000 * 1024))
        self.assertEqual(self.resource_observation(b"anon 0\nfile 0\nslab 0\n")["cgroup_anon_bytes"], 0)

    def test_resource_observation_missing_invalid_private_or_oversized_stat_stays_null(self):
        for body in (b"", b"anon -1\nfile 9223372036854775808\nslab PRIVATE_SENTINEL\n",
                     b"anon 999999999999999999999\nfile 1.2\nslab 4 PRIVATE_SENTINEL\n",
                     b"anon 1\nanon 2\nfile 3\nfile PRIVATE_SENTINEL\nslab 1\nslab 1\n",
                     b"anon 1\nPRIVATE_SENTINEL \xff\n", b"anon 1\n" + b"x" * (16 * 1024) + b"PRIVATE_SENTINEL"):
            with self.subTest(size=len(body)):
                value = self.resource_observation(body)
                self.assertTrue(all(value[key] is None for key in ("cgroup_anon_bytes", "cgroup_file_bytes", "cgroup_slab_bytes")))
        for error in (FileNotFoundError("PRIVATE_SENTINEL"), PermissionError("PRIVATE_SENTINEL"), OSError("PRIVATE_SENTINEL")):
            value = self.resource_observation(error)
            self.assertIsNone(value["cgroup_anon_bytes"])
        value = self.resource_observation(b"anon 9223372036854775807\n", current=2 ** 63, available=2 ** 63)
        self.assertEqual(value["cgroup_anon_bytes"], 2 ** 63 - 1)
        self.assertIsNone(value["cgroup_current_bytes"])
        self.assertIsNone(value["proc_mem_available_bytes"])

    def test_resource_observation_never_runs_for_success_limit_or_disk_refusal(self):
        data = {"/proc/meminfo": "MemAvailable: 5000000 kB\n", "/sys/fs/cgroup/memory.max": str(6 * 1024 ** 3),
                "/sys/fs/cgroup/memory.current": str(3 * 1024 ** 3)}
        output = io.StringIO()
        with mock.patch.object(Path, "read_text", autospec=True, side_effect=lambda path: data[path.as_posix()]), \
                mock.patch.object(Path, "open") as opened, mock.patch.object(sys, "stdout", output), \
                mock.patch.object(gate.shutil, "disk_usage", return_value=SimpleNamespace(free=5 * 1024 ** 3)) as disk:
            self.assertEqual(gate.resource_guard(Path("owned"))["cgroup_current_bytes"], 3 * 1024 ** 3)
            data["/sys/fs/cgroup/memory.max"] = str(4 * 1024 ** 3)
            with self.assertRaisesRegex(ValueError, "^Hosted compiler cgroup must be bounded at 6 GiB$"):
                gate.resource_guard(Path("owned"))
            data["/sys/fs/cgroup/memory.max"] = str(6 * 1024 ** 3)
            disk.return_value.free -= 1
            with self.assertRaisesRegex(ValueError, "^Disposable hosted CI requires 5 GiB free; no waiver$"):
                gate.resource_guard(Path("owned"))
        opened.assert_not_called()
        self.assertEqual(output.getvalue(), "")
        self.assertEqual(self.resource_observation(b"anon 1\n", current=3 * 1024 ** 3 + 1)["cgroup_current_bytes"], 3 * 1024 ** 3 + 1)

    def test_resource_observation_preserves_absolute_budget_timeout(self):
        data = {"/proc/meminfo": "MemAvailable: 5000000 kB\n", "/sys/fs/cgroup/memory.max": str(6 * 1024 ** 3),
                "/sys/fs/cgroup/memory.current": str(4 * 1024 ** 3)}
        for timeout_at in ("memory.stat", "memory.events"):
            output = io.StringIO()
            def open_counter(path, mode):
                if path.name == timeout_at:
                    raise TimeoutError("PRIVATE_SENTINEL")
                return io.BytesIO(b"anon 1\n")
            with mock.patch.object(Path, "read_text", autospec=True, side_effect=lambda path: data[path.as_posix()]), \
                    mock.patch.object(Path, "open", autospec=True, side_effect=open_counter), \
                    mock.patch.object(sys, "stdout", output), \
                    mock.patch.object(gate.shutil, "disk_usage", return_value=SimpleNamespace(free=5 * 1024 ** 3)):
                with self.assertRaises(TimeoutError):
                    gate.resource_guard(Path("owned"))
            self.assertEqual(output.getvalue(), "")

    def test_resource_observation_extended_stat_and_events_are_fixed_bounded_and_private(self):
        value = self.resource_observation(
            b"shmem 11\nkernel 12\ninactive_file 13\nslab 14\nslab_reclaimable 15\nslab_unreclaimable 16\nPRIVATE_SENTINEL 123\n",
            event_body=b"low 1\nhigh 2\nmax 3\noom 0\noom_kill 0\noom_group_kill 0\nPRIVATE_SENTINEL 999\n")
        self.assertEqual([value["cgroup_" + name + "_bytes"] for name in
                          ("shmem", "kernel", "inactive_file", "slab", "slab_reclaimable", "slab_unreclaimable")], [11, 12, 13, 14, 15, 16])
        self.assertEqual(value["cgroup_events"], dict(low=1, high=2, max=3, oom=0, oom_kill=0, oom_group_kill=0))
        for body in (b"", b"low -1\nhigh 9223372036854775808\nmax 3\nmax PRIVATE_SENTINEL\noom 1.5\noom_kill true\n",
                     b"low 1\nPRIVATE_SENTINEL \xff", b"low 1\n" + b"x" * (4 * 1024), FileNotFoundError("PRIVATE_SENTINEL")):
            value = self.resource_observation(b"anon 77\n", event_body=body)
            self.assertEqual(value["cgroup_anon_bytes"], 77)
            self.assertTrue(all(item is None for item in value["cgroup_events"].values()))
        import ast
        frozen = ast.parse(self.source_blob("scripts/tests/test_hosted_backend_gate.py", "40c28041a343b701063ea1762f07c1cf19126726"))
        names = lambda tree: {node.name for node in ast.walk(tree) if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        self.assertEqual(len(names(frozen)), 169)
        self.assertTrue(names(frozen) <= names(ast.parse(Path(__file__).read_bytes())))

    def test_resource_observation_successor_preserves_164_identities_guards_and_provenance(self):
        import ast
        frozen = "4b9e80c1359352dfa24b97650533c14cc6f718a3"
        old = ast.parse(self.source_blob(gate.HELPER, frozen))
        new = ast.parse(self.source_blob(gate.HELPER, "b9a81c8cb375bfe87a6531e001e16a7a1f4da4b7"))
        functions = lambda tree: {node.name: node for node in tree.body if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
        before, after = functions(old), functions(new)
        self.assertEqual(before.keys(), after.keys())
        self.assertEqual({name for name in before if ast.dump(before[name]) != ast.dump(after[name])},
                         {"resource_guard", "main", "safe_test_diagnostics", "validate_failure_evidence"})
        guard = copy.deepcopy(after["resource_guard"])
        blocks = [node for node in guard.body if isinstance(node, ast.If)]
        self.assertEqual(len(blocks), 1)
        self.assertEqual(ast.unparse(blocks[0].test), "int(maximum) - current < 3 * 1024 ** 3")
        guard.body.remove(blocks[0])
        expected = ast.unparse(before["resource_guard"])
        self.assertEqual(expected.count("== 4 * 1024 ** 3"), 1)
        self.assertEqual(expected.count("bounded at 4 GiB"), 1)
        expected = expected.replace("== 4 * 1024 ** 3", "== 6 * 1024 ** 3").replace("bounded at 4 GiB", "bounded at 6 GiB")
        self.assertEqual(ast.dump(ast.parse(expected).body[0]), ast.dump(guard))
        names = lambda tree: {node.name for node in ast.walk(tree) if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        original = names(ast.parse(self.source_blob("scripts/tests/test_hosted_backend_gate.py", frozen)))
        self.assertEqual(len(original), 164)
        self.assertTrue(original <= names(ast.parse(Path(__file__).read_bytes())))
        for path in (gate.WORKFLOW, gate.GATE, gate.INIT, gate.INVENTORY):
            expected = self.source_blob(path, frozen)
            if path == gate.WORKFLOW:
                self.assertEqual(expected.count(b"--memory 4g"), 1)
                expected = expected.replace(b"--memory 4g", b"--memory 6g")
            actual = self.source_blob(path, "78d3727e196ed17af3af371e3a56936e556cd7e8") if path in (gate.WORKFLOW, gate.GATE, gate.INVENTORY) else (ROOT / path).read_bytes()
            self.assertEqual(actual, expected, path)
        self.assertEqual(gate.SOURCE_SHA, "8f8e69d637a64b2d7a3e8c2bc6dca00517539667")
        self.assertEqual(len(gate.GATES), 84)

    def test_hosted_six_gib_policy_is_exact_and_retains_three_gib_reserve(self):
        data = {"/proc/meminfo": "MemAvailable: 15030344 kB\n", "/sys/fs/cgroup/memory.max": str(6 * 1024 ** 3),
                "/sys/fs/cgroup/memory.current": "1079107584"}
        output = io.StringIO()
        with mock.patch.object(Path, "read_text", autospec=True, side_effect=lambda path: data[path.as_posix()]), \
                mock.patch.object(Path, "open", side_effect=FileNotFoundError) as opened, mock.patch.object(sys, "stdout", output), \
                mock.patch.object(gate.shutil, "disk_usage", return_value=SimpleNamespace(free=5 * 1024 ** 3)) as disk:
            value = gate.resource_guard(Path("owned"))
            self.assertEqual(value["cgroup_limit_bytes"], 6 * 1024 ** 3)
            self.assertEqual(value["cgroup_current_bytes"], 1079107584)
            self.assertEqual(value["hosted_initial_cgroup_headroom_guard_bytes"], 3 * 1024 ** 3)
            self.assertEqual(value["hosted_disk_guard_bytes"], 5 * 1024 ** 3)
            self.assertIs(value["native_guards_unchanged"], True)
            for limit in (4 * 1024 ** 3, 5 * 1024 ** 3, 8 * 1024 ** 3, "max"):
                data["/sys/fs/cgroup/memory.max"] = str(limit)
                with self.assertRaisesRegex(ValueError, "^Hosted compiler cgroup must be bounded at 6 GiB$"):
                    gate.resource_guard(Path("owned"))
            opened.assert_not_called()
            self.assertEqual(output.getvalue(), "")
            data["/sys/fs/cgroup/memory.max"] = str(6 * 1024 ** 3)
            data["/sys/fs/cgroup/memory.current"] = str(3 * 1024 ** 3)
            gate.resource_guard(Path("owned"))
            disk.return_value.free -= 1
            with self.assertRaisesRegex(ValueError, "^Disposable hosted CI requires 5 GiB free; no waiver$"):
                gate.resource_guard(Path("owned"))
            disk.return_value.free += 1
            data["/sys/fs/cgroup/memory.current"] = str(3 * 1024 ** 3 + 1)
            with self.assertRaisesRegex(ValueError, "^Hosted initial cgroup headroom below 3 GiB$"):
                gate.resource_guard(Path("owned"))

    def test_hosted_six_gib_policy_keeps_170_identities_and_original_artifact_binding(self):
        import ast
        frozen = "431b7246c1927f220e63f7a644d4c9237c401c90"
        expected = self.source_blob(gate.HELPER, frozen)
        self.assertEqual(expected.count(b"== 4 * 1024 ** 3"), 1)
        self.assertEqual(expected.count(b"bounded at 4 GiB"), 2)
        expected = expected.replace(b"== 4 * 1024 ** 3", b"== 6 * 1024 ** 3").replace(b"bounded at 4 GiB", b"bounded at 6 GiB")
        self.assertEqual(self.source_blob(gate.HELPER, "8ce324536f450f3893ed95f1b43cf683048f0723"), expected)
        project = lambda data: [ast.dump(node) for node in ast.parse(data).body
                               if not isinstance(node, ast.FunctionDef)
                               or node.name not in {"safe_test_diagnostics", "validate_failure_evidence"}]
        self.assertEqual(project(self.source_blob(gate.HELPER, "78d3727e196ed17af3af371e3a56936e556cd7e8")), project(expected))
        for path in (gate.WORKFLOW, gate.GATE, gate.INIT, gate.INVENTORY):
            expected = self.source_blob(path, frozen)
            if path == gate.WORKFLOW:
                self.assertEqual(expected.count(b"--memory 4g"), 1)
                expected = expected.replace(b"--memory 4g", b"--memory 6g")
            actual = self.source_blob(path, "78d3727e196ed17af3af371e3a56936e556cd7e8") if path in (gate.WORKFLOW, gate.GATE, gate.INVENTORY) else (ROOT / path).read_bytes()
            self.assertEqual(actual, expected)
        names = lambda tree: {node.name for node in ast.walk(tree) if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        original = names(ast.parse(self.source_blob("scripts/tests/test_hosted_backend_gate.py", frozen)))
        self.assertEqual(len(original), 170)
        self.assertTrue(original <= names(ast.parse(Path(__file__).read_bytes())))
        binding = json.loads(self.source_blob(gate.INVENTORY,
            "78d3727e196ed17af3af371e3a56936e556cd7e8"))["config_union_preparation"]["codegen_evidence"]
        self.assertEqual(binding, json.loads(self.source_blob(gate.INVENTORY, frozen))["config_union_preparation"]["codegen_evidence"])
        self.assertEqual(binding["source_commit"], "aa11d3b90fcacb6f01a99b8a534cadbffbf54993")
        self.assertEqual(binding["binding_kind"], "verified_api_dependency_closure_parity")

    def test_cleanup_only_exact_owned_scratch_and_verified_identity(self):
        identity = dict(workflow_sha="a" * 40, run_id=1, run_attempt=1)
        with tempfile.TemporaryDirectory() as temporary:
            parent = Path(temporary).resolve()
            root = parent / "fleet-backend-config-union8c"
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
        self.assertEqual(artifact["with"]["path"], "${{ runner.temp }}/fleet-backend-evidence/")
        self.assertEqual(gate.ARTIFACT_FILES, {"report.json", "provenance.json", "SHA256SUMS"})
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

    def test_compiler_category_slice_preserves_frozen36_execution_and_200_identities(self):
        import ast
        frozen = "6843c4f9f050f2f3966abf20e1fc48156cd9a38c"
        before = ast.parse(self.source_blob(gate.HELPER, frozen))
        after = ast.parse((ROOT / gate.HELPER).read_bytes())
        functions = lambda tree: {n.name: ast.dump(n) for n in tree.body if isinstance(n, (ast.FunctionDef, ast.ClassDef))}
        old, new = functions(before), functions(after)
        self.assertEqual(old.keys(), new.keys())
        self.assertEqual({name for name in old if old[name] != new[name]},
                         {"safe_compiler_diagnostics", "validate_failure_evidence"})
        other_nodes = lambda tree: [ast.dump(n) for n in tree.body if not isinstance(n, (ast.FunctionDef, ast.ClassDef))]
        self.assertEqual(other_nodes(before), other_nodes(after))
        names = lambda tree: {n.name for n in ast.walk(tree) if isinstance(n, ast.FunctionDef) and n.name.startswith("test_")}
        old_names = names(ast.parse(self.source_blob("scripts/tests/test_hosted_backend_gate.py", frozen)))
        self.assertEqual(len(old_names), 201)
        self.assertEqual(names(ast.parse(Path(__file__).read_bytes())), old_names)
        for path in gate.WRITE_SET - {gate.HELPER, "scripts/tests/test_hosted_backend_gate.py"}:
            self.assertEqual((ROOT / path).read_bytes(), self.source_blob(path, frozen), path)

    def diagnostics(self, out, err=b""):
        return gate.safe_compiler_diagnostics((io.BytesIO(out), io.BytesIO(err)), REVIEWED["rust_source_sha256"], "/owned/src/fleet-control/backend")

    def test_compiler_json_only_code_allowlisted_primary_location_no_context(self):
        result = self.diagnostics(self.compiler_line())
        self.assertEqual(result, dict(diagnostics=[dict(error_code="E0308", file="backend/api/src/routes/sessions.rs", line=17, column=9)],
                                      categories=[], truncated=False))
        self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))
        for spans in ([], None, "PRIVATE_SENTINEL", [dict(file_name="/private/base/PRIVATE_SENTINEL.rs",
                      line_start=17, column_start=9, is_primary=True)]):
            with self.subTest(spans_type=type(spans).__name__):
                value = json.loads(self.compiler_line())
                value["message"]["spans"] = spans
                result = self.diagnostics((json.dumps(value) + "\n").encode())
                self.assertEqual(result, dict(diagnostics=[dict(error_code="E0308", file=None, line=None, column=None)],
                                             categories=[], truncated=False))
                self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))
        for category, patterns in gate.CATEGORY_PATTERNS.items():
            for pattern in patterns:
                with self.subTest(category=category, pattern=pattern):
                    value = json.loads(self.compiler_line())
                    value["message"]["message"] = pattern.upper() + " PRIVATE_SENTINEL /private/base/token"
                    result = self.diagnostics((json.dumps(value) + "\n").encode())
                    self.assertEqual(result["categories"], [category])
                    self.assertEqual(len(result["diagnostics"]), 1)
                    self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))
                    value["message"]["spans"] = []
                    self.assertEqual(self.diagnostics((json.dumps(value) + "\n").encode()),
                                     dict(diagnostics=[dict(error_code="E0308", file=None, line=None, column=None)],
                                          categories=[category], truncated=False))
        value = json.loads(self.compiler_line())
        value["message"]["spans"].append(dict(file_name="/private/PRIVATE_SENTINEL.rs",
                                             is_primary=True, line_start=1, column_start=1))
        self.assertEqual(self.diagnostics((json.dumps(value) + "\n").encode()),
                         self.diagnostics(self.compiler_line()))

    def test_compiler_paths_only_exact_fleet_allowlist_no_private_prefix_or_traversal(self):
        for file in ("api/src/routes/sessions.rs", "backend/api/src/routes/sessions.rs", "/owned/src/fleet-control/backend/api/src/routes/sessions.rs"):
            self.assertEqual(self.diagnostics(self.compiler_line(file))["diagnostics"][0]["file"], "backend/api/src/routes/sessions.rs")
        for file in ("../services-base/crates/private.rs", "/private/api/src/routes/sessions.rs", "C:/private/x.rs",
                     "api\\src\\routes\\sessions.rs", "/owned/src/fleet-control/backend/../private.rs",
                     "api//src/routes/sessions.rs", "api/src/routes/./sessions.rs", "api/.local/private.rs",
                     "target/private.rs", "api/src/routes/sessions.rs\nPRIVATE_SENTINEL", "api/src/private.rs"):
            result = self.diagnostics(self.compiler_line(file))
            self.assertEqual(result["diagnostics"], [dict(error_code="E0308", file=None, line=None, column=None)])
            self.assertNotIn(file, json.dumps(result))

    def test_crate_relative_span_requires_allowlisted_target_never_guesses(self):
        value = json.loads(self.compiler_line("src/routes/sessions.rs"))
        self.assertEqual(self.diagnostics((json.dumps(value) + "\n").encode())["diagnostics"],
                         [dict(error_code="E0308", file=None, line=None, column=None)])
        value["target"] = dict(src_path="/owned/src/fleet-control/backend/api/src/lib.rs")
        self.assertEqual(self.diagnostics((json.dumps(value) + "\n").encode())["diagnostics"][0]["file"], "backend/api/src/routes/sessions.rs")
        for entry in ("/private/base/src/lib.rs", "../api/src/lib.rs", "api/src/private.rs"):
            value["target"]["src_path"] = entry
            self.assertEqual(self.diagnostics((json.dumps(value) + "\n").encode())["diagnostics"],
                             [dict(error_code="E0308", file=None, line=None, column=None)])

    def test_compiler_untrusted_codes_and_numeric_boundaries(self):
        for code in (None, "clippy::private_token", "E1234 PRIVATE_SENTINEL", "E１２３４", ["E0308"], "PRIVATE_SENTINEL"):
            result = self.diagnostics(self.compiler_line(code=code))
            self.assertIsNone(result["diagnostics"][0]["error_code"])
            self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))
            self.assertEqual(self.diagnostics(self.compiler_line(file="/private/PRIVATE_SENTINEL.rs", code=code)),
                             dict(diagnostics=[], categories=["unknown"], truncated=False))
        for line, column in ((True, 9), (1, True), ("17", 9), (0, 9), (-1, 9), (1000001, 9), (17, 10001), (17, 0)):
            self.assertEqual(self.diagnostics(self.compiler_line(line=line, column=column))["diagnostics"],
                             [dict(error_code="E0308", file=None, line=None, column=None)])

    def test_compiler_only_error_primary_spans_not_rendered_children_or_artifacts(self):
        value = json.loads(self.compiler_line())
        for level in ("warning", "note", "help", "PRIVATE_SENTINEL"):
            value["message"]["level"] = level
            self.assertEqual(self.diagnostics((json.dumps(value) + "\n").encode())["diagnostics"], [])
        self.assertEqual(self.diagnostics(self.compiler_line(primary=False))["diagnostics"],
                         [dict(error_code="E0308", file=None, line=None, column=None)])
        value["reason"], value["message"]["level"] = "compiler-artifact", "error"
        self.assertEqual(self.diagnostics((json.dumps(value) + "\n").encode())["diagnostics"], [])
        for reason, level in (("compiler-message", "warning"), ("compiler-message", "note"),
                              ("compiler-message", "help"), ("compiler-artifact", "error"), (None, "error")):
            value = json.loads(self.compiler_line())
            value["reason"], value["message"]["level"] = reason, level
            value["message"]["message"] = "download of config.json failed PRIVATE_SENTINEL"
            result = self.diagnostics((json.dumps(value) + "\n").encode())
            self.assertEqual(result, dict(diagnostics=[], categories=["unknown"], truncated=False))
            self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))

    def test_nonjson_categories_fixed_allowlist_never_echo_raw(self):
        for category, patterns in gate.CATEGORY_PATTERNS.items():
            result = self.diagnostics(b"", (patterns[0] + " PRIVATE_SENTINEL /private/base/token\n").encode())
            self.assertEqual(result, dict(diagnostics=[], categories=[category], truncated=False))
            self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))
        self.assertEqual(self.diagnostics(b"PRIVATE_SENTINEL\n")["categories"], ["unknown"])
        for marker in ("download of config.json failed", "failed to get successful http response"):
            with self.subTest(marker=marker):
                result = self.diagnostics(b"", (marker + " PRIVATE_SENTINEL /private/base/token\n").encode())
                self.assertEqual(result, dict(diagnostics=[], categories=["network"], truncated=False))
                self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))

    def test_malformed_json_wrong_shapes_and_nested_context_fail_closed(self):
        for raw in (b"{malformed PRIVATE_SENTINEL\n", b"[]\n", b"null\n", b'{"reason":"compiler-message","message":"PRIVATE_SENTINEL"}\n',
                    b'{"reason":"compiler-message","message":{"level":"error","spans":"PRIVATE_SENTINEL"}}\n'):
            self.assertEqual(self.diagnostics(raw), dict(diagnostics=[], categories=["unknown"], truncated=False))
        marker = "download of config.json failed PRIVATE_SENTINEL"
        for message in (marker, None, [], {"level": "error", "message": [marker]},
                        {"level": "error", "message": {"private": marker}}):
            raw = (json.dumps(dict(reason="compiler-message", message=message)) + "\n").encode()
            self.assertEqual(self.diagnostics(raw), dict(diagnostics=[], categories=["unknown"], truncated=False))
        value = json.loads(self.compiler_line())
        value["message"]["rendered"] = marker
        value["message"]["children"] = [dict(message=marker)]
        value["private"] = marker
        self.assertEqual(self.diagnostics((json.dumps(value) + "\n").encode())["categories"], [])

    def test_compiler_deduplication_and_record_limit_are_bounded(self):
        same = self.compiler_line()
        self.assertEqual(len(self.diagnostics(same + same)["diagnostics"]), 1)
        result = self.diagnostics(b"".join(self.compiler_line(line=line) for line in range(1, 80)))
        self.assertEqual(len(result["diagnostics"]), gate.DIAGNOSTIC_LIMIT)
        self.assertTrue(result["truncated"])
        self.assertIn("unknown", result["categories"])
        same = self.compiler_line(file="/private/PRIVATE_SENTINEL.rs")
        self.assertEqual(self.diagnostics(same + same)["diagnostics"],
                         [dict(error_code="E0308", file=None, line=None, column=None)])
        result = self.diagnostics(b"".join(self.compiler_line(file="/private/PRIVATE_SENTINEL.rs", code=f"E{code:04d}")
                                           for code in range(80)))
        self.assertEqual(len(result["diagnostics"]), gate.DIAGNOSTIC_LIMIT)
        self.assertEqual(len({row["error_code"] for row in result["diagnostics"]}), gate.DIAGNOSTIC_LIMIT)
        self.assertTrue(result["truncated"])
        self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))
        # A full located record set must not gain a fallback for its last capped span.
        value = json.loads(self.compiler_line(line=80))
        value["message"]["spans"].append(dict(file_name="/private/PRIVATE_SENTINEL.rs", is_primary=True, line_start=1, column_start=1))
        result = self.diagnostics(b"".join(self.compiler_line(line=line) for line in range(1, 33)) +
                                  (json.dumps(value) + "\n").encode())
        self.assertTrue(result["truncated"])
        self.assertTrue(all(row["file"] is not None for row in result["diagnostics"]))

    def test_compiler_input_and_line_limits_discard_oversized_context(self):
        with mock.patch.object(gate, "DIAGNOSTIC_LINE_LIMIT", 64), mock.patch.object(gate, "DIAGNOSTIC_INPUT_LIMIT", 256):
            result = self.diagnostics(b"PRIVATE_SENTINEL" * 100 + b"\n" + self.compiler_line())
        self.assertTrue(result["truncated"])
        self.assertEqual(result["diagnostics"], [])
        self.assertNotIn("PRIVATE_SENTINEL", json.dumps(result))
        value = json.loads(self.compiler_line())
        value["message"]["message"] = "download of config.json failed PRIVATE_SENTINEL" * 20
        with mock.patch.object(gate, "DIAGNOSTIC_LINE_LIMIT", 64), mock.patch.object(gate, "DIAGNOSTIC_INPUT_LIMIT", 256):
            result = self.diagnostics((json.dumps(value) + "\n").encode())
        self.assertEqual(result, dict(diagnostics=[], categories=["unknown"], truncated=True))
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

    def test_stage_log_custody_distinguishes_missing_empty_unreadable_and_readable(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "private").mkdir()
            path = root / "private/pm_recovery_pg.log"
            project = lambda: gate.test_failure_logs(root, "pm_recovery_pg", REVIEWED)
            self.assertEqual(project()["stage_log"], dict(state="missing", harness_exit_code=None, harness_signal=None))
            path.write_bytes(b"")
            self.assertEqual(project()["stage_log"]["state"], "empty")
            path.write_bytes(b"PRIVATE_SENTINEL SQL env TOKEN\n")
            result = project()
            self.assertEqual(result["stage_log"]["state"], "readable")
            self.assertEqual(result["categories"], ["unknown"])
            self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
            for error in (PermissionError("PRIVATE_SENTINEL"), OSError("PRIVATE_SENTINEL")):
                with mock.patch.object(Path, "open", side_effect=error):
                    result = project()
                self.assertEqual(result["stage_log"]["state"], "unreadable")
                self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
            with mock.patch.object(gate, "safe_test_diagnostics", side_effect=ValueError("PRIVATE_SENTINEL")):
                self.assertEqual(project()["stage_log"]["state"], "unreadable")

    def test_stage_log_known_cargo_exit_and_signals_are_closed_metadata_only(self):
        header = "error: test failed, to rerun pass `-p infra --lib`\n\nCaused by:\n"
        cases = [("exit status: 101", 101, None), ("exit status: 255", 255, None),
                 ("signal: 6, SIGABRT: process abort signal", None, "SIGABRT"),
                 ("signal: 9, SIGKILL: kill", None, "SIGKILL"),
                 ("signal: 11, SIGSEGV: invalid memory reference", None, "SIGSEGV")]
        for detail, code, signal in cases:
            with self.subTest(detail=detail):
                raw = header + "  process didn't exit successfully: `/PRIVATE_SENTINEL --env TOKEN=secret` (" + detail + ")\n"
                result = gate.safe_test_diagnostics(io.BytesIO(raw.encode()), set(), {}, Path("/qa/backend"))
                self.assertEqual(result["stage_log"], dict(state="readable", harness_exit_code=code, harness_signal=signal))
                self.assertEqual(result["categories"], ["unknown"])
                self.assertEqual(result["failed_tests"], [])
                self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
                self.assertNotIn("TOKEN", gate.canonical(result).decode())

    def test_stage_log_unknown_unanchored_conflicting_markers_remain_unknown(self):
        header = "error: test failed, to rerun pass `-p infra --lib`\nCaused by:\n"
        line = "  process didn't exit successfully: `PRIVATE_SENTINEL` (signal: 9, SIGKILL: kill)\n"
        vectors = [line, "Caused by:\n" + line, header + "PRIVATE_SENTINEL\n" + line,
                   header + "prefix" + line, header + line.rstrip() + "suffix\n",
                   header + line.replace("9, SIGKILL: kill", "99, PRIVATE_SENTINEL"),
                   header + line.replace("9, SIGKILL: kill", "9, SIGSEGV: invalid memory reference"),
                   header + line.replace("signal: 9, SIGKILL: kill", "exit status: 256"),
                   header + line.replace("signal: 9, SIGKILL: kill", "exit status: 0"),
                   header + line + header + line.replace("9, SIGKILL: kill", "6, SIGABRT: process abort signal")]
        for raw in vectors:
            result = gate.safe_test_diagnostics(io.BytesIO(raw.encode()), set(), {}, Path("/qa/backend"))
            self.assertEqual(result["stage_log"], dict(state="readable", harness_exit_code=None, harness_signal=None))
            self.assertEqual(result["categories"], ["unknown"])
            self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_stage_log_markers_obey_existing_input_line_and_command_bounds(self):
        header = "error: test failed, to rerun pass `-p infra --lib`\nCaused by:\n"
        line = "  process didn't exit successfully: `PRIVATE_SENTINEL` (signal: 9, SIGKILL: kill)\n"
        for raw, limits in ((header + line + "PRIVATE_SENTINEL" * 100, dict(DIAGNOSTIC_INPUT_LIMIT=len(header + line) + 8)),
                            (header + line, dict(DIAGNOSTIC_LINE_LIMIT=32))):
            with mock.patch.multiple(gate, **limits):
                result = gate.safe_test_diagnostics(io.BytesIO(raw.encode()), set(), {}, Path("/qa/backend"))
            self.assertTrue(result["truncated"])
            self.assertIsNone(result["stage_log"]["harness_signal"])
            self.assertIsNone(result["stage_log"]["harness_exit_code"])
            self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
        raw = header + line.replace("PRIVATE_SENTINEL", "x" * 4097)
        result = gate.safe_test_diagnostics(io.BytesIO(raw.encode()), set(), {}, Path("/qa/backend"))
        self.assertIsNone(result["stage_log"]["harness_signal"])

    def test_stage_log_optional_schema_preserves_old_receipts_and_rejects_unsafe_types(self):
        old = self.failure_test_value()
        self.validate_failure(old)
        blank = dict(old, diagnostics=[], failed_tests=[], categories=["unknown"])
        for state in ("missing", "empty", "unreadable", "readable"):
            self.validate_failure(dict(blank, stage_log=dict(state=state, harness_exit_code=None, harness_signal=None)))
        log = dict(state="readable", harness_exit_code=None, harness_signal="SIGKILL")
        self.validate_failure(dict(blank, stage_log=log))
        for changes in (dict(state="PRIVATE_SENTINEL"), dict(state=True), dict(harness_exit_code=True),
                        dict(harness_exit_code=-1), dict(harness_exit_code=0), dict(harness_exit_code=256),
                        dict(harness_exit_code="101"), dict(harness_signal=9), dict(harness_signal="SIGTERM"),
                        dict(harness_signal="PRIVATE_SENTINEL"), dict(private="PRIVATE_SENTINEL"),
                        dict(harness_exit_code=101), dict(state="missing")):
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                self.validate_failure(dict(blank, stage_log=dict(log, **changes)))
        for invalid in (None, [], {}, dict(state="readable", harness_exit_code=None)):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(blank, stage_log=invalid))
        with self.assertRaises(ValueError):
            self.validate_failure(dict(blank, stage_log=log, truncated=True))
        with self.assertRaises(ValueError):
            self.validate_failure(dict(old, stage_log=dict(log, state="empty", harness_signal=None)))
        with self.assertRaises(ValueError):
            self.validate_failure(dict(self.failure_value(), stage_log=log))

    def test_stage_log_authenticated_failure_readback_accepts_both_closed_shapes_never_pass(self):
        old = self.failure_test_value()
        for value in (old, dict(old, stage_log=dict(state="readable", harness_exit_code=101, harness_signal=None))):
            run, artifact, payload, args = self.failure_artifact(value=value)
            self.assertEqual(json.loads(gate.validate_failure_readback(run, artifact, payload, **args)[gate.FAILURE_FILE]), value)
            with self.assertRaises(ValueError):
                gate.validate_readback(run, artifact, payload, **args)
        run, artifact, payload, args = self.failure_artifact(value=dict(old, stage_log=dict(state="PRIVATE_SENTINEL")))
        with self.assertRaises(ValueError):
            gate.validate_failure_readback(run, artifact, payload, **args)

    def test_aborted_test_identity_requires_complete_pair_and_abort_red_regression(self):
        name = REVIEWED["groups"]["pm_recovery_pg"][1]
        raw = (f"thread '{name}' has overflowed its stack\nfatal runtime error: stack overflow, aborting\n"
               "error: test failed, to rerun pass `-p infra --lib`\nCaused by:\n"
               "  process didn't exit successfully: `/PRIVATE_SENTINEL` (signal: 6, SIGABRT: process abort signal)\n")
        result = gate.safe_test_diagnostics(io.BytesIO(raw.encode()), {name}, {}, Path("/qa/backend"))
        self.assertEqual(result["failed_tests"], [name])
        self.assertEqual(result["categories"], ["runtime_stack_overflow", "test_failure"])
        self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_aborted_test_identity_conflicts_withhold_and_repeats_deduplicate(self):
        names = REVIEWED["groups"]["pm_recovery_pg"][1:3]
        pair = lambda name: f"thread '{name}' has overflowed its stack\nfatal runtime error: stack overflow, aborting\n"
        trailer = ("error: test failed, to rerun pass `-p infra --lib`\nCaused by:\n"
                   "  process didn't exit successfully: `PRIVATE_SENTINEL` (signal: 6, SIGABRT: process abort signal)\n")
        for raw, expected in ((pair(names[0]) * 2, [names[0]]),
                              (pair(names[0]) + pair(names[1]), []),
                              (pair(names[0]) + pair(names[1]) + pair(names[0]), [])):
            result = gate.safe_test_diagnostics(io.BytesIO((raw + trailer).encode()), set(names), {}, Path("/qa/backend"))
            self.assertEqual(result["failed_tests"], expected)
            self.assertIn("runtime_stack_overflow", result["categories"])
            self.assertFalse(result["truncated"])
            self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_aborted_test_identity_partial_foreign_or_unanchored_is_withheld(self):
        name = REVIEWED["groups"]["pm_recovery_pg"][1]
        pair = f"thread '{name}' has overflowed its stack\nfatal runtime error: stack overflow, aborting\n"
        trailer = ("error: test failed, to rerun pass `-p infra --lib`\nCaused by:\n"
                   "  process didn't exit successfully: `PRIVATE_SENTINEL TOKEN SQL` (signal: 6, SIGABRT: process abort signal)\n")
        vectors = [pair, pair + trailer.replace("signal: 6, SIGABRT: process abort signal", "exit status: 101"),
                   pair + trailer.replace("6, SIGABRT: process abort signal", "9, SIGKILL: kill"),
                   pair + trailer + trailer.replace("6, SIGABRT: process abort signal", "9, SIGKILL: kill")]
        for malformed in (pair.replace(name, "PRIVATE_SENTINEL"), pair.replace(name, "tokio-runtime-worker"),
                          "prefix" + pair, pair.replace("stack\n", "stack suffix\n"),
                          pair.replace("\nfatal", "\nPRIVATE_SENTINEL\nfatal"),
                          pair.replace("aborting\n", "aborting suffix\n"),
                          pair.replace("aborting\n", "aborting\r\r\n")):
            vectors.append(malformed + trailer)
        vectors.append(pair.removesuffix("\n"))
        for raw in vectors:
            result = gate.safe_test_diagnostics(io.BytesIO(raw.encode()), {name}, {}, Path("/qa/backend"))
            self.assertEqual(result["failed_tests"], [])
            self.assertNotIn("runtime_stack_overflow", result["categories"])
            for private in ("PRIVATE_SENTINEL", "TOKEN", "SQL", "tokio-runtime-worker"):
                self.assertNotIn(private, gate.canonical(result).decode())
        raw = (pair + trailer).encode()
        for limits in (dict(DIAGNOSTIC_INPUT_LIMIT=len(raw) - 1), dict(DIAGNOSTIC_LINE_LIMIT=32)):
            with mock.patch.multiple(gate, **limits):
                result = gate.safe_test_diagnostics(io.BytesIO(raw), {name}, {}, Path("/qa/backend"))
            self.assertTrue(result["truncated"])
            self.assertEqual(result["failed_tests"], [])
            self.assertIsNone(result["stage_log"]["harness_signal"])

    def test_aborted_test_identity_union_sorted_deduplicated_and_capped_at_32(self):
        names = sorted(gate.failure_test_names(REVIEWED, "workspace"))[:33]
        self.assertEqual(len(names), 33)
        pair = lambda name: f"thread '{name}' has overflowed its stack\nfatal runtime error: stack overflow, aborting\n"
        trailer = ("error: test failed, to rerun pass `-p infra --lib`\nCaused by:\n"
                   "  process didn't exit successfully: `PRIVATE_SENTINEL` (signal: 6, SIGABRT: process abort signal)\n")
        for completed, aborted, expected, truncated in ((names[:2][::-1], names[2], names[:3], False),
                (names[:32], names[0], names[:32], False), (names[:32], names[32], names[:32], True)):
            raw = "".join(f"test {name} ... FAILED\n" for name in completed) + pair(aborted) + trailer
            result = gate.safe_test_diagnostics(io.BytesIO(raw.encode()), set(names), {}, Path("/qa/backend"))
            self.assertEqual(result["failed_tests"], expected)
            self.assertEqual(result["truncated"], truncated)
            self.assertLessEqual(len(result["failed_tests"]), 32)
            self.assertEqual("runtime_stack_overflow" in result["categories"], not truncated)
            self.assertEqual(result["stage_log"]["harness_signal"], None if truncated else "SIGABRT")

    def test_aborted_test_identity_existing_failure_reader_and_legacy_compatibility(self):
        name = REVIEWED["groups"]["pm_recovery_pg"][1]
        raw = (f"thread '{name}' has overflowed its stack\nfatal runtime error: stack overflow, aborting\n"
               "error: test failed, to rerun pass `-p infra --lib`\nCaused by:\n"
               "  process didn't exit successfully: `PRIVATE_SENTINEL` (signal: 6, SIGABRT: process abort signal)\n")
        projected = gate.safe_test_diagnostics(io.BytesIO(raw.encode()), {name}, {}, Path("/qa/backend"))
        value = dict(self.failure_test_value(), stage="pm_recovery_pg", gate_failed_stage="pm_recovery_pg", **projected)
        legacy = dict(value, failed_tests=[], categories=["runtime_stack_overflow", "unknown"])
        for evidence in (value, legacy):
            self.validate_failure(evidence)
            run, artifact, payload, args = self.failure_artifact(value=evidence)
            self.assertEqual(json.loads(gate.validate_failure_readback(run, artifact, payload, **args)[gate.FAILURE_FILE]), evidence)
            with self.assertRaises(ValueError):
                gate.validate_readback(run, artifact, payload, **args)
        for bad in (["PRIVATE_SENTINEL"], [name, name], [name] * 33,
                    [REVIEWED["groups"]["credentials_pg"][0]]):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(value, failed_tests=bad))

    def test_aborted_test_identity_successor_preserves_f5_controls_and_189_test_bodies(self):
        import ast
        frozen = "f5ba26c11db095de5b14d080ce46dcbb998c960c"
        successor = "afab44ddcc06358904efa58942ea94546572693f"
        before, after = (ast.parse(data) for data in (self.source_blob(gate.HELPER, frozen), self.source_blob(gate.HELPER, successor)))
        functions = lambda tree: {node.name: ast.dump(node) for node in tree.body if isinstance(node, ast.FunctionDef)}
        old, new = functions(before), functions(after)
        self.assertEqual(old.keys(), new.keys())
        self.assertEqual({name for name in old if old[name] != new[name]}, {"safe_test_diagnostics"})
        nonfunctions = lambda tree: [ast.dump(node) for node in tree.body if not isinstance(node, ast.FunctionDef)]
        self.assertEqual(nonfunctions(before), nonfunctions(after))
        tests = lambda data: {node.name: ast.dump(node) for node in ast.walk(ast.parse(data))
            if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        old, new = tests(self.source_blob("scripts/tests/test_hosted_backend_gate.py", frozen)), tests(self.source_blob("scripts/tests/test_hosted_backend_gate.py", successor))
        self.assertEqual(len(old), 189)
        self.assertTrue(old.keys() <= new.keys())
        self.assertEqual({name for name in old if old[name] != new[name]}, {
            "test_rust_stack_overflow_complete_pair_and_abort_project_only_enum",
            "test_cb1_source_binding_is_exact_two_operand_allocation_changes_with_188_identities_preserved"})
        for path in (gate.WORKFLOW, gate.GATE, gate.INIT, gate.INVENTORY):
            self.assertEqual(self.source_blob(path, successor), self.source_blob(path, frozen), path)
        self.assertEqual(len(gate.GATES), 84)

    def test_rust_stack_overflow_complete_pair_and_abort_project_only_enum(self):
        name = REVIEWED["groups"]["pm_recovery_pg"][0]
        trailer = ("error: test failed, to rerun pass `-p infra --lib`\nCaused by:\n"
                   "  process didn't exit successfully: `/PRIVATE_SENTINEL TOKEN=secret` (signal: 6, SIGABRT: process abort signal)\n")
        header = f"thread '{name}' has overflowed its stack\nfatal runtime error: stack overflow, aborting\n"
        for newline in ("\n", "\r\n"):
            raw = (header + "PRIVATE_SENTINEL SQL\n" + trailer).replace("\n", newline).encode()
            result = gate.safe_test_diagnostics(io.BytesIO(raw), {name}, REVIEWED["rust_source_sha256"], Path("/qa/backend"))
            self.assertEqual(result["stage_log"], dict(state="readable", harness_exit_code=None, harness_signal="SIGABRT"))
            self.assertEqual(result["categories"], ["runtime_stack_overflow", "test_failure"])
            self.assertEqual(result["failed_tests"], [name])
            self.assertEqual(result["diagnostics"], [])
            for private in ("PRIVATE_SENTINEL", "TOKEN", "SQL"):
                self.assertNotIn(private, gate.canonical(result).decode())

    def test_pm_recovery_complete_markers_project_closed_layout_and_last_phase(self):
        phases = ("concurrent_entered", "lost_ack_entered", "revocation_entered", "fixture_entered",
                  "fixture_database_ready", "fixture_context_ready", "fixture_prepared", "submit_entered", "concurrent_submit_entered")
        for newline in ("\n", "\r\n"):
            for phase in phases:
                raw = ("\nFLEET_PM_RECOVERY_LAYOUT=1,4294967295\nPRIVATE_SENTINEL SQL\n"
                       "FLEET_PM_RECOVERY_PHASE=fixture_entered\nFLEET_PM_RECOVERY_PHASE=" + phase + "\n")
                result = gate.safe_test_diagnostics(io.BytesIO(raw.replace("\n", newline).encode()), set(), {},
                                                    Path("/qa/backend"), stage="pm_recovery_pg")
                self.assertEqual(result["pm_recovery_diagnostics"], dict(fixture_future_bytes=1,
                    submit_future_bytes=4294967295, last_phase=phase))
                self.assertEqual(result["categories"], ["unknown"])
                self.assertEqual(result["diagnostics"], [])
                self.assertEqual(result["failed_tests"], [])
                self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_pm_recovery_phase_enum_matches_exact_source_markers_and_metadata_selector(self):
        path = "backend/infra/src/runtime/pm_recovery_pg_tests.rs"
        source = self.source_blob(path).decode()
        emitted = re.findall(r'eprintln!\("\\nFLEET_PM_RECOVERY_PHASE=([a-z_]+)"\);', source)
        self.assertIsInstance(gate.PM_RECOVERY_PHASES, frozenset)
        self.assertEqual(len(emitted), 9)
        self.assertEqual(len(gate.PM_RECOVERY_PHASES), 9)
        self.assertEqual(set(emitted), gate.PM_RECOVERY_PHASES)
        selector = "runtime::pm_recovery::tests::pg::production_aaa_future_layout_without_constructing_or_polling_runtime"
        self.assertEqual(REVIEWED["groups"]["pm_recovery_pg"][0], selector)
        block = source[source.index("#[test]\n#[ignore"):source.index("#[tokio::test]", source.index("#[test]\n#[ignore"))]
        self.assertIn('"\\nFLEET_PM_RECOVERY_LAYOUT={},{}"', block)
        self.assertIn("fixture_bytes(fixture)", block)
        self.assertIn("submit_bytes(bounded_submit)", block)
        self.assertEqual(block.count("std::mem::size_of::<Fut>()"), 2)
        for unexpected in (".await", "fixture(", "bounded_submit(", "tokio::", "std::env::", "repository("):
            self.assertNotIn(unexpected, block)

    def test_bounded_submit_allocation_binding_is_single_wrapper_and_preserves_afab_195_identities(self):
        # Keep this historical transition bound to its reviewed frozen controls.
        historical = "50cb550af47e06530997fc1f84a5562fd4503b9e"
        REVIEWED = json.loads(self.source_blob(gate.INVENTORY, historical))
        import ast
        frozen = "afab44ddcc06358904efa58942ea94546572693f"
        cb1 = "cb1f62ee86477d60242cc05222ec923dd3c74e37"
        path = "backend/infra/src/runtime/pm_recovery_pg_tests.rs"
        call = b"submit(supervisor, &f.agent, intent, async { Ok(()) })"
        old, actual = self.source_blob(path, cb1), self.source_blob(path)
        self.assertEqual(old.count(call), 1)
        self.assertEqual(actual.count(b"Box::pin(" + call + b")"), 1)
        self.assertEqual(actual, old.replace(call, b"Box::pin(" + call + b")"))
        self.assertEqual(actual.replace(b"Box::pin(" + call + b")", call), old)
        self.assertEqual(gate.digest(actual), "56926dcd735da7e70ffabb0973871a0e3bc70949139f3788afa888c24fbecfc8")
        previous = json.loads(self.source_blob(gate.INVENTORY, frozen))
        expected = copy.deepcopy(previous)
        expected["source_commit"] = expected["config_union_preparation"]["source_commit"] = "5bc0fd3fd92a11a6957858525d9b124be00c1644"
        expected["compiled_source_sha256"]["fleet-control/" + path] = expected["rust_source_sha256"][path] = gate.digest(actual)
        expected["config_union_preparation"]["codegen_evidence"]["artifact_bound_source_commit"] = "5bc0fd3fd92a11a6957858525d9b124be00c1644"
        self.assertEqual(REVIEWED, expected)
        # Authentic artifact source remains AA11, never relabeled to the fixture child.
        self.assertEqual(REVIEWED["config_union_preparation"]["codegen_evidence"]["source_commit"],
                         "aa11d3b90fcacb6f01a99b8a534cadbffbf54993")
        helper = self.source_blob(gate.HELPER, frozen).replace(cb1.encode(), "5bc0fd3fd92a11a6957858525d9b124be00c1644".encode())
        helper = helper.replace(gate.digest(gate.canonical(previous["compiled_source_sha256"])).encode(),
                                "0f55274de1a4b602e0378eb47db48ee23aedd731b4e3d4782f01db811f48e5c0".encode())
        self.assertEqual(self.source_blob(gate.HELPER, historical), helper)
        self.assertEqual(self.source_blob(gate.WORKFLOW, historical),
                         self.source_blob(gate.WORKFLOW, frozen).replace(cb1.encode(), "5bc0fd3fd92a11a6957858525d9b124be00c1644".encode()))
        for file in (gate.GATE, gate.INIT):
            self.assertEqual(self.source_blob(file, historical), self.source_blob(file, frozen))
        tests = lambda data: {node.name: ast.dump(node) for node in ast.walk(ast.parse(data))
            if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        before, after = tests(self.source_blob("scripts/tests/test_hosted_backend_gate.py", frozen)), tests(self.source_blob("scripts/tests/test_hosted_backend_gate.py", historical))
        self.assertEqual(len(before), 195)
        self.assertTrue(before.keys() <= after.keys())
        self.assertEqual({name for name in before if before[name] != after[name]}, {
            "test_successor_source_tree_and_six_lf_controls_remain_closed",
            "test_resource_observation_successor_preserves_164_identities_guards_and_provenance",
            "test_aborted_test_identity_successor_preserves_f5_controls_and_189_test_bodies",
            "test_cb1_source_binding_is_exact_two_operand_allocation_changes_with_188_identities_preserved"})
        self.assertEqual(len(gate.GATES), 84)

    def test_cb1_source_binding_is_exact_two_operand_allocation_changes_with_188_identities_preserved(self):
        # Keep this historical transition bound to its reviewed frozen controls.
        historical = "50cb550af47e06530997fc1f84a5562fd4503b9e"
        REVIEWED = json.loads(self.source_blob(gate.INVENTORY, historical))
        import ast
        frozen = "c5ef8f64e7215659a3ec600846dab8e0e1c5fc93"
        original_source = "6768f6642ac5f08d2c77204f0e79ab356603e54f"
        cb1 = "cb1f62ee86477d60242cc05222ec923dd3c74e37"
        controls = "f5ba26c11db095de5b14d080ce46dcbb998c960c"
        reviewed = json.loads(self.source_blob(gate.INVENTORY, controls))
        path = "backend/infra/src/runtime/pm_recovery_pg_tests.rs"
        old = self.source_blob(path, original_source)
        expected = old
        for call in (b"submit(&f.supervisor, &f.agent, &f.intent, authorize())",
                     b"submit(&other, &f.agent, &f.intent, authorize())"):
            self.assertEqual(expected.count(call), 1)
            expected = expected.replace(call, b"Box::pin(" + call + b")")
        actual = self.source_blob(path, cb1)
        self.assertEqual(actual, expected)
        inverse = actual
        for call in (b"submit(&f.supervisor, &f.agent, &f.intent, authorize())",
                     b"submit(&other, &f.agent, &f.intent, authorize())"):
            inverse = inverse.replace(b"Box::pin(" + call + b")", call)
        self.assertEqual(inverse, old)
        previous = json.loads(self.source_blob(gate.INVENTORY, frozen))
        updated = copy.deepcopy(previous)
        updated["source_commit"] = updated["config_union_preparation"]["source_commit"] = cb1
        updated["compiled_source_sha256"]["fleet-control/" + path] = updated["rust_source_sha256"][path] = gate.digest(actual)
        binding = updated["config_union_preparation"]["codegen_evidence"]
        binding["artifact_bound_source_commit"] = cb1
        binding["source_delta"] = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "diff", "--name-only",
            binding["source_commit"], cb1], capture_output=True, check=True, timeout=30).stdout.decode().splitlines()
        self.assertEqual(reviewed, updated)
        gate.require_codegen_binding(reviewed)
        helper = self.source_blob(gate.HELPER, frozen).replace(original_source.encode(), cb1.encode())
        helper = helper.replace(gate.digest(gate.canonical(previous["compiled_source_sha256"])).encode(),
                                gate.digest(gate.canonical(reviewed["compiled_source_sha256"])).encode())
        without_parser = lambda data: ast.dump(ast.Module(body=[node for node in ast.parse(data).body
            if not (isinstance(node, ast.FunctionDef) and node.name == "safe_test_diagnostics")], type_ignores=[]))
        self.assertEqual(without_parser(self.source_blob(gate.HELPER, controls)), without_parser(helper))
        self.assertEqual(self.source_blob(gate.WORKFLOW, controls), self.source_blob(gate.WORKFLOW, frozen).replace(
            original_source.encode(), cb1.encode()))
        for file in (gate.GATE, gate.INIT):
            self.assertEqual(self.source_blob(file, historical), self.source_blob(file, frozen))
        identities = lambda data: {node.name for node in ast.walk(ast.parse(data))
            if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        before = identities(self.source_blob("scripts/tests/test_hosted_backend_gate.py", frozen))
        self.assertEqual(len(before), 188)
        self.assertTrue(before <= identities(self.source_blob("scripts/tests/test_hosted_backend_gate.py", historical)))

    def test_pm_recovery_nocapture_only_one_command_and_split_libtest_completions_are_exact(self):
        # Keep this historical transition bound to its reviewed frozen controls.
        historical = "50cb550af47e06530997fc1f84a5562fd4503b9e"
        REVIEWED = json.loads(self.source_blob(gate.INVENTORY, historical))
        frozen = "a7a5ac7f9ada5878436b88eefd8e7cad2a42a6a1"
        previous = self.source_blob(gate.GATE, frozen)
        command = b"runtime::pm_recovery::tests::pg:: -- --ignored --test-threads=1"
        self.assertEqual(previous.count(command), 1)
        self.assertEqual(self.source_blob(gate.GATE, historical), previous.replace(command, command + b" --nocapture"))
        names = REVIEWED["groups"]["pm_recovery_pg"]
        lines = ["running 4 tests"]
        for index, name in enumerate(names):
            lines.append("test " + name + " ... ")
            if index == 0:
                lines.extend(("FLEET_PM_RECOVERY_LAYOUT=123,456", "ok"))
            else:
                lines.extend(("FLEET_PM_RECOVERY_PHASE=fixture_entered", "", "FLEET_PM_RECOVERY_PHASE=submit_entered", "ok"))
        lines.append("test result: ok. 4 passed; 0 failed; 0 ignored;")
        text = "\n".join(lines) + "\n"
        expected = dict(passed=4, failed=0, ignored=0, tests=names)
        for newline in ("\n", "\r\n"):
            self.assertEqual(gate.verify_test_log("pm_recovery_pg", text.replace("\n", newline), REVIEWED), expected)
        self.assertEqual(gate.verify_test_log("pm_recovery_pg", self.log("pm_recovery_pg"), REVIEWED), expected)
        for bad in (text.replace("\nok\n", "\nFAILED\n", 1), text.replace("\nok\n", "\nignored\n", 1),
                    text.replace("\nok\n", "\n", 1), text.replace("4 passed", "3 passed"),
                    text.replace(names[0], "PRIVATE_SENTINEL"), text + "ok\n",
                    text.replace("FLEET_PM_RECOVERY_LAYOUT=123,456", "FLEET_PM_RECOVERY_LAYOUT=0,456"),
                    text.replace("FLEET_PM_RECOVERY_PHASE=fixture_entered", "PRIVATE_SENTINEL", 1),
                    text.replace("FLEET_PM_RECOVERY_PHASE=fixture_entered", "FLEET_PM_RECOVERY_PHASE=unknown", 1),
                    text.replace("\nok\n", "\ntest " + names[0] + " ... \nok\n", 1),
                    "FLEET_PM_RECOVERY_LAYOUT=123,456\nFLEET_PM_RECOVERY_PHASE=fixture_prepared\n"
                    "test result: ok. 4 passed; 0 failed; 0 ignored;\n"):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                gate.verify_test_log("pm_recovery_pg", bad, REVIEWED)

    def test_pm_recovery_nocapture_completion_normalization_preserves_every_prior_verifier_guard(self):
        # Keep this historical transition bound to its reviewed frozen controls.
        historical = "50cb550af47e06530997fc1f84a5562fd4503b9e"
        REVIEWED = json.loads(self.source_blob(gate.INVENTORY, historical))
        import ast
        frozen = "a7a5ac7f9ada5878436b88eefd8e7cad2a42a6a1"
        function = lambda data: next(node for node in ast.parse(data).body
            if isinstance(node, ast.FunctionDef) and node.name == "verify_test_log")
        old, new = function(self.source_blob(gate.HELPER, frozen)), function(self.source_blob(gate.HELPER, historical))
        self.assertEqual(ast.unparse(new.body[0].test), "stage == 'pm_recovery_pg'")
        del new.body[0]
        self.assertEqual(ast.dump(new), ast.dump(old))

    def test_pm_recovery_layout_inventory_is_only_one_ignored_diagnostic_and_exact_git_binding(self):
        # Keep this historical transition bound to its reviewed frozen controls.
        historical = "50cb550af47e06530997fc1f84a5562fd4503b9e"
        REVIEWED = json.loads(self.source_blob(gate.INVENTORY, historical))
        frozen = "b9a81c8cb375bfe87a6531e001e16a7a1f4da4b7"
        old = json.loads(self.source_blob(gate.INVENTORY, frozen))
        expected = copy.deepcopy(old)
        path = "backend/infra/src/runtime/pm_recovery_pg_tests.rs"
        source = self.source_blob(path).decode()
        added = "runtime::pm_recovery::tests::pg::production_aaa_future_layout_without_constructing_or_polling_runtime"
        records = [row for row in REVIEWED["ignored"] if row["source"] == path]
        self.assertEqual({row["name"] for row in records} - set(old["groups"]["pm_recovery_pg"]), {added})
        for row in records:
            name = row["name"].rsplit("::", 1)[-1]
            self.assertEqual(row["line"], source[:source.index("fn " + name + "(") + 3].count("\n") + 1)
            self.assertEqual({k: v for k, v in row.items() if k not in ("name", "line")},
                             dict(package="infra", source=path, target_kind="lib", target="infra", gate="pm_recovery_pg"))
        expected["ignored"] = sorted([row for row in old["ignored"] if row["source"] != path] + records,
                                     key=lambda row: (row["source"], row["line"]))
        expected["groups"]["pm_recovery_pg"] = sorted(row["name"] for row in records)
        expected["source_commit"] = expected["config_union_preparation"]["source_commit"] = "5bc0fd3fd92a11a6957858525d9b124be00c1644"
        expected["compiled_source_sha256"]["fleet-control/" + path] = expected["rust_source_sha256"][path] = gate.digest(source.encode())
        binding = expected["config_union_preparation"]["codegen_evidence"]
        binding["artifact_bound_source_commit"] = "5bc0fd3fd92a11a6957858525d9b124be00c1644"
        binding["source_delta"] = subprocess.run(["git", "--no-replace-objects", "-C", str(ROOT), "diff", "--name-only",
            binding["source_commit"], "5bc0fd3fd92a11a6957858525d9b124be00c1644"], capture_output=True, check=True, timeout=30).stdout.decode().splitlines()
        self.assertEqual(REVIEWED, expected)
        gate.require_codegen_binding(REVIEWED)
        self.assertEqual((len(REVIEWED["ignored"]), len(REVIEWED["workspace_default_declarations"])), (179, 400))
        self.assertEqual(len(REVIEWED["groups"]["pm_recovery_pg"]), 4)
        self.assertNotIn(added, REVIEWED["pm_workspace_required"])

    def test_pm_recovery_duplicate_malformed_or_partial_layout_never_projects_numbers(self):
        valid = "FLEET_PM_RECOVERY_LAYOUT=123,456\n"
        malformed = ("0,1", "1,0", "-1,2", "1,-2", "01,2", "1,02", "true,2", "1,False", "1.0,2",
                     "4294967296,1", "1,4294967296", "9" * 50 + ",1", "1", "1,2,3", "1,2 PRIVATE_SENTINEL", "")
        vectors = [valid + valid, valid + valid.replace("123", "125"), valid.removesuffix("\n")]
        for item in malformed:
            bad = "FLEET_PM_RECOVERY_LAYOUT=" + item + "\n"
            vectors.extend((bad, valid + bad, bad + valid))
        vectors.extend((valid + "FLEET_PM_RECOVERY_LAYOUT=3,4", valid + "FLEET_PM_RECOVERY_LAYOUT=3,4\r\r\n"))
        for raw in vectors:
            with self.subTest(raw=raw):
                result = gate.safe_test_diagnostics(io.BytesIO(("FLEET_PM_RECOVERY_PHASE=submit_entered\n" + raw).encode()),
                    set(), {}, Path("/qa/backend"), stage="pm_recovery_pg")
                self.assertEqual(result["pm_recovery_diagnostics"], dict(last_phase="submit_entered"))
                self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_pm_recovery_markers_require_exact_lines_stage_and_existing_bounds(self):
        good = b"FLEET_PM_RECOVERY_LAYOUT=123,456\nFLEET_PM_RECOVERY_PHASE=submit_entered\n"
        for stage in (None, "workspace", "foundation", "credentials_pg", "PRIVATE_SENTINEL"):
            result = gate.safe_test_diagnostics(io.BytesIO(good), set(), {}, Path("/qa/backend"), stage=stage)
            self.assertNotIn("pm_recovery_diagnostics", result)
        for raw in (b"prefix" + good.splitlines(keepends=True)[0], b"FLEET_PM_RECOVERY_PHASE=PRIVATE_SENTINEL\n",
                    b"FLEET_PM_RECOVERY_PHASE=submit_entered suffix\n", b"FLEET_PM_RECOVERY_PHASE=submit_entered\r\r\n",
                    b"FLEET_PM_RECOVERY_PHASE=submit_entered", b"test PRIVATE_SENTINEL ... " + good.splitlines(keepends=True)[0]):
            result = gate.safe_test_diagnostics(io.BytesIO(raw), set(), {}, Path("/qa/backend"), stage="pm_recovery_pg")
            self.assertNotIn("pm_recovery_diagnostics", result)
        for limits in (dict(DIAGNOSTIC_INPUT_LIMIT=len(good) - 1), dict(DIAGNOSTIC_LINE_LIMIT=16)):
            with mock.patch.multiple(gate, **limits):
                result = gate.safe_test_diagnostics(io.BytesIO(good), set(), {}, Path("/qa/backend"), stage="pm_recovery_pg")
            self.assertTrue(result["truncated"])
            self.assertNotIn("pm_recovery_diagnostics", result)

    def test_pm_recovery_stage_log_readback_integration_only_for_owned_recovery_stage(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "private").mkdir()
            for stage in ("pm_recovery_pg", "credentials_pg"):
                (root / "private" / (stage + ".log")).write_bytes(
                    b"\nFLEET_PM_RECOVERY_LAYOUT=123,456\nFLEET_PM_RECOVERY_PHASE=fixture_prepared\n")
                result = gate.test_failure_logs(root, stage, REVIEWED)
                self.assertEqual("pm_recovery_diagnostics" in result, stage == "pm_recovery_pg")
            result = gate.test_failure_logs(root, "foundation", REVIEWED)
            self.assertNotIn("pm_recovery_diagnostics", result)
            self.assertEqual(result["stage_log"]["state"], "missing")

    def test_pm_recovery_optional_closed_schema_rejects_unsafe_values_and_anchors(self):
        blank = dict(self.failure_test_value(), stage="pm_recovery_pg", gate_failed_stage="pm_recovery_pg",
                     diagnostics=[], failed_tests=[], categories=["unknown"],
                     stage_log=dict(state="readable", harness_exit_code=None, harness_signal=None))
        layout = dict(fixture_future_bytes=1, submit_future_bytes=4294967295)
        for data in (layout, dict(last_phase="fixture_entered"), dict(layout, last_phase="submit_entered")):
            self.validate_failure(dict(blank, pm_recovery_diagnostics=data))
        self.validate_failure(blank)
        invalid = [None, [], True, {}, dict(layout, private="PRIVATE_SENTINEL"), dict(fixture_future_bytes=1),
                   dict(submit_future_bytes=1), dict(last_phase="PRIVATE_SENTINEL"), dict(last_phase=True)]
        for key in layout:
            invalid.extend(dict(layout, **{key: item}) for item in (True, False, 0, -1, 4294967296, "1", 1.0, None, []))
        for data in invalid:
            with self.subTest(data=data), self.assertRaises(ValueError):
                self.validate_failure(dict(blank, pm_recovery_diagnostics=data))
        for changes in (dict(stage="credentials_pg", gate_failed_stage="credentials_pg"), dict(truncated=True),
                        dict(stage_log=None), dict(stage_log=dict(state="missing", harness_exit_code=None, harness_signal=None))):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(blank, pm_recovery_diagnostics=layout, **changes))
        with self.assertRaises(ValueError):
            self.validate_failure(dict(self.failure_value(), pm_recovery_diagnostics=layout))
        without_log = dict(blank, pm_recovery_diagnostics=layout)
        del without_log["stage_log"]
        with self.assertRaises(ValueError):
            self.validate_failure(without_log)

    def test_pm_recovery_authenticated_optional_receipt_is_never_success_and_old_receipts_still_validate(self):
        blank = dict(self.failure_test_value(), stage="pm_recovery_pg", gate_failed_stage="pm_recovery_pg",
                     diagnostics=[], failed_tests=[], categories=["unknown"],
                     stage_log=dict(state="readable", harness_exit_code=None, harness_signal=None))
        for data in ({}, dict(pm_recovery_diagnostics=dict(fixture_future_bytes=123, submit_future_bytes=456,
                                                         last_phase="fixture_prepared"))):
            value = dict(blank, **data)
            run, artifact, payload, args = self.failure_artifact(value=value)
            self.assertEqual(json.loads(gate.validate_failure_readback(run, artifact, payload, **args)[gate.FAILURE_FILE]), value)
            with self.assertRaises(ValueError):
                gate.validate_readback(run, artifact, payload, **args)
        value = dict(blank, pm_recovery_diagnostics=dict(last_phase="PRIVATE_SENTINEL"))
        run, artifact, payload, args = self.failure_artifact(value=value)
        with self.assertRaises(ValueError):
            gate.validate_failure_readback(run, artifact, payload, **args)

    def test_pm_recovery_diagnostic_successor_preserves_177_identities_and_all_execution_guards(self):
        # Keep this historical transition bound to its reviewed frozen controls.
        historical = "50cb550af47e06530997fc1f84a5562fd4503b9e"
        REVIEWED = json.loads(self.source_blob(gate.INVENTORY, historical))
        import ast
        frozen = "b9a81c8cb375bfe87a6531e001e16a7a1f4da4b7"
        functions = lambda data: {node.name: ast.dump(node) for node in ast.parse(data).body
                                  if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
        before, after = functions(self.source_blob(gate.HELPER, frozen)), functions(self.source_blob(gate.HELPER, historical))
        self.assertEqual(before.keys(), after.keys())
        self.assertEqual({name for name in before if before[name] != after[name]},
                         {"safe_test_diagnostics", "test_failure_logs", "validate_failure_evidence", "reviewed_inventory",
                          "verify_test_log", "verify_runtime_inventory", "execute", "validate_evidence_files"})
        actual = self.source_blob(gate.HELPER, historical)
        normalized = re.sub(rb"\b179\b", b"178", actual)
        normalized = normalized.replace(b'len(value["groups"]["pm_recovery_pg"]) == 4',
                                        b'len(value["groups"]["pm_recovery_pg"]) == 3')
        normalized = normalized.replace(b'len(set(value["groups"]["pm_recovery_pg"])) == 4',
                                        b'len(set(value["groups"]["pm_recovery_pg"])) == 3')
        def project(data):
            return [ast.dump(node) for node in ast.parse(data).body
                if not (isinstance(node, ast.FunctionDef)
                        and node.name in {"safe_test_diagnostics", "test_failure_logs", "validate_failure_evidence", "verify_test_log"})
                and not (isinstance(node, ast.Assign) and isinstance(node.targets[0], ast.Name)
                         and node.targets[0].id in {"SOURCE_SHA", "SOURCE_INVENTORY_SHA", "PM_RECOVERY_PHASES"})]
        self.assertEqual(project(self.source_blob(gate.HELPER, frozen)), project(normalized))
        tests = lambda data: {node.name for node in ast.walk(ast.parse(data))
                              if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        original = tests(self.source_blob("scripts/tests/test_hosted_backend_gate.py", frozen))
        self.assertEqual(len(original), 177)
        self.assertTrue(original <= tests(self.source_blob("scripts/tests/test_hosted_backend_gate.py", historical)))
        for path in (gate.INIT,):
            self.assertEqual((ROOT / path).read_bytes(), self.source_blob(path, frozen), path)
        self.assertEqual(self.source_blob(gate.WORKFLOW, historical), self.source_blob(gate.WORKFLOW, frozen).replace(
            b"b97e1e6933d1c6156afc629708b53204cff6680a", "5bc0fd3fd92a11a6957858525d9b124be00c1644".encode()))
        self.assertEqual(len(gate.GATES), 84)

    def test_rust_stack_overflow_partial_foreign_prefix_suffix_stay_unknown(self):
        name = REVIEWED["groups"]["pm_recovery_pg"][0]
        trailer = ("error: test failed, to rerun pass `-p infra --lib`\nCaused by:\n"
                   "  process didn't exit successfully: `PRIVATE_SENTINEL` (signal: 6, SIGABRT: process abort signal)\n")
        stack = f"thread '{name}' has overflowed its stack\nfatal runtime error: stack overflow, aborting\n"
        vectors = ["fatal runtime error: stack overflow, aborting\n", stack.replace(name, "PRIVATE_SENTINEL"),
                   stack.replace("\nfatal", "\nPRIVATE_SENTINEL\nfatal"), "prefix" + stack,
                   stack.replace("stack\n", "stack suffix\n"), stack.replace("aborting\n", "aborting suffix\n"),
                   stack.replace("aborting\n", "aborting\r\r\n"),
                   stack.replace(name, "tokio-runtime-worker"), stack.replace(name, "<unknown>"),
                   "memory allocation of 0 bytes failed\n", "memory allocation of -1 bytes failed\n",
                   "memory allocation of " + "9" * 21 + " bytes failed\n",
                   "prefix memory allocation of 1 bytes failed\n", "memory allocation of 1 bytes failed suffix\n",
                   "memory allocation of 1 bytes failed\r\r\n", "thread caused non-unwinding panic. aborting.\n",
                   "thread panicked while processing panic. aborting.\n", "fatal runtime error: PRIVATE_SENTINEL\n"]
        for header in vectors:
            result = gate.safe_test_diagnostics(io.BytesIO((header + trailer).encode()), {name}, {}, Path("/qa/backend"))
            self.assertEqual(result["categories"], ["unknown"])
            self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
        result = gate.safe_test_diagnostics(io.BytesIO(b"memory allocation of 1 bytes failed"), {name}, {}, Path("/qa/backend"))
        self.assertEqual(result["categories"], ["unknown"])

    def test_rust_stack_overflow_requires_unambiguous_abort_and_existing_bounds(self):
        name = REVIEWED["groups"]["pm_recovery_pg"][0]
        header = f"thread '{name}' has overflowed its stack\nfatal runtime error: stack overflow, aborting\n"
        trailer = ("error: test failed, to rerun pass `-p infra --lib`\nCaused by:\n"
                   "  process didn't exit successfully: `PRIVATE_SENTINEL` (signal: 6, SIGABRT: process abort signal)\n")
        for raw in (header, header + trailer.replace("6, SIGABRT: process abort signal", "9, SIGKILL: kill"),
                    header + trailer + trailer.replace("6, SIGABRT: process abort signal", "9, SIGKILL: kill")):
            result = gate.safe_test_diagnostics(io.BytesIO(raw.encode()), {name}, {}, Path("/qa/backend"))
            self.assertEqual(result["categories"], ["unknown"])
        raw = (header + trailer).encode()
        for limits in (dict(DIAGNOSTIC_LINE_LIMIT=32), dict(DIAGNOSTIC_INPUT_LIMIT=len(raw) - 1)):
            with mock.patch.multiple(gate, **limits):
                result = gate.safe_test_diagnostics(io.BytesIO(raw), {name}, {}, Path("/qa/backend"))
            self.assertTrue(result["truncated"])
            self.assertEqual(result["categories"], ["unknown"])

    def test_rust_stack_overflow_strict_category_old_compatibility_readback_never_pass(self):
        blank = dict(self.failure_test_value(), diagnostics=[], failed_tests=[], categories=["unknown"])
        log = dict(state="readable", harness_exit_code=None, harness_signal="SIGABRT")
        value = dict(blank, stage_log=log, categories=["runtime_stack_overflow", "unknown"])
        self.validate_failure(value)
        run, artifact, payload, args = self.failure_artifact(value=value)
        self.assertEqual(json.loads(gate.validate_failure_readback(run, artifact, payload, **args)[gate.FAILURE_FILE]), value)
        with self.assertRaises(ValueError):
            gate.validate_readback(run, artifact, payload, **args)
        self.validate_failure(dict(blank, stage_log=log))
        self.validate_failure(blank)
        for category in (None, True, 1, [], {}, "PRIVATE_SENTINEL", "runtime_stack_overflow SQL", "allocation_failure"):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(blank, stage_log=log, categories=[category]))
        for change in (dict(harness_signal=None), dict(harness_signal="SIGKILL"), dict(state="missing"),
                       dict(harness_exit_code=101), dict(private="PRIVATE_SENTINEL")):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(value, stage_log=dict(log, **change)))
        with self.assertRaises(ValueError):
            self.validate_failure(dict(blank, categories=["runtime_stack_overflow", "unknown"]))
        with self.assertRaises(ValueError):
            self.validate_failure(dict(value, stage_log=dict(log, rust_fatal="stack_overflow")))
        with self.assertRaises(ValueError):
            self.validate_failure(dict(value, truncated=True))

    def test_rust_stack_overflow_successor_preserves_172_selectors_execution_and_source(self):
        import ast
        frozen = "8ce324536f450f3893ed95f1b43cf683048f0723"
        before = ast.parse(self.source_blob(gate.HELPER, frozen))
        after = ast.parse(self.source_blob(gate.HELPER, "b9a81c8cb375bfe87a6531e001e16a7a1f4da4b7"))
        functions = lambda tree: {node.name: ast.dump(node) for node in tree.body if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
        old, new = functions(before), functions(after)
        self.assertEqual(old.keys(), new.keys())
        self.assertEqual({name for name in old if old[name] != new[name]}, {"safe_test_diagnostics", "validate_failure_evidence"})
        constants = lambda tree: [ast.dump(node) for node in tree.body if isinstance(node, (ast.Assign, ast.AnnAssign))]
        self.assertEqual(constants(before), constants(ast.parse(self.source_blob(gate.HELPER,
            "78d3727e196ed17af3af371e3a56936e556cd7e8"))))
        tests = lambda tree: {node.name for node in ast.walk(tree) if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        original = tests(ast.parse(self.source_blob("scripts/tests/test_hosted_backend_gate.py", frozen)))
        self.assertEqual(len(original), 172)
        self.assertTrue(original <= tests(ast.parse(Path(__file__).read_bytes())))
        for path in (gate.WORKFLOW, gate.GATE, gate.INIT, gate.INVENTORY):
            actual = self.source_blob(path, "78d3727e196ed17af3af371e3a56936e556cd7e8") if path in (gate.WORKFLOW, gate.GATE, gate.INVENTORY) else (ROOT / path).read_bytes()
            self.assertEqual(actual, self.source_blob(path, frozen), path)
        self.assertEqual(len(gate.GATES), 84)
        for path in ("backend/infra/src/runtime/pm_recovery.rs", "backend/infra/src/runtime/pm_recovery_pg_tests.rs"):
            self.assertEqual((ROOT / path).read_bytes().replace(b"\r\n", b"\n"), self.source_blob(path))

    def test_stage_log_successor_preserves_151_identities_and_all_execution_guards(self):
        import ast
        frozen = "5b3ddd3ae164a5136e068550ec2e8c98e1234668"
        before = ast.parse(self.source_blob(gate.HELPER, frozen))
        after = ast.parse(self.source_blob(gate.HELPER, "b9a81c8cb375bfe87a6531e001e16a7a1f4da4b7"))
        functions = lambda tree: {node.name: ast.dump(node) for node in tree.body
                                  if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
        old, new = functions(before), functions(after)
        self.assertEqual(old.keys(), new.keys())
        self.assertEqual({name for name in old if old[name] != new[name]},
                         {"safe_test_diagnostics", "test_failure_logs", "validate_failure_evidence", "execute", "main", "resource_guard"})
        constants = lambda tree: {node.targets[0].id: ast.dump(node.value) for node in tree.body
                                  if isinstance(node, ast.Assign) and isinstance(node.targets[0], ast.Name)}
        old_constants = constants(before)
        new_constants = constants(ast.parse(self.source_blob(gate.HELPER, "78d3727e196ed17af3af371e3a56936e556cd7e8")))
        self.assertEqual(set(new_constants) - set(old_constants), {"CONTROL_PHASE"})
        self.assertEqual(old_constants, {key: value for key, value in new_constants.items() if key != "CONTROL_PHASE"})
        tests = lambda tree: {node.name for node in ast.walk(tree)
                              if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        original = tests(ast.parse(self.source_blob("scripts/tests/test_hosted_backend_gate.py", frozen)))
        self.assertEqual(len(original), 151)
        self.assertTrue(original <= tests(ast.parse(Path(__file__).read_bytes())))
        for path in (gate.WORKFLOW, gate.GATE, gate.INIT, gate.INVENTORY):
            expected = self.source_blob(path, frozen)
            if path == gate.WORKFLOW:
                self.assertEqual(expected.count(b"--memory 4g"), 1)
                expected = expected.replace(b"--memory 4g", b"--memory 6g")
            actual = self.source_blob(path, "78d3727e196ed17af3af371e3a56936e556cd7e8") if path in (gate.WORKFLOW, gate.GATE, gate.INVENTORY) else (ROOT / path).read_bytes()
            self.assertEqual(actual, expected, path)

    def catch_projection(self, run):
        budget = SimpleNamespace(arm=lambda: None, close=lambda: None, cleanup=lambda **_: None)
        output = io.StringIO()
        with mock.patch.object(sys, "argv", [gate.HELPER, "execute"]), mock.patch.object(gate, "hosted_budget", return_value=budget), \
                mock.patch.object(sys, "stdout", output), mock.patch.object(gate.signal, "setitimer", create=True), \
                mock.patch.object(gate.signal, "ITIMER_REAL", 0, create=True):
            code = run()
        self.assertEqual(code, 1)
        lines = output.getvalue().splitlines()
        self.assertEqual(lines[0], "Backend control failed; no private diagnostics emitted; acceptance withheld")
        self.assertEqual(len(lines), 2)
        value = json.loads(lines[1])
        self.assertEqual(set(value), {"state", "control_phase", "exception_class", "reason", "http_status",
                                     "backend_quality_gate", "all_quality_gate", "sdlc_acceptance"})
        self.assertEqual(value["state"], "backend_control_exception")
        for key in ("backend_quality_gate", "all_quality_gate", "sdlc_acceptance"):
            self.assertIs(value[key], False)
        self.assertNotIn("PRIVATE_SENTINEL", output.getvalue())
        self.assertTrue(value["http_status"] is None or type(value["http_status"]) is int
                        and value["http_status"] in {400, 401, 403, 404, 408, 409, 422, 429, 500, 502, 503, 504})
        self.assertIsNone(gate.BUDGET)
        return value

    def test_outer_catch_early_execute_predicates_emit_only_closed_phase_class_reason(self):
        cases = [("preflight", "execute_preflight", ValueError("PRIVATE_SENTINEL"), "value_error", "unknown"),
                 ("clean_head", "checkout_qualification", ValueError("Dirty checkout"), "value_error", "checkout_dirty"),
                 ("command", "compiler_version", gate.CommandFailed(1), "command_failed", "unknown"),
                 ("reviewed_inventory", "inventory_binding", ValueError("Input fingerprint drift"), "value_error", "inventory_hash"),
                 ("qualify_utility", "utility_qualification", ValueError("Utility source/worktree drift"), "value_error", "utility_bytes"),
                 ("qualify_package", "package_qualification", ValueError("Canonical package origin required"), "value_error", "package_origin"),
                 ("resource_guard", "resource_guard", ValueError("Hosted compiler cgroup must be bounded at 6 GiB"), "value_error", "cgroup_limit"),
                 ("resource_guard", "resource_guard", ValueError("Hosted initial cgroup headroom below 3 GiB"), "value_error", "cgroup_headroom"),
                 ("resource_guard", "resource_guard", ValueError("Disposable hosted CI requires 5 GiB free; no waiver"), "value_error", "disk_floor")]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "fleet-control").mkdir()
            (root / "fleet-control/.base-revision").write_text(gate.BASE_SHA)
            for name, phase, error, kind, reason in cases:
                mocks = dict(preflight=mock.Mock(return_value=(root, ROOT, "a" * 40)), clean_head=mock.Mock(),
                             command=mock.Mock(return_value=b"rustc 1.88.0"), reviewed_inventory=mock.Mock(return_value=REVIEWED),
                             qualify_utility=mock.Mock(), qualify_package=mock.Mock(), resource_guard=mock.Mock())
                mocks[name].side_effect = error
                with self.subTest(name=name, reason=reason), mock.patch.multiple(gate, **mocks), \
                        mock.patch.dict(os.environ, RUNNER_TEMP=str(root)):
                    value = self.catch_projection(gate.main)
                    self.assertEqual((value["control_phase"], value["exception_class"], value["reason"]), (phase, kind, reason))

    def test_outer_catch_unknown_private_messages_subclasses_and_phase_stay_unknown(self):
        class Foreign(ValueError):
            pass

        for error in (RuntimeError("PRIVATE_SENTINEL SQL TOKEN"), ValueError("Dirty checkout PRIVATE_SENTINEL"),
                      ValueError("x" * 10000), Foreign("Dirty checkout"), TypeError("PRIVATE_SENTINEL")):
            def fail():
                gate.CONTROL_PHASE = "PRIVATE_SENTINEL"
                raise error
            with mock.patch.object(gate, "execute", side_effect=fail):
                value = self.catch_projection(gate.main)
            self.assertEqual(value["control_phase"], "unknown")
            self.assertEqual(value["reason"], "unknown")
            self.assertIn(value["exception_class"], {"unknown", "value_error", "type_error"})

    def test_outer_catch_budget_failures_remain_closed_and_never_invoke_execute(self):
        for phase in ("hosted_budget", "budget_arm"):
            def run():
                if phase == "hosted_budget":
                    with mock.patch.object(gate, "hosted_budget", side_effect=PermissionError("PRIVATE_SENTINEL")):
                        return gate.main()
                budget = SimpleNamespace(arm=mock.Mock(side_effect=TimeoutError("PRIVATE_SENTINEL")), close=lambda: None)
                with mock.patch.object(gate, "hosted_budget", return_value=budget):
                    return gate.main()
            with mock.patch.object(gate, "execute") as execute:
                value = self.catch_projection(run)
                execute.assert_not_called()
            self.assertEqual(value["control_phase"], phase)
            self.assertEqual(value["reason"], "unknown")
            self.assertEqual(value["exception_class"], "permission_error" if phase == "hosted_budget" else "timeout")

    def test_outer_catch_reason_labels_are_literal_source_attested_not_error_fragments(self):
        import ast
        tree = ast.parse((ROOT / gate.HELPER).read_bytes())
        main = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == "main")
        reasons = next(ast.literal_eval(node.value) for node in ast.walk(main) if isinstance(node, ast.Assign)
                       and any(isinstance(target, ast.Name) and target.id == "reasons" for target in node.targets))
        frozen = ast.parse(self.source_blob(gate.HELPER, "73b33f4422df8aa19fc81e45dabf901c00e792df"))
        labels = {node.args[1].value for node in ast.walk(frozen) if isinstance(node, ast.Call)
                  and isinstance(node.func, ast.Name) and node.func.id == "require" and len(node.args) == 2
                  and isinstance(node.args[1], ast.Constant)}
        labels.update(node.args[0].value for node in ast.walk(frozen) if isinstance(node, ast.Call)
                      and isinstance(node.func, ast.Name) and node.func.id in ("ValueError", "TimeoutError")
                      and len(node.args) == 1 and isinstance(node.args[0], ast.Constant))
        self.assertIn("Hosted compiler cgroup must be bounded at 6 GiB", {
            node.args[1].value for node in ast.walk(tree) if isinstance(node, ast.Call)
            and isinstance(node.func, ast.Name) and node.func.id == "require" and len(node.args) == 2
            and isinstance(node.args[1], ast.Constant)})
        labels.add("Hosted compiler cgroup must be bounded at 6 GiB")
        self.assertTrue(set(reasons) <= labels)
        self.assertEqual(len(reasons), 30)
        self.assertTrue(all(re.fullmatch(r"[a-z_]{1,40}", code) for code in reasons.values()))

    def test_outer_catch_hosted_metadata_labels_and_http_status_are_closed(self):
        import urllib.error
        labels = {
            "Execution restricted to the exact dedicated hosted branch push": "hosted_identity",
            "Missing workflow SHA": "workflow_sha", "Missing run/attempt": "run_attempt",
            "Unexpected hosted job inventory": "job_inventory", "Hosted job identity drift": "job_identity",
            "Invalid hosted job clock": "job_clock", "Hosted job elapsed budget exhausted": "job_exhausted",
            "Invalid hosted job budget": "job_budget", "Hosted elapsed budget exhausted": "budget_exhausted",
            "Hosted metadata credentials missing": "metadata_credentials", "Hosted metadata redirect forbidden": "metadata_redirect",
            "Hosted metadata deadline exhausted": "metadata_deadline", "Hosted metadata unavailable": "metadata_unavailable",
            "Hosted metadata oversized": "metadata_oversized"}
        for label, reason in labels.items():
            kind = TimeoutError if label in ("Hosted elapsed budget exhausted", "Hosted metadata deadline exhausted") else ValueError
            def fail():
                with mock.patch.object(gate, "hosted_budget", side_effect=kind(label)):
                    return gate.main()
            value = self.catch_projection(fail)
            self.assertEqual((value["control_phase"], value["reason"], value["http_status"]), ("hosted_budget", reason, None))
        for code in (400, 401, 403, 404, 408, 409, 422, 429, 500, 502, 503, 504, 200, 999, True, "403"):
            error = urllib.error.HTTPError("https://PRIVATE_SENTINEL", code, "PRIVATE_SENTINEL", {"Authorization": "PRIVATE_SENTINEL"}, None)
            def fail_http():
                with mock.patch.object(gate, "hosted_budget", side_effect=error):
                    return gate.main()
            value = self.catch_projection(fail_http)
            self.assertEqual(value["exception_class"], "http_error")
            self.assertEqual(value["reason"], "unknown")
            self.assertEqual(value["http_status"], code if type(code) is int and code not in (200, 999) else None)

    def test_outer_catch_successor_keeps_every_original_guard_and_158_test_identity(self):
        import ast
        frozen = "73b33f4422df8aa19fc81e45dabf901c00e792df"
        old_tree = ast.parse(self.source_blob(gate.HELPER, frozen))
        new_tree = ast.parse(self.source_blob(gate.HELPER, "b9a81c8cb375bfe87a6531e001e16a7a1f4da4b7"))
        functions = lambda tree: {node.name: node for node in tree.body if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
        before, after = functions(old_tree), functions(new_tree)
        self.assertEqual(before.keys(), after.keys())
        self.assertEqual({name for name in before if ast.dump(before[name]) != ast.dump(after[name])},
                         {"execute", "main", "resource_guard", "safe_test_diagnostics", "validate_failure_evidence"})

        class RemoveCheckpoints(ast.NodeTransformer):
            def visit_Global(self, node):
                node.names = [name for name in node.names if name != "CONTROL_PHASE"]
                return node if node.names else None

            def visit_Assign(self, node):
                return None if any(isinstance(target, ast.Name) and target.id == "CONTROL_PHASE" for target in node.targets) else node

        self.assertEqual(ast.dump(before["execute"]), ast.dump(RemoveCheckpoints().visit(copy.deepcopy(after["execute"]))))
        for path in (gate.WORKFLOW, gate.GATE, gate.INIT, gate.INVENTORY):
            expected = self.source_blob(path, frozen)
            if path == gate.WORKFLOW:
                self.assertEqual(expected.count(b"--memory 4g"), 1)
                expected = expected.replace(b"--memory 4g", b"--memory 6g")
            actual = self.source_blob(path, "78d3727e196ed17af3af371e3a56936e556cd7e8") if path in (gate.WORKFLOW, gate.GATE, gate.INVENTORY) else (ROOT / path).read_bytes()
            self.assertEqual(actual, expected)
        names = lambda tree: {node.name for node in ast.walk(tree) if isinstance(node, ast.FunctionDef) and node.name.startswith("test_")}
        original = names(ast.parse(self.source_blob("scripts/tests/test_hosted_backend_gate.py", frozen)))
        self.assertEqual(len(original), 158)
        self.assertTrue(original <= names(ast.parse(Path(__file__).read_bytes())))

    def failure_value(self):
        return dict(version=1, kind="safe_compiler_failure", status="failure", repository=gate.REPOSITORY, branch=gate.BRANCH,
                    workflow_sha="a" * 40, workflow_path=gate.WORKFLOW, run_id=123, run_attempt=1,
                    source_sha=gate.SOURCE_SHA, base_sha=gate.BASE_SHA, auth_sha=gate.AUTH_SHA,
                    utility_sha=gate.UTILITY_SHA, utility_inventory_sha256=gate.UTILITY_INVENTORY_SHA,
                    package_sha=gate.PACKAGE_SHA, package_tree=gate.PACKAGE_TREE,
                    package_inventory_sha256=gate.PACKAGE_INVENTORY_SHA,
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
        code_only = dict(error_code="E0308", file=None, line=None, column=None)
        self.validate_failure(dict(value, diagnostics=[code_only]))
        self.validate_failure(dict(value, diagnostics=[value["diagnostics"][0], code_only]))
        for code in (None, "E0308 PRIVATE_SENTINEL", "E１２３４", "clippy::private", True, ["E0308"]):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(value, diagnostics=[dict(code_only, error_code=code)]))
        located = value["diagnostics"][0]
        for mask in range(1, 7):
            partial = dict(code_only)
            for bit, key in enumerate(("file", "line", "column")):
                if mask & (1 << bit):
                    partial[key] = located[key]
            with self.assertRaises(ValueError):
                self.validate_failure(dict(value, diagnostics=[partial]))
        for records in ([code_only, code_only],
                        [dict(code_only, error_code=f"E{code:04d}") for code in range(gate.DIAGNOSTIC_LIMIT + 1)]):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(value, diagnostics=records))
        for code in (None, "E0308"):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(self.failure_test_value(), diagnostics=[dict(code_only, error_code=code)]))
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
                                     diagnostics=[dict(error_code=None, file="backend/" + path, line=300, column=5)],
                                     stage_log=dict(state="readable", harness_exit_code=None, harness_signal=None)))
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

    def test_owned_custom_panic_hints_are_fixed_and_match_pinned_migration_source(self):
        source = "\n".join(self.source_blob(path).decode() for path in (
            "backend/migration/src/m20261009_000015_container_controller.rs",
            "backend/migration/src/m20261009_000016_mapped_controller_recovery.rs",
            "backend/migration/src/m20261009_000019_recovered_activation.rs"))
        for category, message in gate.TEST_CUSTOM_HINTS.items():
            with self.subTest(category=category):
                self.assertIn('"' + message + '"', source)
                result = self.hint_diagnostics('called Result::unwrap() on Err value: Custom("' + message + '") PRIVATE_SENTINEL /private/token')
                self.assertEqual(result["categories"], sorted([category, "test_failure"]))
                self.assertNotIn(message, gate.canonical(result).decode())
                self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
                self.validate_failure(dict(self.failure_test_value(), **result))

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
        detail = 'Custom("Hermes guard is missing")'
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
        detail = 'Custom("Hermes guard is missing")'
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
        self.assertEqual(result["categories"], ["hermes_guard_missing", "test_failure"])
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
            result = self.hint_diagnostics('Custom("Hermes guard is missing")')
        self.assertTrue(result["truncated"])
        self.assertNotIn("hermes_guard_missing", result["categories"])
        data = "".join(f"thread 't' panicked at infra/tests/support/pm_credential_creation.rs:{i + 1}:5:\n"
                       'Custom("Hermes guard is missing") PRIVATE_SENTINEL\n' for i in range(gate.DIAGNOSTIC_LIMIT + 1))
        result = gate.safe_test_diagnostics(io.BytesIO(data.encode()), set(), REVIEWED["rust_source_sha256"], Path("/qa/src/fleet-control/backend"))
        self.assertEqual(len(result["diagnostics"]), gate.DIAGNOSTIC_LIMIT)
        self.assertTrue(result["truncated"])
        self.assertEqual(result["categories"], ["hermes_guard_missing", "test_failure"])
        self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_panic_hint_schema_rejects_unknown_values_unanchored_or_compiler_categories(self):
        value = self.failure_test_value()
        for categories in (["PRIVATE_SENTINEL"], ["null_definition_decode=PRIVATE_SENTINEL", "test_failure"],
                           ["hermes_guard_missing", "hermes_guard_missing", "test_failure"],
                           ["test_failure", "hermes_guard_missing"], ["hermes_guard_missing"],
                           [True], [{"PRIVATE_SENTINEL": "body"}], ["network", "test_failure"]):
            with self.subTest(categories=categories), self.assertRaises(ValueError):
                self.validate_failure(dict(value, categories=categories))
        with self.assertRaises(ValueError):
            self.validate_failure(dict(value, categories=["hermes_guard_missing", "test_failure"], diagnostics=[]))
        with self.assertRaises(ValueError):
            self.validate_failure(dict(self.failure_value(), categories=["hermes_guard_missing"]))

    def test_panic_hint_authenticated_readback_accepts_old_schema_and_fixed_hints_only(self):
        for categories in (["test_failure"], ["unknown"], ["hermes_guard_missing", "null_body_decode", "test_failure"]):
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

    def test_journal_groups_use_safe_test_failure_without_cross_group_identity_or_acceptance(self):
        for stage in ("clarification_domain", "clarification_api", "clarification_pg", "clarification_migration"):
            name = REVIEWED["groups"][stage][0]
            foreign = REVIEWED["groups"]["credentials_pg"][0]
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
        artifact.update(name="fleet-backend-failure-config-union8c-123-1", digest="sha256:" + gate.digest(payload))
        args["artifact_digest"] = gate.digest(payload)
        return run, artifact, payload, args

    def test_failure_readback_authenticates_failure_run_attempt_sha_artifact_digest(self):
        run, artifact, payload, args = self.failure_artifact()
        self.assertEqual(set(gate.validate_failure_readback(run, artifact, payload, **args)), {gate.FAILURE_FILE})
        for key, item in (("id", 456), ("run_attempt", 2), ("conclusion", "success"), ("event", "pull_request"), ("head_sha", "b" * 40)):
            with self.assertRaises(ValueError):
                gate.validate_failure_readback(dict(run, **{key: item}), artifact, payload, **args)
        for key, item in (("expired", True), ("name", "fleet-backend-config-union8c-123-1"), ("digest", "sha256:" + "0" * 64)):
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
        # Positive evidence fixtures exercise the validator, not pending source qualification.
        self.enterContext(mock.patch.object(gate, "reviewed_inventory", return_value=self.synthetic_bound_inventory()))
        workflow_sha = "a" * 40
        focused = {name: dict(passed=len(names), failed=0, ignored=125 if name == "foundation" else 0,
                              tests=sorted(names)) for name, names in REVIEWED["groups"].items()}
        focused["workspace"] = dict(passed=1, failed=0, ignored=180, tests=["normal"])
        report = dict(backend_quality_gate=True, all_quality_gate=False, sdlc_acceptance=False, status="success",
                      gates=[dict(stage=name, status="passed") for name in gate.GATES], focused=focused,
                      cleanup=dict(scratch=True, synthetic_databases=True), ignored_required=180, foundation_ignored=125,
                      contracts={stage: dict(passed=len(names), failed=0, ignored=0, tests=sorted(names))
                                 for stage, names in REVIEWED["python_contracts"].items()},
                      migration_ledger=dict(snapshots=gate.expected_migration_receipt(REVIEWED), applied_at_preserved=True),
                      runtime_inventory=dict(ignored=180, listed_default_count=1, ignored_names_sha256=gate.digest(gate.canonical(
                          sorted(item["name"] for item in REVIEWED["ignored"])))))
        report.update(report_changes or {})
        provenance = dict(version=1, repository=gate.REPOSITORY, branch=gate.BRANCH, source_sha=gate.SOURCE_SHA,
                          base_sha=gate.BASE_SHA, auth_sha=gate.AUTH_SHA, workflow_sha=workflow_sha, workflow_path=gate.WORKFLOW,
                          utility_sha=gate.UTILITY_SHA, utility_tree=REVIEWED["utility_tree"],
                          utility_inventory_sha256=gate.UTILITY_INVENTORY_SHA, source_inventory_sha256=gate.SOURCE_INVENTORY_SHA,
                          package_sha=gate.PACKAGE_SHA, package_tree=gate.PACKAGE_TREE,
                          package_inventory_sha256=gate.PACKAGE_INVENTORY_SHA,
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
                        name="fleet-backend-config-union8c-123-1", digest="sha256:" + gate.digest(payload))
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

    def evidence_fixture(self, directory):
        _, _, payload, args = self.artifact()
        root = Path(directory) / "evidence"
        root.mkdir()
        with zipfile.ZipFile(io.BytesIO(payload)) as archive:
            for name in gate.ARTIFACT_FILES:
                (root / name).write_bytes(archive.read(name))
        identity = {name: args[name] for name in ("workflow_sha", "run_id", "attempt")}
        return root, identity

    def test_preupload_verifies_same_complete_evidence_as_authenticated_readback(self):
        with tempfile.TemporaryDirectory() as directory:
            root, identity = self.evidence_fixture(directory)
            gate.verify_evidence_directory(root, **identity)
            for name in ("provenance.json", "SHA256SUMS"):
                original = (root / name).read_bytes()
                (root / name).unlink()
                with self.subTest(missing=name), self.assertRaises(ValueError):
                    gate.verify_evidence_directory(root, **identity)
                (root / name).write_bytes(original)

    def test_preupload_rejects_private_extra_file_directory_and_checksum_drift(self):
        with tempfile.TemporaryDirectory() as directory:
            root, identity = self.evidence_fixture(directory)
            extra = root / "PRIVATE_SENTINEL.log"
            extra.write_bytes(b"PRIVATE_SENTINEL")
            with self.assertRaises(ValueError):
                gate.verify_evidence_directory(root, **identity)
            extra.unlink()
            sums = root / "SHA256SUMS"
            original = sums.read_bytes()
            sums.unlink()
            sums.mkdir()
            with self.assertRaises(ValueError):
                gate.verify_evidence_directory(root, **identity)
            sums.rmdir()
            sums.write_bytes(original + b"tampered")
            with self.assertRaises(ValueError):
                gate.verify_evidence_directory(root, **identity)

    @unittest.skipUnless(os.name == "posix", "Linux symlink fixture")
    def test_preupload_rejects_linked_directory_and_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root, identity = self.evidence_fixture(directory)
            alias = Path(directory) / "alias"
            alias.symlink_to(root, target_is_directory=True)
            with self.assertRaises(ValueError):
                gate.verify_evidence_directory(alias, **identity)
            path = root / "provenance.json"
            target = Path(directory) / "provenance.json"
            path.replace(target)
            path.symlink_to(target)
            with self.assertRaises(ValueError):
                gate.verify_evidence_directory(root, **identity)

    def test_preupload_rejects_stale_run_and_incomplete_gate_without_readback_bypass(self):
        with tempfile.TemporaryDirectory() as directory:
            root, identity = self.evidence_fixture(directory)
            with self.assertRaises(ValueError):
                gate.verify_evidence_directory(root, **dict(identity, run_id=999))
            files = {name: (root / name).read_bytes() for name in gate.ARTIFACT_FILES}
            report = json.loads(files["report.json"])
            report["gates"] = report["gates"][:-1]
            files["report.json"] = gate.canonical(report)
            files["SHA256SUMS"] = "".join(gate.digest(files[name]) + "  " + name + "\n"
                                          for name in ("report.json", "provenance.json")).encode()
            for name, data in files.items():
                (root / name).write_bytes(data)
            with self.assertRaises(ValueError):
                gate.verify_evidence_directory(root, **identity)

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
        names = gate.GATES[gate.GATES.index("container_control_unit"):-3]
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

    def test_all_22_databases_init_environment_and_finally_match_exactly(self):
        created = re.findall(r"^CREATE DATABASE (\w+)", (ROOT / gate.INIT).read_text(), re.M)
        self.assertEqual(set(created) | {"fleet_foundation_test"}, set(gate.DATABASES))
        self.assertEqual(len(created), 23)
        self.assertEqual(set(gate.URLS.values()), set(gate.DATABASES))
        shell = (ROOT / gate.GATE).read_text()
        for name in gate.URLS:
            self.assertIn(name, shell)
        with mock.patch.object(gate, "psql") as psql, mock.patch.object(gate, "database_catalog", return_value=["postgres"]):
            gate.drop_databases(list(gate.DATABASES))
            self.assertEqual(psql.call_count, 25)
            self.assertTrue(all(call.args[0].startswith("DROP DATABASE IF EXISTS") for call in psql.call_args_list[:-1]))

    def migration_files(self, root, expected):
        for stage, names in expected.items():
            (root / ("migration-" + stage + ".txt")).write_text("".join(n + "\n" for n in names), newline="\n")
            (root / ("migration-" + stage + "-ledger.tsv")).write_text(
                "".join(n + "\t1000\n" for n in names), newline="\n")

    def test_migration_smoke_exact21_20_19_20_21_0_21_not_count_waiver(self):
        expected = gate.expected_migration_receipt(REVIEWED)
        self.assertEqual([len(v) for v in expected.values()], [26, 25, 24, 23, 22, 21, 20, 19, 20, 21, 22, 23, 24, 25, 26, 0, 26])
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            self.migration_files(root, expected)
            self.assertEqual(gate.verify_migration_snapshots(root, REVIEWED), dict(snapshots=expected, applied_at_preserved=True))
            (root / "migration-down_all.txt").write_text("foreign_history\n", newline="\n")
            with self.assertRaises(ValueError):
                gate.verify_migration_snapshots(root, REVIEWED)
        text = (ROOT / gate.GATE).read_text()
        self.assertIn("cargo run --locked -p migration -- down -n 26", text)
        for name in expected:
            self.assertIn("migration_snapshot " + name, text)
        self.assertEqual(len(REVIEWED["migration_registries"]["split"]), 29)
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

    def test_fallback_drops_only_persisted_exact_owned22_before_scratch_removal(self):
        with tempfile.TemporaryDirectory() as temporary:
            parent = Path(temporary).resolve()
            root = parent / "fleet-backend-config-union8c"
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
        self.assertEqual(len(REVIEWED["compiled_source_sha256"]), 404)
        self.assertEqual(len(REVIEWED["utility_source_sha256"]), 10)
        for name in REVIEWED["compiled_source_sha256"]:
            self.assertFalse({".local", ".git", "target", "node_modules", "__pycache__", ".venv"}.intersection(Path(name).parts))
        self.assertIn("scripts/check_recovered_activation_contract.py", gate.FLEET_ROOTS)
        self.assertNotIn("scripts", gate.FLEET_ROOTS)

    def test_config_union_commands_have_exact_noncolliding_filters_and_scoped_database(self):
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
        self.assertLess(positions[-1], shell.index("run_tests clarification_domain"))
        candidates = sum((REVIEWED["groups"][stage] for stage in (
            "base_package_unit", "config_files_unit", "package_effective_unit")), [])
        self.assertEqual(sorted(name for name in candidates if "tests::base_package_effective_" in name),
                         REVIEWED["groups"]["package_effective_unit"])
        for stage in ("base_package_pg", "config_revision_pg"):
            self.assertIn('FLEET_TEST_DATABASE_URL="$FLEET_CONFIGURATION_TEST_DATABASE_URL" \\\n  run_tests ' + stage, shell)
        self.assertNotEqual(gate.database_environment()["FLEET_TEST_DATABASE_URL"],
                            gate.database_environment()["FLEET_CONFIGURATION_TEST_DATABASE_URL"])

    def test_config_union_all_new_named_groups_reject_missing_duplicate_and_count_only(self):
        for stage in ("config_api", "base_package_unit", "config_files_unit", "package_effective_unit",
                      "config_shared_unit", "base_package_pg", "config_revision_pg", "container_activation_intent"):
            text = self.log(stage)
            self.assertEqual(gate.verify_test_log(stage, text, REVIEWED)["passed"], len(REVIEWED["groups"][stage]))
            first = text.splitlines()[0]
            for bad in (text.replace(first + "\n", "", 1), text + first + "\n", text.splitlines()[-1],
                        text.replace(" ... ok", " ... ignored", 1)):
                with self.subTest(stage=stage), self.assertRaises(ValueError):
                    gate.verify_test_log(stage, bad, REVIEWED)
        old = json.loads(self.source_blob(gate.INVENTORY, "a7d7db205ee2ef7e480b5dc559156ad373135180"))
        added = set(REVIEWED["groups"]["container_activation_intent"]) - set(old["groups"]["container_activation_intent"])
        self.assertEqual(len(added), 6)
        self.assertEqual(added, set(REVIEWED["config_union_preparation"]["new_named_activation_cases"]))

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
        self.assertEqual(len({gate.PACKAGE_SHA, gate.BASE_SHA, gate.UTILITY_SHA}), 3)

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
                      dict(REVIEWED["package_input"], commit=gate.UTILITY_SHA),
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
        self.assertEqual(helper.count("qualify_package(checkouts[4][0], reviewed)"), 2)
        self.assertIn("FLEET_TEST_BASE_PACKAGE_CHECKOUT=str(checkouts[4][0])", helper)
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
                mock.patch.object(gate, "qualify_utility"), \
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

    def probe_diagnostics(self, detail, *, thread=None, path="infra/tests/container_activation.rs", prefix=""):
        name = REVIEWED["groups"]["container_activation_pg"][0]
        text = prefix + f"thread '{thread or name}' panicked at {path}:110:5:\n" + detail + "\n"
        return gate.safe_test_diagnostics(io.BytesIO(text.encode()), {name}, REVIEWED["rust_source_sha256"],
                                          Path("/qa/src/fleet-control/backend"))

    def test_all_exact_activation_probe_literals_emit_only_closed_hint_and_location(self):
        self.assertEqual(len(gate.TEST_ACTIVATION_HINTS), 37)
        source = self.source_blob(gate.ACTIVATION_PROBE_SOURCE).decode()
        helper = source.split("async fn assert_recovered_preconditions(", 1)[1].split("async fn recovered_step(", 1)[0]
        self.assertEqual(set(re.findall(r'"(activation_probe_[a-z_]+)"', helper)), gate.TEST_ACTIVATION_HINTS)
        for hint in gate.TEST_ACTIVATION_HINTS:
            with self.subTest(hint=hint):
                result = self.probe_diagnostics(hint)
                self.assertEqual(result["categories"], [hint, "test_failure"])
                value = dict(self.failure_test_value(), **result, stage="container_activation_pg",
                             gate_failed_stage="container_activation_pg")
                self.validate_failure(value)
                for flag in ("backend_quality_gate", "all_quality_gate", "sdlc_acceptance"):
                    self.assertFalse(value[flag])

    def test_activation_probe_hints_reject_prefix_suffix_sql_debug_and_delayed_details(self):
        hint = "activation_probe_stored_lease_exact"
        for detail in ("PRIVATE_SENTINEL " + hint, hint + " PRIVATE_SENTINEL", " " + hint, hint + " ",
                       'Custom("' + hint + '")', 'assertion failed: ' + hint,
                       "activation_probe_unknown", hint + "_unknown", "blank\n" + hint,
                       "SELECT '" + hint + "';"):
            result = self.probe_diagnostics(detail)
            self.assertEqual(result["categories"], ["test_failure"])
            self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_activation_probe_hints_reject_foreign_threads_paths_and_frame_boundaries(self):
        hint = "activation_probe_stored_lease_exact"
        for options in (dict(thread="foreign"), dict(path="/qa/src/services-base/private.rs"),
                        dict(path="infra/tests/support/pm_credential_creation.rs")):
            result = self.probe_diagnostics(hint, **options)
            self.assertNotIn(hint, result["categories"])
        for boundary in ("thread 'foreign' panicked at infra/tests/container_activation.rs:110:5:",
                         "thread 'malformed' panicked at PRIVATE_SENTINEL", "test result: FAILED",
                         "test foreign ... FAILED"):
            result = self.probe_diagnostics(boundary + "\n" + hint)
            self.assertNotIn(hint, result["categories"])
            self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
        result = gate.safe_test_diagnostics(io.BytesIO(hint.encode()), set(), REVIEWED["rust_source_sha256"], Path("/qa/backend"))
        self.assertEqual(result["categories"], ["unknown"])
        owned = REVIEWED["groups"]["container_activation_pg"][0]
        result = self.probe_diagnostics(f"thread '{owned}' panicked at infra/tests/container_activation.rs:111:5:\n" + hint)
        self.assertIn(hint, result["categories"])

    def test_activation_probe_hints_keep_input_line_count_and_closed_schema_bounds(self):
        hint = "activation_probe_stored_lease_exact"
        with mock.patch.object(gate, "DIAGNOSTIC_LINE_LIMIT", 128):
            result = self.probe_diagnostics("PRIVATE_SENTINEL" * 20 + hint)
        self.assertTrue(result["truncated"])
        self.assertNotIn(hint, result["categories"])
        with mock.patch.object(gate, "DIAGNOSTIC_INPUT_LIMIT", 8):
            result = self.probe_diagnostics(hint)
        self.assertTrue(result["truncated"])
        self.assertNotIn(hint, result["categories"])
        with mock.patch.object(gate, "DIAGNOSTIC_LIMIT", 0):
            result = self.probe_diagnostics(hint)
        self.assertTrue(result["truncated"])
        self.assertNotIn(hint, result["categories"])
        value = dict(self.failure_test_value(), diagnostics=[dict(error_code=None, file=gate.ACTIVATION_PROBE_SOURCE, line=110, column=5)])
        for categories in ([hint], ["activation_probe_unknown", "test_failure"], [hint, hint, "test_failure"]):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(value, categories=categories))
        for changes in (dict(diagnostics=[]), dict(diagnostics=self.failure_test_value()["diagnostics"]),
                        dict(kind="safe_compiler_failure")):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(value, categories=[hint, "test_failure"], **changes))

    def test_authorize_successor_preserves_every_predecessor_ast_guard_and_selector(self):
        import ast
        predecessor = "084d9f0f7b94953251b58a912b32b92cedbda020"
        old = ast.parse(self.source_blob(gate.HELPER, predecessor))
        new = ast.parse(self.source_blob(gate.HELPER, "9565ecc1c2d114d44132598d77f4c5942d440d1d"))
        functions = lambda tree: {n.name: n for n in tree.body if isinstance(n, (ast.FunctionDef, ast.ClassDef))}
        before, after = functions(old), functions(new)
        for name in ("safe_test_diagnostics", "safe_compiler_diagnostics", "validate_failure_evidence",
                     "validate_failure_readback", "require_codegen_binding", "resource_guard", "cleanup_fallback"):
            self.assertEqual(ast.dump(before[name]), ast.dump(after[name]), name)
        previous = json.loads(self.source_blob(gate.INVENTORY, predecessor))
        for stage, names in previous["groups"].items():
            if stage in ("foundation", "container_activation_pg", "credentials_pg", "lineage10", "config_shared_unit"):
                self.assertTrue(set(names) < set(REVIEWED["groups"][stage]))
            else:
                self.assertEqual(names, REVIEWED["groups"][stage], stage)
        for key in ("authority", "utility_source_sha256", "utility_tree", "package_input", "python_contracts"):
            self.assertEqual(REVIEWED[key], previous[key], key)
        for path, digest in previous["compiled_source_sha256"].items():
            if not path.startswith("fleet-control/"):
                self.assertEqual(REVIEWED["compiled_source_sha256"][path], digest, path)

    def test_authorize_labels_are_exact_source_attested_unavailable_not_custom(self):
        expected = {"activation_authorize_" + label for label in (
            "begin", "lock", "agent", "readback", "receipt_decode", "receipt_validation", "current", "config",
            "authority_insert", "commit")}
        self.assertEqual(set(gate.TEST_AUTHORIZE_HINTS), expected)
        source = self.source_blob("backend/infra/src/container_activation.rs").decode()
        self.assertEqual(set(re.findall(r'"(activation_authorize_[a-z_]+)"', source)), expected)
        self.assertIn('AppError::Unavailable(stage.into())', source)
        for hint, detail in gate.TEST_AUTHORIZE_HINTS.items():
            self.assertEqual(detail, 'called `Result::unwrap()` on an `Err` value: Unavailable("' + hint + '")')
            result = self.probe_diagnostics(detail)
            self.assertEqual(result["categories"], [hint, "test_failure"])
            self.assertNotIn("Unavailable", gate.canonical(result).decode())

    def test_authorize_details_reject_private_prefix_suffix_debug_sql_and_unknown_labels(self):
        for hint, detail in gate.TEST_AUTHORIZE_HINTS.items():
            for text in (hint, 'Unavailable("' + hint + '")', 'Custom("' + hint + '")',
                         "PRIVATE_SENTINEL " + detail, detail + " PRIVATE_SENTINEL", " " + detail, detail + " ",
                         detail.replace(hint, hint + "_unknown"), "SELECT '" + detail + "';", "blank\n" + detail):
                with self.subTest(hint=hint, detail=text):
                    result = self.probe_diagnostics(text)
                    self.assertEqual(result["categories"], ["test_failure"])
                    self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())

    def test_authorize_details_require_immediate_owned_panic_and_reset_foreign_frames(self):
        detail = next(iter(gate.TEST_AUTHORIZE_HINTS.values()))
        for options in (dict(thread="foreign"), dict(path="/qa/src/services-base/private.rs"),
                        dict(path="infra/tests/support/pm_credential_creation.rs")):
            result = self.probe_diagnostics(detail, **options)
            self.assertFalse(set(result["categories"]) & gate.TEST_AUTHORIZE_HINTS.keys())
        for boundary in ("thread 'foreign' panicked at infra/tests/container_activation.rs:111:5:",
                         "thread 'malformed' panicked at PRIVATE_SENTINEL", "test result: FAILED",
                         "test foreign ... FAILED"):
            result = self.probe_diagnostics(boundary + "\n" + detail)
            self.assertEqual(result["categories"], ["test_failure"])
            self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
        name = REVIEWED["groups"]["container_activation_pg"][0]
        for boundary in (f"thread '{name}' panicked at infra/tests/container_activation.rs:111:5:",
                         "thread 'foreign' panicked at infra/tests/container_activation.rs:111:5:\nprivate"):
            result = self.probe_diagnostics(boundary + "\n" +
                f"thread '{name}' panicked at infra/tests/container_activation.rs:112:5:\n" + detail)
            self.assertIn(next(iter(gate.TEST_AUTHORIZE_HINTS)), result["categories"])
        result = gate.safe_test_diagnostics(io.BytesIO(detail.encode()), set(), REVIEWED["rust_source_sha256"], Path("/qa/backend"))
        self.assertEqual(result["categories"], ["unknown"])

    def test_authorize_details_keep_byte_line_count_and_schema_bounds(self):
        hint, detail = next(iter(gate.TEST_AUTHORIZE_HINTS.items()))
        for limit, amount in (("DIAGNOSTIC_INPUT_LIMIT", 8), ("DIAGNOSTIC_LINE_LIMIT", 64), ("DIAGNOSTIC_LIMIT", 0)):
            with mock.patch.object(gate, limit, amount):
                result = self.probe_diagnostics(detail)
            self.assertTrue(result["truncated"])
            self.assertNotIn(hint, result["categories"])
        value = dict(self.failure_test_value(), **self.probe_diagnostics(detail), stage="container_activation_pg",
                     gate_failed_stage="container_activation_pg")
        for changes in (dict(categories=[hint]), dict(categories=[hint, hint, "test_failure"]),
                        dict(categories=[hint + "_unknown", "test_failure"]), dict(diagnostics=[]),
                        dict(diagnostics=self.failure_test_value()["diagnostics"]), dict(backend_quality_gate=True),
                        dict(kind="safe_compiler_failure"), dict(message="PRIVATE_SENTINEL")):
            with self.assertRaises(ValueError):
                self.validate_failure(dict(value, **changes))

    def test_authorize_hints_authenticated_readback_remains_failure_only(self):
        for hint, detail in gate.TEST_AUTHORIZE_HINTS.items():
            value = dict(self.failure_test_value(), **self.probe_diagnostics(detail), stage="container_activation_pg",
                         gate_failed_stage="container_activation_pg")
            run, artifact, payload, args = self.failure_artifact(value=value)
            files = gate.validate_failure_readback(run, artifact, payload, **args)
            retained = json.loads(files[gate.FAILURE_FILE])
            self.assertEqual(retained["categories"], [hint, "test_failure"])
            self.assertEqual(set(files), {gate.FAILURE_FILE})
            for flag in ("backend_quality_gate", "all_quality_gate", "sdlc_acceptance"):
                self.assertIs(retained[flag], False)
            with self.assertRaises(ValueError):
                gate.validate_failure_readback(run, artifact, payload, **dict(args, artifact_digest="sha256:" + "0" * 64))


if __name__ == "__main__":
    unittest.main(verbosity=2)
