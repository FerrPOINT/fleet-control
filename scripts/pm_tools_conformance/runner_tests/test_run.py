"""Synthetic runner-boundary tests, separate from the 24 conformance cases.

No Hermes/model imports or real service/Git mutation. Doubled child exit0 is
deliberately untrusted. Success fixtures are NOT conformance/live evidence.
"""

import copy
import hashlib
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "qa"))
import run as runner
from receipt import EXPECTED_COUNTS, EXPECTED_SELECTORS, MANDATORY_IMPORTS, load_receipt, validate_receipt


def fixture():
    return dict(tests_run=24, failures=0, errors=0, skips=0,
                selectors=list(EXPECTED_SELECTORS), unsealed_imports=[],
                git_imports={name: {"git_blob": "a" * 40, "sha256": "b" * 64, "bytes": 1}
                             for name in MANDATORY_IMPORTS},
                python="synthetic host measurement", host_dependencies={}, test_status="PASS",
                producer_admission="BLOCKED", live_evidence=False,
                expected_counts=dict(EXPECTED_COUNTS), actual_counts=dict(EXPECTED_COUNTS),
                pins=dict(runner.PINS),
                git_reader={"pid": 123, "exit_code": 0, "requests": 9, "read_seconds": 0.001})


class RunnerReceiptTests(unittest.TestCase):
    def outer(self, receipt, exit_code=0, malformed=False, provenance_failure=False):
        # Owned temporary fixture tree, never the real retained evidence.
        owned = runner.ROOT / "qa/evidence"
        owned.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="pm-runner-unit-", dir=owned) as temp:
            root = Path(temp)
            def fake_git(repo, *args):
                return b"" if args == ("status", "--porcelain") else b"fixture-head\n"
            def fake_child(command, **kwargs):
                scratch = Path(command[command.index("--child") + 1])
                if receipt is not None:
                    text = "{" if malformed else json.dumps(receipt)
                    (scratch / "receipt.json").write_text(text, encoding="utf-8")
                return subprocess.CompletedProcess(command, exit_code, b"synthetic child", b"")
            provenance = Mock(side_effect=ValueError("synthetic provenance mismatch")
                              if provenance_failure else None)
            with patch.object(runner, "ROOT", root), patch.object(runner, "FLEET", root), \
                 patch.object(runner, "HERMES", root / "hermes"), \
                 patch.object(runner, "packet_inventory", return_value={"fixture": "hash"}), \
                 patch.object(runner, "verify_inputs"), patch.object(runner, "git", side_effect=fake_git), \
                 patch.object(runner.subprocess, "run", side_effect=fake_child), \
                 patch.object(runner, "verify_import_provenance", provenance), \
                 patch("builtins.print"):
                code = runner.run()
            reports = list((root / "qa/evidence").glob("run-*/terminal-report.json"))
            self.assertEqual(len(reports), 1)
            report = json.loads(reports[0].read_text(encoding="utf-8"))
            self.assertTrue(report["scratch_absent"])
            self.assertEqual(list(reports[0].parent.glob("probe-*")), [])
            return code, report, provenance

    def assert_outer_failure(self, receipt, **kwargs):
        code, report, provenance = self.outer(receipt, **kwargs)
        self.assertEqual(code, 1)
        self.assertEqual(report["status"], "FAIL")
        self.assertFalse(report.get("receipt_validated", False))
        provenance.assert_not_called()

    def test_exit0_without_receipt_is_failure(self):
        self.assert_outer_failure(None)

    def test_exit0_with_fail_receipt_is_failure(self):
        value = fixture()
        value.update(test_status="FAIL", tests_run=0, failures=1, errors=1, skips=1)
        self.assert_outer_failure(value)

    def test_exit0_with_wrong_inventory_is_failure(self):
        value = fixture()
        value["selectors"][-1] = "test_hermes.HermesProbes.test_forged_case"
        self.assert_outer_failure(value)

    def test_forged_exit0_and_empty_receipt_are_not_evidence(self):
        self.assert_outer_failure({})

    def test_malformed_json_is_failure(self):
        self.assert_outer_failure(fixture(), malformed=True)

    def test_nonzero_child_exit_cannot_be_overridden_by_pass_receipt(self):
        self.assert_outer_failure(fixture(), exit_code=1)

    def test_only_valid_synthetic_boundary_fixture_reaches_provenance_validation(self):
        code, report, provenance = self.outer(fixture())
        self.assertEqual(code, 0)
        self.assertEqual(report["status"], "PASS_OFFLINE_ONLY")
        self.assertTrue(report["receipt_validated"])
        provenance.assert_called_once()
        self.assertIs(report["execution"]["live_evidence"], False)

    def test_provenance_failure_even_after_valid_receipt_is_failure(self):
        code, report, provenance = self.outer(fixture(), provenance_failure=True)
        self.assertEqual(code, 1)
        self.assertEqual(report["status"], "FAIL")
        self.assertFalse(report.get("receipt_validated", False))
        provenance.assert_called_once()

    def test_duplicate_json_fields_and_nonfinite_numbers_denied(self):
        for text in ('{"test_status":"FAIL","test_status":"PASS"}', '{"counter":NaN}'):
            with patch.object(Path, "read_text", return_value=text), self.assertRaises(ValueError):
                load_receipt(Path("synthetic.json"))

    def test_duplicate_missing_or_added_selector_denied(self):
        for change in (lambda names: names.pop(), lambda names: names.append(names[0]),
                       lambda names: names.__setitem__(1, names[0])):
            value = fixture()
            change(value["selectors"])
            with self.subTest(names=value["selectors"]), self.assertRaises(ValueError):
                validate_receipt(value, 0, runner.PINS)

    def test_counts_status_flags_pins_and_reader_closure_are_fail_closed(self):
        mutations = [{"test_status": "FAIL"}, {"tests_run": 23}, {"failures": 1},
                     {"errors": 1}, {"skips": 1}, {"failures": False},
                     {"actual_counts": {}}, {"expected_counts": {}},
                     {"unsealed_imports": ["foreign.module"]}, {"unsealed_imports": None},
                     {"producer_admission": "READY"}, {"live_evidence": True},
                     {"live_evidence": 0}, {"pins": {}}, {"git_imports": {}},
                     {"git_reader": {"pid": 123, "exit_code": 1, "requests": 9, "read_seconds": 0}}]
        for change in mutations:
            value = fixture()
            value.update(change)
            with self.subTest(change=change), self.assertRaises(ValueError):
                validate_receipt(value, 0, runner.PINS)

    def test_missing_and_unknown_receipt_fields_denied(self):
        for field in fixture():
            value = fixture()
            del value[field]
            with self.subTest(field=field), self.assertRaises(ValueError):
                validate_receipt(value, 0, runner.PINS)
        value = fixture()
        value["unknown_grant"] = True
        with self.assertRaises(ValueError):
            validate_receipt(value, 0, runner.PINS)

    def test_invalid_import_metadata_denied(self):
        for change in ({"git_blob": "main"}, {"sha256": "fake"}, {"bytes": True}):
            value = fixture()
            value["git_imports"]["model_tools.py"].update(change)
            with self.subTest(change=change), self.assertRaises(ValueError):
                validate_receipt(value, 0, runner.PINS)


