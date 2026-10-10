"""Run only the separate real cut binary; never fabricates its exit status."""
import json
from pathlib import Path
import subprocess
import sys

if __name__ == "__main__":
    mode = sys.argv[1]
    if mode not in ("cut-initial","cut-recover"):
        raise SystemExit("Explicit cut mode required")
    with Path("/evidence",mode + ".log").open("xb") as stream:
        result = subprocess.run(["/compiled/cut-live",mode],stdout=stream,stderr=subprocess.STDOUT)
    Path("/evidence",mode + "-exit.json").write_text(json.dumps(dict(exit_code=result.returncode)) + '\n',encoding="utf-8")
    raise SystemExit(result.returncode)
