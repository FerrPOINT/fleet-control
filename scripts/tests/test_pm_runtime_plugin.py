"""Plugin boundary tests; native Hermes and live owner admission are separate gates."""
import importlib.util
import json
import os
from pathlib import Path
import unittest
import asyncio
import sys
from types import SimpleNamespace
from unittest.mock import patch


PATH = Path(__file__).resolve().parents[2] / "runtime_plugins/fleet_pm/__init__.py"
SPEC = importlib.util.spec_from_file_location("fleet_pm_plugin", PATH)
PLUGIN = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PLUGIN)
AGENT = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"


class Context:
    def __init__(self):
        self.tools = {}
        self.middleware = {}
        self.hooks = {}

    def register_tool(self, **value):
        self.tools[value["name"]] = value
        return object()

    def register_middleware(self, name, handler):
        self.middleware[name] = handler

    def register_platform_handler(self, _name, handler):
        self.platform_handler = handler

    def register_hook(self, name, handler):
        self.hooks[name] = handler


class PluginTest(unittest.TestCase):
    def setUp(self):
        self.env = patch.dict(os.environ, {"FLEET_PM_AGENT_ID": AGENT,
            "FLEET_PM_FLEET_ORIGIN": "http://fleet.test/", "API_SERVER_KEY": "x" * 40,
            "FLEET_PM_OUTPUT_LIMIT": "8192"})
        self.env.start()
        self.addCleanup(self.env.stop)
        self.ctx = Context()
        PLUGIN.register(self.ctx)

    def request(self):
        return {"tools": [value["schema"] for value in self.ctx.tools.values()], "max_tokens": 8192}

    def test_only_registered_pm_tools_use_native_session_metadata(self):
        self.assertEqual(set(self.ctx.tools), set(PLUGIN.TOOLS))
        calls = []
        def owner(config, operation, session, body):
            calls.append((operation, session, body))
            return {"ok": True, "result": {"stage": "Draft"}}
        with patch.object(PLUGIN, "owner_request", owner):
            value = self.ctx.tools["fleet_pm_question"]["handler"]({"command": {"prompt": "Business goal?"}},
                session_id="native-session", task_id="native-task")
        self.assertTrue(json.loads(value)["ok"])
        self.assertEqual(calls[0][0:2], ("tools/question", "native-session"))
        self.assertNotIn("API_SERVER_KEY", str(calls))

    def test_cancelled_coroutine_requires_real_session_end_and_cannot_free_next_run(self):
        routes = {}
        app = SimpleNamespace(router=SimpleNamespace(add_get=lambda path, handler: routes.update({path: handler})))
        native = SimpleNamespace(session_id="native-old", skip_background_review=False)
        state = {"session_id": "native-old", "status": "cancelled"}
        adapter = SimpleNamespace(_active_run_agents={"run_old": native},
            _check_auth=lambda request: None, _request_owns_run=lambda request, run: True,
            _durable_run_status=lambda request, run: state)
        web = SimpleNamespace(json_response=lambda value, status=200: {"body": value, "status": status})
        with patch.dict(sys.modules, {"aiohttp": SimpleNamespace(web=web), "agent.title_generator": SimpleNamespace(_auto_title_enabled=lambda: False)}):
            self.ctx.platform_handler(app, adapter)
        self.ctx.hooks["on_session_start"](session_id="native-old", platform="api_server")
        self.assertTrue(native.skip_background_review)
        request = SimpleNamespace(match_info={"run_id": "run_old"})
        endpoint = routes["/fleet/v1/pm/quiescence/{run_id}"]
        adapter._active_run_agents.clear()  # Native /stop already retired its coroutine.
        self.assertFalse(asyncio.run(endpoint(request))["body"]["quiescent"])
        self.ctx.hooks["on_session_end"](session_id="native-other", platform="api_server")
        self.assertFalse(asyncio.run(endpoint(request))["body"]["quiescent"])
        self.ctx.hooks["on_session_end"](session_id="native-old", platform="api_server")
        self.assertTrue(asyncio.run(endpoint(request))["body"]["quiescent"])
        state["session_id"] = "native-next"
        self.assertFalse(asyncio.run(endpoint(request))["body"]["quiescent"])
        adapter._check_auth = lambda request: "denied"
        self.assertEqual(asyncio.run(endpoint(request)), "denied")

    def test_confirmed_checkpoint_cooperatively_interrupts_only_its_original_agent(self):
        interruptions = []
        native = SimpleNamespace(session_id="native-original", interrupt=lambda reason: interruptions.append(reason))
        adapter = SimpleNamespace(_active_run_agents={"run_actual": native})
        app = SimpleNamespace(router=SimpleNamespace(add_get=lambda *_args: None))
        with patch.dict(sys.modules, {"aiohttp": SimpleNamespace(web=SimpleNamespace()), "agent.title_generator": SimpleNamespace(_auto_title_enabled=lambda: False)}):
            self.ctx.platform_handler(app, adapter)
        handler = self.ctx.tools["fleet_pm_checkpoint"]["handler"]
        receipt = {"ok": True, "result": {"state": "waiting", "workflow_step_allowed": False,
                   "identity": {"agent_ref": AGENT}}}
        with patch.object(PLUGIN, "owner_request", return_value=receipt):
            handler({"command": {}}, session_id="native-original")
        self.assertEqual(len(interruptions), 1)
        self.assertIn("run_actual", adapter._active_run_agents)
        for session, altered in [("native-foreign", receipt), ("native-original", {"ok": True, "result": {"state": "active"}})]:
            with patch.object(PLUGIN, "owner_request", return_value=altered), self.assertRaises(ValueError):
                handler({"command": {}}, session_id=session)
        self.assertEqual(len(interruptions), 1)

    def test_admission_denial_and_exception_never_call_provider(self):
        gate = self.ctx.middleware["llm_execution"]
        provider = []
        def next_call(request):
            provider.append(request)
            return "actual-provider-result"
        for result in ({"ok": True, "allowed": False}, ValueError("private upstream detail")):
            with patch.object(PLUGIN, "owner_request", side_effect=result if isinstance(result, Exception) else None,
                              return_value=result):
                with self.assertLogs(PLUGIN.logger, level="WARNING") as captured:
                    self.assertIsNone(gate(self.request(), next_call, platform="api_server", session_id="native-session",
                        model="model", provider="provider", api_mode="chat_completions", base_url="http://model.test/"))
                self.assertNotIn("private upstream detail", str(captured.output))
                self.assertNotIn("native-session", str(captured.output))
        self.assertEqual(provider, [])

    def test_verified_admission_preserves_provider_and_rejects_extra_tools(self):
        gate = self.ctx.middleware["llm_execution"]
        provider = []
        def next_call(request):
            provider.append(request)
            return "actual-provider-result"
        with patch.object(PLUGIN, "owner_request", return_value={"ok": True, "allowed": True}):
            request = self.request()
            self.assertEqual(gate(request, next_call, platform="api_server", session_id="native-session",
                                  model="model", provider="provider", api_mode="chat_completions"),
                             "actual-provider-result")
            self.assertEqual(provider[0]["max_tokens"], 8192)
            request = self.request()
            request["tools"].append({"function": {"name": "terminal"}})
            self.assertIsNone(gate(request, next_call, platform="api_server", session_id="native-session",
                                   model="model", provider="provider", api_mode="chat_completions"))
        self.assertEqual(len(provider), 1)

    def test_invalid_fixed_origins_and_missing_identity_fail_before_network(self):
        for origin in ["file:///tmp/", "http://token@fleet.test/", "http://fleet.test/path", "http://fleet.test/?x=1"]:
            with patch.dict(os.environ, {"FLEET_PM_FLEET_ORIGIN": origin}):
                with self.assertRaises(ValueError):
                    PLUGIN.settings()
        with self.assertRaises(ValueError):
            PLUGIN.owner_request(PLUGIN.settings(), "admit", "", {})


if __name__ == "__main__":
    unittest.main()
