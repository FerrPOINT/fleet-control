"""Pure/static regressions. Never start frontend, browsers, containers, or Cargo."""
import io
from contextlib import redirect_stdout
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


def png_stream(body, *, width=375, height=812, encoding=(8, 2, 0, 0, 0)):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, *encoding))
            + chunk(b"IDAT", body) + chunk(b"IEND", b""))


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
    def test_materialization_is_closed_to_one_base_revision_path_and_exact_bytes(self):
        name = "crates/auth-server/migrations/0001_users_sessions.sql"
        self.assertEqual(gate.BASE_MATERIALIZED_FILES, {name: (
            "69f7ec8116b228dede62375c7c8b45e1af233887",
            "ef0fae09d1a5359eb23ade564541b03bc1f1514c2017317fc7922ced72c26d75",
        )})
        blob, actual, data = "a" * 40, "b" * 40, b"synthetic\r\n"
        with patch.dict(gate.BASE_MATERIALIZED_FILES, {name: (blob, gate.digest(data))}, clear=True):
            gate.verify_materialized_file(gate.BASE_SHA, name, blob, actual, data)
            for sha, path, expected_blob, body in (
                (gate.SOURCE_SHA, name, blob, data),
                ("f" * 40, name, blob, data),
                (gate.BASE_SHA, name + ".copy", blob, data),
                (gate.BASE_SHA, name, "c" * 40, data),
                (gate.BASE_SHA, name, blob, b"synthetic\n"),
                (gate.BASE_SHA, name, blob, data + b"altered"),
            ):
                with self.subTest(sha=sha, path=path), self.assertRaises(ValueError):
                    gate.verify_materialized_file(sha, path, expected_blob, actual, body)
        gate.verify_materialized_file(gate.SOURCE_SHA, "normal.txt", blob, blob, b"synthetic")
        with self.assertRaises(ValueError):
            gate.verify_materialized_file(gate.BASE_SHA, "unattested.txt", blob, actual, data)

    def test_failure_hints_never_echo_unrecognized_private_diagnostics(self):
        self.assertEqual(gate.safe_failure_hint(ValueError("Source bytes differ from exact committed tree")),
                         "source_bytes")
        self.assertEqual(gate.safe_failure_hint(ValueError(
            "Base materialized bytes differ from the pinned attribute contract")), "base_materialization")
        for error in (OSError("PRIVATE_SENTINEL"), KeyError("PRIVATE_SENTINEL"),
                      ValueError("Source bytes differ from exact committed tree PRIVATE_SENTINEL")):
            self.assertEqual(gate.safe_failure_hint(error), "unclassified")

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

    def test_png_invalid_pixel_stream_rejected_by_full_readback(self):
        bad = png_stream(b"PRIVATE_SENTINEL" * 100)
        files = dict(self.files)
        files["fixtures/example-chromium/fixture.png"] = bad
        checksums(files)
        with self.assertRaises(ValueError):
            gate.png_valid(bad)
        with self.assertRaises(ValueError):
            validate(zipped(files))

    def test_png_complete_scanlines_encoding_filters_and_stream_end_required(self):
        row = b"\0" + random.Random(23).randbytes(375 * 3)
        pixels = row * 812
        compressed = zlib.compress(pixels)
        invalid_later_filter = bytearray(pixels)
        invalid_later_filter[65 * len(row)] = 5
        for body in (compressed[:-1], zlib.compress(pixels[:-1]), zlib.compress(pixels + row),
                     zlib.compress(b"\5" + pixels[1:]), compressed + b"PRIVATE_SENTINEL",
                     compressed + zlib.compress(pixels), zlib.compress(invalid_later_filter)):
            with self.subTest(size=len(body)), self.assertRaises(ValueError):
                gate.png_valid(png_stream(body))
        for encoding in ((16, 2, 0, 0, 0), (8, 3, 0, 0, 0), (8, 2, 1, 0, 0),
                         (8, 2, 0, 1, 0), (8, 2, 0, 0, 1)):
            with self.subTest(encoding=encoding), self.assertRaises(ValueError):
                gate.png_valid(png_stream(compressed, encoding=encoding))
        with self.assertRaises(ValueError):
            gate.png_valid(png_stream(compressed, width=8192, height=32768))

    def test_png_header_and_end_chunk_lengths_and_truncated_chunks_rejected(self):
        data = png()
        def chunk(kind, body):
            return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body))
        for bad in (data[:8] + chunk(b"IHDR", data[16:29] + b"\0") + data[33:],
                    data[:-12] + chunk(b"IEND", b"\0"), data + b"\0" * 4,
                    data[:33] + chunk(b"IHDR", data[16:29]) + data[33:]):
            with self.assertRaises(ValueError):
                gate.png_valid(bad)

    def test_png_rgb_rgba_multi_idat_and_streaming_filter_boundaries(self):
        for channels, color in ((3, 2), (4, 6)):
            row = b"\4" + random.Random(23).randbytes(375 * channels)
            data = png_stream(zlib.compress(row * 812), encoding=(8, color, 0, 0, 0))
            gate.png_valid(data)
            size = int.from_bytes(data[33:37], "big")
            body = data[41:41 + size]
            def chunk(kind, part):
                return struct.pack(">I", len(part)) + kind + part + struct.pack(">I", zlib.crc32(kind + part))
            middle = len(body) // 2
            gate.png_valid(data[:33] + chunk(b"IDAT", body[:middle])
                           + chunk(b"IDAT", body[middle:]) + data[45 + size:])
            with self.assertRaises(ValueError):
                gate.png_valid(data[:33] + chunk(b"IDAT", body[:middle])
                               + chunk(b"sRGB", b"\0") + chunk(b"IDAT", body[middle:]) + data[45 + size:])

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
        process.poll.return_value = None
        def spawn(*args, **kwargs):
            kwargs["stdout"].write(b"x" * 21)
            kwargs["stdout"].flush()
            return process
        with patch.object(gate.subprocess, "Popen", side_effect=spawn), self.assertRaises(ValueError):
            gate.bounded_command(["gh", "api", "fixture"], limit=20)
        process.stdout.read.assert_not_called()
        process.kill.assert_called_once()
        process.wait.assert_called_once_with(timeout=5)

    def test_bounded_command_success_and_timeout(self):
        process = MagicMock()
        process.wait.return_value = 0
        process.poll.return_value = 0
        def spawn(*args, **kwargs):
            kwargs["stdout"].write(b"bounded")
            kwargs["stdout"].flush()
            return process
        with patch.object(gate.subprocess, "Popen", side_effect=spawn):
            self.assertEqual(gate.bounded_command(["gh", "api", "fixture"], limit=20), b"bounded")
        process.kill.assert_not_called()
        process.poll.return_value = None
        with patch.object(gate.subprocess, "Popen", return_value=process), \
                patch.object(gate.time, "monotonic", side_effect=[0, 121]), self.assertRaises(TimeoutError):
            gate.bounded_command(["gh", "api", "fixture"], limit=20)
        process.kill.assert_called_once()

    def test_command_timeout_returns_with_retained_stdout_handle_and_reaps_only_child(self):
        process, retained = MagicMock(), []
        process.poll.return_value = None
        def spawn(*args, **kwargs):
            retained.append(os.dup(kwargs["stdout"].fileno()))
            return process
        try:
            with patch.object(gate.subprocess, "Popen", side_effect=spawn), \
                    patch.object(gate.time, "monotonic", side_effect=[0, 121]), self.assertRaises(TimeoutError):
                gate.bounded_command(["gh", "api", "fixture"], limit=20)
            self.assertEqual(os.fstat(retained[0]).st_size, 0)
            process.kill.assert_called_once_with()
            process.wait.assert_called_once_with(timeout=5)
            process.stdout.read.assert_not_called()
        finally:
            for descriptor in retained:
                os.close(descriptor)

    def test_command_deadline_not_renewed_and_checked_after_readback(self):
        process = MagicMock()
        process.poll.return_value = None
        with patch.object(gate.subprocess, "Popen", return_value=process), \
                patch.object(gate.time, "monotonic", side_effect=[0, 119, 121]), \
                patch.object(gate.time, "sleep") as sleep, self.assertRaises(TimeoutError):
            gate.bounded_command(["gh", "api", "fixture"])
        sleep.assert_called_once_with(0.02)
        process.kill.assert_called_once()
        process.poll.return_value = 0
        with patch.object(gate.subprocess, "Popen", return_value=process), \
                patch.object(gate.time, "monotonic", side_effect=[0, 1, 121]), self.assertRaises(ValueError):
            gate.bounded_command(["gh", "api", "fixture"])

    def test_command_spool_read_is_bounded_and_nonzero_or_oversized_errors_refuse(self):
        process = MagicMock()
        process.poll.return_value = 0
        with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
            wrapped = MagicMock(wraps=output)
            wrapped.__enter__.return_value = wrapped
            def spawn(*args, **kwargs):
                kwargs["stdout"].write(b"bounded")
                kwargs["stdout"].flush()
                return process
            with patch.object(gate.tempfile, "TemporaryFile", side_effect=[wrapped, errors]), \
                    patch.object(gate.subprocess, "Popen", side_effect=spawn):
                self.assertEqual(gate.bounded_command(["gh", "api", "fixture"], limit=20), b"bounded")
            wrapped.read.assert_called_once_with(21)
        process.poll.return_value = 7
        with patch.object(gate.subprocess, "Popen", return_value=process), self.assertRaises(ValueError):
            gate.bounded_command(["gh", "api", "fixture"])
        process.poll.return_value = None
        def noisy(*args, **kwargs):
            kwargs["stderr"].write(b"PRIVATE_SENTINEL" * 10)
            kwargs["stderr"].flush()
            return process
        with patch.object(gate.subprocess, "Popen", side_effect=noisy), patch.object(gate, "MAX_FILE", 20), \
                self.assertRaisesRegex(ValueError, "^Control response exceeds size limit$"):
            gate.bounded_command(["gh", "api", "fixture"])
        process.kill.assert_called_once()


