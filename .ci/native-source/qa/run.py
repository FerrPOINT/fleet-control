"""Prepare by default. Actual Docker/Hermes execution requires reviewed seal + exclusive ACK."""
from __future__ import annotations
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
import time
import tomllib
import uuid

sys.dont_write_bytecode = True
from packet import checked, exact_sha, export, git, inventory, member, sha, write_json
import hosted_policy

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
FLEET = Path(os.environ.get("FLEET_QA_SOURCE_REPO", ROOT / "fleet-control"))
SDK = Path(os.environ.get("FLEET_QA_SDK_REPO", ROOT / "services-base"))
BASE = Path(os.environ.get("FLEET_QA_BASE_REPO", ROOT / "base-runtime.git"))
HERMES = Path(os.environ.get("FLEET_QA_HERMES_REPO", ROOT.parent / "fleet-observer-hermes-bbaf7af-20261008"))
SOURCE18 = "bde6486219b40571b9b31a2d6cbaa739aad2df32"
SOURCE_MERGED = "2dcff77e01dc957e3a1d2ffda39b309835ac8d19"
SOURCE_PARENT = "febcb1757089255d0c11208ca1154f3faf789aa3"
SOURCE_DRIVER_BASELINE = "bf27a1d782f87d6f03f728831e700fda78892c8c"
SOURCE16_17 = "be1b040597a9ddd0847aca2c10fdaadb96e4c4a9"
SOURCE_UI98 = "98d950eb618071de7647e56626ad991259058be2"
SOURCE_CAPABILITY = "c1ff1a35f297e1b973bf6ae8007c1437ceede2e0"
SOURCE_ACTIVATION_FIX = "906e102bdc2e48c349d7fecbfd708ad417089768"
SDK_SHA = "19a7a381ae6dbea61a643bb96189e483fa64df5c"
BASE_SHA = "9b53de7b23593949a9e6c05bd5a4f94b930e50a0"
HERMES_SHA = "bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3"
MAINTENANCE_SHA = "2569882c3bb6a8b86ebb42367254720c5f802d5423bc98ef02c6fe3283e0874f"
UTILITY_HASHES = {"runtime_boundary.py":"2e6bfa6907b93e6d436d2b6668ae20211aca53a64c433f7e1a98ab51245b3e89",
                  "runtime_bootstrap.py":"5be8066b6f7dc68f8dda7f1040c0626dad272477c019b07263821df7f86b59a2",
                  "runtime_control.py":"1f53542606d6dc9f88f0fead7c269001f449926d8df6ccf4369d6c9874a9445b",
                  "runtime_replacement.py":"af73f6bac7dd123b2927c79dc4001edb6fcc32cf6a7de06d2be98309a2ce393e"}
# Historical parser fixtures only. Neither prepare nor execute admits these defaults:
# both require a fresh qualified build receipt, which binds all three actual IDs.
CONTROLLER_IMAGE = "sha256:076f31d5379ec5ba92d85b93d2d7006a5fe47b0eafaa400f52d9829945052816"
HERMES_IMAGE = "sha256:f42cb0b1115b587eb21c650ab9f1120c94911fc6234748c634faff929622f193"
PG_IMAGE = "sha256:b0f9560a2de083e2cc7382e75f808c7381a32852a7ec49117deedb300e552b24"
CANDIDATE_BINDING = None
CAPACITY_PROFILE = "local"
TASK = "fleet-native-protocol4"
PURPOSE = "real-rust-two-hermes-protocol4-preparation"
VOLUMES = ("agents", "controller", "compiled", "postgres", "registry", "target", "scratch")
NETWORKS = ("fleet", "build")
SOURCE_PATHS = ("backend", ".base-revision", "frontend", ".github/workflows/ci.yml", "docs/contracts",
                "docs/CONTAINER_ACTIVATION_PREFLIGHT_FIX.md", "docs/RECOVERED_ACTIVATION_CONSUMER.md",
                "scripts/check_recovered_activation_contract.py", "scripts/check_container_activation_contract.py",
                "scripts/check_container_preparation_contract.py", "scripts/verify_container_utilities.py",
                "scripts/tests/test_container_control_loader.py", "scripts/tests/test_verify_container_utilities.py")
HELPERS = ("run.py", "packet.py", "hosted_policy.py", "source_coverage.py", "hermes_fixture.py", "launch.py", "build.sh", "compile_proof.py", "test_driver.py",
           "test_protocol4.py", "README.md",
           "live/Cargo.toml", "live/src/main.rs")
EXPECTED_MATRIX = {"two_agent_preparation":"accepted", "isolation_health_chat":"accepted",
                   "idempotent_transcript":"accepted", "drain_new_generation":"accepted",
                   "readiness_exact_rollback":"accepted", "peer_unchanged":"accepted", "unknown_ack":"held",
                   "controller_restart":"accepted", "unknown_ack_after_restart":"held"}
HELD_UNQUALIFIED = {"stopped_unstarted":"held_not_exercised", "interrupted_activation":"held_not_exercised",
                    "recovered_activation_resume_rollback":"held_not_exercised",
                    "next_activation_recovered_child":"held_not_exercised"}
PROJECT_PREFIX = "sdlc-qa-fleet-native4-"
ACK_KIND = "native4"
EXTRA_SOURCE_FILES = ()
EXTRA_LOCK_MANIFESTS = ()
EXTRA_MATRIX = {}


def clean(root, commit):
    if git(root, "rev-parse", "HEAD").decode().strip() != commit or git(root, "status", "--porcelain", "--untracked-files=all"):
        raise ValueError("Own source checkout HEAD/cleanliness drift")


def ancestor(root, earlier, later):
    result = subprocess.run(["git", "--no-replace-objects", "-C", str(root), "merge-base", "--is-ancestor", earlier, later],
                            capture_output=True, timeout=30)
    if result.returncode not in (0, 1):
        raise ValueError("Prerequisite Git ancestry unavailable")
    return result.returncode == 0


