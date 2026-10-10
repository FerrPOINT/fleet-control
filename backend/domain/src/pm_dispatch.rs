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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PmContinuationOutcome {
    NotRequired,
    Pending,
    Confirmed,
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
    if identity.assignment_revision != 1
        || owner_version != 1
        || identity.root_ref != identity.task_ref
        || identity.assignment_operation_key != format!("pm-draft:{}", identity.assignment_ref)
        || identity
            .task
            .strip_prefix("SDLC-")
            .and_then(|ordinal| ordinal.parse::<i64>().ok())
            .is_none()
        || input.snapshot_ref.is_nil()
        || input.sha256.len() != 64
        || !input
            .sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(stale());
    }
    // Workflow owns role/mode and leaves later-stage work/queue/workspace refs null.
    let mut request = serde_json::to_value(identity).map_err(AppError::internal)?;
    request.as_object_mut().ok_or_else(stale)?.extend(
        json!({"owner_version":owner_version,"input_snapshot_ref":input.snapshot_ref,
            "input_sha256":input.sha256,"runtime_compatibility":compatibility})
        .as_object()
        .ok_or_else(stale)?
        .clone(),
    );
    Ok(request)
}

fn stale() -> AppError {
    AppError::conflict("PM dispatch identity is stale or incomplete")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn continuation_outcomes_are_explicit_and_closed() {
        for (outcome, wire) in [
            (PmContinuationOutcome::NotRequired, "not_required"),
            (PmContinuationOutcome::Pending, "pending"),
            (PmContinuationOutcome::Confirmed, "confirmed"),
        ] {
            assert_eq!(serde_json::to_value(outcome).unwrap(), json!(wire));
            assert_eq!(
                serde_json::from_value::<PmContinuationOutcome>(json!(wire)).unwrap(),
                outcome
            );
        }
        assert!(serde_json::from_value::<PmContinuationOutcome>(json!("delivered")).is_err());
    }

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
        let expected: Value = serde_json::from_str(include_str!(
            "../../infra/tests/fixtures/pm-dispatch/workflow-assign-request.json"
        ))
        .unwrap();
        let value =
            initial_pm_assignment(&operation, expected["runtime_compatibility"].clone()).unwrap();
        assert_eq!(value, expected);
        assert_eq!(value["owner_version"], 1);
        assert_eq!(value["assignment_revision"], 1);
        assert_eq!(
            value["execution_ref"],
            operation.execution_identity().unwrap().execution_ref
        );
        assert_eq!(
            value["input_snapshot_ref"],
            json!(operation.input.as_ref().unwrap().input.snapshot_ref)
        );
        assert_eq!(
            value["input_sha256"],
            operation.input.as_ref().unwrap().input.sha256
        );
        for name in [
            "assignment_shape",
            "operation_key",
            "expected_revision",
            "expected_status",
            "role_key",
            "workflow_key",
            "mode_key",
            "stage_key",
            "work_item_ref",
            "work_item_revision",
            "queue_item_ref",
            "task_workspace_ref",
            "decomposition_revision_ref",
            "tech_execution_workspace_ref",
            "workspace_generation",
            "lease_generation",
        ] {
            assert!(value.get(name).is_none());
        }
    }

    #[test]
    fn draft_assignment_rejects_noninitial_versions_keys_and_input() {
        let operation = operation();
        for field in [
            "assignment_version",
            "owner_version",
            "key",
            "ordinal",
            "snapshot",
            "hash",
        ] {
            let mut wrong = operation.clone();
            match field {
                "assignment_version" => wrong.reservation.as_mut().unwrap().assignment.version = 2,
                "owner_version" => wrong.reservation.as_mut().unwrap().owner_cas.version = 2,
                "key" => {
                    wrong.reservation.as_mut().unwrap().assignment_operation_key = "foreign".into()
                }
                "ordinal" => {
                    wrong.reservation.as_mut().unwrap().execution.key =
                        "SDLC-9223372036854775808".into()
                }
                "snapshot" => wrong.input.as_mut().unwrap().input.snapshot_ref = Uuid::nil(),
                "hash" => wrong.input.as_mut().unwrap().input.sha256 = "G".repeat(64),
                _ => unreachable!(),
            }
            assert!(
                initial_pm_assignment(&wrong, Value::Null).is_err(),
                "{field}"
            );
        }
    }
}
