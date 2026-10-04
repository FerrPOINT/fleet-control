#!/usr/bin/env python3
"""Run native protocol acceptance in a disposable, isolated Compose project."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import uuid


PIN = "bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3"
ROOT = Path(__file__).resolve().parents[2]
TASK = "fleet-hermes-native-protocol"


def git(repo, *args):
    return subprocess.check_output(["git", "-C", str(repo), *args])


def snapshot(repo, destination):
    sha = git(repo, "rev-parse", "HEAD").decode().strip()
    if sha != PIN or git(repo, "status", "--porcelain"):
        raise RuntimeError("Hermes checkout must be clean and match the protocol pin")
    destination.mkdir(parents=True)
    archive = git(repo, "archive", "--format=tar", PIN)
    with tarfile.open(fileobj=io.BytesIO(archive)) as tar:
        for member in tar:
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
                raise RuntimeError("native source archive contains an unsupported entry")
    return hashlib.sha256(archive).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hermes", type=Path, required=True)
    parser.add_argument("--image", required=True, help="existing Base-packaged Hermes dependency image; never installed")
    parser.add_argument("--artifacts", type=Path, default=ROOT / "tmp" / "hermes-protocol-live")
    parser.add_argument("--scenario", choices=("protocol", "renderer"), default="protocol")
    parser.add_argument("--renderer-evidence-root", type=Path)
    args = parser.parse_args()
    if args.scenario == "renderer":
        if args.renderer_evidence_root is None or not args.renderer_evidence_root.is_dir() or args.renderer_evidence_root.is_symlink():
            parser.error("renderer scenario requires an owned non-linked evidence directory")
    elif args.renderer_evidence_root is not None:
        parser.error("renderer evidence is only valid for the renderer scenario")
    probe_path = Path(__file__).with_name("renderer_probe.py" if args.scenario == "renderer" else "probe.py")
    args.artifacts.mkdir(parents=True, exist_ok=True)
    metadata = json.loads(subprocess.check_output(["docker", "image", "inspect", args.image]))[0]
    if metadata["Config"].get("Labels", {}).get("sdlc.hermes.revision") != PIN:
        raise RuntimeError("dependency image does not attest the pinned Hermes revision")
    project = "sdlc-qa-hermes-protocol-" + uuid.uuid4().hex[:12]
    directory = Path(tempfile.mkdtemp(prefix=project + "-", dir=args.artifacts)).resolve()
    compose_path = directory / "compose.json"
    log_path = directory / "run.log"
    report = {"project": project, "native_source_sha": PIN, "image_id": metadata["Id"],
              "scenario":args.scenario,
              "result": "failed", "probe_sha256": hashlib.sha256(probe_path.read_bytes()).hexdigest(),
              "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}

    def dc(*arguments, capture=False, timeout=90):
        command = ["docker", "compose", "-p", project, "-f", str(compose_path), *arguments]
        result = subprocess.run(command, stdout=subprocess.PIPE if capture else None,
                                stderr=subprocess.STDOUT if capture else None, check=False, timeout=timeout)
        return result

    try:
        source = directory / "source"
        report["source_archive_sha256"] = snapshot(args.hermes, source)
        probe = directory / "probe"
        probe.mkdir()
        shutil.copyfile(probe_path, probe / "probe.py")
        output = directory / "output"
        output.mkdir()
        def bind(path, target, readonly=True):
            return {"type": "bind", "source": str(path), "target": target, "read_only": readonly}
        compose = {"services": {"probe": {
            "image": metadata["Id"], "pull_policy": "never", "init": True,
            "labels": {"sdlc.task": TASK, "sdlc.purpose": "native-hermes-http-sqlite-model-acceptance"},
            "entrypoint": ["/opt/hermes/.venv/bin/python"], "command": ["/qa/probe/probe.py"],
            "working_dir": "/qa/source", "read_only": True,
            "environment": {"PYTHONPATH": "/qa/source", "PYTHONDONTWRITEBYTECODE": "1",
                            "PYTHONUNBUFFERED": "1", "HERMES_PROTOCOL_SOURCE_SHA": PIN,
                            "HERMES_PROTOCOL_RESULT": "/qa/output/native-result.json"},
            "volumes": [bind(source, "/qa/source"), bind(probe, "/qa/probe"), bind(output, "/qa/output", False)],
            "tmpfs": ["/tmp:rw,size=512m,mode=1777"], "networks": ["qa"],
            "cpus": 2, "mem_limit": "2g", "pids_limit": 128,
        }}, "networks": {"qa": {"internal": True}}}
        if args.scenario == "renderer":
            compose["services"]["probe"]["labels"]["sdlc.purpose"] = "native-config-loader-actual-rust-renderer"
            # Rust QA owns its mode-0600 fixture as root; do not relax secret-file modes.
            # Loader-only identity, never applied to the protocol or installed runtime.
            compose["services"]["probe"].update(user="0:0", cap_drop=["ALL"],
                                                security_opt=["no-new-privileges:true"])
            compose["services"]["probe"]["volumes"].append(bind(args.renderer_evidence_root.resolve(), "/renderer-evidence"))
        compose_path.write_text(json.dumps(compose, indent=2) + "\n", encoding="utf-8")
        try:
            result = dc("up", "--abort-on-container-exit", "--exit-code-from", "probe", capture=True, timeout=600)
        except subprocess.TimeoutExpired as error:
            log_path.write_bytes(error.output or b"")
            report["test_timed_out"] = True
            raise RuntimeError("native acceptance absolute deadline exceeded") from None
        log_path.write_bytes(result.stdout)
        print(result.stdout.decode("utf-8", errors="replace"))
        report["test_exit_code"] = result.returncode
        report["log_sha256"] = hashlib.sha256(result.stdout).hexdigest()
        if result.returncode:
            raise RuntimeError("native acceptance failed; sanitized diagnostics are in the run log")
        native = json.loads((output / "native-result.json").read_text())
        expected_cases = 1 if args.scenario == "renderer" else 4
        if len(native.get("cases", [])) != expected_cases or native.get("native_source_sha") != PIN:
            raise RuntimeError("native evidence is incomplete")
        report["native_evidence"] = native
        report["result"] = "passed"
    finally:
        try:
            if compose_path.exists():
                try:
                    cleanup = dc("down", "--remove-orphans", capture=True)
                    (directory / "cleanup.log").write_bytes(cleanup.stdout)
                    report["cleanup_exit_code"] = cleanup.returncode
                    if cleanup.returncode:
                        report["result"] = "cleanup_failed"
                except (subprocess.TimeoutExpired, OSError) as error:
                    (directory / "cleanup.log").write_bytes(getattr(error, "output", None) or b"")
                    report["cleanup_error"] = type(error).__name__
                    report["result"] = "cleanup_failed"
                try:
                    remaining = dc("ps", "-a", "--format", "json", capture=True)
                    report["post_cleanup_ps_exit_code"] = remaining.returncode
                    report["post_cleanup_ps_empty"] = not remaining.stdout.strip() or remaining.stdout.strip() == b"[]"
                    if remaining.returncode or not report["post_cleanup_ps_empty"]:
                        report["result"] = "cleanup_failed"
                except (subprocess.TimeoutExpired, OSError) as error:
                    report["post_cleanup_ps_error"] = type(error).__name__
                    report["result"] = "cleanup_failed"
        finally:
            (directory / "evidence.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
            print("Evidence: " + str(directory / "evidence.json"))
        if report["result"] != "passed":
            raise RuntimeError("native protocol gate or exact Compose cleanup did not pass")


if __name__ == "__main__":
    main()
