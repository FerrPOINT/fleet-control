use super::*;
use crate::*;
use chrono::{Duration, Utc};
use uuid::Uuid;

fn compatibility() -> PmWorkflowCompatibility {
    PmWorkflowCompatibility {
        catalog_version: 2,
        catalog_revision: "a".repeat(40),
        catalog_sha256: "b".repeat(64),
        skills_revision: "c".repeat(40),
        skills_manifest_sha256: "d".repeat(64),
        capability_revision: "hermes-sdlc-runtime/v2".into(),
        capability_sha256: "e".repeat(64),
    }
}

#[test]
fn native_step_pins_execution_and_rejects_model_supplied_authority() {
    let op = operation();
    let record = PmRunRecord {
        reservation: PmRunReservation {
            session_id: Uuid::new_v4(),
            session_run_id: Uuid::new_v4(),
            identity: op.execution_identity().unwrap(),
            binding_ref: "original-binding".into(),
            dispatch_operation_key: "original-dispatch".into(),
            checkpoint_ref: None,
            fence: 1,
            runtime_binding: None,
            native_session_key: None,
            native_message_id: None,
        },
        hermes_run_ref: Some("run_actual".into()),
        hermes_session_ref: Some(Uuid::new_v4().to_string()),
        terminal_status: None,
    };
    let command = json!({"operation_key":"report:1","report":null,
        "expected_phase_code":"PM-DRAFT-01","expected_status":"active"});
    let body = native_step_command(&record, &command).unwrap();
    assert_eq!(body["hermes_run_ref"], "run_actual");
    assert_eq!(
        body["assignment_ref"],
        record.reservation.identity.assignment_ref
    );
    assert_eq!(
        body["session_run_id"],
        json!(record.reservation.session_run_id)
    );
    for field in [
        "assignment_ref",
        "hermes_run_ref",
        "session_run_id",
        "execution_token",
        "mode_key",
    ] {
        let mut extra = command.clone();
        extra[field] = json!("forged");
        assert!(native_step_command(&record, &extra).is_err());
    }
    for status in ["active", "blocked"] {
        let mut neighboring = command.clone();
        neighboring["expected_status"] = json!(status);
        neighboring["report"] = json!("Collected actual requirements");
        assert!(native_step_command(&record, &neighboring).is_ok());
    }
    let mut terminal = record;
    terminal.terminal_status = Some(PmRuntimeStatus::Completed);
    assert!(native_step_command(&terminal, &command).is_err());
}

