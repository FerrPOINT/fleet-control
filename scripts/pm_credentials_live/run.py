#!/usr/bin/env python3
"""Disposable real-producer interop; never attach to an installed runtime."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import secrets
import shutil
import subprocess
import tarfile
import tempfile
import time
import urllib.error
import urllib.request
import uuid


ROOT = Path(__file__).resolve().parents[2]
TEST = Path("backend/infra/tests/pm_credentials_live.rs")
TASK = "fleet-pm-live-interop"


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args])


def snapshot(repo, ref, destination, rust_base=False):
    sha = git(repo, "rev-parse", "--verify", f"{ref}^{{commit}}").decode().strip()
    # Auth embeds its theme primitive from frontend source at compile time.
    paths = ["Cargo.toml", "Cargo.lock", "LICENSE", "crates", "frontend/src"] if rust_base else []
    archive = git(repo, "archive", "--format=tar", sha, *paths)
    destination.mkdir(parents=True)
    # Git snapshots cannot install links or escape the dedicated temporary source root.
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        for member in tar.getmembers():
            target = (destination / member.name).resolve()
            if not target.is_relative_to(destination.resolve()):
                raise RuntimeError("unsafe source archive path")
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            elif member.isfile():
                target.parent.mkdir(parents=True, exist_ok=True)
                with tar.extractfile(member) as source, target.open("wb") as output:
                    shutil.copyfileobj(source, output)
                target.chmod(member.mode & 0o777)
            else:
                raise RuntimeError("source archive must contain only regular files/directories")
    return sha


def private_json(path, value):
    with path.open("w", encoding="utf-8", newline="\n") as output:
        json.dump(value, output, indent=2)
        output.write("\n")
    path.chmod(0o600)


def http(url, path, body=None, bearer=None):
    headers = {"Accept": "application/json"}
    data = None
    if body is not None:
        headers["Content-Type"] = "application/json"
        data = json.dumps(body).encode()
    if bearer:
        headers["Authorization"] = f"Bearer {bearer}"
    request = urllib.request.Request(url + path, data=data, headers=headers)
    try:
        with urllib.request.build_opener(urllib.request.ProxyHandler({})).open(
            request, timeout=10
        ) as response:
            data = response.read(16_385)
            if len(data) > 16_384:
                raise RuntimeError("bootstrap response exceeds bound")
            return json.loads(data)
    except urllib.error.HTTPError as error:
        # Registration/login/PAT response bodies and credentials never enter diagnostics.
        raise RuntimeError(f"bootstrap {path} failed with HTTP {error.code}") from None


def wait_health(url, path):
    deadline = time.monotonic() + 90
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    while time.monotonic() < deadline:
        try:
            with opener.open(url + path, timeout=2) as response:
                if response.status == 200:
                    return
        except (OSError, urllib.error.URLError):
            pass
        time.sleep(0.5)
    raise RuntimeError(f"producer did not become healthy: {path}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", type=Path, required=True)
    parser.add_argument("--tracker", type=Path, required=True)
    parser.add_argument("--sdk", type=Path, required=True, help="Git checkout containing each consumer's SDK pin")
    parser.add_argument("--base-ref", default="ddfb436bf2b3253561672c92b2dbc06803cabf90")
    parser.add_argument("--tracker-ref", default="af6ed1ee26f6d26534a0dd1526e3b4d168962160")
    cargo_cache = parser.add_mutually_exclusive_group(required=True)
    cargo_cache.add_argument("--cargo-cache", help="existing external Cargo cache volume")
    cargo_cache.add_argument("--cargo-cache-dir", type=Path, help="existing Cargo cache directory to bind")
    parser.add_argument("--rustup-cache", help="external Rustup volume; omit to use the image's bundled toolchain")
    parser.add_argument("--target-cache", default="fleet-pm-live-target-20261004")
    parser.add_argument("--rust-image", default="rust:1.88.0-bookworm")
    parser.add_argument("--allow-registry", action="store_true",
                        help="allow locked Cargo registry downloads when cache is incomplete")
    parser.add_argument("--artifacts", type=Path, default=ROOT / "tmp" / "pm-credentials-live")
    args = parser.parse_args()
    args.artifacts.mkdir(parents=True, exist_ok=True)
    project = "sdlc-qa-pm-live-" + uuid.uuid4().hex[:12]
    directory = Path(tempfile.mkdtemp(prefix=project + "-", dir=args.artifacts)).resolve()
    work = directory / "source"
    artifacts = directory / "artifacts"
    artifacts.mkdir()
    binaries = directory / "binaries"
    binaries.mkdir()
    fixture_path = artifacts / "fixture.json"
    compose_path = directory / "compose.json"
    evidence = {"project": project, "sdk_pins": {}, "sdk_pin_files": {}, "result": "failed",
                "registry_downloads": args.allow_registry,
                "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                "scoped_sha256": hashlib.sha256(Path(__file__).with_name("scoped.sh").read_bytes()).hexdigest()}
    compose = None

    def dc(*arguments, capture=False, stdin=None):
        command = ["docker", "compose", "-p", project, "-f", str(compose_path), *arguments]
        result = subprocess.run(command, input=stdin, stdout=subprocess.PIPE if capture else None,
                                check=False, text=True)
        if arguments[0] == "run" and arguments[-1] in {"build", "test"}:
            evidence[f"{arguments[-1]}_exit_code"] = result.returncode
        elif arguments[0] == "down":
            evidence["cleanup_exit_code"] = result.returncode
        if result.returncode:
            raise RuntimeError(f"Compose {arguments[0]} failed (exit {result.returncode})")
        return result.stdout if capture else None

    def save_compose():
        private_json(compose_path, compose)

    def published_url(service, port):
        address = dc("port", service, str(port), capture=True).strip()
        return "http://" + address

    try:
        evidence["base_sha"] = snapshot(args.base, args.base_ref, work / "auth-source", True)
        tracker_source = work / "tracker" / "task-tracker"
        fleet_source = work / "fleet" / "fleet-control"
        evidence["tracker_sha"] = snapshot(args.tracker, args.tracker_ref, tracker_source)
        evidence["fleet_sha"] = snapshot(ROOT, "HEAD", fleet_source)
        for name, product in [("fleet", fleet_source), ("tracker", tracker_source)]:
            pin_file = ".namespace-base-revision" if (product / ".namespace-base-revision").is_file() else ".base-revision"
            pin = (product / pin_file).read_text().strip()
            if len(pin) != 40 or any(char not in "0123456789abcdef" for char in pin):
                raise RuntimeError("consumer SDK pin is not an exact commit")
            sdk_sha = snapshot(args.sdk, pin, product.parent / "services-base", True)
            if sdk_sha != pin:
                raise RuntimeError("consumer SDK snapshot differs; refusing to repin")
            evidence["sdk_pins"][name] = pin
            evidence["sdk_pin_files"][name] = pin_file
        shutil.copyfile(ROOT / TEST, fleet_source / TEST)
        evidence["test_sha256"] = hashlib.sha256((fleet_source / TEST).read_bytes()).hexdigest()
        rust_env = {"RUSTUP_TOOLCHAIN": "1.88.0", "CARGO_BUILD_JOBS": "2",
                    "CARGO_HOME": "/cargo",
                    "CARGO_INCREMENTAL": "0", "CARGO_PROFILE_DEV_DEBUG": "0",
                    "CARGO_PROFILE_TEST_DEBUG": "0",
                    "LIVE_PM_ALLOW_REGISTRY": "1" if args.allow_registry else "0"}

        def service(purpose, **settings):
            return {"labels": {"sdlc.task": TASK, "sdlc.purpose": purpose},
                    "networks": ["qa"], **settings}

        def bind(source, target, readonly=True):
            return {"type": "bind", "source": str(source), "target": target,
                    "read_only": readonly}

        build_mounts = [bind(work, "/work"), bind(binaries, "/binaries", False),
                        bind(Path(__file__).with_name("scoped.sh"), "/qa/scoped.sh"),
                        "target-cache:/cache"]
        cache_volumes = {}
        if args.cargo_cache_dir is not None:
            if not args.cargo_cache_dir.is_dir():
                raise RuntimeError("Cargo cache directory does not exist")
            build_mounts.append(bind(args.cargo_cache_dir.resolve(), "/cargo", False))
        else:
            build_mounts.append("cargo-cache:/cargo")
            cache_volumes["cargo-cache"] = {"external": True, "name": args.cargo_cache}
        if args.rustup_cache:
            build_mounts.append("rustup-cache:/usr/local/rustup")
            cache_volumes["rustup-cache"] = {"external": True, "name": args.rustup_cache}
        cache_purpose = "persistent-isolated-interop-build-cache"
        target_volume = {"name": args.target_cache,
                         "labels": {"sdlc.task": TASK, "sdlc.purpose": cache_purpose}}
        existing = subprocess.run(["docker", "volume", "inspect", args.target_cache,
            "--format", "{{json .Labels}}"], capture_output=True, text=True, check=False)
        if existing.returncode == 0:
            labels = json.loads(existing.stdout) or {}
            if labels.get("sdlc.task") != TASK or labels.get("sdlc.purpose") != cache_purpose:
                raise RuntimeError("target cache is not owned by this interop harness")
            target_volume = {"external": True, "name": args.target_cache}
        compose = {"services": {
            "postgres": service("disposable-auth-tracker-databases", image="postgres:17.6-alpine",
                environment={"POSTGRES_USER": "interop", "POSTGRES_DB": "postgres",
                             "POSTGRES_HOST_AUTH_METHOD": "trust"},
                tmpfs=["/var/lib/postgresql/data:rw,size=512m"],
                healthcheck={"test": ["CMD", "pg_isready", "-U", "interop", "-d", "postgres"],
                             "interval": "1s", "timeout": "2s", "retries": 60},
                mem_limit="512m", pids_limit=128),
            "build": service("locked-scoped-producer-binaries-and-live-test", image=args.rust_image,
                environment=rust_env, volumes=build_mounts, command=["bash", "/qa/scoped.sh", "build"],
                cpus=2, mem_limit="3g", pids_limit=256),
            "auth": service("real-base-child-issuer", image=args.rust_image,
                command=["/binaries/auth-server"], volumes=[bind(binaries, "/binaries")],
                environment={"AUTH_DATABASE_URL": "postgres://interop@postgres:5432/auth_interop",
                    "AUTH_BIND": "0.0.0.0:7701", "AUTH_ISSUER": "http://auth:7701",
                    "AUTH_REGISTRATION_OPEN": "true", "AUTH_TOKEN_DELEGATION_POLICIES_JSON": "[]",
                    "RUST_LOG": "warn"}, ports=["127.0.0.1::7701"], mem_limit="512m", pids_limit=128),
            "tracker": service("real-tracker-current-assignment-boundary", image=args.rust_image,
                command=["/binaries/tracker-server"], volumes=[bind(binaries, "/binaries")],
                working_dir="/tmp", environment={
                    "TASKTRACKER_DATABASE__URL": "postgres://interop@postgres:5432/tracker_interop",
                    "TASKTRACKER_SERVER__ADDRESS": "0.0.0.0", "TASKTRACKER_SERVER__PORT": "3456",
                    "TASKTRACKER_AUTH__JWT_SECRET": secrets.token_hex(32),
                    "TASKTRACKER_STORAGE__DIR": "/tmp/uploads", "TASKTRACKER_EMAIL__ENABLED": "false",
                    "TT_AUTH__CENTRAL_JWKS_URI": "http://auth:7701/oidc/jwks",
                    "TT_AUTH__CENTRAL_ISSUER": "http://auth:7701",
                    "TASKTRACKER_SDLC__INSTANCE_ID": project, "RUST_LOG": "warn"},
                ports=["127.0.0.1::3456"], mem_limit="512m", pids_limit=128),
            "test": service("real-fleet-issuer-producer-interoperability", image=args.rust_image,
                environment={**rust_env, "FLEET_PM_LIVE_FIXTURE": "/artifacts/fixture.json"},
                volumes=[*build_mounts, bind(artifacts, "/artifacts")],
                command=["bash", "/qa/scoped.sh", "test"],
                cpus=2, mem_limit="3g", pids_limit=256),
        }, "networks": {"qa": {}}, "volumes": {
            "target-cache": target_volume,
            **cache_volumes,
        }}
        save_compose()
        print(f"LIVE_PM_INTEROP project={project} source={directory}", flush=True)
        print(json.dumps(evidence), flush=True)
        dc("config", "--quiet")
        dc("run", "--rm", "--no-deps", "-T", "build")
        dc("up", "-d", "--wait", "--wait-timeout", "90", "postgres")
        dc("exec", "-T", "postgres", "psql", "-U", "interop", "-d", "postgres", "-v", "ON_ERROR_STOP=1",
           "-c", "CREATE DATABASE auth_interop", "-c", "CREATE DATABASE tracker_interop")
        dc("up", "-d", "auth")
        base_host = published_url("auth", 7701)
        wait_health(base_host, "/health")
        password = secrets.token_urlsafe(32)
        accounts = {}
        for name in ["owner", "pm"]:
            email = f"{name}-{uuid.uuid4().hex}@example.test"
            registered = http(base_host, "/auth/register", {
                "email": email, "username": name, "password": password})
            accounts[name] = {"email": email, "subject": str(uuid.UUID(registered["id"]))}
        pm_login = http(base_host, "/auth/login", {"email": accounts["pm"]["email"], "password": password})
        parent = http(base_host, "/auth/tokens", {
            "label": "isolated interop parent", "scopes": ["task-tracker:read", "task-tracker:write"],
            "expires_in_days": 1}, pm_login["access_token"])
        compose["services"]["auth"]["environment"]["AUTH_TOKEN_DELEGATION_POLICIES_JSON"] = json.dumps([{
            "subject": accounts["pm"]["subject"], "service": "task-tracker",
            "allowed_extra_scope_prefixes": ["task-tracker:sdlc:pm:"]}])
        compose["services"]["tracker"]["environment"]["TASKTRACKER_SDLC__ORCHESTRATOR_SUBJECT"] = accounts["pm"]["subject"]
        save_compose()
        # Finish Base configuration before Tracker initializes its process-global JWKS bridge.
        dc("up", "-d", "--force-recreate", "auth")
        base_host = published_url("auth", 7701)
        wait_health(base_host, "/health")
        logins = {name: http(base_host, "/auth/login", {"email": value["email"], "password": password})
                  for name, value in accounts.items()}
        dc("up", "-d", "tracker")
        wait_health(published_url("tracker", 3456), "/api/v1/health")
        owner_local, pm_local, project_id, board_id, agent_id = [str(uuid.uuid4()) for _ in range(5)]
        seeds = f"""
