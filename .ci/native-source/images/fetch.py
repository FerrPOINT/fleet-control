"""Build-only, bounded hash-verified public inputs; no credentials or proxy inheritance."""
import hashlib
import json
from pathlib import Path
import sys
import urllib.parse
import urllib.request


def fetch(root):
    inputs = json.loads((root / "inputs.json").read_bytes())
    destination = root / "downloads"
    destination.mkdir(exist_ok=False)
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    for name, item in inputs["artifacts"].items():
        url = urllib.parse.urlsplit(item["url"])
        if (url.scheme != "https" or url.username or url.password or url.query or url.fragment
                or url.hostname not in {"files.pythonhosted.org", "static.rust-lang.org", "snapshot.debian.org"}):
            raise ValueError("Closed public artifact origin required")
        digest = hashlib.sha256()
        count = 0
        # The image build has no PAT, HOME credentials, or accepted runtime inputs.
        with opener.open(item["url"], timeout=30) as response, (destination / name).open("xb") as stream:
            final = urllib.parse.urlsplit(response.geturl())
            if final.scheme != "https" or final.hostname != url.hostname:
                raise ValueError("Artifact redirect left its closed origin")
            while block := response.read(65536):
                count += len(block)
                if count > item["limit"]:
                    raise ValueError("Artifact byte budget exceeded")
                digest.update(block)
                stream.write(block)
        if not count or digest.hexdigest() != item["sha256"]:
            raise ValueError("Public artifact hash mismatch")


if __name__ == "__main__":
    try:
        fetch(Path(sys.argv[1]))
    except Exception as error:
        print(json.dumps({"state": "withheld", "failure_class": type(error).__name__}))
        raise SystemExit(1)
