"""Dedicated hosted frontend verification and bounded, authenticated evidence readback."""
from __future__ import annotations

import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import tempfile
import time
import zipfile
import zlib

REPOSITORY = "FerrPOINT/fleet-control"
BRANCH = "build-only/frontend-main-union-20261010"
SOURCE_SHA = "089ee0c7066adf459849556f511d81dc859cb6c3"
SOURCE_TREE = "8f288ba7db64911fc5ca8f3703ee981528e557f6"
SOURCE_PARENTS = ["facb25b9eae66db0c8b762ab68a5963422edf58f", "a8b045a080dd11da9827279c7cd79088f30542e7"]
BASE_SHA = "19a7a381ae6dbea61a643bb96189e483fa64df5c"
BASE_TREE = "aa1a0486af1922c5a7fd4471e71e4fbb6aa4c7cc"
BASE_MATERIALIZED_FILES = {
    "crates/auth-server/migrations/0001_users_sessions.sql": (
        "69f7ec8116b228dede62375c7c8b45e1af233887",
        "ef0fae09d1a5359eb23ade564541b03bc1f1514c2017317fc7922ced72c26d75",
    ),
}
SCHEMA_SHA256 = "ad980604beb2cff0890f4d1a07a185c97a444fda166985f2a6da465a222d129c"
WORKFLOW = ".github/workflows/frontend-build-only.yml"
WRITE_SET = {WORKFLOW, "scripts/hosted_frontend_gate.py", "scripts/tests/test_hosted_frontend_gate.py"}
NODE = "22.20.0"
PNPM = "10.28.1"
QUALIFIED_UNIT_COUNTS = dict(files_passed=38, tests_passed=382, files_skipped=0, tests_skipped=0)
QUALIFIED_INPUTS = {
    "source_inventory_sha256": "53a917fdffc74d764532760ad1b80ac2e97df2b054ab63c6c6617d034319ad6a",
    "base_inventory_sha256": "437244f3861d17356cbe33162dceca877aea74d82dccae9b9b1a2915b62ee444",
    "frontend_lock_sha256": "37918d9d24852a14f24c43a593777e99d36e0e58de2e7b9a58a4415fd927fb67",
    "base_lock_sha256": "149adc7015cd1b7fa1d093e5501156ed6e149efbc82b222c82a2797912261fb4",
}
VIEWPORTS = ("375x812", "1920x1080", "2560x1440")
SOURCE_FIXTURE_SCREENS = frozenset(
    f"docs/assets/screens/chats-core-main-20261007/{name}-{viewport}.png"
    for name in ("dialogue", "clarification-unavailable", "requirements-unavailable", "denied", "read-only")
    for viewport in VIEWPORTS
)
SCREEN_MANIFEST = "docs/assets/screens/manifest.md"
GENERATED = "frontend/src/api/generated.ts"
MAX_FILE = 16 * 1024 ** 2
MAX_ARTIFACT = 192 * 1024 ** 2
MAX_MEMBERS = 800
FAILURE_LIMIT = 128 * 1024
FAILURE_FILE = "failure.json"
FAILURE_SCOPE = dict(frontend_unit_build_fixture=False, live_pm_acceptance=False,
                     live_runtime_acceptance=False, all_sdlc_acceptance=False)
BROWSER_PROJECTS = ("chromium", "firefox", "webkit")
BROWSER_STATUSES = ("expected", "unexpected", "flaky", "skipped")
RESULT_STATUSES = ("passed", "failed", "timedOut", "skipped", "interrupted")
SCOPE = dict(frontend_unit_build_fixture=True, live_pm_acceptance=False,
             live_runtime_acceptance=False, all_sdlc_acceptance=False)
