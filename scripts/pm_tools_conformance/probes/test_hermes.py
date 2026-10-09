"""Pinned real Hermes registration/context execution; no HTTP server or model.

Only adapter profile binding/process ownership and AIAgent are boundary doubles.
No fake Hermes modules, plugin installation or production authorization path.
"""

import contextvars
from contextlib import nullcontext
import json
import os
from pathlib import Path
from types import SimpleNamespace
import threading
import unittest
from unittest.mock import patch

from hermes_cli.plugins import PluginContext, PluginManager
from hermes_cli.plugins_manifest import PluginManifest
from hermes_constants import hermes_home_key
from tools.registry import registry
from tools.approval_context import get_current_session_key
from gateway.session_context import get_session_env, set_session_vars
from gateway.platforms.api_server_runs import _RunLaunch, _run_agent_sync

from contract import Held, Oracle, explicit_context
from test_contract import binding


def launch(run_id="run_one", conversation="conversation-shared"):
    return _RunLaunch(
        owner=None, run_id=run_id, queue=None, session_id=conversation,
        gateway_session_key="conversation-route-not-authority", declared_selected=False,
        user_message="fixture only", conversation_history=[], session_history_delivery=False,
        agent_kwargs={"room_dispatch": None}, request_profile=None,
        browser_control_principal=None, browser_control_transport_family=None,
    )


class AdapterBoundary:
    def _profile_scope(self, _):
        return nullcontext()

    def _bind_api_server_session(self, **kwargs):
        return set_session_vars(platform="api_server", **kwargs)


class AgentBoundary:
    def __init__(self, callback):
        self.callback = callback

    def run_conversation(self, **kwargs):
        return self.callback(kwargs)


def native_call(run, callback):
    ownership = []
    server = SimpleNamespace(
        _publish_turn_process_ownership=lambda agent, task: ownership.append(task),
        _clear_turn_process_ownership=lambda agent: ownership.append("cleared"),
    )
    result = _run_agent_sync(AdapterBoundary(), run, AgentBoundary(callback),
                             lambda *_: None, _api_server=server)
    return result, ownership


