"""Hosted-only generation; authenticated, hash-bound artifact readback. No deploy."""
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
import tarfile
import zipfile

REPOSITORY = "FerrPOINT/fleet-control"
BRANCH = "build-only/fleet-openapi-pm-union-20261010"
SOURCE_SHA = "31ab4e90b77f389b7bc5f6cfaf5f4b3d38f2d75e"
SOURCE_PARENTS = ["3694cbd399113ee25bfaa5bae7ae009df96f1e57", "5cad61a38b29058581a9ce6a8a05426303d32e53"]
SOURCE_TREE = "9b51d60334080a2486b7c232d03399ce5b89729f"
SOURCE_BLOBS = {
    ".base-revision": "1716308f859d23508a6ca0bae105434221c00419",
    "backend/Cargo.lock": "1f2a6fee32bf3dabedafc3927c56c286e14c6daf",
    "backend/api/src/bin/gen_openapi.rs": "254b94c98ee232763c040fb50629080e4282af3f",
    "backend/api/src/lib.rs": "d5b488ca8ed99d4f7e1bddf8e0498e4490fce9f4",
    "backend/api/src/routes/clarification_commands.rs": "51513bee1b4cf823bf48d4482a47f1729c33ca62",
    "backend/domain/src/clarification_commands.rs": "0c5d1610f2cd649cf1754489df6bac556bb19686",
    "backend/api/src/routes/sdlc_configuration.rs": "a9478a23db4e6f9d7db35b8342f382d1fa50775b",
    "backend/api/src/routes/agents.rs": "f512a6f5e4a0fb2c5e86e9c64fca8d827bb828da",
    "backend/domain/src/lib.rs": "2a397f3aa00db7ae1d31cf65a2e9bb66a080c7cf",
    "backend/api/src/routes/sessions.rs": "a66dd4165c8d44fef9277325f0b479b8eadfd660",
}
BASE_SHA = "19a7a381ae6dbea61a643bb96189e483fa64df5c"
QUALIFIED_EXPORT = {
    "base_lock_sha256": "9712da389d3bdc3224fb5185011814b14993a5341250ffd93a6bd8d2cdc1c235",
    "base_tree": "aa1a0486af1922c5a7fd4471e71e4fbb6aa4c7cc",
    "fleet_lock_sha256": "7ca269c7cd50cd0e9f0ca9173630353a231bf718004181e6f08bd97f495edb78",
    "source_file_count": 297,
    "source_inventory_sha256": "48077b8901381fa9609fc88e0e864dc04a19817f3ae83f1e63f90459c72253ff"
}
WORKFLOW = ".github/workflows/openapi-codegen-build-only.yml"
WRITE_SET = {WORKFLOW, "scripts/openapi_codegen.py", "scripts/tests/test_openapi_codegen.py"}
SWAGGER_SHA = "481244d0812097b11fbaeef79f71d942b171617f9c9f9514e63acbe13e71ccdc"
ARTIFACT_FILES = {"openapi.json", "provenance.json", "SHA256SUMS"}
FORBIDDEN = {".local", "target", "node_modules", ".venv", ".git", "backups", ".env"}
JOURNAL_OPERATIONS = {
    "/api/v1/sessions/{session_id}/clarifications/{question_id}/answer-commands": "post",
    "/api/v1/sessions/{session_id}/clarification-answer-commands": "get",
    "/api/v1/sessions/{session_id}/clarification-answer-commands/{command_id}": "get",
    "/api/v1/sessions/{session_id}/clarification-answer-commands/{command_id}/delivery": "post",
}
CONFIG_OPERATIONS = {
    "/api/v1/agents/{agent_id}/config/base-package": "post",
    "/internal/runtime/v1/agents/{agent_id}/configuration": "get",
}
REQUIRED_SCHEMAS = (
    "ClarificationAnswerCommand", "ClarificationAnswerRequest", "ClarificationDeliveryState",
    "ConfigurationObservation", "AgentSession",
)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical(value):
    return (json.dumps(value, sort_keys=True, indent=2) + "\n").encode()


def require(condition, message):
    if not condition:
        raise ValueError(message)


def command(args, **kwargs):
    result = subprocess.run(args, capture_output=True, timeout=300, **kwargs)
    require(result.returncode == 0, "Command failed; no codegen acceptance")
    return result.stdout


def git(root, *args):
    return command(["git", "--no-replace-objects", "-C", str(root), *args])