SAFE_FAILURE_HINTS = {
    "Source bytes differ from exact committed tree": "source_bytes",
    "Base materialized bytes differ from the pinned attribute contract": "base_materialization",
    "Qualified dependency/source inventory mismatch": "input_inventory",
    "Dirty checkout": "dirty_checkout",
    "Persisted checkout credentials forbidden": "checkout_credentials",
    "Checkout origin mismatch": "checkout_origin",
    "Node version mismatch": "node_version",
    "pnpm version mismatch": "pnpm_version",
    "Frontend dependency/generation convention mismatch": "frontend_convention",
    "Pre-existing ignored inputs/secrets/build products forbidden": "ignored_inputs",
}
THEME = """set -euo pipefail
pnpm exec vite preview --host 127.0.0.1 --port 4173 --strictPort > "$RUNNER_TEMP/fleet-frontend-private/theme-preview.log" 2>&1 &
preview_pid=$!
trap 'kill "$preview_pid" 2>/dev/null || true; wait "$preview_pid" 2>/dev/null || true' EXIT
pnpm theme:check http://127.0.0.1:4173
"""
# Keep CI's commands and order; only the browser JSON reporter adds evidence.
GATES = {
    "base": [["python3", "../../services-base/scripts/verify_base_revision.py",
              "--revision", "../.base-revision", "--base", "../../services-base"]],
    "install": [["pnpm", "install", "--frozen-lockfile"]],
    "openapi": [["pnpm", "openapi:check"]],
    "chat": [["pnpm", "chat:contract"]],
    "chat-evidence": [["pnpm", "chat:evidence:verify"]],
    "ui-install": [["pnpm", "--dir", "../../services-base/frontend", "install", "--frozen-lockfile"]],
    "ui": [["pnpm", "ui:check"]],
    "markdown": [["pnpm", "markdown:check"]],
    "screens-before": [["pnpm", "screenshots:verify"]],
    "typecheck": [["pnpm", "typecheck"]],
    "unit": [["pnpm", "test", "--", "--run"]],
    "lint": [["pnpm", "lint"]],
    "compat": [["git", "fetch", "--no-tags", "origin", "+refs/heads/main:refs/remotes/origin/main", "--depth=1"],
               ["pnpm", "openapi:compat"]],
    "generated": [["git", "diff", "--exit-code", "--", "pnpm-lock.yaml",
                   "src/api/generated.ts", "src/api/schema.d.ts"]],
    "build": [["pnpm", "build"]],
    "consumer": [["python", "../../services-base/scripts/check_ui_consumer.py"]],
    "theme-browser": [["pnpm", "exec", "playwright", "install", "--with-deps", "chromium"]],
    "theme": [["bash", "-c", THEME]],
    "format": [["pnpm", "format:check"]],
    "browsers": [["pnpm", "exec", "playwright", "install", "--with-deps", "chromium", "firefox", "webkit"]],
    "fixtures": [["pnpm", "exec", "playwright", "test", "--reporter=list,json", "--max-failures=1"]],
    "capture": [["pnpm", "screenshots:local"]],
    "screens-after": [["pnpm", "screenshots:verify"]],
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return (json.dumps(value, sort_keys=True, indent=2) + "\n").encode()


def bounded_command(args, *, limit=1024 ** 2, cwd=None):
    # Bound the download itself, not just ZIP expansion; never echo diagnostics.
    deadline = time.monotonic() + 120
    with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
        process = subprocess.Popen(args, cwd=cwd, stdout=output, stderr=errors)
        try:
            while True:
                require(os.fstat(output.fileno()).st_size <= limit
                        and os.fstat(errors.fileno()).st_size <= MAX_FILE, "Control response exceeds size limit")
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise TimeoutError("Control command deadline exceeded")
                code = process.poll()
                if code is not None:
                    require(code == 0, "Control command failed")
                    output.seek(0)
                    data = output.read(limit + 1)
                    require(len(data) <= limit, "Control response exceeds size limit")
                    require(time.monotonic() < deadline, "Control command deadline exceeded")
                    return data
                time.sleep(min(0.02, remaining))
        finally:
            # No pipe reader waits on EOF from descendants; signal only this child.
            if process.poll() is None:
                process.kill()
            process.wait(timeout=5)


def git(root, *args):
    return bounded_command(["git", "--no-replace-objects", "-C", str(root), *args], limit=MAX_FILE)


def validate_source_tuple(commit, tree, parents):
    require((commit, tree, parents) == (SOURCE_SHA, SOURCE_TREE, SOURCE_PARENTS),
            "Exact source commit/tree/parent tuple mismatch")


def qualify_source(root):
    parents = git(root, "show", "-s", "--format=%P", SOURCE_SHA).decode().strip().split()
    tree = git(root, "rev-parse", SOURCE_SHA + "^{tree}").decode().strip()
    validate_source_tuple(SOURCE_SHA, tree, parents)
    require(digest(git(root, "show", SOURCE_SHA + ":openapi/openapi.json")) == SCHEMA_SHA256,
            "Source generated OpenAPI hash mismatch")


def validate_delta(value):
    changes = [tuple(line.split("\t")) for line in value.splitlines()]
    require(len(changes) == 3 and set(changes) == {("A", name) for name in WRITE_SET},
            "Only the three new frontend build control files are permitted")


def public_repository(value):
    require(value.get("full_name") == REPOSITORY and value.get("private") is False
            and value.get("visibility") == "public" and value.get("default_branch") == "main",
            "Expected public product repository/main")


def hosted_identity():
    expected = dict(GITHUB_ACTIONS="true", RUNNER_ENVIRONMENT="github-hosted",
                    GITHUB_REPOSITORY=REPOSITORY, GITHUB_EVENT_NAME="push",
                    GITHUB_REF="refs/heads/" + BRANCH, CI="true")
    require(os.name == "posix" and all(os.environ.get(k) == v for k, v in expected.items()),
            "Dedicated GitHub-hosted branch push required")
    require(re.fullmatch(r"[0-9a-f]{40}", os.environ.get("GITHUB_SHA", "")), "Missing workflow SHA")
    for name in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT"):
        require(re.fullmatch(r"[1-9][0-9]*", os.environ.get(name, "")), "Missing run identity")
    workspace = Path(os.environ["GITHUB_WORKSPACE"])
    require(workspace.is_absolute() and workspace.resolve() == workspace, "Workspace alias forbidden")
    return workspace, os.environ["GITHUB_SHA"]


def checkout(root, sha, repository, *, clean=False):
    require(root.resolve() == root and root.is_dir() and not root.is_symlink(), "Checkout alias forbidden")
    require(git(root, "rev-parse", "HEAD").decode().strip() == sha, "Actual checkout SHA mismatch")
    remote = git(root, "remote", "get-url", "origin").decode().strip()
    require(remote in (f"https://github.com/{repository}", f"https://github.com/{repository}.git"),
            "Checkout origin mismatch")
    configuration = git(root, "config", "--local", "--list").decode().lower()
    require("extraheader=" not in configuration and "credential.helper=" not in configuration
            and "insteadOf=".lower() not in configuration, "Persisted checkout credentials forbidden")
    if clean:
        require(not git(root, "status", "--porcelain=v1", "--untracked-files=all"), "Dirty checkout")
        require(not git(root, "ls-files", "--others", "--ignored", "--exclude-standard", "-z"),
                "Pre-existing ignored inputs/secrets/build products forbidden")


def controls_preflight():
    workspace, sha = hosted_identity()
    controls = workspace / "controls"
    checkout(controls, sha, REPOSITORY, clean=True)
    qualify_source(controls)
    git(controls, "merge-base", "--is-ancestor", SOURCE_SHA, sha)
    validate_delta(git(controls, "diff", "--no-renames", "--name-status", SOURCE_SHA, sha).decode())
    require((controls / ".base-revision").read_text().strip() == BASE_SHA, "Control Base pin mismatch")
    return workspace, sha


def preflight():
    workspace, _ = controls_preflight()
    public_repository(json.loads(bounded_command(["gh", "api", "--method", "GET", "repos/" + REPOSITORY])))
    commit = json.loads(bounded_command(["gh", "api", "--method", "GET",
                                        "repos/" + REPOSITORY + "/git/commits/" + SOURCE_SHA]))
    validate_source_tuple(commit["sha"], commit["tree"]["sha"], [parent["sha"] for parent in commit["parents"]])
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_bytes())
    public_repository(event["repository"])
    require(event.get("deleted") is False and event.get("ref") == "refs/heads/" + BRANCH
            and event.get("after") == os.environ["GITHUB_SHA"], "Push payload mismatch")
    private = workspace_temp() / "fleet-frontend-private"
    private.mkdir(mode=0o700, exist_ok=False)
    (private / "preflight.json").write_bytes(canonical(dict(workflow_sha=os.environ["GITHUB_SHA"],
        run_id=os.environ["GITHUB_RUN_ID"], attempt=os.environ["GITHUB_RUN_ATTEMPT"], public=True)))
    print("Public source and exact three-file delta qualified; Base checkout may proceed.")


def workspace_temp():
    root = Path(os.environ["RUNNER_TEMP"])
    require(root.is_absolute() and root.resolve() == root and root.is_dir(), "Temporary root alias forbidden")
    return root


def safe_path(name):
    path = PurePosixPath(name)
    require(name and str(path) == name and not path.is_absolute() and ".." not in path.parts
            and not re.search(r"[\x00-\x20\x7f:\\]", name), "Unsafe relative evidence path")
    return path


def bounded_file(root, name, limit=MAX_FILE):
    path = root.joinpath(*safe_path(name).parts)
    require(path.resolve().is_relative_to(root.resolve()) and path.is_file()
            and not any(part.is_symlink() for part in [path, *path.parents] if part != root.parent),
            "Evidence symlink or non-file forbidden")
    require(path.stat().st_size <= limit, "Oversized evidence file")
    with path.open("rb") as source:
        data = source.read(limit + 1)
    require(len(data) <= limit, "Evidence grew beyond size limit")
    return data


def tracked_inventory(root, sha):
    result = {}
    for item in git(root, "ls-tree", "-rz", sha).split(b"\0"):
        if not item:
            continue
        meta, raw_name = item.split(b"\t", 1)
        mode, kind, blob = meta.split()
        require(mode in (b"100644", b"100755") and kind == b"blob", "Source links/submodules forbidden")
        name = raw_name.decode()
        data = bounded_file(root, name)
        # Git SHA1 here verifies actual bytes against the committed blob, including ignored contexts.
        actual = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
        verify_materialized_file(sha, name, blob.decode(), actual, data)
        result[name] = digest(data)
    return result


