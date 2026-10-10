//! Continuation uses the saved human command, Workflow ledger and native custody.
//! A stop ACK never releases capacity; only the existing verified readback does.
use super::pm_dispatch::{Workflow, decode, unavailable};
use super::*;
use crate::pm_credentials::PmCredentialCoordinator;
use domain::*;

fn command(snapshot: &PmWorkflowSnapshot, key: &str) -> Result<Value, AppError> {
    let mut body = serde_json::to_value(&snapshot.identity).map_err(|_| unavailable())?;
    body.as_object_mut().ok_or_else(unavailable)?.extend(
        json!({"operation_key":key,
        "expected_version":snapshot.version,"expected_fence":snapshot.fence,
        "binding_ref":snapshot.binding_ref,"hermes_run_ref":snapshot.hermes_run_ref,
        "session_run_id":snapshot.session_run_id})
        .as_object()
        .unwrap()
        .clone(),
    );
    Ok(body)
}

fn checkpoint_matches(
    snapshot: &PmWorkflowSnapshot,
    question: &TrackerQuestion,
) -> Result<(), AppError> {
    let data = snapshot.checkpoint.as_ref().ok_or_else(unavailable)?;
    if data.checkpoint_ref != question.checkpoint_id.to_string()
        || data.clarification_request_ref != question.request_id.to_string()
        || data.clarification_version != question.version
        || data.requirements_revision != question.requirement_revision
    {
        return Err(AppError::conflict(
            "saved answer belongs to another PM checkpoint",
        ));
    }
    Ok(())
}

