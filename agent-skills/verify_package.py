"""Validate candidate physical inventory and hashes; never selects routing or installs."""

import json
from pathlib import Path

from refresh_manifest import ROOT, build


def verify(root: Path = ROOT) -> None:
    manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
    # build() reads only this package; no credential, runtime, network or DB access.
    if root != ROOT:
        raise ValueError("verification root must be the versioned package")
    if manifest != build():
        raise ValueError("inventory, role instruction or hash mismatch")
    inventory = set(manifest["sources"]["native"]["skills"])
    allowed = set()
    for role in manifest["roles"].values():
        if len(role["physicalSkills"]) != len(set(role["physicalSkills"])):
            raise ValueError("duplicate skill")
        for name in role["physicalSkills"]:
            if name not in inventory:
                raise ValueError("unknown skill")
            allowed.add(name)
            text = (root / "skills" / name / "SKILL.md").read_text(encoding="utf-8")
            if not text.startswith(f"---\nname: {name}\ndescription: "):
                raise ValueError("invalid skill frontmatter")
    if allowed != inventory:
        raise ValueError("unused physical skill")
    all_files = {p.relative_to(root / "skills").as_posix() for p in (root / "skills").rglob("*") if p.is_file()}
    if all_files != {f"{name}/SKILL.md" for name in inventory}:
        raise ValueError("extra physical skill files")
    if len(manifest["roles"]) != 7 or sum(len(r["modes"]) for r in manifest["roles"].values()) != 11:
        raise ValueError("role/mode inventory mismatch")
    for text in [p.read_text(encoding="utf-8") for p in (root / "skills").glob("*/SKILL.md")]:
        if any(marker in text for marker in ("rbus ", "rwork ", "tester-browser ", "systemAccountUuid")):
            raise ValueError("legacy runtime dependency")


if __name__ == "__main__":
    verify()
    print("PASS: 7 roles / 11 modes; physical allowlists, role instructions and skill hashes")
