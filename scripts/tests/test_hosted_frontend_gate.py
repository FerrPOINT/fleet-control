"""Pure/static regressions. Never start frontend, browsers, containers, or Cargo."""
import io
import json
import os
from pathlib import Path
import random
import re
import struct
import subprocess
import tempfile
import unittest
from unittest.mock import MagicMock, patch
import zipfile
import zlib

from scripts import hosted_frontend_gate as gate

ROOT = Path(__file__).resolve().parents[2]


def png(width=375, height=812):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    row = b"\0" + random.Random(23).randbytes(width * 3)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(row * height)) + chunk(b"IEND", b""))


def checksums(files):
    files["SHA256SUMS"] = "".join(gate.digest(files[name]) + "  " + name + "\n"
                                  for name in sorted(files) if name != "SHA256SUMS").encode()


def fixture_files():
    workflow_sha = "a" * 40
    provenance = dict(version=1, repository=gate.REPOSITORY, branch=gate.BRANCH,
        source_sha=gate.SOURCE_SHA, source_tree=gate.SOURCE_TREE, source_parents=gate.SOURCE_PARENTS,
        base_sha=gate.BASE_SHA, base_tree=gate.BASE_TREE, schema_sha256=gate.SCHEMA_SHA256,
        generated_client_sha256="b" * 64, node=gate.NODE, pnpm=gate.PNPM,
        workflow_sha=workflow_sha, workflow_path=gate.WORKFLOW, run_id=123, run_attempt=2,
        gates=list(gate.GATES), compat_main_sha="c" * 40, compat_schema_sha256="d" * 64,
        helper_sha256=gate.digest((ROOT / "scripts/hosted_frontend_gate.py").read_text(encoding="utf-8").encode()),
        workflow_sha256=gate.digest((ROOT / gate.WORKFLOW).read_text(encoding="utf-8").encode()),
        build_manifest_sha256="f" * 64, build_file_count=5, build_index_sha256="f" * 64,
        runner_image="ubuntu24", runner_image_version="test-only", **gate.QUALIFIED_INPUTS, **gate.SCOPE)
    pictures = {viewport: png(*map(int, viewport.split("x"))) for viewport in gate.VIEWPORTS}
    files = {"screens/" + path.removeprefix("docs/assets/screens/"): pictures[path.split("/")[3]]
             for path in gate.capture_paths(ROOT)}
    manifest = dict(kind="fresh_chromium_fixture_screens", count=len(files), files=sorted(files),
                    original_manifest_sha256="e" * 64)
    for browser in ("chromium", "firefox", "webkit"):
        files[f"fixtures/example-{browser}/fixture.png"] = pictures["375x812"]
    summary = dict(unit=gate.QUALIFIED_UNIT_COUNTS,
                   browsers={b: dict(passed=1, flaky=0, skipped=9) for b in ("chromium", "firefox", "webkit")},
                   **gate.SCOPE)
    files.update({"provenance.json": gate.canonical(provenance), "fixture-summary.json": gate.canonical(summary),
                  "screens/manifest.json": gate.canonical(manifest)})
    checksums(files)
    return files


def zipped(files, extra=None):
    output = io.BytesIO()
    with zipfile.ZipFile(output, "w", zipfile.ZIP_DEFLATED) as archive:
        for name, data in files.items():
            archive.writestr(name, data)
        if extra:
            archive.writestr(*extra)
    return output.getvalue()


def readback_metadata(payload):
    run = dict(id=123, run_attempt=2, status="completed", conclusion="success", event="push",
               head_sha="a" * 40, head_branch=gate.BRANCH, path=gate.WORKFLOW,
               repository=dict(full_name=gate.REPOSITORY))
    artifact = dict(id=456, expired=False, workflow_run=dict(id=123, head_sha="a" * 40, head_branch=gate.BRANCH),
                    name="fleet-frontend-b0ad56c-123-2", digest="sha256:" + gate.digest(payload))
    return run, artifact


