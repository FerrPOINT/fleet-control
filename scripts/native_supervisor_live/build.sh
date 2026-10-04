#!/usr/bin/env bash
set -euo pipefail
cd /work/fleet-control/backend
cargo fmt --all --check
cargo check --locked --offline --workspace --all-targets
cargo clippy --locked --offline -p infra --test native_supervisor_live -- -D warnings
cargo test --locked --offline -p infra --test native_supervisor_live --no-run --message-format=json
