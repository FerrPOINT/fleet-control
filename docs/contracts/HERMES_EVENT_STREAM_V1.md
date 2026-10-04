# Hermes Event Stream Consumer Profile v1

Status: Fleet source implementation; release and native acceptance are recorded
separately in the verification ledger. This is a bounded consumer policy, not a
new Hermes extension/capability, external event cursor or task admission receipt.

## Transport And Framing

Fleet requests the authenticated original run's `/v1/runs/{run_id}/events` with
`Accept: text/event-stream` and `Accept-Encoding: identity`. Only HTTP200 and
`text/event-stream` are accepted; compressed, missing-MIME and other responses
fail without inspecting or exposing their bodies. Header wait remains10s.

Framing follows the newline, BOM, comment and field rules in the
[HTML event-stream standard](https://html.spec.whatwg.org/multipage/server-sent-events.html#event-stream-interpretation):
LF, CRLF and bare CR delimit lines; the initial BOM is ignored; one optional
space after a field colon is removed; multiple data fields join with LF.
An empty frame resets its event name. A blank line, not EOF, dispatches data.
The Fleet profile additionally refuses malformed UTF-8 and non-object/malformed
JSON instead of creating replacement text or interpreting prose as an event.

The accepted run ID is authoritative. Every data event must contain that exact
string `run_id`; missing, non-string and foreign IDs are refused, including
delta, tool and approval events. Any supplied session identity must match the
original pinned session. Conflicting event field
and JSON event names are refused before delta/tool/approval writes. Unknown
well-formed event types do not become terminal receipts. Exact terminal names,
identity and native success flags remain required by the adapter contract.

## Resource Policy

| Resource | Bound |
| --- | --- |
| One assembled frame, including comments/fields | 1 MiB |
| Total received stream bytes | 32 MiB |
| Dispatched data frames | 8192 |
| Accumulated assistant text | 1 MiB UTF-8 bytes |
| Total persisted full-text delta snapshot text | 16 MiB |
| No incoming bytes | 60 seconds |
| Incomplete frame assembly | 30 seconds, not extended by traffic |
| Stream worker lifetime | 30 minutes, not extended by heartbeat |

Limits belong to this consumer, not the Hermes run's execution timeout or the
Tracker assignment lease. Known-bound counters are checked before growing the
buffer or writing the next snapshot. The worker never creates an unbounded list
of decoded events for a chunk. No response-level decompression or truncation is
used to fit a limit. Snapshot accounting measures text bytes, not DB metadata or
JSON escaping overhead; it is not a database-size/per-user quota guarantee.
CRLF line endings consume both bytes while a frame is pending. The optional LF
after a blank CR has dispatched the frame is ignored as part of that delimiter,
not charged to the next frame; the stream total still counts every received byte.
Empty transport chunks do not extend the idle deadline.

## Failure And Recovery

Transport, framing, identity and budget failures retain the original accepted
run, message, session pin and occupied capacity. Diagnostics do not echo payload
content. Delivery remains dispatched with a redacted error and run waiting;
late error writes cannot reopen a separately committed terminal result.
No failure, negative lookup, expired cursor or EOF authorizes another POST.

An incomplete EOF frame is discarded; authenticated bounded status readback
must independently prove the original run/session terminal state. The existing
GET-only recovery worker can later commit that verified result exactly once.
It does not reconnect a second native SSE consumer. Native missed tool/approval
history, control outcome reconciliation and durable upstream event replay remain
separate requirements; budget retirement cannot certify them or safe run stop.
Do not treat this retirement as cancellation, stage completion or permission to
activate config/replace an agent. Long-running or approval-waiting runs remain
held until an independent result or confirmed safe stop is obtained.

## Verification

Source unit tests cover incremental UTF-8/BOM/newlines, empty frames, EOF,
malformed bytes, frame/counter/text/snapshot bounds and nonextendable deadlines.
PostgreSQL/authenticated HTTP tests exercise real30s/60s deadlines, original
identity/capacity hold, truncated terminal frames, Unicode chunk boundaries,
wrong headers, malformed/foreign events and oversized text. Tests without an
owned test DB are not PostgreSQL acceptance. The opt-in managed native supervisor
gate remains an independent compatibility check with a local model fixture.

No schema, public Fleet DTO/route, Java capability, SDK pin or installed runtime
is changed by this profile. See [Hermes adapter](HERMES_ADAPTER_CONTRACT.md),
[testing](../TESTING.md) and [verification](../CHAT_CLARIFICATION_VERIFICATION.md).
