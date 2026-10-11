"""Narrow derivative of the exact Base cold Hermes stage, not a new Hermes source."""
import hashlib
import tomllib

BASE_RECIPE_SHA256 = "8b0660ae06fc0c5a71a28b51b62fb081da6e1bf7d259fb8193df0c10fcc9baa8"


def hermes_recipe(raw, inputs, lock):
    if hashlib.sha256(raw).hexdigest() != BASE_RECIPE_SHA256:
        raise ValueError("Exact reviewed Base cold recipe required")
    original = raw.decode()
    start = original.index("FROM ghcr.io/astral-sh/uv:")
    tail = original[start:]
    tail = tail.replace(tail.splitlines()[0], "FROM " + inputs["parents"]["uv"] + " AS uv", 1)
    tail = tail.replace("FROM debian:bookworm-slim", "FROM " + inputs["parents"]["debian"], 1)
    old_apt = "RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 wget python3 python3-venv git procps && rm -rf /var/lib/apt/lists/*"
    new_apt = ("COPY --from=build-inputs /etc/ssl/certs /etc/ssl/certs\n"
               "COPY recipes/snapshot.sh /build-inputs/snapshot.sh\n"
               "RUN bash /build-inputs/snapshot.sh && apt-get install -y --no-install-recommends "
               "ca-certificates libssl3 wget python3 python3-venv git procps && rm -rf /var/lib/apt/lists/*")
    if tail.count(old_apt) != 1:
        raise ValueError("Closed Base apt instruction required")
    tail = tail.replace(old_apt, new_apt)
    tail = tail.replace("COPY --from=hermes-source / /opt/hermes/", "COPY sources/hermes/ /opt/hermes/")
    tail = tail.replace("COPY fleet-hermes-launch.py", "COPY sources/base/deploy/fleet-hermes-launch.py")
    tail = tail.replace("COPY fleet-hermes-container-launch.py", "COPY sources/base/deploy/fleet-hermes-container-launch.py")
    line = "RUN uv sync --frozen --no-dev --extra web --extra messaging --python /usr/bin/python3"
    packages = tomllib.loads(lock.decode())["package"]
    # Registry sdists would introduce additional, unreviewed build dependencies.
    # Do not fall back: this bounded unit admits locked runtime wheels only, plus
    # Hermes' own editable build using the two explicitly hashed build constraints.
    # The project and pip CLIs use different package-vector flags and delimiters.
    registry = [p["name"] for p in packages if "registry" in p.get("source", {})]
    no_build = "--no-build-package '" + " ".join(registry) + "'"
    only_binary = "--only-binary '" + ",".join(registry) + "'"
    tail = tail.replace(line, "COPY recipes/build-constraints.txt /build-inputs/build-constraints.txt\n" +
                        line + " --no-install-project " + no_build +
                        " && uv pip install --python /opt/hermes/.venv/bin/python --no-deps --editable /opt/hermes"
                        " --build-constraint /build-inputs/build-constraints.txt " + only_binary)
    tail = tail.replace("COPY --from=builder /fleet-server /usr/local/bin/fleet-control-server\n", "")
    tail = tail.replace("groupadd -r fleet-control && useradd -r -g fleet-control", "groupadd -g 999 fleet-control && useradd -u 999 -g fleet-control")
    label_start = tail.index("ARG BASE_REVISION\n")
    label_end = tail.index("WORKDIR /app", label_start)
    tail = tail[:label_start] + ("LABEL sdlc.hermes.revision=" + inputs["hermes"] +
                                 " sdlc.fleet.qa.source=" + inputs["fleet"] + "\n") + tail[label_end:]
    tail = tail.replace('CMD ["fleet-control-server"]', 'CMD ["false"]')
    return ("ARG CONTROLLER_IMAGE\nFROM ${CONTROLLER_IMAGE} AS build-inputs\n" + tail).encode()


def build_constraints(inputs):
    return "".join(name + " @ " + inputs["artifacts"][file]["url"] + "#sha256=" +
                   inputs["artifacts"][file]["sha256"] + "\n"
                   for name, file in (("setuptools", "setuptools.whl"), ("wheel", "wheel.whl"))).encode()