def verify_materialized_file(sha, name, blob, actual, data):
    # Base19a explicitly materializes this legacy migration with CRLF. Never rewrite it.
    if sha == BASE_SHA and name in BASE_MATERIALIZED_FILES:
        expected_blob, expected_bytes = BASE_MATERIALIZED_FILES[name]
        require(blob == expected_blob and digest(data) == expected_bytes,
                "Base materialized bytes differ from the pinned attribute contract")
    else:
        require(actual == blob, "Source bytes differ from exact committed tree")


def safe_failure_hint(error):
    return SAFE_FAILURE_HINTS.get(str(error), "unclassified")


class GateFailure(ValueError):
    def __init__(self, category, code=None):
        super().__init__("Frontend command failed; private diagnostics withheld")
        self.category, self.code = category, code


def attested_test_locations(controls):
    locations = {}
    for item in git(controls, "ls-tree", "-rz", SOURCE_SHA, "frontend/e2e").split(b"\0"):
        if not item:
            continue
        meta, raw_name = item.split(b"\t", 1)
        mode, kind, blob = meta.split()
        name = raw_name.decode()
        if not re.fullmatch(r"frontend/e2e/[a-z0-9-]+\.spec\.ts", name):
            continue
        require(mode in (b"100644", b"100755") and kind == b"blob", "Invalid test source inventory")
        data = bounded_file(controls, name)
        require(hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest() == blob.decode(),
                "Test source differs from canonical Git blob")
        locations[name] = [n for n, line in enumerate(data.decode().splitlines(), 1)
                           if re.match(r"\s*test(?:\.(?:only|skip|fixme))?\(\s*['\"`]", line)]
    require(0 < len(locations) <= 100 and sum(map(len, locations.values())) <= 4096,
            "Invalid test source declaration inventory")
    return locations


def safe_browser_failure(data, locations):
    result = dict(report="unavailable", diagnostics=[], browsers={
        name: {status: 0 for status in BROWSER_STATUSES} for name in BROWSER_PROJECTS})
    if data is None:
        return result
    if len(data) > 4 * 1024 ** 2:
        return dict(result, report="truncated")
    # Copy no runtime strings, errors, titles, attachments, snippets or stdio.
    try:
        report = json.loads(data)
        require(isinstance(report, dict) and isinstance(report.get("suites"), list), "Invalid browser report")
        stack = [(suite, 0) for suite in report["suites"]]
        seen, nodes, tests = set(), 0, 0
        while stack:
            suite, depth = stack.pop()
            nodes += 1
            require(nodes <= 4096 and depth <= 16 and isinstance(suite, dict), "Browser report bound")
            for spec in suite.get("specs", []):
                nodes += 1
                require(nodes <= 4096, "Browser spec bound")
                require(isinstance(spec, dict), "Invalid browser spec")
                file, line = spec.get("file"), spec.get("line")
                file = next((name for name in locations if file in
                             (name, name.removeprefix("frontend/"), name.removeprefix("frontend/e2e/"))), None)
                require(file is not None and type(line) is int and line in locations[file], "Unattested test location")
                for test in spec["tests"]:
                    tests += 1
                    require(tests <= 4096 and isinstance(test, dict), "Browser test bound")
                    project, status = test.get("projectName"), test.get("status")
                    require(project in BROWSER_PROJECTS and status in BROWSER_STATUSES, "Unknown browser status")
                    attempts = test.get("results", [])
                    require(isinstance(attempts, list) and len(attempts) <= 3, "Browser retry bound")
                    statuses = sorted({attempt["status"] for attempt in attempts})
                    require(all(s in RESULT_STATUSES for s in statuses), "Unknown browser result")
                    result["browsers"][project][status] += 1
                    if status == "unexpected":
                        key = (file, line, project, status, tuple(statuses))
                        if key not in seen:
                            seen.add(key)
                            require(len(seen) <= 128, "Browser diagnostic bound")
                            result["diagnostics"].append(dict(file=file, line=line, project=project,
                                                               status=status, results=statuses))
            stack.extend((child, depth + 1) for child in suite.get("suites", []))
            require(len(stack) <= 4096, "Browser suite bound")
        return dict(result, report="valid")
    except (ValueError, TypeError, KeyError, RecursionError):
        # Do not retain partially validated counters or locations.
        return dict(report="rejected", diagnostics=[], browsers={
            name: {status: 0 for status in BROWSER_STATUSES} for name in BROWSER_PROJECTS})


def validate_failure(value, *, workflow_sha, run_id, attempt, locations):
    controls = Path(__file__).resolve().parents[1]
    expected = dict(version=1, kind="safe_frontend_failure", status="failure", repository=REPOSITORY,
        branch=BRANCH, workflow_path=WORKFLOW, workflow_sha=workflow_sha, run_id=run_id, run_attempt=attempt,
        source_sha=SOURCE_SHA, source_tree=SOURCE_TREE, source_parents=SOURCE_PARENTS,
        base_sha=BASE_SHA, base_tree=BASE_TREE, schema_sha256=SCHEMA_SHA256,
        test_inventory_sha256=digest(canonical(locations)),
        control_sha256={name: digest((controls / name).read_text(encoding="utf-8").encode())
                        for name in sorted(WRITE_SET)}, **QUALIFIED_INPUTS, **FAILURE_SCOPE)
    extra = {"gate", "completed_gates", "category", "exit_code", "browser", "cleanup"}
    require(isinstance(value, dict) and set(value) == set(expected) | extra
            and all(type(value.get(k)) is type(v) and value.get(k) == v for k, v in expected.items()),
            "Invalid failure identity/provenance/scope")
    name, completed = value["gate"], value["completed_gates"]
    require(name in (*GATES, "prepare", "finish") and isinstance(completed, list)
            and completed == list(GATES)[:len(completed)]
            and (name == "prepare" and not completed or name == "finish" and completed == list(GATES)
                 or name in GATES and completed == list(GATES)[:list(GATES).index(name)]), "Invalid failure gate prefix")
    require(value["category"] in ("exit", "timeout", "validation", "control_error")
            and (value["exit_code"] is None or type(value["exit_code"]) is int
                 and -255 <= value["exit_code"] <= 255 and value["exit_code"] != 0)
            and (value["category"] == "exit") == (value["exit_code"] is not None), "Invalid failure category/exit")
    require(value["cleanup"] in (dict(private_absent=False), dict(private_absent=True))
            and type(value["cleanup"]["private_absent"]) is bool, "Invalid failure cleanup")
    browser = value["browser"]
    require(isinstance(browser, dict) and set(browser) == {"report", "diagnostics", "browsers"}
            and browser["report"] in ("valid", "unavailable", "rejected", "truncated")
            and isinstance(browser["diagnostics"], list) and len(browser["diagnostics"]) <= 128
            and isinstance(browser["browsers"], dict) and set(browser["browsers"]) == set(BROWSER_PROJECTS),
            "Invalid failure browser schema")
    total = 0
    for counts in browser["browsers"].values():
        require(isinstance(counts, dict) and set(counts) == set(BROWSER_STATUSES)
                and all(type(n) is int and 0 <= n <= 4096 for n in counts.values()), "Invalid browser counters")
        total += sum(counts.values())
    require(total <= 4096, "Browser counter bound")
    seen = set()
    for row in browser["diagnostics"]:
        require(isinstance(row, dict) and set(row) == {"file", "line", "project", "status", "results"}
                and isinstance(row["file"], str) and row["file"] in locations
                and type(row["line"]) is int and row["line"] in locations[row["file"]]
                and row["project"] in BROWSER_PROJECTS and row["status"] == "unexpected"
                and isinstance(row["results"], list) and row["results"] == sorted(set(row["results"]))
                and all(s in RESULT_STATUSES for s in row["results"])
                and browser["browsers"][row["project"]]["unexpected"] > 0, "Unattested failure diagnostic")
        seen.add(canonical(row))
    require(len(seen) == len(browser["diagnostics"]), "Duplicate failure diagnostic")
    require(browser["report"] == "valid" or not browser["diagnostics"] and total == 0,
            "Invalid unavailable browser evidence")
    require(name == "fixtures" or browser["report"] == "unavailable", "Browser evidence outside fixture gate")
    require(len(canonical(value)) <= FAILURE_LIMIT, "Failure receipt exceeds bound")


