"""Hosted full backend gate, derived from reviewed codegen controls and QA42."""
from __future__ import annotations

import argparse
from collections import Counter
from datetime import datetime, timezone
from email.utils import parsedate_to_datetime
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import selectors
import shutil
import signal
import stat
import subprocess
import tarfile
import tempfile
import time
import urllib.request
import zipfile

REPOSITORY = "FerrPOINT/fleet-control"
BRANCH = "build-only/c11-backend-20261010"
SOURCE_SHA = "0e494309d2f94e11598e6e7562c57de423d044e9"
SOURCE_TREE = "934f69f7f8d667a242531cf122917d8e74ca5cf9"
IGNORED_COUNT = 18
DEFAULT_COUNT = 248
FOUNDATION_IGNORED = 0
BASE_SHA = "19a7a381ae6dbea61a643bb96189e483fa64df5c"
AUTH_SHA = "01388dfb43332cbe5837fd5e1fadccf09cb8886d"
PACKAGE_SHA = "4b9b4c9297a13fb28a6ba2039af2f7cb719f2f58"
PACKAGE_TREE = "96d9a7453744fd09f9ee3ba3b2c20f6b3d389b85"
PACKAGE_INVENTORY_SHA = "1bdf56b21b0b97ec4a5a6303b04ecdda1b6609164aa018acc830187ca124f827"
# Exact committed PR64 schema input; authentic C11 regeneration/parity is mandatory in the gate.
OPENAPI_SHA = "f31be57484a012e91dac2dba67cb42385fde01cebcdc3f9d496eaaaa55663fba"
SWAGGER_SHA = "481244d0812097b11fbaeef79f71d942b171617f9c9f9514e63acbe13e71ccdc"
WORKFLOW = ".github/workflows/backend-build-only.yml"
HELPER = "scripts/hosted_backend_gate.py"
GATE = "scripts/hosted-backend/gate.sh"
INIT = "scripts/hosted-backend/init.sql"
INVENTORY = "scripts/hosted-backend/test-inventory.json"
WRITE_SET = {WORKFLOW, HELPER, GATE, INIT, INVENTORY, "scripts/tests/test_hosted_backend_gate.py"}
ARTIFACT_FILES = {"report.json", "provenance.json", "SHA256SUMS"}
FAILURE_FILE = "compiler-diagnostics.json"
SOURCE_INVENTORY_SHA = "2df727b43f468da1c0919fd6a004dd3836d6a548bbabeef807a608c8435de9c8"
DIAGNOSTIC_LIMIT = 32
DIAGNOSTIC_INPUT_LIMIT = 16 * 1024 ** 2
DIAGNOSTIC_LINE_LIMIT = 256 * 1024
FAILURE_SIZE_LIMIT = 16 * 1024
INVENTORY_INPUT_LIMIT = 1024 * 1024
INVENTORY_LINE_LIMIT = 4096
INVENTORY_RECORD_LIMIT = 4096
INVENTORY_SAMPLE_LIMIT = 8
CATEGORY_PATTERNS = {
    "network": ("failed to download", "could not resolve host", "timeout was reached", "network failure", "failed to fetch"),
    "dependency": ("failed to select a version", "no matching package named", "requires rustc", "failed to load source for dependency"),
    "build-script": ("failed to run custom build command",),
    "linker": ("linking with", "linker command failed",),
    "resource": ("no space left on device", "cannot allocate memory", "out of memory", "signal: 9, sigkill"),
}
TEST_CUSTOM_HINTS = {}
TEST_NULL_HINTS = {"null_definition_decode": "definition", "null_body_decode": "body"}
TEST_ACTIVATION_HINTS = frozenset()
ACTIVATION_PROBE_SOURCE = ""
FORBIDDEN = {".local", "target", "node_modules", ".venv", ".git", "backups", ".env"}
GATES = ("preflight","fmt","check","clippy","auth_binary","runtime_inventory","real_auth","api2","credentials_unit","credentials_pg","foundation","config_api","base_package_unit","config_files_unit","package_effective_unit","config_shared_unit","base_package_pg","config_revision_pg","workspace","lineage10","central_profile","message_order","chats_directory","runtime_approval_events","credentials_migration","migration_smoke","openapi","compiled_source_parity")
DATABASES = ("fleet_foundation_test", "fleet_migration_test", "fleet_message_order_test",
             "fleet_chats_directory_test", "fleet_migration_smoke", "fleet_runtime_approval_events_test",
             "fleet_credential_migration_test", "fleet_real_auth_test", "fleet_configuration_test")
URLS = {
    "FLEET_TEST_DATABASE_URL": "fleet_foundation_test",
    "FLEET_MIGRATION_TEST_DATABASE_URL": "fleet_migration_test",
    "FLEET_MESSAGE_ORDER_TEST_DATABASE_URL": "fleet_message_order_test",
    "FLEET_CHATS_DIRECTORY_TEST_DATABASE_URL": "fleet_chats_directory_test",
    "FLEET_RUNTIME_APPROVAL_EVENTS_TEST_DATABASE_URL": "fleet_runtime_approval_events_test",
    "FLEET_CREDENTIAL_MIGRATION_TEST_DATABASE_URL": "fleet_credential_migration_test",
    "DATABASE_URL": "fleet_migration_smoke", "FLEET_REAL_AUTH_TEST_DATABASE_URL": "fleet_real_auth_test",
    "FLEET_CONFIGURATION_TEST_DATABASE_URL": "fleet_configuration_test",
}
FLEET_ROOTS = ("backend", ".base-revision", "openapi", "docs/TESTING.md")
JOB_SECONDS = 7200
CLEANUP_SECONDS = 300
UPLOAD_SECONDS = 240
FALLBACK_SECONDS = 60  # Included in the final 240 seconds; three artifact/summary steps keep 180.
GATE_SECONDS = 6600
BUDGET = None
VERIFY_IN_GATE = False


class CommandFailed(ValueError):
    def __init__(self, returncode):
        super().__init__("Command failed; backend gate withheld")
        self.returncode = returncode


def failure_kind(error):
    category, code = "unexpected", None
    if isinstance(error, CommandFailed):
        category = "command"
        value = error.returncode
        code = value if type(value) is int and -255 <= value <= 255 else None
    elif isinstance(error, (TimeoutError, subprocess.TimeoutExpired)):
        category = "timeout"
    elif isinstance(error, tarfile.TarError):
        category = "archive"
    elif isinstance(error, OSError):
        category = "io"
    elif isinstance(error, ValueError):
        category = "validation"
    return dict(category=category, exit_code=code)


class JobBudget:
    def __init__(self, remaining):
        require(0 < remaining <= JOB_SECONDS, "Invalid hosted job budget")
        self.job_end = time.monotonic() + remaining
        self.work_end = self.job_end - CLEANUP_SECONDS - UPLOAD_SECONDS
        self.end = self.work_end
        self.expired = False

    def check(self):
        if self.expired or time.monotonic() >= self.end:
            self.expired = True
            raise TimeoutError("Hosted elapsed budget exhausted")

    def alarm(self, *_):
        self.expired = True
        raise TimeoutError("Hosted elapsed budget exhausted")

    def arm(self):
        self.check()
        signal.signal(signal.SIGALRM, self.alarm)
        signal.setitimer(signal.ITIMER_REAL, self.end - time.monotonic())

    def cleanup(self, *, fallback=False):
        # Only this transition may leave an expired work phase; it never renews the job.
        signal.setitimer(signal.ITIMER_REAL, 0)
        duration = FALLBACK_SECONDS - 15 if fallback else CLEANUP_SECONDS
        ceiling = self.job_end - UPLOAD_SECONDS + (FALLBACK_SECONDS if fallback else 0)
        self.end = min(time.monotonic() + duration, ceiling)
        self.expired = False
        self.arm()


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *_):
        raise ValueError("Hosted metadata redirect forbidden")


def metadata_remaining(payload, date_header, elapsed):
    require(type(payload.get("total_count")) is int and payload["total_count"] == 1
            and len(payload.get("jobs", [])) == 1, "Unexpected hosted job inventory")
    job = payload["jobs"][0]
    require(job["name"] == "backend" and job["status"] == "in_progress"
            and type(job["id"]) is int and job["id"] > 0
            and job["run_id"] == int(os.environ["GITHUB_RUN_ID"])
            and job["run_attempt"] == int(os.environ["GITHUB_RUN_ATTEMPT"])
            and job["head_sha"] == os.environ["GITHUB_SHA"]
            and job["runner_name"] == os.environ["RUNNER_NAME"], "Hosted job identity drift")
    started = datetime.strptime(job["started_at"], "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    now = parsedate_to_datetime(date_header)
    require(now.utcoffset() is not None and now >= started and elapsed >= 0, "Invalid hosted job clock")
    # Server Date includes checkout/container/setup time; request time and rounding are conservative.
    remaining = JOB_SECONDS - (now - started).total_seconds() - elapsed - 2
    require(0 < remaining <= JOB_SECONDS, "Hosted job elapsed budget exhausted")
    return remaining


def hosted_budget():
    hosted_identity()  # Fail before token access or network on local/foreign execution.
    token = os.environ.get("FLEET_GATE_READ_TOKEN", "")
    require(bool(token) and bool(os.environ.get("RUNNER_NAME")), "Hosted metadata credentials missing")
    url = ("https://api.github.com/repos/" + REPOSITORY + "/actions/runs/"
           + os.environ["GITHUB_RUN_ID"] + "/attempts/" + os.environ["GITHUB_RUN_ATTEMPT"]
           + "/jobs?per_page=100")
    request = urllib.request.Request(url, headers={"Authorization": "Bearer " + token,
        "Accept": "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28"})
    before = time.monotonic()
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    def timeout(*_):
        raise TimeoutError("Hosted metadata deadline exhausted")
    previous = signal.signal(signal.SIGALRM, timeout)
    signal.setitimer(signal.ITIMER_REAL, 15)
    try:
        with opener.open(request, timeout=15) as response:
            require(response.status == 200, "Hosted metadata unavailable")
            body = response.read(256 * 1024 + 1)
            require(len(body) <= 256 * 1024, "Hosted metadata oversized")
            remaining = metadata_remaining(json.loads(body), response.headers["Date"], time.monotonic() - before)
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous)
    return JobBudget(remaining)


