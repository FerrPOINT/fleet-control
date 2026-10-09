use super::*;

const CLAIM_HASH: &str = "fc79f5665b7bff504eb260ce259219224ea43f4aa9eaa4841383c3b04a23f25b";
const HEARTBEAT_HASH: &str = "1d606f38b9da01db063b18c619c7666e93adfceed1568de5bd7f53728c5b0660";
const LEASE_ID: &str = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
const FOREIGN_ID: &str = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";
const CLAIMED: &str = "2026-10-08T08:00:00.123456000Z";

fn fence() -> Value {
    // Exact fence and claim shape from Tracker domain tests at 357caa7.
    json!({
        "assignment_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        "execution_id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
        "agent_id": "cccccccc-cccc-4ccc-8ccc-cccccccccccc",
        "assignment_version": 1
    })
}

fn reservation() -> TrackerPmDraftReservation {
    serde_json::from_value(json!({
        "contract_version": 1,
        "variant": "pm_draft_reserved",
        "binding": {
            "tracker_instance_id": "tracker-test",
            "project_id": "11111111-1111-4111-8111-111111111111",
            "task_id": "22222222-2222-4222-8222-222222222222",
            "root_task_id": "22222222-2222-4222-8222-222222222222",
            "owner_subject": "44444444-4444-4444-8444-444444444444"
        },
        "owner_cas": {"expected_version": 0, "version": 1},
        "assignment": {
            "assignment_id": fence()["assignment_id"],
            "execution_id": fence()["execution_id"],
            "agent_id": fence()["agent_id"],
            "version": 1,
            "machine_subject": "55555555-5555-4555-8555-555555555555"
        },
        "execution": {"ordinal": "1", "key": "SDLC-1"},
        "input": {"snapshot_ref": "66666666-6666-4666-8666-666666666666", "sha256": "a".repeat(64)},
        "assignment_operation_key": "pm-draft:aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
        "admission_state": "reserved",
        "dispatch_allowed": false
    }))
    .unwrap()
}

fn claim() -> PmExecutionLeaseCommand {
    PmExecutionLeaseCommand::Claim(
        serde_json::from_value(json!({
            "expected_owner_version": 1, "fence": fence(), "idempotency_key": "lease-claim"
        }))
        .unwrap(),
    )
}

#[test]
fn claim_receipt_is_bound_to_the_original_owner_fence_and_never_dispatches() {
    let command = claim();
    let PmExecutionLeaseCommand::Claim(claim) = &command else {
        unreachable!()
    };
    let response = with_operation(readback(lease(1, 0)), &command, lease(1, 0));
    let receipt: PmExecutionLeaseReceipt =
        serde_json::from_value(response["operation"]["result"].clone()).unwrap();
    receipt.verify_claim(&reservation(), claim).unwrap();
    for field in ["lease_id", "holder_subject", "expires_at"] {
        let mut changed = response["operation"]["result"].clone();
        changed["lease"][field] = json!(if field == "expires_at" {
            timestamp(31)
        } else {
            FOREIGN_ID.into()
        });
        let changed: PmExecutionLeaseReceipt = serde_json::from_value(changed).unwrap();
        if field == "lease_id" {
            // A fresh valid UUID alone is not authority; original-key readback anchors it.
            let changed_readback =
                with_operation(readback(lease(1, 0)), &command, json!(changed.lease));
            assert!(
                PmExecutionLeaseReadback::verified(
                    changed_readback,
                    &reservation(),
                    Some(&command)
                )
                .is_err()
            );
        } else {
            assert!(changed.verify_claim(&reservation(), claim).is_err());
        }
    }
    let mut foreign = reservation();
    foreign.binding.project_id = Uuid::parse_str(FOREIGN_ID).unwrap();
    assert!(receipt.verify_claim(&foreign, claim).is_err());
    let mut changed = receipt.clone();
    changed.dispatch_allowed = true;
    assert!(changed.verify_claim(&reservation(), claim).is_err());
}

