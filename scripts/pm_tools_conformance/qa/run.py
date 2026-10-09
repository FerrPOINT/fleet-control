"""Light, offline conformance runner. Imports Hermes Python from exact Git blobs.

No source checkout writes, model, service, container, plugin install or credential
access. Missing dependencies/errors fail the suite, never become silent skips.
"""

import argparse
import faulthandler
import hashlib
import importlib.abc
import importlib.util
import io
import json
import os
from pathlib import Path
import platform
import re
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import uuid

from receipt import EXPECTED_COUNTS, EXPECTED_SELECTORS, load_receipt, validate_receipt


ROOT = Path(__file__).resolve().parents[1]
FLEET = ROOT.parents[1]
HERMES = None  # Set only by the explicit --hermes-repo argument.
PINS = {
    "fleet": "ede1e41e843b3992757d4e16c051db6667da7add",
    "hermes": "bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3",
}
FLEET_PATHS = (
    "backend/infra/src/pm_credentials.rs",
    "backend/infra/src/pm_credentials/coordinator.rs",
    "backend/domain/src/pm_execution.rs",
    "backend/infra/src/runtime/mod.rs",
    "backend/api/src/routes/pm_runtime.rs",
    "backend/app/src/pm_draft.rs",
)


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args], stderr=subprocess.PIPE,
                                   env=git_environment())


def git_environment():
    return {**os.environ, "GIT_NO_LAZY_FETCH": "1", "GIT_TERMINAL_PROMPT": "0", "GIT_OPTIONAL_LOCKS": "0"}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def packet_inventory():
    files = [*ROOT.glob("qa/*.py"), *ROOT.glob("probes/*.py"), *ROOT.glob("runner_tests/*.py"),
             ROOT / "README.md", ROOT / ".gitignore",
             FLEET / "docs/contracts/PM_TOOLS_HANDOFF_REQUIREMENTS.md",
             FLEET / "docs/README.md", FLEET / "docs/TESTING.md"]
    return {p.relative_to(FLEET).as_posix(): sha(p.read_bytes()) for p in sorted(files) if p.is_file()}


def verify_inputs():
    for name, repo in (("fleet", FLEET), ("hermes", HERMES)):
        if git(repo, "rev-parse", PINS[name] + "^{commit}").decode().strip() != PINS[name]:
            raise RuntimeError("wrong pinned object")
    if Path(git(FLEET, "rev-parse", "--show-toplevel").decode().strip()).resolve() != FLEET:
        raise RuntimeError("runner is not in its own Fleet checkout")
    git(FLEET, "merge-base", "--is-ancestor", PINS["fleet"], "HEAD")
    # Code uses Git blobs; package resource/discovery paths also need the pin.
    if git(HERMES, "rev-parse", "HEAD").decode().strip() != PINS["hermes"]:
        raise RuntimeError("Hermes resource checkout is not at pinned HEAD")
    if git(HERMES, "status", "--porcelain").strip():
        raise RuntimeError("Hermes resource checkout is not clean")


def tree(repo, pin):
    records = git(repo, "ls-tree", "-r", "-z", pin).split(b"\0")
    result = {}
    for record in records:
        if not record:
            continue
        header, path = record.split(b"\t", 1)
        mode, kind, oid = header.decode("ascii").split()
        if kind == "blob" and mode in ("100644", "100755"):
            result[path.decode("utf-8")] = oid
    return result


def read_git_blob(stream, oid):
    header = stream.readline().split()
    if len(header) != 3 or header[:2] != [oid.encode(), b"blob"]:
        raise ValueError("invalid Git batch header")
    size = int(header[2])
    if not 0 <= size <= 32 * 1024 * 1024:
        raise ValueError("invalid Python blob size")
    body = stream.read(size)
    if len(body) != size or stream.read(1) != b"\n":
        raise ValueError("truncated Git batch body")
    actual = hashlib.sha1(b"blob " + str(size).encode() + b"\0" + body).hexdigest()
    if actual != oid:
        raise ValueError("Git batch object hash mismatch")
    return body