def check_budget():
    if BUDGET is not None:
        BUDGET.check()


def bounded_timeout(maximum):
    check_budget()
    return maximum if BUDGET is None else min(maximum, BUDGET.end - time.monotonic())


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return (json.dumps(value, sort_keys=True, indent=2) + "\n").encode()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def command(args, **kwargs):
    if os.name != "posix" or VERIFY_IN_GATE:
        # verify-log already belongs to the held gate group; never let its children escape it.
        require(BUDGET is None, "Independent hosted budget requires owned Linux commands")
        result = subprocess.run(args, capture_output=True, timeout=300, **kwargs)
        if result.returncode != 0:
            raise CommandFailed(result.returncode)
        return result.stdout
    deadline = time.monotonic() + bounded_timeout(300)
    process = None
    output = bytearray()
    try:
        process = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                   start_new_session=True, **kwargs)
        with selectors.DefaultSelector() as selector:
            for pipe in (process.stdout, process.stderr):
                os.set_blocking(pipe.fileno(), False)
                selector.register(pipe, selectors.EVENT_READ)
            while True:
                check_budget()
                if process.returncode is None and leader_status(process) is not None:
                    stop_owned_group(process)
                if process.returncode is not None and not selector.get_map():
                    break
                require(time.monotonic() < deadline, "Owned command deadline exhausted")
                for key, _ in selector.select(min(0.05, max(0, deadline - time.monotonic()))):
                    block = os.read(key.fileobj.fileno(), 65536)
                    if not block:
                        selector.unregister(key.fileobj)
                    elif key.fileobj is process.stdout:
                        output.extend(block)
                        require(len(output) <= 256 * 1024 ** 2, "Owned command output oversized")
        check_budget()
        if process.returncode != 0:
            raise CommandFailed(process.returncode)
        return bytes(output)
    finally:
        try:
            stop_owned_group(process)
        finally:
            if process is not None:
                for pipe in (process.stdout, process.stderr):
                    if pipe is not None:
                        pipe.close()


def git(root, *args, **kwargs):
    return command(["git", "--no-replace-objects", "-C", str(root), *args], **kwargs)


def clean_head(root, expected):
    require(git(root, "rev-parse", "HEAD").decode().strip() == expected, "Checkout SHA mismatch")
    require(not git(root, "status", "--porcelain=v1", "--untracked-files=all"), "Dirty checkout")


def validate_delta(text):
    changes = [line.split("\t") for line in text.splitlines()]
    require({tuple(line) for line in changes} == {("A", name) for name in WRITE_SET}
            and len(changes) == len(WRITE_SET), "Build branch may add only the six reviewed controls")