def maintenance():
    if os.name == "nt":
        value = checked(["powershell.exe", "-NoProfile", "-Command",
                         "[Environment]::GetEnvironmentVariable('SDLC_MAINTENANCE_BASE','User')"]).decode().strip()
    else:
        value = os.environ.get("SDLC_MAINTENANCE_BASE", "")
    path = Path(value)
    if not path.is_absolute() or sha(path / "scripts/compose_helpers.py") != MAINTENANCE_SHA:
        raise ValueError("Exact maintenance ComposeHelper v2 is mandatory")
    return path


def qualify_utilities(text, blobs):
    """Only CONTROL_SHA256 authorizes executable modules; legacy receipts do not."""
    blocks = re.findall(r"const CONTROL_SHA256: \[&str; 4\] = \[(.*?)\];", text, re.S)
    if len(blocks) != 1 or not re.fullmatch(r'\s*"[a-f0-9]{64}"(?:\s*,\s*"[a-f0-9]{64}"){3}\s*,?\s*', blocks[0]):
        raise ValueError("Closed four-module Fleet executable seal required")
    compiled = re.findall(r'"([a-f0-9]{64})"', blocks[0]) if len(blocks) == 1 else []
    if compiled != list(UTILITY_HASHES.values()):
        raise ValueError("Exact four-module Fleet executable seal required")
    if set(blobs) != set(UTILITY_HASHES) or any(
            hashlib.sha256(blobs[name]).hexdigest() != expected for name, expected in UTILITY_HASHES.items()):
        raise ValueError("Base protocol4 canonical blob hash drift")


def prerequisites(source):
    exact_sha(source)
    if source != SOURCE_MERGED:
        raise ValueError("This successor requires exact reviewed 2dc source")
    clean(FLEET, source)
    clean(SDK, SDK_SHA)
    if git(FLEET, "show", source + ":.base-revision").decode().strip() != SDK_SHA:
        raise ValueError("SDK pin drift")
    if git(FLEET, "show", "-s", "--format=%P", source).decode().split() != [SOURCE_PARENT]:
        raise ValueError("Exact 2dc parent tuple required")
    import source_coverage
    source_coverage.qualify(FLEET, source)
    for root, commit in ((BASE, BASE_SHA), (HERMES, HERMES_SHA)):
        git(root, "cat-file", "-e", commit + "^{commit}")
    text = git(FLEET, "show", source + ":backend/infra/src/runtime/container_lifecycle.rs").decode()
    qualify_utilities(text, {name:git(BASE, "show", BASE_SHA + ":scripts/" + name) for name in UTILITY_HASHES})
    entries = git(FLEET,"ls-tree","-r","-z",source,"--",*SOURCE_PATHS).split(b"\0")
    if not any(entries):
        raise ValueError("Selected source export must not be empty")
    for entry in filter(None,entries):
        meta, name = entry.split(b"\t",1)
        mode, kind, _ = meta.decode().split()
        member(name.decode("utf-8"),mode)
        if kind != "blob":
            raise ValueError("Only safe Git source blobs may enter the packet")
    return {"contains_frozen18":ancestor(FLEET, SOURCE18, source),
            "contains_be1_fix16_and17":ancestor(FLEET, SOURCE16_17, source),
            "contains_ui98":ancestor(FLEET, SOURCE_UI98, source),
            "contains_complete_capability_fixture":ancestor(FLEET, SOURCE_CAPABILITY, source),
            "contains_activation_preflight_fix":ancestor(FLEET, SOURCE_ACTIVATION_FIX, source),
            "contains_driver_baseline":ancestor(FLEET, SOURCE_DRIVER_BASELINE, source),
            "compiled_canonical_utility_hashes":True}


def derive_qa_lock(product, manifest):
    """Append only the standalone QA root; preserve every product dependency/lock byte."""
    original = product.read_bytes()
    packages = tomllib.loads(original.decode())["package"]
    dependencies = tomllib.loads(manifest.read_text(encoding="utf-8"))["dependencies"]
    names = []
    for name, spec in dependencies.items():
        if isinstance(spec,str):
            spec = {"version":spec}
        matches = [p for p in packages if p["name"] == name]
        if "path" not in spec:
            major, minor = spec["version"].split(".")[:2]
            matches = [p for p in matches if p["version"].split(".")[0] == major
                       and (major != "0" or p["version"].split(".")[1] == minor)]
        if len(matches) != 1:
            raise ValueError("Standalone lock dependency is ambiguous: " + name)
        match = matches[0]
        names.append(name + (" " + match["version"] if sum(p["name"] == name for p in packages) > 1 else ""))
    suffix = '\n[[package]]\nname = "fleet-native-acceptance"\nversion = "0.0.0"\ndependencies = [\n'
    suffix += "".join(json.dumps(name) + ",\n" for name in sorted(names)) + "]\n"
    result = original + suffix.encode()
    if tomllib.loads(result.decode())["package"][:-1] != packages:
        raise ValueError("QA lock must not alter a product dependency")
    return result