pub(super) async fn resume(
    supervisor: &LocalRuntimeSupervisor,
    actor: &ClarificationCommandActor,
    receipt: &ClarificationAnswerCommand,
) -> Result<(), AppError> {
    if !supervisor.config.pm.dispatch.enabled {
        return Ok(());
    }
    // Reload custody; neither HTTP caller nor a model can supply the delivery proof.
    let saved = supervisor
        .repo
        .get_clarification_command(actor, receipt.id)
        .await?;
    if saved.state != ClarificationDeliveryState::Delivered {
        return Ok(());
    }
    let answer = saved
        .answer
        .as_ref()
        .filter(|a| answer_matches_command(&saved, a, &actor.subject))
        .ok_or_else(unavailable)?;
    let operation = match supervisor
        .repo
        .read_pm_operation_for_session(actor.session_id, actor.user_id)
        .await
    {
        Ok(operation) => operation,
        Err(AppError::NotFound { .. }) => return Ok(()),
        Err(error) => return Err(error),
    };
    if operation.identity()? != actor.binding
        || operation.owner_subject != actor.subject
        || saved.session_id != actor.session_id
    {
        return Err(AppError::Forbidden);
    }
    let session = supervisor.repo.get_session(actor.session_id).await?;
    if session.user_id != actor.user_id
        || session.primary_agent_id != operation.request.agent_id
        || session.agent_id != operation.request.agent_id
        || !matches!(session.state, SessionState::Active)
    {
        return Err(AppError::Forbidden);
    }
    let agent = supervisor
        .repo
        .get_agent(operation.request.agent_id)
        .await?;
    if agent.kind != AgentKind::Hermes
        || agent.sdlc_role != Some(SdlcRole::ProjectManager)
        || agent.status != AgentStatus::Running
    {
        return Err(AppError::Forbidden);
    }
    crate::pm_tool_config::verify(supervisor.repo.as_ref(), &agent, &supervisor.config).await?;
    let coordinator =
        PmCredentialCoordinator::configured(&supervisor.config)?.ok_or_else(unavailable)?;
    let credential = coordinator.runtime_credential(&operation).await?;
    let context = coordinator.machine_context(&operation, &credential).await?;
    let questions: TrackerClarifications = decode(
        coordinator
            .tracker_call(&credential, reqwest::Method::GET, "clarifications", None)
            .await?,
    )?;
    let question = questions
        .questions
        .into_iter()
        .find(|q| q.id == saved.question_id)
        .ok_or_else(unavailable)?;
    let assignment = &operation
        .reservation
        .as_ref()
        .ok_or_else(unavailable)?
        .assignment;
    if !matches!(question.state, TrackerQuestionState::Answered)
        || serde_json::to_value(&question.answer).map_err(|_| unavailable())?
            != serde_json::to_value(Some(answer)).map_err(|_| unavailable())?
        || question.task_id != actor.binding.task_id
        || question.root_task_id != actor.binding.root_task_id
        || question.assignment_id != assignment.assignment_id
        || question.execution_id != assignment.execution_id
        || question.agent_id != assignment.agent_id
        || u64::try_from(question.assignment_version).ok() != Some(assignment.version)
        || question.author_subject != assignment.machine_subject
        || context.requirement_revision != Some(question.requirement_revision)
    {
        return Err(AppError::conflict(
            "saved answer or current Tracker assignment changed",
        ));
    }
    let workflow = Workflow::configured(supervisor)?;
    let (mut snapshot, _) =
        pm_tools::read_workflow(&workflow, &operation.execution_identity()?).await?;
    checkpoint_matches(&snapshot, &question)?;
    let key = format!("fleet-pm-resume:{}", saved.id);
    if snapshot.state == "active"
        && snapshot.resume_delivered
        && snapshot.resume_session_run_id == Some(saved.id)
        && snapshot.session_run_id == saved.id
        && snapshot.resume_operation_key.as_deref() == Some(&key)
    {
        let current = supervisor.repo.get_pm_run(saved.id).await?;
        if current.reservation.identity != snapshot.identity
            || current.reservation.fence != snapshot.fence
            || current.hermes_run_ref.as_deref() != Some(&snapshot.hermes_run_ref)
        {
            return Err(unavailable());
        }
        return Ok(());
    }
    if !matches!(snapshot.state.as_str(), "waiting" | "resume_pending") {
        return Err(AppError::conflict(
            "PM is not waiting for this saved answer",
        ));
    }
    let old = supervisor.repo.get_pm_run(snapshot.session_run_id).await?;
    if old.reservation.identity != snapshot.identity
        || old.reservation.binding_ref != snapshot.binding_ref
        || old.hermes_run_ref.as_deref() != Some(&snapshot.hermes_run_ref)
        || old.reservation.fence != snapshot.fence - i64::from(snapshot.state == "resume_pending")
    {
        return Err(unavailable());
    }
    let intent = supervisor
        .repo
        .get_pm_dispatch(old.reservation.session_run_id)
        .await?
        .ok_or_else(unavailable)?;
    workflow.verify_intent(&intent)?;
    pm_dispatch::verify_context(supervisor, &agent, &intent).await?;
    let status = supervisor.probe_pm_run(&agent, &old).await?;
    if !status.terminal() {
        stop_once(supervisor, &agent, &old, &intent, &key).await?;
    }
    let status = supervisor.probe_pm_run(&agent, &old).await?;
    if !status.terminal() {
        return Err(AppError::Unavailable(
            "PM stop is not yet terminal; repeat delivery readback with the same saved command"
                .into(),
        ));
    }
    supervisor
        .repo
        .observe_pm_run(old.reservation.session_run_id, status)
        .await?;
    if snapshot.state == "waiting" {
        let mut body = command(&snapshot, &key)?;
        body.as_object_mut().unwrap().extend(
            serde_json::to_value(snapshot.checkpoint.as_ref().unwrap())
                .map_err(|_| unavailable())?
                .as_object()
                .unwrap()
                .clone(),
        );
        body["answer_event_ref"] = json!(answer.id);
        body["new_session_run_id"] = json!(saved.id);
        let (pending, _) = pm_tools::response(
            workflow
                .call(
                    "/internal/runtime/v1/pm/resume",
                    workflow.assignment,
                    Some(body),
                    None,
                )
                .await?,
            &snapshot.identity,
            false,
        )?;
        if pending.state != "resume_pending"
            || pending.version != snapshot.version + 1
            || pending.fence != snapshot.fence + 1
            || pending.session_run_id != snapshot.session_run_id
            || pending.binding_ref != snapshot.binding_ref
            || pending.hermes_run_ref != snapshot.hermes_run_ref
            || pending.terminal_readback.is_none()
        {
            return Err(unavailable());
        }
        snapshot = pending;
    }
    checkpoint_matches(&snapshot, &question)?;
    if snapshot.resume_operation_key.as_deref() != Some(&key)
        || snapshot.resume_session_run_id != Some(saved.id)
    {
        return Err(AppError::conflict("another PM continuation is reserved"));
    }
    let proof = snapshot
        .terminal_readback
        .as_ref()
        .ok_or_else(unavailable)?;
    if proof.fence != old.reservation.fence
        || proof.dispatch_operation_key != old.reservation.dispatch_operation_key
        || proof.checkpoint_ref != old.reservation.checkpoint_ref
    {
        return Err(unavailable());
    }
    let reservation = PmRunReservation {
        session_id: actor.session_id,
        session_run_id: saved.id,
        identity: snapshot.identity.clone(),
        binding_ref: format!("fleet:pm:{}", saved.id),
        dispatch_operation_key: key.clone(),
        checkpoint_ref: Some(question.checkpoint_id.to_string()),
        fence: snapshot.fence,
    };
    // Re-read authorization before capacity reservation and again before the only native POST.
    coordinator.machine_context(&operation, &credential).await?;
    let record = supervisor.repo.reserve_pm_run(reservation.clone()).await?;
    let input = format!(
        "[Fleet PM continuation]\noperation_id={}\nsession_run_id={}\nUse only fleet_pm MCP tools. First call workflow_step with report null for current instructions. Read Tracker requirements and clarifications. The owner's answer is already saved in Tracker; do not publish an answer or confirm requirements. Original goal: {}\nOriginal description: {}\nSaved answer receipt: {}",
        operation.id,
        saved.id,
        operation.request.title,
        operation.request.description,
        serde_json::to_string(answer).map_err(|_| unavailable())?
    );
    crate::pm_tool_config::reject_server_secrets(&input, &supervisor.config)?;
    let next = PmDispatchIntent {
        session_run_id: saved.id,
        request_body: serde_json::to_string(
            &json!({"input":input,"session_id":reservation.runtime_session_id()}),
        )
        .map_err(|_| unavailable())?,
        submitted: false,
        hermes_run_ref: None,
        ..intent.clone()
    };
    let next = supervisor.repo.prepare_pm_dispatch(next).await?;
    let base = pm_dispatch::verify_context(supervisor, &agent, &next).await?;
    let token = crate::agent_runtime_token(&supervisor.config, agent.id)?;
    hermes_wire::task_protocol(&supervisor.probe_hermes(&agent).await?)?;
    let run_ref = if let Some(run) = next.hermes_run_ref {
        run
    } else {
        coordinator.machine_context(&operation, &credential).await?;
        crate::pm_tool_config::verify(supervisor.repo.as_ref(), &agent, &supervisor.config).await?;
        if next.submitted || !supervisor.repo.claim_pm_submission(saved.id).await? {
            return Err(AppError::Unavailable(
                "PM continuation acceptance is unknown; no automatic resubmission".into(),
            ));
        }
        let run = hermes_wire::submit(
            &supervisor.client,
            &base,
            &token,
            saved.id,
            &next.request_body,
            None,
        )
        .await?;
        supervisor
            .repo
            .record_pm_submission(saved.id, run.clone())
            .await?;
        run
    };
    let native =
        hermes_wire::read_accepted_run(&supervisor.client, &base, &token, &run_ref).await?;
    if record
        .hermes_run_ref
        .as_ref()
        .is_some_and(|r| r != &run_ref)
    {
        return Err(unavailable());
    }
    let accepted = supervisor
        .repo
        .accept_pm_run(
            saved.id,
            run_ref.clone(),
            hermes_wire::effective_session(&native, &run_ref)?,
        )
        .await?;
    let status = supervisor.probe_pm_run(&agent, &accepted).await?;
    supervisor.repo.observe_pm_run(saved.id, status).await?;
    if status != PmRuntimeStatus::Running {
        return Err(AppError::conflict(
            "PM continuation terminated before rebind; reconcile without redispatch",
        ));
    }
    coordinator.machine_context(&operation, &credential).await?;
    let current_intent = supervisor
        .repo
        .get_pm_dispatch(saved.id)
        .await?
        .ok_or_else(unavailable)?;
    pm_dispatch::verify_context(supervisor, &agent, &current_intent).await?;
    let mut body = command(&snapshot, &format!("fleet-pm-rebind:{}", saved.id))?;
    body.as_object_mut().unwrap().extend(json!({"checkpoint_ref":question.checkpoint_id,
        "resume_operation_key":key,"new_binding_ref":reservation.binding_ref,"new_hermes_run_ref":run_ref,"new_session_run_id":saved.id}).as_object().unwrap().clone());
    let (bound, _) = pm_tools::response(
        workflow
            .call(
                "/internal/runtime/v1/pm/rebind",
                workflow.assignment,
                Some(body),
                None,
            )
            .await?,
        &snapshot.identity,
        false,
    )?;
    if bound.state != "active"
        || bound.version != snapshot.version + 1
        || bound.fence != snapshot.fence
        || bound.session_run_id != saved.id
        || bound.binding_ref != reservation.binding_ref
        || bound.hermes_run_ref != run_ref
        || !bound.resume_delivered
        || !bound.workflow_step_allowed
        || bound.resume_operation_key.as_deref() != Some(&key)
        || bound.resume_session_run_id != Some(saved.id)
    {
        return Err(unavailable());
    }
    checkpoint_matches(&bound, &question)?;
    Ok(())
}