class GitBlobReader:
    """One owned read-only Git process; only pinned blob IDs reach its stdin."""
    def __init__(self, repo, allowed):
        self.allowed = frozenset(allowed)
        self.lock = threading.RLock()
        self.cache = {}
        self.requests = 0
        self.seconds = 0.0
        self.process = subprocess.Popen(["git", "-C", str(repo), "cat-file", "--batch"],
                                        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=subprocess.DEVNULL, env=git_environment())

    def read(self, oid):
        if oid not in self.allowed or not re.fullmatch(r"[0-9a-f]{40}", oid):
            raise ValueError("blob outside pinned tree")
        with self.lock:
            if oid in self.cache:
                return self.cache[oid]
            started = time.monotonic()
            self.process.stdin.write((oid + "\n").encode("ascii"))
            self.process.stdin.flush()
            body = read_git_blob(self.process.stdout, oid)
            self.requests += 1
            self.seconds += time.monotonic() - started
            self.cache[oid] = body
            return body

    def close(self):
        self.process.stdin.close()
        self.process.stdout.close()
        try:
            self.process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            self.process.kill()  # Only this exact Popen handle, never a PID search/tree kill.
            self.process.wait()
        if self.process.returncode != 0:
            raise RuntimeError("owned Git blob reader did not exit cleanly")


def verify_import_provenance(imports, timeout):
    files = tree(HERMES, PINS["hermes"])
    for name, entry in imports.items():
        if files.get(name) != entry["git_blob"]:
            raise ValueError("receipt import absent from pinned Git tree")
    entries = list(imports.values())
    requested = "".join(entry["git_blob"] + "\n" for entry in entries).encode("ascii")
    completed = subprocess.run(["git", "-C", str(HERMES), "cat-file", "--batch"],
                               input=requested, capture_output=True, check=True, timeout=timeout,
                               env=git_environment())
    stream = io.BytesIO(completed.stdout)
    for entry in entries:
        blob = read_git_blob(stream, entry["git_blob"])
        if len(blob) != entry["bytes"] or sha(blob) != entry["sha256"]:
            raise ValueError("receipt import content hash/size mismatch")
    if stream.read(1):
        raise ValueError("unexpected extra Git batch output")


class BlobSource(importlib.abc.Loader):
    def __init__(self, finder, name):
        self.finder, self.name = finder, name

    def create_module(self, spec):
        return None

    def exec_module(self, module):
        self.finder.exec_path(module, self.name)


class BlobImports(importlib.abc.MetaPathFinder):
    """In-memory canonical Git export, not worktree-byte or CRLF imports."""
    def __init__(self):
        self.files = tree(HERMES, PINS["hermes"])
        self.imported = {}
        self.reader = GitBlobReader(HERMES, (oid for path, oid in self.files.items() if path.endswith(".py")))

    def path(self, fullname):
        base = fullname.replace(".", "/")
        for name in (base + "/__init__.py", base + ".py"):
            if name in self.files:
                return name
        return None

    def find_spec(self, fullname, path=None, target=None):
        name = self.path(fullname)
        if not name and path:
            # Plugin packages can have synthetic module names unrelated to their
            # Git directory. Resolve relative submodules from their package path.
            for directory in path:
                try:
                    parent = Path(directory).resolve().relative_to(HERMES.resolve()).as_posix()
                except ValueError:
                    continue
                leaf = fullname.rsplit(".", 1)[-1]
                for candidate in (f"{parent}/{leaf}/__init__.py", f"{parent}/{leaf}.py"):
                    if candidate in self.files:
                        name = candidate
                        break
                if name:
                    break
        if name:
            return importlib.util.spec_from_loader(fullname, BlobSource(self, name),
                                                  is_package=name.endswith("/__init__.py"))
        return None

    def exec_path(self, module, name):
        print(json.dumps({"event": "blob_read", "path": name, "time": time.monotonic()}),
              file=sys.stderr, flush=True)
        blob = self.reader.read(self.files[name])
        location = HERMES / name
        module.__file__ = str(location)
        if name.endswith("/__init__.py"):
            module.__path__ = [str(location.parent)]
        self.imported[name] = {"git_blob": self.files[name], "sha256": sha(blob), "bytes": len(blob)}
        print(json.dumps({"event": "module_exec", "path": name, "time": time.monotonic()}),
              file=sys.stderr, flush=True)
        exec(compile(blob, str(location), "exec"), module.__dict__)

    def install_file_imports(self):
        original = importlib.util.spec_from_file_location
        def pinned_spec(fullname, location, *args, **kwargs):
            try:
                name = Path(location).resolve().relative_to(HERMES.resolve()).as_posix()
            except (TypeError, ValueError):
                return original(fullname, location, *args, **kwargs)
            if name not in self.files or not name.endswith(".py"):
                raise RuntimeError("dynamic Hermes module absent from pinned Git tree")
            return importlib.util.spec_from_loader(fullname, BlobSource(self, name),
                                                  is_package=name.endswith("/__init__.py"))
        importlib.util.spec_from_file_location = pinned_spec


