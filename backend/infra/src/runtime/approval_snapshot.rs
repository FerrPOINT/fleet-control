use super::*;

pub(super) fn request(
    payload: &Value,
    run: &SessionAgentRun,
) -> Result<RuntimeApprovalCreate, AppError> {
    let native_id = run.runtime_run_id.as_deref().ok_or_else(invalid)?;
    if payload["status"] != "waiting_for_approval"
        || Some(hermes_wire::effective_session(payload, native_id)?.as_str())
            != run.runtime_session_id.as_deref()
    {
        return Err(invalid());
    }
    let event = &payload["approval"];
    let id = event["request_id"]
        .as_str()
        .filter(|id| domain::valid_ref(id, 256))
        .ok_or_else(invalid)?;
    if !event.is_object()
        || event["event"] != "approval.request"
        || event["run_id"].as_str() != Some(native_id)
        || event
            .get("session_id")
            .is_some_and(|id| id.as_str() != run.runtime_session_id.as_deref())
        || serde_json::to_vec(event).map_err(AppError::internal)?.len() > 65_536
    {
        return Err(invalid());
    }
    let choices = event["choices"].as_array().ok_or_else(invalid)?;
    if !choices.iter().any(|choice| choice == "once")
        || !choices.iter().any(|choice| choice == "deny")
    {
        return Err(invalid());
    }
    let prompt = ["prompt", "description", "command", "message"]
        .into_iter()
        .find_map(|field| {
            event[field]
                .as_str()
                .filter(|value| !value.trim().is_empty())
        })
        .filter(|value| value.len() <= 16_384)
        .ok_or_else(invalid)?;
    Ok(RuntimeApprovalCreate {
        session_id: run.session_id,
        session_run_id: run.id,
        agent_id: run.agent_id,
        runtime_run_id: native_id.into(),
        runtime_approval_id: Some(id.into()),
        prompt: prompt.into(),
        detail: event.clone(),
    })
}

fn invalid() -> AppError {
    AppError::Unavailable("native pending approval snapshot is not verified".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run() -> SessionAgentRun {
        SessionAgentRun {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            agent_name: "agent1".into(),
            runtime_session_id: Some("native-session".into()),
            runtime_run_id: Some("run_snapshot".into()),
            run_role: SessionRunRole::Primary,
            state: SessionRunState::Running,
            last_error: None,
            last_event_at: None,
            model: None,
            provider: None,
            model_options: json!({}),
            created_at: shared::now().to_rfc3339(),
            updated_at: shared::now().to_rfc3339(),
        }
    }
    fn payload() -> Value {
        json!({"object":"hermes.run","run_id":"run_snapshot","session_id":"native-session",
            "status":"waiting_for_approval","approval":{"event":"approval.request",
            "run_id":"run_snapshot","request_id":"request_snapshot","description":"Review this exact action",
            "command":"chmod 666 owned.txt","choices":["once","session","always","deny"]}})
    }
    #[test]
    fn pending_snapshot_keeps_exact_request_not_broad_grants() {
        let run = run();
        let req = request(&payload(), &run).unwrap();
        assert_eq!(req.session_run_id, run.id);
        assert_eq!(req.runtime_approval_id.as_deref(), Some("request_snapshot"));
        assert_eq!(req.prompt, "Review this exact action");
    }
    #[test]
    fn malformed_or_foreign_pending_snapshots_are_not_recovered() {
        for (pointer, value) in [
            ("/run_id", json!("run_foreign")),
            ("/session_id", json!("foreign")),
            ("/status", json!("running")),
            ("/approval/event", json!("tool.result")),
            ("/approval/run_id", json!("run_foreign")),
            ("/approval/request_id", json!("")),
            ("/approval/request_id", Value::Null),
            ("/approval/request_id", json!("x".repeat(257))),
            ("/approval/choices", json!(["always"])),
            ("/approval/description", json!("x".repeat(16_385))),
        ] {
            let mut bad = payload();
            *bad.pointer_mut(pointer).unwrap() = value;
            assert!(request(&bad, &run()).is_err(), "{pointer}");
        }
    }
    #[test]
    fn missing_identity_and_oversized_detail_cannot_use_nested_fallbacks() {
        let mut bad = payload();
        bad["approval"]
            .as_object_mut()
            .unwrap()
            .remove("request_id");
        bad["approval"]["nested"] = json!({"request_id":"other"});
        assert!(request(&bad, &run()).is_err());
        let mut bad = payload();
        bad["approval"]["extra"] = json!("x".repeat(65_536));
        assert!(request(&bad, &run()).is_err());
    }
}
