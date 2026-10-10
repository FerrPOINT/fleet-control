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
    let pm = supervisor
        .repo
        .get_task_chat_binding(run.session_id)
        .await?
        .is_some();
    let scope = if pm {
        Some(
            prepare_pm(supervisor, agent, run, operation, actor.user_id)
                .await?
                .1,
        )
    } else {
        None
    };
    let reservation = if let Some(scope) = &scope {
        supervisor
            .repo
            .reserve_pm_runtime_control(run, actor, operation.domain(), input, scope)
            .await?
    } else {
        supervisor
            .repo
            .reserve_runtime_control(run, actor, operation.domain(), input)
            .await?
    };
    let id = reservation.receipt.id;
    if !reservation.dispatch {
        if operation == Operation::Steer
            && reservation.receipt.state == domain::RuntimeControlState::Acknowledged
        {
            return supervisor
                .repo
                .finish_runtime_control(id, "steered", input)
                .await;
        }
        return Ok(reservation.receipt);
    }
    let preparation = if scope.is_some() {
        prepare_pm(supervisor, agent, run, operation, actor.user_id)
            .await
            .map(|(prepared, fresh)| (prepared, Some(fresh)))
    } else {
        prepare(supervisor, agent, run, operation)
            .await
            .map(|prepared| (prepared, None))
    };
    let (prepared, scope) = match preparation {
        Ok(prepared) => prepared,
        Err(error) => {
            let receipt = supervisor.repo.retire_runtime_control(id, false).await?;
            if receipt.state != domain::RuntimeControlState::Rejected {
                return Ok(receipt);
            }
            return Err(error);
        }
    };
    let claim = if let Some(scope) = &scope {
        supervisor.repo.claim_pm_runtime_control(id, scope).await
    } else {
        supervisor.repo.claim_runtime_control(id).await
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
    let ack = post(supervisor, &prepared, operation, body).await;
    match ack {
        Ok(ack) => {
            supervisor
                .repo
                .finish_runtime_control(
                    id,
                    match ack {
                        Acknowledgement::Steered => "steered",
                        Acknowledgement::Stopping => "stopping",
                        Acknowledgement::AlreadyTerminal => "already_terminal",
                    },
                    input,
                )
                .await
        }
        Err(_) => supervisor.repo.retire_runtime_control(id, true).await,
    }
}

pub(super) async fn pm_human_controls(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    run: &SessionAgentRun,
    owner: Uuid,
) -> Result<domain::PmHumanControls, AppError> {
    let receipts = supervisor
        .repo
        .list_runtime_controls(run.session_id, run.id)
        .await?;
    if receipts.iter().any(|receipt| {
        matches!(
            receipt.state,
            domain::RuntimeControlState::Reserved
                | domain::RuntimeControlState::Submitted
                | domain::RuntimeControlState::Uncertain
        ) || (receipt.operation == domain::RuntimeControlOperation::Stop
            && receipt.state == domain::RuntimeControlState::Acknowledged)
    }) {
        return Ok(domain::PmHumanControls::default());
    }
    let can_stop = prepare_pm(supervisor, agent, run, Operation::Stop, owner)
        .await
        .is_ok();
    let can_steer = prepare_pm(supervisor, agent, run, Operation::Steer, owner)
        .await
        .is_ok();
    Ok(domain::PmHumanControls {
        can_steer,
        can_stop,
    })
}

