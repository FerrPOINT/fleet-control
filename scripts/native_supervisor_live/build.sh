#!/usr/bin/env bash
set -euo pipefail
if [[ -e "$CARGO_TARGET_DIR" ]]; then
    echo 'Native target directory already exists; cached compilation is not evidence' >&2
    exit 1
fi
shopt -s nullglob
swagger_found=false
for archive in /cache/*/debug/build/utoipa-swagger-ui-*/out/v5.17.14.zip; do
    if [[ "$(sha256sum "$archive" | cut -d' ' -f1)" == "$FLEET_NATIVE_SWAGGER_SHA256" ]]; then
        cp "$archive" /tmp/fleet-native-swagger-v5.17.14.zip
        swagger_found=true
        break
    fi
done
if [[ "$swagger_found" != true ]]; then
    echo 'Pinned local Swagger UI archive is unavailable; offline build remains blocked' >&2
    exit 1
fi
printf '%s  %s\n' "$FLEET_NATIVE_SWAGGER_SHA256" /tmp/fleet-native-swagger-v5.17.14.zip | sha256sum --check
export SWAGGER_UI_DOWNLOAD_URL=file:///tmp/fleet-native-swagger-v5.17.14.zip
cd /work/fleet-control/backend
cargo fmt --all --check
cargo check --locked --offline --workspace --all-targets
cargo clippy --locked --offline -p infra --test native_supervisor_live -- -D warnings
cargo test --locked --offline -p infra --test native_supervisor_live --no-run --message-format=json