def clean_head(root, expected):
    require(git(root, "rev-parse", "HEAD").decode().strip() == expected, "Checkout SHA mismatch")
    require(not git(root, "status", "--porcelain=v1", "--untracked-files=all"), "Dirty checkout")


def qualify_source(root):
    require(git(root, "rev-list", "--parents", "-n", "1", SOURCE_SHA).decode().strip()
            == " ".join([SOURCE_SHA, *SOURCE_PARENTS]), "Source parent mismatch")
    require(git(root, "rev-parse", SOURCE_SHA + "^{tree}").decode().strip() == SOURCE_TREE,
            "Source tree mismatch")
    for path, blob in SOURCE_BLOBS.items():
        require(git(root, "rev-parse", SOURCE_SHA + ":" + path).decode().strip() == blob,
                "Generator input blob mismatch")


def validate_delta(text):
    changes = [line.split("\t") for line in text.splitlines()]
    require({tuple(line) for line in changes} == {("A", name) for name in WRITE_SET}
            and len(changes) == len(WRITE_SET), "Build branch must add only the three reviewed control files")


def hosted_identity():
    expected = {"GITHUB_ACTIONS": "true", "RUNNER_ENVIRONMENT": "github-hosted",
                "GITHUB_REPOSITORY": REPOSITORY, "GITHUB_EVENT_NAME": "push",
                "GITHUB_REF": "refs/heads/" + BRANCH}
    require(os.name == "posix" and all(os.environ.get(key) == value for key, value in expected.items()),
            "Generation is restricted to the dedicated hosted branch push")
    require(bool(re.fullmatch(r"[0-9a-f]{40}", os.environ.get("GITHUB_SHA", ""))), "Missing workflow SHA")
    return Path(os.environ["GITHUB_WORKSPACE"]).resolve(), os.environ["GITHUB_SHA"]


def preflight():
    workspace, workflow_sha = hosted_identity()
    controls = workspace / "controls"
    clean_head(controls, workflow_sha)
    qualify_source(controls)
    git(controls, "merge-base", "--is-ancestor", SOURCE_SHA, workflow_sha)
    validate_delta(git(controls, "diff", "--no-renames", "--name-status", SOURCE_SHA, workflow_sha).decode())
    require((controls / ".base-revision").read_text().strip() == BASE_SHA, "Base pin drift")
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
    data = git(root, "-c", "core.autocrlf=false", "-c", "core.eol=lf",
               "archive", "--format=tar", commit, "--", *roots)
    destination.mkdir(parents=True, exist_ok=False)
    with tarfile.open(fileobj=io.BytesIO(data)) as archive:
        for member in archive:
            path = safe_member(member.name.rstrip("/") if member.isdir() else member.name,
                               roots, directory=member.isdir())
            target = destination.joinpath(*path.parts)
            require(member.isdir() or member.isfile(), "Source links/submodules are forbidden")
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


def qualify_export(before, base_tree):
    observed = dict(source_inventory_sha256=digest(canonical(before)), source_file_count=len(before),
                    base_tree=base_tree, fleet_lock_sha256=before.get("fleet-control/backend/Cargo.lock"),
                    base_lock_sha256=before.get("services-base/Cargo.lock"))
    require(observed == QUALIFIED_EXPORT, "Qualified Git export inventory mismatch")


def schema_valid(data):
    value = json.loads(data)
    require(isinstance(value, dict) and str(value.get("openapi", "")).startswith("3.")
            and isinstance(value.get("paths"), dict) and bool(value["paths"]), "Invalid generated OpenAPI")
    require(all(isinstance(value["paths"].get(path), dict) and method in value["paths"][path]
                for path, method in (JOURNAL_OPERATIONS | CONFIG_OPERATIONS).items()),
            "Generated required operations missing")
    schemas = value.get("components", {}).get("schemas", {})
    require(all(isinstance(schemas.get(name), dict) for name in REQUIRED_SCHEMAS),
            "Generated required DTOs missing")
    session = schemas["AgentSession"]
    require(session.get("properties", {}).get("pending_delivery") == {"type": ["boolean", "null"]}
            and "pending_delivery" not in session.get("required", []),
            "Generated optional pending-delivery contract missing")


