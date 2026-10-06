use super::*;
use domain::{PmRunRecord, PmRuntimeStatus};

pub(super) async fn probe(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    record: &PmRunRecord,
) -> Result<PmRuntimeStatus, AppError> {
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
    // A fresh authenticated read, never a cached Fleet status or an SSE EOF.
    let response = supervisor
        .client
        .get(format!(
            "{}/v1/runs/{raw_id}",
            supervisor.hermes_base_url(agent).await?
        ))
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
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
    let payload = hermes_wire::read_json(response, reqwest::StatusCode::OK, 1024 * 1024).await?;
    // Hermes may resolve a Fleet session alias to its own persistent UUID.
    // Dispatch pins that effective ID; later probes cannot change it.
    let effective_session = record.hermes_session_ref.as_deref().ok_or_else(|| {
        AppError::Unavailable("PM effective session acceptance is unknown".into())
    })?;
    status(&payload, raw_id, effective_session)
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
    match payload.get("status").and_then(Value::as_str) {
        Some("queued" | "started" | "running" | "waiting_for_approval" | "stopping") => {
            Ok(PmRuntimeStatus::Running)
        }
        Some("completed") => {
            hermes_wire::terminal_readback(payload, raw_id)?;
            Ok(PmRuntimeStatus::Completed)
        }
        Some("failed" | "interrupted") => Ok(PmRuntimeStatus::Failed),
        Some("cancelled") => Ok(PmRuntimeStatus::Cancelled),
        Some("stopped") => Ok(PmRuntimeStatus::Stopped),
        _ => Err(AppError::Unavailable(
            "Hermes PM run status is unknown".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn value(state: &str) -> Value {
        json!({"object":"hermes.run","run_id":"run_test","session_id":"fleet:test:agent","status":state,
            "completed":state == "completed","partial":false,"interrupted":state == "interrupted"})
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
        assert_eq!(
            status(&value("interrupted"), "run_test", "fleet:test:agent").unwrap(),
            PmRuntimeStatus::Failed
        );
        assert_eq!(
            status(&value("completed"), "run_test", "fleet:test:agent").unwrap(),
            PmRuntimeStatus::Completed
        );
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
    fn pm_completion_cannot_hide_partial_or_interrupted_output() {
        for (field, bad) in [
            ("completed", json!(false)),
            ("partial", json!(true)),
            ("interrupted", json!(true)),
        ] {
            let mut payload = value("completed");
            payload[field] = bad;
            assert!(status(&payload, "run_test", "fleet:test:agent").is_err());
        }
        let mut payload = value("completed");
        payload.as_object_mut().unwrap().remove("completed");
        assert!(status(&payload, "run_test", "fleet:test:agent").is_err());
    }
}
