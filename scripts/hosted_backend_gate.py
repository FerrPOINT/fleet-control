"""Hosted full backend gate, derived from reviewed codegen controls and QA42."""
from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import signal
import subprocess
import tarfile
import zipfile

REPOSITORY = "FerrPOINT/fleet-control"
BRANCH = "build-only/fleet-backend-98d950e-20261009"
SOURCE_SHA = "98d950eb618071de7647e56626ad991259058be2"
BASE_SHA = "19a7a381ae6dbea61a643bb96189e483fa64df5c"
AUTH_SHA = "01388dfb43332cbe5837fd5e1fadccf09cb8886d"
OPENAPI_SHA = "b074c77295f7ad89912e3667124545ab66f6727184257d1f72030b4417c87f82"
SWAGGER_SHA = "481244d0812097b11fbaeef79f71d942b171617f9c9f9514e63acbe13e71ccdc"
WORKFLOW = ".github/workflows/backend-build-only.yml"
HELPER = "scripts/hosted_backend_gate.py"
GATE = "scripts/hosted-backend/gate.sh"
INIT = "scripts/hosted-backend/init.sql"
INVENTORY = "scripts/hosted-backend/test-inventory.json"
WRITE_SET = {WORKFLOW, HELPER, GATE, INIT, INVENTORY, "scripts/tests/test_hosted_backend_gate.py"}
ARTIFACT_FILES = {"report.json", "provenance.json", "SHA256SUMS"}
FORBIDDEN = {".local", "target", "node_modules", ".venv", ".git", "backups", ".env"}
GATES = ("preflight", "fmt", "check", "clippy", "auth_binary", "runtime_inventory", "real_auth",
         "api2", "credentials_unit", "credentials_pg", "foundation", "workspace", "lineage10",
         "central_profile", "message_order", "chats_directory", "runtime_approval_events",
         "credentials_migration", "hermes_wire", "hermes_journal", "hermes_acceptance",
         "hermes_readback", "hermes_terminal", "journal_migration", "control_api",
         "lookup_header", "lookup_openapi", "lookup_route", "control_wire", "recovery_wire", "sse_wire",
         "runtime_controls", "runtime_terminal", "runtime_pinned_recovery", "runtime_unknown_recovery",
         "runtime_recovery_races", "runtime_stream_bounds", "controls_migration", "approval_snapshot",
         "targeted_approval", "approval_recovery", "approval_compat", "time_migration",
         "migration_smoke", "openapi", "compiled_source_parity")
DATABASES = ("fleet_foundation_test", "fleet_migration_test", "fleet_message_order_test",
             "fleet_chats_directory_test", "fleet_migration_smoke", "fleet_runtime_approval_events_test",
             "fleet_credential_migration_test", "fleet_real_auth_test", "fleet_dispatch_journal_migration_test",
             "fleet_runtime_controls_migration_test", "fleet_hermes_time_migration_test")
