"""Real scenario/mock-clock and actual Linux file/proc regressions; no native engine."""
import copy
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest.mock import Mock, patch

import cut_contract as c
import cut_files as f
import cut_run as cut
from test_cuts import fixture, identity


def populate(root):
    report, armed, events = copy.deepcopy(fixture())
    peer = dict(launch=dict(prepared=dict(container=dict(registration=dict(generation=identity(90))))), pid=99)
    report["peer_before"] = copy.deepcopy(peer)
    report["peer_after"] = copy.deepcopy(peer)
    gate = dict(event=events[0], stdout_forwarded=False,
                journal=dict(sha256="1" * 64, device=1, inode=2, size=32768, mode=0o600, uid=999))
    ready = dict(cut=report["cut"], arm=armed, peer=peer)
    evidence = root / "evidence"
    evidence.mkdir()
    for name, value in (("cut-live-report.json", report), ("cut-stop-ack.json", gate), ("cut-ready.json", ready),
                        ("cut-stop-report.json", dict(state="original_namespaces_exited", agents=2, runtime_ready=False))):
        (evidence / name).write_text(json.dumps(value), encoding="ascii")
    for event in events:
        (evidence / ("ack-" + event["call_id"] + ".json")).write_text(json.dumps(event))
        (evidence / ("dispatch-" + event["call_id"] + ".json")).write_text(json.dumps(dict(call_id=event["call_id"], request=event["request"])))
    final = {str(i): dict(generation=g) for i, g in enumerate(
        [armed["generation"], identity(90), identity(10), identity(20), identity(30), identity(33)])}
    return gate, ready, final


def sample(event, now=0):
    return dict(witness=dict(call_id=event["call_id"], ack_sha256=event["ack_sha256"],
        pid=10, start_ticks=20, boot_id=identity(300), began=0, deadline=55), observed=now, remaining=55-now)


