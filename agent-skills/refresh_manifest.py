"""Mechanically refresh hashes of the versioned candidate package; no installation."""

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
COMMON = ["immutable-evidence-reporting", "project-workflow-executor", "tracker-operator"]
ROLES = {
    "project_manager": ("project-manager", "project-manager", ["draft"], ["requirements-analysis"]),
    "analyst": ("analyst", "analyst", ["analysis"], ["domain-modeling", "requirements-analysis", "workflow-writing-plans"]),
    "architect": ("architect", "architect", ["decomposition"], ["domain-modeling", "solution-architecture", "workflow-writing-plans"]),
    "developer": ("developer", "developer", ["initial", "rework"], ["forge-operator", "repo-workflow", "test-driven-development", "workflow-systematic-debugging"]),
    "reviewer": ("reviewer", "reviewer", ["delivery", "integration"], ["forge-operator", "exact-code-review"]),
    "tester": ("tester", "quality", ["delivery", "integration"], ["forge-operator", "deployed-acceptance", "workflow-systematic-debugging"]),
    "devops": ("devops", "operations", ["delivery", "integration"], ["forge-operator", "exact-sha-deployment", "workflow-systematic-debugging"]),
}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes().replace(b"\r\n", b"\n")).hexdigest()


def build() -> dict:
    roles = {}
    for role, (namespace, profile, modes, extras) in ROLES.items():
        instruction = f"roles/{role}.md"
        roles[role] = {
            "namespace": f"hermes-{namespace}",
            "profile": f"hermes-sdlc-{profile}",
            "modes": modes,
            "physicalSkills": sorted(COMMON + extras),
            "roleInstruction": {"path": instruction, "sha256": digest(ROOT / instruction)},
        }
    return {
        "schema": "base-hermes-role-skills/v1",
        "status": "candidate-not-installed",
        "catalogAuthority": {
            "purpose": "physical-skill-and-profile-validation-only",
            "selectionAuthority": "task-tracker-backend-assignment",
            "ownsRouting": False,
            "ownsModeSelection": False,
            "ownsWorkspaceSelection": False,
            "ownsPrioritySelection": False,
        },
        "sources": {"native": {
            "repository": "https://github.com/FerrPOINT/fleet-control.git",
            "revision": "SELF",
            "revisionMeaning": "exact commit containing this manifest; Workflow pins it externally",
            "hashAlgorithm": "sha256-normalized-lf-utf8",
            "skills": {p.parent.name: digest(p) for p in sorted((ROOT / "skills").glob("*/SKILL.md"))},
        }},
        "provenance": {
            "adaptation": "curated Base instructions, not verbatim upstream installation",
            "donorSkillsRevision": "46eb27f70b68cbefbf53903090f0c7f0fa68b748",
            "skillsHubRevision": "50b92c54e0e04c510c9669dfb52dab0d4632d028",
            "runtimeDependencyOnDonor": False,
        },
        "roles": roles,
    }


if __name__ == "__main__":
    (ROOT / "manifest.json").write_text(json.dumps(build(), ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