def install_safety(scratch, finder):
    def owned(path):
        try:
            return Path(path).resolve().is_relative_to(scratch)
        except (TypeError, ValueError):
            return False

    def audit(event, args):
        if event in ("socket.connect", "socket.bind", "socket.getaddrinfo", "os.system"):
            raise RuntimeError("offline probe forbids network/process side effects")
        if event == "subprocess.Popen":
            raise RuntimeError("probe forbids new subprocesses; pinned Git reader is already owned")
        if event == "open":
            path, mode, flags = args
            if isinstance(path, int) or str(path).lower() == os.devnull.lower():
                return
            writing = ((isinstance(mode, str) and any(c in mode for c in "wax+"))
                       or isinstance(flags, int) and flags & (os.O_WRONLY | os.O_RDWR | os.O_CREAT | os.O_TRUNC))
            if writing and not owned(path):
                raise RuntimeError("probe write outside disposable scratch denied")
            if not writing and Path(path).name in (".env", "auth.json", "credentials.json") and not owned(path):
                raise RuntimeError("probe forbids reading host credentials")
        if event in ("os.remove", "os.rmdir", "os.mkdir", "os.rename", "os.chmod"):
            paths = args[:2] if event == "os.rename" else args[:1]
            if any(not owned(p) for p in paths):
                raise RuntimeError("probe filesystem mutation outside scratch denied")
    sys.addaudithook(audit)