def hosted_identity():
    expected = {"GITHUB_ACTIONS": "true", "RUNNER_ENVIRONMENT": "github-hosted",
                "GITHUB_REPOSITORY": REPOSITORY, "GITHUB_EVENT_NAME": "push",
                "GITHUB_REF": "refs/heads/" + BRANCH}
    require(os.name == "posix" and all(os.environ.get(key) == value for key, value in expected.items()),
            "Execution restricted to the exact dedicated hosted branch push")
    require(bool(re.fullmatch(r"[0-9a-f]{40}", os.environ.get("GITHUB_SHA", ""))), "Missing workflow SHA")
    require(all(re.fullmatch(r"[1-9][0-9]*", os.environ.get(key, "")) for key in
                ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT")), "Missing run/attempt")
    return Path(os.environ["GITHUB_WORKSPACE"]).resolve(), os.environ["GITHUB_SHA"]


def preflight():
    workspace, workflow_sha = hosted_identity()
    controls = workspace / "controls"
    clean_head(controls, workflow_sha)
    require_codegen_binding(reviewed_inventory(controls))
    git(controls, "merge-base", "--is-ancestor", SOURCE_SHA, workflow_sha)
    validate_delta(git(controls, "diff", "--no-renames", "--name-status", SOURCE_SHA, workflow_sha).decode())
    require(git(controls, "rev-parse", SOURCE_SHA + "^{tree}").decode().strip() == SOURCE_TREE, "Source tree drift")
    require((controls / ".base-revision").read_text().strip() == BASE_SHA, "Base pin drift")
    require(digest(git(controls, "show", SOURCE_SHA + ":openapi/openapi.json")) == OPENAPI_SHA, "Schema pin drift")
    return workspace, controls, workflow_sha


def safe_member(name, roots, *, directory=False):
    path = PurePosixPath(name)
    require(bool(name) and not path.is_absolute() and ".." not in path.parts and "\\" not in name
            and not re.search(r"[\x00-\x20\x7f:]", name) and str(path) == name
            and not FORBIDDEN.intersection(part.lower() for part in path.parts)
            and any(name == root or name.startswith(root + "/")
                    or (directory and root.startswith(name + "/")) for root in roots), "Unsafe export path")
    return path


def export(root, commit, destination, roots):
    # archive applies eol/ident/export-subst attributes; compilation needs exact pinned blob bytes.
    listing = git(root, "ls-tree", "-r", "-z", "--full-tree", commit, "--", *roots)
    require(listing.endswith(b"\0"), "Missing source tree records")
    entries = []
    for record in listing[:-1].split(b"\0"):
        header, separator, name = record.partition(b"\t")
        fields = header.split(b" ")
        require(separator and len(fields) == 3 and fields[0] in (b"100644", b"100755")
                and fields[1] == b"blob" and re.fullmatch(b"[0-9a-f]{40}", fields[2]),
                "Source links/submodules or malformed tree forbidden")
        path = safe_member(name.decode("utf-8"), roots)
        entries.append((path, fields[0], fields[2]))
    require(len({str(path) for path, _, _ in entries}) == len(entries), "Duplicate source tree path")
    require(all(any(str(path) == allowed or str(path).startswith(allowed + "/") for path, _, _ in entries)
                for allowed in roots), "Missing required source root")
    with tempfile.TemporaryFile() as requests:
        requests.write(b"".join(oid + b"\n" for _, _, oid in entries))
        requests.seek(0)
        payload = io.BytesIO(git(root, "cat-file", "--batch", stdin=requests))
    destination.mkdir(parents=True, exist_ok=False)
    for path, mode, oid in entries:
        header = payload.readline(128)
        fields = header.rstrip(b"\n").split(b" ")
        require(header.endswith(b"\n") and len(fields) == 3 and fields[:2] == [oid, b"blob"]
                and re.fullmatch(b"(?:0|[1-9][0-9]{0,8})", fields[2]), "Malformed source blob frame")
        size = int(fields[2])
        require(size <= 128 * 1024 ** 2, "Oversized source member")
        body = payload.read(size)
        require(len(body) == size and payload.read(1) == b"\n", "Truncated source blob frame")
        target = destination.joinpath(*path.parts)
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open("xb") as output:
            output.write(body)
        target.chmod(0o755 if mode == b"100755" else 0o644)
    require(not payload.read(1), "Unexpected source blob frames")


def inventory(root):
    require(not any(path.is_symlink() for path in root.rglob("*")), "Unexpected source link")
    return {path.relative_to(root).as_posix(): digest(path.read_bytes())
            for path in sorted(root.rglob("*")) if path.is_file()}


def reviewed_inventory(controls):
    value = json.loads((controls / INVENTORY).read_bytes())
    require(value["source_commit"] == SOURCE_SHA and value["source_tree"] == SOURCE_TREE
            and value["executed"] is False, "Inventory pin drift")
    require(len(value["ignored"]) == IGNORED_COUNT and value["default_foundation_ignored"] == FOUNDATION_IGNORED
            and len(value["workspace_default_declarations"]) == DEFAULT_COUNT, "Declaration coverage drift")
    require(len({(item["package"], item["target_kind"], item["target"], item["name"])
                 for item in value["ignored"]}) == IGNORED_COUNT, "Duplicate ignored identities")
    require({name: len(names) for name, names in value["groups"].items()} == dict(
        api2=2, credentials_unit=8, credentials_pg=15, foundation=61, config_api=8,
        base_package_unit=11, config_files_unit=4, package_effective_unit=2, config_shared_unit=10,
        base_package_pg=8, config_revision_pg=3, lineage10=10, central_profile=3, message_order=1,
        chats_directory=1, runtime_approval_events=1, credentials_migration=1, real_auth=2),
        "Exact C11 group coverage drift")
    require(value["package_input"] == dict(commit=PACKAGE_SHA, tree=PACKAGE_TREE,
            inventory_sha256=PACKAGE_INVENTORY_SHA), "Package input drift")
    require(value["python_contracts"] == {}, "Unowned Python/native contracts forbidden")
    require(digest(canonical(value["compiled_source_sha256"])) == SOURCE_INVENTORY_SHA, "Input fingerprint drift")
    require(len(value["migration_registries"]["canonical"]) == 12
            and len(value["migration_registries"]["split"]) == 15, "Migration registry drift")
    for item in value["ignored"]:
        require(item["gate"] in GATES and value["groups"][item["gate"]].count(item["name"]) == 1,
                "Unmapped ignored identity")
    return value


def require_codegen_binding(reviewed):
    require(isinstance(OPENAPI_SHA, str) and bool(re.fullmatch(r"[0-9a-f]{64}", OPENAPI_SHA))
            and reviewed.get("openapi_binding") == dict(status="committed_input_parity_required", sha256=OPENAPI_SHA),
            "Exact committed OpenAPI/source binding is pending; prepared controls cannot execute")


def qualify_package(root, reviewed):
    require(not root.is_symlink(), "Package checkout link forbidden")
    clean_head(root, PACKAGE_SHA)
    require(git(root, "config", "--get", "remote.origin.url").decode().strip() in (
        "https://github.com/FerrPOINT/services-base.git", "git@github.com:FerrPOINT/services-base.git"),
        "Canonical package origin required")
    tree = git(root, "rev-parse", "HEAD^{tree}").decode().strip()
    metadata = git(root, "ls-tree", "-r", "-l", "-z", PACKAGE_SHA, "--",
                   "agent-skills/manifest.json", "agent-skills/roles", "agent-skills/skills")
    proof = dict(commit=PACKAGE_SHA, tree=tree, inventory_sha256=digest(metadata))
    require(metadata.endswith(b"\0") and metadata.count(b"\0") == 22
            and proof == reviewed["package_input"], "Package Git metadata drift")
    git(root, "cat-file", "-e", PACKAGE_SHA + ":agent-skills/manifest.json")
    return proof


def expected_migration_receipt(reviewed):
    full = sorted(reviewed["migration_registries"]["canonical"])
    previous = [name for name in full if name != "m20261004_000011_pm_credentials"]
    require(len(full) == 12 and len(previous) == 11
            and full[-1] == "m20261004_000011_pm_credentials", "Credential migration boundary drift")
    return dict(up=full, down_one=previous, reapply=full, down_all=[], final_up=full)


def verify_migration_snapshots(root, reviewed):
    expected = expected_migration_receipt(reviewed)
    actual = {key: (root / ("migration-" + key + ".txt")).read_text().splitlines() for key in expected}
    require(actual == expected, "Migration exact ledger snapshots differ")
    ledgers = {}
    for key, names in expected.items():
        rows = [line.split("\t") for line in (root / ("migration-" + key + "-ledger.tsv")).read_text().splitlines()]
        require(all(len(row) == 2 and re.fullmatch(r"(?:0|[1-9][0-9]*)", row[1]) for row in rows)
                and [row[0] for row in rows] == names, "Migration applied-at ledger shape drift")
        ledgers[key] = dict(rows)
    for before, after in (("up", "down_one"), ("down_one", "reapply")):
        common = set(ledgers[before]) & set(ledgers[after])
        require(all(ledgers[before][name] == ledgers[after][name] for name in common),
                "Surviving migration applied-at history changed")
    return dict(snapshots=actual, applied_at_preserved=True)


def validate_source_inventory(source, reviewed):
    actual = {path.relative_to(source).as_posix(): digest(path.read_bytes())
              for path in sorted((source / "backend").rglob("*.rs"))}
    require(actual == reviewed["rust_source_sha256"], "Unreviewed Rust declaration/source drift")


def diagnostic_file(name, allowed, fleet_backend, target=None):
    if not isinstance(name, str) or len(name) > 4096:
        return None
    path = PurePosixPath(name)
    if ".." in path.parts or "\\" in name or str(path) != name or re.search(r"[\x00-\x1f\x7f:]", name):
        return None
    prefix = str(fleet_backend).rstrip("/") + "/"
    if path.is_absolute():
        if not name.startswith(prefix):
            return None
        name = "backend/" + name[len(prefix):]
    elif not name.startswith("backend/"):
        name = "backend/" + name
    if name in allowed:
        return name
    if not path.is_absolute() and isinstance(target, dict):
        entry = diagnostic_file(target.get("src_path"), allowed, fleet_backend)
        if entry is not None:
            candidate = "/".join(entry.split("/")[:2]) + "/" + str(path)
            return candidate if candidate in allowed else None
    return None


def safe_compiler_diagnostics(streams, allowed, fleet_backend):
    # Raw JSON, rendered diagnostics, snippets and non-JSON text never leave this function.
    diagnostics, seen, categories, truncated, remaining = [], set(), set(), False, DIAGNOSTIC_INPUT_LIMIT
    for stream in streams:
        while remaining > 0:
            line = stream.readline(min(DIAGNOSTIC_LINE_LIMIT + 1, remaining + 1))
            if not line:
                break
            remaining -= len(line)
            if len(line) > DIAGNOSTIC_LINE_LIMIT or remaining < 0:
                truncated = True
                while not line.endswith(b"\n") and remaining > 0:
                    line = stream.readline(min(DIAGNOSTIC_LINE_LIMIT + 1, remaining + 1))
                    if not line:
                        break
                    remaining -= len(line)
                continue
            try:
                value = json.loads(line)
            except (ValueError, UnicodeError, RecursionError):
                text = line.decode("utf-8", errors="replace").lower()
                categories.update(category for category, patterns in CATEGORY_PATTERNS.items()
                                  if any(pattern in text for pattern in patterns))
                continue
            if not isinstance(value, dict) or value.get("reason") != "compiler-message":
                continue
            message = value.get("message")
            if not isinstance(message, dict) or message.get("level") != "error":
                continue
            code = message.get("code")
            code = code.get("code") if isinstance(code, dict) else None
            code = code if isinstance(code, str) and re.fullmatch(r"E[0-9]{4}", code) else None
            spans = message.get("spans")
            if not isinstance(spans, list):
                continue
            for span in spans:
                if not isinstance(span, dict) or span.get("is_primary") is not True:
                    continue
                file = diagnostic_file(span.get("file_name"), allowed, fleet_backend, value.get("target"))
                line, column = span.get("line_start"), span.get("column_start")
                if file is None or type(line) is not int or type(column) is not int or not (1 <= line <= 1000000 and 1 <= column <= 10000):
                    continue
                key = (code, file, line, column)
                if key in seen:
                    continue
                if len(diagnostics) >= DIAGNOSTIC_LIMIT:
                    truncated = True
                    continue
                seen.add(key)
                diagnostics.append(dict(error_code=code, file=file, line=line, column=column))
        if remaining <= 0:
            truncated = True
            break
    if truncated or not diagnostics and not categories:
        categories.add("unknown")
    return dict(diagnostics=diagnostics, categories=sorted(categories), truncated=truncated)


def compiler_failure_logs(root, stage, reviewed):
    result = dict(diagnostics=[], categories=["unknown"], truncated=False, command_exit_code=None)
    try:
        exit_file = root / "private" / (stage + ".exit")
        with exit_file.open("rb") as value:
            data = value.read(5)
        if re.fullmatch(rb"(?:0|[1-9][0-9]{0,2})\n", data) and int(data) <= 255:
            result["command_exit_code"] = int(data)
        with (root / "private" / (stage + ".jsonl")).open("rb") as out, (root / "private" / (stage + ".stderr")).open("rb") as err:
            result.update(safe_compiler_diagnostics((out, err), reviewed["rust_source_sha256"], root / "src/fleet-control/backend"))
    except Exception:
        # Parser/IO failure is not permission to echo raw diagnostics or skip cleanup.
        pass
    return result


def failure_test_names(reviewed, stage):
    if stage == "workspace":
        return {item["name"] for item in reviewed["workspace_default_declarations"]}
    return set(reviewed["groups"].get(stage, ()))


def safe_test_diagnostics(stream, names, allowed, fleet_backend):
    # Panic detail yields only fixed hints, never raw messages, SQL or values.
    diagnostics, seen, failed, hints, truncated = [], set(), set(), set(), False
    panic_detail = False
    activation_detail = False
    remaining = DIAGNOSTIC_INPUT_LIMIT
    while remaining > 0:
        line = stream.readline(min(DIAGNOSTIC_LINE_LIMIT + 1, remaining + 1))
        if not line:
            break
        remaining -= len(line)
        if len(line) > DIAGNOSTIC_LINE_LIMIT or remaining < 0:
            truncated = True
            panic_detail = False
            activation_detail = False
            while not line.endswith(b"\n") and remaining > 0:
                line = stream.readline(min(DIAGNOSTIC_LINE_LIMIT + 1, remaining + 1))
                if not line:
                    break
                remaining -= len(line)
            continue
        text = line.decode("utf-8", errors="replace").rstrip("\r\n")
        if text.startswith(("thread '", "test ")):
            panic_detail = False
            activation_detail = False
        if panic_detail:
            if activation_detail and text in TEST_ACTIVATION_HINTS:
                hints.add(text)
            hints.update(category for category, message in TEST_CUSTOM_HINTS.items()
                         if re.search(r'(?<![A-Za-z0-9_])Custom\("' + re.escape(message) + r'"\)', text))
            for category, field in TEST_NULL_HINTS.items():
                if ('ColumnDecode { index: "\\"' + field + '\\"", source: UnexpectedNullError }' in text
                        or 'Type("A null value was encountered while decoding \\"' + field + '\\"")' in text):
                    hints.add(category)
        panic_detail = False
        activation_detail = False
        result = re.fullmatch(r"test ([^\r\n ]+) \.\.\. FAILED", text)
        if result and result[1] in names:
            if len(failed) < DIAGNOSTIC_LIMIT:
                failed.add(result[1])
            else:
                truncated = True
        panic = re.fullmatch(r"thread '[^'\r\n]{1,256}' panicked at ([^:\r\n]{1,4096}):([0-9]{1,7}):([0-9]{1,5}):?", text)
        if not panic:
            continue
        file = diagnostic_file(panic[1], allowed, fleet_backend)
        line_number, column = int(panic[2]), int(panic[3])
        if file is None or not (1 <= line_number <= 1000000 and 1 <= column <= 10000):
            continue
        panic_detail = True
        key = (file, line_number, column)
        thread = re.match(r"thread '([^']+)'", text)[1]
        activation_detail = (file == ACTIVATION_PROBE_SOURCE and thread in names
                             and (key in seen or len(diagnostics) < DIAGNOSTIC_LIMIT))
        if key in seen:
            continue
        if len(diagnostics) >= DIAGNOSTIC_LIMIT:
            truncated = True
            continue
        seen.add(key)
        diagnostics.append(dict(error_code=None, file=file, line=line_number, column=column))
    if remaining <= 0:
        truncated = True
    return dict(diagnostics=diagnostics, failed_tests=sorted(failed),
                categories=sorted(hints | {"test_failure" if diagnostics or failed else "unknown"}), truncated=truncated)


def test_failure_logs(root, stage, reviewed):
    result = dict(diagnostics=[], failed_tests=[], categories=["unknown"], truncated=False, command_exit_code=None)
    try:
        with (root / "private" / (stage + ".log")).open("rb") as stream:
            result.update(safe_test_diagnostics(stream, failure_test_names(reviewed, stage),
                                               reviewed["rust_source_sha256"], root / "src/fleet-control/backend"))
    except Exception:
        pass
    return result


def inventory_listing(path):
    """Read only bounded regular owned logs; never return non-test lines."""
    names, remaining, lines = Counter(), INVENTORY_INPUT_LIMIT, 0
    try:
        require(not path.is_symlink(), "Inventory log link forbidden")
        fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0))
        with os.fdopen(fd, "rb") as stream:
            require(stat.S_ISREG(os.fstat(stream.fileno()).st_mode), "Inventory log is not regular")
            while remaining > 0 and lines < INVENTORY_RECORD_LIMIT:
                line = stream.readline(min(INVENTORY_LINE_LIMIT + 1, remaining + 1))
                if not line:
                    return names, "complete"
                remaining -= len(line)
                lines += 1
                if len(line) > INVENTORY_LINE_LIMIT or remaining < 0:
                    return names, "limited"
                text = line.decode("utf-8").rstrip("\r\n")
                match = re.fullmatch(r"([^\r\n]+): test", text)
                if match:
                    names[match[1]] += 1
            return names, "limited"
    except Exception:
        # Missing files, symlinks, FIFOs, decoding and IO failures carry no private text.
        return Counter(), "unavailable"


