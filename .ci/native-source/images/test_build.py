"""Pure synthetic/source regressions. None runs a daemon, compiler, resolver or network."""
import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import sys
import tempfile
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