fn operation() -> PmDraftOperation {
    let task_id = Uuid::new_v4();
    let agent_id = Uuid::new_v4();
    let assignment_id = Uuid::new_v4();
    let project_id = Uuid::new_v4();
    let owner = Uuid::new_v4().to_string();
    let request = CreatePmDraftRequest {
        agent_id,
        title: "PM intake".into(),
        description: "Original request".into(),
        idempotency_key: "create-original".into(),
    };
    let input = TrackerDraftInput {
        snapshot_ref: Uuid::new_v4(),
        title: request.title.clone(),
        description: request.description.clone(),
        sha256: pm_input_hash(&request.title, &request.description),
    };
    let binding = TrackerDraftIdentity {
        tracker_instance_id: "tracker:qa".into(),
        project_id,
        task_id,
        root_task_id: task_id,
        owner_subject: owner.clone(),
    };
    let reservation = TrackerPmDraftReservation {
        contract_version: 1,
        variant: "pm_draft_reserved".into(),
        binding,
        owner_cas: TrackerOwnerCas {
            expected_version: 0,
            version: 1,
        },
        assignment: TrackerDraftAssignment {
            assignment_id,
            execution_id: Uuid::new_v4(),
            agent_id,
            version: 1,
            machine_subject: Uuid::new_v4().to_string(),
        },
        execution: TrackerDraftExecution {
            ordinal: "21".into(),
            key: "SDLC-21".into(),
        },
        input: TrackerDraftInputRef {
            snapshot_ref: input.snapshot_ref,
            sha256: input.sha256.clone(),
        },
        assignment_operation_key: format!("pm-draft:{assignment_id}"),
        admission_state: "reserved".into(),
        dispatch_allowed: false,
    };
    let mut op = PmDraftOperation {
        id: Uuid::new_v4(),
        owner_user_id: Uuid::new_v4(),
        owner_subject: owner.clone(),
        tracker_instance_id: "tracker:qa".into(),
        project_id,
        request,
        draft: Some(TrackerCreatedDraft {
            tracker_instance_id: "tracker:qa".into(),
            project_id,
            task_id,
            root_task_id: task_id,
            task_key: "QA-7".into(),
            owner_subject: owner,
            stage: "Draft".into(),
        }),
        input: Some(TrackerDraftInputReceipt {
            contract_version: 1,
            tracker_instance_id: "tracker:qa".into(),
            project_id,
            task_id,
            root_task_id: task_id,
            owner_subject: reservation.binding.owner_subject.clone(),
            input,
        }),
        reservation: Some(reservation.clone()),
        session_id: Some(Uuid::new_v4()),
        credentials: None,
        execution_lease: None,
        workflow_assignment: None,
    };
    let credential = serde_json::to_value(
        PmCredentialCommand::tracker(&op.execution_identity().unwrap(), op.credential_key(), 300)
            .unwrap(),
    )
    .unwrap();
    op.apply(PmDraftProof::CredentialIntent(PmCredentialIntent {
        command: credential.clone(),
        request_sha256: pm_canonical_hash(&credential),
        parent_fingerprint: "a".repeat(64),
        base_origin: "http://base.test/".into(),
        tracker_origin: "http://tracker.test/".into(),
        machine_subject: reservation.assignment.machine_subject.clone(),
    }))
    .unwrap();
    op.apply(PmDraftProof::CredentialAcknowledged(PmCredentialReceipt {
        token_id: Uuid::new_v4(),
        expires_at: Utc::now() + Duration::seconds(300),
        scopes: serde_json::from_value(credential["scopes"].clone()).unwrap(),
    }))
    .unwrap();
    let claim = op.lease_claim().unwrap();
    let now = Utc::now();
    let receipt = PmExecutionLeaseReceipt {
        contract_version: 1,
        binding: reservation.binding,
        owner_version: 1,
        fence: claim.fence.clone(),
        lease: PmExecutionLease {
            lease_id: Uuid::new_v4(),
            version: 1,
            holder_subject: reservation.assignment.machine_subject,
            claimed_at: now,
            heartbeat_at: now,
            expires_at: now + Duration::seconds(30),
        },
        ttl_seconds: 30,
        heartbeat_seconds: 10,
        dispatch_allowed: false,
    };
    op.apply(PmDraftProof::LeaseIntent(claim)).unwrap();
    op.apply(PmDraftProof::LeaseAcknowledged(receipt)).unwrap();
    op
}

fn intent(op: &PmDraftOperation) -> PmWorkflowAssignmentIntent {
    let command = assignment_command(op, &compatibility()).unwrap();
    PmWorkflowAssignmentIntent {
        request_sha256: pm_canonical_hash(&command),
        command,
        workflow_origin: "http://workflow.test/".into(),
        credential_fingerprint: "f".repeat(64),
    }
}

fn response(intent: &PmWorkflowAssignmentIntent) -> Value {
    let c = &intent.command;
    json!({"task_key":c["task"],"workflow_id":7,"workflow_key":"hermes-sdlc:project_manager",
        "mode_id":8,"mode_key":"draft","cycle_number":0,"attempt_number":1,
        "assignment_operation_key":c["assignment_operation_key"],"assignment_revision":1,
        "role_key":"project_manager","execution_scope":"business","stage_key":"draft",
        "business_task_ref":c["task_ref"],"root_task_ref":c["root_ref"],"work_item_ref":null,"work_item_revision":null,
        "queue_item_ref":null,"task_workspace_ref":null,"workspace_revision":null,"tech_execution_workspace_ref":null,
        "tech_execution_attempt_ref":null,"decomposition_revision_ref":null,"stage_revision":"1","assignment_ref":c["assignment_ref"],
        "binding_ref":null,"hermes_run_ref":null,"bind_operation_key":null,"concrete_agent_ref":null,
        "workspace_generation":null,"lease_generation":1,"exact_input_refs":[{"kind":"pm_draft_input","ref":c["input_snapshot_ref"],"hash":c["input_sha256"]}],
        "binding_state":"unbound","status":"active","current_phase_id":9,"current_phase_code":"PM-DRAFT-01","current_phase_name":"Draft intake"})
}

