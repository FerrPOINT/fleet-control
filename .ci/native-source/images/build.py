"""Existing image wrapper's cold-source successor. Preparation is the default task scope."""
import argparse
from datetime import datetime, timezone
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import uuid

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(ROOT / "qa"))
from packet import export, git, inventory, member, sha, write_json
import hosted_policy as policy
import recipes

INPUTS = json.loads((HERE / "inputs.json").read_bytes())
HELPERS = ("build.py", "recipes.py", "qualify.py", "fetch.py", "snapshot.sh", "controller.Dockerfile", "inputs.json")
PREFIX = "sdlc-build-fleet-native-"
PARITY_KEYS = ("sources", "parents", "daemon", "permanent", "resources")
PARENT_KINDS = frozenset(("rust", "postgres", "uv", "docker", "debian"))
PULL_LOG_LIMIT = 65536
PULL_CATEGORIES = frozenset(("rate_limit", "registry_denied", "manifest_unavailable", "dns", "tls", "timeout", "unknown"))
CANDIDATE_KINDS = frozenset(("controller", "hermes"))
BUILD_LOG_LIMIT = 65536
BUILD_SCAN_LIMIT = 8 * 1024 ** 2
BUILD_LINE_LIMIT = 65536
RECIPE_RUNS = {
    "33e8b80d063bda4262e9e01a23ba978b839c2b9627f7a9f938d59c3755c29c8a": "apt_setup",
    "0b58742ab99ab2b9eff32eef4647930b1307bda332bba3a9c73c8d832d304aaa": "uv_sync",
    "08c1d64bde9e3725433a4696291216a991ada87e164562f80789e13b79193fee": "account_setup",
}
BUILD_CATEGORIES = PULL_CATEGORIES | {"rust_compile", "no_space", "dependency_resolution",
    "docker_cli_refused", "compose_config_refused", "pinned_fetch_refused", "pinned_hash_refused", "apt_refused", "account_refused",
    "uv_build_refused", "uv_download_build_refused", "uv_no_solution", "uv_no_platform_distribution"}
OPERATIONS = frozenset(("unknown", "parent_resources", "parent_pull", "parent_identity",
    "candidate_source", "candidate_resources", "candidate_daemon", "candidate_tag",
    "controller_input", "candidate_build", "candidate_metadata", "offline_qualification",
    "qualified_image_check", *("parity_" + key for key in PARITY_KEYS)))
REASONS = frozenset(("unspecified", "command_nonzero", "process_timeout", "image_metadata_rejected",
    "parent_identity_rejected", "candidate_tag_preexisting", "controller_input_changed",
    "candidate_platform_rejected", "candidate_owner_rejected", "daemon_changed",
    "qualified_image_changed", "qualification_command_nonzero", "parity_rejected",
    "qualification_resources_remain"))


class BuildFailure(ValueError):
    def __init__(self, reason):
        if type(reason) is not str or reason not in REASONS:
            raise ValueError("Closed build failure reason required")
        super().__init__("Closed build precondition failed")
        self.reason = reason


FAILURE_CLASSES = {kind: kind.__name__ for kind in (BuildFailure, ValueError, OSError,
    PermissionError, FileNotFoundError, TimeoutError, subprocess.TimeoutExpired,
    subprocess.CalledProcessError, json.JSONDecodeError, KeyError, TypeError, AssertionError)}


def remember_failure(report, error, operation):
    if "failure_class" not in report:
        report["failure_class"] = FAILURE_CLASSES.get(type(error), "OtherError")
        report["failure_operation"] = operation
        report["failure_reason"] = (error.reason if type(error) is BuildFailure else
                                    "process_timeout" if type(error) is subprocess.TimeoutExpired else "unspecified")
        if operation in ("parent_pull", "candidate_build") and type(error) in (BuildFailure, subprocess.TimeoutExpired):
            value = getattr(error, operation, None)
            if type(value) is dict:
                report[operation] = value