INSERT INTO users(id,email,username,display_name,password_hash,central_sub,is_active,is_system_admin) VALUES
('{owner_local}','owner@interop.test','interop-owner','Owner','!','{accounts['owner']['subject']}',true,false),
('{pm_local}','pm@interop.test','interop-pm','PM','!','{accounts['pm']['subject']}',true,false);
INSERT INTO projects(id,key,name,owner_id,default_board_id)
 VALUES('{project_id}','INTEROP','Isolated interop','{owner_local}','{board_id}');
INSERT INTO boards(id,project_id,name,columns) VALUES('{board_id}','{project_id}','Interop','[]');
INSERT INTO project_members(project_id,user_id,role) VALUES('{project_id}','{pm_local}','member');
"""
        dc("exec", "-T", "postgres", "psql", "-U", "interop", "-d", "tracker_interop",
           "-v", "ON_ERROR_STOP=1", stdin=seeds)
        fixture = {"base_url": "http://auth:7701", "tracker_url": "http://tracker:3456",
            "auth_database_url": "postgres://interop@postgres:5432/auth_interop",
            "tracker_database_url": "postgres://interop@postgres:5432/tracker_interop",
            "owner_bearer": logins["owner"]["access_token"], "pm_browser_bearer": logins["pm"]["access_token"],
            "parent_pat": parent["secret"], "parent_id": str(uuid.UUID(parent["id"])),
            "owner_subject": accounts["owner"]["subject"], "machine_subject": accounts["pm"]["subject"],
            "owner_local_id": owner_local, "pm_local_id": pm_local, "project_id": project_id,
            "agent_id": agent_id, "tracker_instance_id": project}
        private_json(fixture_path, fixture)
        dc("run", "--rm", "--no-deps", "-T", "test")
        evidence["result"] = "passed"
        evidence["checks"] = ["producer-binaries-locked", "rustfmt-live-test", "clippy-live-test",
            "real-issuance", "exact-replay", "current-context", "foreign-legacy-owner-only-denials", "parent-revocation"]
    finally:
        fixture_path.unlink(missing_ok=True)
        if compose is not None:
            dc("down", "--remove-orphans")
        # No volume removal: external caches and the isolated persistent target cache survive.
        private_json(directory / "evidence.json", evidence)
        print(f"LIVE_PM_INTEROP result={evidence['result']} evidence={directory / 'evidence.json'}", flush=True)


if __name__ == "__main__":
    main()