URLS = {
    "FLEET_TEST_DATABASE_URL": "fleet_foundation_test",
    "FLEET_MIGRATION_TEST_DATABASE_URL": "fleet_migration_test",
    "FLEET_MESSAGE_ORDER_TEST_DATABASE_URL": "fleet_message_order_test",
    "FLEET_CHATS_DIRECTORY_TEST_DATABASE_URL": "fleet_chats_directory_test",
    "FLEET_RUNTIME_APPROVAL_EVENTS_TEST_DATABASE_URL": "fleet_runtime_approval_events_test",
    "FLEET_CREDENTIAL_MIGRATION_TEST_DATABASE_URL": "fleet_credential_migration_test",
    "DATABASE_URL": "fleet_migration_smoke", "FLEET_REAL_AUTH_TEST_DATABASE_URL": "fleet_real_auth_test",
    "FLEET_DISPATCH_JOURNAL_MIGRATION_TEST_DATABASE_URL": "fleet_dispatch_journal_migration_test",
    "FLEET_RUNTIME_CONTROL_MIGRATION_TEST_DATABASE_URL": "fleet_runtime_controls_migration_test",
    "FLEET_HERMES_TIME_MIGRATION_TEST_DATABASE_URL": "fleet_hermes_time_migration_test",
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return (json.dumps(value, sort_keys=True, indent=2) + "\n").encode()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def command(args, **kwargs):
    result = subprocess.run(args, capture_output=True, timeout=300, **kwargs)
    require(result.returncode == 0, "Command failed; backend gate withheld")
    return result.stdout


def git(root, *args):
    return command(["git", "--no-replace-objects", "-C", str(root), *args])


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
    git(controls, "merge-base", "--is-ancestor", SOURCE_SHA, workflow_sha)
    validate_delta(git(controls, "diff", "--no-renames", "--name-status", SOURCE_SHA, workflow_sha).decode())
    require((controls / ".base-revision").read_text().strip() == BASE_SHA, "Base pin drift")
    require(digest(git(controls, "show", SOURCE_SHA + ":openapi/openapi.json")) == OPENAPI_SHA, "Schema pin drift")
    return workspace, controls, workflow_sha


def safe_member(name, roots):
    path = PurePosixPath(name)
    require(bool(name) and not path.is_absolute() and ".." not in path.parts and "\\" not in name
            and not re.search(r"[\x00-\x20\x7f:]", name) and str(path) == name
            and not FORBIDDEN.intersection(part.lower() for part in path.parts)
            and any(name == root or name.startswith(root + "/") for root in roots), "Unsafe export path")
    return path


def export(root, commit, destination, roots):
    data = git(root, "archive", "--format=tar", commit, "--", *roots)
    destination.mkdir(parents=True, exist_ok=False)
    with tarfile.open(fileobj=io.BytesIO(data)) as archive:
        for member in archive:
            path = safe_member(member.name.rstrip("/") if member.isdir() else member.name, roots)
            target = destination.joinpath(*path.parts)
            require(member.isdir() or member.isfile(), "Source links/submodules forbidden")
            require(member.size <= 128 * 1024 ** 2, "Oversized source member")
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                with archive.extractfile(member) as source, target.open("xb") as output:
                    shutil.copyfileobj(source, output)


def inventory(root):
    require(not any(path.is_symlink() for path in root.rglob("*")), "Unexpected source link")
    return {path.relative_to(root).as_posix(): digest(path.read_bytes())
            for path in sorted(root.rglob("*")) if path.is_file()}


def reviewed_inventory(controls):
    value = json.loads((controls / INVENTORY).read_bytes())
    require(value["source_commit"] == SOURCE_SHA and value["executed"] is False, "Inventory pin drift")
    require(len(value["ignored"]) == 135 and value["default_foundation_ignored"] == 115
            and len(value["groups"]["runtime_controls"]) == 30
            and len(value["groups"]["runtime_terminal"]) == 14, "Ignored coverage weakened")
    require(len({(item["package"], item["target_kind"], item["target"], item["name"])
                 for item in value["ignored"]}) == 135, "Duplicate ignored identities")
    return value


def validate_source_inventory(source, reviewed):
    actual = {path.relative_to(source).as_posix(): digest(path.read_bytes())
              for path in sorted((source / "backend").rglob("*.rs"))}
    require(actual == reviewed["rust_source_sha256"], "Unreviewed Rust declaration/source drift")


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
                and sum(row[2] for row in summaries) == 135, "Workspace result/ignored totals drift")
    else:
        expected = Counter(reviewed["groups"][stage])
        ignored = 115 if stage == "foundation" else 0
        require(bool(expected) and actual == expected, "Focused exact test names missing/duplicate/unexpected")
        require(summaries == [(sum(expected.values()), 0, ignored)], "Focused result count/ignored drift")
    return dict(passed=sum(actual.values()), failed=0,
                ignored=sum(row[2] for row in summaries), tests=sorted(actual.elements()))


