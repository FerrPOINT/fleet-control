use super::pm_dispatch::{Workflow, decode, unavailable};
use super::*;
use crate::pm_credentials::{PmCredentialCoordinator, PmDelegatedCredential};
use domain::*;
use serde::Deserialize;

struct Scope {
    operation: PmDraftOperation,
    record: PmRunRecord,
    agent: Agent,
    intent: PmDispatchIntent,
    coordinator: PmCredentialCoordinator,
    credential: PmDelegatedCredential,
    context: TrackerTaskContext,
}

async fn scope(
    supervisor: &LocalRuntimeSupervisor,
    agent_id: Uuid,
    call: &PmToolCall,
) -> Result<Scope, AppError> {
    let record = supervisor.repo.get_pm_run(call.session_run_id).await?;
    let session = supervisor
        .repo
        .get_session(record.reservation.session_id)
        .await?;
    let operation = supervisor
        .repo
        .read_pm_operation_for_session(session.id, session.user_id)
        .await?;
    let binding = supervisor
        .repo
        .get_task_chat_binding(session.id)
        .await?
        .ok_or(AppError::Forbidden)?;
    if operation.id != call.operation_id
        || record.reservation.identity != operation.execution_identity()?
        || session.primary_agent_id != agent_id
        || session.agent_id != agent_id
        || !matches!(session.state, SessionState::Active)
        || binding != operation.identity()?
        || record.terminal_status.is_some()
    {
        return Err(AppError::Forbidden);
    }
    let agent = supervisor.repo.get_agent(agent_id).await?;
    if agent.kind != AgentKind::Hermes
        || agent.status != AgentStatus::Running
        || agent.sdlc_role != Some(SdlcRole::ProjectManager)
    {
        return Err(AppError::Forbidden);
    }
    crate::pm_tool_config::verify(supervisor.repo.as_ref(), &agent, &supervisor.config).await?;
    let intent = supervisor
        .repo
        .get_pm_dispatch(call.session_run_id)
        .await?
        .ok_or_else(unavailable)?;
    if !intent.submitted
        || intent.hermes_run_ref != record.hermes_run_ref
        || record.hermes_run_ref.is_none()
    {
        return Err(AppError::Unavailable(
            "PM native acceptance is not yet bound".into(),
        ));
    }
    pm_dispatch::verify_context(supervisor, &agent, &intent).await?;
    Workflow::configured(supervisor)?.verify_intent(&intent)?;
    if supervisor.probe_pm_run(&agent, &record).await? != PmRuntimeStatus::Running {
        return Err(AppError::conflict(
            "PM tools require the current nonterminal run",
        ));
    }
    let coordinator =
        PmCredentialCoordinator::configured(&supervisor.config)?.ok_or_else(unavailable)?;
    let credential = coordinator.runtime_credential(&operation).await?;
    let context = coordinator.machine_context(&operation, &credential).await?;
    Ok(Scope {
        operation,
        record,
        agent,
        intent,
        coordinator,
        credential,
        context,
    })
}

pub(super) fn response(
    value: Value,
    identity: &PmExecutionIdentity,
    readback: bool,
) -> Result<(PmWorkflowSnapshot, Option<String>), AppError> {
    let map = value.as_object().ok_or_else(unavailable)?;
    if value["ok"] != true
        || !matches!(map.len(), 2 | 3)
        || map
            .keys()
            .any(|k| !matches!(k.as_str(), "ok" | "result" | "execution_token"))
    {
        return Err(unavailable());
    }
    let mut result = value["result"].clone();
    if readback
        && result
            .as_object_mut()
            .ok_or_else(unavailable)?
            .remove("operation")
            != Some(Value::Null)
    {
        return Err(unavailable());
    }
    let snapshot: PmWorkflowSnapshot = decode(result)?;
    if snapshot.contract_version != 1
        || &snapshot.identity != identity
        || snapshot.version < 1
        || snapshot.fence < 1
        || snapshot.session_run_id.is_nil()
        || !crate::pm_execution::valid_hermes_ref(&snapshot.hermes_run_ref)
        || !valid_ref(&snapshot.binding_ref, 512)
        || !matches!(
            snapshot.state.as_str(),
            "active" | "waiting" | "resume_pending"
        )
        || snapshot.workflow_step_allowed && snapshot.state != "active"
        || snapshot.resume_delivered
            != (snapshot.state == "active" && snapshot.resume_operation_key.is_some())
        || snapshot.state != "active" && snapshot.checkpoint.is_none()
    {
        return Err(unavailable());
    }
    if let Some(checkpoint) = &snapshot.checkpoint {
        if !valid_ref(&checkpoint.checkpoint_ref, 512)
            || !valid_ref(&checkpoint.clarification_request_ref, 512)
            || !(1..=9_007_199_254_740_991).contains(&checkpoint.clarification_version)
            || !(1..=9_007_199_254_740_991).contains(&checkpoint.requirements_revision)
        {
            return Err(unavailable());
        }
    }
    if let Some(proof) = &snapshot.terminal_readback {
        if &proof.identity != identity
            || !proof.status.terminal()
            || proof.session_run_id != snapshot.session_run_id
            || proof.hermes_run_ref != snapshot.hermes_run_ref
            || proof.binding_ref != snapshot.binding_ref
        {
            return Err(unavailable());
        }
    }
    let token = match value.get("execution_token") {
        Some(Value::String(token))
            if snapshot.state == "active"
                && token.len() == 64
                && token
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
        {
            Some(token.clone())
        }
        None if snapshot.state != "active" => None,
        _ => return Err(unavailable()),
    };
    Ok((snapshot, token))
}

