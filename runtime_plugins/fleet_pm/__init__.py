"""Fleet-owned extension for pinned Hermes; no assignment or Tracker parent tokens."""
import hashlib
import json
import logging
import os
from threading import Lock
from urllib.error import HTTPError, URLError
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, ProxyHandler, Request, build_opener
from uuid import UUID


TOOLS = {
    "fleet_pm_context": ("context", "Read this assigned Task, its current revision and questions."),
    "fleet_pm_question": ("question", "Save one structured clarification question."),
    "fleet_pm_requirements": ("requirements", "Save a structured immutable requirements revision."),
    "fleet_pm_workflow": ("workflow", "Read the current phase or submit its report."),
    "fleet_pm_checkpoint": ("checkpoint", "Save this run's awaiting-input checkpoint."),
    "fleet_pm_skills": ("skills", "Read an approved skill for this current phase."),
}
LIMIT = 256 * 1024
logger = logging.getLogger(__name__)


def refusal(reason):
    logger.warning("Fleet PM provider admission refused: %s", reason)
    return None


def command_schema(operation):
    string = {"type": "string"}
    if operation == "context":
        return {"type": "object", "properties": {}, "additionalProperties": False}
    if operation in {"workflow", "skills"}:
        properties = {"operation_key": {**string, "maxLength": 128},
            "expected_phase_code": string, "expected_status": {"type": "string", "enum": ["active", "blocked"]}}
        properties["report" if operation == "workflow" else "name"] = (
            {"type": ["string", "null"], "description": "null reads the phase; text submits evidence"}
            if operation == "workflow" else string)
        return {"type": "object", "properties": properties,
                "required": list(properties), "additionalProperties": False}
    if operation == "requirements":
        return {"type": "object", "properties": {
            "expected_requirement_revision": {"type": ["integer", "null"]},
            "document": {"type": "object", "description": "Tracker RequirementsDocument: goal; arrays scope, exclusions, scenarios, acceptance_criteria, constraints, dependencies, assumptions, checklist, prerequisites"},
            "idempotency_key": {**string, "maxLength": 128}},
            "required": ["expected_requirement_revision", "document", "idempotency_key"], "additionalProperties": False}
    if operation == "checkpoint":
        return {"type": "object", "properties": {"operation_key": {**string, "maxLength": 128},
            "question_id": {**string, "format": "uuid"}},
            "required": ["operation_key", "question_id"], "additionalProperties": False}
    if operation == "question":
        properties = {key: string for key in ["request_id", "question_id", "checkpoint_id", "text", "rationale", "idempotency_key"]}
        properties.update({"expected_question_version": {"type": ["integer", "null"]},
            "requirement_revision": {"type": "integer", "minimum": 1},
            "requirement_reference": {"type": ["string", "null"]}, "required": {"type": "boolean"},
            "mode": {"type": "string", "enum": ["single", "multiple", "text"]},
            "options": {"type": "array", "items": {"type": "object", "description": "id UUID, label, consequences, is_custom boolean"}},
            "recommended_option_id": {"type": ["string", "null"]}})
        return {"type": "object", "properties": properties, "required": list(properties), "additionalProperties": False}
    return {"type": "object"}


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        return None


def settings():
    agent = os.environ.get("FLEET_PM_AGENT_ID", "")
    if str(UUID(agent)) != agent or UUID(agent).int == 0:
        raise ValueError("Canonical PM agent identity required")
    origin = os.environ.get("FLEET_PM_FLEET_ORIGIN", "")
    url = urlsplit(origin)
    if (url.scheme not in {"http", "https"} or not url.hostname or url.username or url.password
            or url.query or url.fragment or url.path not in {"", "/"}):
        raise ValueError("Fixed Fleet origin required")
    token = os.environ.get("API_SERVER_KEY", "")
    if not 32 <= len(token) <= 4096 or not all(33 <= ord(char) <= 126 for char in token):
        raise ValueError("Managed native credential required")
    return agent, origin.rstrip("/"), token


