#!/usr/bin/env bash
set -euo pipefail
stage=preflight
: > ${QA_OUTPUT}/gates.tsv
trap 'code=$?; if ((code != 0)); then printf "%s\tfailed\n" "$stage" >> ${QA_OUTPUT}/gates.tsv; fi' EXIT
passed() {
  python3 "$QA_HELPER" verify-log "$stage"
  test "$(df -PB1 "$QA_ROOT" | awk 'END {print $4}')" -ge 5368709120
  printf '%s\tpassed\n' "$stage" >> "$QA_OUTPUT/gates.tsv"
}
run_tests() {
  stage=$1
  shift
  local log=${QA_OUTPUT}/$stage.log expected=${QA_EXPECTED}/$stage.txt count ignored=0
  local exact=()
  test -s "$expected"
  cargo test --locked "$@" -- "${exact[@]}" --test-threads=1 2>&1 | tee "$log"
  while IFS= read -r name; do
    grep -Fx "test $name ... ok" "$log"
  done < "$expected"
  count=$(wc -l < "$expected")
  grep -F "test result: ok. $count passed; 0 failed; $ignored ignored;" "$log"
  passed
}
run_compiler() {
  stage=$1
  shift
  local code=0
  "$@" > "$QA_OUTPUT/$stage.jsonl" 2> "$QA_OUTPUT/$stage.stderr" || code=$?
  printf '%s\n' "$code" > "$QA_OUTPUT/$stage.exit"
  if ((code != 0)); then exit "$code"; fi
  passed
}
migration_snapshot() {
  psql -X -w "$DATABASE_URL" -v ON_ERROR_STOP=1 -A -t \
    -c 'SELECT version FROM seaql_migrations ORDER BY version' > "$QA_OUTPUT/migration-$1.txt"
  psql -X -w "$DATABASE_URL" -v ON_ERROR_STOP=1 -A -t -F $'\t' \
    -c 'SELECT version,applied_at FROM seaql_migrations ORDER BY version' > "$QA_OUTPUT/migration-$1-ledger.tsv"
}
test "$(rustc --version | awk '{print $2}')" = 1.88.0
command -v git
command -v curl
{ grep -E '^(MemAvailable|MemFree|MemTotal|CommitLimit|Committed_AS):' /proc/meminfo; df -PB1 "$QA_ROOT" "$CARGO_TARGET_DIR"; } > "$QA_OUTPUT/resource-audit.txt"
test "$(df -PB1 "$QA_ROOT" | awk 'END {print $4}')" -ge 5368709120
for empty_volume in "$CARGO_HOME" "$CARGO_TARGET_DIR" "$TMPDIR"; do
  test -z "$(find "$empty_volume" -mindepth 1 -print -quit)"
done
test "$(tr -d '\r\n' < ${QA_ROOT}/src/fleet-control/.base-revision)" = 19a7a381ae6dbea61a643bb96189e483fa64df5c
for name in FLEET_TEST_DATABASE_URL FLEET_MIGRATION_TEST_DATABASE_URL \
  FLEET_MESSAGE_ORDER_TEST_DATABASE_URL FLEET_CHATS_DIRECTORY_TEST_DATABASE_URL \
  FLEET_RUNTIME_APPROVAL_EVENTS_TEST_DATABASE_URL FLEET_CREDENTIAL_MIGRATION_TEST_DATABASE_URL \
  FLEET_REAL_AUTH_TEST_DATABASE_URL FLEET_CONFIGURATION_TEST_DATABASE_URL FLEET_TEST_BASE_PACKAGE_CHECKOUT \
  QA_SOURCE_COMMIT QA_AUTH_SOURCE_COMMIT DATABASE_URL; do
  test -n "${!name}"
done
test "$(git -C "$FLEET_TEST_BASE_PACKAGE_CHECKOUT" rev-parse HEAD)" = 4b9b4c9297a13fb28a6ba2039af2f7cb719f2f58
test "$(git -C "$FLEET_TEST_BASE_PACKAGE_CHECKOUT" config --get remote.origin.url)" = https://github.com/FerrPOINT/services-base.git
git -C "$FLEET_TEST_BASE_PACKAGE_CHECKOUT" cat-file -e 4b9b4c9297a13fb28a6ba2039af2f7cb719f2f58:agent-skills/manifest.json
cd "$QA_ROOT/src"
sha256sum --check "$QA_SOURCES"
printf '#!/bin/sh\nexit 0\n' > "$TMPDIR/qa-exec-probe"
chmod 700 "$TMPDIR/qa-exec-probe"
"$TMPDIR/qa-exec-probe"
rm -- "$TMPDIR/qa-exec-probe"
printf '%s  %s\n' 481244d0812097b11fbaeef79f71d942b171617f9c9f9514e63acbe13e71ccdc \
  ${QA_ROOT}/swagger.zip | sha256sum --check