def record_failure(name, error):
    workspace, sha = controls_preflight()
    private = workspace_temp() / "fleet-frontend-private"
    preflight_receipt = json.loads(bounded_file(private, "preflight.json"))
    require(preflight_receipt == dict(workflow_sha=sha, run_id=os.environ["GITHUB_RUN_ID"],
        attempt=os.environ["GITHUB_RUN_ATTEMPT"], public=True), "Failure preflight identity mismatch")
    completed = []
    if name != "prepare":
        state = json.loads(bounded_file(private, "state.json"))
        require(state["workflow_sha"] == sha and qualified_inputs(state) == QUALIFIED_INPUTS, "Failure state mismatch")
        completed = state["gates"]
    locations = attested_test_locations(workspace / "controls")
    browser = safe_browser_failure(None, locations)
    if name == "fixtures" and (private / "browser.json").exists():
        try:
            if (private / "browser.json").stat().st_size > 4 * 1024 ** 2:
                browser["report"] = "truncated"
            else:
                browser = safe_browser_failure(bounded_file(private, "browser.json", limit=4 * 1024 ** 2), locations)
        except (OSError, ValueError):
            browser["report"] = "rejected"
    if isinstance(error, GateFailure):
        category = error.category
    elif isinstance(error, (TimeoutError, subprocess.TimeoutExpired)):
        category = "timeout"
    else:
        category = "validation" if isinstance(error, ValueError) else "control_error"
    code = error.code if isinstance(error, GateFailure) else None
    value = dict(version=1, kind="safe_frontend_failure", status="failure", repository=REPOSITORY,
        branch=BRANCH, workflow_path=WORKFLOW, workflow_sha=sha, run_id=int(os.environ["GITHUB_RUN_ID"]),
        run_attempt=int(os.environ["GITHUB_RUN_ATTEMPT"]), source_sha=SOURCE_SHA, source_tree=SOURCE_TREE,
        source_parents=SOURCE_PARENTS, base_sha=BASE_SHA, base_tree=BASE_TREE, schema_sha256=SCHEMA_SHA256,
        test_inventory_sha256=digest(canonical(locations)),
        control_sha256={p: digest(bounded_file(workspace / "controls", p)) for p in sorted(WRITE_SET)},
        gate=name, completed_gates=completed, category=category, exit_code=code,
        browser=browser, cleanup=dict(private_absent=False),
        **QUALIFIED_INPUTS, **FAILURE_SCOPE)
    validate_failure(value, workflow_sha=sha, run_id=value["run_id"], attempt=value["run_attempt"], locations=locations)
    with (private / "failure-pending.json").open("xb") as pending:
        pending.write(canonical(value))


def capture_paths(source):
    script = (source / "frontend/scripts/capture-screenshots.mjs").read_text()
    block = script.split("const coreScreens = [", 1)[1].split("\n]", 1)[0]
    names = re.findall(r"\['([a-z0-9-]+\.png)',", block)
    require(len(names) == len(set(names)) and len(names) >= 45, "Unexpected pinned screenshot fixture list")
    return sorted(f"docs/assets/screens/{viewport}/{name}" for viewport in VIEWPORTS for name in names)


def require_fixture_environment():
    forbidden = ("SDLC_LIVE_QA", "SDLC_QA_SESSION_FILE", "PLAYWRIGHT_BASE_URL", "SCREENSHOT_BASE_URL",
                 "SCREENSHOT_PREVIEW_PORT", "E2E_BASE_URL", "SDLC_LIVE_QA_URL", "NODE_OPTIONS",
                 "VITEST_POOL", "VITEST_MAX_THREADS", "VITEST_MIN_THREADS", "VITE_API_BASE_URL")
    require(not any(os.environ.get(name) for name in forbidden), "External/live or test override forbidden")


def prepare():
    workspace, sha = controls_preflight()
    private = workspace_temp() / "fleet-frontend-private"
    receipt = json.loads(bounded_file(private, "preflight.json"))
    require(receipt == dict(workflow_sha=sha, run_id=os.environ["GITHUB_RUN_ID"],
                            attempt=os.environ["GITHUB_RUN_ATTEMPT"], public=True), "Preflight receipt mismatch")
    require_fixture_environment()
    source, base = workspace / "fleet-control", workspace / "services-base"
    checkout(source, SOURCE_SHA, REPOSITORY, clean=True)
    qualify_source(source)
    checkout(base, BASE_SHA, "FerrPOINT/services-base", clean=True)
    require(git(base, "rev-parse", "HEAD^{tree}").decode().strip() == BASE_TREE, "Actual Base tree mismatch")
    require((source / ".base-revision").read_text().strip() == BASE_SHA, "Source Base pin mismatch")
    require(bounded_command(["node", "--version"]).decode().strip() == "v" + NODE, "Node version mismatch")
    require(bounded_command(["pnpm", "--version"]).decode().strip() == PNPM, "pnpm version mismatch")
    require(shutil.disk_usage(workspace_temp()).free >= 5 * 1024 ** 3, "Disposable hosted CI requires 5 GiB free")
    package = json.loads((source / "frontend/package.json").read_bytes())
    require(package["packageManager"] == "pnpm@" + PNPM
            and package["dependencies"]["@sdlc/ui"] == "file:../../services-base/frontend"
            and package["scripts"]["postinstall"] == "openapi-typescript ../openapi/openapi.json -o src/api/generated.ts",
            "Frontend dependency/generation convention mismatch")
    require(git(source, "check-ignore", GENERATED).decode().strip() == GENERATED
            and not git(source, "ls-files", "--", GENERATED), "Generated client must remain ignored")
    state = dict(workflow_sha=sha, source=tracked_inventory(source, SOURCE_SHA),
                 base=tracked_inventory(base, BASE_SHA), gates=[], screens=capture_paths(source))
    require(qualified_inputs(state) == QUALIFIED_INPUTS, "Qualified dependency/source inventory mismatch")
    save_state(private, state)
    print("Exact product/Base trees, tools, frozen dependency inputs and fixture environment qualified.")


