"""Closed offline receipt validation; exit0 is not execution evidence."""

import json
import math
import re


EXPECTED_COUNTS = {"test_contract.ContractTests": 16, "test_hermes.HermesProbes": 8}
EXPECTED_SELECTORS = tuple(
    "test_contract.ContractTests." + name for name in (
        "test_authority_in_model_payload_and_privileged_tools_denied",
        "test_concurrent_same_call_consumes_one_send_permit",
        "test_conversation_is_not_native_run",
        "test_exact_readback_resolves_without_second_write",
        "test_expired_at_boundary_denied_before_io",
        "test_late_send_result_cannot_overwrite_exact_terminal_readback",
        "test_missing_each_producer_primitive_never_calls_first_model",
        "test_non_receipt_result_is_unknown_not_success",
        "test_old_run_not_rebound_to_new_assignment",
        "test_positive_admission_is_explicitly_synthetic",
        "test_rejected_write_is_not_delivered_or_retried",
        "test_stale_fence_or_any_identity_revision_denied",
        "test_two_runs_same_conversation_do_not_share_binding",
        "test_unknown_blocks_new_call_or_run_not_only_same_key_replay",
        "test_unknown_write_timeout_prevents_redispatch",
        "test_wrong_peer_or_conversation_denied_before_io",
    )
) + tuple(
    "test_hermes.HermesProbes." + name for name in (
        "test_inherited_stale_native_context_needs_fresh_server_binding",
        "test_legacy_env_fallback_is_observable_but_rejected_as_authority",
        "test_native_exception_clears_context_and_next_run_is_not_stale",
        "test_pre_llm_hook_denial_and_failure_are_not_a_model_veto",
        "test_real_native_run_context_reaches_registered_handler_not_task_id",
        "test_real_registration_dispatch_and_disposal",
        "test_registered_tool_without_bound_run_is_held_before_gateway",
        "test_two_concurrent_native_runs_same_conversation_isolated",
    )
)
MANDATORY_IMPORTS = frozenset({
    "hermes_cli/plugins.py", "tools/registry.py", "tools/approval_context.py",
    "gateway/session_context.py", "gateway/platforms/api_server_runs.py",
    "model_tools.py", "agent/turn_context.py", "hermes_cli/plugins_dispatch.py",
    "hermes_cli/middleware.py",
})
REQUIRED_FIELDS = frozenset({
    "tests_run", "failures", "errors", "skips", "selectors", "git_imports",
    "unsealed_imports", "python", "host_dependencies", "test_status",
    "producer_admission", "live_evidence", "expected_counts", "actual_counts", "pins", "git_reader",
})


def load_receipt(path):
    def unique_fields(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate receipt field")
            result[key] = value
        return result

    def invalid_constant(value):
        raise ValueError("non-finite receipt number")

    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique_fields,
                      parse_constant=invalid_constant)


def validate_receipt(value, exit_code, pins):
    def require(condition, reason):
        if not condition:
            raise ValueError("invalid offline receipt: " + reason)

    require(type(exit_code) is int and exit_code == 0, "child did not exit0")
    require(type(value) is dict and REQUIRED_FIELDS == value.keys(), "wrong closed receipt fields")
    require(value["test_status"] == "PASS", "test_status is not PASS")
    require(value["pins"] == pins, "wrong source pins")
    require(value["producer_admission"] == "BLOCKED" and value["live_evidence"] is False,
            "unsafe admission/live claim")
    for field, expected in (("tests_run", 24), ("failures", 0), ("errors", 0), ("skips", 0)):
        require(type(value[field]) is int and value[field] == expected, "wrong " + field)
    for field in ("actual_counts", "expected_counts"):
        counts = value[field]
        require(type(counts) is dict and counts == EXPECTED_COUNTS
                and all(type(n) is int for n in counts.values()), "wrong " + field)
    selectors = value["selectors"]
    require(type(selectors) is list and all(type(s) is str for s in selectors)
            and len(selectors) == 24 and sorted(selectors) == sorted(EXPECTED_SELECTORS),
            "wrong exact selectors")
    require(type(value["unsealed_imports"]) is list and not value["unsealed_imports"],
            "unsealed imports")
    require(type(value["python"]) is str and bool(value["python"]), "missing Python measurement")
    require(type(value["host_dependencies"]) is dict, "missing dependency measurement")
    reader = value["git_reader"]
    require(type(reader) is dict and set(reader) == {"pid", "exit_code", "requests", "read_seconds"},
            "missing owned Git reader closure")
    require(type(reader["pid"]) is int and reader["pid"] > 0
            and type(reader["exit_code"]) is int and reader["exit_code"] == 0
            and type(reader["requests"]) is int and reader["requests"] > 0,
            "owned Git reader not closed successfully")
    require(type(reader["read_seconds"]) in (int, float) and math.isfinite(reader["read_seconds"])
            and reader["read_seconds"] >= 0, "invalid Git read measurement")
    imports = value["git_imports"]
    require(type(imports) is dict and MANDATORY_IMPORTS <= imports.keys(), "missing import provenance")
    for path, entry in imports.items():
        require(type(path) is str and path.endswith(".py") and "\\" not in path
                and not path.startswith("/") and not {".", ".."} & set(path.split("/")),
                "invalid import path")
        require(type(entry) is dict and set(entry) == {"git_blob", "sha256", "bytes"},
                "invalid import entry")
        require(type(entry["git_blob"]) is str and re.fullmatch(r"[0-9a-f]{40}", entry["git_blob"]),
                "invalid Git object")
        require(type(entry["sha256"]) is str and re.fullmatch(r"[0-9a-f]{64}", entry["sha256"]),
                "invalid blob hash")
        require(type(entry["bytes"]) is int and entry["bytes"] > 0, "invalid blob size")
    return value