def owner_request(config, operation, session_id, body):
    agent, origin, token = config
    if (not isinstance(session_id, str) or not session_id or len(session_id) > 512
            or any(char.isspace() or ord(char) < 32 for char in session_id)):
        raise ValueError("Native session context required")
    payload = json.dumps({"session_id": session_id, **body}, ensure_ascii=False,
                         separators=(",", ":")).encode()
    if len(payload) > LIMIT:
        raise ValueError("PM command exceeds limit")
    request = Request(f"{origin}/internal/runtime/v1/pm/agents/{agent}/{operation}", data=payload,
                      headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json",
                               "Accept-Encoding": "identity", "Cache-Control": "no-cache, no-store"})
    try:
        with build_opener(ProxyHandler({}), NoRedirect()).open(request, timeout=10) as response:
            if (response.status != 200 or response.headers.get_content_type() != "application/json"
                    or response.headers.get("Content-Encoding", "identity") != "identity"):
                raise ValueError("PM owner response unavailable")
            raw = response.read(LIMIT + 1)
            if len(raw) > LIMIT:
                raise ValueError("PM owner response exceeds limit")
            value = json.loads(raw)
    except (HTTPError, URLError, OSError, ValueError):
        # Never put authenticated URL/error bodies or server credentials into model output.
        raise ValueError("PM owner operation unavailable") from None
    if not isinstance(value, dict) or value.get("ok") is not True:
        raise ValueError("PM owner operation unavailable")
    return value


