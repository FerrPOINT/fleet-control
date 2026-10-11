use super::*;
use domain::{PmRunRecord, PmRuntimeStatus};

pub(super) async fn probe(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    record: &PmRunRecord,
) -> Result<PmRuntimeStatus, AppError> {
    let current = supervisor
        .repo
        .get_pm_run(record.reservation.session_run_id)
        .await?;
    if current.reservation != record.reservation
        || current.hermes_run_ref != record.hermes_run_ref
        || current.hermes_session_ref != record.hermes_session_ref
    {
        return Err(AppError::conflict("PM original run custody changed"));
    }
    let record = &current;
    record.reservation.validate()?;
    if agent.kind != AgentKind::Hermes || agent.id != record.reservation.identity.agent_id()? {
        return Err(AppError::Forbidden);
    }
    let raw_id = record
        .hermes_run_ref
        .as_deref()
        .ok_or_else(|| AppError::Unavailable("PM acceptance is unknown".into()))?;
    if !crate::pm_execution::valid_hermes_ref(raw_id) {
        return Err(AppError::Unavailable(
            "PM runtime reference is invalid".into(),
        ));
    }
    let run = supervisor
        .repo
        .get_session_agent_run(record.reservation.session_run_id)
        .await?;
    if run.session_id != record.reservation.session_id
        || run.agent_id != agent.id
        || run.runtime_run_id.as_deref() != Some(raw_id)
        || run.runtime_session_id.as_deref()
            != Some(record.reservation.runtime_session_id().as_str())
    {
        return Err(AppError::Unavailable(
            "PM runtime mapping does not match reservation".into(),
        ));
    }
    let base = if let Some(intent) = supervisor.repo.get_pm_dispatch(run.id).await? {
        if !intent.submitted || intent.hermes_run_ref.as_deref() != Some(raw_id) {
            return Err(AppError::conflict(
                "PM original dispatch ACK does not match",
            ));
        }
        if let Some(terminal) = record.terminal_status.filter(|status| status.terminal()) {
            // Revalidate owner/task custody, not the now-replaced native generation.
            let (custody, terminal_committed) = supervisor.repo.pm_stream_context(run.id).await?;
            if custody.reservation != record.reservation
                || custody.hermes_run_ref != record.hermes_run_ref
                || custody.hermes_session_ref != record.hermes_session_ref
                || custody.terminal_status != Some(terminal)
            {
                return Err(AppError::conflict("PM terminal custody changed"));
            }
            if let Some(terminal) = durable_terminal(record, &run, &intent, terminal_committed) {
                return Ok(terminal);
            }
        }
        pm_dispatch::verify_context(supervisor, agent, &intent).await?
    } else {
        supervisor.run_base_url(agent, &run).await?
    };
    // Incomplete terminal custody and live runs still require the original native probe.
    let response = supervisor
        .client
        .get(format!("{base}/v1/runs/{raw_id}"))
        .timeout(Duration::from_secs(10))
        .bearer_auth(crate::agent_runtime_token(&supervisor.config, agent.id)?)
        .send()
        .await
        .map_err(|_| AppError::Unavailable("Hermes PM readback is unavailable".into()))?;
    if !response.status().is_success() {
        return Err(AppError::Unavailable(
            "Hermes PM readback did not return a run".into(),
        ));
    }
    let mut stream = response.bytes_stream();
    let mut body = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk
            .map_err(|_| AppError::Unavailable("Hermes PM readback was interrupted".into()))?;
        if body.len().saturating_add(chunk.len()) > 1024 * 1024 {
            return Err(AppError::Unavailable(
                "Hermes PM readback exceeded its size limit".into(),
            ));
        }
        body.extend_from_slice(&chunk);
    }
    let payload: Value = serde_json::from_slice(&body)
        .map_err(|_| AppError::Unavailable("Hermes PM readback is malformed".into()))?;
    // Hermes may resolve a Fleet session alias to its own persistent UUID.
    // Dispatch pins that effective ID; later probes cannot change it.
    let effective_session = record.hermes_session_ref.as_deref().ok_or_else(|| {
        AppError::Unavailable("PM effective session acceptance is unknown".into())
    })?;
    status(&payload, raw_id, effective_session)
}