class GitBatchTests(unittest.TestCase):
    def setUp(self):
        self.body = b"print('canonical LF')\n"
        self.oid = hashlib.sha1(b"blob " + str(len(self.body)).encode() + b"\0" + self.body).hexdigest()
        self.frame = (self.oid + " blob " + str(len(self.body)) + "\n").encode() + self.body + b"\n"

    def test_canonical_frame_parses_and_hashes_exact_bytes(self):
        self.assertEqual(runner.read_git_blob(io.BytesIO(self.frame), self.oid), self.body)

    def test_wrong_header_truncated_and_mutated_body_denied(self):
        for frame in (b"missing\n", self.frame[:-3], self.frame.replace(b"canonical", b"CANONICAL")):
            with self.subTest(frame=frame), self.assertRaises(ValueError):
                runner.read_git_blob(io.BytesIO(frame), self.oid)

    def test_parent_checks_all_receipt_blob_hashes_against_git(self):
        imports = {"model_tools.py": {"git_blob": self.oid, "sha256": runner.sha(self.body),
                                     "bytes": len(self.body)}}
        def execute(*args, **kwargs):
            self.assertEqual(kwargs["input"], (self.oid + "\n").encode())
            return subprocess.CompletedProcess(args, 0, self.frame, b"")
        with patch.object(runner, "tree", return_value={"model_tools.py": self.oid}), \
             patch.object(runner.subprocess, "run", side_effect=execute):
            runner.verify_import_provenance(imports, timeout=1)
            bad = copy.deepcopy(imports)
            bad["model_tools.py"]["sha256"] = "0" * 64
            with self.assertRaises(ValueError):
                runner.verify_import_provenance(bad, timeout=1)
        with patch.object(runner, "tree", return_value={}), self.assertRaises(ValueError):
            runner.verify_import_provenance(imports, timeout=1)

    def test_reader_rejects_foreign_oid_before_writing_and_caches_exact_blob(self):
        process = SimpleProcess(self.frame)
        with patch.object(runner.subprocess, "Popen", return_value=process):
            reader = runner.GitBlobReader(Path("synthetic-repo"), {self.oid})
        with self.assertRaises(ValueError):
            reader.read("f" * 40)
        self.assertEqual(process.stdin.getvalue(), b"")
        self.assertEqual(reader.read(self.oid), self.body)
        self.assertEqual(reader.read(self.oid), self.body)
        self.assertEqual(process.stdin.getvalue(), (self.oid + "\n").encode())
        self.assertEqual(reader.requests, 1)
        reader.close()
        self.assertTrue(process.waited)

    def test_cleanup_timeout_kills_only_owned_handle_and_cannot_claim_clean_exit(self):
        process = SimpleProcess(self.frame)
        process.returncode = -9
        process.wait = Mock(side_effect=[subprocess.TimeoutExpired("owned-reader", 3), -9])
        process.kill = Mock()
        with patch.object(runner.subprocess, "Popen", return_value=process):
            reader = runner.GitBlobReader(Path("synthetic-repo"), {self.oid})
        with self.assertRaises(RuntimeError):
            reader.close()
        process.kill.assert_called_once_with()
        self.assertEqual(process.wait.call_count, 2)


class SimpleProcess:
    def __init__(self, frame):
        self.stdin = io.BytesIO()
        self.stdout = io.BytesIO(frame)
        self.waited = False
        self.returncode = 0

    def wait(self, timeout=None):
        self.waited = True
        return self.returncode
