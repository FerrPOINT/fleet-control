use super::*;
use domain::{ApprovalChoice, RuntimeApprovalRequest};

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
    let response = supervisor
        .client
        .post(format!(
            "{}/v1/runs/{}/approval",
            LocalRuntimeSupervisor::hermes_base_url(agent)?,
            approval.runtime_run_id
        ))
        .timeout(Duration::from_secs(10))
        .bearer_auth(crate::agent_runtime_token(&supervisor.config, agent.id)?)
        .json(&json!({"choice":choice,"request_id":request_id,"resolve_all":false}))
        .send()
        .await
        .map_err(|_| AppError::Unavailable("approval acceptance is unknown".into()))?;
    if !response.status().is_success() {
        return Err(AppError::Unavailable(
            "approval was not acknowledged".into(),
        ));
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| {
            AppError::Unavailable("approval acknowledgement was interrupted".into())
        })?;
        if body.len().saturating_add(chunk.len()) > 64 * 1024 {
            return Err(AppError::Unavailable(
                "approval acknowledgement exceeded its limit".into(),
            ));
        }
        body.extend_from_slice(&chunk);
    }
    let payload: Value = serde_json::from_slice(&body)
        .map_err(|_| AppError::Unavailable("approval acknowledgement is malformed".into()))?;
    validate_ack(&payload, &approval.runtime_run_id, request_id, choice)
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
