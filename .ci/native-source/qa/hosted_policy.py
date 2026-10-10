"""Explicit disposable-CI capacity profile. Local native guards remain 30 GiB."""
import ctypes
import os
from pathlib import Path
import shutil

GIB = 1024 ** 3
CI_ENV = {"GITHUB_ACTIONS": "true", "RUNNER_OS": "Linux", "CI": "true"}
PROFILES = {"local": {"floor_gib": 30, "phase_headroom_gib": 8},
            "disposable-ci": {"floor_gib": 5, "phase_headroom_gib": 8}}


def policy(profile):
    if profile not in PROFILES:
        raise ValueError("Closed capacity profile required")
    if profile == "disposable-ci" and (os.name != "posix" or any(os.environ.get(k) != v for k, v in CI_ENV.items())):
        raise ValueError("Disposable hosted Linux identity required")
    return dict(profile=profile, **PROFILES[profile], physical_gib=6, available_commit_gib=6)


def resources(path):
    if os.name == "nt":
        class Memory(ctypes.Structure):
            _fields_ = [("length", ctypes.c_ulong), ("load", ctypes.c_ulong)] + [
                (n, ctypes.c_ulonglong) for n in ("total_physical", "physical", "total_commit", "commit",
                                               "total_virtual", "virtual", "extended")]
        value = Memory()
        value.length = ctypes.sizeof(value)
        if not ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(value)):
            raise ValueError("Memory observation unavailable")
        physical, commit = value.physical, value.commit
    else:
        values = {k: int(v.split()[0]) * 1024 for k, v in
                  (line.split(":", 1) for line in Path("/proc/meminfo").read_text().splitlines())}
        physical, commit = values["MemAvailable"], values["CommitLimit"] - values["Committed_AS"]
        for limit, used in (("/sys/fs/cgroup/memory.max", "/sys/fs/cgroup/memory.current"),
                            ("/sys/fs/cgroup/memory/memory.limit_in_bytes", "/sys/fs/cgroup/memory/memory.usage_in_bytes")):
            if Path(limit).is_file() and Path(used).is_file():
                raw = Path(limit).read_text().strip()
                if raw != "max":
                    available = int(raw) - int(Path(used).read_text())
                    physical, commit = min(physical, available), min(commit, available)
    return dict(physical=physical, available_commit=commit, disk=shutil.disk_usage(path).free)


def capacity(value, profile, *, headroom=False):
    p = policy(profile)
    disk = p["floor_gib"] + (p["phase_headroom_gib"] if headroom else 0)
    if value["physical"] < 6 * GIB or value["available_commit"] < 6 * GIB or value["disk"] < disk * GIB:
        raise ValueError("Measured memory/cgroup/disk budget insufficient; no cleanup or waiver")
    return p


def phase_preflight(root, profile):
    """Measured floor plus estimated phase headroom, not a quota/reservation guarantee."""
    return capacity(resources(root), profile, headroom=True)
