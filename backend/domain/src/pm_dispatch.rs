use crate::{PmDraftOperation, PmRunReservation, TrackerStage, TrackerTaskContext};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::AppError;
use uuid::Uuid;

/// Private dispatch custody. Bearers and Workflow execution tokens never enter this record.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmDispatchIntent {
    pub session_run_id: Uuid,
    pub origin: String,
    pub credential_fingerprint: String,
    pub request_body: String,
    pub workflow_assignment: Value,
    pub workflow_origin: String,
    pub workflow_credential_fingerprint: String,
    pub runtime_context: Value,
    pub submitted: bool,
    pub hermes_run_ref: Option<String>,
}

pub enum PmGuidancePermit {
    Claimed,
    Delivered,
    Unknown,
}

pub fn initial_pm_reservation(operation: &PmDraftOperation) -> Result<PmRunReservation, AppError> {
    let reservation = PmRunReservation {
        session_id: operation.session_id.ok_or_else(stale)?,
        // The creation operation is durable; retries must not allocate another run UUID/key.
        session_run_id: operation.id,
        identity: operation.execution_identity()?,
        binding_ref: format!("fleet:pm:{}", operation.id),
        dispatch_operation_key: format!("fleet-pm-bind:{}", operation.id),
        checkpoint_ref: None,
        fence: 1,
    };
    reservation.validate()?;
    Ok(reservation)
}

pub fn verify_initial_pm_context(
    operation: &PmDraftOperation,
    context: &TrackerTaskContext,
) -> Result<(), AppError> {
    let binding = operation.identity()?;
    let expected = operation.reservation.as_ref().ok_or_else(stale)?;
    let current = context.assignment.as_ref().ok_or_else(stale)?;
    if context.contract_version != 1
        || context.tracker_instance_id != binding.tracker_instance_id
        || context.project_id != binding.project_id
        || context.task_id != binding.task_id
        || context.root_task_id != binding.root_task_id
        || context.owner_subject != binding.owner_subject
        || current.assignment_id != expected.assignment.assignment_id
        || current.execution_id != expected.assignment.execution_id
        || current.agent_id != operation.request.agent_id
        || u64::try_from(current.version).ok() != Some(expected.assignment.version)
        || current.machine_subject != expected.assignment.machine_subject
    {
        return Err(stale());
    }
    // This is initial Draft dispatch, not an answer/resume, requirements confirmation or Analysis.
    if !matches!(context.stage, TrackerStage::Draft)
        || context.requirement_revision.is_some()
        || context.waiting_reason.is_some()
    {
        return Err(AppError::conflict(
            "initial PM dispatch requires an unblocked Draft",
        ));
    }
    Ok(())
}

pub fn initial_pm_assignment(
    operation: &PmDraftOperation,
    compatibility: Value,
) -> Result<Value, AppError> {
    let identity = operation.execution_identity()?;
    let input = &operation.input.as_ref().ok_or_else(stale)?.input;
    let owner_version = operation
        .reservation
        .as_ref()
        .ok_or_else(stale)?
        .owner_cas
        .version;
    // Workflow's business-pre-decomposition profile has no code workspace/decomposition.
    // Its opaque work/queue refs map to the actual business task and persisted PM assignment.
    Ok(json!({
        "assignment_shape":"business-pre-decomposition", "task":identity.task,
        "role_key":"project_manager", "workflow_key":"hermes-sdlc:project_manager",
        "mode_key":"draft", "execution_scope":"business", "stage_key":"Draft",
        "cycle_number":0, "attempt_number":1, "operation_key":identity.assignment_operation_key,
        "business_task_ref":identity.task_ref, "root_task_ref":identity.root_ref,
        "work_item_ref":identity.task_ref, "work_item_revision":0,
        "queue_item_ref":identity.assignment_ref, "stage_revision":owner_version.to_string(),
        "assignment_ref":identity.assignment_ref,
        "exact_input_refs":[{"kind":"original_input", "ref":input.snapshot_ref, "hash":input.sha256}],
        "runtime_compatibility":compatibility, "expected_revision":0, "expected_status":"missing"
    }))
}