class HermesProbes(unittest.TestCase):
    def setUp(self):
        self.scope = hermes_home_key(Path(os.environ["HERMES_HOME"]))
        self.manager = PluginManager(scope_key=self.scope)
        self.ctx = PluginContext(PluginManifest(name="pm-contract-probe", source="user",
                                               path=self.scope), self.manager)
        self.handles = []
        self.addCleanup(self.dispose)

    def dispose(self):
        for handle in reversed(self.handles):
            handle.dispose()

    def register(self, handler):
        handle = self.ctx.register_tool(
            "pm_contract_probe", "pm_contract_probe",
            {"name": "pm_contract_probe", "description": "Fixture only",
             "parameters": {"type": "object", "properties": {}}}, handler,
        )
        self.assertIsNotNone(handle)
        self.handles.append(handle)

    def test_real_registration_dispatch_and_disposal(self):
        calls = []
        self.register(lambda args, **kw: calls.append((args, kw)) or json.dumps({"fixture": True}))
        result = registry.dispatch("pm_contract_probe", {"content": "q"}, scope=self.scope,
                                   task_id="conversation-shared")
        self.assertEqual(json.loads(result), {"fixture": True})
        self.assertEqual(calls, [({"content": "q"}, {"task_id": "conversation-shared"})])
        self.handles.pop().dispose()
        self.assertIsNone(registry.get_entry("pm_contract_probe", scope=self.scope))

    def test_real_native_run_context_reaches_registered_handler_not_task_id(self):
        from model_tools import _CallIds, _execute_tool
        def handler(args, **kwargs):
            run, conversation = explicit_context(contextvars.copy_context())
            values = {var.name: value for var, value in contextvars.copy_context().items()}
            return json.dumps([run, conversation, kwargs["task_id"], get_current_session_key(),
                               values.get("approval_tool_call_id"), values.get("approval_turn_id")])
        self.register(handler)
        run = launch()
        def callback(kwargs):
            return _execute_tool(
                "pm_contract_probe", {}, {},
                _CallIds(task_id=kwargs["task_id"], session_id=run.session_id,
                         tool_call_id="native-tool-call-one", turn_id="native-turn-one"),
                user_task=None, enabled_tools=None, skip_tool_execution_middleware=False,
            )
        (result, usage), ownership = native_call(run, callback)
        self.assertEqual(json.loads(result), ["run_one", "conversation-shared",
                                             "conversation-shared", "run_one",
                                             "native-tool-call-one", "native-turn-one"])
        self.assertEqual(run.approval_session_key, run.run_id)
        self.assertEqual(ownership, ["conversation-shared", "cleared"])
        self.assertEqual(usage, {"input_tokens": 0, "output_tokens": 0, "total_tokens": 0})
        with self.assertRaises(Held):
            explicit_context(contextvars.copy_context())

    def test_legacy_env_fallback_is_observable_but_rejected_as_authority(self):
        def clean_context():
            with patch.dict(os.environ, {"HERMES_SESSION_KEY": "run_env_forged",
                                         "HERMES_SESSION_ID": "conversation-forged"}):
                self.assertEqual(get_current_session_key(), "run_env_forged")
                self.assertEqual(get_session_env("HERMES_SESSION_ID"), "conversation-forged")
                with self.assertRaises(Held):
                    explicit_context(contextvars.copy_context())
        contextvars.Context().run(clean_context)

    def test_registered_tool_without_bound_run_is_held_before_gateway(self):
        gateway_calls = []
        def handler(args, **kwargs):
            try:
                selected = explicit_context(contextvars.copy_context())
            except Held:
                return json.dumps({"state": "held"})
            gateway_calls.append(selected)
            return json.dumps({"state": "not-qualified"})
        self.register(handler)
        def unbound():
            with patch.dict(os.environ, {"HERMES_SESSION_KEY": "run_env_forged"}):
                return registry.dispatch("pm_contract_probe", {}, scope=self.scope,
                                         task_id="conversation-shared")
        self.assertEqual(json.loads(contextvars.Context().run(unbound)), {"state": "held"})
        self.assertEqual(gateway_calls, [])

    def test_inherited_stale_native_context_needs_fresh_server_binding(self):
        oracle = Oracle()
        old = binding("run_old")
        oracle.bind_fixture(old)
        (saved, _), _ = native_call(launch("run_old"), lambda _: contextvars.copy_context())
        oracle.bind_fixture(binding("run_new", fence=4, revision=8))
        def stale_worker():
            run, conversation = explicit_context(contextvars.copy_context())
            self.assertEqual(run, "run_old")
            with self.assertRaises(Held):
                oracle.authorize(old.peer, run, conversation, 10)
        saved.run(stale_worker)

    def test_two_concurrent_native_runs_same_conversation_isolated(self):
        barrier = threading.Barrier(2, timeout=4)
        results, errors = {}, []
        def worker(run_id):
            try:
                def callback(_):
                    barrier.wait()
                    return explicit_context(contextvars.copy_context())
                (result, _), _ = native_call(launch(run_id), callback)
                results[run_id] = result
                with self.assertRaises(Held):
                    explicit_context(contextvars.copy_context())
            except BaseException as exc:
                errors.append(exc)
        workers = [threading.Thread(target=worker, args=(name,)) for name in ("run_A", "run_B")]
        for worker in workers:
            worker.start()
        for worker in workers:
            worker.join(6)
        self.assertTrue(all(not worker.is_alive() for worker in workers))
        self.assertEqual(errors, [])
        self.assertEqual(results, {"run_A": ("run_A", "conversation-shared"),
                                   "run_B": ("run_B", "conversation-shared")})

    def test_native_exception_clears_context_and_next_run_is_not_stale(self):
        def broken(_):
            raise ValueError("fixture failure")
        with self.assertRaisesRegex(ValueError, "fixture failure"):
            native_call(launch("run_old"), broken)
        with self.assertRaises(Held):
            explicit_context(contextvars.copy_context())
        (result, _), _ = native_call(launch("run_new"), lambda _: explicit_context(contextvars.copy_context()))
        self.assertEqual(result, ("run_new", "conversation-shared"))

    def test_pre_llm_hook_denial_and_failure_are_not_a_model_veto(self):
        from agent.turn_context import _collect_pre_llm_call_context
        import hermes_cli.lifecycle
        agent = SimpleNamespace(session_id="conversation-shared", model="NOT-CALLED")
        def collect():
            return _collect_pre_llm_call_context(
                agent, effective_task_id="conversation-shared", turn_id="fixture-turn",
                original_user_message="fixture", messages=[], conversation_history=[],
            )
        denial = self.ctx.register_hook("pre_llm_call", lambda **_: {"allow": False})
        self.handles.append(denial)
        with patch.object(hermes_cli.lifecycle, "invoke_hook", self.manager.invoke_hook):
            self.assertEqual(collect(), "")
            denial.dispose()
            def fail(**_):
                raise RuntimeError("synthetic admission denial")
            self.handles.append(self.ctx.register_hook("pre_llm_call", fail))
            self.assertEqual(collect(), "")
        # No model callback is invoked: this tests the real context collector only.