def generate():
    workspace, controls, workflow_sha = preflight()
    source, base = workspace / "fleet-control", workspace / "services-base"
    clean_head(source, SOURCE_SHA)
    qualify_source(source)
    clean_head(base, BASE_SHA)
    require((source / ".base-revision").read_text().strip() == BASE_SHA, "Source Base pin drift")
    require(command(["rustc", "--version"]).decode().split()[1] == "1.88.0", "Rust toolchain drift")
    temporary = Path(os.environ["RUNNER_TEMP"]).resolve()
    require(temporary.is_absolute() and temporary.is_dir(), "Missing hosted temporary root")
    require(shutil.disk_usage(temporary).free >= 5 * 1024 ** 3, "Disposable hosted CI requires 5 GiB free")
    scratch, evidence = temporary / "fleet-openapi-codegen", temporary / "fleet-openapi-evidence"
    scratch.mkdir(exist_ok=False)
    try:
        evidence.mkdir(exist_ok=False)
        export(source, SOURCE_SHA, scratch / "src/fleet-control", ("backend", ".base-revision"))
        export(base, BASE_SHA, scratch / "src/services-base", ("crates", "Cargo.toml", "Cargo.lock", "LICENSE"))
        before = inventory(scratch / "src")
        qualify_export(before, git(base, "rev-parse", "HEAD^{tree}").decode().strip())
        for name in ("tmp", "cargo", "target"):
            (scratch / name).mkdir()
        swagger = scratch / "swagger.zip"
        command(["curl", "--fail", "--location", "--proto", "=https", "--proto-redir", "=https", "--retry", "0",
                 "https://github.com/swagger-api/swagger-ui/archive/refs/tags/v5.17.14.zip", "--output", str(swagger)])
        require(digest(swagger.read_bytes()) == SWAGGER_SHA, "Swagger archive hash mismatch")
        environment = dict(os.environ, CARGO_HOME=str(scratch / "cargo"), CARGO_TARGET_DIR=str(scratch / "target"),
                           TMPDIR=str(scratch / "tmp"), CARGO_BUILD_JOBS="1", CARGO_INCREMENTAL="0",
                           CARGO_PROFILE_DEV_DEBUG="0", RUSTUP_TOOLCHAIN="1.88.0", SWAGGER_UI_DOWNLOAD_URL=swagger.as_uri())
        backend = scratch / "src/fleet-control/backend"
        # Cargo diagnostics can quote private Base sources; keep them out of public logs/artifacts.
        with (evidence / "openapi.json").open("xb") as output, (scratch / "cargo-stderr.log").open("xb") as diagnostics:
            result = subprocess.run(["cargo", "run", "--locked", "-p", "api", "--bin", "gen-openapi"],
                                    cwd=backend, env=environment, stdout=output, stderr=diagnostics, timeout=2400)
        require(result.returncode == 0, "Rust generator failed")
        schema = (evidence / "openapi.json").read_bytes()
        schema_valid(schema)
        require(inventory(scratch / "src") == before, "Compiled source content drift")
        clean_head(source, SOURCE_SHA)
        clean_head(base, BASE_SHA)
        clean_head(controls, workflow_sha)
        provenance = dict(version=1, repository=REPOSITORY, branch=BRANCH, source_sha=SOURCE_SHA, base_sha=BASE_SHA,
                          source_parents=SOURCE_PARENTS, qualified_source_blobs=SOURCE_BLOBS,
                          workflow_sha=workflow_sha, workflow_path=WORKFLOW,
                          run_id=int(os.environ["GITHUB_RUN_ID"]), run_attempt=int(os.environ["GITHUB_RUN_ATTEMPT"]),
                          rust="1.88.0", command=["cargo", "run", "--locked", "-p", "api", "--bin", "gen-openapi"],
                          schema_sha256=digest(schema), source_inventory_sha256=digest(canonical(before)),
                          source_file_count=len(before),
                          source_tree=git(source, "rev-parse", "HEAD^{tree}").decode().strip(),
                          base_tree=git(base, "rev-parse", "HEAD^{tree}").decode().strip(),
                          fleet_lock_sha256=before["fleet-control/backend/Cargo.lock"],
                          base_lock_sha256=before["services-base/Cargo.lock"], swagger_sha256=SWAGGER_SHA,
                          helper_sha256=digest(Path(__file__).read_bytes()),
                          workflow_sha256=digest((controls / WORKFLOW).read_bytes()),
                          runner_image=os.environ.get("ImageOS"), runner_image_version=os.environ.get("ImageVersion"),
                          schema_generator_success=True, all_quality_gate=False, sdlc_acceptance=False)
        (evidence / "provenance.json").write_bytes(canonical(provenance))
        checksums = "".join(digest((evidence / name).read_bytes()) + "  " + name + "\n"
                            for name in ("openapi.json", "provenance.json"))
        (evidence / "SHA256SUMS").write_text(checksums, encoding="ascii", newline="\n")
    finally:
        require(scratch.resolve() == temporary / "fleet-openapi-codegen" and not scratch.is_symlink(),
                "Scratch cleanup path changed; manual recovery required")
        shutil.rmtree(scratch)