def failure_projection(report):
    """Only fixed enums/bools; never serialize an exception, command or private report."""
    result = {}
    for key, allowed, fallback in (("failure_class", {*FAILURE_CLASSES.values(), "OtherError"}, "OtherError"),
                                  ("failure_operation", OPERATIONS, "unknown"),
                                  ("failure_reason", REASONS, "unspecified")):
        value = report.get(key)
        result[key] = value if type(value) is str and value in allowed else fallback
    parity = report.get("parity")
    result["parity"] = {key: parity.get(key) if type(parity) is dict and type(parity.get(key)) is bool else None
                        for key in PARITY_KEYS}
    pull = report.get("parent_pull")
    if result["failure_operation"] == "parent_pull" and type(pull) is dict:
        kind, code, category = (pull.get(key) for key in ("kind", "exit_code", "category"))
        result["parent_pull"] = dict(
            kind=kind if type(kind) is str and kind in PARENT_KINDS else "unknown",
            exit_code=code if type(code) is int and -2147483648 <= code <= 4294967295 and code != 0 else None,
            category=category if type(category) is str and category in PULL_CATEGORIES else "unknown")
    candidate = report.get("candidate_build")
    if result["failure_operation"] == "candidate_build" and type(candidate) is dict:
        kind, code, category, scope = (candidate.get(key) for key in ("kind", "exit_code", "category", "log_scope"))
        value = dict(kind=kind if type(kind) is str and kind in CANDIDATE_KINDS else "unknown",
            exit_code=code if type(code) is int and -2147483648 <= code <= 4294967295 and code != 0 else None,
            category=category if type(category) is str and category in BUILD_CATEGORIES else "unknown",
            log_scope=scope if type(scope) is str and scope in ("full", "tail", "unavailable") else "unavailable")
        codes = candidate.get("rust_codes")
        if type(codes) is list:
            codes = list(dict.fromkeys(code for code in codes[:8] if type(code) is str and re.fullmatch(r"E[0-9]{4}", code)))
            if codes:
                value["rust_codes"] = codes
        if "recipe_instruction" in candidate or "inner_exit_code" in candidate:
            instruction, inner = candidate.get("recipe_instruction"), candidate.get("inner_exit_code")
            valid = (value["kind"] == "hermes" and value["log_scope"] == "full"
                     and value["exit_code"] is not None and type(instruction) is str
                     and instruction in RECIPE_RUNS.values() and type(inner) is int and 1 <= inner <= 255)
            value.update(recipe_instruction=instruction if valid else None, inner_exit_code=inner if valid else None)
            if not valid:
                value["category"] = "unknown"
        result["candidate_build"] = value
    if len(json.dumps(result).encode("ascii")) > 1024:
        raise ValueError("Closed failure projection exceeds bound")
    return result


def checked(args, timeout=120):
    result = subprocess.run(args, capture_output=True, timeout=timeout)
    if result.returncode:
        raise BuildFailure("command_nonzero")
    return result.stdout


def parent_pull_category(raw):
    """Closed symptoms, not a registry root cause; incomplete/ambiguous evidence stays unknown."""
    if type(raw) is not bytes or len(raw) > PULL_LOG_LIMIT:
        return "unknown"
    raw = raw.lower()
    if b"repository does not exist or may require" in raw or b"pull access denied for" in raw:
        return "unknown"
    patterns = {
        "rate_limit": rb"toomanyrequests:|429 too many requests|you have reached your (?:unauthenticated )?pull rate limit",
        "registry_denied": rb"unauthorized: authentication required|denied: requested access to the resource is denied|denied: access forbidden",
        "manifest_unavailable": rb"manifest unknown|manifest not found|manifest for [^\r\n]{1,512} not found",
        "dns": rb"lookup [^\r\n]{1,512}: (?:no such host|server misbehaving)",
        "tls": rb"x509: certificate signed by unknown authority|x509: certificate has expired or is not yet valid|tls: failed to verify certificate|remote error: tls: handshake failure",
        "timeout": rb"context deadline exceeded|i/o timeout|tls handshake timeout|client\.timeout exceeded",
    }
    matches = [kind for kind, pattern in patterns.items() if re.search(pattern, raw)]
    return matches[0] if len(matches) == 1 else "unknown"