class DeadlineTests(unittest.TestCase):
    def exercise(self, kind=None):
        clock, stages, timeouts = [0.0], [], []
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            gate, _, final = populate(root)
            operation = Mock(command=["NEVER_EXECUTED"], project="owned")
            controller = dict(Id="owned-controller", Image="owned-image", State=dict(StartedAt="old", Running=True))
            probes, inventory = [0], [0]

            def checked(argv, **kwargs):
                timeouts.append(kwargs["timeout"])
                if "probe" in argv:
                    probes[0] += 1
                    value = sample(gate["event"], clock[0])
                    if kind == "expired": value = sample(gate["event"], 55)
                    if kind == "dead": raise RuntimeError("actual probe held")
                    if kind == "epoch" and probes[0] == 2: value["witness"]["start_ticks"] += 1
                    if kind == "roundtrip" and probes[0] == 1: clock[0] += 120
                    return json.dumps(value).encode()
                if "ls" in argv:
                    inventory[0] += 1
                    if kind == "inventory" and inventory[0] == 1: clock[0] += 120
                    return b"a b"
                if argv[-1] == "owned-controller":
                    item = copy.deepcopy(controller)
                    if "cut_physical_controller_restart" in stages:
                        item["State"]["StartedAt"] = "new"
                        if kind == "inspect": clock[0] += 120
                    if kind == "controller_epoch": item["State"]["StartedAt"] = "foreign"
                    return json.dumps([item]).encode()
                labels = {"com.docker.compose.project": "owned", "sdlc.task": cut.driver.TASK,
                    "sdlc.purpose": cut.driver.PURPOSE, "com.docker.compose.service": "agent",
                    "sdlc.boundary.resource": identity(500)}
                items = []
                for i, g in enumerate((identity(1), identity(90))):
                    label = dict(labels, **{"sdlc.boundary.generation": g})
                    if kind == "foreign_inventory": label["sdlc.task"] = "foreign"
                    items.append(dict(Id=str(i), Image=cut.driver.HERMES_IMAGE, Config=dict(Labels=label), State=dict(StartedAt="agent")))
                return json.dumps(items).encode()

            def logged(argv, name, timeout=None):
                stages.append(name)
                if "restart" in argv:
                    timeouts.append(timeout)
                    self.assertLess(clock[0], 53)
                    if kind == "restart": clock[0] += 120
                    else: clock[0] += 1

            outputs = {}
            with patch.object(cut.time, "monotonic", side_effect=lambda:clock[0]), \
                    patch.object(cut, "qualification", return_value={}), \
                    patch.object(cut.driver, "write_json"), \
                    patch.object(cut.driver, "native_inventory", return_value=final), \
                    patch.object(cut.driver, "checked", side_effect=checked):
                if kind is None:
                    cut.scenario(operation, root, ["NEVER_EXECUTED"], "owned-controller", controller, {}, logged, outputs)
                    self.assertIn("scenario", outputs)
                else:
                    with self.assertRaises((RuntimeError, ValueError)):
                        cut.scenario(operation, root, ["NEVER_EXECUTED"], "owned-controller", controller, {}, logged, outputs)
                    self.assertNotIn("scenario", outputs)
                    self.assertNotIn("cut_recover_and_f6", stages)
                    self.assertIn("failure_phase", outputs)
            self.assertTrue(all(0 < n <= 53 for n in timeouts))
            return stages

    def test_real_scenario_mock_clock_known_cut_within_budget(self):
        self.assertIn("cut_recover_and_f6", self.exercise())

    def test_original_120s_inventory_counterexample_no_longer_restarts(self):
        self.assertNotIn("cut_physical_controller_restart", self.exercise("inventory"))

    def test_120s_restart_return_is_not_host_success(self):
        self.assertIn("cut_physical_controller_restart", self.exercise("restart"))

    def test_slow_inspect_after_restart_never_accepts_cut(self):
        self.exercise("inspect")

    def test_probe_round_trip_cannot_extend_budget(self):
        self.assertNotIn("cut_physical_controller_restart", self.exercise("roundtrip"))

    def test_expired_dead_or_changed_wrapper_epoch_no_restart(self):
        for kind in ("expired", "dead", "epoch", "controller_epoch"):
            with self.subTest(kind=kind):
                self.assertNotIn("cut_physical_controller_restart", self.exercise(kind))

    def test_original_foreign_inventory_guards_preserved(self):
        self.assertNotIn("cut_physical_controller_restart", self.exercise("foreign_inventory"))

    def test_malformed_completed_ready_remains_strict_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            populate(root)
            (root / "evidence/cut-ready.json").write_bytes(b'{')
            with patch.object(cut, "qualification", return_value={}), patch.object(cut.driver, "write_json"), \
                    patch.object(cut.driver, "checked") as checked, self.assertRaises(ValueError):
                cut.scenario(Mock(command=[]), root, [], "owned", {}, {}, Mock(), {})
            checked.assert_not_called()

    def test_closed_probe_rejects_forged_budget_and_identity(self):
        event = fixture()[2][0]
        for kind in ("extra", "nan", "bool", "window", "remaining", "call", "ack", "boot", "pid"):
            value = sample(event)
            if kind == "extra": value["extra"] = 1
            if kind == "nan": value["observed"] = float("nan")
            if kind == "bool": value["remaining"] = True
            if kind == "window": value["witness"]["deadline"] = 60
            if kind == "remaining": value["remaining"] = 120
            if kind == "call": value["witness"]["call_id"] = identity(600)
            if kind == "ack": value["witness"]["ack_sha256"] = "f" * 64
            if kind == "boot": value["witness"]["boot_id"] = "invalid"
            if kind == "pid": value["witness"]["pid"] = True
            with self.subTest(kind=kind), self.assertRaises(ValueError):
                f.validate_probe(value, event["call_id"], event["ack_sha256"])

    def test_later_probe_never_extends_first_conservative_deadline(self):
        event, clock = fixture()[2][0], [0.0]
        with patch.object(cut.time, "monotonic", side_effect=lambda:clock[0]), \
                patch.object(cut.driver, "checked", return_value=json.dumps(sample(event)).encode()):
            budget = cut.CutBudget(["NEVER_EXECUTED"], event)
            self.assertEqual(budget.deadline, 53)
            clock[0] = 10
            budget.refresh()
            self.assertEqual(budget.deadline, 53)
            self.assertEqual(budget.left(), 43)

    def test_exact_absolute_deadline_boundary_refused(self):
        event, clock = fixture()[2][0], [0.0]
        with patch.object(cut.time, "monotonic", side_effect=lambda:clock[0]), \
                patch.object(cut.driver, "checked", return_value=json.dumps(sample(event)).encode()):
            budget = cut.CutBudget(["NEVER_EXECUTED"], event)
            clock[0] = 53
            with self.assertRaises(RuntimeError): budget.left()