def save_state(private, state):
    (private / "state.json").write_bytes(canonical(state))


def qualified_inputs(state):
    return dict(source_inventory_sha256=digest(canonical(state["source"])),
                base_inventory_sha256=digest(canonical(state["base"])),
                frontend_lock_sha256=state["source"]["frontend/pnpm-lock.yaml"],
                base_lock_sha256=state["base"]["frontend/pnpm-lock.yaml"])


def load_state():
    workspace, sha = controls_preflight()
    require_fixture_environment()
    private = workspace_temp() / "fleet-frontend-private"
    state = json.loads(bounded_file(private, "state.json"))
    require(state["workflow_sha"] == sha, "Gate state workflow mismatch")
    require(qualified_inputs(state) == QUALIFIED_INPUTS, "Private source inventory changed")
    return workspace, private, state


def verify_parity(workspace, state, *, captured=False, fixtures=False):
    source, base = workspace / "fleet-control", workspace / "services-base"
    checkout(source, SOURCE_SHA, REPOSITORY)
    checkout(base, BASE_SHA, "FerrPOINT/services-base")
    allowed = set(state["screens"]) | {SCREEN_MANIFEST} if captured else set()
    if fixtures or "fixtures" in state.get("gates", []):
        require(SOURCE_FIXTURE_SCREENS <= state["source"].keys(), "Missing pinned fixture screenshots")
        allowed.update(SOURCE_FIXTURE_SCREENS)
    if captured:
        allowed.update(name for name in state["source"] if re.fullmatch(
            r"docs/assets/screens/(375x812|1920x1080|2560x1440)/[a-z0-9-]+\.png", name))
    for root, inventory in ((source, state["source"]), (base, state["base"])):
        git(root, "diff", "--cached", "--quiet")
        changes = git(root, "diff", "--name-only", "HEAD", "--").decode().splitlines()
        require(all(root == source and name in allowed for name in changes), "Unexpected tracked or file-mode change")
        for name, expected in inventory.items():
            if root == source and name in allowed:
                if name in SOURCE_FIXTURE_SCREENS:
                    png_valid(bounded_file(root, name))
                continue
            require(digest(bounded_file(root, name)) == expected, "Tracked source/dependency drift")
        untracked = git(root, "ls-files", "--others", "--exclude-standard", "-z").decode().split("\0")
        require(all(not name or (root == source and name in allowed) for name in untracked),
                "Unexpected untracked source input")
    require(digest(bounded_file(source, "openapi/openapi.json")) == SCHEMA_SHA256, "Generated OpenAPI changed")
    if "generated_sha256" in state:
        require(digest(bounded_file(source, GENERATED)) == state["generated_sha256"], "Postinstall client changed after tests")
    if "build" in state:
        require(build_inventory(source) == state["build"], "Built assets changed during fixture verification")
    return source, base


def build_inventory(source):
    root = source / "frontend/dist"
    require(root.is_dir() and not root.is_symlink(), "Missing fresh frontend build")
    files = {path.relative_to(root).as_posix(): digest(bounded_file(root, path.relative_to(root).as_posix()))
             for path in sorted(root.rglob("*")) if path.is_file()}
    require("index.html" in files and any(name.endswith(".js") for name in files)
            and any(name.endswith(".css") for name in files) and 3 <= len(files) <= 2000, "Incomplete built consumer")
    return dict(build_manifest_sha256=digest(canonical(files)), build_file_count=len(files),
                build_index_sha256=files["index.html"])


def unit_counts(data):
    text = re.sub(r"\x1b\[[0-9;]*[A-Za-z]", "", data.decode("utf-8", errors="replace"))
    files = re.findall(r"Test Files\s+(\d+) passed", text)
    tests = re.findall(r"\bTests\s+(\d+) passed", text)
    require(files and tests and int(files[-1]) > 0 and int(tests[-1]) > 0,
            "No completed passing unit test summary; startup/abort is not evidence")
    summaries = re.findall(r"(?:Test Files|\bTests)\s+[^\n]+", text)
    require(not any(re.search(r"\d+ failed", line) for line in summaries), "Failed unit summary")
    skips = [re.search(r"(\d+) skipped", line) for line in summaries[-2:]]
    return dict(files_passed=int(files[-1]), tests_passed=int(tests[-1]),
                files_skipped=int(skips[0][1]) if skips and skips[0] else 0,
                tests_skipped=int(skips[1][1]) if len(skips) == 2 and skips[1] else 0)


def browser_counts(report):
    require(not report.get("errors"), "Browser reporter has global errors")
    projects = {name: dict(passed=0, flaky=0, skipped=0) for name in ("chromium", "firefox", "webkit")}

    def visit(suite):
        for spec in suite.get("specs", []):
            for test in spec["tests"]:
                project, status = test["projectName"], test["status"]
                require(project in projects and status in ("expected", "flaky", "skipped"), "Failed/unknown browser test")
                results = test.get("results", [])
                if status == "skipped":
                    require(test.get("expectedStatus") == "skipped" or all(r["status"] == "skipped" for r in results),
                            "Unexpected browser skip")
                    projects[project]["skipped"] += 1
                else:
                    require(test.get("expectedStatus") == "passed" and any(r["status"] == "passed" for r in results),
                            "Expected-failure test is not passing evidence")
                    projects[project]["flaky" if status == "flaky" else "passed"] += 1
        for child in suite.get("suites", []):
            visit(child)

    for suite in report["suites"]:
        visit(suite)
    require(all(p["passed"] + p["flaky"] > 0 for p in projects.values()), "Every browser must actually execute fixtures")
    return projects


def gate(name):
    workspace, private, state = load_state()
    require(len(state["gates"]) < len(GATES) and name == list(GATES)[len(state["gates"])], "Gate missing/reordered/repeated")
    source, _ = verify_parity(workspace, state, captured=name == "screens-after")
    env = dict(os.environ, PLAYWRIGHT_JSON_OUTPUT_NAME=str(private / "browser.json"))
    for key in list(env):
        if key in ("GH_TOKEN", "GITHUB_TOKEN", "SERVICES_BASE_TOKEN"):
            del env[key]
    log = private / (name + ".log")
    print("Running frontend gate: " + name, flush=True)
    with log.open("xb") as diagnostics:
        for args in GATES[name]:
            try:
                result = subprocess.run(args, cwd=source / "frontend", env=env,
                                        stdout=diagnostics, stderr=subprocess.STDOUT, timeout=1200)
            except subprocess.TimeoutExpired:
                raise GateFailure("timeout") from None
            if result.returncode != 0:
                raise GateFailure("exit", result.returncode)
    if name == "install":
        state["generated_sha256"] = digest(bounded_file(source, GENERATED))
        installed = source / "frontend/node_modules/@sdlc/ui/package.json"
        require(installed.is_file(), "Missing installed @sdlc/ui")
        require(json.loads(installed.read_bytes()) == json.loads(bounded_file(workspace / "services-base", "frontend/package.json")),
                "Installed Base package differs from actual checkout")
    if name == "unit":
        state["unit"] = unit_counts(bounded_file(private, "unit.log"))
        require(state["unit"] == QUALIFIED_UNIT_COUNTS, "Exact frozen frontend unit baseline did not complete")
    if name == "compat":
        state["compat_main_sha"] = git(source, "rev-parse", "refs/remotes/origin/main").decode().strip()
        state["compat_schema_sha256"] = digest(git(source, "show", "origin/main:openapi/openapi.json"))
    if name == "fixtures":
        state["browsers"] = browser_counts(json.loads(bounded_file(private, "browser.json")))
    if name == "build":
        state["build"] = build_inventory(source)
    verify_parity(workspace, state, captured=name in ("capture", "screens-after"), fixtures=name == "fixtures")
    state["gates"].append(name)
    save_state(private, state)
    print("Passed frontend gate: " + name)