def candidate_build_diagnostic(raw, *, tail=False, recipe=None, recipe_sha256=None):
    """Bounded complete frames only; large logs do not enable broad legacy heuristics."""
    if (type(raw) is not bytes or type(tail) is not bool
            or len(raw) > (BUILD_LOG_LIMIT if tail else BUILD_SCAN_LIMIT)
            or raw.count(b"\n") > BUILD_LINE_LIMIT):
        result = dict(category="unknown", log_scope="unavailable")
        if recipe_sha256 is not None:
            result.update(recipe_instruction=None, inner_exit_code=None)
        return result
    legacy_full = not tail and len(raw) <= BUILD_LOG_LIMIT
    if tail:
        raw = raw.partition(b"\n")[2]
    complete = [line.removesuffix(b"\r") for line in raw.split(b"\n")[:-1]]
    uv_patterns = {
        "uv_build_refused": rb"  \xc3\x97 Failed to build `[^`\r\n\x1b]{1,2048}`",
        "uv_download_build_refused": rb"  \xc3\x97 Failed to download and build `[^`\r\n\x1b]{1,2048}`",
        "uv_no_solution": rb"  \xc3\x97 No solution found when resolving dependencies(?: for [^\r\n\x1b]{1,2048})?:",
        "uv_no_platform_distribution": rb"error: Distribution `[^`\r\n\x1b]{1,2048}` can't be installed because it doesn't have a source distribution or wheel for the current platform",
    }
    uv_headers, uv_failed, terminals = {}, set(), []
    # Correlate complete producer frames before stripping their vertex identity.
    for line in complete:
        if b"\x1b" in line:
            continue
        header = re.fullmatch(rb"#([1-9][0-9]{0,5}) [0-9]{1,8}(?:\.[0-9]{1,6})? (.+)", line)
        if header:
            for reason, pattern in uv_patterns.items():
                if re.fullmatch(pattern, header[2]):
                    uv_headers.setdefault(header[1], set()).add(reason)
        terminal = re.fullmatch(rb'#([1-9][0-9]{0,5}) ERROR: process "([^\r\n\x1b]{1,16384})" did not complete successfully: exit code: ([1-9][0-9]{0,2})', line)
        if terminal:
            terminals.append(terminal)
            if int(terminal[3]) <= 255 and terminal[1] in uv_headers:
                uv_failed.add(terminal[1])
    uv_reasons = {reason for vertex in uv_failed for reason in uv_headers[vertex]}
    prefix = rb"^(?:#[0-9]{1,6} )?[0-9]{1,8}(?:\.[0-9]{1,6})? "
    lines = [re.sub(prefix, b"", line) for line in complete]
    codes = []
    for line in lines:
        frame = re.fullmatch(rb"error\[(E[0-9]{4})\]:[^\r\n]*", line)
        if frame:
            code = frame[1].decode("ascii")
            if code not in codes and len(codes) < 8:
                codes.append(code)
    result = dict(category="unknown", log_scope="tail" if tail else "full")
    if codes:
        result["rust_codes"] = codes
    matches = {"rust_compile"} if codes else set()
    # These are exact existing CLI/recipe diagnostic shapes, not arbitrary messages.
    refusals = {
        "docker_cli_refused": rb"(?:ERROR: )?unknown flag: --(?:builder|pull|no-cache|provenance)",
        "compose_config_refused": rb"validating [^\r\n]{1,1024}build-compose\.json: services\.(?:controller|hermes)-image\.build Additional property [a-zA-Z0-9_]{1,64} is not allowed",
        "pinned_fetch_refused": rb'\{"state": "withheld", "failure_class": "(?:ValueError|HTTPError|URLError|TimeoutError|SSLError|OSError|FileNotFoundError)"\}',
        "pinned_hash_refused": rb"sha256sum: WARNING: [1-9][0-9]{0,5} computed checksums? did NOT match",
        "apt_refused": rb"E: (?:Unable to locate package [a-zA-Z0-9.+:-]{1,256}|Unable to correct problems, you have held broken packages\.|Sub-process /usr/bin/dpkg returned an error code \([1-9][0-9]{0,2}\)|The repository '[^\r\n]{1,1024}' (?:is not signed\.|does not have a Release file\.))",
        "account_refused": rb"groupadd: GID '999' already exists|useradd: UID 999 is not unique|useradd: group '999' does not exist",
    }
    for category, pattern in refusals.items():
        if any(re.fullmatch(pattern, line) for line in lines):
            matches.add(category)
    if legacy_full:
        network = parent_pull_category(raw)
        if network != "unknown":
            matches.add(network)
    patterns = {
        "no_space": rb"no space left on device",
        "dependency_resolution": rb"error: failed to select a version for|no solution found when resolving dependencies",
    }
    for category, pattern in patterns.items():
        if legacy_full and re.search(pattern, raw.lower()):
            matches.add(category)
    if uv_reasons == {"uv_no_solution"} and matches == {"dependency_resolution"}:
        matches.clear()  # The paired UV header is the same resolution symptom.
    matches.update(uv_reasons)
    if len(matches) == 1:
        result["category"] = next(iter(matches))
    if recipe_sha256 is not None:
        result.update(recipe_instruction=None, inner_exit_code=None)
        # Both the whole generated recipe seal and the three reviewed RUN bytes
        # must match. No shell parsing, substring match or command projection.
        if (not tail and raw.endswith(b"\n") and type(recipe) is bytes and len(recipe) <= 65536
                and type(recipe_sha256) is str and re.fullmatch(r"[a-f0-9]{64}", recipe_sha256)
                and hashlib.sha256(recipe).hexdigest() == recipe_sha256 and len(terminals) == 1
                and len(matches) <= 1
                and sum(b"ERROR: process " in line for line in complete) == 1):
            runs = [line[4:] for line in recipe.splitlines() if line.startswith(b"RUN ")]
            hashes = [hashlib.sha256(command).hexdigest() for command in runs]
            command, inner = terminals[0][2], int(terminals[0][3])
            if len(runs) == 3 and set(hashes) == set(RECIPE_RUNS) and 1 <= inner <= 255:
                for run, digest in zip(runs, hashes):
                    if command == b"/bin/sh -c " + run:
                        result.update(recipe_instruction=RECIPE_RUNS[digest], inner_exit_code=inner)
        if result["recipe_instruction"] is None:
            result["category"] = "unknown"
    return result


