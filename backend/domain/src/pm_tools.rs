use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::AppError;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmToolCall {
    pub operation_id: Uuid,
    pub session_run_id: Uuid,
    pub command: Value,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmMachineFence {
    pub assignment_id: Uuid,
    pub execution_id: Uuid,
    pub agent_id: Uuid,
    pub assignment_version: i64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmPublishRevision {
    pub fence: PmMachineFence,
    pub expected_requirement_revision: Option<i64>,
    pub document: PmRequirementsDocument,
    pub idempotency_key: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmRequirementsDocument {
    pub goal: String,
    pub scope: Vec<String>,
    pub exclusions: Vec<String>,
    pub scenarios: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub constraints: Vec<String>,
    pub dependencies: Vec<String>,
    pub assumptions: Vec<String>,
    pub checklist: Vec<String>,
    pub prerequisites: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmPublishQuestion {
    pub fence: PmMachineFence,
    pub request_id: Uuid,
    pub question_id: Uuid,
    pub expected_question_version: Option<i64>,
    pub requirement_revision: i64,
    pub checkpoint_id: Uuid,
    pub requirement_reference: Option<String>,
    pub text: String,
    pub rationale: String,
    pub required: bool,
    pub mode: TrackerQuestionMode,
    pub options: Vec<TrackerQuestionOption>,
    pub recommended_option_id: Option<Uuid>,
    pub idempotency_key: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmWorkflowStep {
    pub step_operation_key: String,
    pub report: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmCheckpoint {
    pub checkpoint_ref: String,
    pub clarification_request_ref: String,
    pub clarification_version: i64,
    pub requirements_revision: i64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmWorkflowSnapshot {
    pub contract_version: u8,
    pub identity: PmExecutionIdentity,
    pub state: String,
    pub version: i64,
    pub fence: i64,
    pub session_run_id: Uuid,
    pub binding_ref: String,
    pub hermes_run_ref: String,
    pub checkpoint: Option<PmCheckpoint>,
    pub resume_operation_key: Option<String>,
    pub resume_session_run_id: Option<Uuid>,
    pub terminal_readback: Option<PmRuntimeObservation>,
    pub workflow_step_allowed: bool,
    pub resume_delivered: bool,
}

pub fn verify_pm_machine_context(
    operation: &PmDraftOperation,
    context: &TrackerTaskContext,
) -> Result<(), AppError> {
    let binding = operation.identity()?;
    let expected = &operation
        .reservation
        .as_ref()
        .ok_or(AppError::Forbidden)?
        .assignment;
    let current = context.assignment.as_ref().ok_or(AppError::Forbidden)?;
    if context.contract_version != 1
        || context.tracker_instance_id != binding.tracker_instance_id
        || context.project_id != binding.project_id
        || context.task_id != binding.task_id
        || context.root_task_id != binding.root_task_id
        || context.owner_subject != binding.owner_subject
        || current.assignment_id != expected.assignment_id
        || current.execution_id != expected.execution_id
        || current.agent_id != expected.agent_id
        || u64::try_from(current.version).ok() != Some(expected.version)
        || current.machine_subject != expected.machine_subject
        || context.permissions.can_answer
        || context.permissions.can_confirm
        || !matches!(
            context.stage,
            TrackerStage::Draft | TrackerStage::Clarification
        )
    {
        return Err(AppError::conflict(
            "PM machine assignment is stale or outside Draft",
        ));
    }
    Ok(())
}

impl PmMachineFence {
    pub fn verify(&self, operation: &PmDraftOperation) -> Result<(), AppError> {
        let assignment = &operation
            .reservation
            .as_ref()
            .ok_or(AppError::Forbidden)?
            .assignment;
        if self.assignment_id != assignment.assignment_id
            || self.execution_id != assignment.execution_id
            || self.agent_id != assignment.agent_id
            || u64::try_from(self.assignment_version).ok() != Some(assignment.version)
        {
            return Err(AppError::Forbidden);
        }
        Ok(())
    }
}

pub fn pm_tool_key(key: &str) -> Result<(), AppError> {
    if !crate::pm_draft::valid_key(key) {
        return Err(AppError::validation("invalid PM tool operation key"));
    }
    Ok(())
}

/// Private durable command custody; never contains credentials or execution tokens.
#[derive(Clone, Serialize, Deserialize)]
pub struct PmToolCommand {
    pub session_run_id: Uuid,
    pub key: String,
    pub kind: String,
    pub request: Value,
    pub result: Option<Value>,
    pub attempted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn tool_target_is_closed_and_does_not_accept_credential_or_fence_overrides() {
        let value =
            json!({"operation_id":Uuid::new_v4(),"session_run_id":Uuid::new_v4(),"command":{}});
        assert!(serde_json::from_value::<PmToolCall>(value.clone()).is_ok());
        for field in ["task_id", "agent_id", "token", "execution_token", "url"] {
            let mut bad = value.clone();
            bad[field] = json!("override");
            assert!(serde_json::from_value::<PmToolCall>(bad).is_err());
        }
        assert!(serde_json::from_value::<PmWorkflowStep>(json!({"step_operation_key":"original","report":null,"expected_phase_code":"PM-DRAFT-99"})).is_err());
    }
    #[test]
    fn tool_keys_preserve_original_bounded_native_custody() {
        assert!(pm_tool_key("original:question-1").is_ok());
        for key in ["", "two words", "bad\nkey", &"x".repeat(129)] {
            assert!(pm_tool_key(key).is_err());
        }
    }
}
