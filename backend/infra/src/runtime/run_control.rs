//! Run controls acknowledge native effects; they never prove task completion or safe OS stop.
use super::*;
use reqwest::{StatusCode, header};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Operation {
    Steer,
    Stop,
}

impl Operation {
    fn endpoint(self) -> &'static str {
        match self {
            Self::Steer => "steer",
            Self::Stop => "stop",
        }
    }
    fn domain(self) -> domain::RuntimeControlOperation {
        match self {
            Self::Steer => domain::RuntimeControlOperation::Steer,
            Self::Stop => domain::RuntimeControlOperation::Stop,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub(super) enum Acknowledgement {
    Steered,
    Stopping,
    AlreadyTerminal,
}

struct PreparedControl {
    base: String,
    token: String,
    run_id: String,
    session_id: String,
    outcome: Option<control_outcome_wire::Context>,
}

pub(super) async fn send<T: Serialize + ?Sized>(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    run: &SessionAgentRun,
    operation: Operation,
    body: Option<&T>,
    actor: &domain::RuntimeControlActor,
    input: Option<&str>,
) -> Result<domain::RuntimeControlReceipt, AppError> {
    if agent.kind != AgentKind::Hermes || agent.id != run.agent_id {
        return Err(AppError::conflict("runtime control agent changed"));
    }
    let reservation = supervisor
        .repo
        .reserve_runtime_control(run, actor, operation.domain(), input)
        .await?;
    let id = reservation.receipt.id;
    if !reservation.dispatch {
        return Ok(reservation.receipt);
    }
    let prepared = match prepare(supervisor, agent, run, operation, id, input).await {
        Ok(prepared) => prepared,
        Err(error) => {
            let receipt = supervisor.repo.retire_runtime_control(id, false).await?;
            if receipt.state != domain::RuntimeControlState::Rejected {
                return Ok(receipt);
            }
            return Err(error);
        }
    };
    let outcome = prepared
        .outcome
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(AppError::internal)?;
    let claim = match &outcome {
        Some(context) => {
            supervisor
                .repo
                .claim_runtime_control_outcome(id, context.clone())
                .await
        }
        None => supervisor.repo.claim_runtime_control(id).await,
    };
    let claimed = match claim {
        Ok(claimed) => claimed,
        Err(error) => {
            // A concurrent claimant may already own the effect; never reset its submitted state.
            let receipt = supervisor.repo.retire_runtime_control(id, false).await?;
            if receipt.state != domain::RuntimeControlState::Rejected {
                return Ok(receipt);
            }
            return Err(error);
        }
    };
    if !claimed {
        return supervisor
            .repo
            .get_runtime_control(run.session_id, id)
            .await;
    }
    let ack = if let Some(context) = &prepared.outcome {
        control_outcome_wire::send(&supervisor.client, context, &prepared.base, &prepared.token)
            .await
            .and_then(|ack| match ack {
                control_outcome_wire::Acknowledgement::Steered => Ok(Acknowledgement::Steered),
                control_outcome_wire::Acknowledgement::Stopping => Ok(Acknowledgement::Stopping),
                control_outcome_wire::Acknowledgement::ApprovalResolved => {
                    Err(AppError::conflict("unexpected control ACK"))
                }
            })
    } else {
        post(supervisor, &prepared, operation, body).await
    };
    match ack {
        Ok(ack) => {
            let ack = match ack {
                Acknowledgement::Steered => "steered",
                Acknowledgement::Stopping => "stopping",
                Acknowledgement::AlreadyTerminal => "already_terminal",
            };
            if let Some(context) = outcome {
                match supervisor
                    .repo
                    .finish_runtime_control_outcome(id, context, ack)
                    .await
                {
                    Ok(receipt) => Ok(receipt),
                    // A failed DB ACK commit must not turn into another POST on caller retry.
                    Err(_) => supervisor.repo.retire_runtime_control(id, true).await,
                }
            } else {
                supervisor.repo.finish_runtime_control(id, ack).await
            }
        }
        Err(_) => supervisor.repo.retire_runtime_control(id, true).await,
    }
}

async fn prepare(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    run: &SessionAgentRun,
    operation: Operation,
    command_id: Uuid,
    input: Option<&str>,
) -> Result<PreparedControl, AppError> {
    let context = native_context::accepted(supervisor, agent, run).await?;
    let current = context.run;
    if !matches!(
        current.state,
        SessionRunState::Running | SessionRunState::Waiting | SessionRunState::Stopping
    ) || (operation == Operation::Steer && current.state != SessionRunState::Running)
    {
        return Err(AppError::conflict(
            "runtime run is not accepting this control",
        ));
    }
    let run_id = current
        .runtime_run_id
        .as_deref()
        .filter(|id| crate::pm_execution::valid_hermes_ref(id))
        .ok_or_else(|| AppError::conflict("verified native run identity is required"))?;
    let session_id = current
        .runtime_session_id
        .as_deref()
        .filter(|id| domain::valid_ref(id, 512))
        .ok_or_else(|| AppError::conflict("verified native session identity is required"))?;
    let base = context.base;
    let token = context.token;
    let capabilities = supervisor.probe_hermes(agent).await?;
    capability(&capabilities, operation)?;
    let status = hermes_wire::read_accepted_run(&supervisor.client, &base, &token, run_id).await?;
    if hermes_wire::effective_session(&status, run_id)? != session_id {
        return Err(AppError::Unavailable(
            "native control session changed".into(),
        ));
    }
    let active = match operation {
        Operation::Steer => status["status"] == "running",
        Operation::Stop => matches!(
            status["status"].as_str(),
            Some("queued" | "started" | "running" | "waiting_for_approval" | "stopping")
        ),
    };
    if !active {
        return Err(AppError::conflict(
            "native run is no longer accepting this control; await readback",
        ));
    }
    let outcome = if supervisor.config.fleet.hermes_control_outcome_enabled {
        let request = match operation {
            Operation::Steer => control_outcome_wire::Request::Steer(
                input.ok_or_else(|| AppError::validation("steer input is required"))?,
            ),
            Operation::Stop => control_outcome_wire::Request::Stop,
        };
        Some(
            control_outcome_wire::prepare(
                &supervisor.client,
                &base,
                &token,
                command_id,
                run_id,
                request,
            )
            .await?,
        )
    } else {
        None
    };
    Ok(PreparedControl {
        base,
        token,
        run_id: run_id.to_owned(),
        session_id: session_id.to_owned(),
        outcome,
    })
}

async fn post<T: Serialize + ?Sized>(
    supervisor: &LocalRuntimeSupervisor,
    prepared: &PreparedControl,
    operation: Operation,
    body: Option<&T>,
) -> Result<Acknowledgement, AppError> {
    // No retry: a lost acknowledgement may already have applied the guidance/interrupt.
    let mut request = supervisor
        .client
        .post(format!(
            "{}/v1/runs/{}/{}",
            prepared.base,
            prepared.run_id,
            operation.endpoint()
        ))
        .bearer_auth(&prepared.token)
        .header(header::ACCEPT_ENCODING, "identity")
        .timeout(Duration::from_secs(10));
    if let Some(body) = body {
        request = request.json(body);
    }
    let response = request.send().await.map_err(|_| {
        AppError::Unavailable(
            "runtime control acceptance is unknown; do not automatically retry".into(),
        )
    })?;
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
            "runtime control acknowledgement MIME does not match".into(),
        ));
    }
    let payload = hermes_wire::read_json(response, StatusCode::OK, 64 * 1024).await?;
    validate_ack(&payload, &prepared.run_id, &prepared.session_id, operation)
}

