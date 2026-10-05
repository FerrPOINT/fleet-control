"""Check every tracked source byte before the managed gateway/model test begins."""
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import os
from concurrent.futures import ThreadPoolExecutor


def control_plugin_required(test_name):
    return test_name in {
        'managed_native_original_control_outcomes_recover_lost_http_ack',
        'native_control_restart::managed_native_control_outcomes_survive_fleet_process_death',
        'native_approvals::managed_native_original_approval_outcomes_recover_lost_http_ack',
        'native_approvals::native_approval_restart::managed_native_approval_outcomes_survive_fleet_process_death',
        'native_approvals::native_approval_restart::managed_native_combined_run_and_approval_outcomes_survive_fleet_process_death',
        'native_control_restart::managed_native_combined_run_and_control_outcomes_survive_fleet_process_death',
    }


def recovery_plugin_required(test_name):
    return test_name in {
        'managed_native_lost_ack_recovers_original_run_across_fleet_processes',
        'native_approvals::native_approval_restart::managed_native_combined_run_and_approval_outcomes_survive_fleet_process_death',
        'native_control_restart::managed_native_combined_run_and_control_outcomes_survive_fleet_process_death',
    }


def verify(root, hashes):
    if not isinstance(hashes, dict) or not hashes:
        raise RuntimeError('Native source inventory is missing')
    canonical = root.resolve()
    def entry(item):
        name, digest = item
        path = PurePosixPath(name)
        if path.is_absolute() or '..' in path.parts or '\\' in name or not name or ':' in name:
            raise RuntimeError('Unsafe native source path')
        if not isinstance(digest, str) or not re.fullmatch('[a-f0-9]{64}', digest):
            raise RuntimeError('Invalid source digest')
        file = root / path
        if (not file.resolve().is_relative_to(canonical) or not file.is_file()
                or file.is_symlink() or hashlib.sha256(file.read_bytes()).hexdigest() != digest):
            raise RuntimeError('Native source differs from immutable archive: ' + name)
    with ThreadPoolExecutor(max_workers=8) as pool:
        for index, _ in enumerate(pool.map(entry, hashes.items()), 1):
            if index % 2000 == 0:
                print('Verified native source files: ' + str(index), flush=True)


if __name__ == '__main__':
    hashes = json.loads(Path('/qa/source-hashes.json').read_text())
    verify(Path('/opt/hermes'), hashes)
    print('Exact pinned native tracked source verified: ' + str(len(hashes)) + ' files', flush=True)
    if recovery_plugin_required(os.environ.get('FLEET_NATIVE_TEST_NAME')):
        plugin = json.loads(Path('/qa/recovery-hashes.json').read_text())
        if set(plugin) != {'__init__.py','plugin.py','store.py','plugin.yaml'}:
            raise RuntimeError('Recovery plugin inventory differs from committed files')
        verify(Path('/qa/recovery-plugin'), plugin)
        print('Exact committed recovery plugin verified: 4 files', flush=True)
    if control_plugin_required(os.environ.get('FLEET_NATIVE_TEST_NAME')):
        plugin = json.loads(Path('/qa/control-hashes.json').read_text())
        if set(plugin) != {'__init__.py','plugin.py','store.py','plugin.yaml'}:
            raise RuntimeError('Control plugin inventory differs from committed files')
        verify(Path('/qa/control-plugin'), plugin)
        print('Exact committed control plugin verified: 4 files', flush=True)
