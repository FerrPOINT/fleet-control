"""Fast negative checks of the candidate verifier, without installation."""

import copy
import sys
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import verify_package  # noqa: E402
from refresh_manifest import build  # noqa: E402


class PackageTests(unittest.TestCase):
    def test_current_package(self):
        verify_package.verify()

    def assert_rejected(self, mutate):
        manifest = copy.deepcopy(build())
        mutate(manifest)
        with patch.object(verify_package.json, "loads", return_value=manifest):
            with self.assertRaises(ValueError):
                verify_package.verify()

    def test_wrong_hash(self):
        self.assert_rejected(lambda m: m["sources"]["native"]["skills"].update({"tracker-operator": "0" * 64}))

    def test_unknown_skill(self):
        self.assert_rejected(lambda m: m["roles"]["developer"]["physicalSkills"].append("untrusted"))

    def test_wrong_namespace(self):
        self.assert_rejected(lambda m: m["roles"]["tester"].update({"namespace": "hermes-developer"}))

    def test_extra_mode(self):
        self.assert_rejected(lambda m: m["roles"]["developer"]["modes"].append("build"))

    def test_manifest_does_not_route(self):
        self.assert_rejected(lambda m: m["catalogAuthority"].update({"ownsRouting": True}))


if __name__ == "__main__":
    unittest.main()
