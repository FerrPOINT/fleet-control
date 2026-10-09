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
  if [[ "$stage" == foundation ]]; then ignored=115; fi
  local exact=()
  if [[ "$stage" == lookup_* || "$stage" == control_api ]]; then exact=(--exact); fi
  test -s "$expected"
  cargo test --locked "$@" -- "${exact[@]}" --test-threads=1 2>&1 | tee "$log"
  while IFS= read -r name; do
    grep -Fx "test $name ... ok" "$log"
  done < "$expected"
  count=$(wc -l < "$expected")
  grep -F "test result: ok. $count passed; 0 failed; $ignored ignored;" "$log"
  passed
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
  FLEET_REAL_AUTH_TEST_DATABASE_URL FLEET_DISPATCH_JOURNAL_MIGRATION_TEST_DATABASE_URL \
  FLEET_RUNTIME_CONTROL_MIGRATION_TEST_DATABASE_URL FLEET_HERMES_TIME_MIGRATION_TEST_DATABASE_URL \
  QA_SOURCE_COMMIT QA_AUTH_SOURCE_COMMIT DATABASE_URL; do
  test -n "${!name}"
done
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
stage=check
cargo check --locked --workspace --all-targets 2>&1 | tee ${QA_OUTPUT}/check.log
passed
stage=clippy
cargo clippy --locked --workspace --all-targets -- -D warnings 2>&1 | tee ${QA_OUTPUT}/clippy.log
passed
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
run_tests hermes_wire -p infra --lib runtime::hermes_wire::tests::
for entry in hermes_journal hermes_acceptance hermes_readback hermes_terminal; do
  case "$entry" in
    hermes_journal) filter=hermes_dispatch_journal:: ;;
    hermes_acceptance) filter=runtime_acceptance:: ;;
    hermes_readback) filter=runtime_acceptance_readback_http:: ;;
    hermes_terminal) filter=runtime_http_ ;;
  esac
  stage=$entry
  cargo test --locked -p infra --test sdlc_foundation "$filter" -- --ignored --test-threads=1 2>&1 | tee "${QA_OUTPUT}/$entry.log"
  while IFS= read -r name; do grep -Fx "test $name ... ok" "${QA_OUTPUT}/$entry.log"; done < "${QA_EXPECTED}/$entry.txt"
  count=$(wc -l < "${QA_EXPECTED}/$entry.txt")
  grep -F "test result: ok. $count passed; 0 failed; 0 ignored;" "${QA_OUTPUT}/$entry.log"
  passed
done
stage=journal_migration
cargo test --locked -p migration --test hermes_dispatch_journal -- --ignored --test-threads=1 2>&1 | tee ${QA_OUTPUT}/journal_migration.log
while IFS= read -r name; do grep -Fx "test $name ... ok" ${QA_OUTPUT}/journal_migration.log; done < ${QA_EXPECTED}/journal_migration.txt
grep -F 'test result: ok. 1 passed; 0 failed; 0 ignored;' ${QA_OUTPUT}/journal_migration.log
passed
run_tests control_api -p api --lib routes::sessions::tests::runtime_controls_require_one_bounded_key_and_derive_the_actor
run_tests lookup_header -p api --lib routes::sessions::tests::runtime_control_lookup_requires_bounded_unambiguous_header_without_echo
run_tests lookup_openapi -p api --lib tests::runtime_control_lookup_openapi_requires_key_and_returns_one_receipt
run_tests lookup_route -p api --lib tests::runtime_control_lookup_literal_route_cannot_be_an_old_uuid_receipt
run_tests control_wire -p infra --lib runtime::run_control::tests::
run_tests recovery_wire -p infra --lib runtime::recovery_wire::tests::
run_tests sse_wire -p infra --lib runtime::sse_wire::tests::
for entry in runtime_controls runtime_terminal runtime_pinned_recovery runtime_unknown_recovery runtime_recovery_races runtime_stream_bounds; do
  case "$entry" in
    runtime_controls) filter=runtime_run_control:: ;;
    runtime_terminal) filter=runtime_terminal:: ;;
    runtime_pinned_recovery) filter=runtime_pinned_recovery:: ;;
    runtime_unknown_recovery) filter=runtime_unknown_recovery:: ;;
    runtime_recovery_races) filter=runtime_recovery_races:: ;;
    runtime_stream_bounds) filter=runtime_stream_bounds:: ;;
  esac
  stage=$entry
  cargo test --locked -p infra --test sdlc_foundation "$filter" -- --ignored --test-threads=1 2>&1 | tee "${QA_OUTPUT}/$entry.log"
  while IFS= read -r name; do grep -Fx "test $name ... ok" "${QA_OUTPUT}/$entry.log"; done < "${QA_EXPECTED}/$entry.txt"
  count=$(wc -l < "${QA_EXPECTED}/$entry.txt")
  grep -F "test result: ok. $count passed; 0 failed; 0 ignored;" "${QA_OUTPUT}/$entry.log"
  passed
done
stage=controls_migration
cargo test --locked -p migration --test runtime_controls -- --ignored --test-threads=1 2>&1 | tee ${QA_OUTPUT}/controls_migration.log
while IFS= read -r name; do grep -Fx "test $name ... ok" ${QA_OUTPUT}/controls_migration.log; done < ${QA_EXPECTED}/controls_migration.txt
grep -F 'test result: ok. 1 passed; 0 failed; 0 ignored;' ${QA_OUTPUT}/controls_migration.log
passed
run_tests approval_snapshot -p infra --lib runtime::approval_snapshot::tests::
run_tests targeted_approval -p infra --lib runtime::targeted_approval::tests::
stage=approval_recovery
cargo test --locked -p infra --test sdlc_foundation runtime_approval_recovery:: -- --ignored --test-threads=1 2>&1 | tee ${QA_OUTPUT}/approval_recovery.log
while IFS= read -r name; do grep -Fx "test $name ... ok" ${QA_OUTPUT}/approval_recovery.log; done < ${QA_EXPECTED}/approval_recovery.txt
grep -F 'test result: ok. 15 passed; 0 failed; 0 ignored;' ${QA_OUTPUT}/approval_recovery.log
passed
stage=approval_compat
cargo test --locked -p infra --test sdlc_foundation targeted_approval_http_requires_human_and_unknown_ack_is_not_repeated -- --exact --test-threads=1 2>&1 | tee ${QA_OUTPUT}/approval_compat.log
while IFS= read -r name; do grep -Fx "test $name ... ok" ${QA_OUTPUT}/approval_compat.log; done < ${QA_EXPECTED}/approval_compat.txt
grep -F 'test result: ok. 1 passed; 0 failed; 0 ignored;' ${QA_OUTPUT}/approval_compat.log
passed
stage=time_migration
cargo test --locked -p migration --test hermes_journal_time_order -- --ignored --test-threads=1 2>&1 | tee ${QA_OUTPUT}/time_migration.log
while IFS= read -r name; do grep -Fx "test $name ... ok" ${QA_OUTPUT}/time_migration.log; done < ${QA_EXPECTED}/time_migration.txt
grep -F 'test result: ok. 1 passed; 0 failed; 0 ignored;' ${QA_OUTPUT}/time_migration.log
passed
stage=migration_smoke
{
  cargo run --locked -p migration -- up
  cargo run --locked -p migration -- status
  cargo run --locked -p migration -- down -n 1
  cargo run --locked -p migration -- up
  cargo run --locked -p migration -- status
  cargo run --locked -p migration -- down -n 15
  cargo run --locked -p migration -- status
  cargo run --locked -p migration -- up
  cargo run --locked -p migration -- status
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