def validate(payload, run=None, artifact=None):
    actual_run, actual_artifact = readback_metadata(payload)
    return gate.validate_readback(run or actual_run, artifact or actual_artifact, payload,
        run_id=123, attempt=2, workflow_sha="a" * 40, artifact_id=456, artifact_digest=gate.digest(payload))


class SourceContracts(unittest.TestCase):
    def test_exact_two_parent_tuple(self):
        gate.validate_source_tuple(gate.SOURCE_SHA, gate.SOURCE_TREE, gate.SOURCE_PARENTS)
        for parents in (gate.SOURCE_PARENTS[:1], list(reversed(gate.SOURCE_PARENTS)),
                        gate.SOURCE_PARENTS + ["f" * 40], ["f" * 40, gate.SOURCE_PARENTS[1]]):
            with self.subTest(parents=parents), self.assertRaises(ValueError):
                gate.validate_source_tuple(gate.SOURCE_SHA, gate.SOURCE_TREE, parents)

    def test_wrong_commit_or_tree(self):
        for sha, tree in (("f" * 40, gate.SOURCE_TREE), (gate.SOURCE_SHA, "f" * 40)):
            with self.assertRaises(ValueError):
                gate.validate_source_tuple(sha, tree, gate.SOURCE_PARENTS)

    def test_delta_requires_exactly_three_additions(self):
        good = "\n".join("A\t" + name for name in sorted(gate.WRITE_SET))
        gate.validate_delta(good)
        for value in (good + "\nM\tfrontend/package.json", good.replace("A\t", "M\t", 1),
                      good + "\n" + good.splitlines()[0], "\n".join(good.splitlines()[:2]),
                      good.replace("scripts/hosted_frontend_gate.py", "scripts/openapi_codegen.py")):
            with self.subTest(delta=value), self.assertRaises(ValueError):
                gate.validate_delta(value)

    def test_public_repo_is_explicit(self):
        value = dict(full_name=gate.REPOSITORY, private=False, visibility="public", default_branch="main")
        gate.public_repository(value)
        for key, bad in (("private", True), ("private", 0), ("visibility", "private"),
                         ("full_name", "other/fleet-control"), ("default_branch", "other")):
            with self.subTest(key=key), self.assertRaises(ValueError):
                gate.public_repository(dict(value, **{key: bad}))

    def test_schema_pin_is_current(self):
        self.assertEqual(gate.digest((ROOT / "openapi/openapi.json").read_bytes()), gate.SCHEMA_SHA256)
        self.assertEqual(gate.SCHEMA_SHA256, "1167220ea9f3d65ddca4cce1112a26d53c77f8c1684ef958859f737f20210953")

    def test_no_local_or_external_runner(self):
        with patch.dict(os.environ, {}, clear=True), self.assertRaises(ValueError):
            gate.hosted_identity()

    def test_no_live_environment_or_pool_overrides(self):
        for key in ("PLAYWRIGHT_BASE_URL", "SDLC_LIVE_QA", "SDLC_QA_SESSION_FILE", "VITEST_POOL", "NODE_OPTIONS"):
            with patch.dict(os.environ, {key: "override"}, clear=True), self.assertRaises(ValueError):
                gate.require_fixture_environment()

    def test_client_is_ignored_by_existing_convention(self):
        package = json.loads((ROOT / "frontend/package.json").read_bytes())
        self.assertEqual(package["scripts"]["postinstall"], "openapi-typescript ../openapi/openapi.json -o src/api/generated.ts")
        self.assertIn(gate.GENERATED, (ROOT / ".gitignore").read_text().splitlines())
        self.assertEqual(package["dependencies"]["@sdlc/ui"], "file:../../services-base/frontend")

    def test_qualified_inventory_cannot_drift(self):
        state = dict(source={"frontend/pnpm-lock.yaml": "a" * 64}, base={"frontend/pnpm-lock.yaml": "b" * 64})
        self.assertNotEqual(gate.qualified_inputs(state), gate.QUALIFIED_INPUTS)
        original = gate.qualified_inputs(state)
        state["base"]["frontend/src/ui/button.tsx"] = "c" * 64
        self.assertNotEqual(gate.qualified_inputs(state), original)


