"""Run selected original Base fake-engine contracts, never Docker or native setup."""
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
    prefix = "scripts.tests.test_runtime_control.MappedLifecycleTests."
    names = [prefix + name for name in (
        "test_mapped_preparation_readback_requires_original_mapping_and_controller",
        "test_mapped_preparation_readback_never_creates_a_missing_mapping_file",
        "test_mapped_missing_preparation_ack_completes_registration_without_lifecycle",
    )]
    names += ["scripts.tests.test_runtime_preparation.PreparationTests." + name for name in (
        "test_unknown_without_resource_holds_and_never_issues_second_create",
        "test_concurrent_replays_have_one_creation",
    )]
    suite = unittest.defaultTestLoader.loadTestsFromNames(names)
    if suite.countTestCases() != 5:
        raise SystemExit("Original preparation selectors changed")
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() and result.testsRun == 5 and not result.skipped else 1


if __name__ == "__main__":
    sys.exit(main())
