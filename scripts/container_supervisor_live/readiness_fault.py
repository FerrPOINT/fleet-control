#!/opt/hermes/.venv/bin/python
"""QA-only boot delay; normal launches delegate to the frozen Base launcher."""
from pathlib import Path
import os
import sys
import time


def main():
    if "CONTAINER_QA_READINESS_DELAY" in Path("/config/SOUL.md").read_text():
        print("Owned QA readiness fault: delaying gateway boot", flush=True)
        time.sleep(120)
    os.execv(sys.executable, [sys.executable, "/runtime/hermes-container.py", *sys.argv[1:]])


if __name__ == "__main__":
    main()
