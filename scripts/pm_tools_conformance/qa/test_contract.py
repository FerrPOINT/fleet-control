"""Synthetic contract tests, not existing Fleet/Hermes integration evidence."""

from dataclasses import replace
import threading
import unittest
from unittest.mock import Mock

from contract import Binding, Held, Oracle, ProducerProof


def binding(run="run_one", **changes):
    base = Binding("runtime-one", run, "conversation-shared", "fleet-run-one",
                   "task-one", "root-one", "execution-one", "assignment-one",
                   7, "checkpoint-one", 3, 100)
    return replace(base, **changes)


class ContractTests(unittest.TestCase):
    def setUp(self):
        self.oracle = Oracle()
        self.b = binding()
        self.oracle.bind_fixture(self.b)
        self.send = Mock(return_value="accepted")

    def call(self, **kw):
        values = dict(peer=self.b.peer, run=self.b.native_run,
                      conversation=self.b.conversation, now=10, call_id="call-one",
                      tool="clarification_publish", payload={"content": "question?"},
                      send=self.send)
        values.update(kw)
        return self.oracle.call(**values)

    def test_conversation_is_not_native_run(self):
        with self.assertRaises(Held):
            self.call(run=self.b.conversation)
        self.send.assert_not_called()

    def test_wrong_peer_or_conversation_denied_before_io(self):
        for kw in ({"peer": "runtime-two"}, {"conversation": "other"}):
            with self.subTest(kw=kw), self.assertRaises(Held):
                self.call(**kw)
        self.send.assert_not_called()

    def test_missing_each_producer_primitive_never_calls_first_model(self):
        model = Mock()
        for proof in (ProducerProof(), ProducerProof(False, True, True),
                      ProducerProof(True, False, True), ProducerProof(True, True, False)):
            with self.subTest(proof=proof), self.assertRaises(Held):
                self.oracle.admit(self.b.peer, self.b.native_run, self.b.conversation,
                                  10, proof, model)
        model.assert_not_called()

    def test_positive_admission_is_explicitly_synthetic(self):
        sentinel = Mock(return_value="synthetic-not-a-model")
        self.assertEqual(self.oracle.admit(
            self.b.peer, self.b.native_run, self.b.conversation, 10,
            ProducerProof(True, True, True), sentinel), "synthetic-not-a-model")
        sentinel.assert_called_once_with(self.b)

    def test_stale_fence_or_any_identity_revision_denied(self):
        for change in ({"fence": 4}, {"revision": 8}, {"execution": "new"},
                       {"assignment": "new"}, {"checkpoint": "new"}, {"root": "new"}):
            with self.subTest(change=change):
                self.oracle.current[self.b.task] = replace(self.b, **change)
                with self.assertRaises(Held):
                    self.call()
        self.send.assert_not_called()

    def test_expired_at_boundary_denied_before_io(self):
        with self.assertRaises(Held):
            self.call(now=100)
        self.send.assert_not_called()

    def test_authority_in_model_payload_and_privileged_tools_denied(self):
        for field in ("task_id", "run_id", "token", "fence", "scope", "url"):
            with self.subTest(field=field), self.assertRaises(Held):
                self.call(payload={"content": "q", field: "forged"})
        for tool in ("answer", "confirm", "heartbeat", "workflow_transition", "shell"):
            with self.subTest(tool=tool), self.assertRaises(Held):
                self.call(tool=tool)
        self.send.assert_not_called()

    def test_unknown_write_timeout_prevents_redispatch(self):
        self.send.side_effect = TimeoutError("synthetic lost response")
        self.assertEqual(self.call(), "unknown")
        self.assertEqual(self.call(), "unknown")
        self.send.assert_called_once()
        with self.assertRaises(Held):
            self.call(payload={"content": "changed"})
        self.send.assert_called_once()

    def test_unknown_blocks_new_call_or_run_not_only_same_key_replay(self):
        self.send.side_effect = TimeoutError()
        self.call()
        with self.assertRaises(Held):
            self.call(call_id="call-new")
        replacement = binding("run_new", fence=4)
        self.oracle.bind_fixture(replacement)
        with self.assertRaises(Held):
            self.call(run=replacement.native_run, call_id="call-new")
        model = Mock()
        with self.assertRaises(Held):
            self.oracle.admit(replacement.peer, replacement.native_run, replacement.conversation,
                              10, ProducerProof(True, True, True), model)
        model.assert_not_called()
        self.send.assert_called_once()

    def test_non_receipt_result_is_unknown_not_success(self):
        self.send.return_value = {"ok": True}
        self.assertEqual(self.call(), "unknown")
        self.assertEqual(self.call(), "unknown")
        self.send.assert_called_once()

    def test_exact_readback_resolves_without_second_write(self):
        self.send.side_effect = TimeoutError()
        self.call()
        record = self.oracle.journal[(self.b.native_run, "call-one")]
        args = (self.b.peer, self.b.native_run, self.b.conversation, 10, "call-one")
        with self.assertRaises(Held):
            self.oracle.readback(*args, "wrong", self.b, "accepted")
        with self.assertRaises(Held):
            self.oracle.readback(*args, record["digest"], replace(self.b, fence=4), "accepted")
        self.oracle.readback(*args, record["digest"], self.b, "accepted")
        self.assertEqual(self.call(), "accepted")
        with self.assertRaises(Held):
            self.oracle.readback(*args, record["digest"], self.b, "rejected")
        self.send.assert_called_once()

    def test_rejected_write_is_not_delivered_or_retried(self):
        self.send.return_value = "rejected"
        self.assertEqual(self.call(), "rejected")
        self.assertEqual(self.call(), "rejected")
        self.send.assert_called_once()

    def test_late_send_result_cannot_overwrite_exact_terminal_readback(self):
        def send(*_):
            record = self.oracle.journal[(self.b.native_run, "call-one")]
            self.oracle.readback(self.b.peer, self.b.native_run, self.b.conversation,
                                 10, "call-one", record["digest"], self.b, "accepted")
            return "rejected"
        self.send.side_effect = send
        with self.assertRaises(Held):
            self.call()
        self.assertEqual(self.call(), "accepted")
        self.send.assert_called_once()

    def test_concurrent_same_call_consumes_one_send_permit(self):
        entered, release = threading.Event(), threading.Event()
        def slow(*_):
            entered.set()
            if not release.wait(3):
                raise AssertionError("fixture barrier timeout")
            return "accepted"
        self.send.side_effect = slow
        results = []
        worker = threading.Thread(target=lambda: results.append(self.call()))
        worker.start()
        try:
            self.assertTrue(entered.wait(3))
            self.assertEqual(self.call(), "unknown")
            self.send.assert_called_once()
        finally:
            release.set()
            worker.join(3)
        self.assertFalse(worker.is_alive())
        self.assertEqual(results, ["accepted"])

    def test_two_runs_same_conversation_do_not_share_binding(self):
        second = binding("run_two", peer="runtime-two", task="task-two",
                         fleet_run="fleet-run-two", execution="execution-two")
        self.oracle.bind_fixture(second)
        self.assertEqual(self.call(), "accepted")
        self.assertEqual(self.call(peer=second.peer, run=second.native_run), "accepted")
        with self.assertRaises(Held):
            self.call(peer=second.peer)
        self.assertEqual(self.send.call_args_list[0].args[0], self.b)
        self.assertEqual(self.send.call_args_list[1].args[0], second)
        self.assertEqual(self.send.call_count, 2)

    def test_old_run_not_rebound_to_new_assignment(self):
        replacement = binding("run_new", fence=4, revision=8)
        self.oracle.bind_fixture(replacement)
        with self.assertRaises(Held):
            self.call()
        with self.assertRaises(Held):
            self.oracle.bind_fixture(replace(self.b, fence=4))
        self.send.assert_not_called()