async fn stop_once(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    record: &PmRunRecord,
    intent: &PmDispatchIntent,
    key: &str,
) -> Result<(), AppError> {
    let run = record.hermes_run_ref.as_deref().ok_or_else(unavailable)?;
    let session = record
        .hermes_session_ref
        .as_deref()
        .ok_or_else(unavailable)?;
    let body = json!({"run_id":run,"session_id":session,"resume_operation_key":key});
    let journal = supervisor
        .repo
        .prepare_pm_tool(PmToolCommand {
            session_run_id: record.reservation.session_run_id,
            key: format!("stop:{key}"),
            kind: "stop".into(),
            request: body,
            result: None,
            attempted: false,
        })
        .await?;
    if journal.attempted || journal.result.is_some() {
        return Ok(());
    }
    let capabilities = supervisor.probe_hermes(agent).await?;
    run_control::capability(&capabilities, run_control::Operation::Stop)?;
    let base = pm_dispatch::verify_context(supervisor, agent, intent).await?;
    if !supervisor
        .repo
        .claim_pm_tool(journal.session_run_id, &journal.key)
        .await?
    {
        return Ok(());
    }
    let response = supervisor
        .client
        .post(format!("{base}/v1/runs/{run}/stop"))
        .bearer_auth(crate::agent_runtime_token(&supervisor.config, agent.id)?)
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|_| unavailable())?;
    let ack = hermes_wire::read_json(response, reqwest::StatusCode::OK, 65536).await?;
    run_control::validate_ack(&ack, run, session, run_control::Operation::Stop)?;
    // Store only a validated receipt, not native output/transcripts.
    supervisor
        .repo
        .finish_pm_tool(
            journal.session_run_id,
            &journal.key,
            json!({"stop_acknowledged":true}),
        )
        .await
}
