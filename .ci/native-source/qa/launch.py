"""Retain the real harness exit/log without emitting raw subprocess arguments."""
import json
from pathlib import Path
import subprocess
import sys

mode = sys.argv[1]
if mode not in ("initial", "recover"):
    raise SystemExit("Explicit driver mode required")
with Path("/evidence", mode + ".log").open("xb") as output:
    result = subprocess.run(["/compiled/live", mode], stdout=output, stderr=subprocess.STDOUT)
Path("/evidence", mode + "-exit.json").write_text(json.dumps({"exit_code": result.returncode}) + "\n",
                                              encoding="utf-8", newline="\n")
raise SystemExit(result.returncode)