def register(ctx):
    config = settings()
    output_limit = int(os.environ.get("FLEET_PM_OUTPUT_LIMIT", "0"))
    if not 1 <= output_limit <= 1_000_000:
        raise ValueError("Explicit PM output limit required")
    request_lock = Lock()
    latest_request = {}
    adapters = []
    ended_sessions = set()

    def session_start(*, session_id="", platform="", **_metadata):
        if platform != "api_server" or len(adapters) != 1:
            return
        with request_lock:
            ended_sessions.discard(session_id)
        matches = [agent for agent in adapters[0]._active_run_agents.values()
                   if getattr(agent, "session_id", None) == session_id]
        if len(matches) == 1 and type(getattr(matches[0], "skip_background_review", None)) is bool:
            # This PM profile has no autonomous memory/skills review authority.
            # Apply the existing native option before the first provider call.
            matches[0].skip_background_review = True

    def session_end(*, session_id="", platform="", **_metadata):
        if platform == "api_server":
            with request_lock:
                ended_sessions.add(session_id)

    ctx.register_hook("on_session_start", session_start)
    ctx.register_hook("on_session_end", session_end)

    for name, (operation, description) in TOOLS.items():
        def handler(arguments, *, session_id="", _operation=operation, **_metadata):
            if not isinstance(arguments, dict):
                raise ValueError("Structured PM arguments required")
            value = owner_request(config, "tools/" + _operation, session_id, {"arguments": arguments})
            if _operation == "checkpoint":
                receipt = value.get("result", {})
                if (receipt.get("state") != "waiting" or receipt.get("workflow_step_allowed") is not False
                        or receipt.get("identity", {}).get("agent_ref") != config[0] or len(adapters) != 1):
                    raise ValueError("PM waiting acknowledgement unavailable")
                matches = [agent for agent in adapters[0]._active_run_agents.values()
                           if getattr(agent, "session_id", None) == session_id]
                if len(matches) != 1:
                    raise ValueError("PM original native conversation unavailable")
                # Cooperative interruption retains the native executor until its
                # real finalizer exits; cancelling its coroutine would retire early.
                matches[0].interrupt("Awaiting the original structured clarification answer")
            return json.dumps(value, ensure_ascii=False)

        schema = {"type": "function", "function": {"name": name, "description": description,
                  "parameters": {"type": "object", "properties": {"command": command_schema(operation)},
                                 "required": ["command"], "additionalProperties": False}}}
        if ctx.register_tool(name=name, toolset="fleet_pm", schema=schema, handler=handler) is None:
            raise ValueError("PM tool registration failed")

    def admission(request, next_call, *, session_id="", platform="", model="", provider="",
                  base_url="", api_mode="", **_metadata):
        # Execution middleware exceptions normally fall through in pinned Hermes.
        # Refusal returns no provider response; only verified admission calls next_call.
        try:
            if platform != "api_server" or not isinstance(request, dict):
                return refusal("native_request_context")
            names = sorted(tool.get("function", tool).get("name", "") for tool in request.get("tools", []))
            if names != sorted(TOOLS):
                logger.warning("Fleet PM provider admission refused: tool_inventory expected=%s observed=%s",
                               sorted(TOOLS), names)
                return None
            if (api_mode != "chat_completions"
                    or any(not isinstance(value, str) or not value or len(value) > 512
                           for value in (model, provider, api_mode))):
                return refusal("model_configuration")
            # The gateway constructor has no max_tokens option at this pin.
            # Apply the owner-frozen bound to the actual provider request.
            request["max_tokens"] = output_limit
            request.pop("max_completion_tokens", None)
            request.pop("max_output_tokens", None)
            observed = {"model": model, "provider": provider, "api_mode": api_mode,
                "route_sha256": hashlib.sha256(base_url.encode()).hexdigest(), "tools": names,
                "output_limit": output_limit}
            with request_lock:
                latest_request[session_id] = observed
            approved = owner_request(config, "admit", session_id, {"native_configuration": {
                "model": model, "provider": provider, "api_mode": api_mode,
                "route_sha256": hashlib.sha256(base_url.encode()).hexdigest(), "tools": names,
                "output_limit": output_limit,
            }})
            if approved.get("allowed") is not True:
                return refusal("owner_denied")
        except Exception:
            return refusal("owner_unavailable")
        return next_call(request)

    ctx.register_middleware("llm_execution", admission)

    def inventory_routes(app, adapter):
        from aiohttp import web
        from agent.title_generator import _auto_title_enabled
        adapters.append(adapter)

        async def inventory(request):
            denied = adapter._check_auth(request)
            if denied is not None:
                return denied
            run_id = request.match_info["run_id"]
            # This is the actual constructed run agent while its first provider
            # request is fenced by admission, never a configuration-only probe.
            agent = adapter._active_run_agents.get(run_id)
            if agent is None:
                return web.json_response({"error": "native_inventory_unavailable"}, status=409)
            context_limit = getattr(getattr(agent, "context_compressor", None), "context_length", None)
            with request_lock:
                observed = latest_request.get(agent.session_id)
            actual_output_limit = observed.get("output_limit") if observed else None
            turn_budget = getattr(agent, "max_iterations", None)
            tools = sorted(getattr(agent, "valid_tool_names", ()))
            if (tools != sorted(TOOLS) or any(type(value) is not int or value <= 0
                                             for value in (context_limit, actual_output_limit, turn_budget))):
                return web.json_response({"error": "native_inventory_unsupported"}, status=409)
            result = {"contract_version": 1, "agent_id": config[0], "native_run_ref": run_id,
                "session_id": agent.session_id, "model": agent.model, "provider": agent.provider, "api_mode": agent.api_mode,
                "context_limit": context_limit, "output_limit": actual_output_limit, "turn_budget": turn_budget,
                "tools": tools, "route_sha256": hashlib.sha256(agent.base_url.encode()).hexdigest(),
                "background_review_disabled": getattr(agent, "skip_background_review", None) is True,
                "automatic_titles_disabled": _auto_title_enabled() is False,
                "memory_disabled": getattr(agent, "_memory_enabled", None) is False
                    and getattr(agent, "_user_profile_enabled", None) is False}
            return web.json_response(result)

        async def quiescence(request):
            denied = adapter._check_auth(request)
            if denied is not None:
                return denied
            run_id = request.match_info["run_id"]
            if not adapter._request_owns_run(request, run_id):
                return web.json_response({"error": "native_run_unknown"}, status=404)
            status = adapter._durable_run_status(request, run_id)
            if not isinstance(status, dict) or not isinstance(status.get("session_id"), str):
                return web.json_response({"error": "native_run_unknown"}, status=409)
            session = status["session_id"]
            with request_lock:
                ended = session in ended_sessions
            # /stop can retire the coroutine before its provider thread exits.
            # Only the real conversation finalizer proves this run session ended.
            return web.json_response({"contract_version": 1, "agent_id": config[0],
                "native_run_ref": run_id, "session_id": session, "quiescent": ended})

        app.router.add_get("/fleet/v1/pm/inventory/{run_id}", inventory)
        app.router.add_get("/fleet/v1/pm/quiescence/{run_id}", quiescence)

    ctx.register_platform_handler("api_server", inventory_routes)
