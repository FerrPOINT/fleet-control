#!/usr/bin/env bash
set -euo pipefail
rustc --version | grep '^rustc 1.88.0 '
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
cargo_mode=(--locked)
if [ "${LIVE_PM_ALLOW_REGISTRY:-0}" != 1 ]; then
  cargo_mode+=(--offline)
fi
if [ "${1:-}" = build ]; then
  cd /work/auth-source
  CARGO_TARGET_DIR=/cache/base cargo build "${cargo_mode[@]}" -p auth-server --bin auth-server
  cp /cache/base/debug/auth-server /binaries/auth-server
  cd /work/task-tracker/backend
  CARGO_TARGET_DIR=/cache/tracker cargo build "${cargo_mode[@]}" -p server --bin server
  cp /cache/tracker/debug/server /binaries/tracker-server
  cd /work/fleet-control/backend
  rustfmt --edition 2024 --check infra/tests/pm_credentials_live.rs
  CARGO_TARGET_DIR=/cache/final cargo test "${cargo_mode[@]}" -p infra --test pm_credentials_live --no-run
  CARGO_TARGET_DIR=/cache/final cargo clippy "${cargo_mode[@]}" -p infra --test pm_credentials_live -- -D warnings
else
  cd /work/fleet-control/backend
  CARGO_TARGET_DIR=/cache/final cargo test --locked --offline -p infra --test pm_credentials_live -- --ignored --exact actual_base_child_authorizes_current_tracker_assignment --nocapture --test-threads=1
fi