def validate_readback(run, artifact, payload, *, run_id, attempt, workflow_sha, artifact_digest):
    require(run["id"] == run_id and run["run_attempt"] == attempt and run["status"] == "completed"
            and run["conclusion"] == "success" and run["event"] == "push" and run["head_sha"] == workflow_sha
            and run["head_branch"] == BRANCH and run["path"] == WORKFLOW
            and run["repository"]["full_name"] == REPOSITORY, "Unexpected/unsuccessful workflow identity")
    require(not artifact["expired"] and artifact["workflow_run"]["id"] == run_id
            and artifact["workflow_run"]["head_sha"] == workflow_sha
            and artifact["name"] == f"fleet-openapi-pm-union-{run_id}-{attempt}"
            and artifact["digest"] == "sha256:" + artifact_digest
            and digest(payload) == artifact_digest, "Artifact identity/digest mismatch")
    require(len(payload) <= 8 * 1024 ** 2, "Oversized artifact ZIP")
    with zipfile.ZipFile(io.BytesIO(payload)) as archive:
        members = archive.infolist()
        require(len(members) == 3 and {item.filename for item in members} == ARTIFACT_FILES
                and all(item.file_size <= 8 * 1024 ** 2 and (item.external_attr >> 16) & 0o170000 != 0o120000
                        for item in members), "Unsafe artifact members")
        files = {item.filename: archive.read(item) for item in members}
    provenance = json.loads(files["provenance.json"])
    expected = dict(version=1, repository=REPOSITORY, branch=BRANCH, source_sha=SOURCE_SHA, base_sha=BASE_SHA,
                    source_parents=SOURCE_PARENTS, source_tree=SOURCE_TREE, qualified_source_blobs=SOURCE_BLOBS,
                    workflow_sha=workflow_sha, workflow_path=WORKFLOW, run_id=run_id, run_attempt=attempt,
                    rust="1.88.0", swagger_sha256=SWAGGER_SHA, schema_generator_success=True,
                    all_quality_gate=False, sdlc_acceptance=False,
                    command=["cargo", "run", "--locked", "-p", "api", "--bin", "gen-openapi"],
                    helper_sha256=digest(Path(__file__).read_text(encoding="utf-8").encode()),
                    workflow_sha256=digest((Path(__file__).resolve().parents[1] / WORKFLOW).read_text(encoding="utf-8").encode()))
    expected.update(QUALIFIED_EXPORT)
    require(all(type(provenance.get(key)) is type(value) and provenance.get(key) == value
                for key, value in expected.items()), "Provenance mismatch")
    require(provenance["schema_sha256"] == digest(files["openapi.json"]), "Schema hash mismatch")
    expected_sums = "".join(digest(files[name]) + "  " + name + "\n" for name in ("openapi.json", "provenance.json"))
    require(files["SHA256SUMS"] == expected_sums.encode(), "File checksum manifest mismatch")
    schema_valid(files["openapi.json"])
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
    print(json.dumps(dict(state="verified_codegen_artifact", schema_sha256=digest(files["openapi.json"]),
                          output=str(args.output.resolve()), all_quality_gate=False, sdlc_acceptance=False)))


def main():
    parser = argparse.ArgumentParser(__doc__)
    modes = parser.add_subparsers(dest="mode", required=True)
    modes.add_parser("preflight")
    modes.add_parser("generate")
    read = modes.add_parser("readback")
    for name in ("run-id", "attempt", "artifact-id"):
        read.add_argument("--" + name, type=int, required=True)
    for name in ("workflow-sha", "artifact-digest"):
        read.add_argument("--" + name, required=True)
    read.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.mode == "readback":
        readback(args)
    elif args.mode == "generate":
        generate()
    else:
        preflight()


if __name__ == "__main__":
    main()
