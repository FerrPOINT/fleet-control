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


if __name__ == "__main__":
    unittest.main()