#[test]
fn workflow_command_uses_real_reservation_and_refuses_missing_lease_or_source_drift() {
    let op = operation();
    let original = intent(&op);
    original.verify(&op).unwrap();
    assert_eq!(original.command.as_object().unwrap().len(), 14);
    assert!(original.command.get("queue_item_ref").is_none());
    assert!(original.command.get("task_workspace_ref").is_none());
    let mut missing = op.clone();
    missing.execution_lease = None;
    assert!(assignment_command(&missing, &compatibility()).is_err());
    let mut drift = op;
    drift.input.as_mut().unwrap().input.sha256 = "e".repeat(64);
    assert!(original.verify(&drift).is_err());
    let mut wrong = compatibility();
    wrong.catalog_version = 3;
    assert!(wrong.validate().is_err());
}

#[test]
fn original_workflow_intent_and_ack_are_immutable_and_do_not_dispatch() {
    let mut op = operation();
    let original = intent(&op);
    let receipt = PmWorkflowAssignedDraft::verified(response(&original), &original).unwrap();
    assert!(
        op.apply(PmDraftProof::WorkflowAcknowledged(receipt.clone()))
            .is_err()
    );
    op.apply(PmDraftProof::WorkflowIntent(original.clone()))
        .unwrap();
    op.apply(PmDraftProof::WorkflowAcknowledged(receipt.clone()))
        .unwrap();
    let before = op.clone();
    op.apply(PmDraftProof::WorkflowIntent(original.clone()))
        .unwrap();
    op.apply(PmDraftProof::WorkflowAcknowledged(receipt.clone()))
        .unwrap();
    assert_eq!(op, before);
    assert!(!op.response().dispatch_allowed);
    let mut changed = original;
    changed.workflow_origin = "http://another-owner.test/".into();
    assert!(op.apply(PmDraftProof::WorkflowIntent(changed)).is_err());
    let mut changed = receipt;
    changed.phase_id += 1;
    assert!(
        op.apply(PmDraftProof::WorkflowAcknowledged(changed))
            .is_err()
    );
}

#[test]
fn workflow_receipt_requires_exact_closed_nulls_roles_refs_and_binding() {
    let original = intent(&operation());
    let value = response(&original);
    let expected = PmWorkflowAssignedDraft::verified(value.clone(), &original).unwrap();
    for (field, changed) in [
        ("queue_item_ref", json!("fake")),
        ("mode_key", json!("analysis")),
        ("role_key", json!("analyst")),
        ("business_task_ref", json!(Uuid::new_v4())),
        ("cycle_number", json!(1)),
        ("current_phase_id", json!(0)),
        ("unknown", json!(true)),
    ] {
        let mut malformed = value.clone();
        malformed[field] = changed;
        assert!(
            PmWorkflowAssignedDraft::verified(malformed, &original).is_err(),
            "{field}"
        );
    }
    let mut omitted = value.clone();
    omitted.as_object_mut().unwrap().remove("queue_item_ref");
    assert!(PmWorkflowAssignedDraft::verified(omitted, &original).is_err());
    let mut bound = value;
    bound["binding_state"] = json!("bound");
    bound["binding_ref"] = json!("original-binding");
    bound["hermes_run_ref"] = json!("native_run");
    bound["bind_operation_key"] = json!("original-bind");
    bound["concrete_agent_ref"] = original.command["agent_ref"].clone();
    assert_eq!(
        PmWorkflowAssignedDraft::verified(bound.clone(), &original).unwrap(),
        expected
    );
    bound["concrete_agent_ref"] = json!(Uuid::new_v4());
    assert!(PmWorkflowAssignedDraft::verified(bound, &original).is_err());
}
