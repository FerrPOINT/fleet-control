//! Server-owned PM projection into Workflow's initial Draft assignment.
use crate::{PmDraftOperation, PmExecutionIdentity, pm_canonical_hash};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmNativeStepCommand {
    pub operation_key: String,
    pub report: Option<String>,
    pub expected_phase_code: String,
    pub expected_status: String,
}

pub fn native_step_command(
    record: &crate::PmRunRecord,
    command: &Value,
) -> Result<Value, AppError> {
    let input: PmNativeStepCommand = serde_json::from_value(command.clone())
        .map_err(|_| AppError::validation("closed PM step command required"))?;
    if serde_json::to_value(&input).map_err(AppError::internal)? != *command
        || !crate::pm_draft::valid_key(&input.operation_key)
        || !input.operation_key.is_ascii()
        || !crate::valid_ref(&input.expected_phase_code, 128)
        || !matches!(input.expected_status.as_str(), "active" | "blocked")
        || input
            .report
            .as_ref()
            .is_some_and(|v| v.trim().is_empty() || v.len() > 100_000 || v.contains('\0'))
    {
        return Err(AppError::validation("invalid PM report or phase cursor"));
    }
    record.reservation.validate()?;
    if record.terminal_status.is_some() {
        return Err(AppError::conflict("PM run is terminal"));
    }
    let reservation = &record.reservation;
    let native = record
        .hermes_run_ref
        .as_ref()
        .ok_or_else(|| AppError::conflict("PM acceptance missing"))?;
    Ok(
        json!({"task":reservation.identity.task,"report":input.report,
        "step_operation_key":input.operation_key,"assignment_revision":reservation.identity.assignment_revision,
        "assignment_ref":reservation.identity.assignment_ref,"binding_ref":reservation.binding_ref,
        "hermes_run_ref":native,"mode_key":"draft","cycle_number":0,"attempt_number":1,
        "expected_phase_code":input.expected_phase_code,"expected_status":input.expected_status,
        "session_run_id":reservation.session_run_id}),
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PmWorkflowCompatibility {
    pub catalog_version: u8,
    pub catalog_revision: String,
    pub catalog_sha256: String,
    pub skills_revision: String,
    pub skills_manifest_sha256: String,
    pub capability_revision: String,
    pub capability_sha256: String,
}

impl PmWorkflowCompatibility {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.catalog_version != 2
            || self.capability_revision != "hermes-sdlc-runtime/v2"
            || !hex_value(&self.catalog_revision, 40)
            || !hex_value(&self.skills_revision, 40)
            || [
                &self.catalog_sha256,
                &self.skills_manifest_sha256,
                &self.capability_sha256,
            ]
            .iter()
            .any(|v| !hex_value(v, 64))
        {
            return Err(inconsistent());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmWorkflowAssignmentIntent {
    pub command: Value,
    pub request_sha256: String,
    pub workflow_origin: String,
    pub credential_fingerprint: String,
}

impl PmWorkflowAssignmentIntent {
    pub fn verify(&self, operation: &PmDraftOperation) -> Result<(), AppError> {
        let compatibility: PmWorkflowCompatibility = serde_json::from_value(
            self.command
                .get("runtime_compatibility")
                .cloned()
                .ok_or_else(inconsistent)?,
        )
        .map_err(|_| inconsistent())?;
        compatibility.validate()?;
        if self.command != assignment_command(operation, &compatibility)?
            || self.request_sha256 != pm_canonical_hash(&self.command)
            || !hex_value(&self.credential_fingerprint, 64)
            || !crate::valid_ref(&self.workflow_origin, 1024)
        {
            return Err(inconsistent());
        }
        Ok(())
    }
}

pub fn assignment_command(
    op: &PmDraftOperation,
    compatibility: &PmWorkflowCompatibility,
) -> Result<Value, AppError> {
    compatibility.validate()?;
    let identity = op.execution_identity()?;
    let reservation = op.reservation.as_ref().ok_or_else(inconsistent)?;
    let input = &op.input.as_ref().ok_or_else(inconsistent)?.input;
    let journal = op.execution_lease.as_ref().ok_or_else(inconsistent)?;
    journal
        .receipt
        .as_ref()
        .ok_or_else(inconsistent)?
        .verify_claim(reservation, &journal.claim)?;
    if op.session_id.is_none()
        || identity.assignment_revision != 1
        || reservation.owner_cas.version != 1
        || identity.root_ref != identity.task_ref
        || identity.assignment_operation_key
            != format!("pm-draft:{}", reservation.assignment.assignment_id)
        || input.snapshot_ref.is_nil()
        || !hex_value(&input.sha256, 64)
        || input.snapshot_ref != reservation.input.snapshot_ref
        || input.sha256 != reservation.input.sha256
    {
        return Err(inconsistent());
    }
    let mut value = serde_json::to_value(identity).map_err(AppError::internal)?;
    let object = value.as_object_mut().ok_or_else(inconsistent)?;
    object.extend([
        ("owner_version".into(), json!(reservation.owner_cas.version)),
        ("input_snapshot_ref".into(), json!(input.snapshot_ref)),
        ("input_sha256".into(), json!(input.sha256)),
        ("runtime_compatibility".into(), json!(compatibility)),
    ]);
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmWorkflowAssignmentReceipt {
    pub workflow_id: i64,
    pub mode_id: i64,
    pub phase_id: i64,
    pub phase_code: String,
}

impl PmWorkflowAssignmentReceipt {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.workflow_id <= 0
            || self.mode_id <= 0
            || self.phase_id <= 0
            || self.phase_code != "PM-DRAFT-01"
        {
            return Err(inconsistent());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmWorkflowAssignmentJournal {
    pub intent: PmWorkflowAssignmentIntent,
    pub receipt: Option<PmWorkflowAssignmentReceipt>,
}

/// Exact existing Workflow response, including explicit nulls; no model/readiness claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmWorkflowAssignedDraft {
    pub task_key: String,
    pub workflow_id: i64,
    pub workflow_key: String,
    pub mode_id: i64,
    pub mode_key: String,
    pub cycle_number: i64,
    pub attempt_number: i64,
    pub assignment_operation_key: String,
    pub assignment_revision: i64,
    pub role_key: String,
    pub execution_scope: String,
    pub stage_key: String,
    pub business_task_ref: String,
    pub root_task_ref: String,
    pub work_item_ref: Option<String>,
    pub work_item_revision: Option<i64>,
    pub queue_item_ref: Option<String>,
    pub task_workspace_ref: Option<String>,
    pub workspace_revision: Option<i64>,
    pub tech_execution_workspace_ref: Option<String>,
    pub tech_execution_attempt_ref: Option<String>,
    pub decomposition_revision_ref: Option<String>,
    pub stage_revision: String,
    pub assignment_ref: String,
    pub binding_ref: Option<String>,
    pub hermes_run_ref: Option<String>,
    pub bind_operation_key: Option<String>,
    pub concrete_agent_ref: Option<String>,
    pub workspace_generation: Option<i64>,
    pub lease_generation: i64,
    pub exact_input_refs: Vec<Value>,
    pub binding_state: String,
    pub status: String,
    pub current_phase_id: i64,
    pub current_phase_code: String,
    pub current_phase_name: String,
}

impl PmWorkflowAssignedDraft {
    pub fn verified(
        value: Value,
        intent: &PmWorkflowAssignmentIntent,
    ) -> Result<PmWorkflowAssignmentReceipt, AppError> {
        let result: Self = serde_json::from_value(value.clone()).map_err(|_| inconsistent())?;
        if serde_json::to_value(&result).map_err(AppError::internal)? != value {
            return Err(inconsistent());
        }
        let command = &intent.command;
        let identity: PmExecutionIdentity = serde_json::from_value(json!({
            "task":command["task"], "execution_ref":command["execution_ref"],
            "tracker_instance_ref":command["tracker_instance_ref"], "tracker_project_ref":command["tracker_project_ref"],
            "task_ref":command["task_ref"], "root_ref":command["root_ref"], "agent_ref":command["agent_ref"],
            "assignment_operation_key":command["assignment_operation_key"], "assignment_ref":command["assignment_ref"],
            "assignment_revision":command["assignment_revision"],
        })).map_err(|_| inconsistent())?;
        identity.validate()?;
        let bound = match result.binding_state.as_str() {
            "unbound" => {
                result.binding_ref.is_none()
                    && result.hermes_run_ref.is_none()
                    && result.bind_operation_key.is_none()
                    && result.concrete_agent_ref.is_none()
            }
            "bound" => {
                result.concrete_agent_ref.as_deref() == Some(identity.agent_ref.as_str())
                    && [
                        &result.binding_ref,
                        &result.hermes_run_ref,
                        &result.bind_operation_key,
                    ]
                    .iter()
                    .all(|v| v.as_ref().is_some_and(|v| crate::valid_ref(v, 512)))
            }
            _ => false,
        };
        if result.task_key != identity.task
            || result.assignment_operation_key != identity.assignment_operation_key
            || result.assignment_ref != identity.assignment_ref
            || result.assignment_revision != identity.assignment_revision
            || result.business_task_ref != identity.task_ref
            || result.root_task_ref != identity.root_ref
            || result.workflow_key != "hermes-sdlc:project_manager"
            || result.role_key != "project_manager"
            || result.mode_key != "draft"
            || result.execution_scope != "business"
            || result.stage_key != "draft"
            || result.cycle_number != 0
            || result.attempt_number != 1
            || result.stage_revision != "1"
            || result.lease_generation != 1
            || result.status != "active"
            || !bound
            || result.work_item_revision.is_some()
            || result.workspace_revision.is_some()
            || result.workspace_generation.is_some()
            || [
                &result.work_item_ref,
                &result.queue_item_ref,
                &result.task_workspace_ref,
                &result.tech_execution_workspace_ref,
                &result.tech_execution_attempt_ref,
                &result.decomposition_revision_ref,
            ]
            .iter()
            .any(|v| v.is_some())
            || result.exact_input_refs
                != vec![
                    json!({"kind":"pm_draft_input","ref":command["input_snapshot_ref"],"hash":command["input_sha256"]}),
                ]
        {
            return Err(inconsistent());
        }
        let receipt = PmWorkflowAssignmentReceipt {
            workflow_id: result.workflow_id,
            mode_id: result.mode_id,
            phase_id: result.current_phase_id,
            phase_code: result.current_phase_code,
        };
        receipt.validate()?;
        Ok(receipt)
    }
}

fn hex_value(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
}

fn inconsistent() -> AppError {
    AppError::conflict("PM Workflow assignment evidence is inconsistent")
}

#[cfg(test)]
#[path = "pm_workflow_tests.rs"]
mod tests;
