"""PROPOSED contract oracle only: no server, credential issuer or durable storage.

Authenticated peer, binding, clock and producer proofs are fixture inputs. Passing
this oracle does not establish those inputs in either pinned product.
"""

from dataclasses import dataclass
import hashlib
import json
import threading


class Held(Exception):
    pass


@dataclass(frozen=True)
class Binding:
    peer: str
    native_run: str
    conversation: str
    fleet_run: str
    task: str
    root: str
    execution: str
    assignment: str
    revision: int
    checkpoint: str
    fence: int
    expires: int


@dataclass(frozen=True)
class ProducerProof:
    # These are required semantics, NOT capabilities advertised by bbaf7af.
    pre_model_veto: bool = False
    explicit_native_identity: bool = False
    server_credential_custody: bool = False


def explicit_context(context):
    """Inspect bound vars only. Legacy env-fallback getters are not authority.

    Test-only pinned-source adapter, not a supported producer identity API.
    Even a positive result is merely a selector requiring authenticated binding.
    """
    values = {var.name: value for var, value in context.items()}
    run = values.get("approval_session_key")
    if not isinstance(run, str) or not run.startswith("run_"):
        raise Held("missing explicit native run identity")
    if values.get("HERMES_SESSION_KEY") != run:
        raise Held("conflicting native run context")
    conversation = values.get("HERMES_SESSION_ID")
    if not isinstance(conversation, str) or not conversation:
        raise Held("missing conversation identity")
    return run, conversation


class Oracle:
    READS = frozenset({"context", "clarifications", "requirements"})
    WRITES = frozenset({"clarification_publish", "clarification_cancel", "requirements_publish"})

    def __init__(self):
        self.bindings = {}
        self.current = {}
        self.journal = {}
        self.lock = threading.RLock()

    def bind_fixture(self, binding):
        """Synthetic coordinator fixture, never a model-facing operation."""
        with self.lock:
            if binding.native_run in self.bindings:
                raise Held("immutable run binding")
            self.bindings[binding.native_run] = binding
            self.current[binding.task] = binding

    def authorize(self, peer, run, conversation, now):
        binding = self.bindings.get(run)
        if binding is None or binding.peer != peer or binding.conversation != conversation:
            raise Held("unauthenticated or unmapped exact run")
        if self.current.get(binding.task) != binding or now >= binding.expires:
            raise Held("expired or stale assignment/fence")
        return binding

    def admit(self, peer, run, conversation, now, proof, first_model):
        with self.lock:
            binding = self.authorize(peer, run, conversation, now)
            if not all((proof.pre_model_veto, proof.explicit_native_identity,
                        proof.server_credential_custody)):
                raise Held("producer admission primitives unavailable")
            if any(r["state"] == "unknown" and r["binding"].task == binding.task
                   for r in self.journal.values()):
                raise Held("task write acceptance unresolved")
            # Real implementation requires a producer-side atomic barrier/fence.
            return first_model(binding)

    def call(self, peer, run, conversation, now, call_id, tool, payload, send):
        with self.lock:
            binding = self.authorize(peer, run, conversation, now)
            if tool not in self.READS | self.WRITES:
                raise Held("tool outside PM draft scopes")
            if not call_id or not isinstance(payload, dict):
                raise Held("missing native tool call identity or content")
            if set(payload) != {"content"} or not isinstance(payload["content"], str):
                raise Held("model may supply content only, not authority")
            digest = hashlib.sha256(json.dumps(
                [tool, payload], sort_keys=True, ensure_ascii=False,
                separators=(",", ":"), allow_nan=False,
            ).encode("utf-8")).hexdigest()
            if tool in self.READS:
                return send(binding, payload)
            key = (run, call_id)
            previous = self.journal.get(key)
            if previous:
                if previous["digest"] != digest:
                    raise Held("immutable tool-call payload")
                return previous["state"]
            if any(r["state"] == "unknown" and r["binding"].task == binding.task
                   for r in self.journal.values()):
                raise Held("unresolved task write prohibits a new send permit")
            # A real implementation MUST commit this before obtaining one send permit.
            self.journal[key] = {"digest": digest, "state": "unknown", "binding": binding}
        try:
            result = send(binding, payload)
        except Exception:
            return "unknown"
        with self.lock:
            if result in ("accepted", "rejected"):
                if self.journal[key]["state"] not in ("unknown", result):
                    raise Held("contradictory terminal send/readback receipt")
                self.journal[key]["state"] = result
            return self.journal[key]["state"]

    def readback(self, peer, run, conversation, now, call_id, digest, binding, state):
        """Synthetic authenticated exact readback; there is deliberately no resend API."""
        with self.lock:
            self.authorize(peer, run, conversation, now)
            record = self.journal[(run, call_id)]
            if record["digest"] != digest or record["binding"] != binding:
                raise Held("contradictory write readback")
            if state not in ("accepted", "rejected"):
                raise Held("write acceptance remains unknown")
            if record["state"] not in ("unknown", state):
                raise Held("contradictory terminal write receipt")
            record["state"] = state
