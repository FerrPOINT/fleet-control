#!/usr/bin/env bash
set -euo pipefail
build_step=toolchain
trap 'code=$?; trap - ERR; python3 -B /qa/compile_proof.py --failure-step "$build_step" --failure-exit "$code" --artifacts /output/build-artifacts.jsonl --root /scratch/src --executable /target/debug/fleet-native-acceptance --copied /compiled/live --output /output/build-failure.json >/dev/null 2>&1 || :; exit "$code"' ERR
test "$(rustc --version | awk '{print $2}')" = 1.88.0
build_step=docker_cli
command -v docker
build_step=compose_cli
docker compose version --short
for path in /cargo /target /scratch; do
  build_step=capacity
  test "$(df -PB1 "$path" | awk 'END {print $4}')" -ge "${FLEET_QA_MIN_FREE_BYTES:?sealed capacity floor required}"
  build_step=empty_volumes
  test -z "$(find "$path" -mindepth 1 -print -quit)"
done
build_step=scratch
mkdir /scratch/tmp /scratch/src
export TMPDIR=/scratch/tmp CARGO_HOME=/cargo CARGO_TARGET_DIR=/target
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
cd /input
build_step=source_hash
sha256sum --check sources.sha256
build_step=copy_source
cp -a /input/. /scratch/src/
cd /scratch/src
build_step=source_hash
sha256sum --check sources.sha256
# Separate QA crate/lock, never mutate the frozen product workspace or its lock.
build_step=cargo
cargo build --locked --manifest-path live/Cargo.toml --message-format=json > /output/build-artifacts.jsonl
build_step=executable
test -x /target/debug/fleet-native-acceptance
build_step=copy_binary
cp /target/debug/fleet-native-acceptance /compiled/live
chmod 755 /compiled/live
build_step=compile_proof
python3 -B /qa/compile_proof.py --artifacts /output/build-artifacts.jsonl \
  --root /scratch/src --executable /target/debug/fleet-native-acceptance \
  --copied /compiled/live --output /output/compile-proof.json
build_step=final_hash
sha256sum /compiled/live > /output/live-binary.sha256
sha256sum --check sources.sha256