fn durable_terminal(
    record: &PmRunRecord,
    run: &domain::SessionAgentRun,
    intent: &domain::PmDispatchIntent,
    terminal_committed: bool,
) -> Option<PmRuntimeStatus> {
    let terminal = record.terminal_status.filter(|status| status.terminal())?;
    // Historical completed observations did not require the native completion flags.
    if terminal == PmRuntimeStatus::Completed && !terminal_committed {
        return None;
    }
    let raw_id = record.hermes_run_ref.as_deref()?;
    let effective = record.hermes_session_ref.as_deref()?;
    let body: Value = serde_json::from_str(&intent.request_body).ok()?;
    (record.reservation.validate().is_ok()
        && crate::pm_execution::valid_hermes_ref(raw_id)
        && domain::valid_ref(effective, 512)
        && intent.submitted
        && intent.session_run_id == record.reservation.session_run_id
        && intent.hermes_run_ref.as_deref() == Some(raw_id)
        && run.id == record.reservation.session_run_id
        && run.session_id == record.reservation.session_id
        && Some(run.agent_id) == record.reservation.identity.agent_id().ok()
        && run.run_role == domain::SessionRunRole::Primary
        && run.runtime_run_id.as_deref() == Some(raw_id)
        && run.runtime_session_id.as_deref()
            == Some(record.reservation.runtime_session_id().as_str())
        && run.state.as_str() == crate::pm_execution::visible_terminal(terminal)
        && body.as_object().is_some_and(|map| map.len() == 2)
        && body["input"]
            .as_str()
            .is_some_and(|input| !input.is_empty())
        && body["session_id"].as_str() == run.runtime_session_id.as_deref())
    .then_some(terminal)
}

