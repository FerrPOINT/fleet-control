# PM Tools: Current MCP Handoff

Status: implemented source path, not complete PostgreSQL or live PM acceptance.
Source baseline: `c3fc175b97168736717c72c2b32e1036c5b6f9db`.

## Supported Path

Fleet uses unchanged Hermes through its existing runtime API and HTTP MCP
client. The [runtime scope decision](CHAT_CLARIFICATION_CONTRACT.md#runtime-scope-decision-2026-10-10)
rejects a mandatory pre-model hook, reserved-native-run handshake, separate
controller or second scheduler. The
[full historical proposal at c3fc](https://github.com/FerrPOINT/fleet-control/blob/c3fc175b97168736717c72c2b32e1036c5b6f9db/docs/contracts/PM_TOOLS_HANDOFF_REQUIREMENTS.md)
is retained for evidence, not as current release requirements.

The [MCP route](../../backend/api/src/routes/pm_tools.rs) implements stateless
JSON Streamable HTTP at `/internal/runtime/v1/pm/agents/{agent_id}/mcp`, using
the isolated agent runtime bearer and protocol `2025-03-26`. Dispatch must be
enabled; browser Origin requests are rejected. Its six tools are:

| Tool                       | Implemented operation                                                                     |
| -------------------------- | ----------------------------------------------------------------------------------------- |
| `tracker_context`          | Read current machine assignment and requirements revision                                 |
| `tracker_clarifications`   | Read published questions and saved answers                                                |
| `tracker_requirements`     | Read immutable requirements revisions                                                     |
| `tracker_publish_revision` | Publish a requirements revision with the current assignment fence                         |
| `tracker_publish_question` | Publish a clarification and checkpoint the current PM run                                 |
| `workflow_step`            | Read instructions with `report: null`, or submit a report with its original operation key |

The [domain DTOs](../../backend/domain/src/pm_tools.rs) close the call envelope
to `operation_id`, `session_run_id` and `command`; command DTOs reject extra
fields. There are no answer, confirmation, credential or arbitrary-target tools.
Owner answers and exact-revision confirmation remain separate business actions.

## Existing Authority And Recovery

The [runtime implementation](../../backend/infra/src/runtime/pm_tools.rs)
resolves persisted operation/run/session custody, checks the original agent and
task binding, requires a current nonterminal accepted native run, and rechecks
Tracker assignment and Workflow state. Caller-supplied IDs do not grant authority.
Delegated Tracker credentials stay server-side; the
[isolated MCP profile](../../backend/infra/src/pm_tool_config.rs) references the
existing agent runtime credential, not a parent PAT or a new admission capability.

Writes retain original keys and bodies in the existing PM tool journal. Unknown
publication is reconciled by exact readback, not a replacement operation/run.
Tool errors return a fixed unconfirmed-command response, not provider/DB details.
Instruction receipts, saved answers, terminal proof and continuation remain
distinct; a tool receipt is not workflow completion or owner confirmation.

## Qualification Boundary

Current-source PG/HTTP, crash/replay and real owner-flow acceptance remain open;
see [current evidence](../CURRENT_STATE.md) and [release gaps](../GAP_REGISTER.md).
The [historical offline oracle](../../scripts/pm_tools_conformance/README.md)
tests the rejected proposal and is not a current admission release gate.

The newer external Tracker/Fleet fixed native-plugin contract is
[incompatible with this no-hook path](CHAT_CLARIFICATION_CONTRACT.md#external-pm-contract-divergence-11-october-2026),
not merely a missing HTTP endpoint. Tracker's
[exact87cd validator](https://github.com/FerrPOINT/task-tracker/blob/87cd86e8bbe02ea0fa384920e4d4a93e6e01a59c/backend/domain/src/sdlc_native_admission.rs#L59)
requires `chat_completions`, its exact native tool inventory and fresh native/config
observation. Renaming MCP tools cannot supply that native-plugin proof.
Do not silently repin dependencies, fabricate admission proof or add the rejected
hook. Source implementation and pure tests do not resolve that compatibility gap
or qualify live PM execution.