class FailureContracts(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.locations = gate.attested_test_locations(ROOT)

    def browser_report(self):
        return dict(errors=[dict(message="PRIVATE_SENTINEL")], suites=[dict(title="PRIVATE_SENTINEL", specs=[dict(
            title="PRIVATE_SENTINEL", file="runtime-controls.spec.ts", line=83, tests=[dict(
                projectName="webkit", status="unexpected", results=[dict(status="failed",
                    error=dict(message="PRIVATE_SENTINEL"), stdout=["PRIVATE_SENTINEL"],
                    attachments=[dict(path="PRIVATE_SENTINEL")])])])])])

    def browser(self, value=None):
        return gate.safe_browser_failure(gate.canonical(value or self.browser_report()), self.locations)

    def receipt(self):
        return dict(version=1, kind="safe_frontend_failure", status="failure", repository=gate.REPOSITORY,
            branch=gate.BRANCH, workflow_path=gate.WORKFLOW, workflow_sha="a" * 40, run_id=123, run_attempt=2,
            source_sha=gate.SOURCE_SHA, source_tree=gate.SOURCE_TREE, source_parents=gate.SOURCE_PARENTS,
            base_sha=gate.BASE_SHA, base_tree=gate.BASE_TREE, schema_sha256=gate.SCHEMA_SHA256,
            test_inventory_sha256=gate.digest(gate.canonical(self.locations)),
            control_sha256={name: gate.digest((ROOT / name).read_text(encoding="utf-8").encode())
                            for name in sorted(gate.WRITE_SET)},
            gate="fixtures", completed_gates=list(gate.GATES)[:list(gate.GATES).index("fixtures")],
            category="exit", exit_code=1, browser=self.browser(), cleanup=dict(private_absent=True),
            **gate.QUALIFIED_INPUTS, **gate.FAILURE_SCOPE)

    def verify(self, value):
        return gate.validate_failure(value, workflow_sha="a" * 40, run_id=123, attempt=2, locations=self.locations)

    def files(self, value=None):
        data = gate.canonical(value or self.receipt())
        return {gate.FAILURE_FILE: data, "SHA256SUMS": (gate.digest(data) + "  " + gate.FAILURE_FILE + "\n").encode()}

    def metadata(self, payload):
        run, artifact = readback_metadata(payload)
        run["conclusion"] = "failure"
        artifact["name"] = "fleet-frontend-failure-b0ad56c-123-2"
        return run, artifact

    def readback(self, payload, run=None, artifact=None):
        actual_run, actual_artifact = self.metadata(payload)
        return gate.validate_failure_readback(run or actual_run, artifact or actual_artifact, payload,
            run_id=123, attempt=2, workflow_sha="a" * 40, artifact_id=456, artifact_digest=gate.digest(payload))

    def test_fixture_parser_never_serializes_private_fields_or_runtime_titles(self):
        result = self.browser()
        self.assertEqual(result["report"], "valid")
        self.assertEqual(result["diagnostics"], [dict(file="frontend/e2e/runtime-controls.spec.ts", line=83,
            project="webkit", status="unexpected", results=["failed"])])
        self.assertEqual(result["browsers"]["webkit"]["unexpected"], 1)
        self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(result).decode())
        self.verify(self.receipt())

    def test_file_and_line_are_canonical_source_declarations_only(self):
        self.assertIn(83, self.locations["frontend/e2e/runtime-controls.spec.ts"])
        self.assertNotIn(86, self.locations["frontend/e2e/runtime-controls.spec.ts"])
        for file, line in (("../runtime-controls.spec.ts", 83), ("services-base/private.spec.ts", 83),
                           ("/tmp/e2e/runtime-controls.spec.ts", 83), ("runtime-controls.spec.tsPRIVATE_SENTINEL", 83),
                           ("runtime-controls.spec.ts", 86), ("runtime-controls.spec.ts", True),
                           ("runtime-controls.spec.ts", "83")):
            report = self.browser_report()
            report["suites"][0]["specs"][0].update(file=file, line=line)
            self.assertEqual(self.browser(report)["report"], "rejected")
        for file in ("runtime-controls.spec.ts", "e2e/runtime-controls.spec.ts", "frontend/e2e/runtime-controls.spec.ts"):
            report = self.browser_report()
            report["suites"][0]["specs"][0]["file"] = file
            self.assertEqual(self.browser(report)["report"], "valid")

    def test_unknown_browser_status_project_attempt_and_partial_data_rejected(self):
        for change in (dict(projectName="PRIVATE_SENTINEL"), dict(status="PRIVATE_SENTINEL"),
                       dict(results=[dict(status="PRIVATE_SENTINEL")]), dict(results=[dict(status="failed")] * 4)):
            report = self.browser_report()
            test = report["suites"][0]["specs"][0]["tests"][0]
            report["suites"][0]["specs"][0]["tests"].append(dict(test, **change))
            result = self.browser(report)
            self.assertEqual(result["report"], "rejected")
            self.assertEqual(result["diagnostics"], [])
            self.assertTrue(all(not any(p.values()) for p in result["browsers"].values()))

    def test_browser_report_bounds_missing_invalid_and_recursive_input(self):
        for raw, expected in ((None, "unavailable"), (b"PRIVATE_SENTINEL", "rejected"),
                              (b"x" * (4 * 1024 ** 2 + 1), "truncated"), (b"[]", "rejected")):
            self.assertEqual(gate.safe_browser_failure(raw, self.locations)["report"], expected)
        report = self.browser_report()
        leaf = report["suites"][0]
        for _ in range(18):
            leaf["suites"] = [dict()]
            leaf = leaf["suites"][0]
        self.assertEqual(self.browser(report)["report"], "rejected")
        self.assertEqual(self.browser(dict(suites=[dict()] * 4097))["report"], "rejected")

    def test_failure_receipt_rejects_private_fields_pins_and_acceptance_claims(self):
        for change in (dict(message="PRIVATE_SENTINEL"), dict(source_sha="f" * 40), dict(workflow_sha="f" * 40),
                       dict(schema_sha256="0" * 64), dict(test_inventory_sha256="0" * 64),
                       dict(control_sha256={}), dict(live_pm_acceptance=True), dict(frontend_unit_build_fixture=True),
                       dict(cleanup=dict(private_absent=1)), dict(exit_code=True), dict(exit_code=0),
                       dict(category="PRIVATE_SENTINEL"), dict(category="timeout"), dict(gate="PRIVATE_SENTINEL"),
                       dict(completed_gates=["fixtures"]), dict(completed_gates=list(gate.GATES))):
            with self.subTest(change=change), self.assertRaises(ValueError):
                self.verify(dict(self.receipt(), **change))
        self.verify(dict(self.receipt(), category="timeout", exit_code=None))

    def test_failure_browser_schema_cannot_smuggle_raw_values(self):
        for kind in ("extra", "title", "line", "file", "project", "status", "results", "count", "duplicate", "unavailable"):
            value = self.receipt()
            browser, row = value["browser"], value["browser"]["diagnostics"][0]
            if kind == "extra":
                browser["raw"] = "PRIVATE_SENTINEL"
            elif kind == "title":
                row["title"] = "PRIVATE_SENTINEL"
            elif kind in ("file", "project", "status"):
                row[kind] = "PRIVATE_SENTINEL"
            elif kind == "line":
                row[kind] = 84
            elif kind == "results":
                row[kind] = ["PRIVATE_SENTINEL"]
            elif kind == "count":
                browser["browsers"]["webkit"]["unexpected"] = True
            elif kind == "duplicate":
                browser["diagnostics"].append(dict(row))
            else:
                browser["report"] = "unavailable"
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                self.verify(value)

    def test_full_authenticated_failure_readback_is_distinct_from_success(self):
        files = self.files()
        payload = zipped(files)
        self.assertEqual(self.readback(payload), files)
        self.assertNotIn("PRIVATE_SENTINEL", files[gate.FAILURE_FILE].decode())
        with self.assertRaises(ValueError):
            validate(payload)
        with self.assertRaises(ValueError):
            self.readback(zipped(fixture_files()))

    def test_failure_readback_requires_exact_run_artifact_digest_and_cleanup_schema(self):
        payload = zipped(self.files())
        for key, value in (("conclusion", "success"), ("run_attempt", 1), ("head_sha", "f" * 40),
                           ("event", "workflow_dispatch"), ("status", "in_progress"), ("head_branch", "main"),
                           ("path", ".github/workflows/ci.yml"), ("repository", dict(full_name="other/repo"))):
            run, _ = self.metadata(payload)
            run[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.readback(payload, run=run)
        for key, value in (("id", 1), ("expired", True), ("digest", "sha256:" + "0" * 64), ("name", "other"),
                           ("workflow_run", dict(id=123, head_sha="a" * 40, head_branch="other"))):
            _, artifact = self.metadata(payload)
            artifact[key] = value
            with self.subTest(key=key), self.assertRaises(ValueError):
                self.readback(payload, artifact=artifact)
        value = self.receipt()
        value["browser"]["diagnostics"][0]["line"] = 84
        with self.assertRaises(ValueError):
            self.readback(zipped(self.files(value)))

    def test_failure_zip_member_checksum_size_and_canonical_json_bounds(self):
        files = self.files()
        for name in ("../failure.json", "PRIVATE_SENTINEL.log", "source.ts", "fixtures/image.png"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                self.readback(zipped(files, (name, b"PRIVATE_SENTINEL")))
        with self.assertWarns(UserWarning):
            payload = zipped(files, (gate.FAILURE_FILE, files[gate.FAILURE_FILE]))
        with self.assertRaises(ValueError):
            self.readback(payload)
        with self.assertRaises(ValueError):
            self.readback(zipped(dict(files, SHA256SUMS=b"0" * 64)))
        with patch.object(gate, "FAILURE_LIMIT", 100), self.assertRaises(ValueError):
            self.readback(zipped(files))
        files[gate.FAILURE_FILE] = b" " + files[gate.FAILURE_FILE]
        files["SHA256SUMS"] = (gate.digest(files[gate.FAILURE_FILE]) + "  failure.json\n").encode()
        with self.assertRaises(ValueError):
            self.readback(zipped(files))

    def cleanup_fixture(self, root, value):
        private = root / "fleet-frontend-private"
        private.mkdir()
        (private / "private.log").write_bytes(b"PRIVATE_SENTINEL")
        (private / "failure-pending.json").write_bytes(gate.canonical(value))
        return private

    def test_finally_cleanup_seals_absence_and_only_safe_receipt(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            private = self.cleanup_fixture(root, dict(self.receipt(), cleanup=dict(private_absent=False)))
            with patch.object(gate, "hosted_identity", return_value=(ROOT.parent, "a" * 40)), \
                    patch.object(gate, "workspace_temp", return_value=root), \
                    patch.object(gate, "attested_test_locations", return_value=self.locations), \
                    patch.dict(os.environ, GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="2"):
                gate.cleanup()
            self.assertFalse(private.exists())
            files = {p.name: p.read_bytes() for p in (root / "fleet-frontend-failure").iterdir()}
            self.assertEqual(self.readback(zipped(files)), files)
            self.assertTrue(json.loads(files[gate.FAILURE_FILE])["cleanup"]["private_absent"])

    def test_cleanup_error_retains_failure_not_false_absence(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            self.cleanup_fixture(root, dict(self.receipt(), cleanup=dict(private_absent=False)))
            with patch.object(gate, "hosted_identity", return_value=(ROOT.parent, "a" * 40)), \
                    patch.object(gate, "workspace_temp", return_value=root), \
                    patch.object(gate, "attested_test_locations", return_value=self.locations), \
                    patch.dict(os.environ, GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="2"), \
                    patch.object(gate.shutil, "rmtree", side_effect=OSError("PRIVATE_SENTINEL")), self.assertRaises(OSError):
                gate.cleanup()
            value = json.loads((root / "fleet-frontend-failure/failure.json").read_bytes())
            self.assertFalse(value["cleanup"]["private_absent"])
            self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(value).decode())

    def test_invalid_pending_receipt_still_cleans_private_and_cannot_publish(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            private = self.cleanup_fixture(root, dict(self.receipt(), raw="PRIVATE_SENTINEL"))
            with patch.object(gate, "hosted_identity", return_value=(ROOT.parent, "a" * 40)), \
                    patch.object(gate, "workspace_temp", return_value=root), \
                    patch.object(gate, "attested_test_locations", return_value=self.locations), \
                    patch.dict(os.environ, GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="2"), self.assertRaises(ValueError):
                gate.cleanup()
            self.assertFalse(private.exists())
            self.assertFalse((root / "fleet-frontend-failure").exists())

    def test_gate_command_exit_and_timeout_are_numeric_or_fixed_only(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            for error in (subprocess.CompletedProcess(["pnpm"], 17), subprocess.TimeoutExpired(["PRIVATE_SENTINEL"], 1200)):
                state = dict(gates=[])
                with patch.object(gate, "load_state", return_value=(ROOT, root, state)), \
                        patch.object(gate, "verify_parity", return_value=(ROOT, ROOT)), \
                        patch.object(gate.subprocess, "run", side_effect=error if isinstance(error, Exception) else None,
                                     return_value=error), self.assertRaises(gate.GateFailure) as caught:
                    gate.gate("base")
                self.assertNotIn("PRIVATE_SENTINEL", str(caught.exception))
                self.assertEqual(caught.exception.category, "timeout" if isinstance(error, Exception) else "exit")
                self.assertEqual(caught.exception.code, None if isinstance(error, Exception) else 17)
                self.assertEqual(state["gates"], [])
                (root / "base.log").unlink()

    def test_record_failure_reads_only_json_and_retains_original_numeric_exit(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder).resolve()
            private, controls = root / "fleet-frontend-private", root / "controls"
            private.mkdir()
            for name in gate.WRITE_SET:
                path = controls / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes((ROOT / name).read_text(encoding="utf-8").encode())
            (private / "preflight.json").write_bytes(gate.canonical(dict(
                workflow_sha="a" * 40, run_id="123", attempt="2", public=True)))
            (private / "state.json").write_bytes(gate.canonical(dict(workflow_sha="a" * 40,
                gates=self.receipt()["completed_gates"])))
            (private / "fixtures.log").write_bytes(b"PRIVATE_SENTINEL")
            (private / "browser.json").write_bytes(gate.canonical(self.browser_report()))
            with patch.object(gate, "controls_preflight", return_value=(root, "a" * 40)), \
                    patch.object(gate, "workspace_temp", return_value=root), \
                    patch.object(gate, "attested_test_locations", return_value=self.locations), \
                    patch.object(gate, "qualified_inputs", return_value=gate.QUALIFIED_INPUTS), \
                    patch.dict(os.environ, GITHUB_RUN_ID="123", GITHUB_RUN_ATTEMPT="2"), \
                    patch.object(gate, "bounded_file", wraps=gate.bounded_file) as read:
                gate.record_failure("fixtures", gate.GateFailure("exit", 17))
                self.assertFalse(any(call.args[1].endswith(".log") for call in read.call_args_list))
                value = json.loads((private / "failure-pending.json").read_bytes())
                self.assertEqual(value["exit_code"], 17)
                self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(value).decode())
                self.assertFalse(value["cleanup"]["private_absent"])
                self.assertEqual(value["browser"]["report"], "valid")
                with self.assertRaises(FileExistsError):
                    gate.record_failure("fixtures", gate.GateFailure("timeout"))
                self.assertEqual(json.loads((private / "failure-pending.json").read_bytes()), value)
                (private / "failure-pending.json").unlink()
                (private / "browser.json").write_bytes(b"x" * (4 * 1024 ** 2 + 1))
                gate.record_failure("fixtures", OSError("PRIVATE_SENTINEL"))
                value = json.loads((private / "failure-pending.json").read_bytes())
                self.assertEqual(value["category"], "control_error")
                self.assertIsNone(value["exit_code"])
                self.assertEqual(value["browser"]["report"], "truncated")
                self.assertNotIn("PRIVATE_SENTINEL", gate.canonical(value).decode())
                (private / "failure-pending.json").unlink()
                gate.record_failure("fixtures", TimeoutError("PRIVATE_SENTINEL"))
                value = json.loads((private / "failure-pending.json").read_bytes())
                self.assertEqual(value["category"], "timeout")
                self.assertIsNone(value["exit_code"])

    def test_record_failure_error_cannot_replace_original_failure_or_echo_details(self):
        error = gate.GateFailure("exit", 17)
        stream = io.StringIO()
        with patch("sys.argv", ["helper", "gate", "fixtures"]), \
                patch.object(gate, "gate", side_effect=error), \
                patch.object(gate, "record_failure", side_effect=OSError("PRIVATE_SENTINEL")) as record, \
                redirect_stdout(stream), self.assertRaises(gate.GateFailure) as caught:
            gate.main()
        self.assertIs(caught.exception, error)
        record.assert_called_once_with("fixtures", error)
        self.assertNotIn("PRIVATE_SENTINEL", stream.getvalue())

    def test_attested_test_inventory_refuses_worktree_line_drift(self):
        actual_git = gate.git
        def changed(root, name, limit=gate.MAX_FILE):
            return b"test('PRIVATE_SENTINEL', async () => {})\n"
        with patch.object(gate, "bounded_file", side_effect=changed), self.assertRaisesRegex(ValueError, "canonical Git blob"):
            gate.attested_test_locations(ROOT)
        self.assertIs(gate.git, actual_git)

    def test_failure_readback_rejects_zip_symlink_and_true_claim_even_with_rehashed_receipt(self):
        files = self.files(dict(self.receipt(), live_runtime_acceptance=True))
        with self.assertRaises(ValueError):
            self.readback(zipped(files))
        output = io.BytesIO()
        with zipfile.ZipFile(output, "w") as archive:
            info = zipfile.ZipInfo(gate.FAILURE_FILE)
            info.create_system = 3
            info.external_attr = 0o120777 << 16
            archive.writestr(info, self.files()[gate.FAILURE_FILE])
            archive.writestr("SHA256SUMS", self.files()["SHA256SUMS"])
        with self.assertRaises(ValueError):
            self.readback(output.getvalue())

    def test_workflow_failure_upload_is_separate_after_always_cleanup(self):
        workflow = (ROOT / gate.WORKFLOW).read_text()
        self.assertLess(workflow.index("hosted_frontend_gate.py cleanup"), workflow.index("Upload bounded safe failure"))
        block = workflow.split("- name: Upload bounded safe failure", 1)[1]
        self.assertIn("if: failure()", block)
        self.assertIn("path: ${{ runner.temp }}/fleet-frontend-failure", block)
        self.assertNotIn("fleet-frontend-private", block)
        self.assertNotIn("continue-on-error", workflow)
        self.assertEqual(len(gate.GATES), 23)


if __name__ == "__main__":
    unittest.main()