def logged(args, path, timeout=1800, *, parent_kind=None, candidate_kind=None, recipe_sha256=None):
    if parent_kind is not None and (type(parent_kind) is not str or parent_kind not in PARENT_KINDS):
        raise ValueError("Closed source parent kind required")
    if candidate_kind is not None and (parent_kind is not None or type(candidate_kind) is not str or candidate_kind not in CANDIDATE_KINDS):
        raise ValueError("Closed source candidate kind required")
    if recipe_sha256 is not None and candidate_kind != "hermes":
        raise ValueError("Recipe diagnostics require the Hermes candidate")
    with path.open("x+b" if parent_kind is not None or candidate_kind is not None else "xb") as output:
        try:
            result = subprocess.run(args, stdout=output, stderr=subprocess.STDOUT, timeout=timeout)
        except subprocess.TimeoutExpired as error:
            if parent_kind is not None:
                error.parent_pull = dict(kind=parent_kind, exit_code=None, category="timeout")
            if candidate_kind is not None:
                error.candidate_build = dict(kind=candidate_kind, exit_code=None, category="timeout", log_scope="unavailable")
            raise
        if result.returncode:
            error = BuildFailure("command_nonzero")
            if parent_kind is not None:
                category = "unknown"
                try:
                    output.seek(0)
                    category = parent_pull_category(output.read(PULL_LOG_LIMIT + 1))
                except OSError:
                    pass
                error.parent_pull = dict(kind=parent_kind, exit_code=result.returncode, category=category)
            if candidate_kind is not None:
                diagnostic = dict(category="unknown", log_scope="unavailable")
                recipe = None
                if recipe_sha256 is not None:
                    diagnostic.update(recipe_instruction=None, inner_exit_code=None)
                    try:
                        filename = path.parent.parent / "context/recipes/hermes.Dockerfile"
                        if filename.is_symlink():
                            raise OSError("Ordinary recipe required")
                        fd = os.open(filename, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0))
                        with os.fdopen(fd, "rb") as stream:
                            st = os.fstat(stream.fileno())
                            if stat.S_ISREG(st.st_mode) and st.st_size <= 65536:
                                recipe = stream.read(65537)
                    except OSError:
                        pass
                try:
                    output.seek(0)
                    raw = output.read(BUILD_SCAN_LIMIT + 1)
                    diagnostic = (candidate_build_diagnostic(raw) if recipe_sha256 is None else
                                  candidate_build_diagnostic(raw, recipe=recipe, recipe_sha256=recipe_sha256))
                except OSError:
                    pass
                error.candidate_build = dict(kind=candidate_kind, exit_code=result.returncode, **diagnostic)
            raise error


def image(docker, ref):
    template = '{{json .Id}}|{{json .RepoDigests}}|{{json .RootFS.Layers}}|{{json .Config.Labels}}|{{.Os}}|{{.Architecture}}|{{json .Config.Volumes}}'
    p = checked(docker + ["image", "inspect", ref, "--format", template]).decode().strip().split("|")
    if len(p) != 7:
        raise BuildFailure("image_metadata_rejected")
    return dict(id=json.loads(p[0]), repo_digests=json.loads(p[1]) or [], layers=json.loads(p[2]),
                labels=json.loads(p[3]) or {}, os=p[4], architecture=p[5], volumes=json.loads(p[6]) or {})


def canonical_ref(ref):
    if ref.startswith("docker.io/library/"):
        return ref.removeprefix("docker.io/library/")
    if ref.startswith("library/"):
        return ref.removeprefix("library/")
    return ref


def parent_identity(ref, value):
    if (canonical_ref(ref) not in [canonical_ref(x) for x in value["repo_digests"]]
            or value["os"] != "linux" or value["architecture"] != "amd64"
            or not re.fullmatch(r"sha256:[a-f0-9]{64}", value["id"])):
        raise BuildFailure("parent_identity_rejected")


def compose(root):
    project = root.name
    tags = {kind: project + "-" + kind + ":candidate" for kind in ("controller", "hermes")}
    labels = {"sdlc.task": "fleet-native-source-build", "sdlc.purpose": "qualified-candidates-not-runtime-promotion",
              "sdlc.fleet.qa.source": INPUTS["fleet"], "sdlc.hermes.revision": INPUTS["hermes"]}
    result = {}
    for kind in tags:
        args = dict(RUST_IMAGE=INPUTS["parents"]["rust"], DOCKER_IMAGE=INPUTS["parents"]["docker"]) if kind == "controller" else dict(CONTROLLER_IMAGE=tags["controller"])
        result[kind + "-image"] = dict(image=tags[kind], build=dict(context=str(root / "context"),
            dockerfile="recipes/" + kind + ".Dockerfile", args=args, labels=labels))
    return dict(name=project, services=result)


