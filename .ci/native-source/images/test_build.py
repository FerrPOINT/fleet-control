"""Pure synthetic/source regressions. None runs a daemon, compiler, resolver or network."""
import copy
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import stat
import sys
import tempfile
import types
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import build
import recipes
from packet import git, inventory, sha, write_json
import hosted_policy as policy


class SourceTests(unittest.TestCase):
    def base_recipe(self):
        repo = Path(os.environ["FLEET_QA_BASE_REPO"])
        return git(repo, "show", build.INPUTS["base"] + ":deploy/fleet-standard.Dockerfile")

    def lock(self):
        repo = Path(os.environ["FLEET_QA_HERMES_REPO"])
        return git(repo, "show", build.INPUTS["hermes"] + ":uv.lock")

    def test_exact_source_and_parent_tuple_are_frozen(self):
        self.assertEqual(build.INPUTS["fleet"], "5db4ff92d2168c46ce96b56f37acbbf7de92db33")
        self.assertEqual(build.INPUTS["fleet_parent"], "e00b73070c327d28883c3f7a2a523b759df6662b")
        self.assertEqual(build.INPUTS["fleet_tree"], "5328b7de2d2e1922ae6748b02f974471a01d9de4")

    def test_final_source_delta_is_only_exact_fixtures_and_docs(self):
        import source_coverage
        proof = source_coverage.qualify(Path(os.environ["FLEET_QA_SOURCE_REPO"]), build.INPUTS["fleet"])
        self.assertEqual(proof["final_delta"], source_coverage.FINAL_DELTA)
        self.assertTrue(proof["production_byte_parity"])
        self.assertFalse(proof["pm_acceptance"])

    def test_five_parents_are_digest_only_not_local_refs(self):
        self.assertEqual(set(build.INPUTS["parents"]), {"rust", "postgres", "uv", "docker", "debian"})
        for ref in build.INPUTS["parents"].values():
            self.assertRegex(ref, r"^[a-z0-9.-]+/[a-z0-9/.-]+@sha256:[a-f0-9]{64}$")

    def test_hermes_source_and_count_are_unchanged(self):
        self.assertEqual(build.INPUTS["hermes"], "bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3")
        self.assertEqual(build.INPUTS["hermes_blobs"], 13770)

    def test_recipe_requires_exact_base_bytes(self):
        with self.assertRaises(ValueError):
            recipes.hermes_recipe(self.base_recipe() + b"\n", build.INPUTS, self.lock())

    def test_cold_hermes_recipe_has_no_old_server_or_dependency_parent(self):
        raw = recipes.hermes_recipe(self.base_recipe(), build.INPUTS, self.lock()).decode()
        self.assertNotIn("/fleet-server", raw)
        self.assertNotIn("fdc3e80", raw)
        self.assertNotIn("sdlc-fleet-runtime-base", raw)
        self.assertIn('CMD ["false"]', raw)
        self.assertIn("COPY sources/hermes/ /opt/hermes/", raw)

    def test_cold_recipe_retains_uv_extras_and_real_source_launchers(self):
        raw = recipes.hermes_recipe(self.base_recipe(), build.INPUTS, self.lock()).decode()
        self.assertIn("uv sync --frozen --no-dev --extra web --extra messaging --python /usr/bin/python3", raw)
        self.assertIn("COPY sources/base/deploy/fleet-hermes-launch.py", raw)
        self.assertIn("COPY sources/base/deploy/fleet-hermes-container-launch.py", raw)
        self.assertIn("useradd -u 999 -g fleet-control", raw)

    def test_no_unreviewed_registry_sdist_build_fallback(self):
        import tomllib
        raw = recipes.hermes_recipe(self.base_recipe(), build.INPUTS, self.lock()).decode()
        for package in tomllib.loads(self.lock().decode())["package"]:
            if "registry" in package.get("source", {}):
                self.assertIn("--no-build-package " + package["name"], raw)

    def test_wheel_and_setuptools_are_explicit_hash_url_constraints(self):
        raw = recipes.build_constraints(build.INPUTS).decode().splitlines()
        self.assertEqual(len(raw), 2)
        self.assertIn("setuptools-83.0.0", raw[0])
        self.assertIn("wheel-0.45.1", raw[1])
        for line in raw:
            self.assertRegex(line, r"#sha256=[a-f0-9]{64}$")

    def test_snapshot_release_hashes_are_checked_before_install(self):
        raw = (HERE / "snapshot.sh").read_text()
        for name in ("debian.InRelease", "updates.InRelease", "security.InRelease"):
            self.assertIn(build.INPUTS["artifacts"][name]["sha256"], raw)
        self.assertNotIn("trusted=yes", raw)
        self.assertNotIn("allow-unauthenticated", raw)

    def test_existing_offline_qualifier_remains_exact_lf_bytes(self):
        original = HERE.parent.parent / "fleet-native-image-build-successor-20261009/qualify.py"
        self.assertEqual((HERE / "qualify.py").read_bytes(), original.read_bytes().replace(b"\r\n", b"\n"))

    def test_compose_only_two_existing_build_roles_no_runtime_or_socket(self):
        spec = build.compose(Path("/owned/sdlc-build-fleet-native-0123456789ab"))
        self.assertEqual(set(spec["services"]), {"controller-image", "hermes-image"})
        for service in spec["services"].values():
            self.assertEqual(set(service), {"image", "build"})
            self.assertNotIn("volumes", service)

    def test_controller_keeps_original_build_volume_permissions_and_native_uid(self):
        import run
        raw = (HERE / "controller.Dockerfile").read_text()
        self.assertNotIn("USER 999", raw)
        self.assertIn("useradd --uid 999 --gid 999", raw)
        spec = run.services(Path("/synthetic"), build.INPUTS["fleet"])
        self.assertEqual(spec["fleet-backend"]["user"], "999:999")
        self.assertNotIn("user", spec["build"])

    def test_controller_adds_only_actual_socket_group_without_changing_uid_or_mount(self):
        import run
        with patch.object(run, "CANDIDATE_BINDING", {"socket_gid": 123}):
            spec = run.services(Path("/synthetic"), build.INPUTS["fleet"])
        self.assertEqual(spec["fleet-backend"]["user"], "999:999")
        self.assertEqual(set(spec["fleet-backend"]["group_add"]), {"0", "123"})
        sockets = [(name, item["read_only"]) for name, service in spec.items()
                   for item in service.get("volumes", []) if item.get("source") == "/var/run/docker.sock"]
        self.assertEqual(sockets, [("fleet-backend", True)])

    def test_existing_docker_hub_name_normalization_keeps_digest_binding(self):
        ref = build.INPUTS["parents"]["docker"]
        value = dict(repo_digests=[ref.removeprefix("docker.io/library/")], os="linux", architecture="amd64", id="sha256:" + "1" * 64)
        build.parent_identity(ref, value)
        value["repo_digests"] = ["docker@sha256:" + "2" * 64]
        with self.assertRaises(ValueError):
            build.parent_identity(ref, value)

    def test_wrong_platform_parent_refused(self):
        ref = build.INPUTS["parents"]["rust"]
        with self.assertRaises(ValueError):
            build.parent_identity(ref, dict(repo_digests=[ref], os="linux", architecture="arm64", id="sha256:" + "1" * 64))

    def test_build_ack_rejected_before_any_capacity_or_daemon_io(self):
        with patch.object(build, "verify", side_effect=AssertionError("no I/O")), self.assertRaises(ValueError):
            build.execute(Path("unused"), "", "", "")