fn stale() -> AppError {
    AppError::conflict("PM dispatch identity is stale or incomplete")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn operation() -> PmDraftOperation {
        let mut reservation: TrackerPmDraftReservation = serde_json::from_str(include_str!(
            "../../api/tests/fixtures/pm-draft/reservation.json"
        ))
        .unwrap();
        let binding = reservation.binding.clone();
        let request = CreatePmDraftRequest {
            agent_id: reservation.assignment.agent_id,
            title: "Owner goal".into(),
            description: "Exact original input".into(),
            idempotency_key: "owner-request".into(),
        };
        reservation.input.sha256 = pm_input_hash(&request.title, &request.description);
        PmDraftOperation {
            id: Uuid::from_u128(1),
            owner_user_id: Uuid::from_u128(2),
            owner_subject: binding.owner_subject.clone(),
            tracker_instance_id: binding.tracker_instance_id.clone(),
            project_id: binding.project_id,
            draft: Some(TrackerCreatedDraft {
                tracker_instance_id: binding.tracker_instance_id.clone(),
                project_id: binding.project_id,
                task_id: binding.task_id,
                root_task_id: binding.root_task_id,
                task_key: "PM-1".into(),
                owner_subject: binding.owner_subject.clone(),
                stage: "Draft".into(),
            }),
            input: Some(TrackerDraftInputReceipt {
                contract_version: 1,
                tracker_instance_id: binding.tracker_instance_id,
                project_id: binding.project_id,
                task_id: binding.task_id,
                root_task_id: binding.root_task_id,
                owner_subject: binding.owner_subject,
                input: TrackerDraftInput {
                    snapshot_ref: reservation.input.snapshot_ref,
                    title: request.title.clone(),
                    description: request.description.clone(),
                    sha256: reservation.input.sha256.clone(),
                },
            }),
            request,
            reservation: Some(reservation),
            session_id: Some(Uuid::from_u128(3)),
            credentials: None,
        }
    }

    fn context(operation: &PmDraftOperation) -> Value {
        let binding = operation.identity().unwrap();
        json!({"contract_version":1,"tracker_instance_id":binding.tracker_instance_id,"project_id":binding.project_id,
            "task_id":binding.task_id,"root_task_id":binding.root_task_id,"owner_subject":binding.owner_subject,
            "stage":"Draft","requirement_revision":null,"waiting_reason":null,
            "permissions":{"can_answer":false,"can_confirm":false},"assignment":operation.reservation.as_ref().unwrap().assignment})
    }

    #[test]
    fn initial_dispatch_reuses_the_original_operation_run_and_key() {
        let operation = operation();
        let first = initial_pm_reservation(&operation).unwrap();
        assert_eq!(first, initial_pm_reservation(&operation.clone()).unwrap());
        assert_eq!(first.session_run_id, operation.id);
        assert_eq!(first.identity, operation.execution_identity().unwrap());
        assert_eq!(first.checkpoint_ref, None);
        assert_eq!(first.fence, 1);
    }

    #[test]
    fn initial_dispatch_rejects_foreign_identity_and_current_assignment() {
        let operation = operation();
        let valid = context(&operation);
        verify_initial_pm_context(&operation, &serde_json::from_value(valid.clone()).unwrap())
            .unwrap();
        for (pointer, wrong) in [
            ("/contract_version", json!(2)),
            ("/tracker_instance_id", json!("foreign")),
            ("/project_id", json!(Uuid::from_u128(4))),
            ("/task_id", json!(Uuid::from_u128(4))),
            ("/root_task_id", json!(Uuid::from_u128(4))),
            ("/owner_subject", json!(Uuid::from_u128(4))),
            ("/assignment/assignment_id", json!(Uuid::from_u128(4))),
            ("/assignment/execution_id", json!(Uuid::from_u128(4))),
            ("/assignment/agent_id", json!(Uuid::from_u128(4))),
            ("/assignment/version", json!(2)),
            ("/assignment/machine_subject", json!("other")),
            ("/assignment", Value::Null),
        ] {
            let mut wrong_context = valid.clone();
            *wrong_context.pointer_mut(pointer).unwrap() = wrong;
            assert!(
                verify_initial_pm_context(
                    &operation,
                    &serde_json::from_value(wrong_context).unwrap()
                )
                .is_err(),
                "{pointer}"
            );
        }
    }

    #[test]
    fn initial_dispatch_does_not_bypass_clarification_or_requirements() {
        let operation = operation();
        for (pointer, wrong) in [
            ("/stage", json!("Clarification")),
            ("/stage", json!("Backlog")),
            ("/stage", json!("Analysis")),
            ("/requirement_revision", json!(1)),
            ("/waiting_reason", json!("clarification")),
        ] {
            let mut value = context(&operation);
            *value.pointer_mut(pointer).unwrap() = wrong;
            assert!(
                verify_initial_pm_context(&operation, &serde_json::from_value(value).unwrap())
                    .is_err()
            );
        }
    }

    #[test]
    fn workflow_assignment_pins_original_input_without_inventing_code_resources() {
        let operation = operation();
        let value = initial_pm_assignment(&operation, json!({"exact":"compatibility"})).unwrap();
        assert_eq!(value["assignment_shape"], "business-pre-decomposition");
        assert_eq!(value["expected_revision"], 0);
        assert_eq!(value["expected_status"], "missing");
        assert_eq!(
            value["exact_input_refs"][0]["ref"],
            json!(operation.input.as_ref().unwrap().input.snapshot_ref)
        );
        assert_eq!(
            value["exact_input_refs"][0]["hash"],
            operation.input.as_ref().unwrap().input.sha256
        );
        for name in [
            "task_workspace_ref",
            "decomposition_revision_ref",
            "tech_execution_workspace_ref",
            "workspace_generation",
            "lease_generation",
        ] {
            assert!(value.get(name).is_none());
        }
    }
}
