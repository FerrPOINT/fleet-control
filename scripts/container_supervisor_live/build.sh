#!/usr/bin/env bash
set -euo pipefail
mkdir /tmp/src /tmp/container-target
cp -a /input/. /tmp/src/
sha256sum --check /qa/manifest.sha256
shopt -s nullglob
swagger_found=false
for archive in /qa-target/*/build/utoipa-swagger-ui-*/out/v5.17.14.zip; do
    if [[ "$(sha256sum "$archive" | cut -d' ' -f1)" == 481244d0812097b11fbaeef79f71d942b171617f9c9f9514e63acbe13e71ccdc ]]; then
        cp "$archive" /tmp/swagger.zip
        swagger_found=true
        break
    fi
done
test "$swagger_found" == true
export SWAGGER_UI_DOWNLOAD_URL=file:///tmp/swagger.zip
export CARGO_TARGET_DIR=/tmp/container-target CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
cd /tmp/src/fleet-control/backend
rustc --version
cargo fmt --all --check
cargo clippy --locked --offline --workspace --all-targets -- -D warnings
cargo test --locked --offline -p infra --lib runtime::controller_recovery_wire::tests:: -- --nocapture --test-threads=1
cargo test --locked --offline -p infra --lib --no-run --message-format=json > /tmp/library-artifacts.jsonl
python3 /qa/select_artifact.py /tmp/library-artifacts.jsonl /out/fleet-runtime-tests
cargo test --locked --offline -p infra --test container_supervisor_live --no-run --message-format=json > /tmp/artifacts.jsonl
python3 /qa/select_artifact.py /tmp/artifacts.jsonl /out/fleet-container-live
ldd /out/fleet-container-live
