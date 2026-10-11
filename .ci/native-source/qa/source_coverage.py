"""Source qualification only; actual locked compilation remains a mandatory future gate."""
import hashlib
from packet import git

SOURCE = "c3fc175b97168736717c72c2b32e1036c5b6f9db"
REVIEWED_PRODUCTION = "aabe7885c1bc0521dc2521bb9c82fc1f19cf9bba"
FINAL_DELTA = ["backend/infra/src/runtime/pm_continuation.rs", "backend/infra/tests/support/pm_credential_creation.rs",
               "docs/CURRENT_STATE.md", "docs/GAP_REGISTER.md", "docs/REMAINING_DELIVERY_WORK.md",
               "docs/contracts/CHAT_CLARIFICATION_CONTRACT.md"]
OLD = "b249bc895e5160fe13383c49d42a24c9308852b3"
CHECKS = {
    "backend/domain/src/lib.rs": ["pub task_bound: Option<bool>", "pub request_payload_hash: Option<String>"],
    "backend/infra/src/lib.rs": ["async fn list_session_messages(&self, id: Uuid)",
                                 "task-bound messages require a verified workflow assignment",
                                 "message.request_payload_hash = request_payload_hash;"],
    "backend/app/src/sdlc_workflow.rs": ["pub async fn verify_revision_binding(",
                                        '.get("fleet_sdlc_package")', "return Ok(());"],
    "backend/infra/src/runtime/container_activation.rs": ["async fn activation_configuration_binding(",
                                                           "activation_binding_target(revision, Some(&record))?"],
    "backend/shared/src/config.rs": ["pub struct PmDispatchConfig", "pub configuration_readback_enabled: bool"],
    "backend/migration/src/lib.rs": ["m20261011_000026"],
    "backend/infra/src/runtime/pm_continuation.rs": ['return Err(AppError::conflict("PM continuation custody changed"));'],
}


def qualify(repo, source):
    if source != SOURCE:
        raise ValueError("Exact audited current native source required")
    changed = git(repo, "diff", "--name-only", REVIEWED_PRODUCTION, source).decode().splitlines()
    if changed != FINAL_DELTA:
        raise ValueError("Final source must match the exact reviewed source delta")
    result = {}
    for path, markers in CHECKS.items():
        raw = git(repo, "show", source + ":" + path)
        text = raw.decode()
        if any(marker not in text for marker in markers):
            raise ValueError("Audited current contract changed")
        result[path] = hashlib.sha256(raw).hexdigest()
    # Explicitly retained free-chat path, not a PM fixture or a fabricated Workflow binding.
    return dict(source=source, historical_source=OLD, reviewed_production=REVIEWED_PRODUCTION,
                final_delta=changed, production_byte_parity=False, audited_files=result,
                qualification="source_only_actual_compile_and_native_pending",
                added_runtime_assertions=["task_bound_false", "exact_request_payload_hash"],
                pm_acceptance=False, workflow_binding_acceptance=False)