def inventory_failure_logs(root, reviewed):
    ordinary, ordinary_status = inventory_listing(root / "private/workspace-list.log")
    ignored, ignored_status = inventory_listing(root / "private/ignored-list.log")
    expected = dict(default=Counter(item["name"] for item in reviewed["workspace_default_declarations"]),
                    ignored=Counter(item["name"] for item in reviewed["ignored"]))
    defaults = ordinary - expected["ignored"]
    actual = dict(default=Counter(name.rsplit("::", 1)[-1] for name in defaults.elements()), ignored=ignored)
    differences, samples = {}, []
    eligible_samples = 0
    for scope in ("default", "ignored"):
        missing, extra = expected[scope] - actual[scope], actual[scope] - expected[scope]
        differences[scope] = dict(missing=sum(missing.values()), extra=sum(extra.values()),
            unallowlisted_extra=sum(count for name, count in extra.items() if name not in expected[scope]))
        for difference, values in (("missing", missing), ("extra", extra)):
            for name, count in sorted(values.items()):
                if name in expected[scope]:
                    eligible_samples += 1
                    if len(samples) < INVENTORY_SAMPLE_LIMIT:
                        samples.append(dict(scope=scope, difference=difference, name=name, count=count))
    statuses = dict(ordinary=ordinary_status, ignored=ignored_status)
    reason = ("listing_unavailable" if "unavailable" in statuses.values() else
              "listing_limit" if "limited" in statuses.values() else
              "listing_mismatch" if any(x["missing"] or x["extra"] for x in differences.values()) else "listing_matches")
    inventory = dict(reason=reason, input_status=statuses,
        expected=dict(default=DEFAULT_COUNT, ignored=IGNORED_COUNT),
        observed=dict(ordinary=sum(ordinary.values()), ignored=sum(ignored.values()), default=sum(defaults.values())),
        differences=differences, samples=samples, samples_truncated=eligible_samples > len(samples))
    return dict(diagnostics=[], command_exit_code=None,
                categories=["inventory_mismatch" if reason == "listing_mismatch" else "inventory_unavailable"],
                truncated="limited" in statuses.values(), inventory=inventory)


def validate_inventory_failure(value, reviewed):
    require(isinstance(value, dict) and set(value) == {
        "reason", "input_status", "expected", "observed", "differences", "samples", "samples_truncated"},
        "Unsafe inventory evidence fields")
    require(isinstance(value["input_status"], dict) and set(value["input_status"]) == {"ordinary", "ignored"}
            and all(x in ("complete", "limited", "unavailable") for x in value["input_status"].values()),
            "Invalid inventory input status")
    require(value["expected"] == dict(default=DEFAULT_COUNT, ignored=IGNORED_COUNT)
            and all(type(x) is int for x in value["expected"].values()), "Inventory expectation drift")
    observed, differences = value["observed"], value["differences"]
    require(isinstance(observed, dict) and set(observed) == {"ordinary", "ignored", "default"}
            and all(type(x) is int and 0 <= x <= INVENTORY_RECORD_LIMIT for x in observed.values())
            and 0 <= observed["ordinary"] - observed["default"] <= IGNORED_COUNT
            and (value["input_status"]["ordinary"] != "unavailable" or observed["ordinary"] == 0)
            and (value["input_status"]["ignored"] != "unavailable" or observed["ignored"] == 0), "Invalid inventory counts")
    require(isinstance(differences, dict) and set(differences) == {"default", "ignored"}, "Invalid inventory scopes")
    allowed = dict(default=Counter(item["name"] for item in reviewed["workspace_default_declarations"]),
                   ignored=Counter(item["name"] for item in reviewed["ignored"]))
    for scope, row in differences.items():
        require(isinstance(row, dict) and set(row) == {"missing", "extra", "unallowlisted_extra"}
                and all(type(x) is int and 0 <= x <= INVENTORY_RECORD_LIMIT for x in row.values())
                and row["unallowlisted_extra"] <= row["extra"]
                and row["missing"] <= value["expected"][scope]
                and value["expected"][scope] - row["missing"] + row["extra"] == observed[scope],
                "Inconsistent inventory differences")
    statuses = value["input_status"].values()
    reason = ("listing_unavailable" if "unavailable" in statuses else "listing_limit" if "limited" in statuses else
              "listing_mismatch" if any(x["missing"] or x["extra"] for x in differences.values()) else "listing_matches")
    require(value["reason"] == reason, "Invalid inventory reason")
    samples = value["samples"]
    require(isinstance(samples, list) and len(samples) <= INVENTORY_SAMPLE_LIMIT
            and type(value["samples_truncated"]) is bool, "Inventory sample count bound")
    seen, totals = set(), Counter()
    for sample in samples:
        require(isinstance(sample, dict) and set(sample) == {"scope", "difference", "name", "count"}, "Unsafe inventory sample fields")
        scope, difference, name = sample["scope"], sample["difference"], sample["name"]
        require(isinstance(scope, str) and scope in allowed and difference in ("missing", "extra")
                and isinstance(name, str) and name in allowed[scope], "Non-allowlisted inventory identity")
        require(type(sample["count"]) is int and 1 <= sample["count"] <= INVENTORY_RECORD_LIMIT
                and (difference != "missing" or sample["count"] <= allowed[scope][name]), "Invalid inventory sample count")
        key = (scope, difference, name)
        require(key not in seen and (scope, "extra" if difference == "missing" else "missing", name) not in seen,
                "Duplicate/conflicting inventory sample")
        seen.add(key)
        totals[(scope, difference)] += sample["count"]
    omitted = False
    for scope, row in differences.items():
        for difference in ("missing", "extra"):
            safe_count = row[difference] - (row["unallowlisted_extra"] if difference == "extra" else 0)
            require(totals[(scope, difference)] <= safe_count
                    and (value["samples_truncated"] or totals[(scope, difference)] == safe_count), "Inventory sample totals drift")
            omitted |= totals[(scope, difference)] < safe_count
    require(value["samples_truncated"] == omitted
            and (not omitted or len(samples) == INVENTORY_SAMPLE_LIMIT), "Invalid inventory sample truncation")


