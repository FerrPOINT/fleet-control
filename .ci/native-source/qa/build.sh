#!/usr/bin/env bash
set -euo pipefail
test "$(rustc --version | awk '{print $2}')" = 1.88.0
command -v docker
docker compose version --short
for path in /cargo /target /scratch; do
  test "$(df -PB1 "$path" | awk 'END {print $4}')" -ge "${FLEET_QA_MIN_FREE_BYTES:?sealed capacity floor required}"
  test -z "$(find "$path" -mindepth 1 -print -quit)"
done
mkdir /scratch/tmp /scratch/src
export TMPDIR=/scratch/tmp CARGO_HOME=/cargo CARGO_TARGET_DIR=/target
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
cd /input
sha256sum --check sources.sha256
cp -a /input/. /scratch/src/
cd /scratch/src
sha256sum --check sources.sha256
# Separate QA crate/lock, never mutate the frozen product workspace or its lock.
cargo build --locked --manifest-path live/Cargo.toml --message-format=json > /output/build-artifacts.jsonl
test -x /target/debug/fleet-native-acceptance
cp /target/debug/fleet-native-acceptance /compiled/live
chmod 755 /compiled/live
python3 -B /qa/compile_proof.py --artifacts /output/build-artifacts.jsonl \
  --root /scratch/src --executable /target/debug/fleet-native-acceptance \
  --copied /compiled/live --output /output/compile-proof.json
sha256sum /compiled/live > /output/live-binary.sha256
sha256sum --check sources.sha256
