"""Exercise the exact Rust-owned Python bootstrap without Docker or credentials."""

import json
import os
from pathlib import Path
import py_compile
import re
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "backend/infra/src/runtime/container_control.rs"


class ContainerControlLoaderTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="fleet-control-loader-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.scripts = self.root / "scripts"
        self.scripts.mkdir()
        matches = re.findall(r'const BOOTSTRAP: &str = r#"(.*?)"#;', SOURCE.read_text(), re.S)
        self.assertEqual(len(matches), 1)
        self.bootstrap = matches[0]
        self.sources = [
            b"VALUE='verified'\n",
            b"from scripts.runtime_boundary import VALUE\n",
            b"import json,sys\nfrom scripts.runtime_bootstrap import VALUE\n"
            b"def main():\n"
            b" from scripts.runtime_replacement import RESULT\n"
            b" r=json.load(sys.stdin.buffer)\n"
            b" print(json.dumps({'protocol_version':1,'action':r['action'],'result':RESULT}))\n"
            b" return 0\n",
            b"from scripts.runtime_bootstrap import VALUE\nRESULT=VALUE\n",
        ]
        for name, content in zip(self.names(), self.sources):
            (self.scripts / (name + ".py")).write_bytes(content)

    @staticmethod
    def names():
        return ("runtime_boundary", "runtime_bootstrap", "runtime_control", "runtime_replacement")

    def execute(self, *, sources=None, entry=None, request=None):
        payload = {
            "sources": [value.decode("utf-8") for value in
                        (self.sources if sources is None else sources)],
            "request": request or {"protocol_version": 1, "action": "observe"},
        }
        if entry is not None:
            payload["entry"] = entry
        return subprocess.run(
            [sys.executable, "-I", "-B", "-c", self.bootstrap, str(self.root)],
            input=json.dumps(payload).encode("utf-8"), capture_output=True, timeout=10,
        )

    def assert_verified(self, result):
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(json.loads(result.stdout), {
            "protocol_version": 1, "action": "observe", "result": "verified",
        })

    def test_binary_stdin_and_pinned_dependency_imports(self):
        self.assert_verified(self.execute())

    def test_original_stop_readback_private_entry_never_dispatches_control_main(self):
        sources=[
            b"import hashlib,json\nclass BoundaryError(Exception): pass\n"
            b"def closed(value,keys):\n if set(value)!=set(keys): raise BoundaryError('private-secret')\n"
            b"def canonical_uuid(value): pass\n"
            b"def digest(value): return hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':')).encode()).hexdigest()\n",
            b"VALUE='unused'\n",
            b"def main(): raise AssertionError('control dispatch forbidden')\n"
            b"def validate_request(value): return value['action']\n"
            b"def Engine(context): return object()\n"
            b"def recovered_mount_guard(engine,anchor): return lambda *args: None\n",
            b"def stop_readback(*args): return {'state':'observed','operation_id':args[5]}\n",
        ]
        request={"anchor":{"protocol_version":3,"action":"observe","context":"protected","policy":{},
            "registration":{},"compose":str(self.root/'compose'),"journal":str(self.root/'launch')},
            "operation_id":"original-stop","stop_journal":str(self.root/'stop')}
        result=self.execute(sources=sources,entry="read_original_stop",request=request)
        self.assertEqual(result.returncode,0,result.stderr.decode())
        envelope=json.loads(result.stdout)
        self.assertEqual(envelope["action"],"read_original_stop")
        self.assertEqual(envelope["result"],{"state":"observed","operation_id":"original-stop"})
        self.assertEqual(len(envelope["request_sha256"]),64)
        request["foreign"]="private-secret"
        result=self.execute(sources=sources,entry="read_original_stop",request=request)
        self.assertEqual(result.returncode,2)
        self.assertEqual(json.loads(result.stdout)["result"],{"state":"held"})
        self.assertNotIn(b"private-secret",result.stdout+result.stderr)

    def test_valid_poisoned_bytecode_is_not_executed(self):
        path = self.scripts / "runtime_boundary.py"
        path.write_bytes(self.sources[0].replace(b"verified", b"poisoned"))
        py_compile.compile(str(path), doraise=True)
        stat = path.stat()
        path.write_bytes(self.sources[0])
        os.utime(path, ns=(stat.st_atime_ns, stat.st_mtime_ns))
        # Prove this cache is valid and would be used by the previous import path.
        imported = subprocess.run(
            [sys.executable, "-I", "-c",
             "import sys; sys.path.insert(0,sys.argv[1]); "
             "from scripts.runtime_boundary import VALUE; print(VALUE)", str(self.root)],
            capture_output=True, timeout=10,
        )
        self.assertEqual(imported.returncode, 0)
        self.assertEqual(imported.stdout.strip(), b"poisoned")
        self.assert_verified(self.execute())

    def test_source_replacement_after_capture_does_not_change_execution(self):
        for name in self.names():
            (self.scripts / (name + ".py")).write_text("raise RuntimeError('replacement')\n")
        self.assert_verified(self.execute())

    def test_package_initializer_is_never_executed(self):
        (self.scripts / "__init__.py").write_text("raise RuntimeError('initializer')\n")
        self.assert_verified(self.execute())

    def test_unpinned_checkout_module_cannot_be_imported(self):
        (self.scripts / "unpinned.py").write_text("VALUE='untrusted'\n")
        sources = self.sources.copy()
        sources[1] = b"from scripts.unpinned import VALUE\n"
        result = self.execute(sources=sources)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, b"")

    def test_captured_utf8_source_is_preserved(self):
        sources = self.sources.copy()
        sources[0] = "VALUE='v\u00e9rified'\n".encode("utf-8")
        result = self.execute(sources=sources)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(json.loads(result.stdout)["result"], "v\u00e9rified")

    def rust_fixture(self, filename, function, bindings):
        text = (ROOT / "backend/infra/src/runtime" / filename).read_text()
        marker = "async fn " + function + "("
        self.assertEqual(text.count(marker), 1)
        body = text.split(marker, 1)[1]
        setup = body.split("let control = ContainerControl::new(", 1)[0]
        self.assertEqual(re.findall(r'"(runtime_\w+\.py)"', setup),
                         [name + ".py" for name in self.names()])
        arrays = re.findall(r'let sources = \[(.*?)\];', setup, re.S)
        self.assertEqual(len(arrays), 1)
        self.assertEqual(arrays[0].count(".to_owned()"), 3)
        self.assertEqual(len(re.findall(r'\bsource\b', arrays[0])), 1)
        templates = re.findall(r'let source = format!\(\s*r#"(.*?)"#,', setup, re.S)
        self.assertEqual(len(templates), 1)
        # This is only the two reviewed fixture templates, not a Rust format parser.
        source = templates[0].replace("{{", "{").replace("}}", "}")
        for key, value in bindings.items():
            slot = "{" + key + ":?}"
            self.assertEqual(source.count(slot), 1)
            source = source.replace(slot, json.dumps(value))
        self.assertNotRegex(source, r'\{\w+:\?\}')
        return source.encode()

    def test_rust_preparation_fixture_uses_four_modules_and_main_entry(self):
        operation = "00000000-0000-4000-8000-000000000001"
        receipt = {"state": "prepared", "registration": {"operation_id": operation}}
        self.sources[2] = self.rust_fixture(
            "container_preparation_tests.rs",
            "adapter_fake_preserves_original_command_on_unknown_readback",
            {"operation": operation, "encoded": json.dumps(receipt)},
        )
        request = {
            "protocol_version": 1, "action": "prepare", "context": "fixture",
            "policy": {}, "compose": str(self.root / "start.json"),
            "journal": str(self.root / "launch.sqlite"),
            "process": {"environment": {"API_SERVER_KEY": "private-original"}},
            "operation_id": operation, "creation_compose": str(self.root / "create.json"),
            "creation_journal": str(self.root / "create.sqlite"),
        }
        first = self.execute(request=request)
        self.assertEqual(first.returncode, 2, first.stderr.decode())
        self.assertEqual(json.loads(first.stdout)["result"], {"state": "held"})
        request["action"] = "reconcile_preparation"
        for _ in range(2):
            replay = self.execute(request=request)
            self.assertEqual(replay.returncode, 0, replay.stderr.decode())
            self.assertEqual(json.loads(replay.stdout)["result"], receipt)
        self.assertEqual((self.root / "create.sqlite").read_text(),
                         "prepare\nreconcile_preparation\nreconcile_preparation\n")

    def test_rust_observe_fixture_uses_four_modules_and_read_only_retry(self):
        original = {"operation_id": "original-operation"}
        snapshot = {"init_pid": 42, "started_at": "2026-10-09T10:00:00Z"}
        receipt = {"state": "observed", "observation": "running", "snapshot": snapshot}
        self.sources[2] = self.rust_fixture(
            "container_activation.rs",
            "read_only_observe_failure_retries_original_command_until_recovery",
            {"original": json.dumps(original), "receipt": json.dumps(receipt)},
        )
        request = {"protocol_version": 1, "action": "observe", "registration": original}
        first = self.execute(request=request)
        self.assertEqual(first.returncode, 2, first.stderr.decode())
        self.assertEqual(json.loads(first.stdout)["result"],
                         {"state": "held", "observation": "unavailable", "snapshot": None})
        second = self.execute(request=request)
        self.assertEqual(second.returncode, 0, second.stderr.decode())
        self.assertEqual(json.loads(second.stdout)["result"], receipt)
        self.assertEqual((self.root / "read-attempts").read_text(), "observe\nobserve\n")

    def test_heartbeat_rust_fixture_materializes_the_entire_sealed_source_set(self):
        text = SOURCE.read_text()
        body = text.split("async fn foreign_owner_cannot_deliver_or_extend_any_heartbeat()", 1)[1]
        setup = body.split("let (mut files, r, _)", 1)[0]
        self.assertEqual(re.findall(r'"(runtime_\w+\.py)"', setup),
                         [name + ".py" for name in self.names()])

    def test_lineage_single_down_asserts_exact_recovered_activation_removal(self):
        text = (ROOT / "backend/migration/src/lineage_tests.rs").read_text()
        body = text.split("async fn fresh_canonical_install_is_repeatable()", 1)[1]
        body = body.split("async fn common_prefix_completes_with_canonical_foundation()", 1)[0]
        self.assertIn("Migrator::down(&fixture.db, Some(1))", body)
        self.assertIn("assert_eq!(after_down.len(), before.len() - 1)", body)
        self.assertIn(".filter(|(version, _)| version != RECOVERED_ACTIVATION)", body)
        self.assertIn("Migrator::up(&fixture.db, None)", body)
        self.assertNotIn("ledger(&fixture.db).await.len(), 18", body)


if __name__ == "__main__":
    unittest.main()
