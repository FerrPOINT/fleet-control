"""Protocol4 helper qualification only: no Docker, Cargo, PG or model execution."""
import ast
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import packet
import run
import test_driver

BASELINE = "a9ced17e1f16d27e0713b065944674499b292ccd"


class Protocol4Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.text = packet.git(run.FLEET, "show", run.SOURCE_MERGED +
            ":backend/infra/src/runtime/container_lifecycle.rs").decode()
        cls.blobs = {name:packet.git(run.BASE, "show", run.BASE_SHA + ":scripts/" + name)
                     for name in run.UTILITY_HASHES}

    def test_exact_four_module_executable_seal(self):
        run.qualify_utilities(self.text, self.blobs)
        self.assertEqual(len(run.UTILITY_HASHES), 4)
        self.assertIn("runtime_replacement.py", run.UTILITY_HASHES)

    def test_receipt_three_hash_array_is_not_executable_authority(self):
        text = self.text.replace("const CONTROL_SHA256:", "const NOT_AUTHORITY:")
        self.assertIn("const UTILITY_SHA256: [&str; 3]", text)
        with self.assertRaises(ValueError):
            run.qualify_utilities(text, self.blobs)

    def test_old_or_ambiguous_executable_seal_denied(self):
        old = "a650ed055334799af115a229c202b0f8a63a0917284d722a75f0cac19f22ebb8"
        extra = '\nconst CONTROL_SHA256: [&str; 4] = [];'
        for text in (self.text.replace(run.UTILITY_HASHES["runtime_control.py"], old),
                     self.text + extra, self.text.replace("CONTROL_SHA256: [&str; 4]", "CONTROL_SHA256: [&str; 3]")):
            with self.subTest(text_sha=hashlib.sha256(text.encode()).hexdigest()), self.assertRaises(ValueError):
                run.qualify_utilities(text, self.blobs)

    def test_executable_module_order_is_closed(self):
        control, replacement = list(run.UTILITY_HASHES.values())[-2:]
        text = self.text.replace(control, "SWAP").replace(replacement, control).replace("SWAP", replacement)
        with self.assertRaises(ValueError):
            run.qualify_utilities(text, self.blobs)

    def test_executable_seal_rejects_extra_nonhash_or_expression(self):
        block = self.text.split("const CONTROL_SHA256: [&str; 4] = [",1)[1].split("];",1)[0]
        for entry in ('"not-a-hash",', 'foreign_expression(),'):
            with self.subTest(entry=entry), self.assertRaises(ValueError):
                run.qualify_utilities(self.text.replace(block, block + entry), self.blobs)

    def test_missing_extra_corrupt_or_crlf_module_denied(self):
        for name in self.blobs:
            for kind in ("missing", "corrupt", "crlf"):
                changed = dict(self.blobs)
                if kind == "missing":
                    del changed[name]
                elif kind == "crlf":
                    changed[name] = changed[name].replace(b"\n", b"\r\n")
                else:
                    changed[name] += b"# drift\n"
                with self.subTest(name=name, kind=kind), self.assertRaises(ValueError):
                    run.qualify_utilities(self.text, changed)
        with self.assertRaises(ValueError):
            run.qualify_utilities(self.text, dict(self.blobs, foreign=b"unowned"))

    def test_old_source_and_sdk_pin_fail_before_effects(self):
        with patch.object(run, "clean", side_effect=AssertionError("no source I/O for wrong SHA")):
            with self.assertRaises(ValueError):
                run.prerequisites(run.SOURCE_DRIVER_BASELINE)
        with patch.object(run, "clean"), patch.object(run, "git", return_value=b"foreign-sdk\n"):
            with self.assertRaisesRegex(ValueError, "SDK pin drift"):
                run.prerequisites(run.SOURCE_MERGED)

    def manifest(self):
        return dict(project="sdlc-qa-fleet-native4-0123456789ab", fleet_commit=run.SOURCE_MERGED,
                    recovered_activation=True, utility_hashes=run.UTILITY_HASHES)

    def physical(self):
        return dict(Id="original-cid", Name="/owned-controller", Image=run.CONTROLLER_IMAGE,
                    State=dict(Running=True))

    def test_proof_keeps_original_controller_and_exact_resource_guards(self):
        manifest, physical = self.manifest(), self.physical()
        proof = run.controller_proof(manifest, "original-engine", physical)
        self.assertEqual(proof["source_commit"], run.SOURCE_MERGED)
        self.assertEqual(proof["engine_id"], "original-engine")
        self.assertEqual(proof["agents_volume"], manifest["project"] + "_agents")
        self.assertEqual(proof["model_host"], "owned-controller")
        self.assertEqual(proof["control"], dict(python="python3",base_root="/base-runtime",context="default",
            controller_root="/controller",recovered_activation=True,
            mapping_controller=dict(container_id=physical["Id"],image_id=run.CONTROLLER_IMAGE,service="fleet-backend"),
            provisioning=dict(project=manifest["project"],image_id=run.HERMES_IMAGE,task=run.TASK,purpose=run.PURPOSE,
                user="999:999",entrypoint=["/opt/hermes/.venv/bin/python","/runtime/hermes_fixture.py"],
                network_internal=True,pids_limit=128,memory_bytes=1073741824,nano_cpus=1000000000)))

    def test_proof_rejects_unqualified_or_nonlive_controller(self):
        for key, value in (("fleet_commit",run.SOURCE_DRIVER_BASELINE),("recovered_activation",False),
                           ("recovered_activation",1),("utility_hashes",{})):
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                run.controller_proof(dict(self.manifest(), **{key:value}), "engine", self.physical())
        for physical in (dict(self.physical(), Image="foreign"),dict(self.physical(), State=dict(Running=False))):
            with self.assertRaises(ValueError):
                run.controller_proof(self.manifest(), "engine", physical)

    def test_resealed_packet_cannot_downgrade_opt_in_or_module_pin(self):
        for key, value in (("recovered_activation",False),("recovered_activation",1),
                           ("utility_hashes",{}),("base_utility_commit","ae8af2342b61090094292e75a7c23bf464757468")):
            with tempfile.TemporaryDirectory() as temp, self.subTest(key=key, value=value):
                root = Path(temp)
                owned = test_driver.DriverTests().synthetic_packet(root)
                manifest = json.loads((owned / "manifest.json").read_bytes())
                manifest[key] = value
                packet.write_json(owned / "manifest.json", manifest)
                packet.write_json(owned / "seal.json", dict(version=1,manifest_sha256=packet.sha(owned / "manifest.json")))
                with patch.object(run,"HERE",root), self.assertRaises(ValueError):
                    run.verify(owned)

    def test_base_alias_export_and_opt_in_match_current_source_contract(self):
        definitions = run.services(Path("C:/owned-packet"), run.SOURCE_MERGED)
        mounts = definitions["fleet-backend"]["volumes"]
        base = [item for item in mounts if item["target"] == "/base-runtime"]
        self.assertEqual(base, [run.mount("C:/owned-packet/input/base-runtime", "/base-runtime", True, "bind")])
        self.assertIn('"/base-runtime/deploy/fleet-hermes-container-launch.py"',
                      (run.HERE / "live/src/main.rs").read_text())
        config = packet.git(run.FLEET, "show", run.SOURCE_MERGED + ":backend/shared/src/config.rs").decode()
        self.assertIn("pub recovered_activation: bool", config)
        self.assertIn("scripts/check_recovered_activation_contract.py", run.SOURCE_PATHS)
        self.assertIn("docs/RECOVERED_ACTIVATION_CONSUMER.md", run.SOURCE_PATHS)

    def test_original_launcher_bytes_and_sdk_stay_exact(self):
        launcher = ":deploy/fleet-hermes-container-launch.py"
        self.assertEqual(packet.git(run.BASE, "show", run.BASE_SHA + launcher),
                         packet.git(run.BASE, "show", "ae8af2342b61090094292e75a7c23bf464757468" + launcher))
        self.assertEqual(run.SDK_SHA, "19a7a381ae6dbea61a643bb96189e483fa64df5c")

    def test_old_heavy_ack_cannot_authorize_new_packet(self):
        with patch.object(run, "verify", side_effect=AssertionError("no packet I/O")), self.assertRaises(ValueError):
            run.execute(Path("unused"), "exclusive-native18-b249bc895e51", "desktop-linux")

    def test_original_rust_assertions_and_runtime_fixtures_preserved(self):
        root = run.HERE.parent
        original = packet.git(root,"show",BASELINE + ":qa/live/src/main.rs")
        current = (run.HERE / "live/src/main.rs").read_bytes().replace(b"\r\n",b"\n")
        current = current.replace(b"    assert_eq!(session.task_bound, Some(false));\n", b"")
        current = current.replace(b"    assert_eq!(\n        message.request_payload_hash,\n        Some(hex::encode(Sha256::digest(\n            serde_json::to_vec(&serde_json::to_value(&request).unwrap()).unwrap()\n        )))\n    );\n", b"")
        self.assertEqual(current, original.replace(b"/base169/", b"/base-runtime/"))
        for name in ("packet.py","hermes_fixture.py","launch.py","compile_proof.py","live/Cargo.toml"):
            with self.subTest(name=name):
                self.assertEqual((run.HERE / name).read_bytes().replace(b"\r\n",b"\n"),
                                 packet.git(root,"show",BASELINE + ":qa/" + name))
        build = (run.HERE / "build.sh").read_bytes().replace(b"\r\n", b"\n")
        build = build.replace(b'"${FLEET_QA_MIN_FREE_BYTES:?sealed capacity floor required}"', b"32212254720")
        self.assertEqual(build, packet.git(root, "show", BASELINE + ":qa/build.sh"))

    def test_original_cleanup_receipt_image_and_resource_guards_preserved(self):
        before = ast.parse(packet.git(run.HERE.parent,"show",BASELINE + ":qa/run.py"))
        after = ast.parse((run.HERE / "run.py").read_bytes())
        functions = lambda tree: {node.name:ast.dump(node) for node in tree.body if isinstance(node,ast.FunctionDef)}
        for name in ("maintenance","derive_qa_lock","verify_compile_proof","validate_live",
                     "native_manifests","native_inventory","cleanup_native"):
            with self.subTest(name=name):
                self.assertEqual(functions(before)[name], functions(after)[name])
        constants = lambda tree: {node.targets[0].id:ast.literal_eval(node.value) for node in tree.body
            if isinstance(node,ast.Assign) and isinstance(node.targets[0],ast.Name)
            and node.targets[0].id in {"SDK_SHA","HERMES_SHA","MAINTENANCE_SHA","CONTROLLER_IMAGE",
                                      "HERMES_IMAGE","PG_IMAGE","VOLUMES","NETWORKS","EXPECTED_MATRIX"}}
        self.assertEqual(constants(before),constants(after))

    def test_narrow_canonical_base_export_contains_all_four_modules(self):
        with tempfile.TemporaryDirectory() as temp:
            target = Path(temp) / "base-runtime"
            paths = ["scripts/" + name for name in run.UTILITY_HASHES] + ["deploy/fleet-hermes-container-launch.py"]
            packet.export(run.BASE,run.BASE_SHA,target,paths)
            self.assertEqual(set(packet.inventory(target)),set(paths))
            for name,expected in run.UTILITY_HASHES.items():
                self.assertEqual(packet.sha(target / "scripts" / name),expected)

    def test_protocol4_opt_in_does_not_fabricate_native_coverage(self):
        for key in ("stopped_unstarted","interrupted_activation","recovered_activation_resume_rollback",
                    "next_activation_recovered_child"):
            self.assertEqual(run.HELD_UNQUALIFIED[key], "held_not_exercised")
        self.assertEqual(len(run.EXPECTED_MATRIX), 9)


if __name__ == "__main__":
    unittest.main()
