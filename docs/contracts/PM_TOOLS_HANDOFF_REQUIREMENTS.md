# PM structured tools handoff: proposed contract, not runtime admission

## Superseded Producer Proposal (2026-10-10)

The product owner rejected modifying Hermes and the extra pre-model admission
mechanism proposed below. This document is retained as historical source/probe
evidence, NOT current release requirements. In particular its producer barrier,
native-run reservation handshake and `producer_admission=BLOCKED` conclusion
must not be used to block the existing Hermes API integration.

The current decision is in
[CHAT_CLARIFICATION_CONTRACT](CHAT_CLARIFICATION_CONTRACT.md#runtime-scope-decision-2026-10-10).
Fleet must implement supported runtime dispatch/tools and existing task/workflow
coordination without changing Hermes. Ordinary backend authorization, owner-only
answers/confirmation, isolation, delivery idempotency and uncertainty recovery
remain required. Offline results below are still not live PM acceptance.

## Historical Packet

Status: offline executable conformance packet. No plugin is installed and no
gateway, credential handoff, HTTP endpoint, capability or model path is enabled.
The contract identifier `pm-tools-handoff/v1` below is a proposal owned by this
packet, NOT a feature advertised by pinned Hermes or Fleet.

## Exact source inputs

| Product | Immutable commit | Read-only source |
| --- | --- | --- |
| Fleet | `ede1e41e843b3992757d4e16c051db6667da7add` | Git object in the runner's own Fleet checkout (default `--fleet-pin`) |
| Hermes | `bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3` | Explicit `--hermes-repo` clean checkout at this fixed pin |

Source locations/lines refer to these commits, not future mutable donor HEADs.
The [runner](../../scripts/pm_tools_conformance/README.md) reads Git objects,
executes canonical Python blobs in memory and
records each imported blob ID and SHA-256. It does not export a full checkout,
reuse a venv/cache or import Hermes worktree Python bytes. Normal imports and
dynamic `spec_from_file_location` provider imports use the same blob loader.

## Existing source facts

1. `hermes_cli/plugins.py:460` supplies real `PluginContext.register_tool`.
   Registration owns a profile-scoped registry slot with a disposable handle.
   `tools/registry.py:845` dispatches that handler. Availability/check functions
   are not authorization. No model tool schema should carry tokens or task authority.
2. `gateway/platforms/api_server_runs.py:351` defines `_RunLaunch`;
   `approval_session_key` returns its exact native `run_id` at line 375.
   `_run_agent_sync:549` binds that key, but the agent's `task_id` is
   `session_id or run_id`: with a conversation this is NOT the native run ID.
   The caller's gateway routing key is not the native authorization namespace.
3. `model_tools.py:817` (`_execute_tool`) binds approval observability IDs during
   real tool execution; task/session kwargs remain conversation-oriented.
   These internal ContextVars are executable selectors, NOT a supported,
   authenticated cross-process PM identity API or an admission proof.
4. `tools/approval_context.py:102` and `gateway/session_context.py:173` retain
   process-environment fallbacks. An env-only value must not authorize a call.
   A copied old Context may still contain an old run after its native turn ends.
   Fresh server-owned binding/fence checks remain necessary on every request.
5. `hermes_cli/plugins_dispatch.py:188` isolates hook failures.
   `agent/turn_context.py:666` only collects `pre_llm_call` text/context. A denial
   dict is not a veto and a throwing hook yields no context, not a rejected model
   turn. Tests execute this collector but deliberately do NOT call a model.
6. `gateway/platforms/api_server.py:67,77,2271` describes existing capabilities;
   there is no advertised PM handoff or fail-closed pre-model barrier contract.
   The existing runs handler schedules an agent run, not a PM-reserved blocked
   run followed by authenticated admission. Room policy is not PM task authority.
7. Fleet `backend/infra/src/pm_credentials.rs:93,172` already restricts delegated
   Tracker request paths, methods, origin, scopes and expiry. Parent PAT stays
   server-side. The coordinator's `prepare:258` discards the prepared in-memory
   credential with `.map(|_| ())`; no runtime delivery is implemented by it.
8. Fleet `backend/infra/src/runtime/mod.rs:1197` explicitly holds task-bound
   admission. The PM runtime readback credential is a separate authority and
   must not be repurposed for tools. This packet changes none of those guards.
9. `tools/mcp_tool_handlers.py:374` calls MCP with tool arguments; profile-scoped
   MCP configuration alone does not establish authenticated native run identity.
   This packet does not qualify MCP as a safe alternate admission mechanism.

## Minimal vertical contract

Implementable Fleet-owned pieces are a closed tool-call validator, server-only
credential-handle registry, immutable operation journal, exact scoped readback,
and an out-of-tree tool registration adapter. They can be developed/tested while
task runtime admission stays held. None is installed or implemented as a service
by this packet; `qa/contract.py` is only an in-memory contract oracle.

### Producer boundary required before the first model

The producer must reserve the actual native run ID before a model can execute.
Its exact run/conversation/profile/controller generation and fixed toolset
snapshot must be bound to an authenticated server-owned reservation. A
versioned producer capability/proof must establish ALL of:

- Explicit native-run context, distinct from conversation/session/task labels,
  propagated and cleared across async, executor and parallel-tool boundaries.
- An awaited fail-closed admission barrier before the FIRST provider/model
  request. Denial, timeout, missing callback, exception, stale fence and unknown
  acceptance cannot invoke a provider, retry launch or create a replacement run.
- Server-only credential custody and authenticated callback transport. No
  inherited parent PAT, raw delegated bearer in prompts/env/model args, or
  accidental reuse of Fleet's runtime readback bearer.

Admission must bind the reserved native run to Fleet's session_run_id, runtime
instance/controller generation, Tracker instance/project/task/root/agent,
execution, assignment operation/ref/revision, checkpoint and current Workflow
lease/fence. The proposed oracle abbreviates these with fixture strings; it
does NOT validate the full closed production DTO, TLS or a Workflow protocol.
No positive proof can be inferred from plugin presence or an arbitrary JSON
`capabilities` key. Capability negotiation and the barrier must be implemented
and exercised by the producer before admission can be enabled.

### Tool call boundary

The trusted producer/adapter supplies native run ID, native turn/tool-call ID,
conversation/profile/controller identity and an authenticated runtime peer.
The Fleet gateway resolves the exact immutable reservation. It never infers
"the currently active run" from a conversation, accepts model-supplied task IDs,
or falls back to environment identity. ContextVars alone cannot authenticate
the HTTP peer. Actual transport/path/auth scheme remain unimplemented; this
document deliberately declares no existing or newly deployed endpoint.

Every call checks fresh assignment/context, stage, binding, lease/fence, issuer
credential expiry and exact allowed operation. Cross-task calls, foreign peers,
replaced assignments, old runs and expired credentials are held before Tracker
I/O. Reconnect never silently rebinds an old call to a new task/run/credential.
Schemas/toolsets stay profile-stable for prompt-cache compatibility; authorization
is per call, not a cached `check_fn` or a per-run mutation of global registries.

Candidate tools: read context/clarifications/requirements; publish a clarification,
cancel an exact scoped clarification, publish a requirements revision. Answer,
confirm, transition Workflow, renew credentials and lease heartbeat are NOT
model tools. Cancel must verify a canonical question identity belongs to the
same task/assignment; publish must use Tracker's actual closed DTOs.

The oracle uses `{"content": "..."}` as a deliberately small synthetic content
envelope for scope/idempotency tests. It is NOT the Tracker DTO or a deployable
wire schema. Actual question UUID/revision/hash validation, full DTO closure,
redaction and integration with existing Tracker endpoints remain required.

### Write acceptance and recovery

Before any write, persist its exact server-derived operation key, original native
run/tool-call ID, immutable canonical request/body hash and original authority
binding. Commit one send permit before I/O, with no automatic HTTP retry,
redirect, hidden key renewal or second native tool submission. The producer must
preserve operation identity across retries; an uncertain operation cannot escape
by obtaining a new model tool-call ID, turn or native run.

Lost response, malformed/unbound receipt or crash after send is UNKNOWN, not
delivered/rejected/completed. It blocks new task writes and admission, including
new call/run IDs, until exact authenticated readback resolves the ORIGINAL
operation. Repeated known accepted/rejected calls return the retained state,
never write again. Contradictory body/binding/terminal receipts are held.

Readback must prove exact original operation, body digest and task/run authority,
not merely observe a similar question/revision in a list. Requirements operation
identity/readback and crash-durable gateway storage are not established by this
packet. If the pinned APIs cannot prove acceptance, remain held; no readback or
idempotency endpoint may be invented. Credential loss/restart is also held;
recovery/reissuance needs a separately reviewed custody protocol, not PAT fallback.

## Executed and future acceptance matrix

Current execution has two distinct layers:

| Layer | Executed proof | Not proved |
| --- | --- | --- |
| Real Hermes probes (8) | Real register/dispatch/dispose; native run binding; real tool execution middleware and observability; env fallback negative; concurrent native turns; exception clear; inherited stale-context rejection using fixture binding; real hook context non-veto | Plugin discovery/install, HTTP admission, model/provider, authenticated transport, full AIAgent/parallel tool executor |
| Synthetic oracle (16) | Missing each primitive => zero first-model sentinel calls; exact run/peer/session; stale/expired fence; scope/payload authority denial before I/O; concurrent one permit; unknown/replay/new-run hold; exact fixture readback; rejected not delivered; late contradictory send cannot overwrite terminal readback | Production gateway, credential custody, database durability, actual Tracker mutation/readback, producer barrier |

Mandatory producer/integration tests before admission (NOT executed here):

1. Start a reserved native run with a controlled provider recorder. Missing or
   stale admission proof, callback timeout/raise/deny and absent native identity
   each produce exactly ZERO first-provider requests and ZERO tool writes.
2. A positive server-authenticated, current exact binding yields one admitted
   original run; verify it is the same run seen by tools, not its conversation.
   No acceptance is claimed until a genuine producer supports this test.
3. Two concurrent tasks and two runs sharing a conversation retain separate
   tool authority. Executor/parallel/subagent boundaries cannot inherit/rebind
   task authority; undeclared delegated identities fail closed.
4. Rotate assignment/lease/controller/fence during a delayed tool call; stale
   work is rejected before I/O. All receiver-side fences must cover the actual
   mutation transaction, not only a stale preflight snapshot.
5. Lose a Tracker write response and restart gateway/controller. Durable original
   intent consumes at most one send permit; retry/reconnect/new tool/run ID
   cannot redispatch. Exact readback resolves once; unsupported readback holds.
6. Assert no raw credentials in model requests, tool results, logs, transcripts,
   journal metadata, subprocess env or callback payloads; foreign peer is denied.
7. Preserve the current task-bound hold unless all producer, custody, Workflow
   and Tracker contracts above pass on exact source/binary inputs.

Until those tests and missing primitives are satisfied: `runtime_ready=false`,
`producer_admission=BLOCKED`. Offline green tests do not grant admission.
