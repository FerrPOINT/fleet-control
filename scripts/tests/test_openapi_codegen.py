"""Pure static and in-memory checks; no compiler, Docker, exports or API calls."""
import ast
import copy
import io
import json
from pathlib import Path
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
        schema = codegen.canonical({"openapi": "3.1.0", "paths": {"/fixture": {}}})
        provenance = dict(version=1, repository=codegen.REPOSITORY, branch=codegen.BRANCH,
                          source_sha=codegen.SOURCE_SHA, base_sha=codegen.BASE_SHA, workflow_sha=workflow_sha,
                          workflow_path=codegen.WORKFLOW, run_id=123, run_attempt=1, rust="1.88.0",
                          swagger_sha256=codegen.SWAGGER_SHA, schema_generator_success=True,
                          all_quality_gate=False, sdlc_acceptance=False, schema_sha256=codegen.digest(schema),
                          command=["cargo", "run", "--locked", "-p", "api", "--bin", "gen-openapi"],
                          helper_sha256=codegen.digest((ROOT / "scripts/openapi_codegen.py").read_bytes()),
                          workflow_sha256=codegen.digest((ROOT / codegen.WORKFLOW).read_bytes()))
        provenance.update(provenance_changes or {})
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
                        name="fleet-openapi-fc6ef12-123-1", digest="sha256:" + codegen.digest(payload))
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
        for changes in ({"source_sha": "b" * 40}, {"base_sha": "b" * 40}, {"all_quality_gate": True},
                        {"sdlc_acceptance": True}, {"schema_generator_success": 1}, {"helper_sha256": "b" * 64}):
            run, artifact, payload, args = self.fixture(provenance_changes=changes)
            with self.subTest(changes=changes), self.assertRaises(ValueError):
                codegen.validate_readback(run, artifact, payload, **args)
        run, artifact, payload, args = self.fixture(extra={"../private": b"unsafe"})
        with self.assertRaises(ValueError):
            codegen.validate_readback(run, artifact, payload, **args)


if __name__ == "__main__":
    unittest.main()