#[test]
fn private_claim_journal_is_original_key_only_and_preserves_creation_response() {
    let mut reserved = reservation();
    let request = crate::CreatePmDraftRequest {
        agent_id: reserved.assignment.agent_id,
        title: "Frozen intake".into(),
        description: "Original request".into(),
        idempotency_key: "create-one".into(),
    };
    reserved.input.sha256 = crate::pm_input_hash(&request.title, &request.description);
    let mut operation = crate::PmDraftOperation {
        id: Uuid::new_v4(),
        owner_user_id: Uuid::new_v4(),
        owner_subject: reserved.binding.owner_subject.clone(),
        tracker_instance_id: reserved.binding.tracker_instance_id.clone(),
        project_id: reserved.binding.project_id,
        request,
        draft: Some(crate::TrackerCreatedDraft {
            tracker_instance_id: reserved.binding.tracker_instance_id.clone(),
            project_id: reserved.binding.project_id,
            task_id: reserved.binding.task_id,
            root_task_id: reserved.binding.root_task_id,
            task_key: "PM-1".into(),
            owner_subject: reserved.binding.owner_subject.clone(),
            stage: "Draft".into(),
        }),
        input: None,
        reservation: Some(reserved.clone()),
        session_id: Some(Uuid::new_v4()),
        credentials: None,
        execution_lease: None,
    };
    assert!(operation.lease_claim().is_err());
    let credential = crate::PmCredentialCommand::tracker(
        &operation.execution_identity().unwrap(),
        operation.credential_key(),
        300,
    )
    .unwrap();
    let command = serde_json::to_value(credential).unwrap();
    operation
        .apply(crate::PmDraftProof::CredentialIntent(
            crate::PmCredentialIntent {
                request_sha256: crate::pm_canonical_hash(&command),
                command,
                parent_fingerprint: "a".repeat(64),
                base_origin: "http://base.test".into(),
                tracker_origin: "http://tracker.test".into(),
                machine_subject: reserved.assignment.machine_subject.clone(),
            },
        ))
        .unwrap();
    operation
        .apply(crate::PmDraftProof::CredentialAcknowledged(
            crate::PmCredentialReceipt {
                token_id: Uuid::new_v4(),
                expires_at: Utc::now() + Duration::seconds(60),
                scopes: serde_json::from_value(
                    operation.credentials.as_ref().unwrap().intent.command["scopes"].clone(),
                )
                .unwrap(),
            },
        ))
        .unwrap();
    let claim = operation.lease_claim().unwrap();
    let command = PmExecutionLeaseCommand::Claim(claim.clone());
    let response = with_operation(readback(lease(1, 0)), &command, lease(1, 0));
    let receipt: PmExecutionLeaseReceipt =
        serde_json::from_value(response["operation"]["result"].clone()).unwrap();
    assert!(
        operation
            .apply(crate::PmDraftProof::LeaseAcknowledged(receipt.clone()))
            .is_err()
    );
    operation
        .apply(crate::PmDraftProof::LeaseIntent(claim.clone()))
        .unwrap();
    operation
        .apply(crate::PmDraftProof::LeaseAcknowledged(receipt.clone()))
        .unwrap();
    let saved = operation.clone();
    operation
        .apply(crate::PmDraftProof::LeaseIntent(claim.clone()))
        .unwrap();
    operation
        .apply(crate::PmDraftProof::LeaseAcknowledged(receipt.clone()))
        .unwrap();
    assert_eq!(operation, saved);
    let mut foreign = claim;
    foreign.idempotency_key = "different-key".into();
    assert!(
        operation
            .apply(crate::PmDraftProof::LeaseIntent(foreign))
            .is_err()
    );
    let mut foreign = receipt;
    foreign.lease.lease_id = Uuid::new_v4();
    assert!(
        operation
            .apply(crate::PmDraftProof::LeaseAcknowledged(foreign))
            .is_err()
    );
    assert_eq!(operation, saved);
    assert!(!operation.response().dispatch_allowed);
}

fn heartbeat() -> PmExecutionLeaseCommand {
    // Exact renewal command shape from Tracker's execution_lease HTTP example.
    PmExecutionLeaseCommand::Heartbeat(
        serde_json::from_value(json!({
            "expected_owner_version": 1, "fence": fence(), "lease_id": LEASE_ID,
            "expected_lease_version": 1, "idempotency_key": "heartbeat-1"
        }))
        .unwrap(),
    )
}