pub(super) async fn read_workflow(
    workflow: &Workflow<'_>,
    identity: &PmExecutionIdentity,
) -> Result<(PmWorkflowSnapshot, Option<String>), AppError> {
    response(
        workflow
            .call(
                "/internal/runtime/v1/pm/readback",
                workflow.assignment,
                Some(serde_json::to_value(identity).map_err(|_| unavailable())?),
                None,
            )
            .await?,
        identity,
        true,
    )
}

fn matches_run(snapshot: &PmWorkflowSnapshot, record: &PmRunRecord) -> Result<(), AppError> {
    if snapshot.session_run_id != record.reservation.session_run_id
        || snapshot.identity != record.reservation.identity
        || Some(snapshot.hermes_run_ref.as_str()) != record.hermes_run_ref.as_deref()
        || snapshot.binding_ref != record.reservation.binding_ref
        || snapshot.fence != record.reservation.fence
    {
        return Err(AppError::conflict("PM tool run is no longer current"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> (PmExecutionIdentity, Value) {
        let id = Uuid::new_v4();
        let identity = PmExecutionIdentity {
            task: "SDLC-1".into(),
            execution_ref: id.to_string(),
            tracker_instance_ref: "tracker".into(),
            tracker_project_ref: id.to_string(),
            task_ref: id.to_string(),
            root_ref: id.to_string(),
            agent_ref: id.to_string(),
            assignment_operation_key: "original-assignment".into(),
            assignment_ref: id.to_string(),
            assignment_revision: 1,
        };
        let value = json!({"ok":true,"execution_token":"a".repeat(64),"result":{"contract_version":1,"identity":identity,
            "state":"active","version":1,"fence":1,"session_run_id":id,"binding_ref":"binding","hermes_run_ref":"run_one",
            "checkpoint":null,"resume_operation_key":null,"resume_session_run_id":null,"terminal_readback":null,
            "workflow_step_allowed":true,"resume_delivered":false,"operation":null}});
        (identity, value)
    }

    #[test]
    fn workflow_readback_requires_closed_current_snapshot_and_keeps_token_in_memory() {
        let (identity, value) = snapshot();
        let (typed, token) = response(value.clone(), &identity, true).unwrap();
        assert_eq!(token, Some("a".repeat(64)));
        assert!(
            !serde_json::to_string(&typed)
                .unwrap()
                .contains("execution_token")
        );
        for pointer in [
            "/result/identity/agent_ref",
            "/result/contract_version",
            "/execution_token",
        ] {
            let mut bad = value.clone();
            *bad.pointer_mut(pointer).unwrap() = Value::Null;
            assert!(response(bad, &identity, true).is_err(), "{pointer}");
        }
        let mut bad = value;
        bad["result"]["operation"] = json!({});
        assert!(response(bad.clone(), &identity, true).is_err());
        bad["result"]["operation"] = Value::Null;
        bad["result"]["unknown"] = json!(true);
        assert!(response(bad, &identity, true).is_err());
    }

    #[test]
    fn waiting_snapshot_requires_exact_checkpoint_and_cannot_claim_step_or_resume() {
        let (identity, mut value) = snapshot();
        value.as_object_mut().unwrap().remove("execution_token");
        value["result"]["state"] = json!("waiting");
        value["result"]["version"] = json!(2);
        value["result"]["workflow_step_allowed"] = json!(false);
        assert!(response(value.clone(), &identity, true).is_err());
        value["result"]["checkpoint"] = json!({"checkpoint_ref":"checkpoint","clarification_request_ref":"request","clarification_version":1,"requirements_revision":1});
        assert!(response(value.clone(), &identity, true).is_ok());
        for field in ["workflow_step_allowed", "resume_delivered"] {
            let mut bad = value.clone();
            bad["result"][field] = json!(true);
            assert!(response(bad, &identity, true).is_err());
        }
    }
}

pub(super) async fn call(
    supervisor: &LocalRuntimeSupervisor,
    agent: Uuid,
    name: &str,
    call: PmToolCall,
) -> Result<Value, AppError> {
    let scope = scope(supervisor, agent, &call).await?;
    let workflow = Workflow::configured(supervisor)?;
    let (snapshot, token) = read_workflow(&workflow, &scope.record.reservation.identity).await?;
    matches_run(&snapshot, &scope.record)?;
    match name {
        "tracker_context" | "tracker_clarifications" | "tracker_requirements" => {
            if call.command != json!({}) {
                return Err(AppError::validation("read tool takes an empty command"));
            }
            match name {
                "tracker_context" => serde_json::to_value(scope.context).map_err(|_| unavailable()),
                "tracker_clarifications" => {
                    let result: TrackerClarifications = decode(
                        scope
                            .coordinator
                            .tracker_call(
                                &scope.credential,
                                reqwest::Method::GET,
                                "clarifications",
                                None,
                            )
                            .await?,
                    )?;
                    serde_json::to_value(result).map_err(|_| unavailable())
                }
                _ => {
                    let result: TrackerRequirements = decode(
                        scope
                            .coordinator
                            .tracker_call(
                                &scope.credential,
                                reqwest::Method::GET,
                                "requirements/revisions",
                                None,
                            )
                            .await?,
                    )?;
                    serde_json::to_value(result).map_err(|_| unavailable())
                }
            }
        }
        "tracker_publish_revision" => {
            if snapshot.state != "active" || !snapshot.workflow_step_allowed {
                return Err(AppError::conflict("PM is waiting for an answer"));
            }
            let request: PmPublishRevision = decode(call.command.clone())?;
            request.fence.verify(&scope.operation)?;
            if request
                .expected_requirement_revision
                .is_some_and(|v| !(1..9_007_199_254_740_991).contains(&v))
            {
                return Err(AppError::validation("invalid requirements version"));
            }
            publish_revision(supervisor, &scope, request, call.command).await
        }
        "tracker_publish_question" => {
            let request: PmPublishQuestion = decode(call.command.clone())?;
            request.fence.verify(&scope.operation)?;
            if !(1..=9_007_199_254_740_991).contains(&request.requirement_revision)
                || request
                    .expected_question_version
                    .is_some_and(|v| !(1..9_007_199_254_740_991).contains(&v))
            {
                return Err(AppError::validation("invalid clarification version"));
            }
            if scope.context.requirement_revision != Some(request.requirement_revision)
                || request.question_id.is_nil()
                || request.request_id.is_nil()
                || request.checkpoint_id.is_nil()
            {
                return Err(AppError::conflict(
                    "clarification revision or identity changed",
                ));
            }
            let question =
                publish_question(supervisor, &scope, &snapshot, request, call.command).await?;
            checkpoint(&workflow, &scope, &snapshot, token.as_deref(), &question).await?;
            serde_json::to_value(question).map_err(|_| unavailable())
        }
        "workflow_step" => {
            let request: PmWorkflowStep = decode(call.command.clone())?;
            domain::pm_tool_key(&request.step_operation_key)?;
            if request
                .report
                .as_ref()
                .is_some_and(|v| v.is_empty() || v.len() > 32768)
            {
                return Err(AppError::validation("PM report exceeds its bound"));
            }
            crate::pm_tool_config::reject_server_secrets(
                &call.command.to_string(),
                &supervisor.config,
            )?;
            let mut journal = supervisor
                .repo
                .get_pm_tool(call.session_run_id, &request.step_operation_key)
                .await?;
            let frozen: StoredWorkflowStep = if let Some(saved) = &journal {
                let frozen: StoredWorkflowStep = decode(saved.request.clone())?;
                if saved.kind != "workflow_step"
                    || frozen.caller != call.command
                    || scope.agent.workflow_id.as_deref()
                        != Some(frozen.workflow_id.to_string().as_str())
                {
                    return Err(AppError::conflict(
                        "PM step key has a different original report",
                    ));
                }
                if let Some(result) = &saved.result {
                    validate_step_receipt(result)?;
                    return Ok(result.clone());
                }
                frozen
            } else {
                if snapshot.state != "active" || !snapshot.workflow_step_allowed {
                    return Err(AppError::conflict("PM Workflow is waiting"));
                }
                let (workflow_id, mode_id, phase, status) =
                    workflow.cursor(&scope.intent, &scope.operation).await?;
                let mut body = json!({"task":snapshot.identity.task,"step_operation_key":request.step_operation_key,
                "assignment_revision":snapshot.identity.assignment_revision,"assignment_ref":snapshot.identity.assignment_ref,
                "binding_ref":snapshot.binding_ref,"hermes_run_ref":snapshot.hermes_run_ref,"mode_key":"draft",
                "cycle_number":0,"attempt_number":1,"expected_phase_code":phase,"expected_status":status,"session_run_id":snapshot.session_run_id});
                if let Some(report) = &request.report {
                    body["report"] = json!(report);
                }
                let frozen = StoredWorkflowStep {
                    caller: call.command,
                    body,
                    workflow_id,
                    mode_id,
                };
                if request.report.is_some() {
                    journal = Some(
                        prepare(
                            supervisor,
                            &scope,
                            "workflow_step",
                            &request.step_operation_key,
                            serde_json::to_value(&frozen).map_err(|_| unavailable())?,
                        )
                        .await?,
                    );
                }
                frozen
            };
            let workflow_id = frozen.workflow_id;
            let mode_id = frozen.mode_id;
            let phase = frozen.body["expected_phase_code"]
                .as_str()
                .ok_or_else(unavailable)?;
            let status = frozen.body["expected_status"]
                .as_str()
                .ok_or_else(unavailable)?;
            if scope.agent.workflow_id.as_deref() != Some(workflow_id.to_string().as_str()) {
                return Err(AppError::conflict("PM Workflow binding changed"));
            }
            if let Some(saved) = &journal {
                if !saved.attempted
                    && !supervisor
                        .repo
                        .claim_pm_tool(saved.session_run_id, &saved.key)
                        .await?
                {
                    let current = supervisor
                        .repo
                        .get_pm_tool(saved.session_run_id, &saved.key)
                        .await?
                        .ok_or_else(unavailable)?;
                    if !current.attempted || current.request != saved.request {
                        return Err(unavailable());
                    }
                }
            }
            // Workflow durably replays this exact report/key before validating its changed task cursor.
            let value = workflow
                .call(
                    "/internal/runtime/step",
                    workflow.runtime,
                    Some(frozen.body.clone()),
                    token.as_deref(),
                )
                .await?;
            if request.report.is_none() {
                let result: Instructions = decode(value)?;
                if !result.ok
                    || result.exit_code != 0
                    || !result.result.ok
                    || result.result.task_key != snapshot.identity.task
                    || result.result.phase_code != phase
                    || result.result.status != status
                    || result.result.workflow_id != workflow_id
                    || result.result.mode_id != mode_id
                    || result.result.mode_key != "draft"
                    || result.result.cycle_number != 0
                    || result.output != result.result.instructions
                    || result.result.instructions.is_empty()
                    || !result.result.phase_contract.is_object()
                {
                    return Err(unavailable());
                }
                serde_json::to_value(result).map_err(|_| unavailable())
            } else {
                let result: Evaluation = decode(value)?;
                if !matches!(result.exit_code, 0 | 1)
                    || result.ok != (result.exit_code == 0)
                    || !result.result.is_object()
                {
                    return Err(unavailable());
                }
                // A transport receipt is not a business completion claim. Supervisor owns its verdict.
                let receipt = json!({"supervisor_accepted":result.ok,"exit_code":result.exit_code});
                let saved = journal.ok_or_else(unavailable)?;
                supervisor
                    .repo
                    .finish_pm_tool(saved.session_run_id, &saved.key, receipt.clone())
                    .await?;
                Ok(receipt)
            }
        }
        _ => Err(AppError::validation("unknown PM tool")),
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredWorkflowStep {
    caller: Value,
    body: Value,
    workflow_id: i64,
    mode_id: i64,
}

fn validate_step_receipt(value: &Value) -> Result<(), AppError> {
    let exit = value["exit_code"].as_i64().ok_or_else(unavailable)?;
    if value.as_object().is_none_or(|v| v.len() != 2)
        || !matches!(exit, 0 | 1)
        || value["supervisor_accepted"] != (exit == 0)
    {
        return Err(unavailable());
    }
    Ok(())
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Evaluation {
    ok: bool,
    exit_code: i64,
    output: String,
    result: Value,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Instructions {
    ok: bool,
    exit_code: i64,
    output: String,
    result: InstructionResult,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InstructionResult {
    ok: bool,
    task_key: String,
    phase_code: String,
    status: String,
    instructions: String,
    phase_contract: Value,
    workflow_id: i64,
    mode_id: i64,
    mode_key: String,
    cycle_number: i64,
}

async fn prepare(
    supervisor: &LocalRuntimeSupervisor,
    scope: &Scope,
    kind: &str,
    key: &str,
    request: Value,
) -> Result<PmToolCommand, AppError> {
    supervisor
        .repo
        .prepare_pm_tool(PmToolCommand {
            session_run_id: scope.record.reservation.session_run_id,
            key: key.into(),
            kind: kind.into(),
            request,
            result: None,
            attempted: false,
        })
        .await
}

async fn publish_revision(
    supervisor: &LocalRuntimeSupervisor,
    scope: &Scope,
    request: PmPublishRevision,
    body: Value,
) -> Result<Value, AppError> {
    let journal = prepare(
        supervisor,
        scope,
        "revision",
        &request.idempotency_key,
        body.clone(),
    )
    .await?;
    if let Some(result) = journal.result {
        let _: TrackerRequirementsRevision = decode(result.clone())?;
        return Ok(result);
    }
    if !journal.attempted
        && request.expected_requirement_revision != scope.context.requirement_revision
    {
        return Err(AppError::conflict("requirements revision changed"));
    }
    let result = if !journal.attempted
        && supervisor
            .repo
            .claim_pm_tool(journal.session_run_id, &journal.key)
            .await?
    {
        scope
            .coordinator
            .tracker_call(
                &scope.credential,
                reqwest::Method::POST,
                "requirements",
                Some(&body),
            )
            .await?
    } else {
        // Unknown publication is reconciled by the exact immutable revision, never another POST.
        let revisions: TrackerRequirements = decode(
            scope
                .coordinator
                .tracker_call(
                    &scope.credential,
                    reqwest::Method::GET,
                    "requirements/revisions",
                    None,
                )
                .await?,
        )?;
        serde_json::to_value(
            revisions
                .revisions
                .into_iter()
                .find(|r| r.revision == request.expected_requirement_revision.unwrap_or(0) + 1)
                .ok_or_else(unavailable)?,
        )
        .map_err(|_| unavailable())?
    };
    let revision: TrackerRequirementsRevision = decode(result.clone())?;
    let mut document = serde_json::to_value(&revision).map_err(|_| unavailable())?;
    for key in ["revision", "content_hash", "author_subject", "created_at"] {
        document.as_object_mut().unwrap().remove(key);
    }
    if document != serde_json::to_value(request.document).map_err(|_| unavailable())?
        || revision.author_subject
            != scope
                .operation
                .reservation
                .as_ref()
                .unwrap()
                .assignment
                .machine_subject
        || revision.revision != request.expected_requirement_revision.unwrap_or(0) + 1
    {
        return Err(unavailable());
    }
    supervisor
        .repo
        .finish_pm_tool(journal.session_run_id, &journal.key, result.clone())
        .await?;
    Ok(result)
}

async fn publish_question(
    supervisor: &LocalRuntimeSupervisor,
    scope: &Scope,
    snapshot: &PmWorkflowSnapshot,
    request: PmPublishQuestion,
    body: Value,
) -> Result<TrackerQuestion, AppError> {
    let journal = prepare(
        supervisor,
        scope,
        "question",
        &request.idempotency_key,
        body.clone(),
    )
    .await?;
    let result = if let Some(result) = journal.result {
        result
    } else if snapshot.state == "active"
        && snapshot.workflow_step_allowed
        && !journal.attempted
        && supervisor
            .repo
            .claim_pm_tool(journal.session_run_id, &journal.key)
            .await?
    {
        scope
            .coordinator
            .tracker_call(
                &scope.credential,
                reqwest::Method::POST,
                "clarifications",
                Some(&body),
            )
            .await?
    } else if journal.attempted {
        let questions: TrackerClarifications = decode(
            scope
                .coordinator
                .tracker_call(
                    &scope.credential,
                    reqwest::Method::GET,
                    "clarifications",
                    None,
                )
                .await?,
        )?;
        serde_json::to_value(
            questions
                .questions
                .into_iter()
                .find(|q| q.id == request.question_id)
                .ok_or_else(unavailable)?,
        )
        .map_err(|_| unavailable())?
    } else {
        return Err(AppError::conflict(
            "PM is waiting; no new clarification may be published",
        ));
    };
    let question: TrackerQuestion = decode(result.clone())?;
    let expected = serde_json::to_value(&request).map_err(|_| unavailable())?;
    let actual = serde_json::to_value(&question).map_err(|_| unavailable())?;
    for field in [
        "request_id",
        "requirement_revision",
        "checkpoint_id",
        "requirement_reference",
        "text",
        "rationale",
        "required",
        "mode",
        "options",
        "recommended_option_id",
    ] {
        if actual[field] != expected[field] {
            return Err(unavailable());
        }
    }
    if question.id != request.question_id
        || question.task_id.to_string() != snapshot.identity.task_ref
        || question.root_task_id.to_string() != snapshot.identity.root_ref
        || question.assignment_id != request.fence.assignment_id
        || question.execution_id != request.fence.execution_id
        || question.agent_id != request.fence.agent_id
        || question.assignment_version != request.fence.assignment_version
        || question.version != request.expected_question_version.unwrap_or(0) + 1
        || question.author_subject
            != scope
                .operation
                .reservation
                .as_ref()
                .unwrap()
                .assignment
                .machine_subject
        || !matches!(
            question.state,
            TrackerQuestionState::Open | TrackerQuestionState::Answered
        )
    {
        return Err(unavailable());
    }
    supervisor
        .repo
        .finish_pm_tool(journal.session_run_id, &journal.key, result)
        .await?;
    Ok(question)
}

async fn checkpoint(
    workflow: &Workflow<'_>,
    scope: &Scope,
    snapshot: &PmWorkflowSnapshot,
    token: Option<&str>,
    question: &TrackerQuestion,
) -> Result<(), AppError> {
    let data = PmCheckpoint {
        checkpoint_ref: question.checkpoint_id.to_string(),
        clarification_request_ref: question.request_id.to_string(),
        clarification_version: question.version,
        requirements_revision: question.requirement_revision,
    };
    if snapshot.state == "waiting" {
        if serde_json::to_value(&snapshot.checkpoint).map_err(|_| unavailable())?
            != serde_json::to_value(Some(&data)).map_err(|_| unavailable())?
        {
            return Err(AppError::conflict("another PM checkpoint is current"));
        }
        return Ok(());
    }
    if snapshot.state != "active" {
        return Err(AppError::conflict("PM checkpoint is no longer active"));
    }
    let mut body = serde_json::to_value(&snapshot.identity).map_err(|_| unavailable())?;
    body.as_object_mut().unwrap().extend(
        serde_json::to_value(&data)
            .map_err(|_| unavailable())?
            .as_object()
            .unwrap()
            .clone(),
    );
    body.as_object_mut().unwrap().extend(json!({"operation_key":format!("fleet-pm-checkpoint:{}",question.checkpoint_id),
        "expected_version":snapshot.version,"expected_fence":snapshot.fence,"binding_ref":snapshot.binding_ref,
        "hermes_run_ref":snapshot.hermes_run_ref,"session_run_id":snapshot.session_run_id}).as_object().unwrap().clone());
    let (result, _) = response(
        workflow
            .call(
                "/internal/runtime/v1/pm/checkpoint",
                workflow.runtime,
                Some(body),
                token,
            )
            .await?,
        &snapshot.identity,
        false,
    )?;
    if result.state != "waiting"
        || result.version != snapshot.version + 1
        || result.fence != snapshot.fence
        || result.session_run_id != scope.record.reservation.session_run_id
        || result.binding_ref != snapshot.binding_ref
        || result.hermes_run_ref != snapshot.hermes_run_ref
        || result.workflow_step_allowed
        || serde_json::to_value(result.checkpoint).map_err(|_| unavailable())?
            != serde_json::to_value(data).map_err(|_| unavailable())?
    {
        return Err(unavailable());
    }
    Ok(())
}