def prepare(repos, profile):
    p = policy.policy(profile)
    for key in ("fleet", "sdk", "base", "maintenance", "hermes"):
        git(repos[key], "cat-file", "-e", INPUTS[key] + "^{commit}")
    if git(repos["fleet"], "show", INPUTS["fleet"] + ":.base-revision").decode().strip() != INPUTS["sdk"]:
        raise ValueError("Product SDK pin mismatch")
    parents = git(repos["fleet"], "show", "-s", "--format=%P", INPUTS["fleet"]).decode().split()
    if parents != [INPUTS["fleet_parent"]]:
        raise ValueError("Exact current product parent tuple required")
    if git(repos["fleet"], "rev-parse", INPUTS["fleet"] + "^{tree}").decode().strip() != INPUTS["fleet_tree"]:
        raise ValueError("Exact current product tree required")
    import source_coverage
    coverage = source_coverage.qualify(repos["fleet"], INPUTS["fleet"])
    root = HERE / (PREFIX + uuid.uuid4().hex[:12])
    root.mkdir()
    try:
        context = root / "context"
        sources = context / "sources"
        recipes_dir = context / "recipes"
        recipes_dir.mkdir(parents=True)
        for key, paths in (("fleet", ["backend", ".base-revision"]),
                           ("sdk", ["Cargo.toml", "Cargo.lock", "crates", "LICENSE"]),
                           ("base", ["deploy/fleet-standard.Dockerfile", "deploy/fleet-hermes-launch.py",
                                     "deploy/fleet-hermes-container-launch.py", "scripts/runtime_boundary.py",
                                     "scripts/runtime_bootstrap.py", "scripts/runtime_control.py", "scripts/runtime_replacement.py"]),
                           ("maintenance", ["scripts/compose_helpers.py"]), ("hermes", ["."])):
            export(repos[key], INPUTS[key], sources / key, paths)
        if sha(sources / "maintenance/scripts/compose_helpers.py") != INPUTS["maintenance_sha256"]:
            raise ValueError("Exact published maintenance helper required")
        source_inventory = inventory(sources)
        entries = {}
        for entry in git(repos["hermes"], "ls-tree", "-r", "-z", INPUTS["hermes"]).split(b"\0"):
            if not entry:
                continue
            meta, name = entry.split(b"\t", 1)
            mode, kind, oid = meta.decode().split()
            path = member(name.decode(), mode)
            if kind != "blob":
                raise ValueError("Ordinary Hermes Git blobs required")
            entries[str(path)] = dict(git_blob=oid, size=(sources / "hermes" / path).stat().st_size)
        if len(entries) != INPUTS["hermes_blobs"]:
            raise ValueError("All 13770 Hermes blobs required")
        for name in HELPERS:
            shutil.copyfile(HERE / name, recipes_dir / name)
        (recipes_dir / "hermes.Dockerfile").write_bytes(recipes.hermes_recipe(
            (sources / "base/deploy/fleet-standard.Dockerfile").read_bytes(), INPUTS,
            (sources / "hermes/uv.lock").read_bytes()))
        (recipes_dir / "build-constraints.txt").write_bytes(recipes.build_constraints(INPUTS))
        (context / ".dockerignore").write_text(".git\n**/.git\n**/.local\n**/target\n**/node_modules\n**/.venv\n**/__pycache__\n", encoding="utf-8", newline="\n")
        parent = root / "parent"
        (parent / "evidence").mkdir(parents=True)
        write_json(parent / "manifest.json", {"hermes_source_files": len(entries)})
        write_json(parent / "hermes-git-inventory.json", entries)
        for name in ("pyproject.toml", "uv.lock"):
            shutil.copyfile(sources / "hermes" / name, parent / "evidence" / ("hermes-" + name))
        shutil.copyfile(HERE / "qualify.py", root / "qualify.py")
        (root / "evidence").mkdir()
        write_json(root / "build-compose.json", compose(root))
        write_json(root / "manifest.json", dict(state="prepared_not_executed", project=root.name,
            inputs=INPUTS, policy=p, source_inventory=source_inventory, files=inventory(root),
            helpers={n: sha(HERE / n) for n in HELPERS}, policy_sha256=sha(ROOT / "qa/hosted_policy.py"),
            source_coverage_sha256=sha(ROOT / "qa/source_coverage.py"),
            packet_sha256=sha(ROOT / "qa/packet.py"), inherited_image_ids=[], no_runtime_promotion=True,
            source_coverage=coverage, source_count=len(source_inventory), native_executed=False, schema_generation=False))
        write_json(root / "seal.json", {"manifest_sha256": sha(root / "manifest.json")})
        verify(root)
    except BaseException as error:
        write_json(root / "prepare-failure.json", {"state": "failed", "failure_class": type(error).__name__})
        raise
    return root


def verify(root):
    if root.is_symlink() or root.parent.resolve() != HERE.resolve() or not re.fullmatch(PREFIX + r"[a-f0-9]{12}", root.name):
        raise ValueError("Owned direct-child packet required")
    if (root / "prepare-failure.json").exists():
        raise ValueError("Failed packet is immutable")
    for name in ("context", "parent", "evidence", "manifest.json", "seal.json", "build-compose.json"):
        if (root / name).is_symlink():
            raise ValueError("Ordinary packet paths required")
    m = json.loads((root / "manifest.json").read_bytes())
    if (json.loads((root / "seal.json").read_bytes()) != {"manifest_sha256": sha(root / "manifest.json")}
            or m["inputs"] != INPUTS or m["project"] != root.name
            or m["policy"] != policy.policy(m["policy"]["profile"])
            or m["inherited_image_ids"] != [] or m["native_executed"] is not False
            or m["policy_sha256"] != sha(ROOT / "qa/hosted_policy.py")
            or m["source_coverage_sha256"] != sha(ROOT / "qa/source_coverage.py")
            or m["packet_sha256"] != sha(ROOT / "qa/packet.py")):
        raise ValueError("Input/capacity/source seal mismatch")
    for name, digest in m["files"].items():
        member(name, "100644")
        path = root / name
        if path.is_symlink() or sha(path) != digest:
            raise ValueError("Sealed context differs")
    context_files = {n.removeprefix("context/"): digest for n, digest in m["files"].items() if n.startswith("context/")}
    if (inventory(root / "context") != context_files
            or inventory(root / "parent") != {n.removeprefix("parent/"): digest for n, digest in m["files"].items() if n.startswith("parent/")}
            or inventory(root / "context/sources") != m["source_inventory"]
            or m["source_count"] != len(m["source_inventory"])
            or any(m["helpers"][n] != sha(HERE / n) for n in HELPERS)
            or json.loads((root / "build-compose.json").read_bytes()) != compose(root)):
        raise ValueError("Current helper/source/Compose differs")
    return m