def prepare(source):
    require_candidates()
    capacity_policy = hosted_policy.capacity(hosted_policy.resources(HERE), CAPACITY_PROFILE)
    readiness = prerequisites(source)
    maintenance()  # Hash only. No import, Docker metadata call or container creation.
    packet = HERE / (PROJECT_PREFIX + uuid.uuid4().hex[:12])
    packet.mkdir()
    try:
        inputs = packet / "input"
        export(FLEET, source, inputs / "fleet-control", SOURCE_PATHS)
        export(SDK, SDK_SHA, inputs / "services-base", ["Cargo.toml", "Cargo.lock", "crates", "LICENSE"])
        export(BASE, BASE_SHA, inputs / "base-runtime", ["scripts/" + n for n in UTILITY_HASHES] + ["deploy/fleet-hermes-container-launch.py"])
        export(HERMES, HERMES_SHA, inputs / "hermes", ["."])
        for name in HELPERS:
            path = packet / name
            path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(HERE / name, path)
        (inputs / "live/src").mkdir(parents=True)
        shutil.copyfile(HERE / "live/Cargo.toml", inputs / "live/Cargo.toml")
        shutil.copyfile(HERE / "live/src/main.rs", inputs / "live/src/main.rs")
        (inputs / "live/Cargo.lock").write_bytes(derive_qa_lock(inputs / "fleet-control/backend/Cargo.lock", inputs / "live/Cargo.toml"))
        for name in EXTRA_SOURCE_FILES:
            member(name, "100644")
            target = inputs / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(HERE / name, target)
        for name in EXTRA_LOCK_MANIFESTS:
            manifest = inputs / name
            manifest.with_name("Cargo.lock").write_bytes(derive_qa_lock(inputs / "fleet-control/backend/Cargo.lock", manifest))
        for name, expected in UTILITY_HASHES.items():
            if sha(inputs / "base-runtime/scripts" / name) != expected:
                raise ValueError("Exported Base protocol4 hash drift")
        sources = inventory(inputs)
        (inputs / "sources.sha256").write_text("".join(f"{h}  {p}\n" for p, h in sources.items()), encoding="utf-8", newline="\n")
        write_json(packet / "hermes-source.json", inventory(inputs / "hermes"))
        for name in ("proof", "evidence", "output"):
            (packet / name).mkdir()
        manifest = dict(state="prepared_not_executed", project=packet.name, task=TASK, purpose=PURPOSE,
            fleet_commit=source, sdk_commit=SDK_SHA, base_utility_commit=BASE_SHA, hermes_commit=HERMES_SHA,
            fleet_source_paths=list(SOURCE_PATHS),
            utility_hashes=UTILITY_HASHES, recovered_activation=True,
            extra_matrix=EXTRA_MATRIX,
            prerequisite_checks=readiness, execute_eligible=all(readiness.values()),
            maintenance_helper_sha256=MAINTENANCE_SHA, source_files=inventory(inputs),
            source_count=len(sources) + 1, helper_files={n:sha(packet / n) for n in HELPERS},
            hermes_manifest_sha256=sha(packet / "hermes-source.json"), expected_matrix=EXPECTED_MATRIX,
            held_unqualified=HELD_UNQUALIFIED,
            images={"controller":CONTROLLER_IMAGE,"hermes":HERMES_IMAGE,"postgres":PG_IMAGE}, images_live_verified=False,
            candidate_binding=CANDIDATE_BINDING, capacity_policy=capacity_policy,
            exact_disposable_volumes=[packet.name + "_" + n for n in VOLUMES],
            exact_disposable_networks=[packet.name + "_" + n for n in NETWORKS],
            native_networks="sealed original Base creation manifests; same exact project, unique service/generation",
            docker_socket_mount="/var/run/docker.sock: readonly controller only; explicitly required trust boundary",
            min_free_gib=capacity_policy["floor_gib"], shared_caches=[], host_ports=[], accepted_secrets=[],
            runtime_ready=False, sdlc_completion=False, actual_native_acceptance=False,
            fixture_boundary="genuine Hermes bbaf7af; synthetic local model/key, QA ACK-loss/readiness proxy; free chat only",
            source_scope="exact 5db backend+frontend+selected contracts; free-chat native only; no PM/SDLC/UI acceptance",
            unsupported=["Interrupted/recovered config activation and F6 next-child path are not exercised by this scenario",
                         "Stopped/unstarted recovery is held/unqualified; this scenario starts genuine namespaces first",
                         "Unknown native run ID cannot be recovered by fabricated request lookup or a second POST",
                         "No PM/task/model-quality/UI/published runtime acceptance"])
        write_json(packet / "manifest.json", manifest)
        write_json(packet / "seal.json", dict(version=1, manifest_sha256=sha(packet / "manifest.json")))
        verify(packet)
    except BaseException as error:
        write_json(packet / "prepare-failure.json", dict(state="failed", failure_class=type(error).__name__))
        raise
    return packet


def verify(packet):
    if packet.is_symlink() or packet.parent.resolve() != HERE.resolve() or not re.fullmatch(re.escape(PROJECT_PREFIX) + "[a-f0-9]{12}", packet.name):
        raise ValueError("Owned direct-child packet required")
    if (packet / "prepare-failure.json").exists():
        raise ValueError("Failed preparation is immutable and not executable")
    if json.loads((packet / "seal.json").read_text()) != dict(version=1, manifest_sha256=sha(packet / "manifest.json")):
        raise ValueError("Manifest seal drift")
    m = json.loads((packet / "manifest.json").read_text())
    if (m["project"] != packet.name or m["task"] != TASK or m["purpose"] != PURPOSE or m["expected_matrix"] != EXPECTED_MATRIX
            or m["held_unqualified"] != HELD_UNQUALIFIED or m["fleet_commit"] != SOURCE_MERGED
            or m["fleet_source_paths"] != list(SOURCE_PATHS)
            or m.get("utility_hashes") != UTILITY_HASHES or m.get("recovered_activation") is not True
            or m.get("extra_matrix", {}) != EXTRA_MATRIX
            or m["sdk_commit"] != SDK_SHA or m["base_utility_commit"] != BASE_SHA or m["hermes_commit"] != HERMES_SHA
            or m["maintenance_helper_sha256"] != MAINTENANCE_SHA or m["images"] != services_images()
            or m.get("candidate_binding") != CANDIDATE_BINDING
            or m.get("capacity_policy", hosted_policy.policy("local")) != hosted_policy.policy(CAPACITY_PROFILE)
            or (CANDIDATE_BINDING is not None and m.get("min_free_gib") != hosted_policy.policy(CAPACITY_PROFILE)["floor_gib"])
            or m["exact_disposable_volumes"] != [packet.name + "_" + n for n in VOLUMES]
            or m["exact_disposable_networks"] != [packet.name + "_" + n for n in NETWORKS]
            or m["source_files"] != inventory(packet / "input") or m["source_count"] != len(m["source_files"])):
        raise ValueError("Frozen contract/source/resource inventory drift")
    for n in HELPERS:
        if sha(HERE / n) != m["helper_files"][n] or sha(packet / n) != m["helper_files"][n]:
            raise ValueError("Helper freeze drift; prepare a fresh packet")
    if (sha(packet / "hermes-source.json") != m["hermes_manifest_sha256"]
            or json.loads((packet / "hermes-source.json").read_text()) != inventory(packet / "input/hermes")):
        raise ValueError("Hermes source proof drift")
    files = {p:h for p,h in m["source_files"].items() if p != "sources.sha256"}
    if (packet / "input/sources.sha256").read_bytes() != "".join(f"{h}  {p}\n" for p,h in sorted(files.items())).encode():
        raise ValueError("Canonical LF inventory drift")
    return m