async fn prepare_pm(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    expected: &SessionAgentRun,
    operation: Operation,
    owner: Uuid,
) -> Result<(PreparedControl, domain::PmHumanControlScope), AppError> {
    use super::{pm_dispatch, pm_tools};
    use crate::pm_credentials::PmCredentialCoordinator;
    use domain::*;
    let unavailable = || AppError::Unavailable("PM human control authority is unavailable".into());
    let session = supervisor.repo.get_session(expected.session_id).await?;
    let user = supervisor
        .repo
        .find_user_by_id(owner)
        .await?
        .ok_or(AppError::Forbidden)?;
    let binding = supervisor
        .repo
        .get_task_chat_binding(session.id)
        .await?
        .ok_or(AppError::Forbidden)?;
    let pm = supervisor
        .repo
        .read_pm_operation_for_session(session.id, owner)
        .await
        .map_err(|error| match error {
            AppError::NotFound { .. } => {
                AppError::conflict("task control admission is not verified")
            }
            error => error,
        })?;
    let record = supervisor
        .repo
        .get_pm_run(expected.id)
        .await
        .map_err(|error| match error {
            AppError::NotFound { .. } => {
                AppError::conflict("task control admission is not verified")
            }
            error => error,
        })?;
    let current = supervisor.repo.get_session_agent_run(expected.id).await?;
    let current_agent = supervisor.repo.get_agent(agent.id).await?;
    if session.user_id != owner
        || !user.is_active
        || pm.owner_subject != binding.owner_subject
        || !pm_tools::binding_matches(&binding, &pm.identity()?, pm.request.agent_id)
        || session.primary_agent_id != agent.id
        || session.agent_id != agent.id
        || session.state != SessionState::Active
        || current_agent.kind != AgentKind::Hermes
        || current_agent.sdlc_role != Some(SdlcRole::ProjectManager)
        || current_agent.status != AgentStatus::Running
        || pm.request.agent_id != agent.id
        || record.reservation.identity != pm.execution_identity()?
        || record.reservation.session_id != session.id
        || record.terminal_status.is_some()
        || current.session_id != session.id
        || current.agent_id != agent.id
        || current.runtime_run_id != expected.runtime_run_id
        || current.runtime_session_id != expected.runtime_session_id
        || current.runtime_run_id != record.hermes_run_ref
        || current.runtime_session_id.as_deref()
            != Some(record.reservation.runtime_session_id().as_str())
        || !matches!(
            current.state,
            SessionRunState::Running | SessionRunState::Waiting | SessionRunState::Stopping
        )
    {
        return Err(AppError::Forbidden);
    }
    crate::pm_tool_config::verify(supervisor.repo.as_ref(), &current_agent, &supervisor.config)
        .await?;
    let intent = supervisor
        .repo
        .get_pm_dispatch(expected.id)
        .await?
        .ok_or_else(unavailable)?;
    if !intent.submitted
        || intent.hermes_run_ref != record.hermes_run_ref
        || record.hermes_run_ref.is_none()
    {
        return Err(unavailable());
    }
    let workflow = pm_dispatch::Workflow::configured(supervisor)?;
    workflow.verify_intent(&intent)?;
    let coordinator =
        PmCredentialCoordinator::configured(&supervisor.config)?.ok_or_else(unavailable)?;
    let credential = coordinator.runtime_credential(&pm).await?;
    let context = coordinator.machine_context(&pm, &credential).await?;
    let (snapshot, _) = pm_tools::read_workflow(&workflow, &record.reservation.identity).await?;
    if snapshot.session_run_id != expected.id
        || snapshot.identity != record.reservation.identity
        || snapshot.binding_ref != record.reservation.binding_ref
        || snapshot.fence != record.reservation.fence
        || Some(snapshot.hermes_run_ref.as_str()) != record.hermes_run_ref.as_deref()
        || (operation == Operation::Steer
            && (snapshot.state != "active"
                || !snapshot.workflow_step_allowed
                || current.state != SessionRunState::Running
                || !matches!(
                    context.stage,
                    TrackerStage::Draft | TrackerStage::Clarification
                )
                || context.waiting_reason.as_deref() == Some("waiting_for_owner_confirmation")))
    {
        return Err(AppError::conflict(
            "PM control requires the current Workflow run and fence",
        ));
    }
    if operation == Operation::Steer {
        let questions: TrackerClarifications = pm_dispatch::decode(
            coordinator
                .tracker_call(&credential, reqwest::Method::GET, "clarifications", None)
                .await?,
        )?;
        if questions
            .questions
            .iter()
            .any(|q| matches!(q.state, TrackerQuestionState::Open))
        {
            return Err(AppError::conflict(
                "PM clarification requires the structured owner answer",
            ));
        }
    }
    let base = pm_dispatch::verify_context(supervisor, &current_agent, &intent).await?;
    let token = crate::agent_runtime_token(&supervisor.config, agent.id)?;
    capability(&supervisor.probe_hermes(&current_agent).await?, operation)?;
    let run_id = record.hermes_run_ref.clone().ok_or_else(unavailable)?;
    let session_id = record.hermes_session_ref.clone().ok_or_else(unavailable)?;
    let native = hermes_wire::read_accepted_run(&supervisor.client, &base, &token, &run_id).await?;
    let status = native["status"].as_str();
    if hermes_wire::effective_session(&native, &run_id)? != session_id
        || match operation {
            Operation::Steer => status != Some("running"),
            Operation::Stop => !matches!(
                status,
                Some("queued" | "started" | "running" | "waiting_for_approval" | "stopping")
            ),
        }
    {
        return Err(AppError::conflict(
            "PM native run no longer accepts this control",
        ));
    }
    coordinator.machine_context(&pm, &credential).await?;
    pm_dispatch::verify_context(supervisor, &current_agent, &intent).await?;
    let scope = PmHumanControlScope {
        record,
        intent,
        owner_subject: binding.owner_subject,
        owner_user_id: owner,
    };
    supervisor
        .repo
        .check_pm_runtime_control(&current, owner, operation.domain(), &scope)
        .await?;
    Ok((
        PreparedControl {
            base,
            token,
            run_id,
            session_id,
        },
        scope,
    ))
}