def verify_runtime_inventory(ordinary, ignored, reviewed):
    expected = Counter(item["name"] for item in reviewed["ignored"])
    require(listed_names(ignored) == expected and sum(expected.values()) == 135, "Compiler ignored inventory drift")
    require(bool(listed_names(ordinary) - expected), "Zero workspace default selection")
    return dict(ignored=135, ignored_names_sha256=digest(canonical(sorted(expected.elements()))),
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


def stop_owned_group(process):
    # The gate starts its own session; never target the runner/service process groups.
    if process is not None:
        for sig in (signal.SIGTERM, signal.SIGKILL):
            try:
                os.killpg(process.pid, sig)
            except ProcessLookupError:
                break
        process.wait(timeout=30)


def remove_scratch(root, temporary, identity):
    require(root.parent == temporary and root.name == "fleet-backend-98d950e"
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


def execute():
    workspace, controls, workflow_sha = preflight()
    checkouts = ((workspace / "fleet-control", SOURCE_SHA), (workspace / "services-base", BASE_SHA),
                 (workspace / "base-auth-source", AUTH_SHA))
    for checkout, commit in checkouts:
        clean_head(checkout, commit)
    require(command(["rustc", "--version"]).decode().split()[1] == "1.88.0", "Rust drift")
    require((checkouts[0][0] / ".base-revision").read_text().strip() == BASE_SHA, "Source Base drift")
    reviewed = reviewed_inventory(controls)
    temporary = Path(os.environ["RUNNER_TEMP"]).resolve()
    require(temporary.is_dir(), "Hosted temporary root missing")
    resources = resource_guard(temporary)
    root, evidence = temporary / "fleet-backend-98d950e", temporary / "fleet-backend-evidence"
    evidence.mkdir(exist_ok=False)
    root.mkdir(exist_ok=False)
    identity = dict(workflow_sha=workflow_sha, run_id=int(os.environ["GITHUB_RUN_ID"]),
                    run_attempt=int(os.environ["GITHUB_RUN_ATTEMPT"]))
    (root / "owner.json").write_bytes(canonical(identity))
    owned_dbs, process, before, focused, rows = [], None, {}, {}, []
    runtime_inventory, auth_binary_sha, failed_stage = {}, None, "preflight"
    cleanup, success = dict(scratch=False, synthetic_databases=False), False
    try:
        export(checkouts[0][0], SOURCE_SHA, root / "src/fleet-control", ("backend", ".base-revision", "openapi", "docs/TESTING.md"))
        export(checkouts[1][0], BASE_SHA, root / "src/services-base", ("crates", "Cargo.toml", "Cargo.lock", "LICENSE"))
        export(checkouts[2][0], AUTH_SHA, root / "src/base-auth-source", ("crates", "Cargo.toml", "Cargo.lock", "LICENSE", "frontend/src/lib/theme-preference.js"))
        before = inventory(root / "src")
        validate_source_inventory(root / "src/fleet-control", reviewed)
        for name in ("tmp", "cargo", "target", "private", "expected"):
            (root / name).mkdir()
        for name, tests in reviewed["groups"].items():
            require(bool(tests) and len(tests) == len(set(tests)), "Empty/duplicate expectation")
            (root / "expected" / (name + ".txt")).write_text("\n".join(tests) + "\n", newline="\n")
        (root / "sources.sha256").write_text("".join(value + "  " + name + "\n" for name, value in before.items()), newline="\n")
        command(["curl", "--fail", "--location", "--proto", "=https", "--proto-redir", "=https", "--retry", "0",
                 "https://github.com/swagger-api/swagger-ui/archive/refs/tags/v5.17.14.zip", "--output", str(root / "swagger.zip")])
        require(digest((root / "swagger.zip").read_bytes()) == SWAGGER_SHA, "Swagger hash mismatch")
        require(database_catalog() == ["fleet_foundation_test", "postgres"], "Service not a fresh owned PostgreSQL")
        require(psql("SHOW server_version_num").strip() == b"170006", "PostgreSQL version drift")
        require(psql("SELECT current_user").strip() == b"fleet_test", "Synthetic PG owner drift")
        require(psql("SELECT count(*) FROM pg_tables WHERE schemaname='public'",
                     database="fleet_foundation_test").strip() == b"0", "Initial fixture DB not empty")
        require(psql("SELECT rolname FROM pg_roles WHERE rolname='fleet_approval_events_test'").strip() == b"", "Fixture role already exists")
        owned_dbs = list(DATABASES)
        psql(file=controls / INIT)
        require(database_catalog() == sorted([*DATABASES, "postgres"]), "Synthetic DB initialization incomplete")
        environment = {key: os.environ[key] for key in ("PATH", "HOME", "RUSTUP_HOME", "RUNNER_TRACKING_ID") if key in os.environ}
        environment.update(database_environment())
        environment.update(QA_ROOT=str(root), QA_OUTPUT=str(root / "private"), QA_EXPECTED=str(root / "expected"),
                           QA_SOURCES=str(root / "sources.sha256"), QA_HELPER=str(controls / HELPER),
                           QA_INVENTORY=str(controls / INVENTORY), QA_SOURCE_COMMIT=SOURCE_SHA, QA_AUTH_SOURCE_COMMIT=AUTH_SHA,
                           RUSTUP_TOOLCHAIN="1.88.0", CARGO_HOME=str(root / "cargo"), CARGO_TARGET_DIR=str(root / "target"),
                           TMPDIR=str(root / "tmp"), CARGO_BUILD_JOBS="1", CARGO_INCREMENTAL="0",
                           CARGO_PROFILE_DEV_DEBUG="0", CARGO_PROFILE_TEST_DEBUG="0", LC_ALL="C",
                           SWAGGER_UI_DOWNLOAD_URL=(root / "swagger.zip").as_uri())
        with (root / "private/driver.log").open("xb") as diagnostics:
            process = subprocess.Popen(["bash", str(controls / GATE)], env=environment,
                                       stdout=diagnostics, stderr=diagnostics, start_new_session=True)
            code = process.wait(timeout=6600)
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
        for checkout, commit in checkouts:
            clean_head(checkout, commit)
        clean_head(controls, workflow_sha)
        auth_binary_sha = (root / "private/auth-binary.sha256").read_text().split()[0]
        require(bool(re.fullmatch(r"[0-9a-f]{64}", auth_binary_sha)), "Missing fresh Auth binary hash")
        success = True
    except Exception:
        # Never print raw Cargo/Base/test diagnostics, arguments or exception text.
        if (root / "private/gates.tsv").exists():
            rows = [line.split("\t") for line in (root / "private/gates.tsv").read_text().splitlines()]
            failed_stage = failure_stage(rows)
    finally:
        try:
            stop_owned_group(process)
            if owned_dbs:
                drop_databases(owned_dbs)
                cleanup["synthetic_databases"] = True
            else:
                cleanup["synthetic_databases"] = "not_initialized"
            remove_scratch(root, temporary, identity)
            cleanup["scratch"] = True
        except Exception:
            success, failed_stage = False, "cleanup"
    safe_rows = [{"stage": name, "status": state} for name, state in rows
                 if name in GATES and state in ("passed", "failed")]
    report = dict(version=1, backend_quality_gate=success, all_quality_gate=False, sdlc_acceptance=False,
                  status="success" if success else "failure", failed_stage=None if success else failed_stage,
                  gates=safe_rows, focused=focused, runtime_inventory=runtime_inventory,
                  ignored_required=135, foundation_ignored=115, resources=resources, cleanup=cleanup,
                  service_disposal="GitHub-managed ephemeral service, platform cleanup after job",
                  local_docker_or_native_guard_waiver=False, private_diagnostics_uploaded=False)
    provenance = dict(version=1, repository=REPOSITORY, branch=BRANCH, source_sha=SOURCE_SHA, base_sha=BASE_SHA,
                      auth_sha=AUTH_SHA, source_tree=git(checkouts[0][0], "rev-parse", "HEAD^{tree}").decode().strip(),
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
        (evidence / name).write_bytes(canonical(value))
    (evidence / "SHA256SUMS").write_text("".join(digest((evidence / name).read_bytes()) + "  " + name + "\n"
                                              for name in ("report.json", "provenance.json")), newline="\n")
    print(json.dumps(dict(state="backend_quality_gate_passed" if success else "backend_quality_gate_failed",
                          failed_stage=None if success else failed_stage, private_logs_uploaded=False,
                          all_quality_gate=False, sdlc_acceptance=False)))
    return 0 if success else 1


def cleanup_fallback():
    workspace, workflow_sha = hosted_identity()
    temporary = Path(os.environ["RUNNER_TEMP"]).resolve()
    root = temporary / "fleet-backend-98d950e"
    if root.exists():
        identity = dict(workflow_sha=workflow_sha, run_id=int(os.environ["GITHUB_RUN_ID"]),
                        run_attempt=int(os.environ["GITHUB_RUN_ATTEMPT"]))
        require(not root.is_symlink() and json.loads((root / "owner.json").read_bytes()) == identity,
                "Fallback ownership mismatch")
        # A killed step cannot publish PASS; platform disposal owns its service.
        remove_scratch(root, temporary, identity)


def validate_readback(run, artifact, payload, *, run_id, attempt, workflow_sha, artifact_digest):
    require(run["id"] == run_id and run["run_attempt"] == attempt and run["status"] == "completed"
            and run["conclusion"] == "success" and run["event"] == "push" and run["head_sha"] == workflow_sha
            and run["head_branch"] == BRANCH and run["path"] == WORKFLOW
            and run["repository"]["full_name"] == REPOSITORY, "Unexpected/unsuccessful workflow identity")
    require(not artifact["expired"] and artifact["workflow_run"]["id"] == run_id
            and artifact["workflow_run"]["head_sha"] == workflow_sha
            and artifact["name"] == f"fleet-backend-98d950e-{run_id}-{attempt}"
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
                    control_sha256={name: digest((controls / name).read_bytes()) for name in sorted(WRITE_SET)})
    require(all(type(provenance.get(key)) is type(value) and provenance.get(key) == value
                for key, value in expected.items()), "Provenance mismatch")
    require(report["backend_quality_gate"] is True and report["all_quality_gate"] is False
            and report["sdlc_acceptance"] is False and report["status"] == "success"
            and report["gates"] == [dict(stage=name, status="passed") for name in GATES]
            and report["cleanup"] == dict(scratch=True, synthetic_databases=True)
            and report["ignored_required"] == 135 and report["foundation_ignored"] == 115
            and report["runtime_inventory"]["ignored"] == 135, "Incomplete backend gate receipt")
    reviewed = reviewed_inventory(controls)
    require(set(report["focused"]) == set(reviewed["groups"]) | {"workspace"}, "Focused stage omission")
    for stage, names in reviewed["groups"].items():
        actual = report["focused"][stage]
        require(actual == dict(passed=len(names), failed=0, ignored=115 if stage == "foundation" else 0,
                               tests=sorted(names)), "Focused test receipt mismatch")
    ws = report["focused"]["workspace"]
    require(ws["passed"] > 0 and ws["failed"] == 0 and ws["ignored"] == 135
            and len(ws["tests"]) == ws["passed"] == report["runtime_inventory"]["listed_default_count"],
            "Empty/failed workspace receipt")
    require(report["runtime_inventory"]["ignored_names_sha256"] == digest(canonical(sorted(
        item["name"] for item in reviewed["ignored"]))), "Ignored runtime identity receipt drift")
    require(bool(re.fullmatch(r"[0-9a-f]{64}", provenance.get("auth_binary_sha256", ""))), "Missing Auth binary provenance")
    expected_sums = "".join(digest(files[name]) + "  " + name + "\n" for name in ("report.json", "provenance.json"))
    require(files["SHA256SUMS"] == expected_sums.encode(), "Checksum manifest mismatch")
    return files


def readback(args):
    require(bool(re.fullmatch(r"[0-9a-f]{40}", args.workflow_sha))
            and bool(re.fullmatch(r"[0-9a-f]{64}", args.artifact_digest))
            and min(args.run_id, args.attempt, args.artifact_id) > 0, "Explicit reviewed artifact identity required")
    endpoint = "repos/" + REPOSITORY + "/actions/"
    run = json.loads(command(["gh", "api", "--method", "GET", endpoint + f"runs/{args.run_id}/attempts/{args.attempt}"]))
    artifact_url = endpoint + f"artifacts/{args.artifact_id}"
    artifact = json.loads(command(["gh", "api", "--method", "GET", artifact_url]))
    require(artifact["id"] == args.artifact_id and artifact["size_in_bytes"] <= 8 * 1024 ** 2, "Unexpected artifact size/ID")
    payload = command(["gh", "api", "--method", "GET", artifact_url + "/zip"])
    files = validate_readback(run, artifact, payload, run_id=args.run_id, attempt=args.attempt,
                              workflow_sha=args.workflow_sha, artifact_digest=args.artifact_digest)
    args.output.mkdir(parents=True, exist_ok=False)
    for name, data in files.items():
        (args.output / name).write_bytes(data)
    print(json.dumps(dict(state="verified_hosted_backend_gate", output=str(args.output.resolve()),
                          backend_quality_gate=True, all_quality_gate=False, sdlc_acceptance=False)))


def verify_log_cli(stage):
    root = Path(os.environ["QA_OUTPUT"])
    reviewed = json.loads(Path(os.environ["QA_INVENTORY"]).read_bytes())
    if stage == "runtime_inventory":
        verify_runtime_inventory((root / "workspace-list.log").read_text(), (root / "ignored-list.log").read_text(), reviewed)
    elif stage in reviewed["groups"] or stage == "workspace":
        listing = (root / "workspace-list.log").read_text() if stage == "workspace" else ""
        verify_test_log(stage, (root / (stage + ".log")).read_text(), reviewed, listing)


def main():
    parser = argparse.ArgumentParser(__doc__)
    modes = parser.add_subparsers(dest="mode", required=True)
    for name in ("preflight", "execute", "cleanup"):
        modes.add_parser(name)
    verify = modes.add_parser("verify-log")
    verify.add_argument("stage", choices=GATES)
    read = modes.add_parser("readback")
    for name in ("run-id", "attempt", "artifact-id"):
        read.add_argument("--" + name, type=int, required=True)
    for name in ("workflow-sha", "artifact-digest"):
        read.add_argument("--" + name, required=True)
    read.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        if args.mode == "readback":
            readback(args)
        elif args.mode == "preflight":
            preflight()
        elif args.mode == "cleanup":
            cleanup_fallback()
        elif args.mode == "verify-log":
            verify_log_cli(args.stage)
        else:
            return execute()
    except Exception:
        print("Backend control failed; no private diagnostics emitted; acceptance withheld")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