def services_images():
    return dict(controller=CONTROLLER_IMAGE, hermes=HERMES_IMAGE, postgres=PG_IMAGE)


def bind_candidates(root):
    global CANDIDATE_BINDING, CONTROLLER_IMAGE, HERMES_IMAGE, PG_IMAGE, CAPACITY_PROFILE
    image_sources = HERE.parent / "images"
    sys.path.insert(0, str(image_sources))
    spec = importlib.util.spec_from_file_location("native_source_images", image_sources / "build.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    proof = module.candidate_receipt(root.resolve())
    if proof["source"] != SOURCE_MERGED:
        raise ValueError("Current-source image receipt required")
    CANDIDATE_BINDING = dict(proof, packet=str(root.resolve()))
    CONTROLLER_IMAGE, HERMES_IMAGE, PG_IMAGE = (proof["images"][n] for n in ("controller", "hermes", "postgres"))
    CAPACITY_PROFILE = proof["policy"]["profile"]


def require_candidates():
    if CANDIDATE_BINDING is None:
        raise ValueError("Fresh source-built/offline-qualified images required; no historical image fallback")
    previous = CANDIDATE_BINDING.copy()
    bind_candidates(Path(previous["packet"]))
    if previous != CANDIDATE_BINDING:
        raise ValueError("Candidate receipt changed")


def remember_failure(report, phase, error):
    report.setdefault("failure_phase", phase)
    report.setdefault("failure_class", type(error).__name__)


def verify_compile_proof(packet, manifest):
    proof = json.loads((packet / "output/compile-proof.json").read_text())
    expected = {"source_main_sha256":"live/src/main.rs", "qa_lock_sha256":"live/Cargo.lock",
                "product_lock_sha256":"fleet-control/backend/Cargo.lock"}
    if (proof.get("state") != "actual_locked_compile_verified" or proof.get("native_executed") is not False
            or type(proof.get("compiler_artifacts")) is not int or proof["compiler_artifacts"] < 1
            or not re.fullmatch("[a-f0-9]{64}",proof.get("binary_sha256",""))
            or proof.get("cargo_artifacts_sha256") != sha(packet / "output/build-artifacts.jsonl")
            or any(proof.get(key) != manifest["source_files"][path] for key,path in expected.items())):
        raise ValueError("Actual compilation/source artifact proof missing or drifted")
    return proof


def mount(source, target, ro=False, kind="volume"):
    return dict(type=kind, source=str(source).replace("\\", "/"), target=target, read_only=ro)


def services(packet, source):
    bind = lambda p,t,ro=True: mount(p,t,ro,"bind")
    check = "import hashlib,json;from pathlib import Path;m=json.loads(Path('/qa/hermes-source.json').read_text());assert all(hashlib.sha256((Path('/opt/hermes')/p).read_bytes()).hexdigest()==h for p,h in m.items());print('verified_source_files',len(m))"
    return {
        "volume-init":dict(image=CONTROLLER_IMAGE, network_mode="none", read_only=True, user="0:0", cap_drop=["ALL"], cap_add=["CHOWN","FOWNER"],
            entrypoint=["python3","-B","-c","import os;[(os.chmod(p,0o700),os.chown(p,999,999)) for p in ('/agents','/controller')]"] ,
            volumes=[mount("agents","/agents"),mount("controller","/controller")]),
        "source-check":dict(image=HERMES_IMAGE,network_mode="none",read_only=True,user="999:999",cap_drop=["ALL"],
            entrypoint=["/opt/hermes/.venv/bin/python","-B","-c",check],volumes=[bind(packet / "hermes-source.json","/qa/hermes-source.json")]),
        "build":dict(image=CONTROLLER_IMAGE,read_only=True,networks=["build"],cpus=2,mem_limit="4g",pids_limit=256,
            environment=dict(RUSTUP_TOOLCHAIN="1.88.0",FLEET_QA_MIN_FREE_BYTES=str(hosted_policy.policy(CAPACITY_PROFILE)["floor_gib"] * hosted_policy.GIB)),entrypoint=["bash","/qa/build.sh"],tmpfs=["/tmp:rw,noexec,size=256m"],
            volumes=[bind(packet / "input","/input"),bind(packet / "build.sh","/qa/build.sh"),
                     bind(packet / "compile_proof.py","/qa/compile_proof.py"),bind(packet / "packet.py","/qa/packet.py"),
                     bind(packet / "output","/output",False),
                     mount("compiled","/compiled"),mount("registry","/cargo"),mount("target","/target"),mount("scratch","/scratch")]),
        "postgres":dict(image=PG_IMAGE,networks=["fleet"],cpus=1,mem_limit="512m",pids_limit=128,
            environment=dict(POSTGRES_USER="fleet_qa",POSTGRES_DB="fleet_native",POSTGRES_HOST_AUTH_METHOD="trust"),
            healthcheck=dict(test=["CMD","pg_isready","-U","fleet_qa","-d","fleet_native"],interval="1s",timeout="2s",retries=60),
            volumes=[mount("postgres","/var/lib/postgresql/data")]),
        "fleet-backend":dict(image=CONTROLLER_IMAGE,networks=["fleet"],read_only=True,user="999:999",
            group_add=sorted({"0", str(CANDIDATE_BINDING["socket_gid"])} if CANDIDATE_BINDING is not None else {"0"}),cap_drop=["ALL"],
            cpus=2,mem_limit="2g",pids_limit=256,security_opt=["no-new-privileges:true"],tmpfs=["/tmp:rw,size=128m"],
            entrypoint=["python3","-B","-c","import time;time.sleep(10800)"],working_dir="/tmp",
            environment=dict(HOME="/tmp",QA_NATIVE_EXECUTE="1",QA_SOURCE_COMMIT=source,PYTHONDONTWRITEBYTECODE="1",PYTHONUTF8="1",
                FLEET_TEST_DATABASE_URL="postgres://fleet_qa@postgres:5432/fleet_native",FLEET_CONTROL_SECRET__LOCAL_MODEL="owned-local-model-fixture"),
            volumes=[mount("agents","/agents"),mount("controller","/controller"),mount("compiled","/compiled",True),
                     bind(packet / "input/base-runtime","/base-runtime"),bind(packet,"/qa"),bind(packet / "proof","/proof"),
                     bind(packet / "evidence","/evidence",False),bind("/var/run/docker.sock","/var/run/docker.sock")]),
    }


def validate_live(report, source):
    if (report.get("state") != "scenario_passed" or report.get("source_commit") != source
            or report.get("matrix") != EXPECTED_MATRIX or report.get("actual_rust_supervisor") is not True
            or report.get("genuine_hermes") is not True or report.get("controlled_model_prompts") != 6
            or report.get("no_second_native_post_or_generation") is not True
            or report.get("runtime_ready") is not False or report.get("sdlc_completion") is not False):
        raise ValueError("Incomplete/held/overclaimed native matrix")
    identities = report["isolation"]
    if len(identities) != 2 or len({i["agent_id"] for i in identities}) != 2 or len({i["container_id"] for i in identities}) != 2:
        raise ValueError("Two distinct real agent identities required")


def native_manifests(values, project):
    """Closed cleanup ledger from original Base creation manifests, not guessed resource names."""
    services, networks = {}, {}
    for spec in values:
        if spec.get("name") != project or set(spec.get("services", {})) == set() or len(spec["services"]) != 1:
            raise ValueError("Foreign native Compose manifest")
        name, service = next(iter(spec["services"].items()))
        if not re.fullmatch(r"agent[12]-runtime-[a-f0-9]{32}", name) or name in services:
            raise ValueError("Unexpected/duplicate native service")
        labels = service.get("labels", {})
        if labels.get("sdlc.task") != TASK or labels.get("sdlc.purpose") != PURPOSE or service.get("image") != HERMES_IMAGE:
            raise ValueError("Foreign native owner/image")
        resource = str(uuid.UUID(labels["sdlc.boundary.resource"]))
        generation = str(uuid.UUID(labels["sdlc.boundary.generation"]))
        if not name.endswith(generation.replace("-", "")) or resource == str(uuid.UUID(int=0)):
            raise ValueError("Native boundary identity drift")
        definitions = spec.get("networks", {})
        if set(definitions) != {name} or definitions[name].get("name") != project + "-" + name:
            raise ValueError("Foreign native network")
        if definitions[name].get("labels") != labels or definitions[name].get("internal") is not True:
            raise ValueError("Foreign native network custody")
        external = spec.get("volumes", {})
        if external != {"agent_storage":{"external":True,"name":project + "_agents"}}:
            raise ValueError("Foreign native volume; no cleanup adoption")
        services[name] = service
        networks[name] = definitions[name]
    return services, networks


def native_inventory(operation, docker):
    ids = operation.resources("container")
    items = json.loads(checked(docker + ["container","inspect",*ids])) if ids else []
    result = {}
    for item in items:
        labels = item["Config"].get("Labels") or {}
        if "sdlc.boundary.generation" not in labels:
            continue
        if (labels.get("com.docker.compose.project") != operation.project or labels.get("sdlc.task") != TASK
                or labels.get("sdlc.purpose") != PURPOSE or item["Image"] != HERMES_IMAGE):
            raise ValueError("Foreign native namespace inventory")
        result[item["Id"]] = dict(image_id=item["Image"],started_at=item["State"]["StartedAt"],
            generation=labels["sdlc.boundary.generation"],resource_id=labels["sdlc.boundary.resource"],
            service=labels["com.docker.compose.service"])
    if not result:
        raise ValueError("Original native namespaces absent")
    return dict(sorted(result.items()))


def cleanup_native(operation, packet, docker, controller_id):
    operation.check_endpoint()
    if sha(operation.path) != operation.manifest_hash:
        raise ValueError("Outer Compose manifest drift")
    originals = json.loads(operation.path.read_text())
    # Read private creation recipes through the exact original controller, never stdout logging.
    code = "import json;from pathlib import Path;p=Path('/controller');files=sorted(p.glob('*.create.json'));assert all(f.is_file() and not f.is_symlink() and f.stat().st_size<1048576 for f in files);print(json.dumps([json.loads(f.read_text()) for f in files]))"
    current = json.loads(checked(docker + ["container","inspect",controller_id]))[0]
    if current["Image"] != CONTROLLER_IMAGE or current["Config"]["Labels"].get("sdlc.cleanup-id") != operation.cleanup_id:
        raise ValueError("Original controller cleanup identity drift")
    specs = json.loads(checked(operation.command + ["exec","-T","fleet-backend","python3","-B","-c",code])) if current["State"]["Running"] else []
    native, networks = native_manifests(specs, packet.name)
    # Stop the one controller before resource validation, so its Rust workers cannot create more resources.
    checked(operation.command + ["stop","-t","10","fleet-backend"])
    for kind in ("container","network","volume"):
        ids = operation.resources(kind)
        items = json.loads(checked(docker + [kind,"inspect",*ids])) if ids else []
        for item in items:
            labels = item.get("Config",{}).get("Labels",{}) if kind == "container" else item.get("Labels",{})
            if labels.get("sdlc.task") != TASK or labels.get("sdlc.purpose") != PURPOSE:
                raise ValueError("Foreign resource in exact QA project; preserve all")
            if labels.get("sdlc.cleanup-id") == operation.cleanup_id:
                if labels.get("sdlc.lifecycle") != "disposable":
                    raise ValueError("Maintenance lifecycle drift")
                if kind == "container" and labels.get("com.docker.compose.service") not in originals["services"]:
                    raise ValueError("Unexpected maintenance service")
                if kind == "container":
                    service = originals["services"][labels["com.docker.compose.service"]]
                    if (item["Image"] != service["image"]
                            or labels.get("com.docker.compose.project.config_files") != str(operation.path)):
                        raise ValueError("Maintenance image/manifest drift")
                if kind != "container":
                    logical = labels.get("com.docker.compose." + kind)
                    definition = originals[kind + "s"].get(logical)
                    if not definition or definition.get("external") or item["Name"] != definition["name"]:
                        raise ValueError("Unexpected maintenance resource identity")
                continue
            if kind == "volume":
                raise ValueError("Unregistered disposable volume")
            logical = labels.get("com.docker.compose.service") if kind == "container" else labels.get("com.docker.compose.network")
            known = native if kind == "container" else networks
            if logical not in known:
                raise ValueError("Native resource has no sealed original Compose recipe")
            expected = native[logical]["labels"] if kind == "container" else networks[logical]["labels"]
            if any(labels.get(key) != value for key,value in expected.items()):
                raise ValueError("Native boundary ownership drift")
            if kind == "container" and item["Image"] != HERMES_IMAGE:
                raise ValueError("Native image drift")
            if kind == "network" and item["Name"] != networks[logical]["name"]:
                raise ValueError("Native network drift")
    merged = json.loads(json.dumps(originals))
    if set(native).intersection(merged["services"]) or set(networks).intersection(merged["networks"]):
        raise ValueError("Native/maintenance identity collision")
    merged["services"].update(native)
    merged["networks"].update(networks)
    if native:
        merged["volumes"]["agent_storage"] = dict(external=True,name=packet.name + "_agents")
    private = packet / "native-cleanup.private.json"
    write_json(private, merged)
    private.chmod(0o600)
    write_json(packet / "native-cleanup.json", dict(state="validated", manifest_sha256=sha(private),
               native_services=sorted(native), native_networks=sorted(n["name"] for n in networks.values())))
    try:
        checked(docker + ["compose","-p",packet.name,"-f",str(private),"down","--remove-orphans"])
    finally:
        # Contains synthetic per-agent tokens: retain hash/ledger, not the temporary recipe copy.
        private.unlink()
    if operation.resources("container") or operation.resources("network"):
        raise ValueError("Native resources remain; v2 cleanup cannot be claimed")


def controller_proof(manifest, engine_id, controller):
    if (manifest["fleet_commit"] != SOURCE_MERGED or manifest.get("recovered_activation") is not True
            or manifest.get("utility_hashes") != UTILITY_HASHES
            or controller["Image"] != CONTROLLER_IMAGE or controller["State"]["Running"] is not True):
        raise ValueError("Qualified opt-in source and live original controller required")
    return dict(source_commit=manifest["fleet_commit"],engine_id=engine_id,
        agents_volume=manifest["project"] + "_agents",model_host=controller["Name"].lstrip("/"),
        control=dict(python="python3",base_root="/base-runtime",context="default",controller_root="/controller",
            recovered_activation=True,
            mapping_controller=dict(container_id=controller["Id"],image_id=CONTROLLER_IMAGE,service="fleet-backend"),
            provisioning=dict(project=manifest["project"],image_id=HERMES_IMAGE,task=TASK,purpose=PURPOSE,user="999:999",
                entrypoint=["/opt/hermes/.venv/bin/python","/runtime/hermes_fixture.py"],network_internal=True,
                pids_limit=128,memory_bytes=1073741824,nano_cpus=1000000000)))


def original_scenario(operation, packet, docker, controller_id, controller, manifest, logged, report):
    """The original nine scenarios remain the default, independent of cut QA."""
    phase = "initial"
    try:
        logged(operation.command + ["exec","-d","fleet-backend","python3","-B","/qa/launch.py","initial"],"initial-launch")
        deadline = time.monotonic() + 900
        while not (packet / "evidence/before-restart.json").exists():
            if (packet / "evidence/initial-exit.json").exists() or time.monotonic() >= deadline:
                raise RuntimeError("Initial actual scenario did not reach restart boundary")
            time.sleep(1)
        report["stages"][phase] = "passed"
        generations_before = native_inventory(operation,docker)
        write_json(packet / "native-generations-before.json",generations_before)
        phase = "physical_controller_restart"
        logged(operation.command + ["restart","--no-deps","fleet-backend"],phase)
        restarted = json.loads(checked(docker + ["container","inspect",controller_id]))[0]
        if (restarted["Id"] != controller["Id"] or restarted["Image"] != controller["Image"]
                or restarted["State"]["StartedAt"] == controller["State"]["StartedAt"] or not restarted["State"]["Running"]):
            raise ValueError("Physical same-CID controller restart proof missing")
        write_json(packet / "restart-proof.json",dict(container_id=controller_id,image_id=CONTROLLER_IMAGE,
            original_started_at=controller["State"]["StartedAt"],current_started_at=restarted["State"]["StartedAt"]))
        phase = "recover"
        logged(operation.command + ["exec","-T","fleet-backend","python3","-B","/qa/launch.py","recover"],phase,300)
        live = json.loads((packet / "evidence/live-report.json").read_text())
        validate_live(live,manifest["fleet_commit"])
        generations_after = native_inventory(operation,docker)
        write_json(packet / "native-generations-after.json",generations_after)
        if generations_after != generations_before:
            raise ValueError("Physical native generation inventory changed across controller restart")
        stop = json.loads((packet / "evidence/stop-report.json").read_text())
        if stop != dict(state="original_namespaces_exited",agents=2,runtime_ready=False):
            raise ValueError("Original physical stop proof missing")
        report["scenario"] = live
    except BaseException as error:
        remember_failure(report,phase,error)
        raise


def execute(packet, ack, context, *, scenario_runner=None, services_provider=None, compile_validator=None):
    if (not re.search(r"(?<![a-z0-9])exclusive(?![a-z0-9])",ack)
            or not re.search(r"(?<![a-z0-9])" + re.escape(ACK_KIND) + r"(?![a-z0-9])",ack)
            or not 1 <= len(ack) <= 256):
        raise ValueError("Reviewed source-qualified exclusive " + ACK_KIND + " grant required")
    require_candidates()
    m = verify(packet)
    if not m["execute_eligible"] or not all(prerequisites(m["fleet_commit"]).values()):
        raise ValueError("Exact current source with all prerequisite ancestors/hashes is mandatory before execution")
    if m["fleet_commit"][:12] not in ack:
        raise ValueError("Grant must identify exact reviewed source")
    if (packet / "execution.json").exists():
        raise ValueError("One immutable attempt only; prepare a fresh packet")
    hosted_policy.phase_preflight(packet, CAPACITY_PROFILE)
    import hashlib
    write_json(packet / "execution.json", dict(started_utc=datetime.now(timezone.utc).isoformat(),ack_sha256=hashlib.sha256(ack.encode()).hexdigest()))
    report = dict(state="failed",fleet_commit=m["fleet_commit"],manifest_sha256=sha(packet / "manifest.json"),
                  source_count=m["source_count"],stages={},parity={},cleanup="not_started",runtime_ready=False,sdlc_completion=False)
    docker = ["docker","--context",context]
    operation = permanent = before = controller_id = None
    phase = "maintenance"
    def logged(args, name, timeout=3600):
        with (packet / (name + ".log")).open("xb") as stream:
            result = subprocess.run(args,stdout=stream,stderr=subprocess.STDOUT,timeout=timeout)
        if result.returncode:
            raise RuntimeError("Stage command failed")
        report["stages"][name] = "passed"
    try:
        if checked(docker + ["info", "--format", "{{.ID}}"]).decode().strip() != CANDIDATE_BINDING["daemon"]:
            raise ValueError("Build, offline qualification and native must use the same daemon")
        socket = Path("/var/run/docker.sock").stat()
        if not stat.S_ISSOCK(socket.st_mode) or socket.st_gid != CANDIDATE_BINDING["socket_gid"]:
            raise ValueError("Original locally owned daemon socket group changed")
        daemon_root = Path(checked(docker + ["info", "--format", "{{.DockerRootDir}}"]).decode().strip())
        if not daemon_root.is_absolute():
            raise ValueError("Measured local daemon filesystem required")
        hosted_policy.phase_preflight(daemon_root, CAPACITY_PROFILE)
        path = maintenance()
        sys.path.insert(0,str(path))
        spec = importlib.util.spec_from_file_location("native4_maintenance",path / "scripts/compose_helpers.py")
        api = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(api)
        permanent = api.permanent_state
        before = permanent(docker)
        write_json(packet / "permanent-before.json",before)
        for image in services_images().values():
            if checked(docker + ["image","inspect",image,"--format","{{.Id}}"]).decode().strip() != image:
                raise ValueError("Immutable local image unavailable; no build/pull fallback")
        item = json.loads(checked(docker + ["image","inspect",HERMES_IMAGE]))[0]
        if item["Config"]["Labels"].get("sdlc.hermes.revision") != HERMES_SHA:
            raise ValueError("Genuine Hermes image revision label mismatch")
        os.environ["SDLC_MIN_FREE_GIB"] = str(hosted_policy.policy(CAPACITY_PROFILE)["floor_gib"])
        operation = api.ComposeHelper(project=packet.name,task=TASK,purpose=PURPOSE,docker=docker,directory=packet / "compose")
        with operation:
            operation.write((services_provider or services)(packet,m["fleet_commit"]),volumes={n:{} for n in VOLUMES},
                            networks={"fleet":{"internal":True},"build":{}},daemon_bind_sources=("/var/run/docker.sock",))
            try:
                for name in ("source-check","volume-init","build"):
                    if name == "build":
                        hosted_policy.phase_preflight(packet, CAPACITY_PROFILE)
                        hosted_policy.phase_preflight(daemon_root, CAPACITY_PROFILE)
                    else:
                        hosted_policy.capacity(hosted_policy.resources(packet), CAPACITY_PROFILE)
                        hosted_policy.capacity(hosted_policy.resources(daemon_root), CAPACITY_PROFILE)
                    phase = name
                    logged(operation.run(name),name)
                phase = "compile_qualification"
                report["compile_proof"] = verify_compile_proof(packet,m)
                if compile_validator is not None:
                    report["cut_compile_proof"] = compile_validator(packet,m)
                report["stages"][phase] = "passed"
                phase = "startup"
                hosted_policy.capacity(hosted_policy.resources(packet), CAPACITY_PROFILE)
                hosted_policy.capacity(hosted_policy.resources(daemon_root), CAPACITY_PROFILE)
                logged(operation.command + ["up","-d","--wait","--wait-timeout","90","postgres","fleet-backend"],phase)
                controller_id = checked(operation.command + ["ps","-q","fleet-backend"]).decode().strip()
                controller = json.loads(checked(docker + ["container","inspect",controller_id]))[0]
                write_json(packet / "proof/controller.json",controller_proof(m,operation.identity,controller))
                phase = "scenario"
                (scenario_runner or original_scenario)(operation,packet,docker,controller_id,controller,m,logged,report)
            except BaseException as error:
                remember_failure(report,phase,error)
                raise
            finally:
                phase_before_cleanup = phase
                phase = "native_cleanup"
                try:
                    if controller_id:
                        cleanup_native(operation,packet,docker,controller_id)
                except BaseException as error:
                    report["native_cleanup_error"] = type(error).__name__
                    remember_failure(report,phase,error)
                    raise
                phase = phase_before_cleanup
    except BaseException as error:
        remember_failure(report,phase,error)
    finally:
        def check(name, callback):
            try:
                callback()
                report["parity"][name] = "passed"
            except BaseException as error:
                report["parity"][name] = "failed"
                report.setdefault("parity_errors",{})[name] = type(error).__name__
        def same(a,b):
            if a != b:
                raise ValueError("Evidence parity drift")
        check("sealed_sources_helpers",lambda:verify(packet))
        check("source_pins",lambda:same(prerequisites(m["fleet_commit"]),m["prerequisite_checks"]))
        check("maintenance",lambda:maintenance())
        def images_after():
            for image in services_images().values():
                same(checked(docker + ["image","inspect",image,"--format","{{.Id}}"]).decode().strip(),image)
        check("immutable_images",images_after)
        if before is not None:
            def permanent_after():
                after = permanent(docker)
                write_json(packet / "permanent-after.json",after)
                same(after,before)
            check("permanent_runtime_images",permanent_after)
        if operation is not None:
            def cleanup():
                operation.check_endpoint()
                remaining = {k:operation.resources(k) for k in ("container","network","volume")}
                names = checked(docker + ["volume","ls","--format","{{.Name}}"]).decode().splitlines()
                exact = sorted(set(names).intersection(m["exact_disposable_volumes"]))
                networks = list(m["exact_disposable_networks"])
                if (packet / "native-cleanup.json").is_file():
                    networks += json.loads((packet / "native-cleanup.json").read_text())["native_networks"]
                network_names = checked(docker + ["network","ls","--format","{{.Name}}"]).decode().splitlines()
                exact_networks = sorted(set(network_names).intersection(networks))
                journal = json.loads(operation.journal.read_text())
                report["cleanup"] = dict(remaining,exact_volume_names_still_present=exact,
                                         exact_network_names_still_present=exact_networks,journal_phase=journal["phase"])
                same(any(remaining.values()) or bool(exact) or bool(exact_networks),False)
                same((journal["version"],journal["phase"]),(2,"cleaned"))
            check("cleanup_inventory",cleanup)
        def artifacts():
            report["artifact_hashes"] = inventory(packet / "evidence")
        check("retained_evidence",artifacts)
        required = {"sealed_sources_helpers","source_pins","maintenance","immutable_images",
                    "permanent_runtime_images","cleanup_inventory","retained_evidence"}
        if report.get("scenario") and not report.get("failure_phase") and set(report["parity"]) == required and set(report["parity"].values()) == {"passed"}:
            report["state"] = "scoped_native_matrix_passed_not_sdlc_acceptance"
        report["completed_utc"] = datetime.now(timezone.utc).isoformat()
        try:
            write_json(packet / "terminal-report.json",report)
        except BaseException:
            print("TERMINAL_REPORT_UNSAVED " + json.dumps(dict(report,state="failed")))
            return 1
    print(json.dumps(dict(state=report["state"],terminal_report=str(packet / "terminal-report.json"))))
    return 0 if report["state"] == "scoped_native_matrix_passed_not_sdlc_acceptance" else 1


def main():
    parser = argparse.ArgumentParser(__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--prepare",action="store_true")
    mode.add_argument("--verify",action="store_true")
    mode.add_argument("--execute",action="store_true")
    parser.add_argument("--fleet-revision",default=SOURCE_MERGED)
    parser.add_argument("--packet",type=Path)
    parser.add_argument("--heavy-slot-ack",default="")
    parser.add_argument("--docker-context",default="desktop-linux")
    parser.add_argument("--image-build-packet",type=Path)
    args = parser.parse_args()
    try:
        if args.image_build_packet is None:
            raise ValueError("Explicit fresh qualified image packet required")
        bind_candidates(args.image_build_packet)
        if args.prepare:
            packet = prepare(args.fleet_revision)
            m = verify(packet)
            print(json.dumps(dict(state=m["state"],packet=str(packet),execute_eligible=m["execute_eligible"],seal_sha256=sha(packet / "seal.json"))))
        elif args.packet is None:
            raise ValueError("Explicit owned packet required")
        elif args.verify:
            m = verify(args.packet)
            print(json.dumps(dict(state="seal_verified_not_executed",execute_eligible=m["execute_eligible"],source_count=m["source_count"])))
        else:
            return execute(args.packet,args.heavy_slot_ack,args.docker_context)
        return 0
    except Exception as error:
        print("QA withheld: " + type(error).__name__,file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
