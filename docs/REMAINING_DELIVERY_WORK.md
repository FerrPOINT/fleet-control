# Remaining Delivery Work

## Scope

Deliver the agreed runtime/execution scope: isolated agents, configuration
activation, Tracker assignment consumption, Hermes dispatch/recovery, PM tools
and continuation, production Chats, Forge delivery and necessary Base utilities.
Tracker and Workflow remain read-only producer dependencies. Do not add a second
scheduler, host-controller service or custom Hermes authorization hook.

The PM clarification vertical is independently testable, but does not replace
the full runtime/execution objective. Leaders are outside this scope. Java
lifecycle remains available; automated Java SDLC needs its own capabilities and
execution evidence.

## Delivery Order

| Workstream                | Remaining implementation or qualification                                                                                              | Acceptance boundary                                                                                                                                |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Combined Fleet source     | Reconcile the current Chats foundation with PM runtime work; preserve private history, task-bound access and original command recovery | One reviewed candidate preserving both Git histories; generated API/client and complete backend/frontend gates on that source                      |
| Agents and configuration  | Qualify isolation, readiness, drain, activation, interrupted recovery and rollback                                                     | Two real isolated Hermes agents; peer unchanged; no configuration replacement during an active run; exact rollback and cleanup                     |
| Assignment execution      | Finish compatible producer-backed admission, heartbeat, fencing and first workflow step                                                | Stale/replaced assignments cannot perform side effects; Fleet consumes Tracker assignments rather than scheduling business stages                  |
| Hermes execution          | Qualify dispatch, original-key unknown-ACK recovery, streaming and human controls                                                      | One native run after crash/retry; EOF is not completion; final response stored once; stop ACK alone is not safe-stop proof                         |
| PM tools and continuation | Qualify scoped tools, saved-answer delivery, checkpoint and workflow rebind                                                            | Real Tracker/Workflow/Base/Hermes calls; original command survives restart; old run is terminal or safely stopped before continuation              |
| Production Chats          | Complete reviewed creation entrypoint and current history/control/clarification UI qualification                                       | Agent -> tasks navigation, own-user default, owner-only answers/confirmation, preserved input, explicit partial success and denied foreign access  |
| Forge                     | Complete exact-source full12 and task delivery/rollback acceptance                                                                     | Real candidate commit, repo pipeline, isolated attempt, artifacts, deployment, health and acceptance receipts; cleanup preserves permanent runtime |
| Base                      | Qualify the task-owned maintenance/auth inputs and actual consumers                                                                    | Exact published input, private CI and native consumer checks; no silent installed-packet promotion or clarification business logic in Base         |
| Release                   | Synchronize docs/captures and qualify task-owned release units                                                                         | Current reviewed heads, migration ownership, generated contracts, required checks and explicit remaining dependencies                              |

## Known Producer Dependencies

Tracker PR114 at `357caa7a60a717eb7b0ac72f286b793326992931` provides Analysis
intent/reservation/heartbeat, not completed execution admission or safe-stop
release/reacquire. Architect, Developer, Reviewer, Tester and DevOps execution
lifecycles remain missing producer dependencies.

Workflow PR90 at `66e5d6db9fc2ae9129c9162688bacb1a98c7a4a3` provides the
role/mode catalog; catalog presence is not execution authority. PM continuation
requires a genuine waiting checkpoint and answer event. An arbitrary idle PM
turn must not fabricate those inputs or fall back to free-chat dispatch.

Source wire compatibility and the eleven accepted Tracker metadata event types
do not prove live admission. See [metadata compatibility](TRACKER_METADATA11_COMPATIBILITY.md)
and [PM tools handoff](contracts/PM_TOOLS_HANDOFF_REQUIREMENTS.md).

## Release Rules

1. Keep one reviewed source candidate per qualification cycle. Do not repeat a
   failed unchanged run or create another controls framework to bypass it.
2. Preserve task-owned and dependency history without force push. Existing
   foundation/configuration PRs are dependencies, not permission to mutate their
   branches or publish the whole integration assembly as a narrow release.
3. Qualify API generation, backend/PG and frontend/browser checks against the
   final relevant inputs. A source review, pure check or fixture is not native
   execution acceptance. Platform skips remain explicit.
4. Run the real PM vertical: Draft -> questions -> owner answers -> final full
   revision -> owner exact-revision/hash confirmation -> Tracker Backlog.
   Restarts, stale/foreign inputs and unavailable dependencies cannot produce
   duplicate work or false success.
5. Complete the broader seven-agent delivery/integration scenario separately.
   A successful PM vertical does not complete that obligation. Existing tasks
   stay unenrolled; opt-in disablement stops new assignments and keeps history.

## Evidence And History

[CURRENT_STATE](CURRENT_STATE.md) records current source/check evidence;
[GAP_REGISTER](GAP_REGISTER.md) defines open release gates. The approved
[clarification plan](CHAT_CLARIFICATION_IMPLEMENTATION_PLAN.md),
[contract](contracts/CHAT_CLARIFICATION_CONTRACT.md) and
[full SDLC plan](SDLC_IMPLEMENTATION.md) retain the product requirements.

Superseded qualification tables are retained in
[state history](CURRENT_STATE_HISTORY_2026-10-10.md),
[gap history](GAP_REGISTER_HISTORY_2026-10-10.md) and the
[previous delivery ledger](https://github.com/FerrPOINT/fleet-control/blob/3c900b00f15aeda2016a45d080d850fa028cdcfd/docs/REMAINING_DELIVERY_WORK.md).
They are not repeated here or upgraded to acceptance of a newer source.