def validate_failure_evidence(value, *, workflow_sha, run_id, attempt):
    controls = Path(__file__).resolve().parents[1]
    require(isinstance(value, dict) and value.get("kind") in ("safe_compiler_failure", "safe_test_failure", "safe_inventory_failure"), "Invalid failure kind")
    test_failure = value["kind"] == "safe_test_failure"
    inventory_failure = value["kind"] == "safe_inventory_failure"
    expected = dict(version=1, kind=value["kind"], status="failure", repository=REPOSITORY,
                    branch=BRANCH, workflow_sha=workflow_sha, workflow_path=WORKFLOW,
                    source_sha=SOURCE_SHA, base_sha=BASE_SHA, auth_sha=AUTH_SHA, run_id=run_id, run_attempt=attempt,
                    source_inventory_sha256=SOURCE_INVENTORY_SHA, backend_quality_gate=False,
                    all_quality_gate=False, sdlc_acceptance=False,
                    package_sha=PACKAGE_SHA, package_tree=PACKAGE_TREE, package_inventory_sha256=PACKAGE_INVENTORY_SHA,
                    control_sha256={name: digest((controls / name).read_bytes()) for name in sorted(WRITE_SET)})
    extra = {"stage", "gate_failed_stage", "gate_exit_code", "command_exit_code", "diagnostics", "categories", "truncated", "cleanup"}
    if test_failure:
        extra.add("failed_tests")
    if inventory_failure:
        extra.add("inventory")
    require(isinstance(value, dict) and set(value) == set(expected) | extra, "Unsafe failure evidence fields")
    require(all(type(value[key]) is type(item) and value[key] == item for key, item in expected.items()), "Failure provenance mismatch")
    reviewed = reviewed_inventory(controls)
    require(isinstance(value["stage"], str) and value["stage"] in GATES, "Invalid failure stage")
    require((value["stage"] == "runtime_inventory" if inventory_failure else
             bool(failure_test_names(reviewed, value["stage"])) if test_failure else value["stage"] in ("check", "clippy"))
            and value["gate_failed_stage"] in (value["stage"], "cleanup"), "Invalid failure stage")
    if test_failure:
        names = value["failed_tests"]
        allowed_names = failure_test_names(reviewed, value["stage"])
        require(isinstance(names, list) and len(names) <= DIAGNOSTIC_LIMIT
                and all(isinstance(name, str) and name in allowed_names for name in names)
                and names == sorted(set(names)), "Unsafe failed test identities")
    require(all(value[key] is None or type(value[key]) is int and 0 <= value[key] <= 255
                for key in ("gate_exit_code", "command_exit_code")), "Invalid exit code")
    require(type(value["truncated"]) is bool and isinstance(value["categories"], list)
            and all(isinstance(item, str) and item in ({"inventory_mismatch", "inventory_unavailable"} if inventory_failure else
                                                      {"test_failure", "unknown", *TEST_CUSTOM_HINTS, *TEST_NULL_HINTS, *TEST_ACTIVATION_HINTS}
                                                       if test_failure else {*CATEGORY_PATTERNS, "unknown"}) for item in value["categories"])
            and value["categories"] == sorted(set(value["categories"])), "Invalid fixed failure categories")
    if inventory_failure:
        validate_inventory_failure(value["inventory"], reviewed)
        require(value["diagnostics"] == [] and value["command_exit_code"] is None
                and value["categories"] == ["inventory_mismatch" if value["inventory"]["reason"] == "listing_mismatch" else "inventory_unavailable"]
                and value["truncated"] == ("limited" in value["inventory"]["input_status"].values()),
                "Invalid inventory diagnostic framing")
    if test_failure and set(value["categories"]) & (TEST_CUSTOM_HINTS.keys() | TEST_NULL_HINTS.keys() | TEST_ACTIVATION_HINTS):
        require("test_failure" in value["categories"] and bool(value["diagnostics"]), "Unanchored test failure hint")
    if test_failure and set(value["categories"]) & TEST_ACTIVATION_HINTS:
        require(isinstance(value["diagnostics"], list)
                and any(isinstance(record, dict) and record.get("file") == ACTIVATION_PROBE_SOURCE
                    for record in value["diagnostics"]), "Unanchored activation probe hint")
    records = value["diagnostics"]
    allowed = reviewed["rust_source_sha256"]
    require(isinstance(records, list) and len(records) <= DIAGNOSTIC_LIMIT, "Diagnostic count bound")
    seen = set()
    for record in records:
        require(isinstance(record, dict) and set(record) == {"error_code", "file", "line", "column"}, "Unsafe diagnostic fields")
        code, file = record["error_code"], record["file"]
        require(code is None or isinstance(code, str) and re.fullmatch(r"E[0-9]{4}", code), "Unsafe diagnostic code")
        require(not test_failure or code is None, "Test failure cannot claim compiler diagnostics")
        require(isinstance(file, str) and file in allowed, "Non-allowlisted Fleet diagnostic file")
        require(type(record["line"]) is int and 1 <= record["line"] <= 1000000
                and type(record["column"]) is int and 1 <= record["column"] <= 10000, "Invalid diagnostic location")
        seen.add((code, file, record["line"], record["column"]))
    require(len(seen) == len(records) and (records or value["categories"]), "Empty/duplicate diagnostic evidence")
    cleanup = value["cleanup"]
    require(isinstance(cleanup, dict) and set(cleanup) == {"scratch", "synthetic_databases"}
            and type(cleanup["scratch"]) is bool
            and (type(cleanup["synthetic_databases"]) is bool or cleanup["synthetic_databases"] == "not_initialized"), "Unsafe cleanup fields")
    require(len(canonical(value)) <= FAILURE_SIZE_LIMIT, "Failure evidence size bound")
    return value


def listed_names(text):
    return Counter(re.findall(r"^(.+): test$", text, re.M))