export SWAGGER_UI_DOWNLOAD_URL=file://${QA_ROOT}/swagger.zip
export CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
cd fleet-control/backend
passed
stage=fmt
cargo fmt --all -- --check 2>&1 | tee ${QA_OUTPUT}/fmt.log
passed
run_compiler check cargo check --locked --workspace --all-targets --message-format=json
run_compiler clippy cargo clippy --locked --workspace --all-targets --message-format=json -- -D warnings
stage=auth_binary
(
  cd ${QA_ROOT}/src/base-auth-source
  CARGO_TARGET_DIR=${CARGO_TARGET_DIR}/base-auth cargo build --locked -p auth-server --bin auth-server
) 2>&1 | tee ${QA_OUTPUT}/auth_binary.log
export FLEET_REAL_AUTH_TEST_OWNED=source-qualified-disposable
export FLEET_REAL_AUTH_TEST_SOURCE_SHA="$QA_AUTH_SOURCE_COMMIT"
export FLEET_REAL_AUTH_TEST_BINARY=${CARGO_TARGET_DIR}/base-auth/debug/auth-server
export FLEET_REAL_AUTH_TEST_BINARY_SHA256
FLEET_REAL_AUTH_TEST_BINARY_SHA256=$(sha256sum "$FLEET_REAL_AUTH_TEST_BINARY" | awk '{print $1}')
printf '%s  %s\n' "$FLEET_REAL_AUTH_TEST_BINARY_SHA256" "$FLEET_REAL_AUTH_TEST_BINARY" > ${QA_OUTPUT}/auth-binary.sha256
printf '%s\n' "$QA_AUTH_SOURCE_COMMIT" > ${QA_OUTPUT}/auth-source.txt
passed
stage=runtime_inventory
cargo test --locked --workspace -- --list > "$QA_OUTPUT/workspace-list.log" 2>&1
cargo test --locked --workspace -- --ignored --list > "$QA_OUTPUT/ignored-list.log" 2>&1
passed
stage=real_auth
cargo test --locked -p infra --test pm_credentials_real_auth -- --ignored --test-threads=1 2>&1 | tee ${QA_OUTPUT}/real_auth.log
while IFS= read -r name; do grep -Fx "test $name ... ok" ${QA_OUTPUT}/real_auth.log; done < ${QA_EXPECTED}/real_auth.txt
grep -F 'test result: ok. 1 passed; 0 failed; 0 ignored;' ${QA_OUTPUT}/real_auth.log
passed
run_tests api2 -p api --lib routes::pm_runtime::tests::
run_tests credentials_unit -p infra --lib pm_credentials::
run_tests credentials_pg -p infra --test sdlc_foundation pm_credential_creation::
run_tests foundation -p infra --test sdlc_foundation
run_tests config_api -p api --lib routes::sdlc_configuration::tests::
run_tests base_package_unit -p infra --lib base_package::tests::
run_tests config_files_unit -p infra --lib effective_configuration::tests::
run_tests package_effective_unit -p infra --lib tests::base_package_effective_
run_tests config_shared_unit -p shared --lib config::tests::
FLEET_TEST_DATABASE_URL="$FLEET_CONFIGURATION_TEST_DATABASE_URL" \
  run_tests base_package_pg -p infra --test sdlc_foundation base_package::
FLEET_TEST_DATABASE_URL="$FLEET_CONFIGURATION_TEST_DATABASE_URL" \
  run_tests config_revision_pg -p infra --test sdlc_foundation config_revision_
stage=workspace
cargo test --locked --workspace -- --test-threads=1 2>&1 | tee ${QA_OUTPUT}/workspace.log
passed
stage=lineage10
cargo test --locked -p migration --lib lineage_tests -- --include-ignored --test-threads=1 2>&1 | tee ${QA_OUTPUT}/lineage10.log
while IFS= read -r name; do grep -Fx "test $name ... ok" ${QA_OUTPUT}/lineage10.log; done < ${QA_EXPECTED}/lineage10.txt
grep -F 'test result: ok. 10 passed; 0 failed; 0 ignored;' ${QA_OUTPUT}/lineage10.log
passed
for entry in central_profile message_order chats_directory runtime_approval_events credentials_migration; do
  stage=$entry
  package=infra
  test_target=$entry
  if [[ "$entry" == message_order ]]; then package=migration; fi
  if [[ "$entry" == credentials_migration ]]; then package=migration; test_target=pm_credentials; fi
  cargo test --locked -p "$package" --test "$test_target" -- --ignored --test-threads=1 2>&1 | tee "${QA_OUTPUT}/$entry.log"
  while IFS= read -r name; do grep -Fx "test $name ... ok" "${QA_OUTPUT}/$entry.log"; done < "${QA_EXPECTED}/$entry.txt"
  count=$(wc -l < "${QA_EXPECTED}/$entry.txt")
  grep -F "test result: ok. $count passed; 0 failed; 0 ignored;" "${QA_OUTPUT}/$entry.log"
  passed
done
stage=migration_smoke
{
  cargo run --locked -p migration -- up
  cargo run --locked -p migration -- status
  migration_snapshot up
  cargo run --locked -p migration -- down -n 1
  migration_snapshot down_one
  cargo run --locked -p migration -- up
  cargo run --locked -p migration -- status
  migration_snapshot reapply
  cargo run --locked -p migration -- down -n 12
  cargo run --locked -p migration -- status
  migration_snapshot down_all
  cargo run --locked -p migration -- up
  cargo run --locked -p migration -- status
  migration_snapshot final_up
} 2>&1 | tee ${QA_OUTPUT}/migration_smoke.log
passed
stage=openapi
cargo run --locked -p api --bin gen-openapi > ${QA_OUTPUT}/openapi.json
cmp ../openapi/openapi.json ${QA_OUTPUT}/openapi.json
passed
stage=compiled_source_parity
cd ${QA_ROOT}/src
sha256sum --check "$QA_SOURCES"
passed
echo HOSTED_EXACT_SOURCE_BACKEND_GATE=PASS
