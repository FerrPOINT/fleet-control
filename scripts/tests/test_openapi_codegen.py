"""Pure static and in-memory checks; no compiler, Docker, exports or API calls."""
import ast
import copy
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

import yaml

from scripts import openapi_codegen as codegen

ROOT = Path(__file__).resolve().parents[2]


class CodegenTests(unittest.TestCase):
    def workflow(self):
        return yaml.load((ROOT / codegen.WORKFLOW).read_text(), Loader=yaml.BaseLoader)

    def test_exact_push_filter_and_read_only_permissions(self):
        workflow = self.workflow()
        self.assertEqual(workflow["on"], {"push": {"branches": [codegen.BRANCH]}})
        self.assertEqual(workflow["permissions"], {"contents": "read"})
        self.assertEqual(set(workflow["jobs"]), {"codegen"})
        job = workflow["jobs"]["codegen"]
        for expected in ("FerrPOINT/fleet-control", "github.event_name == 'push'", codegen.BRANCH, "github.event.deleted == false"):
            self.assertIn(expected, job["if"])
        self.assertEqual(job["runs-on"], "ubuntu-24.04")
        self.assertEqual(job["timeout-minutes"], "45")
        self.assertEqual(job["defaults"], {"run": {"shell": "bash"}})
        self.assertNotIn("services", job)
        self.assertNotIn("container", job)
        self.assertNotIn("environment", job)
        self.assertEqual(job["env"]["CARGO_BUILD_JOBS"], "1")
        self.assertEqual(job["env"]["RUSTUP_TOOLCHAIN"], "1.88.0")

    def test_action_pins_source_pins_and_secret_scope(self):
        steps = self.workflow()["jobs"]["codegen"]["steps"]
        actions = [step for step in steps if "uses" in step]
        self.assertEqual(len(actions), 4)
        for step in actions:
            self.assertRegex(step["uses"], r"^actions/(checkout|upload-artifact)@[0-9a-f]{40}$")
        checkouts = [step for step in actions if step["uses"].startswith("actions/checkout@")]
        self.assertEqual({step["with"]["path"] for step in checkouts}, {"controls", "fleet-control", "services-base"})
        by_path = {step["with"]["path"]: step["with"] for step in checkouts}
        self.assertEqual(by_path["controls"]["ref"], "${{ github.sha }}")
        self.assertEqual(by_path["controls"]["fetch-depth"], "0")
        self.assertEqual(by_path["fleet-control"]["ref"], codegen.SOURCE_SHA)
        self.assertEqual(by_path["fleet-control"]["fetch-depth"], "2")
        self.assertEqual(by_path["services-base"]["ref"], codegen.BASE_SHA)
        self.assertEqual(by_path["services-base"]["token"], "${{ secrets.SERVICES_BASE_TOKEN }}")
        for item in by_path.values():
            self.assertEqual(item["persist-credentials"], "false")
        text = (ROOT / codegen.WORKFLOW).read_text()
        self.assertEqual(text.count("secrets."), 1)
        self.assertLess(text.index("openapi_codegen.py preflight"), text.index("secrets.SERVICES_BASE_TOKEN"))
        self.assertNotIn("continue-on-error", text)

    def test_artifact_allowlist_retention_and_no_failure_upload(self):
        step = next(step for step in self.workflow()["jobs"]["codegen"]["steps"] if step.get("id") == "artifact")
        self.assertNotIn("if", step)
        self.assertEqual(step["with"]["retention-days"], "14")
        self.assertEqual(step["with"]["archive"], "true")
        self.assertEqual(step["with"]["overwrite"], "false")
        self.assertEqual(step["with"]["include-hidden-files"], "false")
        self.assertEqual(step["with"]["if-no-files-found"], "error")
        self.assertEqual({Path(line).name for line in step["with"]["path"].splitlines()}, codegen.ARTIFACT_FILES)
        self.assertIn("${{ github.run_id }}-${{ github.run_attempt }}", step["with"]["name"])

    def test_branch_delta_cannot_carry_backend_ci_schema_or_migrations(self):
        good = "".join("A\t" + name + "\n" for name in sorted(codegen.WRITE_SET))
        codegen.validate_delta(good)
        for bad in (good + "M\t.github/workflows/ci.yml\n", good + "M\topenapi/openapi.json\n",
                    good + "A\tbackend/migration/src/m000013.rs\n", good.replace("A\t", "M\t", 1), ""):
            with self.subTest(bad=bad), self.assertRaises(ValueError):
                codegen.validate_delta(bad)

    def test_no_local_generation_before_hosted_identity(self):
        with patch.dict(codegen.os.environ, {}, clear=True), patch.object(codegen, "git") as git, \
             patch.object(codegen, "export") as export:
            with self.assertRaises(ValueError):
                codegen.generate()
            git.assert_not_called()
            export.assert_not_called()

    def test_archive_safety(self):
        for path in ("/backend/x", "backend/../x", "backend\\x", "backend/.local/x", "backend/target/x",
                     "backend/node_modules/x", "backend/.venv/x", "backend/.env", "frontend/src/api/types.ts", "backend/a\nb"):
            with self.subTest(path=path), self.assertRaises(ValueError):
                codegen.safe_member(path, ("backend", ".base-revision"))
        self.assertEqual(str(codegen.safe_member("backend/Cargo.lock", ("backend",))), "backend/Cargo.lock")

    def test_locked_generator_and_finally_cleanup_are_static(self):
        text = (ROOT / "scripts/openapi_codegen.py").read_text()
        tree = ast.parse(text)
        calls = [node for node in ast.walk(tree) if isinstance(node, ast.Call)]
        cargo = [node for node in calls if node.args and isinstance(node.args[0], ast.List)
                 and node.args[0].elts and isinstance(node.args[0].elts[0], ast.Constant)
                 and node.args[0].elts[0].value == "cargo"]
        self.assertEqual(len(cargo), 1)
        self.assertEqual([item.value for item in cargo[0].args[0].elts],
                         ["cargo", "run", "--locked", "-p", "api", "--bin", "gen-openapi"])
        self.assertIn("finally:", text)
        self.assertIn('scratch.resolve() == temporary / "fleet-openapi-codegen"', text)
        self.assertIn("shutil.rmtree(scratch)", text)
        self.assertIn('source, SOURCE_SHA, scratch / "src/fleet-control", ("backend", ".base-revision")', text)
        self.assertIn('base, BASE_SHA, scratch / "src/services-base", ("crates", "Cargo.toml", "Cargo.lock", "LICENSE")', text)
        self.assertIn("5 * 1024 ** 3", text)
        self.assertTrue(any(item.arg == "stderr" and isinstance(item.value, ast.Name) and item.value.id == "diagnostics"
                            for item in cargo[0].keywords))

    def fixture(self, *, provenance_changes=None, extra=None):
        workflow_sha = "a" * 40
        schema = codegen.canonical(self.schema_fixture())
        provenance = dict(version=1, repository=codegen.REPOSITORY, branch=codegen.BRANCH,
                          source_sha=codegen.SOURCE_SHA, base_sha=codegen.BASE_SHA, workflow_sha=workflow_sha,
                          source_parents=codegen.SOURCE_PARENTS, source_tree=codegen.SOURCE_TREE,
                          qualified_source_blobs=codegen.SOURCE_BLOBS,
                          workflow_path=codegen.WORKFLOW, run_id=123, run_attempt=1, rust="1.88.0",
                          swagger_sha256=codegen.SWAGGER_SHA, schema_generator_success=True,
                          all_quality_gate=False, sdlc_acceptance=False, schema_sha256=codegen.digest(schema),
                          command=["cargo", "run", "--locked", "-p", "api", "--bin", "gen-openapi"],
                          helper_sha256=codegen.digest((ROOT / "scripts/openapi_codegen.py").read_text(encoding="utf-8").encode()),
                          workflow_sha256=codegen.digest((ROOT / codegen.WORKFLOW).read_text(encoding="utf-8").encode()))
        provenance.update(provenance_changes or {})
        for key, value in codegen.QUALIFIED_EXPORT.items():
            provenance.setdefault(key, value)
        files = {"openapi.json": schema, "provenance.json": codegen.canonical(provenance)}
        files["SHA256SUMS"] = "".join(codegen.digest(data) + "  " + name + "\n" for name, data in files.items()).encode()
        files.update(extra or {})
        output = io.BytesIO()
        with zipfile.ZipFile(output, "w") as archive:
            for name, data in files.items():
                archive.writestr(name, data)
        payload = output.getvalue()
        run = dict(id=123, run_attempt=1, status="completed", conclusion="success", event="push", head_sha=workflow_sha,
                   head_branch=codegen.BRANCH, path=codegen.WORKFLOW, repository={"full_name": codegen.REPOSITORY})
        artifact = dict(expired=False, workflow_run={"id": 123, "head_sha": workflow_sha},
                        name="fleet-openapi-pm-union-123-1", digest="sha256:" + codegen.digest(payload))
        args = dict(run_id=123, attempt=1, workflow_sha=workflow_sha, artifact_digest=codegen.digest(payload))
        return run, artifact, payload, args

    def test_verified_readback_is_bound_to_run_attempt_sha_and_zip_digest(self):
        run, artifact, payload, args = self.fixture()
        self.assertEqual(set(codegen.validate_readback(run, artifact, payload, **args)), codegen.ARTIFACT_FILES)
        for key, value in (("conclusion", "failure"), ("head_sha", "b" * 40), ("run_attempt", 2),
                           ("head_branch", "main"), ("event", "pull_request")):
            bad = copy.deepcopy(run)
            bad[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                codegen.validate_readback(bad, artifact, payload, **args)
        with self.assertRaises(ValueError):
            codegen.validate_readback(run, dict(artifact, expired=True), payload, **args)
        with self.assertRaises(ValueError):
            codegen.validate_readback(run, artifact, payload + b"tampered", **args)

    def test_readback_rejects_forged_source_acceptance_and_unsafe_zip(self):
        for changes in ({"source_sha": "b" * 40}, {"source_sha": "32b9f063f9b5099ff61bca24ecdfeb9952889034"},
                        {"source_sha": "212d07391b83b8a5081c946b87b4215053c4a153"},
                        {"source_sha": "83091f055e3b34fcfe6a6d59b1703c117261c027"},
                        {"source_sha": "31ab4e90b77f389b7bc5f6cfaf5f4b3d38f2d75e"},
                        {"source_sha": "f7d586be10a958f4f454c357831018250c779256"},
                        {"source_inventory_sha256": "48077b8901381fa9609fc88e0e864dc04a19817f3ae83f1e63f90459c72253ff"},
                        {"source_inventory_sha256": "2f9240f7c8c5ca1da152aed05a50d6748122490463332e7b41fb1e17da4bf842"},
                        {"source_inventory_sha256": "6105d3c5d660201536c9f06e91c7622d8ebabd54b83f5440f5cddd204bcdfceb"},
                        {"source_inventory_sha256": "c94d72c164e26e7cab2e9810f99cdcf8a960e9716c27cc2f63632df83337dec6"},
                        {"source_inventory_sha256": "388ec5ba3512d0f94614874e0c9f394b7fd6bda50f2288c25d36e62f7b079918"},
                        {"source_file_count": 297},
                        {"source_file_count": 292},
                        {"source_file_count": 294},
                        {"source_file_count": 283}, {"base_sha": "b" * 40}, {"all_quality_gate": True},
                        {"sdlc_acceptance": True}, {"schema_generator_success": 1}, {"helper_sha256": "b" * 64},
                        {"source_parents": "b" * 40}, {"source_parents": []},
                        {"source_parents": ["5cad61a38b29058581a9ce6a8a05426303d32e53", "3694cbd399113ee25bfaa5bae7ae009df96f1e57"]},
                        {"source_parents": ["b" * 40]},
                        {"source_parents": ["dd5744e34cfd6d69dc7cffad882bdd635a887253"]},
                        {"source_tree": "b" * 40}, {"qualified_source_blobs": {}},
                        {"source_inventory_sha256": "b" * 64}, {"base_tree": "b" * 40},
                        {"source_file_count": 0}, {"fleet_lock_sha256": "b" * 64}, {"base_lock_sha256": "b" * 64}):
            run, artifact, payload, args = self.fixture(provenance_changes=changes)
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                codegen.validate_readback(run, artifact, payload, **args)
        run, artifact, payload, args = self.fixture(extra={"../private": b"unsafe"})
        with self.assertRaises(ValueError):
            codegen.validate_readback(run, artifact, payload, **args)


    def schema_fixture(self):
        # Synthetic validation fixture only; never written to product or uploaded.
        value = {"openapi": "3.1.0", "paths": {path: {method: {}} for path, method in
                (codegen.JOURNAL_OPERATIONS | codegen.CONFIG_OPERATIONS).items()},
                "components": {"schemas": {name: {} for name in codegen.REQUIRED_SCHEMAS}}}
        value["components"]["schemas"]["AgentSession"] = {
            "properties": {"pending_delivery": {"type": ["boolean", "null"]}}}
        return value

    def test_exact_journal_source_parent_tree_and_blobs(self):
        self.assertEqual(codegen.SOURCE_SHA, "4449a3b1cdd915e265543a24054506f15385393d")
        self.assertEqual(codegen.BASE_SHA, "19a7a381ae6dbea61a643bb96189e483fa64df5c")
        codegen.qualify_source(ROOT)
        results = [" ".join([codegen.SOURCE_SHA, *codegen.SOURCE_PARENTS]).encode(), codegen.SOURCE_TREE.encode()]
        results += [blob.encode() for blob in codegen.SOURCE_BLOBS.values()]
        with patch.object(codegen, "git", side_effect=results) as git:
            codegen.qualify_source(ROOT)
        self.assertEqual(git.call_count, 2 + len(codegen.SOURCE_BLOBS))
        for index in range(len(results)):
            changed = list(results)
            changed[index] = b"0" * 40
            with self.subTest(index=index), patch.object(codegen, "git", side_effect=changed), self.assertRaises(ValueError):
                codegen.qualify_source(ROOT)

        for parents in ([], ["b" * 40],
                        ["dd5744e34cfd6d69dc7cffad882bdd635a887253"],
                        ["5cad61a38b29058581a9ce6a8a05426303d32e53", "3694cbd399113ee25bfaa5bae7ae009df96f1e57"],
                        [*codegen.SOURCE_PARENTS, "b" * 40]):
            changed = [" ".join([codegen.SOURCE_SHA, *parents]).encode(), *results[1:]]
            with self.subTest(parents=parents), patch.object(codegen, "git", side_effect=changed), self.assertRaises(ValueError):
                codegen.qualify_source(ROOT)

    def test_workflow_summary_and_artifact_match_new_union(self):
        workflow = self.workflow()
        steps = workflow["jobs"]["codegen"]["steps"]
        summary = steps[-1]["run"]
        self.assertIn("printf 'Source: `%s`\\n\\n' " + codegen.SOURCE_SHA, summary)
        artifact = next(step for step in steps if step.get("id") == "artifact")
        self.assertEqual(artifact["with"]["name"],
                         "fleet-openapi-pm-union-${{ github.run_id }}-${{ github.run_attempt }}")
        self.assertNotIn("8c93f43f", (ROOT / codegen.WORKFLOW).read_text())

    def test_pending_delivery_is_optional_nullable_boolean_not_stale(self):
        for field in (None, {}, {"type": "boolean"}, {"type": ["string", "null"]}):
            value = self.schema_fixture()
            properties = value["components"]["schemas"]["AgentSession"]["properties"]
            if field is None:
                del properties["pending_delivery"]
            else:
                properties["pending_delivery"] = field
            with self.subTest(field=field), self.assertRaises(ValueError):
                codegen.schema_valid(codegen.canonical(value))
        value = self.schema_fixture()
        value["components"]["schemas"]["AgentSession"]["required"] = ["pending_delivery"]
        with self.assertRaises(ValueError):
            codegen.schema_valid(codegen.canonical(value))

    def test_readback_rejects_previous_artifact_namespace(self):
        run, artifact, payload, args = self.fixture()
        for name in ("fleet-openapi-config8c93-123-1", "fleet-openapi-authority-union-123-1"):
            artifact["name"] = name
            with self.subTest(name=name), self.assertRaises(ValueError):
                codegen.validate_readback(run, artifact, payload, **args)

    def test_journal_operations_and_dtos_required_not_stale_schema(self):
        codegen.schema_valid(codegen.canonical(self.schema_fixture()))
        for path, method in (codegen.JOURNAL_OPERATIONS | codegen.CONFIG_OPERATIONS).items():
            value = self.schema_fixture()
            del value["paths"][path][method]
            with self.subTest(path=path), self.assertRaises(ValueError):
                codegen.schema_valid(codegen.canonical(value))
        for name in self.schema_fixture()["components"]["schemas"]:
            value = self.schema_fixture()
            del value["components"]["schemas"][name]
            with self.subTest(dto=name), self.assertRaises(ValueError):
                codegen.schema_valid(codegen.canonical(value))
        with self.assertRaises(ValueError):
            codegen.schema_valid(codegen.canonical({"openapi": "3.1.0", "paths": {"/old": {}}}))

    def test_export_qualification_fails_before_compilation_on_drift(self):
        before = {"fleet-control/backend/Cargo.lock": "a", "services-base/Cargo.lock": "b"}
        expected = dict(source_inventory_sha256=codegen.digest(codegen.canonical(before)),
                        source_file_count=2, base_tree="c", fleet_lock_sha256="a", base_lock_sha256="b")
        with patch.object(codegen, "QUALIFIED_EXPORT", expected):
            codegen.qualify_export(before, "c")
            for changed, tree in ((dict(before, extra="d"), "c"), (before, "d"), ({}, "c")):
                with self.subTest(changed=changed, tree=tree), self.assertRaises(ValueError):
                    codegen.qualify_export(changed, tree)

    def test_archive_scaffold_only_for_directories_and_lf_config(self):
        roots = ("backend/api",)
        self.assertEqual(str(codegen.safe_member("backend", roots, directory=True)), "backend")
        with self.assertRaises(ValueError):
            codegen.safe_member("backend", roots)
        data = io.BytesIO()
        with tarfile.open(fileobj=data, mode="w") as archive:
            directory = tarfile.TarInfo("backend/")
            directory.type = tarfile.DIRTYPE
            archive.addfile(directory)
            member = tarfile.TarInfo("backend/api/fixture.rs")
            member.size = 2
            archive.addfile(member, io.BytesIO(b"ok"))
        with tempfile.TemporaryDirectory() as temporary, patch.object(codegen, "git", return_value=data.getvalue()) as git:
            destination = Path(temporary) / "export"
            codegen.export(ROOT, codegen.SOURCE_SHA, destination, roots)
            self.assertEqual((destination / "backend/api/fixture.rs").read_bytes(), b"ok")
            self.assertEqual(git.call_args.args[1:5], ("-c", "core.autocrlf=false", "-c", "core.eol=lf"))

    def test_no_full_gate_or_manual_schema_commands(self):
        text = (ROOT / "scripts/openapi_codegen.py").read_text()
        workflow = (ROOT / codegen.WORKFLOW).read_text()
        for forbidden in ("--all-targets", "clippy", "docker ", "pnpm", "cargo test", "workflow_dispatch"):
            self.assertNotIn(forbidden, text + workflow)
        self.assertNotIn("openapi/openapi.json", text + workflow)
        self.assertIn("timeout=2400", text)
        self.assertIn("schema_valid(schema)", text)
        self.assertIn("inventory(scratch / \"src\") == before", text)

    def test_existing_full_ci_stays_main_only_not_bootstrap(self):
        ci = yaml.load((ROOT / ".github/workflows/ci.yml").read_text(), Loader=yaml.BaseLoader)
        self.assertEqual(ci["on"]["push"]["branches"], ["main"])
        self.assertNotIn(codegen.BRANCH, (ROOT / ".github/workflows/ci.yml").read_text())


if __name__ == "__main__":
    unittest.main()