def png_valid(data):
    require(1024 <= len(data) <= MAX_FILE and data.startswith(b"\x89PNG\r\n\x1a\n")
            and data[12:16] == b"IHDR", "Invalid/bounded fixture PNG required")
    width, height = int.from_bytes(data[16:20], "big"), int.from_bytes(data[20:24], "big")
    require(1 <= width <= 8192 and 1 <= height <= 32768, "Unexpected PNG dimensions")
    require(data[24] == 8 and data[25] in (2, 6) and data[26:29] == b"\0\0\0",
            "Unsupported screenshot PNG encoding")
    stride = width * (3 if data[25] == 2 else 4) + 1
    expected = stride * height
    require(expected <= 128 * 1024 ** 2, "Decoded screenshot exceeds size limit")
    decoder, produced, ended = zlib.decompressobj(), 0, False
    offset, chunks = 8, []
    while offset < len(data):
        require(offset + 12 <= len(data), "Truncated PNG chunk")
        size = int.from_bytes(data[offset:offset + 4], "big")
        kind = data[offset + 4:offset + 8]
        require(offset + 12 + size <= len(data) and kind in
                (b"IHDR", b"IDAT", b"IEND", b"PLTE", b"tRNS", b"sRGB", b"gAMA", b"cHRM", b"pHYs",
                 b"iCCP", b"cICP", b"sBIT", b"bKGD"),
                "Malformed PNG or non-fixture metadata")
        chunk = data[offset + 4:offset + 8 + size]
        require(zlib.crc32(chunk) == int.from_bytes(data[offset + 8 + size:offset + 12 + size], "big"),
                "Invalid PNG chunk checksum")
        require(kind != b"IHDR" or (offset == 8 and size == 13), "Invalid PNG header")
        require(kind != b"IEND" or (size == 0 and offset + 12 == len(data)), "Invalid PNG end")
        if kind == b"IDAT":
            require(not ended, "Noncontiguous PNG image stream")
            pending = data[offset + 8:offset + 8 + size]
            while True:
                bound = min(64 * 1024, expected + 1 - produced)
                try:
                    pixels = decoder.decompress(pending, bound)
                except zlib.error:
                    raise ValueError("Invalid PNG image stream") from None
                pending = decoder.unconsumed_tail
                require(not decoder.unused_data and produced + len(pixels) <= expected,
                        "PNG image stream size or trailing data mismatch")
                require(all(pixels[i] <= 4 for i in range((-produced) % stride, len(pixels), stride)),
                        "Invalid PNG scanline filter")
                produced += len(pixels)
                if not pending and len(pixels) < bound:
                    break
        elif b"IDAT" in chunks:
            ended = True
        chunks.append(kind)
        offset += 12 + size
    require(chunks[0] == b"IHDR" and chunks.count(b"IHDR") == 1 and b"IDAT" in chunks
            and chunks[-1] == b"IEND" and chunks.count(b"IEND") == 1, "Incomplete fixture PNG")
    require(decoder.eof and produced == expected, "Incomplete PNG scanlines")


def artifact_name(name):
    safe_path(name)
    return name in ("provenance.json", "fixture-summary.json", "SHA256SUMS", "screens/manifest.json") or bool(
        re.fullmatch(r"screens/(375x812|1920x1080|2560x1440)/[a-z0-9-]+\.png", name)
        or re.fullmatch(r"fixtures/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+\.png", name))


def evidence_contract(files):
    require(len(files) <= MAX_MEMBERS and all(artifact_name(name) for name in files), "Artifact allowlist violation")
    require(sum(map(len, files.values())) <= MAX_ARTIFACT and all(len(data) <= MAX_FILE for data in files.values()),
            "Artifact exceeds uncompressed bounds")
    require({"provenance.json", "fixture-summary.json", "screens/manifest.json", "SHA256SUMS"} <= files.keys(),
            "Required evidence missing")
    for name, data in files.items():
        if name.endswith(".png"):
            png_valid(data)
            if name.startswith("screens/"):
                viewport = name.split("/")[1].split("x")
                require(int.from_bytes(data[16:20], "big") == int(viewport[0])
                        and int.from_bytes(data[20:24], "big") >= int(viewport[1]), "Screenshot viewport mismatch")
    fixture_projects = {match[1] for name in files if (match := re.search(
        r"^fixtures/[^/]+-(chromium|firefox|webkit)(?:-retry[0-9]+)?/[^/]+\.png$", name))}
    require(fixture_projects == {"chromium", "firefox", "webkit"}, "Fresh fixture screenshots required from every browser")
    manifest = json.loads(files["screens/manifest.json"])
    expected = set(manifest["files"])
    require(set(manifest) == {"kind", "count", "files", "original_manifest_sha256"}
            and re.fullmatch(r"[0-9a-f]{64}", manifest["original_manifest_sha256"])
            and manifest["kind"] == "fresh_chromium_fixture_screens" and len(expected) == manifest["count"]
            and manifest["count"] >= 135 and expected == {name for name in files if name.startswith("screens/") and name.endswith(".png")}
            and {name.split("/")[1] for name in expected} == set(VIEWPORTS), "Screenshot manifest mismatch")
    expected_sums = "".join(digest(files[name]) + "  " + name + "\n" for name in sorted(files) if name != "SHA256SUMS")
    require(files["SHA256SUMS"] == expected_sums.encode(), "Artifact file checksum mismatch")