fn timestamp(seconds: i64) -> String {
    (CLAIMED.parse::<DateTime<Utc>>().unwrap() + Duration::seconds(seconds))
        .to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn lease(version: u64, heartbeat_seconds: i64) -> Value {
    json!({
        "lease_id": LEASE_ID, "version": version,
        "holder_subject": reservation().assignment.machine_subject,
        "claimed_at": CLAIMED, "heartbeat_at": timestamp(heartbeat_seconds),
        "expires_at": timestamp(heartbeat_seconds + 30)
    })
}

fn readback(current: Value) -> Value {
    json!({
        "contract_version": 1, "binding": reservation().binding,
        "owner_version": 1, "fence": fence(), "observed_at": timestamp(25),
        "state": if current.is_null() { "unclaimed" } else { "active" },
        "current": current, "operation": null, "dispatch_allowed": false
    })
}

fn with_operation(mut readback: Value, command: &PmExecutionLeaseCommand, result: Value) -> Value {
    readback["operation"] = json!({
        "idempotency_key": command.idempotency_key(),
        "request_sha256": match command {
            PmExecutionLeaseCommand::Claim(_) => CLAIM_HASH,
            PmExecutionLeaseCommand::Heartbeat(_) => HEARTBEAT_HASH,
        },
        "result": {
            "contract_version": 1, "binding": reservation().binding,
            "owner_version": 1, "fence": fence(), "lease": result,
            "ttl_seconds": 30, "heartbeat_seconds": 10, "dispatch_allowed": false
        }
    });
    readback
}

fn verified(value: Value, command: Option<&PmExecutionLeaseCommand>) -> PmExecutionLeaseReadback {
    let parsed =
        PmExecutionLeaseReadback::verified(value.clone(), &reservation(), command).unwrap();
    assert_eq!(serde_json::to_value(&parsed).unwrap(), value);
    parsed
}

fn rejected(value: Value, command: Option<&PmExecutionLeaseCommand>) {
    assert!(matches!(
        PmExecutionLeaseReadback::verified(value, &reservation(), command),
        Err(AppError::Conflict(_))
    ));
}

#[test]
fn producer_hash_labels_and_original_payload_are_exact() {
    assert_eq!(claim().request_sha256(), CLAIM_HASH);
    assert_eq!(heartbeat().request_sha256(), HEARTBEAT_HASH);
    assert_eq!(claim().idempotency_key(), "lease-claim");
    assert_eq!(heartbeat().idempotency_key(), "heartbeat-1");
    assert_eq!(heartbeat().payload()["expected_lease_version"], 1);
}

#[test]
fn producer_unclaimed_active_and_expiry_boundary_roundtrip() {
    assert_eq!(
        verified(readback(Value::Null), None).state,
        PmExecutionLeaseState::Unclaimed
    );
    verified(readback(Value::Null), Some(&claim()));
    let active = verified(readback(lease(1, 0)), None);
    assert_eq!(active.state, PmExecutionLeaseState::Active);
    assert!(active.operation.is_none());
    let mut expired = readback(lease(1, 0));
    expired["observed_at"] = json!(timestamp(30));
    expired["state"] = json!("expired");
    assert_eq!(
        verified(expired, None).state,
        PmExecutionLeaseState::Expired
    );
}

#[test]
fn historical_claim_and_heartbeat_can_be_lower_or_equal_to_current() {
    let claim = claim();
    let heartbeat = heartbeat();
    verified(
        with_operation(readback(lease(1, 0)), &claim, lease(1, 0)),
        Some(&claim),
    );
    verified(
        with_operation(readback(lease(3, 20)), &claim, lease(1, 0)),
        Some(&claim),
    );
    verified(
        with_operation(readback(lease(2, 10)), &heartbeat, lease(2, 10)),
        Some(&heartbeat),
    );
    verified(
        with_operation(readback(lease(3, 10)), &heartbeat, lease(2, 10)),
        Some(&heartbeat),
    );
    let mut history = with_operation(readback(lease(3, 20)), &heartbeat, lease(2, 10));
    // An expired historical receipt remains valid evidence, and cannot renew current.
    history["observed_at"] = json!(timestamp(45));
    let parsed = verified(history.clone(), Some(&heartbeat));
    assert_eq!(parsed.current.unwrap().version, 3);
    assert_eq!(parsed.operation.unwrap().result.lease.version, 2);
    history["observed_at"] = json!(timestamp(50));
    history["state"] = json!("expired");
    assert_eq!(
        verified(history, Some(&heartbeat)).state,
        PmExecutionLeaseState::Expired
    );
}

#[test]
fn required_nullables_and_all_nested_fields_cannot_be_omitted() {
    for field in ["current", "operation"] {
        let mut value = readback(Value::Null);
        value.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<PmExecutionLeaseReadback>(value.clone()).is_err());
        rejected(value, None);
    }
    let command = claim();
    let original = with_operation(readback(lease(1, 0)), &command, lease(1, 0));
    for path in [
        "",
        "/binding",
        "/fence",
        "/current",
        "/operation",
        "/operation/result",
        "/operation/result/binding",
        "/operation/result/fence",
        "/operation/result/lease",
    ] {
        let keys: Vec<_> = original
            .pointer(path)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        for key in keys {
            let mut value = original.clone();
            value
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(&key);
            rejected(value, Some(&command));
        }
    }
}

