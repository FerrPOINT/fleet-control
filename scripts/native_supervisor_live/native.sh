#!/usr/bin/env bash
set -euo pipefail
mkdir -p /tmp/fleet-native-supervisor/parent
test -n "${FLEET_NATIVE_TEST_BINARY:-}"
test -f "$FLEET_NATIVE_TEST_BINARY"
echo "FLEET_TEST_BINARY_SHA256=$(sha256sum "$FLEET_NATIVE_TEST_BINARY" | cut -d ' ' -f 1)"
/opt/hermes/.venv/bin/python /qa/preflight.py
exec "$FLEET_NATIVE_TEST_BINARY" --ignored --test-threads=1 --nocapture