class CapacityTests(unittest.TestCase):
    def value(self, disk=38):
        return dict(physical=6 * policy.GIB, available_commit=6 * policy.GIB, disk=disk * policy.GIB)

    def test_local_floor_still_thirty(self):
        self.assertEqual(policy.policy("local")["floor_gib"], 30)
        with self.assertRaises(ValueError):
            policy.capacity(self.value(29), "local")

    def test_local_phase_requires_budget_in_addition_to_floor(self):
        with self.assertRaises(ValueError):
            policy.capacity(self.value(37), "local", headroom=True)
        policy.capacity(self.value(38), "local", headroom=True)

    def test_ci_is_explicit_and_cannot_be_selected_on_ordinary_windows(self):
        with patch.dict(os.environ, {}, clear=True), self.assertRaises(ValueError):
            policy.policy("disposable-ci")

    def test_ci_floor_five_plus_actual_eight_budget(self):
        with patch.object(policy.os, "name", "posix"), patch.dict(os.environ, policy.CI_ENV, clear=True):
            self.assertEqual(policy.policy("disposable-ci")["floor_gib"], 5)
            with self.assertRaises(ValueError):
                policy.capacity(self.value(12), "disposable-ci", headroom=True)
            policy.capacity(self.value(13), "disposable-ci", headroom=True)

    def test_ci_each_identity_field_required(self):
        for field in policy.CI_ENV:
            env = dict(policy.CI_ENV)
            del env[field]
            with patch.object(policy.os, "name", "posix"), patch.dict(os.environ, env, clear=True), self.assertRaises(ValueError):
                policy.policy("disposable-ci")

    def test_physical_memory_never_lowered(self):
        value = self.value()
        value["physical"] -= 1
        with self.assertRaises(ValueError):
            policy.capacity(value, "local")

    def test_available_commit_never_lowered(self):
        value = self.value()
        value["available_commit"] -= 1
        with self.assertRaises(ValueError):
            policy.capacity(value, "local")

    def test_insufficient_capacity_never_allocates_or_deletes(self):
        with patch.object(policy, "resources", return_value=self.value(2)), patch.object(policy.os, "open") as opened:
            with self.assertRaises(ValueError):
                policy.phase_preflight(Path("unused"), "local")
            opened.assert_not_called()