#[test]
fn every_object_is_closed_and_dispatch_is_always_false() {
    let command = claim();
    let original = with_operation(readback(lease(1, 0)), &command, lease(1, 0));
    for path in [
        "",
        "/binding",
        "/fence",
        "/current",
        "/operation",
        "/operation/result",
        "/operation/result/binding",
        "/operation/result/fence",
        "/operation/result/lease",
    ] {
        for field in ["unknown", "dispatch_allowed"] {
            let mut value = original.clone();
            value.pointer_mut(path).unwrap()[field] = json!(true);
            rejected(value, Some(&command));
        }
    }
    let mut r = reservation();
    r.dispatch_allowed = true;
    assert!(PmExecutionLeaseReadback::verified(readback(Value::Null), &r, None).is_err());
}

#[test]
fn uuids_are_canonical_non_nil_and_binding_is_exact() {
    let command = claim();
    let original = with_operation(readback(lease(1, 0)), &command, lease(1, 0));
    for path in ["/binding", "/operation/result/binding"] {
        for field in ["project_id", "task_id", "root_task_id"] {
            for id in [
                FOREIGN_ID,
                "00000000-0000-0000-0000-000000000000",
                "22222222222242228222222222222222",
                "invalid",
            ] {
                let mut value = original.clone();
                value.pointer_mut(path).unwrap()[field] = json!(id);
                rejected(value, Some(&command));
            }
        }
        for field in ["owner_subject", "tracker_instance_id"] {
            let mut value = original.clone();
            value.pointer_mut(path).unwrap()[field] = json!("foreign");
            rejected(value, Some(&command));
        }
    }
    for path in ["/fence", "/operation/result/fence"] {
        for field in ["assignment_id", "execution_id", "agent_id"] {
            for id in [
                FOREIGN_ID,
                "AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA",
                "00000000-0000-0000-0000-000000000000",
                "invalid",
            ] {
                let mut value = original.clone();
                value.pointer_mut(path).unwrap()[field] = json!(id);
                rejected(value, Some(&command));
            }
        }
    }
    for path in ["/current", "/operation/result/lease"] {
        for id in [
            "DDDDDDDD-DDDD-4DDD-8DDD-DDDDDDDDDDDD",
            "00000000-0000-0000-0000-000000000000",
            "invalid",
        ] {
            let mut value = original.clone();
            value.pointer_mut(path).unwrap()["lease_id"] = json!(id);
            rejected(value, Some(&command));
        }
        for subject in ["foreign", "", &reservation().binding.owner_subject] {
            let mut value = original.clone();
            value.pointer_mut(path).unwrap()["holder_subject"] = json!(subject);
            rejected(value, Some(&command));
        }
    }
}

#[test]
fn versions_are_positive_safe_integers_and_owner_and_fence_match() {
    let command = claim();
    let original = with_operation(readback(lease(1, 0)), &command, lease(1, 0));
    for path in [
        "/owner_version",
        "/fence/assignment_version",
        "/current/version",
        "/operation/result/owner_version",
        "/operation/result/fence/assignment_version",
        "/operation/result/lease/version",
    ] {
        for bad in [
            json!(0),
            json!(-1),
            json!(MAX_SAFE_VERSION + 1),
            json!(u64::MAX),
            json!(1.0),
            json!("1"),
            Value::Null,
        ] {
            let mut value = original.clone();
            *value.pointer_mut(path).unwrap() = bad;
            rejected(value, Some(&command));
        }
    }
    for path in [
        "/owner_version",
        "/fence/assignment_version",
        "/operation/result/owner_version",
        "/operation/result/fence/assignment_version",
    ] {
        let mut value = original.clone();
        *value.pointer_mut(path).unwrap() = json!(2);
        rejected(value, Some(&command));
    }
    verified(readback(lease(MAX_SAFE_VERSION, 10)), None);
    let mut r = reservation();
    r.owner_cas.version = MAX_SAFE_VERSION;
    r.assignment.version = MAX_SAFE_VERSION;
    let mut value = readback(Value::Null);
    value["owner_version"] = json!(MAX_SAFE_VERSION);
    value["fence"]["assignment_version"] = json!(MAX_SAFE_VERSION);
    assert!(PmExecutionLeaseReadback::verified(value, &r, None).is_ok());
}