def verify_test_log(stage, text, reviewed, ordinary_listing=""):
    require(not re.search(r"not configured[^\n]*skipp|PostgreSQL tests skipped", text, re.I), "Skipped PG fixture")
    require(not re.search(r"^test result: FAILED|^test .+ \.\.\. FAILED", text, re.M), "Failed test result")
    actual = Counter(re.findall(r"^test (.+) \.\.\. ok$", text, re.M))
    summaries = [tuple(map(int, row)) for row in re.findall(
        r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", text, re.M)]
    if stage == "workspace":
        expected = listed_names(ordinary_listing) - Counter(item["name"] for item in reviewed["ignored"])
        require(bool(expected) and actual == expected, "Workspace default execution differs from compiler listing")
        require(bool(summaries) and sum(row[0] for row in summaries) == sum(actual.values())
                and all(row[1] == 0 for row in summaries)
                and sum(row[2] for row in summaries) == IGNORED_COUNT, "Workspace result/ignored totals drift")
    else:
        expected = Counter(reviewed["groups"][stage])
        ignored = FOUNDATION_IGNORED if stage == "foundation" else 0
        require(bool(expected) and actual == expected, "Focused exact test names missing/duplicate/unexpected")
        require(summaries == [(sum(expected.values()), 0, ignored)], "Focused result count/ignored drift")
    return dict(passed=sum(actual.values()), failed=0,
                ignored=sum(row[2] for row in summaries), tests=sorted(actual.elements()))


def verify_runtime_inventory(ordinary, ignored, reviewed):
    expected = Counter(item["name"] for item in reviewed["ignored"])
    require(listed_names(ignored) == expected and sum(expected.values()) == IGNORED_COUNT, "Compiler ignored inventory drift")
    defaults = listed_names(ordinary) - expected
    require(sum(defaults.values()) == DEFAULT_COUNT
            and Counter(name.rsplit("::", 1)[-1] for name in defaults.elements())
                == Counter(item["name"] for item in reviewed["workspace_default_declarations"]),
            "Compiler default declaration inventory drift")
    return dict(ignored=IGNORED_COUNT, ignored_names_sha256=digest(canonical(sorted(expected.elements()))),
                listed_default_count=sum((listed_names(ordinary) - expected).values()))


def resource_guard(root):
    mem = {line.split(":", 1)[0]: int(line.split()[1]) * 1024
           for line in Path("/proc/meminfo").read_text().splitlines()
           if line.startswith(("MemAvailable:", "CommitLimit:", "Committed_AS:"))}
    maximum = Path("/sys/fs/cgroup/memory.max").read_text().strip()
    current = int(Path("/sys/fs/cgroup/memory.current").read_text())
    free = shutil.disk_usage(root).free
    require(maximum.isdigit() and int(maximum) == 4 * 1024 ** 3, "Hosted compiler cgroup must be bounded at 4 GiB")
    require(int(maximum) - current >= 3 * 1024 ** 3, "Hosted initial cgroup headroom below 3 GiB")
    require(free >= 5 * 1024 ** 3, "Disposable hosted CI requires 5 GiB free; no waiver")
    return dict(mem, cgroup_limit_bytes=int(maximum), cgroup_current_bytes=current, disk_free_bytes=free,
                hosted_initial_cgroup_headroom_guard_bytes=3 * 1024 ** 3,
                hosted_disk_guard_bytes=5 * 1024 ** 3, native_guards_unchanged=True)


def database_environment():
    return {key: f"postgres://{'fleet_approval_events_test' if key == 'FLEET_RUNTIME_APPROVAL_EVENTS_TEST_DATABASE_URL' else 'fleet_test'}@postgres:5432/{name}"
            for key, name in URLS.items()}


def psql(sql=None, file=None, database="postgres"):
    require(database in (*DATABASES, "postgres"), "Unowned PostgreSQL target")
    args = ["psql", "-X", "-w", "-h", "postgres", "-U", "fleet_test", "-d", database,
            "-v", "ON_ERROR_STOP=1", "-A", "-t"]
    return command(args + (["-f", str(file)] if file else ["-c", sql]), env={"PATH": os.environ["PATH"], "LC_ALL": "C"})


def database_catalog():
    return sorted(psql("SELECT datname FROM pg_database WHERE NOT datistemplate ORDER BY datname").decode().splitlines())


def drop_databases(names):
    require(set(names).issubset(DATABASES), "Refusing foreign database cleanup")
    for name in names:
        psql(f'DROP DATABASE IF EXISTS "{name}" WITH (FORCE)')
    psql("DROP ROLE IF EXISTS fleet_approval_events_test")
    require(not set(DATABASES).intersection(database_catalog()), "Owned synthetic DB remains")


def leader_status(process):
    require(process.returncode is None, "Owned leader already reaped")
    return os.waitid(os.P_PID, process.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)


def live_group(process):
    for path in Path("/proc").glob("[0-9]*/stat"):
        try:
            fields = path.read_text().rsplit(") ", 1)[1].split()
        except FileNotFoundError:
            continue
        if int(fields[2]) == process.pid:
            require(int(fields[3]) == process.pid, "Owned process session drift")
            if fields[0] not in ("Z", "X"):
                return True
    return False


def stop_owned_group(process):
    if process is None:
        return
    if getattr(process, "_fleet_group_drained", False):
        if process.returncode is None:
            process.wait(timeout=10)
        return
    # Keep even an exited leader unreaped until descendants have stopped. No historical PID signals.
    for sig in (signal.SIGTERM, signal.SIGKILL):
        leader_status(process)
        require(os.getpgid(process.pid) == process.pid and os.getsid(process.pid) == process.pid,
                "Owned process custody lost")
        os.killpg(process.pid, sig)
    deadline = time.monotonic() + 10
    while live_group(process):
        require(time.monotonic() < deadline, "Owned group drain timed out")
        time.sleep(0.01)
    # Record drain before reap, including interruption between wait() and its caller's next statement.
    process._fleet_group_drained = True
    process.wait(timeout=10)


def wait_owned_exit(process, timeout):
    deadline = time.monotonic() + bounded_timeout(timeout)
    while True:
        check_budget()
        status = leader_status(process)
        if status is not None:
            return status.si_status if status.si_code == os.CLD_EXITED else -status.si_status
        require(time.monotonic() < deadline, "Owned gate deadline exhausted")
        time.sleep(0.05)


def remove_scratch(root, temporary, identity):
    require(root.parent == temporary and root.name == "fc11"
            and root.resolve() == root and not root.is_symlink(), "Scratch cleanup path drift")
    require(json.loads((root / "owner.json").read_bytes()) == identity, "Scratch ownership mismatch")
    shutil.rmtree(root)
    require(not root.exists(), "Scratch removal incomplete")


def failure_stage(rows):
    failed = next((name for name, state in rows if state == "failed" and name in GATES), None)
    if failed:
        return failed
    passed = [name for name, state in rows if state == "passed"]
    return GATES[len(passed)] if tuple(passed) == GATES[:len(passed)] and len(passed) < len(GATES) else "gate_contract"


def scratch_root(temporary):
    root = temporary / "fc11"
    # Preserve the exact source fixture while keeping its AF_UNIX name within Linux sun_path.
    socket_fixture = root / "tmp" / ("fleet-skill-readback-" + "0" * 36) / "agent1/config/skills/socket"
    require(len(os.fsencode(socket_fixture)) <= 107, "Owned TMPDIR exceeds source Unix socket fixture budget")
    return root


def execute():
    workspace, controls, workflow_sha = preflight()
    checkouts = ((workspace / "fleet-control", SOURCE_SHA), (workspace / "services-base", BASE_SHA),
                 (workspace / "base-auth-source", AUTH_SHA), (workspace / "base-role-package", PACKAGE_SHA))
    for checkout, commit in checkouts:
        clean_head(checkout, commit)
    require(command(["rustc", "--version"]).decode().split()[1] == "1.88.0", "Rust drift")
    require((checkouts[0][0] / ".base-revision").read_text().strip() == BASE_SHA, "Source Base drift")
    reviewed = reviewed_inventory(controls)
    package_before = qualify_package(checkouts[3][0], reviewed)
    temporary = Path(os.environ["RUNNER_TEMP"]).resolve()
    require(temporary.is_dir(), "Hosted temporary root missing")
    resources = resource_guard(temporary)
    root, evidence = scratch_root(temporary), temporary / "fleet-backend-evidence"
    evidence.mkdir(exist_ok=False)
    root.mkdir(exist_ok=False)
    identity = dict(workflow_sha=workflow_sha, run_id=int(os.environ["GITHUB_RUN_ID"]),
                    run_attempt=int(os.environ["GITHUB_RUN_ATTEMPT"]))
    (root / "owner.json").write_bytes(canonical(identity))
    owned_dbs, process, before, focused, rows = [], None, {}, {}, []
    code, compiler_failure, compiler_stage = None, None, None
    runtime_inventory, auth_binary_sha, failed_stage = {}, None, "preflight"
    contracts, migration_ledger = {}, {}
    cleanup, success = dict(scratch=False, synthetic_databases=False), False
    phase, failure = "source_export_fleet", None
    try:
        export(checkouts[0][0], SOURCE_SHA, root / "src/fleet-control", FLEET_ROOTS)
        phase = "source_export_sdk"
        export(checkouts[1][0], BASE_SHA, root / "src/services-base", ("crates", "Cargo.toml", "Cargo.lock", "LICENSE"))
        phase = "source_export_auth"
        export(checkouts[2][0], AUTH_SHA, root / "src/base-auth-source", ("crates", "Cargo.toml", "Cargo.lock", "LICENSE", "frontend/src/lib/theme-preference.js"))
        phase = "source_inventory"
        before = inventory(root / "src")
        require(before == reviewed["compiled_source_sha256"] and digest(canonical(before)) == SOURCE_INVENTORY_SHA, "Compiled input aggregate pin drift")
        phase = "source_declarations"
        validate_source_inventory(root / "src/fleet-control", reviewed)
        phase = "expectations"
        for name in ("tmp", "cargo", "target", "private", "expected"):
            (root / name).mkdir()
        for name, tests in reviewed["groups"].items():
            require(bool(tests) and len(tests) == len(set(tests)), "Empty/duplicate expectation")
            (root / "expected" / (name + ".txt")).write_text("\n".join(tests) + "\n", newline="\n")
        (root / "sources.sha256").write_text("".join(value + "  " + name + "\n" for name, value in before.items()), newline="\n")
        phase = "swagger_download"
        command(["curl", "--fail", "--location", "--proto", "=https", "--proto-redir", "=https", "--retry", "0",
                 "https://github.com/swagger-api/swagger-ui/archive/refs/tags/v5.17.14.zip", "--output", str(root / "swagger.zip")])
        phase = "swagger_hash"
        require(digest((root / "swagger.zip").read_bytes()) == SWAGGER_SHA, "Swagger hash mismatch")
        phase = "postgres_qualification"
        require(database_catalog() == ["fleet_foundation_test", "postgres"], "Service not a fresh owned PostgreSQL")
        require(psql("SHOW server_version_num").strip() == b"170006", "PostgreSQL version drift")
        require(psql("SELECT current_user").strip() == b"fleet_test", "Synthetic PG owner drift")
        require(psql("SELECT count(*) FROM pg_tables WHERE schemaname='public'",
                     database="fleet_foundation_test").strip() == b"0", "Initial fixture DB not empty")
        require(psql("SELECT rolname FROM pg_roles WHERE rolname='fleet_approval_events_test'").strip() == b"", "Fixture role already exists")
        owned_dbs = list(DATABASES)
        (root / "owned-databases.json").write_bytes(canonical(owned_dbs))
        phase = "postgres_initialization"
        psql(file=controls / INIT)
        require(database_catalog() == sorted([*DATABASES, "postgres"]), "Synthetic DB initialization incomplete")
        phase = "gate_execution"
        environment = {key: os.environ[key] for key in ("PATH", "HOME", "RUSTUP_HOME", "RUNNER_TRACKING_ID") if key in os.environ}
        environment.update(database_environment())
        environment.update(QA_ROOT=str(root), QA_OUTPUT=str(root / "private"), QA_EXPECTED=str(root / "expected"),
                           QA_SOURCES=str(root / "sources.sha256"), QA_HELPER=str(controls / HELPER),
                           QA_INVENTORY=str(controls / INVENTORY), QA_SOURCE_COMMIT=SOURCE_SHA, QA_AUTH_SOURCE_COMMIT=AUTH_SHA,
                           PYTHONDONTWRITEBYTECODE="1",
                           FLEET_TEST_BASE_PACKAGE_CHECKOUT=str(checkouts[3][0]),
                           RUSTUP_TOOLCHAIN="1.88.0", CARGO_HOME=str(root / "cargo"), CARGO_TARGET_DIR=str(root / "target"),
                           TMPDIR=str(root / "tmp"), CARGO_BUILD_JOBS="1", CARGO_INCREMENTAL="0",
                           CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0", LC_ALL="C",
                           SWAGGER_UI_DOWNLOAD_URL=(root / "swagger.zip").as_uri())
        with (root / "private/driver.log").open("xb") as diagnostics:
            process = subprocess.Popen(["bash", str(controls / GATE)], env=environment,
                                       stdout=diagnostics, stderr=diagnostics, start_new_session=True)
            code = wait_owned_exit(process, GATE_SECONDS)
            stop_owned_group(process)
        phase = "gate_receipts"
        rows = [line.split("\t") for line in (root / "private/gates.tsv").read_text().splitlines()]
        failed_stage = failure_stage(rows)
        require(code == 0 and rows == [[name, "passed"] for name in GATES], "Incomplete or failed backend gate")
        runtime_inventory = verify_runtime_inventory((root / "private/workspace-list.log").read_text(),
                                                      (root / "private/ignored-list.log").read_text(), reviewed)
        for stage in (*reviewed["groups"], "workspace"):
            focused[stage] = verify_test_log(stage, (root / "private" / (stage + ".log")).read_text(), reviewed,
                                            (root / "private/workspace-list.log").read_text())
        require(digest((root / "private/openapi.json").read_bytes()) == OPENAPI_SHA, "Strict schema hash mismatch")
        require(inventory(root / "src") == before, "Compiled source drift")
        require(qualify_package(checkouts[3][0], reviewed) == package_before, "Package post-gate drift")
        migration_ledger = verify_migration_snapshots(root / "private", reviewed)
        for checkout, commit in checkouts:
            clean_head(checkout, commit)
        clean_head(controls, workflow_sha)
        auth_binary_sha = (root / "private/auth-binary.sha256").read_text().split()[0]
        require(bool(re.fullmatch(r"[0-9a-f]{64}", auth_binary_sha)), "Missing fresh Auth binary hash")
        check_budget()
        success = True
    except Exception as error:
        # Never print raw Cargo/Base/test diagnostics, arguments or exception text.
        failure = failure_kind(error)
        if (root / "private/gates.tsv").exists():
            rows = [line.split("\t") for line in (root / "private/gates.tsv").read_text().splitlines()]
            failed_stage = failure_stage(rows)
    finally:
        try:
            if BUDGET is not None:
                BUDGET.cleanup()
            stop_owned_group(process)
            if not success and failed_stage in ("check", "clippy"):
                compiler_stage = failed_stage
                compiler_failure = compiler_failure_logs(root, compiler_stage, reviewed)
            elif not success and failed_stage == "runtime_inventory":
                compiler_stage = failed_stage
                compiler_failure = inventory_failure_logs(root, reviewed)
            elif not success and failure_test_names(reviewed, failed_stage):
                compiler_stage = failed_stage
                compiler_failure = test_failure_logs(root, compiler_stage, reviewed)
            if owned_dbs:
                drop_databases(owned_dbs)
                cleanup["synthetic_databases"] = True
            else:
                cleanup["synthetic_databases"] = "not_initialized"
            remove_scratch(root, temporary, identity)
            cleanup["scratch"] = True
        except Exception as error:
            success, failed_stage = False, "cleanup"
            phase, failure = "cleanup", failure_kind(error)
    safe_rows = [{"stage": name, "status": state} for name, state in rows
                 if name in GATES and state in ("passed", "failed")]
    report = dict(version=1, backend_quality_gate=success, all_quality_gate=False, sdlc_acceptance=False,
                  status="success" if success else "failure", failed_stage=None if success else failed_stage,
                  gates=safe_rows, focused=focused, runtime_inventory=runtime_inventory,
                  contracts=contracts, migration_ledger=migration_ledger,
                  ignored_required=IGNORED_COUNT, foundation_ignored=FOUNDATION_IGNORED, resources=resources, cleanup=cleanup,
                  service_disposal="GitHub-managed ephemeral service, platform cleanup after job",
                  local_docker_or_native_guard_waiver=False, private_diagnostics_uploaded=False)
    provenance = dict(version=1, repository=REPOSITORY, branch=BRANCH, source_sha=SOURCE_SHA, base_sha=BASE_SHA,
                      auth_sha=AUTH_SHA,
                      package_sha=PACKAGE_SHA, package_tree=PACKAGE_TREE, package_inventory_sha256=PACKAGE_INVENTORY_SHA,
                      source_tree=git(checkouts[0][0], "rev-parse", "HEAD^{tree}").decode().strip(),
                      base_tree=git(checkouts[1][0], "rev-parse", "HEAD^{tree}").decode().strip(),
                      auth_tree=git(checkouts[2][0], "rev-parse", "HEAD^{tree}").decode().strip(),
                      workflow_sha=workflow_sha, workflow_path=WORKFLOW, **{k: identity[k] for k in ("run_id", "run_attempt")},
                      rust="1.88.0", postgres="17.6", cargo_build_jobs=1, auth_binary_sha256=auth_binary_sha,
                      openapi_sha256=OPENAPI_SHA, source_inventory_sha256=digest(canonical(before)),
                      lock_sha256={name: value for name, value in before.items() if name.endswith("Cargo.lock")},
                      control_sha256={name: digest((controls / name).read_bytes()) for name in sorted(WRITE_SET)},
                      swagger_sha256=SWAGGER_SHA, runner_image=os.environ.get("ImageOS"),
                      runner_image_version=os.environ.get("ImageVersion"), backend_quality_gate=success,
                      all_quality_gate=False, sdlc_acceptance=False)
    for name, value in (("report.json", report), ("provenance.json", provenance)):
        check_budget()
        (evidence / name).write_bytes(canonical(value))
    (evidence / "SHA256SUMS").write_text("".join(digest((evidence / name).read_bytes()) + "  " + name + "\n"
                                              for name in ("report.json", "provenance.json")), newline="\n")
    if not success and compiler_failure is not None:
        failure_artifact = dict(version=1, kind="safe_inventory_failure" if compiler_stage == "runtime_inventory" else
                               "safe_compiler_failure" if compiler_stage in ("check", "clippy") else "safe_test_failure", status="failure", repository=REPOSITORY,
                       branch=BRANCH, workflow_path=WORKFLOW,
                       **identity, source_sha=SOURCE_SHA, base_sha=BASE_SHA, auth_sha=AUTH_SHA,
                       package_sha=PACKAGE_SHA, package_tree=PACKAGE_TREE, package_inventory_sha256=PACKAGE_INVENTORY_SHA,
                       source_inventory_sha256=digest(canonical(before)), control_sha256=provenance["control_sha256"],
                       stage=compiler_stage, gate_failed_stage=failed_stage,
                       gate_exit_code=code if type(code) is int and 0 <= code <= 255 else None,
                       **compiler_failure, cleanup=cleanup, backend_quality_gate=False,
                       all_quality_gate=False, sdlc_acceptance=False)
        validate_failure_evidence(failure_artifact, workflow_sha=workflow_sha, run_id=identity["run_id"], attempt=identity["run_attempt"])
        data = canonical(failure_artifact)
        require(len(data) <= FAILURE_SIZE_LIMIT, "Safe failure evidence exceeds bound")
        failure_root = temporary / "fleet-backend-failure-evidence"
        failure_root.mkdir(exist_ok=False)
        with (failure_root / FAILURE_FILE).open("xb") as output:
            output.write(data)
        print(json.dumps(failure_artifact))
    print(json.dumps(dict(state="backend_quality_gate_passed" if success else "backend_quality_gate_failed",
                          failed_stage=None if success else failed_stage, private_logs_uploaded=False,
                          control_phase=None if success else phase, failure=None if success else failure,
                          cleanup=cleanup,
                          all_quality_gate=False, sdlc_acceptance=False)))
    check_budget()
    return 0 if success else code if type(code) is int and 1 <= code <= 255 else 1


def cleanup_fallback():
    workspace, workflow_sha = hosted_identity()
    temporary = Path(os.environ["RUNNER_TEMP"]).resolve()
    root = temporary / "fc11"
    if root.exists():
        identity = dict(workflow_sha=workflow_sha, run_id=int(os.environ["GITHUB_RUN_ID"]),
                        run_attempt=int(os.environ["GITHUB_RUN_ATTEMPT"]))
        require(not root.is_symlink() and json.loads((root / "owner.json").read_bytes()) == identity,
                "Fallback ownership mismatch")
        # The marker is written only after fresh service/role qualification, before any owned CREATE.
        marker = root / "owned-databases.json"
        if marker.exists():
            require(json.loads(marker.read_bytes()) == list(DATABASES), "Fallback database manifest drift")
            drop_databases(list(DATABASES))
        # A killed step cannot publish PASS; platform disposal owns its service.
        remove_scratch(root, temporary, identity)


def validate_readback(run, artifact, payload, *, run_id, attempt, workflow_sha, artifact_digest):
    require(run["id"] == run_id and run["run_attempt"] == attempt and run["status"] == "completed"
            and run["conclusion"] == "success" and run["event"] == "push" and run["head_sha"] == workflow_sha
            and run["head_branch"] == BRANCH and run["path"] == WORKFLOW
            and run["repository"]["full_name"] == REPOSITORY, "Unexpected/unsuccessful workflow identity")
    require(not artifact["expired"] and artifact["workflow_run"]["id"] == run_id
            and artifact["workflow_run"]["head_sha"] == workflow_sha
            and artifact["name"] == f"fleet-backend-c11-{run_id}-{attempt}"
            and artifact["digest"] == "sha256:" + artifact_digest
            and digest(payload) == artifact_digest, "Artifact identity/digest mismatch")
    require(len(payload) <= 8 * 1024 ** 2, "Oversized artifact ZIP")
    with zipfile.ZipFile(io.BytesIO(payload)) as archive:
        members = archive.infolist()
        require(len(members) == 3 and {item.filename for item in members} == ARTIFACT_FILES
                and all(item.file_size <= 8 * 1024 ** 2 and (item.external_attr >> 16) & 0o170000 != 0o120000
                        for item in members), "Unsafe artifact members")
        files = {item.filename: archive.read(item) for item in members}
    provenance, report = json.loads(files["provenance.json"]), json.loads(files["report.json"])
    controls = Path(__file__).resolve().parents[1]
    expected = dict(version=1, repository=REPOSITORY, branch=BRANCH, source_sha=SOURCE_SHA, base_sha=BASE_SHA,
                    auth_sha=AUTH_SHA, workflow_sha=workflow_sha, workflow_path=WORKFLOW, run_id=run_id,
                    run_attempt=attempt, rust="1.88.0", postgres="17.6", cargo_build_jobs=1,
                    swagger_sha256=SWAGGER_SHA, openapi_sha256=OPENAPI_SHA, backend_quality_gate=True,
                    all_quality_gate=False, sdlc_acceptance=False,
                    package_sha=PACKAGE_SHA, package_tree=PACKAGE_TREE, package_inventory_sha256=PACKAGE_INVENTORY_SHA,
                    source_inventory_sha256=SOURCE_INVENTORY_SHA,
                    control_sha256={name: digest((controls / name).read_bytes()) for name in sorted(WRITE_SET)})
    require(all(type(provenance.get(key)) is type(value) and provenance.get(key) == value
                for key, value in expected.items()), "Provenance mismatch")
    require(report["backend_quality_gate"] is True and report["all_quality_gate"] is False
            and report["sdlc_acceptance"] is False and report["status"] == "success"
            and report["gates"] == [dict(stage=name, status="passed") for name in GATES]
            and report["cleanup"] == dict(scratch=True, synthetic_databases=True)
            and report["ignored_required"] == IGNORED_COUNT and report["foundation_ignored"] == FOUNDATION_IGNORED
            and report["runtime_inventory"]["ignored"] == IGNORED_COUNT, "Incomplete backend gate receipt")
    reviewed = reviewed_inventory(controls)
    require_codegen_binding(reviewed)
    require(provenance["source_tree"] == SOURCE_TREE, "Source tree drift")
    require(report["migration_ledger"] == dict(snapshots=expected_migration_receipt(reviewed), applied_at_preserved=True),
            "Migration ledger receipt drift")
    require(set(report["contracts"]) == set(reviewed["python_contracts"]), "Python stage receipt omission")
    for stage, names in reviewed["python_contracts"].items():
        require(report["contracts"][stage] == dict(passed=len(names), failed=0, ignored=0, tests=sorted(names)),
                "Python exact case receipt mismatch")
    require(set(report["focused"]) == set(reviewed["groups"]) | {"workspace"}, "Focused stage omission")
    for stage, names in reviewed["groups"].items():
        actual = report["focused"][stage]
        require(actual == dict(passed=len(names), failed=0, ignored=FOUNDATION_IGNORED if stage == "foundation" else 0,
                               tests=sorted(names)), "Focused test receipt mismatch")
    ws = report["focused"]["workspace"]
    require(ws["passed"] == DEFAULT_COUNT and ws["failed"] == 0 and ws["ignored"] == IGNORED_COUNT
            and len(ws["tests"]) == ws["passed"] == report["runtime_inventory"]["listed_default_count"],
            "Empty/failed workspace receipt")
    require(report["runtime_inventory"]["ignored_names_sha256"] == digest(canonical(sorted(
        item["name"] for item in reviewed["ignored"]))), "Ignored runtime identity receipt drift")
    require(bool(re.fullmatch(r"[0-9a-f]{64}", provenance.get("auth_binary_sha256", ""))), "Missing Auth binary provenance")
    expected_sums = "".join(digest(files[name]) + "  " + name + "\n" for name in ("report.json", "provenance.json"))
    require(files["SHA256SUMS"] == expected_sums.encode(), "Checksum manifest mismatch")
    return files


def validate_failure_readback(run, artifact, payload, *, run_id, attempt, workflow_sha, artifact_digest):
    require(run["id"] == run_id and run["run_attempt"] == attempt and run["status"] == "completed"
            and run["conclusion"] == "failure" and run["event"] == "push" and run["head_sha"] == workflow_sha
            and run["head_branch"] == BRANCH and run["path"] == WORKFLOW
            and run["repository"]["full_name"] == REPOSITORY, "Unexpected failure workflow identity")
    require(not artifact["expired"] and artifact["workflow_run"]["id"] == run_id
            and artifact["workflow_run"]["head_sha"] == workflow_sha
            and artifact["name"] == f"fleet-backend-failure-c11-{run_id}-{attempt}"
            and artifact["digest"] == "sha256:" + artifact_digest and digest(payload) == artifact_digest,
            "Failure artifact identity/digest mismatch")
    require(len(payload) <= 2 * FAILURE_SIZE_LIMIT, "Failure artifact ZIP bound")
    with zipfile.ZipFile(io.BytesIO(payload)) as archive:
        members = archive.infolist()
        require(len(members) == 1 and members[0].filename == FAILURE_FILE
                and members[0].file_size <= FAILURE_SIZE_LIMIT
                and (members[0].external_attr >> 16) & 0o170000 != 0o120000, "Unsafe failure artifact members")
        data = archive.read(members[0])
    validate_failure_evidence(json.loads(data), workflow_sha=workflow_sha, run_id=run_id, attempt=attempt)
    return {FAILURE_FILE: data}


def readback(args):
    require_codegen_binding(reviewed_inventory(Path(__file__).resolve().parents[1]))
    require(bool(re.fullmatch(r"[0-9a-f]{40}", args.workflow_sha))
            and bool(re.fullmatch(r"[0-9a-f]{64}", args.artifact_digest))
            and min(args.run_id, args.attempt, args.artifact_id) > 0, "Explicit reviewed artifact identity required")
    endpoint = "repos/" + REPOSITORY + "/actions/"
    run = json.loads(command(["gh", "api", "--method", "GET", endpoint + f"runs/{args.run_id}/attempts/{args.attempt}"]))
    artifact_url = endpoint + f"artifacts/{args.artifact_id}"
    artifact = json.loads(command(["gh", "api", "--method", "GET", artifact_url]))
    require(artifact["id"] == args.artifact_id and artifact["size_in_bytes"] <= 8 * 1024 ** 2, "Unexpected artifact size/ID")
    payload = command(["gh", "api", "--method", "GET", artifact_url + "/zip"])
    failure = args.mode == "readback-failure"
    validator = validate_failure_readback if failure else validate_readback
    files = validator(run, artifact, payload, run_id=args.run_id, attempt=args.attempt,
                      workflow_sha=args.workflow_sha, artifact_digest=args.artifact_digest)
    args.output.mkdir(parents=True, exist_ok=False)
    for name, data in files.items():
        (args.output / name).write_bytes(data)
    print(json.dumps(dict(state="verified_" + json.loads(files[FAILURE_FILE])["kind"] if failure else "verified_hosted_backend_gate",
                          output=str(args.output.resolve()), backend_quality_gate=not failure,
                          all_quality_gate=False, sdlc_acceptance=False)))


def verify_log_cli(stage):
    root = Path(os.environ["QA_OUTPUT"])
    reviewed = json.loads(Path(os.environ["QA_INVENTORY"]).read_bytes())
    if stage == "runtime_inventory":
        verify_runtime_inventory((root / "workspace-list.log").read_text(), (root / "ignored-list.log").read_text(), reviewed)
    elif stage == "migration_smoke":
        verify_migration_snapshots(root, reviewed)
    elif stage == "compiled_source_parity":
        qualify_package(Path(os.environ["FLEET_TEST_BASE_PACKAGE_CHECKOUT"]), reviewed)
    elif stage in reviewed["groups"] or stage == "workspace":
        listing = (root / "workspace-list.log").read_text() if stage == "workspace" else ""
        verify_test_log(stage, (root / (stage + ".log")).read_text(), reviewed, listing)


def main():
    global BUDGET, VERIFY_IN_GATE
    parser = argparse.ArgumentParser(__doc__)
    modes = parser.add_subparsers(dest="mode", required=True)
    for name in ("preflight", "execute", "cleanup"):
        modes.add_parser(name)
    verify = modes.add_parser("verify-log")
    verify.add_argument("stage", choices=GATES)
    for mode in ("readback", "readback-failure"):
        read = modes.add_parser(mode)
        for name in ("run-id", "attempt", "artifact-id"):
            read.add_argument("--" + name, type=int, required=True)
        for name in ("workflow-sha", "artifact-digest"):
            read.add_argument("--" + name, required=True)
        read.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.mode in ("preflight", "execute", "cleanup"):
            BUDGET = hosted_budget()
            if args.mode == "cleanup":
                BUDGET.cleanup(fallback=True)
            else:
                BUDGET.arm()
        if args.mode in ("readback", "readback-failure"):
            readback(args)
        elif args.mode == "preflight":
            preflight()
        elif args.mode == "cleanup":
            cleanup_fallback()
        elif args.mode == "verify-log":
            VERIFY_IN_GATE = True
            verify_log_cli(args.stage)
        else:
            return execute()
    except Exception:
        print("Backend control failed; no private diagnostics emitted; acceptance withheld")
        return 1
    finally:
        VERIFY_IN_GATE = False
        if BUDGET is not None:
            signal.setitimer(signal.ITIMER_REAL, 0)
            BUDGET = None
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