def maintenance(root):
    path = root / "context/sources/maintenance"
    if sha(path / "scripts/compose_helpers.py") != INPUTS["maintenance_sha256"]:
        raise ValueError("Maintenance source mismatch")
    spec = importlib.util.spec_from_file_location("native_build_maintenance", path / "scripts/compose_helpers.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def qualification_proof(root, kind):
    path = root / "evidence" / (kind + "-qualification.log")
    if path.is_symlink() or path.stat().st_size > 1048576:
        raise ValueError("Bounded ordinary qualification receipt required")
    proof = json.loads(path.read_bytes())
    if proof.get("state") != "qualified" or proof.get("kind") != kind or proof.get("native_executed") is not False:
        raise ValueError("Actual offline qualification failed")
    if kind == "controller":
        if (not proof.get("rust", "").startswith("rustc 1.88.0 ")
                or not proof.get("cargo", "").startswith("cargo 1.88.0 ")
                or not proof.get("docker", "").startswith("Docker version ") or not proof.get("compose")):
            raise ValueError("Controller toolchain proof incomplete")
    elif (proof.get("source_files_verified") != 13770 or not proof.get("uv", "").startswith("uv 0.11.6")
          or proof.get("installed_closure_check") != "uv_frozen_offline_check" or not proof.get("packages")
          or proof.get("uv_lock_sha256") != sha(root / "context/sources/hermes/uv.lock")):
        raise ValueError("Hermes source/installed closure proof incomplete")
    return {"sha256": sha(path), "state": "qualified"}


def qualify(api, root, docker, candidates, daemon):
    helper = api.ComposeHelper(project=root.name, task="fleet-native-source-build",
        purpose="offline-13770-blobs-and-locked-dependencies", docker=docker,
        directory=root / "qualification", daemon_id=daemon)
    proofs = {}
    with helper:
        services = {}
        for kind in ("controller", "hermes"):
            if candidates[kind]["volumes"]:
                raise ValueError("Candidate inherited anonymous volumes")
            services[kind] = dict(image=candidates[kind]["id"], network_mode="none", read_only=True,
                user="999:999", cap_drop=["ALL"], security_opt=["no-new-privileges:true"], cpus=1,
                mem_limit="512m", pids_limit=128, tmpfs=["/tmp:rw,noexec,nosuid,size=64m,mode=1777"],
                environment=dict(HOME="/tmp", UV_OFFLINE="1", UV_NO_CACHE="1", UV_PYTHON_DOWNLOADS="never",
                                 RUSTUP_TOOLCHAIN="1.88.0", RUSTUP_AUTO_INSTALL="0", PYTHONDONTWRITEBYTECODE="1"),
                working_dir="/opt/hermes" if kind == "hermes" else "/tmp",
                entrypoint=["/opt/hermes/.venv/bin/python" if kind == "hermes" else "python3", "-B", "/qa/qualify.py", kind],
                volumes=[dict(type="bind", source=str(root), target="/qa", read_only=True)])
        helper.write(services)
        for kind in services:
            policy.capacity(policy.resources(root), verify(root)["policy"]["profile"])
            log = root / "evidence" / (kind + "-qualification.log")
            # Compose's status output is not the qualifier's JSON receipt.
            with log.open("xb") as output, (root / "evidence" / (kind + "-qualification-stderr.log")).open("xb") as errors:
                result = subprocess.run(helper.run(kind), stdout=output, stderr=errors, timeout=180)
            if result.returncode:
                raise BuildFailure("qualification_command_nonzero")
            proofs[kind] = qualification_proof(root, kind)
    if json.loads(helper.journal.read_bytes()).get("phase") != "cleaned":
        raise ValueError("Qualification v2 cleanup incomplete")
    return proofs


def candidate_receipt(root):
    m = verify(root)
    p = root / "terminal-report.json"
    if p.is_symlink() or p.stat().st_size > 65536:
        raise ValueError("Bounded candidate receipt required")
    r = json.loads(p.read_bytes())
    if (r.get("state") != "qualified_candidate_images_not_native_acceptance"
            or r.get("source") != INPUTS["fleet"] or r.get("seal_sha256") != sha(root / "seal.json")
            or r.get("cleanup") != "cleaned" or r.get("native_executed") is not False
            or r.get("parity") != {"sources": True, "parents": True, "daemon": True, "permanent": True, "resources": True}
            or set(r.get("qualification", {})) != {"controller", "hermes"}
            or set(r.get("images", {})) != {"controller", "hermes", "postgres"}):
        raise ValueError("Real qualified image receipt required")
    for kind in ("controller", "hermes"):
        q = r["qualification"][kind]
        if q != qualification_proof(root, kind):
            raise ValueError("Qualification readback hash mismatch")
    for value in r["images"].values():
        if not re.fullmatch(r"sha256:[a-f0-9]{64}", value):
            raise ValueError("Closed actual image ID required")
    if not isinstance(r.get("daemon"), str) or not re.fullmatch(r"[A-Za-z0-9:._-]{1,128}", r["daemon"]):
        raise ValueError("Closed actual daemon identity required")
    if type(r.get("socket_gid")) is not int or not 0 <= r["socket_gid"] <= 4294967294:
        raise ValueError("Exact owned socket group required")
    return dict(source=INPUTS["fleet"], images=r["images"], daemon=r["daemon"],
                socket_gid=r["socket_gid"],
                seal_sha256=sha(root / "seal.json"), receipt_sha256=sha(p), policy=m["policy"])


def execute(root, ack, context, builder):
    if ack != "exclusive-source-image-build-" + root.name + "-" + INPUTS["fleet"][:12]:
        raise ValueError("Separate exact source build ACK required before any daemon IO")
    m = verify(root)
    if (root / "execution.json").exists():
        raise ValueError("One immutable build attempt only")
    profile = m["policy"]["profile"]
    policy.phase_preflight(root, profile)
    if not context or not builder:
        raise ValueError("Explicit same-daemon context and existing builder required")
    endpoint = json.loads(checked(["docker", "context", "inspect", context, "--format", "{{json .Endpoints.docker.Host}} "]))
    if endpoint != "unix:///var/run/docker.sock" or os.name != "posix":
        raise ValueError("Exact original local Linux socket required; no remote host controller")
    socket = Path("/var/run/docker.sock").stat()
    if not stat.S_ISSOCK(socket.st_mode):
        raise ValueError("Actual local daemon socket required")
    docker = ["docker", "--context", context]
    daemon_root = Path(checked(docker + ["info", "--format", "{{.DockerRootDir}}"]).decode().strip())
    if not daemon_root.is_absolute():
        raise ValueError("Measured local daemon filesystem required")
    policy.phase_preflight(daemon_root, profile)
    buildx = checked(docker + ["buildx", "inspect", builder]).decode()
    if not re.search(r"^Driver:\s+docker\s*$", buildx, re.M) or not re.search(r"^Endpoint:\s+" + re.escape(context) + r"\s*$", buildx, re.M):
        raise ValueError("Existing same-daemon docker builder only; no bootstrap")
    daemon = checked(docker + ["info", "--format", "{{.ID}}"]).decode().strip()
    platform = checked(docker + ["info", "--format", "{{.OSType}}|{{.MemTotal}}|{{.NCPU}}"]).decode().strip().split("|")
    if platform[0] != "linux" or int(platform[1]) < 6 * policy.GIB or int(platform[2]) < 2:
        raise ValueError("Linux daemon capacity insufficient")
    # Exclusive creation, before first pull/build: no repeated attempt on this packet.
    with (root / "execution.json").open("x", encoding="utf-8") as stream:
        json.dump({"started": datetime.now(timezone.utc).isoformat(), "ack_sha256": hashlib.sha256(ack.encode()).hexdigest()}, stream)
    api = maintenance(root)
    before = api.permanent_state(docker)
    parents = {}
    report = dict(state="failed", source=INPUTS["fleet"], seal_sha256=sha(root / "seal.json"),
                  daemon=daemon, socket_gid=socket.st_gid, images={}, qualification={}, parity={}, cleanup="not_started", native_executed=False,
                  runtime_ready=False, sdlc_completion=False)
    spec = compose(root)
    command = docker + ["compose", "-p", root.name, "-f", str(root / "build-compose.json")]
    os.environ["SDLC_MIN_FREE_GIB"] = str(m["policy"]["floor_gib"])
    os.environ["SDLC_RESOURCE_REGISTRY"] = str(root / "evidence/resource-registry")
    operation = "unknown"
    try:
        for kind, ref in INPUTS["parents"].items():
            operation = "parent_resources"
            policy.capacity(policy.resources(root), profile)
            policy.capacity(policy.resources(daemon_root), profile)
            operation = "parent_pull"
            logged(docker + ["pull", "--platform", "linux/amd64", ref], root / "evidence" / (kind + "-pull.log"), parent_kind=kind)
            operation = "parent_identity"
            parents[kind] = image(docker, ref)
            parent_identity(ref, parents[kind])
        candidates = {}
        for kind in ("controller", "hermes"):
            operation = "candidate_source"
            m = verify(root)
            operation = "candidate_resources"
            policy.phase_preflight(root, profile)
            policy.phase_preflight(daemon_root, profile)
            operation = "candidate_daemon"
            if checked(docker + ["info", "--format", "{{.ID}}"]).decode().strip() != daemon:
                raise BuildFailure("daemon_changed")
            tag = spec["services"][kind + "-image"]["image"]
            operation = "candidate_tag"
            tags = checked(docker + ["image", "ls", "--format", "{{.Repository}}:{{.Tag}}"], timeout=30).decode().splitlines()
            if tag in tags:
                raise BuildFailure("candidate_tag_preexisting")
            operation = "controller_input"
            if kind == "hermes" and image(docker, spec["services"]["controller-image"]["image"]) != candidates["controller"]:
                raise BuildFailure("controller_input_changed")
            operation = "candidate_build"
            logged(command + ["build", "--builder", builder, "--pull=false", "--no-cache", kind + "-image"],
                   root / "evidence" / (kind + "-build.log"), candidate_kind=kind,
                   recipe_sha256=m["files"]["context/recipes/hermes.Dockerfile"] if kind == "hermes" else None)
            operation = "candidate_metadata"
            value = image(docker, spec["services"][kind + "-image"]["image"])
            if value["os"] != "linux" or value["architecture"] != "amd64" or value["volumes"]:
                raise BuildFailure("candidate_platform_rejected")
            if any(value["labels"].get(k) != v for k, v in spec["services"][kind + "-image"]["build"]["labels"].items()):
                raise BuildFailure("candidate_owner_rejected")
            candidates[kind] = value
            operation = "candidate_resources"
            policy.capacity(policy.resources(root), profile)
            policy.capacity(policy.resources(daemon_root), profile)
        operation = "offline_qualification"
        report["qualification"] = qualify(api, root, docker, candidates, daemon)
        operation = "qualified_image_check"
        for kind, value in candidates.items():
            if image(docker, spec["services"][kind + "-image"]["image"]) != value:
                raise BuildFailure("qualified_image_changed")
        report["images"] = {kind: value["id"] for kind, value in candidates.items()} | {"postgres": parents["postgres"]["id"]}
    except BaseException as error:
        remember_failure(report, error, operation)
    finally:
        def parity(name, function):
            try:
                if not function():
                    raise BuildFailure("parity_rejected")
                report["parity"][name] = True
            except BaseException as error:
                report["parity"][name] = False
                remember_failure(report, error, "parity_" + name)
        parity("sources", lambda: verify(root) is not None)
        parity("parents", lambda: len(parents) == len(INPUTS["parents"]) and all(image(docker, INPUTS["parents"][k]) == v for k, v in parents.items()))
        parity("daemon", lambda: checked(docker + ["info", "--format", "{{.ID}}"]).decode().strip() == daemon)
        parity("permanent", lambda: api.permanent_state(docker) == before)
        def cleanup():
            # Qualification v2 owns its resources. Never bypass a failed v2 close.
            for kind in ("container", "network", "volume"):
                if checked(docker + [kind, "ls", *(["-a"] if kind == "container" else []), "-q", "--filter", "label=com.docker.compose.project=" + root.name]).strip():
                    raise BuildFailure("qualification_resources_remain")
            logged(command + ["down", "--remove-orphans"], root / "evidence/build-cleanup.log", 120)
            report["cleanup"] = "cleaned"
            return True
        parity("resources", cleanup)
        if not report.get("failure_class") and all(report["parity"].values()) and report["images"]:
            report["state"] = "qualified_candidate_images_not_native_acceptance"
        write_json(root / "terminal-report.json", report)
    public = {k: report[k] for k in ("state", "source", "images", "cleanup", "native_executed")}
    if report["state"] == "failed":
        public.update(failure_projection(report))
    print(json.dumps(public))
    return 0 if report["state"] == "qualified_candidate_images_not_native_acceptance" else 1


def main():
    parser = argparse.ArgumentParser(__doc__)
    action = parser.add_mutually_exclusive_group(required=True)
    for name in ("prepare", "verify", "execute-build"):
        action.add_argument("--" + name, action="store_true")
    for name in ("fleet", "sdk", "base", "maintenance", "hermes"):
        parser.add_argument("--" + name + "-repo", type=Path)
    parser.add_argument("--profile", choices=tuple(policy.PROFILES), default="local")
    parser.add_argument("--packet", type=Path)
    parser.add_argument("--heavy-slot-ack", default="")
    parser.add_argument("--docker-context", default="")
    parser.add_argument("--builder", default="")
    args = parser.parse_args()
    try:
        if args.prepare:
            repos = {name: getattr(args, name + "_repo") for name in ("fleet", "sdk", "base", "maintenance", "hermes")}
            if any(p is None or not p.is_absolute() for p in repos.values()):
                raise ValueError("Explicit existing Git input directories required")
            root = prepare(repos, args.profile)
        else:
            if args.packet is None:
                raise ValueError("Explicit owned packet required")
            root = args.packet.resolve()
            if args.execute_build:
                return execute(root, args.heavy_slot_ack, args.docker_context, args.builder)
            verify(root)
        print(json.dumps({"state": "prepared_not_executed", "packet": str(root), "seal_sha256": sha(root / "seal.json")}))
        return 0
    except BaseException as error:
        report = {}
        remember_failure(report, error, "unknown")
        print(json.dumps(dict(state="withheld", **failure_projection(report))))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