#[test]
fn timestamps_require_producer_utc_nanos_and_valid_chronology() {
    let command = claim();
    let original = with_operation(readback(lease(1, 0)), &command, lease(1, 0));
    for path in [
        "/observed_at",
        "/current/claimed_at",
        "/current/heartbeat_at",
        "/current/expires_at",
        "/operation/result/lease/claimed_at",
        "/operation/result/lease/heartbeat_at",
        "/operation/result/lease/expires_at",
    ] {
        for bad in [
            "invalid",
            "2026-02-30T08:00:00.123456000Z",
            "2026-10-08T08:00:00Z",
            "2026-10-08T08:00:00.123456Z",
            "2026-10-08T08:00:00.123456000+00:00",
            "2026-10-08T08:00:60.123456000Z",
        ] {
            let mut value = original.clone();
            *value.pointer_mut(path).unwrap() = json!(bad);
            rejected(value, Some(&command));
        }
    }
    for path in ["/current", "/operation/result/lease"] {
        for (field, seconds) in [
            ("claimed_at", 1),
            ("heartbeat_at", -1),
            ("heartbeat_at", 1),
            ("expires_at", 31),
        ] {
            let mut value = original.clone();
            value.pointer_mut(path).unwrap()[field] = json!(timestamp(seconds));
            rejected(value, Some(&command));
        }
    }
    let mut future = readback(lease(2, 26));
    rejected(future.clone(), None);
    future["observed_at"] = json!(timestamp(26));
    verified(future, None);
    let mut fractional_expiry = readback(lease(1, 0));
    fractional_expiry["current"]["expires_at"] = json!("2026-10-08T08:00:30.123456001Z");
    rejected(fractional_expiry, None);
    let mut overflow = readback(lease(1, 0));
    for field in ["claimed_at", "heartbeat_at"] {
        overflow["current"][field] = json!("+262142-12-31T23:59:50.000000000Z");
    }
    overflow["observed_at"] = json!("+262142-12-31T23:59:50.000000000Z");
    overflow["current"]["expires_at"] = json!("+262142-12-31T23:59:59.999999999Z");
    rejected(overflow, None);
}

#[test]
fn state_contract_and_receipt_cadence_are_exact() {
    let command = claim();
    let original = with_operation(readback(lease(1, 0)), &command, lease(1, 0));
    for state in ["unclaimed", "expired", "unknown", "Active"] {
        let mut value = original.clone();
        value["state"] = json!(state);
        rejected(value, Some(&command));
    }
    for state in ["active", "expired"] {
        let mut value = readback(Value::Null);
        value["state"] = json!(state);
        rejected(value, None);
    }
    let mut at_expiry = original.clone();
    at_expiry["observed_at"] = json!(timestamp(30));
    rejected(at_expiry, Some(&command));
    for path in [
        "/contract_version",
        "/operation/result/contract_version",
        "/operation/result/ttl_seconds",
        "/operation/result/heartbeat_seconds",
    ] {
        for bad in [json!(0), json!(2), json!(31), Value::Null] {
            let mut value = original.clone();
            *value.pointer_mut(path).unwrap() = bad;
            rejected(value, Some(&command));
        }
    }
}

#[test]
fn operation_requires_original_command_key_hash_and_monotone_same_lease() {
    let command = heartbeat();
    let original = with_operation(readback(lease(3, 20)), &command, lease(2, 10));
    rejected(original.clone(), None);
    let mut no_current = original.clone();
    no_current["current"] = Value::Null;
    no_current["state"] = json!("unclaimed");
    rejected(no_current, Some(&command));
    verified(readback(lease(3, 20)), Some(&command));
    for (path, bad) in [
        ("/operation/idempotency_key", json!("another-key")),
        ("/operation/request_sha256", json!("0".repeat(64))),
        (
            "/operation/request_sha256",
            json!(HEARTBEAT_HASH.to_uppercase()),
        ),
        ("/operation/result/lease/lease_id", json!(FOREIGN_ID)),
        ("/operation/result/lease/claimed_at", json!(timestamp(1))),
        ("/operation/result/lease/version", json!(4)),
    ] {
        let mut value = original.clone();
        *value.pointer_mut(path).unwrap() = bad;
        rejected(value, Some(&command));
    }
    let mut later_history = with_operation(readback(lease(3, 20)), &command, lease(2, 21));
    rejected(later_history.clone(), Some(&command));
    // Equal versions must describe the same immutable result, including heartbeat.
    later_history["current"] = lease(2, 20);
    later_history["operation"]["result"]["lease"] = lease(2, 10);
    rejected(later_history, Some(&command));
    let mut changed = command.clone();
    if let PmExecutionLeaseCommand::Heartbeat(c) = &mut changed {
        c.expected_lease_version = 2;
    }
    rejected(original.clone(), Some(&changed));
    // Even a matching changed hash cannot certify a receipt for the old CAS.
    let mut changed_hash = original.clone();
    changed_hash["operation"]["request_sha256"] = json!(changed.request_sha256());
    rejected(changed_hash, Some(&changed));
    if let PmExecutionLeaseCommand::Heartbeat(c) = &mut changed {
        c.expected_lease_version = 1;
        c.lease_id = Uuid::parse_str(FOREIGN_ID).unwrap();
    }
    let mut foreign = original.clone();
    foreign["operation"]["request_sha256"] = json!(changed.request_sha256());
    rejected(foreign, Some(&changed));
    let claim = claim();
    let wrong_claim = with_operation(readback(lease(3, 20)), &claim, lease(2, 10));
    rejected(wrong_claim, Some(&claim));
}

