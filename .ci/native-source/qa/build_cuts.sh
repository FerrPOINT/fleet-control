#!/usr/bin/env bash
set -euo pipefail
# First preserve/qualify the original nine-scenario binary and every original cold guard.
bash /qa/build.sh
export TMPDIR=/scratch/tmp CARGO_HOME=/cargo CARGO_TARGET_DIR=/target
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
cd /scratch/src
sha256sum --check sources.sha256
cargo build --locked --manifest-path cut-live/Cargo.toml --message-format=json > /output/cut-build-artifacts.jsonl
test -x /target/debug/fleet-native-acceptance
cp /target/debug/fleet-native-acceptance /compiled/cut-live
chmod 755 /compiled/cut-live
python3 -B /qa/cut_compile_proof.py --artifacts /output/cut-build-artifacts.jsonl \
  --root /scratch/src --executable /target/debug/fleet-native-acceptance \
  --copied /compiled/cut-live --output /output/cut-compile-proof.json
install -m 755 /input/cut_transport.py /compiled/cut-transport
install -m 644 /input/cut_contract.py /compiled/cut_contract.py
install -m 644 /input/cut_files.py /compiled/cut_files.py
test "$(sha256sum /compiled/cut-transport | awk '{print $1}')" = "$(sha256sum /input/cut_transport.py | awk '{print $1}')"
test "$(sha256sum /compiled/cut_contract.py | awk '{print $1}')" = "$(sha256sum /input/cut_contract.py | awk '{print $1}')"
test "$(sha256sum /compiled/cut_files.py | awk '{print $1}')" = "$(sha256sum /input/cut_files.py | awk '{print $1}')"
sha256sum --check sources.sha256