def child(scratch):
    scratch = Path(scratch).resolve()
    verify_inputs()
    # Windows stdlib may query `ver` once, before source execution is fenced.
    platform.uname()
    finder = BlobImports()
    print(json.dumps({"event": "git_reader_owned", "pid": finder.reader.process.pid}),
          file=sys.stderr, flush=True)
    faulthandler.dump_traceback_later(30, repeat=True)
    try:
        receipt = execute_child(scratch, finder)
    finally:
        faulthandler.cancel_dump_traceback_later()
        finder.reader.close()
    receipt["git_reader"] = {"pid": finder.reader.process.pid, "exit_code": finder.reader.process.returncode,
                             "requests": finder.reader.requests, "read_seconds": finder.reader.seconds}
    (scratch / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8", newline="\n")
    return 0 if receipt["test_status"] == "PASS" else 1


def execute_child(scratch, finder):
    os.chdir(scratch)
    sys.path.insert(0, str(ROOT / "qa"))
    install_safety(scratch, finder)
    sys.meta_path.insert(0, finder)
    finder.install_file_imports()
    loader = unittest.TestLoader()
    suites = [loader.discover(str(ROOT / "qa"), pattern="test_*.py"),
              unittest.TestLoader().discover(str(ROOT / "probes"), pattern="test_*.py")]
    suite = unittest.TestSuite(suites)
    def names(node):
        if isinstance(node, unittest.TestSuite):
            return [name for item in node for name in names(item)]
        return [node.id()]
    selectors = names(suite)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    hermes_imports = {
        name: getattr(module, "__file__", "") for name, module in sys.modules.copy().items()
        if str(getattr(module, "__file__", "")).startswith(str(HERMES))
    }
    # No fallback imports from worktree Python files may escape Git-blob provenance.
    unsealed = [name for name, path in hermes_imports.items()
                if Path(path).relative_to(HERMES).as_posix() not in finder.imported]
    import importlib.metadata
    versions = {}
    for package in ("PyYAML", "pydantic", "aiohttp", "httpx", "python-dotenv", "rich"):
        try:
            versions[package] = importlib.metadata.version(package)
        except importlib.metadata.PackageNotFoundError:
            versions[package] = None
    receipt = dict(tests_run=result.testsRun, failures=len(result.failures), errors=len(result.errors),
                   skips=len(result.skipped), selectors=selectors, git_imports=finder.imported,
                   unsealed_imports=unsealed, python=sys.version, host_dependencies=versions,
                   test_status="PASS" if result.wasSuccessful() and not result.skipped and not unsealed else "FAIL",
                   producer_admission="BLOCKED", live_evidence=False)
    expected_counts = EXPECTED_COUNTS
    actual_counts = {group: sum(s.rsplit(".", 1)[0] == group for s in selectors) for group in expected_counts}
    receipt["expected_counts"] = expected_counts
    receipt["actual_counts"] = actual_counts
    if actual_counts != expected_counts or sorted(selectors) != sorted(EXPECTED_SELECTORS):
        receipt["test_status"] = "FAIL"
    receipt["pins"] = dict(PINS)
    return receipt


def run():
    before = packet_inventory()
    report = {"pins": PINS, "packet_files": before, "status": "FAIL", "producer_admission": "BLOCKED"}
    evidence = ROOT / "qa/evidence" / ("run-" + uuid.uuid4().hex[:12])
    evidence.mkdir(parents=True)
    scratch_path = None
    timeout_stage = "preflight"
    try:
        verify_inputs()
        report["source_revision"] = git(FLEET, "rev-parse", "HEAD").decode().strip()
        report["fleet_blobs"] = {
            name: {"git_blob": git(FLEET, "rev-parse", f"{PINS['fleet']}:{name}").decode().strip(),
                   "sha256": sha(git(FLEET, "show", f"{PINS['fleet']}:{name}"))}
            for name in FLEET_PATHS
        }
        report["donor_status_before"] = {name: git(repo, "status", "--porcelain").decode()
                                         for name, repo in (("fleet", FLEET), ("hermes", HERMES))}
        with tempfile.TemporaryDirectory(prefix="probe-", dir=evidence) as temp:
            scratch_path = Path(temp)
            home = scratch_path / "hermes-home"
            home.mkdir()
            allowed = ("SystemRoot", "WINDIR", "PATH", "PATHEXT")
            env = {key: os.environ[key] for key in allowed if key in os.environ}
            env.update(HOME=temp, USERPROFILE=temp, APPDATA=temp, LOCALAPPDATA=temp,
                       TEMP=temp, TMP=temp, HERMES_HOME=str(home), PYTHONDONTWRITEBYTECODE="1",
                       PYTHONUTF8="1", HERMES_SKIP_UPDATE_CHECK="1", GIT_NO_LAZY_FETCH="1",
                       GIT_TERMINAL_PROMPT="0", GIT_OPTIONAL_LOCKS="0")
            deadline = time.monotonic() + 90
            timeout_stage = "child execution"
            completed = subprocess.run([sys.executable, "-B", "-X", "utf8", str(Path(__file__).resolve()),
                                        "--hermes-repo", str(HERMES), "--fleet-pin", PINS["fleet"],
                                        "--child", temp], env=env, capture_output=True, timeout=90)
            (evidence / "tests.log").write_bytes(completed.stdout + completed.stderr)
            receipt = scratch_path / "receipt.json"
            report["exit_code"] = completed.returncode
            timeout_stage = "receipt validation"
            execution = load_receipt(receipt)
            report["execution"] = execution
            validate_receipt(execution, completed.returncode, PINS)
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise RuntimeError("receipt validation exceeded original 90s budget")
            verify_import_provenance(execution["git_imports"], timeout=remaining)
            report["receipt_validated"] = True
        report["donor_status_after"] = {name: git(repo, "status", "--porcelain").decode()
                                        for name, repo in (("fleet", FLEET), ("hermes", HERMES))}
        if report["donor_status_before"] != report["donor_status_after"]:
            raise RuntimeError("donor status changed during probes")
        if before != packet_inventory():
            raise RuntimeError("packet changed during probes")
        report["status"] = "PASS_OFFLINE_ONLY"
    except subprocess.TimeoutExpired as exc:
        log = evidence / "tests.log"
        if not log.exists():
            log.write_bytes((exc.stdout or b"") + (exc.stderr or b""))
        report["error"] = "offline " + timeout_stage + " timeout; receipt not accepted"
    except Exception as exc:
        report["error"] = f"{type(exc).__name__}: {exc}"
    finally:
        report["scratch_absent"] = scratch_path is None or not scratch_path.exists()
        report["packet_seal_sha256"] = sha(json.dumps(before, sort_keys=True, separators=(",", ":")).encode())
        (evidence / "terminal-report.json").write_text(json.dumps(report, indent=2) + "\n",
                                                       encoding="utf-8", newline="\n")
    print(json.dumps({key: report.get(key) for key in ("status", "producer_admission", "packet_seal_sha256", "error")}))
    print(str(evidence / "terminal-report.json"))
    return 0 if report["status"] == "PASS_OFFLINE_ONLY" and report["scratch_absent"] else 1


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--hermes-repo", required=True, type=Path,
                        help="Existing clean Hermes checkout at the fixed bbaf7af pin; no fetch/install")
    parser.add_argument("--fleet-pin", default=PINS["fleet"],
                        help="Full immutable Fleet commit available here (default: reviewed ede1e41)")
    parser.add_argument("--child", help=argparse.SUPPRESS)
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.fleet_pin):
        parser.error("--fleet-pin must be a full lowercase immutable Git SHA")
    HERMES = args.hermes_repo.resolve()
    PINS["fleet"] = args.fleet_pin
    sys.exit(child(args.child) if args.child else run())
