"""Original Base169 fake-engine acceptance selectors. No native Docker or setup."""
import argparse
from pathlib import Path
import sys
import unittest
from verify_container_utilities import verify_contract_checkout


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True, type=Path)
    args = parser.parse_args()
    verify_contract_checkout(args.base)
    sys.dont_write_bytecode = True
    sys.path.insert(0, str(args.base.resolve()))
    cases = {
        "test_runtime_preparation.PreparationTests": (
            "test_saved_result_replay_does_not_restart_a_running_or_exited_namespace",
            "test_unknown_without_resource_holds_and_never_issues_second_create",
            "test_changed_payload_and_operation_conflict_without_another_effect",
        ),
        "test_runtime_bootstrap.BootstrapTests": (
            "test_original_terminal_readback_never_restarts_and_changed_start_is_denied",
            "test_lost_start_ack_remains_held_without_adopting_fresh_running_pid",
        ),
        "test_runtime_boundary.BoundaryTests": (
            "test_exact_stop_and_replay_never_kill_twice",
            "test_unknown_effect_holds_and_replay_never_kills_again",
        ),
        "test_runtime_control.MappedLifecycleTests": (
            "test_mapped_prepare_start_observe_stop_replay_preserves_original_identity",
            "test_actual_mapping_drift_denies_all_lifecycle_actions_without_effects",
        ),
    }
    names = ["scripts.tests." + cls + "." + name for cls, tests in cases.items() for name in tests]
    suite = unittest.defaultTestLoader.loadTestsFromNames(names)
    if suite.countTestCases() != 9:
        raise SystemExit("Mandatory activation contract selectors changed")
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() and result.testsRun == 9 and not result.skipped else 1


if __name__ == "__main__":
    sys.exit(main())