def finish():
    workspace, private, state = load_state()
    require(state["gates"] == list(GATES), "Full original frontend pipeline must complete")
    source, base = verify_parity(workspace, state, captured=True)
    manifest = bounded_file(source, SCREEN_MANIFEST).decode()
    rows = re.findall(r"^\|\s*([^|]+?)\s*\|\s*`([^`]+\.png)`\s*\|\s*`([^`]+)`\s*\|$", manifest, re.M)
    names = [name for _, name, _ in rows]
    require(len(names) == len(set(names)) and sorted(names) == state["screens"], "Fresh capture file list differs from pinned script")
    files = {}
    for name in state["screens"]:
        files["screens/" + name.removeprefix("docs/assets/screens/")] = bounded_file(source, name)
    # Only PNGs from the fixture runner; no traces, JSON reports, logs, source, or auth storage.
    results = source / "frontend/test-results"
    if results.is_dir():
        for path in sorted(results.rglob("*.png")):
            name = "fixtures/" + path.relative_to(results).as_posix()
            require(artifact_name(name), "Unexpected fixture screenshot path")
            files[name] = bounded_file(results, path.relative_to(results).as_posix())
    files["screens/manifest.json"] = canonical(dict(kind="fresh_chromium_fixture_screens",
        count=len(state["screens"]), files=sorted(name for name in files if name.startswith("screens/")),
        original_manifest_sha256=digest(bounded_file(source, SCREEN_MANIFEST))))
    files["fixture-summary.json"] = canonical(dict(unit=state["unit"], browsers=state["browsers"], **SCOPE))
    provenance = dict(version=1, repository=REPOSITORY, branch=BRANCH, source_sha=SOURCE_SHA,
        source_tree=SOURCE_TREE, source_parents=SOURCE_PARENTS, base_sha=BASE_SHA, base_tree=BASE_TREE,
        schema_sha256=SCHEMA_SHA256, generated_client_sha256=state["generated_sha256"], node=NODE, pnpm=PNPM,
        workflow_sha=state["workflow_sha"], workflow_path=WORKFLOW, run_id=int(os.environ["GITHUB_RUN_ID"]),
        run_attempt=int(os.environ["GITHUB_RUN_ATTEMPT"]), gates=list(GATES),
        source_inventory_sha256=digest(canonical(state["source"])), base_inventory_sha256=digest(canonical(state["base"])),
        frontend_lock_sha256=state["source"]["frontend/pnpm-lock.yaml"], base_lock_sha256=state["base"]["frontend/pnpm-lock.yaml"],
        compat_main_sha=state["compat_main_sha"], compat_schema_sha256=state["compat_schema_sha256"],
        helper_sha256=digest(bounded_file(workspace / "controls", "scripts/hosted_frontend_gate.py")),
        workflow_sha256=digest(bounded_file(workspace / "controls", WORKFLOW)),
        runner_image=os.environ.get("ImageOS"), runner_image_version=os.environ.get("ImageVersion"), **state["build"], **SCOPE)
    files["provenance.json"] = canonical(provenance)
    files["SHA256SUMS"] = "".join(digest(files[name]) + "  " + name + "\n" for name in sorted(files)).encode()
    evidence_contract(files)
    destination = workspace_temp() / "fleet-frontend-public"
    destination.mkdir(exist_ok=False)
    for name, data in files.items():
        path = destination.joinpath(*safe_path(name).parts)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    print("Full frontend unit/build/fixture gate passed; allowlisted public evidence prepared.")


def validate_readback(run, artifact, payload, *, run_id, attempt, workflow_sha, artifact_id, artifact_digest):
    require(len(payload) <= MAX_ARTIFACT, "Oversized artifact ZIP")
    require(run["id"] == run_id and run["run_attempt"] == attempt and run["status"] == "completed"
            and run["conclusion"] == "success" and run["event"] == "push" and run["head_sha"] == workflow_sha
            and run["head_branch"] == BRANCH and run["path"] == WORKFLOW
            and run["repository"]["full_name"] == REPOSITORY, "Unexpected/unsuccessful workflow run")
    require(artifact["id"] == artifact_id and artifact["expired"] is False
            and artifact["workflow_run"]["id"] == run_id and artifact["workflow_run"]["head_sha"] == workflow_sha
            and artifact["workflow_run"]["head_branch"] == BRANCH
            and artifact["name"] == f"fleet-frontend-chats-{run_id}-{attempt}"
            and artifact["digest"] == "sha256:" + artifact_digest and digest(payload) == artifact_digest,
            "Artifact identity/digest mismatch")
    with zipfile.ZipFile(io.BytesIO(payload)) as archive:
        members = archive.infolist()
        require(len(members) <= MAX_MEMBERS and len({m.filename for m in members}) == len(members)
                and sum(m.file_size for m in members) <= MAX_ARTIFACT
                and all(artifact_name(m.filename) and not m.is_dir() and m.file_size <= MAX_FILE
                        and (m.external_attr >> 16) & 0o170000 in (0, 0o100000)
                        and not m.flag_bits & 1 for m in members), "Unsafe ZIP members/bounds")
        files = {m.filename: archive.read(m) for m in members}
    evidence_contract(files)
    provenance = json.loads(files["provenance.json"])
    expected = dict(version=1, repository=REPOSITORY, branch=BRANCH, source_sha=SOURCE_SHA,
        source_tree=SOURCE_TREE, source_parents=SOURCE_PARENTS, base_sha=BASE_SHA, base_tree=BASE_TREE,
        schema_sha256=SCHEMA_SHA256, node=NODE, pnpm=PNPM, workflow_sha=workflow_sha, workflow_path=WORKFLOW,
        run_id=run_id, run_attempt=attempt, gates=list(GATES), **SCOPE)
    controls = Path(__file__).resolve().parents[1]
    expected.update(helper_sha256=digest((controls / "scripts/hosted_frontend_gate.py").read_text(encoding="utf-8").encode()),
                    workflow_sha256=digest((controls / WORKFLOW).read_text(encoding="utf-8").encode()))
    expected.update(QUALIFIED_INPUTS)
    require(set(provenance) == set(expected) | {"generated_client_sha256", "compat_main_sha", "compat_schema_sha256",
                                              "runner_image", "runner_image_version", "build_manifest_sha256",
                                              "build_file_count", "build_index_sha256"}
            and all(type(provenance.get(k)) is type(v) and provenance.get(k) == v for k, v in expected.items()),
            "Provenance/source/scope mismatch")
    manifest = json.loads(files["screens/manifest.json"])
    require(manifest["files"] == sorted("screens/" + name.removeprefix("docs/assets/screens/")
                                       for name in capture_paths(controls)), "Artifact differs from exact source fixture list")
    for name in ("generated_client_sha256", "source_inventory_sha256", "base_inventory_sha256", "frontend_lock_sha256",
                 "base_lock_sha256", "compat_schema_sha256", "build_manifest_sha256", "build_index_sha256"):
        require(re.fullmatch(r"[0-9a-f]{64}", provenance.get(name, "")), "Missing provenance hash")
    require(type(provenance["build_file_count"]) is int and 3 <= provenance["build_file_count"] <= 2000,
            "Missing actual frontend build inventory")
    require(re.fullmatch(r"[0-9a-f]{40}", provenance.get("compat_main_sha", "")), "Missing fresh main identity")
    summary = json.loads(files["fixture-summary.json"])
    require(set(summary) == set(SCOPE) | {"unit", "browsers"}
            and summary["unit"] == QUALIFIED_UNIT_COUNTS
            and set(summary["unit"]) == {"tests_passed", "files_passed", "tests_skipped", "files_skipped"}
            and all(type(n) is int and n >= 0 for n in summary["unit"].values())
            and all(set(p) == {"passed", "flaky", "skipped"} and all(type(n) is int and n >= 0 for n in p.values())
                    for p in summary["browsers"].values())
            and all(summary.get(k) is v for k, v in SCOPE.items()) and summary["unit"]["tests_passed"] > 0
            and summary["unit"]["files_passed"] > 0 and set(summary["browsers"]) == {"chromium", "firefox", "webkit"}
            and all(p["passed"] + p["flaky"] > 0 and p["skipped"] >= 0 for p in summary["browsers"].values()),
            "Missing actual unit/three-browser evidence")
    return files