fn status(payload: &Value, raw_id: &str, session_id: &str) -> Result<PmRuntimeStatus, AppError> {
    if payload.get("object").and_then(Value::as_str) != Some("hermes.run")
        || payload.get("run_id").and_then(Value::as_str) != Some(raw_id)
        || payload.get("session_id").and_then(Value::as_str) != Some(session_id)
    {
        return Err(AppError::Unavailable(
            "Hermes PM readback identity mismatch".into(),
        ));
    }
    if matches!(
        payload.get("status").and_then(Value::as_str),
        Some("queued" | "started" | "running" | "waiting_for_approval" | "stopping")
    ) {
        return Ok(PmRuntimeStatus::Running);
    }
    match hermes_wire::terminal_readback(payload, raw_id)? {
        "run.completed" => Ok(PmRuntimeStatus::Completed),
        "run.failed" | "run.interrupted" => Ok(PmRuntimeStatus::Failed),
        "run.cancelled" => Ok(PmRuntimeStatus::Cancelled),
        "run.stopped" => Ok(PmRuntimeStatus::Stopped),
        _ => Err(AppError::Unavailable(
            "Hermes PM run status is unknown".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custody() -> (
        PmRunRecord,
        domain::SessionAgentRun,
        domain::PmDispatchIntent,
    ) {
        let id = Uuid::new_v4();
        let session = Uuid::new_v4();
        let agent = Uuid::new_v4();
        let reservation = domain::PmRunReservation {
            session_id: session,
            session_run_id: id,
            identity: domain::PmExecutionIdentity {
                task: "SDLC-1".into(),
                execution_ref: Uuid::new_v4().to_string(),
                tracker_instance_ref: "tracker".into(),
                tracker_project_ref: Uuid::new_v4().to_string(),
                task_ref: Uuid::new_v4().to_string(),
                root_ref: Uuid::new_v4().to_string(),
                agent_ref: agent.to_string(),
                assignment_operation_key: "assign:original".into(),
                assignment_ref: Uuid::new_v4().to_string(),
                assignment_revision: 1,
            },
            binding_ref: "binding:original".into(),
            dispatch_operation_key: "dispatch:original".into(),
            checkpoint_ref: Some("checkpoint:original".into()),
            fence: 1,
        };
        let alias = reservation.runtime_session_id();
        let record = PmRunRecord {
            reservation,
            hermes_run_ref: Some("run_original".into()),
            hermes_session_ref: Some("effective_original".into()),
            terminal_status: Some(PmRuntimeStatus::Stopped),
        };
        let run: domain::SessionAgentRun = serde_json::from_value(json!({
            "id":id,"session_id":session,"agent_id":agent,"agent_name":"agent1",
            "runtime_session_id":alias,"runtime_run_id":"run_original","run_role":"primary",
            "state":"cancelled","last_error":null,"last_event_at":null,"model":null,"provider":null,
            "model_options":{},"created_at":"2026-10-11T00:00:00Z","updated_at":"2026-10-11T00:00:00Z"
        })).unwrap();
        let intent = domain::PmDispatchIntent {
            session_run_id: id,
            origin: "http://127.0.0.1:24003".into(),
            credential_fingerprint: "a".repeat(64),
            request_body: json!({"input":"original","session_id":alias}).to_string(),
            workflow_assignment: json!({}),
            workflow_origin: "http://workflow.test".into(),
            workflow_credential_fingerprint: "b".repeat(64),
            runtime_context: json!({"fleet_container_generation":Uuid::new_v4()}),
            submitted: true,
            hermes_run_ref: Some("run_original".into()),
        };
        (record, run, intent)
    }

    #[test]
    fn durable_terminal_requires_complete_original_acceptance_and_terminal_mapping() {
        let (record, run, intent) = custody();
        assert_eq!(
            durable_terminal(&record, &run, &intent, false),
            Some(PmRuntimeStatus::Stopped)
        );
        for field in [
            "status",
            "running",
            "ack",
            "effective",
            "submitted",
            "journal-id",
            "journal-ack",
            "run-id",
            "session",
            "agent",
            "role",
            "native",
            "alias",
            "body",
            "body-alias",
            "fence",
        ] {
            let (mut record, mut run, mut intent) = custody();
            match field {
                "status" => record.terminal_status = None,
                "running" => record.terminal_status = Some(PmRuntimeStatus::Running),
                "ack" => record.hermes_run_ref = None,
                "effective" => record.hermes_session_ref = None,
                "submitted" => intent.submitted = false,
                "journal-id" => intent.session_run_id = Uuid::new_v4(),
                "journal-ack" => intent.hermes_run_ref = Some("run_foreign".into()),
                "run-id" => run.id = Uuid::new_v4(),
                "session" => run.session_id = Uuid::new_v4(),
                "agent" => run.agent_id = Uuid::new_v4(),
                "role" => run.run_role = domain::SessionRunRole::Executor,
                "native" => run.runtime_run_id = Some("run_foreign".into()),
                "alias" => run.runtime_session_id = Some("fleet:foreign".into()),
                "body" => intent.request_body = "{}".into(),
                "body-alias" => {
                    intent.request_body =
                        json!({"input":"original","session_id":"foreign"}).to_string()
                }
                "fence" => record.reservation.fence = 0,
                _ => unreachable!(),
            }
            assert_eq!(
                durable_terminal(&record, &run, &intent, false),
                None,
                "{field}"
            );
        }
        for (terminal, state) in [
            (
                PmRuntimeStatus::Completed,
                domain::SessionRunState::Completed,
            ),
            (PmRuntimeStatus::Failed, domain::SessionRunState::Failed),
            (
                PmRuntimeStatus::Cancelled,
                domain::SessionRunState::Cancelled,
            ),
            (PmRuntimeStatus::Stopped, domain::SessionRunState::Cancelled),
        ] {
            let (mut record, mut run, intent) = custody();
            record.terminal_status = Some(terminal);
            run.state = state;
            assert_eq!(
                durable_terminal(&record, &run, &intent, true),
                Some(terminal)
            );
            assert_eq!(
                durable_terminal(&record, &run, &intent, false),
                (terminal != PmRuntimeStatus::Completed).then_some(terminal)
            );
            run.state = domain::SessionRunState::Running;
            assert_eq!(durable_terminal(&record, &run, &intent, true), None);
        }
    }
    fn value(state: &str) -> Value {
        json!({"object":"hermes.run","run_id":"run_test","session_id":"fleet:test:agent","status":state,
            "completed":state=="completed","partial":false,"interrupted":state=="interrupted"})
    }
    #[test]
    fn only_runtime_terminal_status_is_proof() {
        for state in [
            "queued",
            "started",
            "running",
            "waiting_for_approval",
            "stopping",
        ] {
            assert_eq!(
                status(&value(state), "run_test", "fleet:test:agent").unwrap(),
                PmRuntimeStatus::Running
            );
        }
        for (state, expected) in [
            ("completed", PmRuntimeStatus::Completed),
            ("failed", PmRuntimeStatus::Failed),
            ("interrupted", PmRuntimeStatus::Failed),
            ("cancelled", PmRuntimeStatus::Cancelled),
            ("stopped", PmRuntimeStatus::Stopped),
        ] {
            assert_eq!(
                status(&value(state), "run_test", "fleet:test:agent").unwrap(),
                expected
            );
        }
        for state in ["eof", "success", "succeeded", "unknown", ""] {
            assert!(status(&value(state), "run_test", "fleet:test:agent").is_err());
        }
    }
    #[test]
    fn wrong_or_missing_run_identity_fails_closed() {
        let payload = value("completed");
        assert!(status(&payload, "run_other", "fleet:test:agent").is_err());
        assert!(status(&payload, "run_test", "fleet:other:agent").is_err());
        let mut missing = payload.clone();
        missing.as_object_mut().unwrap().remove("session_id");
        assert!(status(&missing, "run_test", "fleet:test:agent").is_err());
        assert!(status(&Value::Null, "run_test", "fleet:test:agent").is_err());
    }

    #[test]
    fn completed_requires_the_shared_terminal_flags() {
        for (field, wrong) in [
            ("completed", false),
            ("partial", true),
            ("interrupted", true),
        ] {
            let mut missing = value("completed");
            missing.as_object_mut().unwrap().remove(field);
            assert!(status(&missing, "run_test", "fleet:test:agent").is_err());
            for bad in [json!(wrong), json!("false"), Value::Null] {
                let mut payload = value("completed");
                payload[field] = bad;
                assert!(status(&payload, "run_test", "fleet:test:agent").is_err());
            }
        }
    }
}
