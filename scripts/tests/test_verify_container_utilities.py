"""Canonical Git export regressions; no Base checkout, Rust or Docker required."""

import hashlib
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts import verify_container_utilities as verifier


class ContainerUtilityGitTests(unittest.TestCase):
    def setUp(self):
        directory = tempfile.TemporaryDirectory(prefix="fleet-utility-git-")
        self.addCleanup(directory.cleanup)
        self.root = Path(directory.name)
        self.git("init", "--quiet")
        self.git("config", "core.autocrlf", "true")
        self.sources = {name: f"# canonical {name}\nVALUE = 1\n".encode()
                        for name in verifier.NAMES}
        self.hashes = {name: hashlib.sha256(blob).hexdigest()
                       for name, blob in self.sources.items()}

    def git(self, *args, input=None):
        return subprocess.run(["git", *args], cwd=self.root, input=input,
                              capture_output=True, check=True, timeout=10).stdout.strip()

    def tree(self, sources):
        entries = []
        for name, blob in sorted(sources.items()):
            oid = self.git("hash-object", "-w", "--stdin", input=blob).decode()
            entries.append(f"100644 blob {oid}\t{name}\n")
        scripts = self.git("mktree", input="".join(entries).encode()).decode()
        return self.git("mktree", input=f"040000 tree {scripts}\tscripts\n".encode()).decode()

    def test_git_blob_export_ignores_crlf_working_tree(self):
        revision = self.tree(self.sources)
        (self.root / "scripts").mkdir()
        for name, blob in self.sources.items():
            (self.root / "scripts" / name).write_bytes(blob.replace(b"\n", b"\r\n"))
        with patch.object(verifier, "utility_hashes", return_value=self.hashes):
            self.assertEqual(verifier.verify(self.root, revision),
                             {"revision": revision, "sources": self.hashes})

    def test_crlf_git_blobs_are_not_normalized_into_trusted_sources(self):
        revision = self.tree({name: blob.replace(b"\n", b"\r\n")
                              for name, blob in self.sources.items()})
        with patch.object(verifier, "utility_hashes", return_value=self.hashes):
            with self.assertRaisesRegex(ValueError, "canonical Git blob"):
                verifier.verify(self.root, revision)

    def test_fake_contract_checkout_allows_only_newline_equivalent_source(self):
        revision = self.tree(self.sources)
        (self.root / "scripts").mkdir()
        for name, blob in self.sources.items():
            (self.root / "scripts" / name).write_bytes(blob.replace(b"\n", b"\r\n"))
        with patch.object(verifier, "utility_hashes", return_value=self.hashes):
            verifier.verify_contract_checkout(self.root, revision)
            source = self.root / "scripts" / verifier.NAMES[-1]
            source.write_bytes(source.read_bytes() + b"VALUE = 2\r\n")
            with self.assertRaisesRegex(ValueError, "fake-contract checkout"):
                verifier.verify_contract_checkout(self.root, revision)

    def test_modified_lf_git_blob_is_rejected(self):
        changed = self.sources.copy()
        changed[verifier.NAMES[-1]] += b"VALUE = 2\n"
        revision = self.tree(changed)
        with patch.object(verifier, "utility_hashes", return_value=self.hashes):
            with self.assertRaisesRegex(ValueError, "runtime_control.py"):
                verifier.verify(self.root, revision)

    def test_compiled_hashes_are_base169_canonical_git_hashes(self):
        self.assertEqual(verifier.utility_hashes(), dict(zip(verifier.NAMES, (
            "2e6bfa6907b93e6d436d2b6668ae20211aca53a64c433f7e1a98ab51245b3e89",
            "5be8066b6f7dc68f8dda7f1040c0626dad272477c019b07263821df7f86b59a2",
            "a650ed055334799af115a229c202b0f8a63a0917284d722a75f0cac19f22ebb8",
        ))))


if __name__ == "__main__":
    unittest.main()