def validate_failure_readback(run, artifact, payload, *, run_id, attempt, workflow_sha, artifact_id, artifact_digest):
    require(len(payload) <= 2 * FAILURE_LIMIT, "Oversized failure ZIP")
    require(run["id"] == run_id and run["run_attempt"] == attempt and run["status"] == "completed"
            and run["conclusion"] == "failure" and run["event"] == "push" and run["head_sha"] == workflow_sha
            and run["head_branch"] == BRANCH and run["path"] == WORKFLOW
            and run["repository"]["full_name"] == REPOSITORY, "Unexpected failure workflow run")
    require(artifact["id"] == artifact_id and artifact["expired"] is False
            and artifact["workflow_run"]["id"] == run_id and artifact["workflow_run"]["head_sha"] == workflow_sha
            and artifact["workflow_run"]["head_branch"] == BRANCH
            and artifact["name"] == f"fleet-frontend-failure-chats-{run_id}-{attempt}"
            and artifact["digest"] == "sha256:" + artifact_digest and digest(payload) == artifact_digest,
            "Failure artifact identity/digest mismatch")
    with zipfile.ZipFile(io.BytesIO(payload)) as archive:
        members = archive.infolist()
        require(len(members) == 2 and {m.filename for m in members} == {FAILURE_FILE, "SHA256SUMS"}
                and all(not m.is_dir() and m.file_size <= FAILURE_LIMIT and not m.flag_bits & 1
                        and (m.external_attr >> 16) & 0o170000 in (0, 0o100000) for m in members),
                "Unsafe failure ZIP members")
        files = {m.filename: archive.read(m) for m in members}
    require(files["SHA256SUMS"] == (digest(files[FAILURE_FILE]) + "  " + FAILURE_FILE + "\n").encode(),
            "Failure checksum mismatch")
    value = json.loads(files[FAILURE_FILE])
    validate_failure(value, workflow_sha=workflow_sha, run_id=run_id, attempt=attempt,
                     locations=attested_test_locations(Path(__file__).resolve().parents[1]))
    require(files[FAILURE_FILE] == canonical(value), "Noncanonical failure receipt")
    return files


def readback(args):
    require(re.fullmatch(r"[0-9a-f]{40}", args.workflow_sha)
            and re.fullmatch(r"[0-9a-f]{64}", args.artifact_digest)
            and min(args.run_id, args.attempt, args.artifact_id) > 0, "Explicit reviewed artifact identity required")
    endpoint = "repos/" + REPOSITORY + "/actions/"
    run = json.loads(bounded_command(["gh", "api", "--method", "GET", endpoint + f"runs/{args.run_id}/attempts/{args.attempt}"]))
    path = endpoint + f"artifacts/{args.artifact_id}"
    artifact = json.loads(bounded_command(["gh", "api", "--method", "GET", path]))
    failure = args.mode == "readback-failure"
    limit = 2 * FAILURE_LIMIT if failure else MAX_ARTIFACT
    require(artifact["id"] == args.artifact_id and 0 < artifact["size_in_bytes"] <= limit, "Artifact ID/size mismatch")
    payload = bounded_command(["gh", "api", "--method", "GET", path + "/zip"], limit=limit)
    validator = validate_failure_readback if failure else validate_readback
    files = validator(run, artifact, payload, run_id=args.run_id, attempt=args.attempt,
        workflow_sha=args.workflow_sha, artifact_id=args.artifact_id, artifact_digest=args.artifact_digest)
    args.output.mkdir(parents=True, exist_ok=False)
    for name, data in files.items():
        target = args.output.joinpath(*safe_path(name).parts)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    print(json.dumps(dict(state="verified_safe_frontend_failure" if failure else "verified_frontend_fixture_artifact",
        output=str(args.output.resolve()), **(FAILURE_SCOPE if failure else SCOPE))))


def cleanup():
    workspace, sha = hosted_identity()
    target = workspace_temp() / "fleet-frontend-private"
    require(target.resolve() == target and not target.is_symlink(), "Private cleanup alias forbidden")
    value, locations = None, None
    try:
        if (target / "failure-pending.json").exists():
            locations = attested_test_locations(workspace / "controls")
            value = json.loads(bounded_file(target, "failure-pending.json", limit=FAILURE_LIMIT))
            validate_failure(value, workflow_sha=sha, run_id=int(os.environ["GITHUB_RUN_ID"]),
                             attempt=int(os.environ["GITHUB_RUN_ATTEMPT"]), locations=locations)
    finally:
        try:
            if target.exists():
                shutil.rmtree(target)
        finally:
            if value is not None:
                value["cleanup"] = dict(private_absent=not target.exists() and not target.is_symlink())
                validate_failure(value, workflow_sha=sha, run_id=int(os.environ["GITHUB_RUN_ID"]),
                                 attempt=int(os.environ["GITHUB_RUN_ATTEMPT"]), locations=locations)
                destination = workspace_temp() / "fleet-frontend-failure"
                destination.mkdir(exist_ok=False)
                with (destination / FAILURE_FILE).open("xb") as output:
                    output.write(canonical(value))
                with (destination / "SHA256SUMS").open("xb") as output:
                    output.write((digest(canonical(value)) + "  " + FAILURE_FILE + "\n").encode())


def main():
    parser = argparse.ArgumentParser(__doc__)
    modes = parser.add_subparsers(dest="mode", required=True)
    for name in ("preflight", "prepare", "finish", "cleanup"):
        modes.add_parser(name)
    check = modes.add_parser("gate")
    check.add_argument("name", choices=list(GATES))
    for mode in ("readback", "readback-failure"):
        read = modes.add_parser(mode)
        for name in ("run-id", "attempt", "artifact-id"):
            read.add_argument("--" + name, type=int, required=True)
        for name in ("workflow-sha", "artifact-digest"):
            read.add_argument("--" + name, required=True)
        read.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.mode == "gate":
            gate(args.name)
        elif args.mode in ("readback", "readback-failure"):
            readback(args)
        else:
            globals()[args.mode]()
    except (ValueError, KeyError, TypeError, OSError, TimeoutError, RecursionError, subprocess.SubprocessError) as error:
        if args.mode in ("gate", "prepare", "finish"):
            try:
                record_failure(args.name if args.mode == "gate" else args.mode, error)
            except (ValueError, KeyError, TypeError, OSError, TimeoutError, RecursionError, subprocess.SubprocessError):
                print("Safe failure receipt unavailable; no acceptance claimed.", flush=True)
        raise


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, TypeError, OSError, TimeoutError, RecursionError, subprocess.SubprocessError, zipfile.BadZipFile) as error:
        raise SystemExit("Frontend control failed [" + safe_failure_hint(error)
                         + "]; no acceptance claimed. Private diagnostics are withheld.") from None