async fn prepare(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    run: &SessionAgentRun,
    operation: Operation,
) -> Result<PreparedControl, AppError> {
    let current = supervisor.repo.get_session_agent_run(run.id).await?;
    let current_agent = supervisor.repo.get_agent(agent.id).await?;
    if current_agent.kind != AgentKind::Hermes
        || supervisor.hermes_base_url(&current_agent).await?
            != supervisor.hermes_base_url(agent).await?
        || agent.kind != AgentKind::Hermes
        || current.agent_id != agent.id
        || current.session_id != run.session_id
        || current.runtime_run_id != run.runtime_run_id
        || current.runtime_session_id != run.runtime_session_id
    {
        return Err(AppError::conflict("runtime control identity changed"));
    }
    if !matches!(
        current.state,
        SessionRunState::Running | SessionRunState::Waiting | SessionRunState::Stopping
    ) || (operation == Operation::Steer && current.state != SessionRunState::Running)
    {
        return Err(AppError::conflict(
            "runtime run is not accepting this control",
        ));
    }
    let session = supervisor.repo.get_session(current.session_id).await?;
    if session.primary_agent_id != agent.id
        || supervisor
            .repo
            .get_task_chat_binding(session.id)
            .await?
            .is_some()
    {
        return Err(AppError::Unavailable(
            "task control admission is not verified".into(),
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
    let base = supervisor.run_base_url(&current_agent, &current).await?;
    let token = crate::agent_runtime_token(&supervisor.config, agent.id)?;
    let intent = supervisor
        .repo
        .get_hermes_dispatch_intent_for_run(run.id)
        .await?
        .ok_or_else(|| {
            AppError::Unavailable("legacy run has no original control context".into())
        })?;
    hermes_wire::verify_intent(&intent, &base, &token)?;
    supervisor
        .verify_container_intent(&current_agent, &intent)
        .await?;
    if intent.state != "accepted"
        || !intent.submission_attempted
        || intent.run.id != current.id
        || intent.run.agent_id != current.agent_id
        || intent.run.session_id != current.session_id
        || intent.run.runtime_run_id != current.runtime_run_id
        || intent.run.runtime_session_id != current.runtime_session_id
    {
        return Err(AppError::conflict(
            "original runtime control context changed",
        ));
    }
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
    supervisor
        .verify_container_intent(&current_agent, &intent)
        .await?;
    Ok(PreparedControl {
        base,
        token,
        run_id: run_id.to_owned(),
        session_id: session_id.to_owned(),
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

pub(super) fn capability(payload: &Value, operation: Operation) -> Result<(), AppError> {
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

pub(super) fn validate_ack(
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