class ReceiptTests(unittest.TestCase):
    def setup_root(self, root):
        (root / "evidence").mkdir()
        sources = root / "context/sources/hermes"
        sources.mkdir(parents=True)
        for name in ("pyproject.toml", "uv.lock"):
            (sources / name).write_bytes(b"synthetic-unit-source")
        (root / "seal.json").write_bytes(b"synthetic-unit-seal")
        qualification = {}
        for kind in ("controller", "hermes"):
            log = root / "evidence" / (kind + "-qualification.log")
            proof = dict(state="qualified", kind=kind, native_executed=False,
                         rust="rustc 1.88.0 synthetic", cargo="cargo 1.88.0 synthetic",
                         docker="Docker version synthetic", compose="synthetic",
                         source_files_verified=13770, uv="uv 0.11.6 synthetic",
                         installed_closure_check="uv_frozen_offline_check", packages=["synthetic"],
                         uv_lock_sha256=sha(sources / "uv.lock"))
            write_json(log, proof)
            qualification[kind] = {"sha256": sha(log), "state": "qualified"}
        return dict(state="qualified_candidate_images_not_native_acceptance", source=build.INPUTS["fleet"],
            seal_sha256=sha(root / "seal.json"), cleanup="cleaned", native_executed=False, daemon="synthetic-daemon", socket_gid=123,
            parity={"sources": True, "parents": True, "daemon": True, "permanent": True, "resources": True},
            qualification=qualification, images={kind: "sha256:" + str(i) * 64 for i, kind in enumerate(("controller", "hermes", "postgres"), 1)})

    def test_closed_candidate_receipt_requires_actual_source_and_cleanup(self):
        for field, value in (("source", "b" * 40), ("cleanup", "held"), ("native_executed", True), ("state", "prepared_not_executed")):
            with tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                report = self.setup_root(root)
                report[field] = value
                write_json(root / "terminal-report.json", report)
                with patch.object(build, "verify", return_value={"policy": policy.policy("local")}), self.assertRaises(ValueError):
                    build.candidate_receipt(root)

    def test_modified_qualification_hash_is_not_an_image_receipt(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            report = self.setup_root(root)
            write_json(root / "terminal-report.json", report)
            (root / "evidence/controller-qualification.log").write_bytes(b"changed")
            with patch.object(build, "verify", return_value={"policy": policy.policy("local")}), self.assertRaises(ValueError):
                build.candidate_receipt(root)

    def test_preparation_cannot_be_promoted_by_historical_image_defaults(self):
        import run
        with patch.object(run, "CANDIDATE_BINDING", None), self.assertRaises(ValueError):
            run.require_candidates()

    def test_daemon_identity_is_closed(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            report = self.setup_root(root)
            report["daemon"] = "unbounded arbitrary message\n"
            write_json(root / "terminal-report.json", report)
            with patch.object(build, "verify", return_value={"policy": policy.policy("local")}), self.assertRaises(ValueError):
                build.candidate_receipt(root)

    def test_original_qualifier_fields_are_sufficient_without_invented_pyproject_field(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            report = self.setup_root(root)
            write_json(root / "terminal-report.json", report)
            with patch.object(build, "verify", return_value={"policy": policy.policy("local")}):
                self.assertEqual(build.candidate_receipt(root)["images"], report["images"])

    def test_socket_group_is_not_an_arbitrary_receipt_field(self):
        for value in (None, True, -1, 4294967295, "123"):
            with tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                report = self.setup_root(root)
                report["socket_gid"] = value
                write_json(root / "terminal-report.json", report)
                with patch.object(build, "verify", return_value={"policy": policy.policy("local")}), self.assertRaises(ValueError):
                    build.candidate_receipt(root)


class DiagnosticTests(unittest.TestCase):
    def test_projection_has_only_closed_fields_and_boolean_parity(self):
        report = dict(failure_class="ValueError", failure_operation="parent_pull", failure_reason="unspecified",
                      parity={"sources": True, "parents": False, "daemon": "SECRET", "resources": 1, "private": "SECRET"},
                      stdout="SECRET", stderr="SECRET", args=["SECRET"], env={"TOKEN": "SECRET"})
        value = build.failure_projection(report)
        self.assertEqual(set(value), {"failure_class", "failure_operation", "failure_reason", "parity"})
        self.assertEqual(value["parity"], dict(sources=True, parents=False, daemon=None, permanent=None, resources=None))
        self.assertNotIn("SECRET", json.dumps(value))

    def test_unknown_and_secret_classifications_never_escape(self):
        class Private:
            def __str__(self):
                raise AssertionError("Private values must never be formatted")
        for value in ("SECRET=" + "x" * 100000, Private(), ["SECRET"], {"SECRET": True}, True, None):
            report = {key: value for key in ("failure_class", "failure_operation", "failure_reason", "parity")}
            result = build.failure_projection(report)
            self.assertEqual(result["failure_class"], "OtherError")
            self.assertEqual(result["failure_operation"], "unknown")
            self.assertEqual(result["failure_reason"], "unspecified")
            self.assertEqual(set(result["parity"].values()), {None})
            self.assertNotIn("SECRET", json.dumps(result))

    def test_unknown_exception_class_and_message_are_not_public(self):
        error = type("SECRET_TOKEN_CLASS", (ValueError,), {})("SECRET private command/message")
        report = {}
        build.remember_failure(report, error, "candidate_build")
        result = build.failure_projection(report)
        self.assertEqual(result["failure_class"], "OtherError")
        self.assertEqual(result["failure_reason"], "unspecified")
        self.assertNotIn("SECRET", json.dumps(result))

    def test_timeout_reason_does_not_expose_command_or_output(self):
        error = build.subprocess.TimeoutExpired(["SECRET"], 9, output=b"SECRET", stderr=b"SECRET")
        report = {}
        build.remember_failure(report, error, "parent_pull")
        result = build.failure_projection(report)
        self.assertEqual(result["failure_class"], "TimeoutExpired")
        self.assertEqual(result["failure_reason"], "process_timeout")
        self.assertNotIn("SECRET", json.dumps(result))

    def test_checked_and_logged_nonzero_are_typed_without_private_output(self):
        with tempfile.TemporaryDirectory() as folder:
            for call in (lambda: build.checked(["SECRET"]),
                         lambda: build.logged(["SECRET"], Path(folder) / "private.log")):
                with patch.object(build.subprocess, "run", return_value=types.SimpleNamespace(
                        returncode=1, stdout=b"SECRET", stderr=b"SECRET")):
                    with self.assertRaises(build.BuildFailure) as raised:
                        call()
                report = {}
                build.remember_failure(report, raised.exception, "candidate_build")
                result = build.failure_projection(report)
                self.assertEqual(result["failure_reason"], "command_nonzero")
                self.assertNotIn("SECRET", json.dumps(result))

    def test_unknown_typed_reason_is_rejected(self):
        for reason in ("SECRET", "unproved_resource_shortage", None, True):
            with self.assertRaises(ValueError):
                build.BuildFailure(reason)

    def test_first_failure_survives_later_cleanup_failure(self):
        report = {}
        build.remember_failure(report, build.BuildFailure("command_nonzero"), "candidate_build")
        build.remember_failure(report, ValueError("SECRET cleanup"), "parity_resources")
        result = build.failure_projection(report)
        self.assertEqual(result["failure_operation"], "candidate_build")
        self.assertEqual(result["failure_reason"], "command_nonzero")

    def test_projection_size_is_bounded_for_every_enum(self):
        for key, values in (("failure_class", build.FAILURE_CLASSES.values()),
                            ("failure_operation", build.OPERATIONS), ("failure_reason", build.REASONS)):
            for value in values:
                self.assertLessEqual(len(json.dumps(build.failure_projection({key: value})).encode()), 1024)

    def execute_case(self, case):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder) / (build.PREFIX + "a" * 12)
            (root / "evidence").mkdir(parents=True)
            (root / "seal.json").write_bytes(b"synthetic-unit-seal")
            spec = build.compose(root)
            socket = Path("/var/run/docker.sock")
            actual_stat = Path.stat
            daemon_root = str(Path(folder).resolve())
            api = types.SimpleNamespace(permanent_state=lambda docker: {})
            observations = []

            def checked(args, timeout=120):
                if args[1:3] == ["context", "inspect"]:
                    return b'"unix:///var/run/docker.sock"'
                if "buildx" in args:
                    return b"Driver: docker\nEndpoint: unit-context\n"
                if "info" in args:
                    field = args[-1]
                    if field == "{{.DockerRootDir}}":
                        return daemon_root.encode()
                    if field == "{{.ID}}":
                        return b"unit-daemon"
                    self.assertEqual(field, "{{.OSType}}|{{.MemTotal}}|{{.NCPU}}")
                    return f"linux|{6 * policy.GIB}|2".encode()
                if args[3:5] == ["image", "ls"]:
                    return b""
                self.assertIn(args[3], ("container", "network", "volume"))
                return b"SECRET leftover" if case == "cleanup" else b""

            def logged(args, path, timeout=1800, *, parent_kind=None, candidate_kind=None):
                observations.append("pull" if "pull" in args else "build" if "build" in args else "cleanup")
                if case == "pull" and "pull" in args or case == "build" and "build" in args:
                    raise build.BuildFailure("command_nonzero")

            def image(docker, ref):
                labels = {}
                digest = "sha256:" + "1" * 64
                for kind in ("controller", "hermes"):
                    service = spec["services"][kind + "-image"]
                    if ref == service["image"]:
                        labels = service["build"]["labels"]
                        digest = "sha256:" + ("2" if kind == "hermes" else "3") * 64
                return dict(id=digest, repo_digests=[ref], layers=["SECRET"], labels=labels,
                            os="linux", architecture="amd64", volumes={})

            def capacity(*args, **kwargs):
                if case == "resources":
                    raise ValueError("SECRET message mentions insufficient budget but has no typed proof")

            def qualify(*args):
                if case == "qualifier":
                    raise build.BuildFailure("qualification_command_nonzero")
                return {kind: {"state": "qualified", "sha256": "a" * 64} for kind in ("controller", "hermes")}

            with contextlib.ExitStack() as stack:
                for name, value in (("verify", lambda root: {"policy": policy.policy("local")}),
                                    ("maintenance", lambda root: api), ("checked", checked),
                                    ("logged", logged), ("image", image), ("qualify", qualify),
                                    ("os", types.SimpleNamespace(name="posix", environ={}))):
                    stack.enter_context(patch.object(build, name, value))
                stack.enter_context(patch.object(Path, "stat", lambda path, *a, **kw:
                    types.SimpleNamespace(st_mode=stat.S_IFSOCK, st_gid=123) if path == socket else actual_stat(path, *a, **kw)))
                stack.enter_context(patch.object(policy, "phase_preflight"))
                stack.enter_context(patch.object(policy, "resources", return_value={}))
                stack.enter_context(patch.object(policy, "capacity", capacity))
                if case == "parity":
                    api.permanent_state = lambda docker: {"changed": True} if observations else {}
                output = io.StringIO()
                stack.enter_context(contextlib.redirect_stdout(output))
                result = build.execute(root, "exclusive-source-image-build-" + root.name + "-" + build.INPUTS["fleet"][:12],
                                       "unit-context", "default")
            return result, json.loads(output.getvalue()), json.loads((root / "terminal-report.json").read_bytes())

    def test_execute_pull_failure_category_is_actual_source_operation(self):
        result, public, private = self.execute_case("pull")
        self.assertEqual(result, 1)
        self.assertEqual(public["failure_operation"], "parent_pull")
        self.assertEqual(public["failure_reason"], "command_nonzero")
        self.assertEqual(public["parity"]["parents"], False)
        self.assertEqual(public["cleanup"], "cleaned")

    def test_execute_build_failure_category_is_actual_source_operation(self):
        result, public, private = self.execute_case("build")
        self.assertEqual(result, 1)
        self.assertEqual(public["failure_operation"], "candidate_build")
        self.assertEqual(public["failure_reason"], "command_nonzero")
        self.assertEqual(public["parity"], private["parity"])

    def test_execute_qualifier_failure_category_is_actual_source_operation(self):
        result, public, private = self.execute_case("qualifier")
        self.assertEqual(result, 1)
        self.assertEqual(public["failure_operation"], "offline_qualification")
        self.assertEqual(public["failure_reason"], "qualification_command_nonzero")
        self.assertEqual(public["images"], {})

    def test_execute_resource_message_is_not_used_to_guess_a_reason(self):
        result, public, private = self.execute_case("resources")
        self.assertEqual(result, 1)
        self.assertEqual(public["failure_operation"], "parent_resources")
        self.assertEqual(public["failure_reason"], "unspecified")
        self.assertNotIn("SECRET", json.dumps(public))

    def test_execute_cleanup_failure_does_not_claim_resource_absence(self):
        result, public, private = self.execute_case("cleanup")
        self.assertEqual(result, 1)
        self.assertEqual(public["failure_operation"], "parity_resources")
        self.assertEqual(public["failure_reason"], "qualification_resources_remain")
        self.assertFalse(public["parity"]["resources"])
        self.assertEqual(public["cleanup"], "not_started")

    def test_execute_success_console_schema_is_unchanged(self):
        result, public, private = self.execute_case("success")
        self.assertEqual(result, 0)
        self.assertEqual(set(public), {"state", "source", "images", "cleanup", "native_executed"})
        self.assertEqual(public["state"], "qualified_candidate_images_not_native_acceptance")
        self.assertTrue(all(private["parity"].values()))

    def test_execute_parity_failure_preserves_images_and_typed_operation(self):
        result, public, private = self.execute_case("parity")
        self.assertEqual(result, 1)
        self.assertEqual(public["failure_operation"], "parity_permanent")
        self.assertEqual(public["failure_reason"], "parity_rejected")
        self.assertTrue(public["images"])
        self.assertFalse(public["parity"]["permanent"])

    def test_cli_withheld_error_never_exposes_unknown_class_or_message(self):
        error = type("SECRET_CLASS", (Exception,), {})("SECRET message")
        with patch.object(sys, "argv", ["build.py", "--verify", "--packet", str(HERE)]), \
                patch.object(build, "verify", side_effect=error), contextlib.redirect_stdout(io.StringIO()) as output:
            self.assertEqual(build.main(), 1)
        value = json.loads(output.getvalue())
        self.assertEqual(value["state"], "withheld")
        self.assertEqual(value["failure_class"], "OtherError")
        self.assertEqual(value["failure_operation"], "unknown")
        self.assertNotIn("SECRET", output.getvalue())

    def test_diagnostic_failure_cannot_satisfy_success_candidate_validator(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            report = ReceiptTests().setup_root(root)
            report.update(state="failed", failure_class="ValueError", failure_operation="parent_pull")
            write_json(root / "terminal-report.json", report)
            with patch.object(build, "verify", return_value={"policy": policy.policy("local")}), self.assertRaises(ValueError):
                build.candidate_receipt(root)


class ParentPullDiagnosticTests(unittest.TestCase):
    def test_parent_enum_is_exact_source_pin_keys_and_callsite_binds_kind(self):
        import ast
        self.assertEqual(build.PARENT_KINDS, set(build.INPUTS["parents"]))
        tree = ast.parse((HERE / "build.py").read_text())
        execute = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == "execute")
        calls = [node for node in ast.walk(execute) if isinstance(node, ast.Call)
                 and isinstance(node.func, ast.Name) and node.func.id == "logged"
                 and any(key.arg == "parent_kind" for key in node.keywords)]
        self.assertEqual(len(calls), 1)
        self.assertEqual(ast.dump(calls[0].keywords[0].value), ast.dump(ast.Name(id="kind", ctx=ast.Load())))
        self.assertIn("'pull', '--platform', 'linux/amd64', ref", ast.unparse(calls[0]))

    def test_explicit_public_pull_symptoms_have_only_closed_categories(self):
        samples = {
            "rate_limit": b"Error response from daemon: toomanyrequests: You have reached your unauthenticated pull rate limit.",
            "registry_denied": b"Error response from daemon: unauthorized: authentication required",
            "manifest_unavailable": b"Error response from daemon: manifest unknown: manifest unknown",
            "dns": b"lookup registry.example on 192.0.2.1:53: no such host",
            "tls": b"x509: certificate signed by unknown authority",
            "timeout": b"net/http: TLS handshake timeout",
        }
        for expected, raw in samples.items():
            with self.subTest(expected=expected):
                self.assertEqual(build.parent_pull_category(raw), expected)

    def test_ambiguous_denial_and_conflicting_symptoms_remain_unknown(self):
        for raw in (
            b"pull access denied for example, repository does not exist or may require 'docker login': denied: requested access to the resource is denied",
            b"repository does not exist or may require credentials: unauthorized: authentication required",
            b"toomanyrequests: registry rate limit\nunauthorized: authentication required",
            b"lookup registry.example: no such host\nx509: certificate signed by unknown authority",
            b"manifest unknown\ncontext deadline exceeded",
        ):
            self.assertEqual(build.parent_pull_category(raw), "unknown")

    def test_anonymous_challenge_and_unknown_text_are_not_denial_proof(self):
        for raw in (b"HTTP/1.1 401 Unauthorized\nWWW-Authenticate: Bearer SECRET",
                    b"403 Forbidden", b"404 Not Found", b"denied", b"timeout", b"SECRET", b"\xff\x00", b"",
                    "unauthorized: authentication required", None):
            self.assertEqual(build.parent_pull_category(raw), "unknown")

    def test_any_truncation_is_unknown_even_with_clear_prefix(self):
        raw = b"toomanyrequests:".ljust(build.PULL_LOG_LIMIT, b" ")
        self.assertEqual(build.parent_pull_category(raw), "rate_limit")
        self.assertEqual(build.parent_pull_category(raw + b"x"), "unknown")

    def failed_pull(self, raw, code=17, kind="rust"):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "owned-public-parent-pull.log"
            def run(args, **kwargs):
                self.assertEqual(args, ["docker", "pull", "--platform", "linux/amd64", build.INPUTS["parents"][kind]])
                self.assertEqual(kwargs["stderr"], build.subprocess.STDOUT)
                self.assertEqual(kwargs["timeout"], 1800)
                kwargs["stdout"].write(raw)
                kwargs["stdout"].flush()
                return types.SimpleNamespace(returncode=code)
            with patch.object(build.subprocess, "run", side_effect=run), self.assertRaises(build.BuildFailure) as raised:
                build.logged(["docker", "pull", "--platform", "linux/amd64", build.INPUTS["parents"][kind]], path, parent_kind=kind)
            report = {}
            build.remember_failure(report, raised.exception, "parent_pull")
            return report, build.failure_projection(report)

    def test_exact_source_kind_actual_exit_and_secret_free_projection(self):
        for kind in build.PARENT_KINDS:
            report, value = self.failed_pull(b"manifest unknown\nhttps://SECRET TOKEN=SECRET args/env SECRET", kind=kind)
            self.assertEqual(value["parent_pull"], dict(kind=kind, exit_code=17, category="manifest_unavailable"))
            self.assertEqual(value["failure_reason"], "command_nonzero")
            self.assertNotIn("SECRET", json.dumps(report))
            self.assertNotIn("SECRET", json.dumps(value))
            self.assertLessEqual(len(json.dumps(value).encode()), 1024)

    def test_owned_log_read_is_bounded_and_truncation_not_classified(self):
        with patch.object(build, "parent_pull_category", wraps=build.parent_pull_category) as classify:
            _, value = self.failed_pull(b"manifest unknown".ljust(build.PULL_LOG_LIMIT + 100, b" "))
        self.assertEqual(len(classify.call_args.args[0]), build.PULL_LOG_LIMIT + 1)
        self.assertEqual(value["parent_pull"], dict(kind="rust", exit_code=17, category="unknown"))

    def test_success_and_nonparent_failure_never_read_log(self):
        with tempfile.TemporaryDirectory() as folder:
            for code, kind in ((0, "rust"), (0, None), (1, None)):
                path = Path(folder) / (str(code) + str(kind))
                with patch.object(build.subprocess, "run", return_value=types.SimpleNamespace(returncode=code)), \
                     patch.object(build, "parent_pull_category", side_effect=AssertionError("No private log read")):
                    if code:
                        with self.assertRaises(build.BuildFailure) as raised:
                            build.logged(["SECRET"], path)
                        self.assertFalse(hasattr(raised.exception, "parent_pull"))
                    else:
                        build.logged(["SECRET"], path, parent_kind=kind)

    def test_real_process_timeout_retains_no_fabricated_exit_code(self):
        with tempfile.TemporaryDirectory() as folder:
            error = build.subprocess.TimeoutExpired(["SECRET"], 1800, output=b"SECRET")
            with patch.object(build.subprocess, "run", side_effect=error), self.assertRaises(build.subprocess.TimeoutExpired):
                build.logged(["SECRET"], Path(folder) / "owned-pull.log", parent_kind="uv")
            report = {}
            build.remember_failure(report, error, "parent_pull")
            value = build.failure_projection(report)
            self.assertEqual(value["parent_pull"], dict(kind="uv", exit_code=None, category="timeout"))
            self.assertEqual(value["failure_reason"], "process_timeout")
            self.assertNotIn("SECRET", json.dumps(value))

    def test_unknown_parent_kind_refused_before_any_process_or_file_io(self):
        for kind in ("SECRET", True, ["rust"]):
            with patch.object(Path, "open") as opened, patch.object(build.subprocess, "run") as run, self.assertRaises(ValueError):
                build.logged(["SECRET"], Path("unused"), parent_kind=kind)
            opened.assert_not_called()
            run.assert_not_called()

    def test_invalid_projection_metadata_cannot_leak_or_grow(self):
        class Private:
            def __str__(self):
                raise AssertionError("No private formatting")
        for value in ("SECRET" * 100000, Private(), True, ["SECRET"], {"SECRET": 1}, 0, -2147483649, 4294967296):
            result = build.failure_projection(dict(failure_operation="parent_pull",
                parent_pull=dict(kind=value, exit_code=value, category=value, raw="SECRET")))
            self.assertEqual(result["parent_pull"], dict(kind="unknown", exit_code=None, category="unknown"))
            self.assertLessEqual(len(json.dumps(result).encode()), 1024)
            self.assertNotIn("SECRET", json.dumps(result))

    def test_first_failure_retains_original_parent_and_actual_signal(self):
        report, expected = self.failed_pull(b"unknown", code=-9, kind="docker")
        second, _ = self.failed_pull(b"manifest unknown", code=1, kind="debian")
        error = build.BuildFailure("command_nonzero")
        error.parent_pull = second["parent_pull"]
        build.remember_failure(report, error, "parent_pull")
        build.remember_failure(report, ValueError("SECRET cleanup"), "parity_resources")
        self.assertEqual(build.failure_projection(report), expected)
        self.assertEqual(expected["parent_pull"], dict(kind="docker", exit_code=-9, category="unknown"))

    def test_log_read_error_does_not_mask_actual_process_failure(self):
        class Unreadable(io.BytesIO):
            def seek(self, *args):
                raise OSError("SECRET")
        with patch.object(Path, "open", return_value=Unreadable()), \
             patch.object(build.subprocess, "run", return_value=types.SimpleNamespace(returncode=23)), \
             self.assertRaises(build.BuildFailure) as raised:
            build.logged(["SECRET"], Path("unused"), parent_kind="postgres")
        report = {}
        build.remember_failure(report, raised.exception, "parent_pull")
        self.assertEqual(build.failure_projection(report)["parent_pull"], dict(kind="postgres", exit_code=23, category="unknown"))


class CandidateBuildDiagnosticTests(unittest.TestCase):
    def test_source_candidate_enum_and_single_callsite_preserve_build_flags(self):
        import ast
        spec = build.compose(Path("/owned/synthetic"))
        self.assertEqual(build.CANDIDATE_KINDS, {name.removesuffix("-image") for name in spec["services"]})
        execute = next(node for node in ast.parse((HERE / "build.py").read_text()).body
                       if isinstance(node, ast.FunctionDef) and node.name == "execute")
        calls = [node for node in ast.walk(execute) if isinstance(node, ast.Call)
                 and isinstance(node.func, ast.Name) and node.func.id == "logged"
                 and any(key.arg == "candidate_kind" for key in node.keywords)]
        self.assertEqual(len(calls), 1)
        self.assertEqual(ast.unparse(calls[0].keywords[0].value), "kind")
        self.assertIn("['build', '--builder', builder, '--pull=false', '--no-cache', '--provenance=mode=max', kind + '-image']", ast.unparse(calls[0]))

    def test_complete_anchored_rust_frames_only_with_actual_buildkit_prefixes(self):
        for prefix in (b"", b"#16 35.89 ", b"35.89 "):
            value = build.candidate_build_diagnostic(prefix + b"error[E0432]: SECRET import/body\n")
            self.assertEqual(value, dict(category="rust_compile", log_scope="full", rust_codes=["E0432"]))
            self.assertNotIn("SECRET", json.dumps(value))
        for raw in (b'RUN echo "error[E0432]: SECRET"\n', b"   | error[E0432]: SECRET\n",
                    b"some text error[E0432]: SECRET\n", b"warning[E0432]: SECRET\n",
                    b"error[E0432]: SECRET", b"error[E12345]: SECRET\n", b"error[Eabcd]: SECRET\n"):
            self.assertEqual(build.candidate_build_diagnostic(raw), dict(category="unknown", log_scope="full"))

    def test_rust_codes_deduplicated_at_most_eight_and_no_messages(self):
        raw = b"".join(f"#16 1.0 error[E{i:04}]: SECRET URL/path/argv/token\n".encode() for i in range(12) for _ in range(2))
        value = build.candidate_build_diagnostic(raw)
        self.assertEqual(value["rust_codes"], [f"E{i:04}" for i in range(8)])
        self.assertNotIn("SECRET", json.dumps(value))
        self.assertLessEqual(len(json.dumps(value)), 1024)

    def test_source_known_cli_compose_fetch_checksum_apt_refusals_full_or_tail(self):
        samples = {
            "docker_cli_refused": b"unknown flag: --provenance",
            "compose_config_refused": b"validating /SECRET/build-compose.json: services.controller-image.build Additional property SECRET is not allowed",
            "pinned_fetch_refused": b'{"state": "withheld", "failure_class": "ValueError"}',
            "pinned_hash_refused": b"sha256sum: WARNING: 1 computed checksum did NOT match",
            "apt_refused": b"E: The repository 'https://SECRET SECRET' is not signed.",
            "account_refused": b"groupadd: GID '999' already exists",
        }
        for expected, line in samples.items():
            for tail in (False, True):
                with self.subTest(expected=expected, tail=tail):
                    raw = (b"discarded partial SECRET\n" if tail else b"") + b"#12 3.14 " + line + b"\n"
                    value = build.candidate_build_diagnostic(raw, tail=tail)
                    self.assertEqual(value, dict(category=expected, log_scope="tail" if tail else "full"))
                    self.assertNotIn("SECRET", json.dumps(value))
        # The existing fetch contract does NOT expose whether ValueError was a hash/size/origin refusal.
        self.assertNotEqual(samples["pinned_fetch_refused"], samples["pinned_hash_refused"])

    def test_ambiguous_refusals_and_generic_summary_do_not_establish_cause(self):
        for raw in (b"unknown flag: --SECRET\n", b"ERROR: process /bin/sh SECRET did not complete successfully: exit code: 1\n",
                    b"ValueError: SECRET hash mismatch\n", b"HTTP Error 403: Forbidden\n",
                    b'{"state": "withheld", "failure_class": "SECRET"}\n',
                    b"unknown flag: --provenance\nE: Unable to locate package python3\n"):
            self.assertEqual(build.candidate_build_diagnostic(raw), dict(category="unknown", log_scope="full"))
        value = build.candidate_build_diagnostic(b"error[E0432]: SECRET\nunknown flag: --provenance\n")
        self.assertEqual(value, dict(category="unknown", log_scope="full", rust_codes=["E0432"]))

    def test_full_network_disk_dependency_symptoms_are_not_promoted_from_tail(self):
        for category, line in (("dns", b"lookup registry.example: no such host"),
                               ("no_space", b"write SECRET: no space left on device"),
                               ("dependency_resolution", b"error: failed to select a version for SECRET")):
            self.assertEqual(build.candidate_build_diagnostic(line + b"\n")["category"], category)
            self.assertEqual(build.candidate_build_diagnostic(b"partial\n" + line + b"\n", tail=True),
                             dict(category="unknown", log_scope="tail"))

    def test_tail_discards_partial_first_and_incomplete_last_frames(self):
        raw = b"error[E0001]: partial SECRET\n#9 1.2 error[E0432]: complete SECRET\nerror[E0002]: incomplete"
        self.assertEqual(build.candidate_build_diagnostic(raw, tail=True),
                         dict(category="rust_compile", log_scope="tail", rust_codes=["E0432"]))
        for raw in (b"error[E0001]: partial SECRET", b"unknown flag: --provenance\n", b"partial\nerror[E0001]: incomplete"):
            self.assertEqual(build.candidate_build_diagnostic(raw, tail=True), dict(category="unknown", log_scope="tail"))
        for raw in (b"x" * (build.BUILD_LOG_LIMIT + 1), "SECRET", None):
            self.assertEqual(build.candidate_build_diagnostic(raw), dict(category="unknown", log_scope="unavailable"))

    def failed_build(self, raw, kind="controller", code=23):
        with tempfile.TemporaryDirectory() as folder:
            def run(args, **kwargs):
                self.assertEqual(args, ["SECRET", kind])
                self.assertEqual(kwargs["stderr"], build.subprocess.STDOUT)
                self.assertEqual(kwargs["timeout"], 1800)
                kwargs["stdout"].write(raw)
                kwargs["stdout"].flush()
                return types.SimpleNamespace(returncode=code)
            with patch.object(build.subprocess, "run", side_effect=run), self.assertRaises(build.BuildFailure) as raised:
                build.logged(["SECRET", kind], Path(folder) / "SECRET-build.log", candidate_kind=kind)
            report = {}
            build.remember_failure(report, raised.exception, "candidate_build")
            return report, build.failure_projection(report)

    def test_logged_retains_exact_role_actual_exit_no_private_fields(self):
        for kind in build.CANDIDATE_KINDS:
            report, public = self.failed_build(b"#3 1.0 error[E0432]: SECRET\n", kind=kind)
            self.assertEqual(public["candidate_build"], dict(kind=kind, exit_code=23, category="rust_compile",
                                                           log_scope="full", rust_codes=["E0432"]))
            self.assertEqual(public["failure_reason"], "command_nonzero")
            self.assertNotIn("SECRET", json.dumps(report))
            self.assertNotIn("SECRET", json.dumps(public))

    def test_long_owned_log_reads_at_most_64k_tail_and_not_the_whole_log(self):
        raw = b"SECRET" * 20000 + b"\n#8 9.1 error[E0308]: SECRET\n"
        with patch.object(build, "candidate_build_diagnostic", wraps=build.candidate_build_diagnostic) as classify:
            _, public = self.failed_build(raw)
        self.assertEqual(len(classify.call_args.args[0]), build.BUILD_LOG_LIMIT)
        self.assertEqual(classify.call_args.kwargs, {"tail": True})
        self.assertEqual(public["candidate_build"], dict(kind="controller", exit_code=23, category="rust_compile",
                                                       log_scope="tail", rust_codes=["E0308"]))

    def test_success_parent_and_other_logs_never_read_candidate_diagnostics(self):
        with tempfile.TemporaryDirectory() as folder:
            for code, parent, candidate in ((0, None, "controller"), (1, "rust", None), (1, None, None)):
                with patch.object(build.subprocess, "run", return_value=types.SimpleNamespace(returncode=code)), \
                     patch.object(build, "candidate_build_diagnostic", side_effect=AssertionError("No candidate read")):
                    path = Path(folder) / (str(code) + str(parent))
                    if code:
                        with self.assertRaises(build.BuildFailure):
                            build.logged(["SECRET"], path, parent_kind=parent)
                    else:
                        build.logged(["SECRET"], path, candidate_kind=candidate)

    def test_timeout_has_null_actual_exit_and_no_exception_payload(self):
        with tempfile.TemporaryDirectory() as folder:
            error = build.subprocess.TimeoutExpired(["SECRET"], 1800, output=b"SECRET")
            with patch.object(build.subprocess, "run", side_effect=error), self.assertRaises(build.subprocess.TimeoutExpired):
                build.logged(["SECRET"], Path(folder) / "owned.log", candidate_kind="hermes")
            report = {}
            build.remember_failure(report, error, "candidate_build")
            public = build.failure_projection(report)
            self.assertEqual(public["candidate_build"], dict(kind="hermes", exit_code=None, category="timeout", log_scope="unavailable"))
            self.assertNotIn("SECRET", json.dumps(public))

    def test_invalid_metadata_and_oversized_code_lists_are_closed(self):
        class Private:
            def __str__(self):
                raise AssertionError("No private formatting")
        for value in ("SECRET" * 10000, Private(), True, ["SECRET"], None):
            public = build.failure_projection(dict(failure_operation="candidate_build", candidate_build={
                key: value for key in ("kind", "exit_code", "category", "log_scope", "rust_codes", "SECRET")}))
            self.assertEqual(public["candidate_build"], dict(kind="unknown", exit_code=None, category="unknown", log_scope="unavailable"))
            self.assertNotIn("SECRET", json.dumps(public))
        public = build.failure_projection(dict(failure_operation="candidate_build", candidate_build=dict(
            kind="controller", exit_code=4294967295, category="dependency_resolution", log_scope="tail",
            rust_codes=[f"E{i:04}" for i in range(1000)])))
        self.assertEqual(public["candidate_build"]["rust_codes"], [f"E{i:04}" for i in range(8)])
        self.assertLessEqual(len(json.dumps(public).encode()), 1024)

    def test_invalid_or_dual_kind_refused_before_process_and_file_io(self):
        for candidate, parent in (("SECRET", None), (True, None), (["controller"], None), ("controller", "rust")):
            with patch.object(Path, "open") as opened, patch.object(build.subprocess, "run") as run, self.assertRaises(ValueError):
                build.logged(["SECRET"], Path("unused"), candidate_kind=candidate, parent_kind=parent)
            opened.assert_not_called()
            run.assert_not_called()

    def test_first_failure_preserves_original_role_exit_codes_and_frames(self):
        report, expected = self.failed_build(b"error[E0432]: SECRET\n", code=-9)
        second, _ = self.failed_build(b"unknown flag: --provenance\n", kind="hermes", code=1)
        error = build.BuildFailure("command_nonzero")
        error.candidate_build = second["candidate_build"]
        build.remember_failure(report, error, "candidate_build")
        build.remember_failure(report, ValueError("SECRET cleanup"), "parity_resources")
        self.assertEqual(build.failure_projection(report), expected)

    def test_read_error_preserves_actual_failure_with_unknown_unavailable_log(self):
        class Unreadable(io.BytesIO):
            def seek(self, *args):
                raise OSError("SECRET")
        with patch.object(Path, "open", return_value=Unreadable()), \
             patch.object(build.subprocess, "run", return_value=types.SimpleNamespace(returncode=42)), \
             self.assertRaises(build.BuildFailure) as raised:
            build.logged(["SECRET"], Path("unused"), candidate_kind="controller")
        report = {}
        build.remember_failure(report, raised.exception, "candidate_build")
        self.assertEqual(build.failure_projection(report)["candidate_build"],
                         dict(kind="controller", exit_code=42, category="unknown", log_scope="unavailable"))


class ContextTests(unittest.TestCase):
    def prepared_fixture(self, parent):
        root = parent / (build.PREFIX + "0" * 12)
        (root / "context/sources").mkdir(parents=True)
        (root / "parent").mkdir()
        (root / "evidence").mkdir()
        (root / "context/recipe").write_bytes(b"synthetic context")
        write_json(root / "build-compose.json", build.compose(root))
        manifest = dict(inputs=build.INPUTS, project=root.name, policy=policy.policy("local"),
                        inherited_image_ids=[], native_executed=False,
                        policy_sha256=sha(build.ROOT / "qa/hosted_policy.py"),
                        source_coverage_sha256=sha(build.ROOT / "qa/source_coverage.py"),
                        packet_sha256=sha(build.ROOT / "qa/packet.py"),
                        files=inventory(root), source_inventory={}, source_count=0,
                        helpers={n: sha(HERE / n) for n in build.HELPERS})
        write_json(root / "manifest.json", manifest)
        write_json(root / "seal.json", {"manifest_sha256": sha(root / "manifest.json")})
        return root

    def test_extra_context_file_is_rejected_even_without_source_changes(self):
        with tempfile.TemporaryDirectory() as folder:
            parent = Path(folder)
            root = self.prepared_fixture(parent)
            with patch.object(build, "HERE", parent):
                (root / "context/undeclared").write_bytes(b"not sealed")
                with self.assertRaisesRegex(ValueError, "Current helper/source/Compose differs"):
                    build.verify(root)

    def test_extra_qualifier_parent_evidence_is_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            parent = Path(folder)
            root = self.prepared_fixture(parent)
            with patch.object(build, "HERE", parent):
                (root / "parent/undeclared").write_bytes(b"not sealed")
                with self.assertRaisesRegex(ValueError, "Current helper/source/Compose differs"):
                    build.verify(root)


if __name__ == "__main__":
    unittest.main()