class CompletionContracts(unittest.TestCase):
    def test_unit_summary_needs_completed_tests(self):
        for output in (b"RUN v4\nWaiting for worker...", b"Test Files 0 passed\nTests 0 passed",
                       b"Test Files 2 passed | 1 failed\nTests 7 passed | 1 failed", b"Aborted"):
            with self.subTest(output=output), self.assertRaises(ValueError):
                gate.unit_counts(output)

    def test_unit_counts_record_preserved_skips(self):
        self.assertEqual(gate.unit_counts(b"\x1b[32mTest Files 2 passed (2)\nTests 8 passed | 1 skipped (9)\x1b[0m"),
                         dict(files_passed=2, tests_passed=8, files_skipped=0, tests_skipped=1))

    def test_exact_frozen_baseline_counts(self):
        self.assertEqual(gate.unit_counts(b"Test Files 36 passed (36)\nTests 337 passed (337)"), gate.QUALIFIED_UNIT_COUNTS)
        self.assertNotEqual(gate.unit_counts(b"Test Files 35 passed (35)\nTests 336 passed (336)"), gate.QUALIFIED_UNIT_COUNTS)

    @staticmethod
    def browser_report():
        return dict(errors=[], suites=[dict(specs=[dict(tests=[
            dict(projectName=name, status="expected", expectedStatus="passed", results=[dict(status="passed")])
            for name in ("chromium", "firefox", "webkit")])])])

    def test_actual_three_browser_completion(self):
        counts = gate.browser_counts(self.browser_report())
        self.assertEqual(set(counts), {"chromium", "firefox", "webkit"})
        self.assertTrue(all(p["passed"] == 1 for p in counts.values()))

    def test_no_empty_missing_failed_or_expected_failure_browser(self):
        for kind in ("empty", "missing", "failed", "expected-failure", "global-error"):
            report = self.browser_report()
            test = report["suites"][0]["specs"][0]["tests"][0]
            if kind == "empty":
                report["suites"] = []
            elif kind == "missing":
                report["suites"][0]["specs"][0]["tests"].pop()
            elif kind == "failed":
                test["status"] = "unexpected"
            elif kind == "expected-failure":
                test.update(expectedStatus="failed", results=[dict(status="failed")])
            else:
                report["errors"] = [dict(message="private diagnostics")]
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                gate.browser_counts(report)

    def test_existing_retries_and_live_skips_are_reported(self):
        report = self.browser_report()
        tests = report["suites"][0]["specs"][0]["tests"]
        tests[0].update(status="flaky", results=[dict(status="failed"), dict(status="passed")])
        tests.append(dict(projectName="webkit", status="skipped", expectedStatus="skipped", results=[dict(status="skipped")]))
        counts = gate.browser_counts(report)
        self.assertEqual(counts["chromium"]["flaky"], 1)
        self.assertEqual(counts["webkit"]["skipped"], 1)

    def test_reordered_gate_never_executes(self):
        with patch.object(gate, "load_state", return_value=(ROOT, ROOT, dict(gates=[]))), \
                patch.object(gate.subprocess, "run") as run, self.assertRaises(ValueError):
            gate.gate("unit")
        run.assert_not_called()

    def test_incomplete_pipeline_cannot_publish(self):
        with patch.object(gate, "load_state", return_value=(ROOT, ROOT, dict(gates=["base"]))), \
                self.assertRaises(ValueError):
            gate.finish()

    def test_schema_and_client_parity_after_tests(self):
        state = dict(source={}, base={}, generated_sha256=gate.digest(b"generated"))
        with patch.object(gate, "checkout"), patch.object(gate, "git", return_value=b""), \
                patch.object(gate, "bounded_file", side_effect=[b"schema", b"modified"]), \
                patch.object(gate, "SCHEMA_SHA256", gate.digest(b"schema")), self.assertRaises(ValueError):
            gate.verify_parity(ROOT, state)

    def test_tracked_base_drift_fails(self):
        state = dict(source={}, base={"frontend/package.json": gate.digest(b"original")})
        with patch.object(gate, "checkout"), patch.object(gate, "git", return_value=b""), \
                patch.object(gate, "bounded_file", return_value=b"modified"), self.assertRaises(ValueError):
            gate.verify_parity(ROOT, state)

    def test_build_requires_index_js_css_and_records_content(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            dist = root / "frontend/dist"
            dist.mkdir(parents=True)
            with self.assertRaises(ValueError):
                gate.build_inventory(root)
            for name in ("index.html", "assets/main.js", "assets/main.css"):
                path = dist / name
                path.parent.mkdir(exist_ok=True)
                path.write_bytes(b"fixture-only")
            before = gate.build_inventory(root)
            self.assertEqual(before["build_file_count"], 3)
            (dist / "assets/main.js").write_bytes(b"changed")
            self.assertNotEqual(gate.build_inventory(root), before)


class WorkflowContracts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = (ROOT / gate.WORKFLOW).read_text()
        cls.original = (ROOT / ".github/workflows/ci.yml").read_text().split("\n  frontend:\n", 1)[1].split("\n  minimum-rust:", 1)[0]

    def test_all_original_frontend_commands_preserved(self):
        exceptions = {"base", "compat", "generated", "theme", "fixtures"}
        for name, commands in gate.GATES.items():
            if name not in exceptions:
                for command in commands:
                    with self.subTest(gate=name):
                        self.assertIn(" ".join(command), self.original)
        self.assertEqual(gate.GATES["unit"], [["pnpm", "test", "--", "--run"]])
        self.assertEqual(gate.GATES["fixtures"], [["pnpm", "exec", "playwright", "test", "--reporter=list,json"]])
        self.assertIn("pnpm exec playwright test", self.original)
        self.assertIn("pnpm openapi:compat", self.original)
        self.assertEqual(gate.GATES["compat"][-1], ["pnpm", "openapi:compat"])
        self.assertIn("+refs/heads/main:refs/remotes/origin/main", gate.GATES["compat"][0])
        self.assertIn("--depth=1", gate.GATES["compat"][0])
        self.assertIn("pnpm theme:check http://127.0.0.1:4173", gate.THEME)
        self.assertIn("trap '", gate.THEME)
        self.assertIn("--strictPort", gate.THEME)

    def test_workflow_gate_order_and_scope(self):
        names = re.findall(r"run: python3 -B controls/scripts/hosted_frontend_gate.py gate ([a-z-]+)", self.workflow)
        self.assertEqual(names, list(gate.GATES))
        self.assertIn("branches: [" + gate.BRANCH + "]", self.workflow)
        for forbidden in ("workflow_dispatch:", "pull_request:", "self-hosted", "continue-on-error", "contents: write",
                          "cargo ", "docker ", "--passWithNoTests", "--pool", "--grep", "--project="):
            self.assertNotIn(forbidden, self.workflow)

    def test_secret_is_after_preflight_and_read_only(self):
        self.assertLess(self.workflow.index("hosted_frontend_gate.py preflight"), self.workflow.index("secrets.SERVICES_BASE_TOKEN"))
        self.assertEqual(self.workflow.count("secrets.SERVICES_BASE_TOKEN"), 1)
        self.assertIn("permissions:\n  contents: read", self.workflow)
        self.assertEqual(self.workflow.count("persist-credentials: false"), 3)
        self.assertIn("ref: " + gate.SOURCE_SHA, self.workflow)
        self.assertIn("ref: " + gate.BASE_SHA, self.workflow)
        self.assertIn("node-version: \"" + gate.NODE + "\"", self.workflow)
        self.assertIn("version: " + gate.PNPM, self.workflow)

    def test_only_verified_official_action_shas(self):
        expected = {"actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1",
                    "actions/setup-node@949feb2413d6458794dcd2491c4babbbce0c15c1",
                    "pnpm/action-setup@f520eceda224fe1a4aed5a2a27a194379a409996",
                    "actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9"}
        self.assertEqual(set(re.findall(r"uses: (\S+)", self.workflow)), expected)

    def test_artifact_is_success_only_and_public_allowlist(self):
        block = self.workflow.split("- name: Upload qualified", 1)[1].split("- name: Record artifact", 1)[0]
        self.assertNotIn("always()", block)
        self.assertIn("path: ${{ runner.temp }}/fleet-frontend-public", block)
        self.assertIn("include-hidden-files: false", block)
        self.assertIn("if-no-files-found: error", block)
        self.assertIn("archive: true", block)
        self.assertIn("if: always()", self.workflow.split("- name: Remove only", 1)[1])


class EvidenceContracts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files = fixture_files()
        cls.payload = zipped(cls.files)

    def test_authenticated_exact_run_readback(self):
        self.assertEqual(validate(self.payload), self.files)

    def test_failed_wrong_attempt_branch_workflow_repo_or_commit_rejected(self):
        for key, value in (("conclusion", "failure"), ("status", "in_progress"), ("event", "workflow_dispatch"),
                           ("run_attempt", 1), ("head_branch", "main"), ("path", ".github/workflows/ci.yml"),
                           ("head_sha", "f" * 40), ("repository", dict(full_name="other/repo"))):
            run, _ = readback_metadata(self.payload)
            run[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate(self.payload, run=run)

    def test_wrong_artifact_identity_digest_or_expiry_rejected(self):
        for key, value in (("expired", True), ("id", 1), ("name", "other"), ("digest", "sha256:" + "0" * 64),
                           ("workflow_run", dict(id=123, head_sha="f" * 40, head_branch=gate.BRANCH))):
            _, artifact = readback_metadata(self.payload)
            artifact[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate(self.payload, artifact=artifact)

    def test_zip_member_allowlist_blocks_source_logs_and_traversal(self):
        for name in ("../source.png", "/tmp/source.png", "fixtures\\secret.png", "source.rs", ".git/config",
                     "services-base/frontend/src/ui/button.tsx", "raw.log", "test-results/trace.zip",
                     "fixtures/a/secret.txt", "fixtures/a/nested/image.png"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                validate(zipped(self.files, (name, b"private")))

    def test_duplicate_members_rejected(self):
        with self.assertWarns(UserWarning):
            payload = zipped(self.files, ("provenance.json", self.files["provenance.json"]))
        with self.assertRaises(ValueError):
            validate(payload)

    def test_zip_symlink_rejected_before_read(self):
        output = io.BytesIO()
        with zipfile.ZipFile(output, "w") as archive:
            info = zipfile.ZipInfo("fixtures/example-chromium/link.png")
            info.create_system = 3
            info.external_attr = 0o120777 << 16
            archive.writestr(info, b"../../private")
        with self.assertRaises(ValueError):
            validate(output.getvalue())

    def test_compressed_and_uncompressed_bounds(self):
        with patch.object(gate, "MAX_ARTIFACT", len(self.payload) - 1), self.assertRaises(ValueError):
            validate(self.payload)
        with patch.object(gate, "MAX_FILE", 100), self.assertRaises(ValueError):
            validate(self.payload)
        with patch.object(gate, "MAX_MEMBERS", 2), self.assertRaises(ValueError):
            validate(self.payload)

    def test_file_checksum_tampering_rejected(self):
        files = dict(self.files, **{"fixture-summary.json": b"{}"})
        with self.assertRaises(ValueError):
            validate(zipped(files))

    def test_provenance_cannot_claim_live_acceptance_or_other_inputs(self):
        for key, value in (("all_sdlc_acceptance", True), ("live_pm_acceptance", True),
                           ("live_runtime_acceptance", True), ("source_parents", gate.SOURCE_PARENTS[:1]),
                           ("schema_sha256", "874" + "0" * 61), ("base_sha", "f" * 40),
                           ("base_inventory_sha256", "0" * 64), ("gates", ["build"]),
                           ("build_file_count", 0), ("build_manifest_sha256", ""),
                           ("raw_private_source", "forbidden")):
            files = dict(self.files)
            provenance = json.loads(files["provenance.json"])
            provenance[key] = value
            files["provenance.json"] = gate.canonical(provenance)
            checksums(files)
            with self.subTest(key=key), self.assertRaises(ValueError):
                validate(zipped(files))

    def test_no_evidence_for_zero_unit_or_missing_browser(self):
        for kind in ("unit", "browser", "live"):
            files = dict(self.files)
            summary = json.loads(files["fixture-summary.json"])
            if kind == "unit":
                summary["unit"]["tests_passed"] = 0
            elif kind == "browser":
                summary["browsers"]["webkit"]["passed"] = 0
            else:
                summary["live_pm_acceptance"] = True
            files["fixture-summary.json"] = gate.canonical(summary)
            checksums(files)
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                validate(zipped(files))

    def test_missing_browser_screenshot_or_manifest_entry(self):
        for name in ("fixtures/example-webkit/fixture.png", next(n for n in self.files if n.startswith("screens/") and n.endswith(".png"))):
            files = dict(self.files)
            del files[name]
            checksums(files)
            with self.assertRaises(ValueError):
                validate(zipped(files))

    def test_png_corruption_or_metadata_is_rejected(self):
        data = png()
        for bad in (b"not a PNG" * 300, data[:-1] + b"x", data + b"private source", data[:20]):
            with self.assertRaises(ValueError):
                gate.png_valid(bad)

    def test_source_file_symlink_and_path_escape_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "real.txt").write_bytes(b"okay")
            self.assertEqual(gate.bounded_file(root, "real.txt"), b"okay")
            for name in ("../outside", "C:/secrets", "a\\b", "/absolute"):
                with self.assertRaises(ValueError):
                    gate.bounded_file(root, name)
            with self.assertRaises(ValueError):
                gate.bounded_file(root, "real.txt", limit=1)
            with patch.object(Path, "is_symlink", return_value=True), self.assertRaises(ValueError):
                gate.bounded_file(root, "real.txt")

    def test_bounded_command_stops_before_oversized_response_consumed(self):
        process = MagicMock()
        process.__enter__.return_value = process
        process.stdout.read.return_value = b"x" * 21
        process.poll.return_value = None
        with patch.object(gate.subprocess, "Popen", return_value=process), self.assertRaises(ValueError):
            gate.bounded_command(["gh", "api", "fixture"], limit=20)
        process.stdout.read.assert_called_once_with(21)
        process.kill.assert_called_once()

    def test_bounded_command_success_and_timeout(self):
        process = MagicMock()
        process.__enter__.return_value = process
        process.stdout.read.return_value = b"bounded"
        process.wait.return_value = 0
        process.poll.return_value = 0
        with patch.object(gate.subprocess, "Popen", return_value=process):
            self.assertEqual(gate.bounded_command(["gh", "api", "fixture"], limit=20), b"bounded")
        process.poll.return_value = None
        with patch.object(gate.subprocess, "Popen", return_value=process), \
                patch.object(gate, "ThreadPoolExecutor") as executor, self.assertRaises(TimeoutError):
            executor.return_value.__enter__.return_value.submit.return_value.result.side_effect = TimeoutError
            gate.bounded_command(["gh", "api", "fixture"], limit=20)
        process.kill.assert_called_once()


if __name__ == "__main__":
    unittest.main()