fn capability(payload: &Value, operation: Operation) -> Result<(), AppError> {
    hermes_wire::task_protocol(payload)?;
    let feature = format!("run_{}", operation.endpoint());
    if payload["features"][&feature] != true
        || payload["endpoints"][&feature]["method"] != "POST"
        || payload["endpoints"][&feature]["path"]
            != format!("/v1/runs/{{run_id}}/{}", operation.endpoint())
    {
        return Err(AppError::Unavailable(
            "native control capability is not verified".into(),
        ));
    }
    Ok(())
}

fn validate_ack(
    payload: &Value,
    run_id: &str,
    session_id: &str,
    operation: Operation,
) -> Result<Acknowledgement, AppError> {
    if payload["run_id"].as_str() != Some(run_id) {
        return Err(AppError::Unavailable(
            "runtime control acknowledgement identifies another run".into(),
        ));
    }
    match operation {
        Operation::Steer
            if payload["object"] == "hermes.run.steer" && payload["accepted"] == true =>
        {
            Ok(Acknowledgement::Steered)
        }
        Operation::Stop
            if payload["status"] == "stopping"
                && payload
                    .get("session_id")
                    .is_none_or(|id| id.as_str() == Some(session_id)) =>
        {
            Ok(Acknowledgement::Stopping)
        }
        Operation::Stop
            if hermes_wire::terminal_readback(payload, run_id).is_ok()
                && hermes_wire::effective_session(payload, run_id)
                    .is_ok_and(|id| id == session_id) =>
        {
            Ok(Acknowledgement::AlreadyTerminal)
        }
        _ => Err(AppError::Unavailable(
            "runtime control acknowledgement does not prove acceptance".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn steer_requires_exact_native_ack_and_true_boolean() {
        let good = json!({"object":"hermes.run.steer","run_id":"run_a","accepted":true});
        assert_eq!(
            validate_ack(&good, "run_a", "session_a", Operation::Steer).unwrap(),
            Acknowledgement::Steered
        );
        for (field, value) in [
            ("object", json!("hermes.run")),
            ("run_id", json!("run_b")),
            ("run_id", Value::Null),
            ("accepted", json!(false)),
            ("accepted", json!("true")),
            ("accepted", Value::Null),
        ] {
            let mut bad = good.clone();
            bad[field] = value;
            assert!(validate_ack(&bad, "run_a", "session_a", Operation::Steer).is_err());
        }
    }
    #[test]
    fn stop_ack_is_not_terminal_or_capacity_release() {
        let ack = json!({"run_id":"run_a","status":"stopping"});
        assert_eq!(
            validate_ack(&ack, "run_a", "session_a", Operation::Stop).unwrap(),
            Acknowledgement::Stopping
        );
        for bad in [
            json!({"accepted":true}),
            json!({"run_id":"run_b","status":"stopping"}),
            json!({"run_id":"run_a","status":"running"}),
            json!({"run_id":"run_a","status":"stopping","session_id":"foreign"}),
            json!({"run_id":"run_a","status":"completed"}),
        ] {
            assert!(validate_ack(&bad, "run_a", "session_a", Operation::Stop).is_err());
        }
    }
    #[test]
    fn terminal_stop_race_requires_full_pinned_native_status() {
        let good = json!({"object":"hermes.run","run_id":"run_a","session_id":"session_a",
            "status":"completed","completed":true,"partial":false,"interrupted":false});
        assert_eq!(
            validate_ack(&good, "run_a", "session_a", Operation::Stop).unwrap(),
            Acknowledgement::AlreadyTerminal
        );
        for (field, value) in [
            ("session_id", json!("session_b")),
            ("completed", json!(false)),
            ("partial", json!(true)),
            ("interrupted", json!(true)),
            ("object", Value::Null),
        ] {
            let mut bad = good.clone();
            bad[field] = value;
            assert!(validate_ack(&bad, "run_a", "session_a", Operation::Stop).is_err());
        }
    }
}