#[test]
fn typed_commands_are_closed_canonical_and_checked_even_without_a_receipt() {
    for command in [claim(), heartbeat()] {
        let original = command.payload();
        let parse = |value: Value| match &command {
            PmExecutionLeaseCommand::Claim(_) => {
                serde_json::from_value::<PmExecutionLeaseClaim>(value).is_ok()
            }
            PmExecutionLeaseCommand::Heartbeat(_) => {
                serde_json::from_value::<PmExecutionLeaseHeartbeat>(value).is_ok()
            }
        };
        assert!(parse(original.clone()));
        for key in original.as_object().unwrap().keys() {
            let mut missing = original.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(!parse(missing));
        }
        for path in ["", "/fence"] {
            let mut extra = original.clone();
            extra.pointer_mut(path).unwrap()["dispatch_allowed"] = json!(true);
            assert!(!parse(extra));
        }
        for path in ["/expected_owner_version", "/fence/assignment_version"] {
            for bad in [
                json!(0),
                json!(-1),
                json!(MAX_SAFE_VERSION + 1),
                json!(1.0),
                json!("1"),
            ] {
                let mut invalid = original.clone();
                *invalid.pointer_mut(path).unwrap() = bad;
                assert!(!parse(invalid));
            }
        }
        for field in ["assignment_id", "execution_id", "agent_id"] {
            for bad in [
                "AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA",
                "00000000-0000-0000-0000-000000000000",
            ] {
                let mut invalid = original.clone();
                invalid["fence"][field] = json!(bad);
                assert!(!parse(invalid));
            }
        }
    }
    for bad in [
        json!(0),
        json!(-1),
        json!(MAX_SAFE_VERSION + 1),
        json!(1.0),
        json!("1"),
    ] {
        let mut invalid = heartbeat().payload();
        invalid["expected_lease_version"] = bad;
        assert!(serde_json::from_value::<PmExecutionLeaseHeartbeat>(invalid).is_err());
    }
    for bad in [
        "DDDDDDDD-DDDD-4DDD-8DDD-DDDDDDDDDDDD",
        "00000000-0000-0000-0000-000000000000",
    ] {
        let mut invalid = heartbeat().payload();
        invalid["lease_id"] = json!(bad);
        assert!(serde_json::from_value::<PmExecutionLeaseHeartbeat>(invalid).is_err());
    }
    for key in ["", "white space", "control\n", &"x".repeat(129)] {
        let mut invalid = claim();
        if let PmExecutionLeaseCommand::Claim(c) = &mut invalid {
            c.idempotency_key = key.into();
        }
        rejected(readback(Value::Null), Some(&invalid));
    }
    let mut invalid = heartbeat();
    if let PmExecutionLeaseCommand::Heartbeat(c) = &mut invalid {
        c.expected_lease_version = MAX_SAFE_VERSION + 1;
    }
    rejected(readback(Value::Null), Some(&invalid));
    let mut invalid = claim();
    if let PmExecutionLeaseCommand::Claim(c) = &mut invalid {
        c.expected_owner_version = 2;
    }
    rejected(readback(Value::Null), Some(&invalid));
    let mut invalid = claim();
    if let PmExecutionLeaseCommand::Claim(c) = &mut invalid {
        c.fence.agent_id = Uuid::parse_str(FOREIGN_ID).unwrap();
    }
    rejected(readback(Value::Null), Some(&invalid));
}