@unittest.skipUnless(sys.platform == "linux", "Linux O_NOFOLLOW/dir-fsync/proc: not a Windows PASS")
class FileTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="nativecutsbc0-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def test_actual_atomic_publication_bytes_mode_fsync_and_no_overwrite(self):
        target = self.root / "ready.json"
        value = dict(ready="complete", epoch=1)
        original_fsync = os.fsync
        calls = []
        def sync(fd):
            calls.append((stat.S_ISDIR(os.fstat(fd).st_mode), os.fstat(fd).st_size))
            return original_fsync(fd)
        with patch.object(f.os, "fsync", side_effect=sync): f.atomic_json(target, value)
        self.assertEqual(c.decode(target.read_bytes()), value)
        self.assertEqual(stat.S_IMODE(target.stat().st_mode), 0o600)
        self.assertEqual(target.stat().st_uid, os.geteuid())
        self.assertEqual(target.stat().st_nlink, 1)
        self.assertTrue(calls[-1][0])
        self.assertTrue(any(not directory and size > 0 for directory, size in calls[:-1]))
        with self.assertRaises(FileExistsError): f.atomic_json(target, dict(replaced=True))
        self.assertEqual(c.decode(target.read_bytes()), value)

    def test_existing_final_without_latch_never_overwritten(self):
        target = self.root / "ready.json"
        target.write_bytes(b'old-completed')
        with self.assertRaises(FileExistsError): f.atomic_json(target, dict(new=True))
        self.assertEqual(target.read_bytes(), b'old-completed')

    def test_two_actual_writers_one_publication_and_no_retry_after_delete(self):
        target = self.root / "ready.json"
        start, errors, winners = threading.Barrier(2), [], []
        def publish(number):
            start.wait(timeout=5)
            try:
                f.atomic_json(target, dict(writer=number))
                winners.append(number)
            except BaseException as error: errors.append(error)
        threads = [threading.Thread(target=publish, args=(n,)) for n in range(2)]
        for thread in threads: thread.start()
        for thread in threads: thread.join(5)
        self.assertTrue(all(not thread.is_alive() for thread in threads))
        self.assertEqual(len(winners), 1)
        self.assertEqual(len(errors), 1)
        self.assertIsInstance(errors[0], FileExistsError)
        self.assertEqual(c.decode(target.read_bytes()), dict(writer=winners[0]))
        target.unlink()
        with self.assertRaises(FileExistsError): f.atomic_json(target, dict(writer=3))

    def test_link_publication_follows_file_fsync_then_directory_fsync(self):
        target = self.root / "ready.json"
        sync, link, calls = os.fsync, os.link, []
        def synced(fd):
            info = os.fstat(fd)
            calls.append("dir" if stat.S_ISDIR(info.st_mode) else ("data" if info.st_size else "claim"))
            return sync(fd)
        def linked(*args, **kwargs):
            self.assertEqual(calls[-1], "data")
            self.assertFalse(target.exists())
            result = link(*args, **kwargs)
            self.assertEqual(c.decode(target.read_bytes()), dict(complete=True))
            calls.append("publish")
            return result
        with patch.object(f.os, "fsync", side_effect=synced), patch.object(f.os, "link", side_effect=linked):
            f.atomic_json(target, dict(complete=True))
        self.assertEqual(calls, ["claim", "dir", "data", "publish", "dir"])

    def test_real_partial_temp_never_visible_at_final_name(self):
        target = self.root / "ready.json"
        entered, release = threading.Event(), threading.Event()
        raw = (json.dumps(dict(payload="x" * 10000), sort_keys=True, separators=(',', ':')) + '\n').encode()
        errors = []
        original_fdopen = os.fdopen
        class SplitWriter:
            def __init__(self, *args, **kwargs): self.stream = original_fdopen(*args, **kwargs)
            def __enter__(self): return self
            def __exit__(self, *args): return self.stream.__exit__(*args)
            def write(self, data):
                self.stream.write(data[:5]); self.stream.flush(); entered.set()
                if not release.wait(5): raise RuntimeError("test reader did not release")
                self.stream.write(data[5:])
            def flush(self): self.stream.flush()
        def publish():
            try: f.atomic_json(target, dict(payload="x" * 10000))
            except BaseException as error: errors.append(error)
        with patch.object(f.os, "fdopen", SplitWriter):
            thread = threading.Thread(target=publish)
            thread.start()
            try:
                self.assertTrue(entered.wait(5))
                self.assertFalse(target.exists())
                self.assertEqual(len(list(self.root.glob("*.tmp"))), 1)
                self.assertEqual(next(self.root.glob("*.tmp")).read_bytes(), raw[:5])
            finally:
                release.set(); thread.join(5)
        self.assertFalse(thread.is_alive())
        self.assertEqual(errors, [])
        self.assertEqual(target.read_bytes(), raw)
        self.assertEqual(c.decode(target.read_bytes()), dict(payload="x" * 10000))

    def test_failed_fsync_keeps_single_attempt_and_no_completed_name(self):
        target = self.root / "ready.json"
        with patch.object(f.os, "fsync", side_effect=OSError("disk error")), self.assertRaises(OSError):
            f.atomic_json(target, dict(ready=True))
        self.assertFalse(target.exists())
        with self.assertRaises(FileExistsError): f.atomic_json(target, dict(ready=True))

    def test_final_and_parent_symlink_and_foreign_owner_refused(self):
        other = self.root / "other"
        other.mkdir()
        target = self.root / "ready.json"
        target.symlink_to(other / "absent")
        with self.assertRaises(FileExistsError): f.atomic_json(target, {})
        self.assertFalse((other / "absent").exists())
        link = self.root / "linked"
        link.symlink_to(other, target_is_directory=True)
        with self.assertRaises(OSError): f.atomic_json(link / "ready.json", {})
        with patch.object(f.os, "geteuid", return_value=os.geteuid()+1), self.assertRaises(ValueError):
            f.atomic_json(self.root / "foreign.json", {})

    def prepared(self):
        evidence = self.root / "evidence"
        evidence.mkdir()
        root = self.root / "controller"
        root.mkdir(mode=0o700)
        fixture_root = self.root / "fixture"
        fixture_root.mkdir()
        gate, ready, _ = populate(fixture_root)
        for name, value in (("qa-cut-once.json", dict(call_id=gate["event"]["call_id"], request=gate["event"]["request"])),
                            ("qa-cut-arm.json", ready["arm"]), ("qa-cut-blocked.json", f.blocking(gate["event"]))):
            f.atomic_json(root / name, value)
        f.atomic_json(evidence / "cut-stop-ack.json", gate)
        return root, evidence, gate, ready

    def test_publish_ready_real_files_original_arm_and_live_proc(self):
        root, evidence, _, ready = self.prepared()
        with patch.object(f, "ROOT", root), patch.object(f, "EVIDENCE", evidence):
            f.publish_ready(json.dumps(ready).encode())
            self.assertEqual(c.decode((evidence / "cut-ready.json").read_bytes()), ready)
            with self.assertRaises(FileExistsError): f.publish_ready(json.dumps(ready).encode())

    def test_original_pid_start_boot_deadline_or_arm_drift_no_ready(self):
        root, evidence, gate, ready = self.prepared()
        blocked = root / "qa-cut-blocked.json"
        original = blocked.read_bytes()
        with patch.object(f, "ROOT", root), patch.object(f, "EVIDENCE", evidence):
            for kind in ("start", "boot", "deadline", "arm", "owner", "once"):
                value = c.decode(original)
                changed = copy.deepcopy(ready)
                if kind == "start": value["start_ticks"] += 1
                if kind == "boot": value["boot_id"] = identity(900)
                if kind == "deadline": value["deadline"] = value["began"] - 1
                if kind == "arm": changed["arm"]["stop_id"] = identity(900)
                if kind == "once": value["call_id"] = identity(900)
                blocked.write_text(json.dumps(value))
                if kind == "owner": blocked.chmod(0o644)
                with self.subTest(kind=kind), self.assertRaises(ValueError):
                    f.publish_ready(json.dumps(changed).encode())
                self.assertFalse((evidence / "cut-ready.json").exists())
                blocked.chmod(0o600)

    def test_exited_actual_process_does_not_prove_blocked_wrapper(self):
        child = subprocess.Popen([sys.executable, "-B", "-c", "import time; time.sleep(30)"])
        try: self.assertGreater(f.process(child.pid), 0)
        finally: child.terminate(); child.wait(timeout=5)
        with self.assertRaises((OSError, ValueError)): f.process(child.pid)

    def test_private_symlink_hardlink_permissions_and_parent_mode_refused(self):
        root = self.root / "controller"
        root.mkdir(mode=0o700)
        path = root / "witness.json"
        f.atomic_json(path, dict(known=True))
        self.assertEqual(f.private(path), dict(known=True))
        os.link(path, root / "hard")
        with self.assertRaises(ValueError): f.private(path)
        (root / "hard").unlink()
        (root / "link").symlink_to(path)
        with self.assertRaises(OSError): f.private(root / "link")
        path.chmod(0o644)
        with self.assertRaises(ValueError): f.private(path)
        path.chmod(0o600); root.chmod(0o755)
        with self.assertRaises(ValueError): f.private(path)


if __name__ == "__main__":
    unittest.main(verbosity=2)
