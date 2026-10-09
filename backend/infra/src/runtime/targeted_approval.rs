use super::*;
use domain::{ApprovalChoice, RuntimeApprovalRequest};
use reqwest::{StatusCode, header};

pub(super) async fn resolve(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    run: &SessionAgentRun,
    approval: &RuntimeApprovalRequest,
    choice: ApprovalChoice,
) -> Result<(), AppError> {
    let request_id = approval
        .runtime_approval_id
        .as_deref()
        .filter(|id| domain::valid_ref(id, 256))
        .ok_or_else(|| AppError::conflict("exact runtime approval ID is required"))?;
    if agent.kind != AgentKind::Hermes
        || agent.id != run.agent_id
        || approval.agent_id != agent.id
        || approval.session_run_id != run.id
        || approval.session_id != run.session_id
        || run.runtime_run_id.as_deref() != Some(approval.runtime_run_id.as_str())
        || !crate::pm_execution::valid_hermes_ref(&approval.runtime_run_id)
    {
        return Err(AppError::conflict(
            "targeted approval runtime identity mismatch",
        ));
    }
    let context = native_context::accepted(supervisor, agent, run).await?;
    if !matches!(
        context.run.state,
        SessionRunState::Running | SessionRunState::Waiting
    ) {
        return Err(AppError::conflict(
            "runtime is no longer waiting for this approval",
        ));
    }
    let capabilities = supervisor.probe_hermes(agent).await?;
    verify_capability(&capabilities)?;
    let status = hermes_wire::read_accepted_run(
        &supervisor.client,
        &context.base,
        &context.token,
        &approval.runtime_run_id,
    )
    .await?;
    if Some(hermes_wire::effective_session(&status, &approval.runtime_run_id)?.as_str())
        != context.run.runtime_session_id.as_deref()
    {
        return Err(AppError::Unavailable(
            "native approval session changed".into(),
        ));
    }
    verify_pending(&status, &approval.runtime_run_id, request_id)?;
    // GET preflight may have waited; repeat fresh original-context checks before POST.
    let fresh = native_context::accepted(supervisor, agent, run).await?;
    if !matches!(
        fresh.run.state,
        SessionRunState::Running | SessionRunState::Waiting
    ) || fresh.base != context.base
        || fresh.token != context.token
    {
        return Err(AppError::conflict(
            "approval context changed during preflight",
        ));
    }
    // No retry: losing an exact-action ACK cannot authorize another decision.
    let response = supervisor
        .client
        .post(format!(
            "{}/v1/runs/{}/approval",
            context.base, approval.runtime_run_id
        ))
        .timeout(Duration::from_secs(10))
        .bearer_auth(&context.token)
        .header(header::ACCEPT_ENCODING, "identity")
        .json(&json!({"choice":choice,"request_id":request_id,"resolve_all":false}))
        .send()
        .await
        .map_err(|_| AppError::Unavailable("approval acceptance is unknown".into()))?;
    if !response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|mime| {
            mime.split(';')
                .next()
                .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
        })
    {
        return Err(AppError::Unavailable(
            "approval acknowledgement MIME does not match".into(),
        ));
    }
    let payload = hermes_wire::read_json(response, StatusCode::OK, 64 * 1024).await?;
    validate_ack(&payload, &approval.runtime_run_id, request_id, choice)
}

pub(super) fn verify_capability(payload: &Value) -> Result<(), AppError> {
    hermes_wire::task_protocol(payload)?;
    if payload["features"]["run_approval_response"] != true
        || payload["features"]["approval_events"] != true
        || payload["endpoints"]["run_approval"]["method"] != "POST"
        || payload["endpoints"]["run_approval"]["path"] != "/v1/runs/{run_id}/approval"
    {
        return Err(AppError::Unavailable(
            "native targeted approval capability is not verified".into(),
        ));
    }
    Ok(())
}

fn verify_pending(payload: &Value, run_id: &str, request_id: &str) -> Result<(), AppError> {
    if payload["status"] != "waiting_for_approval"
        || payload["approval"]["event"] != "approval.request"
        || payload["approval"]["run_id"].as_str() != Some(run_id)
        || payload["approval"]["request_id"].as_str() != Some(request_id)
    {
        return Err(AppError::conflict(
            "native run is not waiting for this exact approval",
        ));
    }
    Ok(())
}

fn validate_ack(
    payload: &Value,
    run_id: &str,
    request_id: &str,
    choice: ApprovalChoice,
) -> Result<(), AppError> {
    if payload.get("object").and_then(Value::as_str) != Some("hermes.run.approval_response")
        || payload.get("run_id").and_then(Value::as_str) != Some(run_id)
        || payload.get("request_id").and_then(Value::as_str) != Some(request_id)
        || payload.get("choice").and_then(Value::as_str) != Some(choice.as_str())
        || payload.get("resolved").and_then(Value::as_u64) != Some(1)
    {
        return Err(AppError::Unavailable(
            "approval acknowledgement does not match the decision".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_approval_must_match_the_original_request_and_run() {
        let good = json!({"status":"waiting_for_approval","approval":{"event":"approval.request",
            "run_id":"run_a","request_id":"request_a"}});
        assert!(verify_pending(&good, "run_a", "request_a").is_ok());
        for (path, value) in [
            ("/status", json!("running")),
            ("/status", json!("completed")),
            ("/approval/event", json!("tool.result")),
            ("/approval/run_id", json!("run_b")),
            ("/approval/request_id", json!("request_b")),
            ("/approval/request_id", Value::Null),
        ] {
            let mut bad = good.clone();
            *bad.pointer_mut(path).unwrap() = value;
            assert!(
                verify_pending(&bad, "run_a", "request_a").is_err(),
                "{path}"
            );
        }
    }
    #[test]
    fn acknowledgement_must_identify_exactly_one_requested_action() {
        let ack = json!({"object":"hermes.run.approval_response","run_id":"run_a","request_id":"approval_a","choice":"once","resolved":1});
        assert!(validate_ack(&ack, "run_a", "approval_a", ApprovalChoice::Once).is_ok());
        for (field, value) in [
            ("object", json!("hermes.run")),
            ("run_id", json!("run_b")),
            ("request_id", json!("approval_b")),
            ("choice", json!("always")),
            ("resolved", json!(2)),
            ("resolved", json!(0)),
        ] {
            let mut wrong = ack.clone();
            wrong[field] = value;
            assert!(validate_ack(&wrong, "run_a", "approval_a", ApprovalChoice::Once).is_err());
        }
    }
}
