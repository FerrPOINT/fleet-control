use super::*;

pub(super) async fn execute(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    record: &domain::PmRunRecord,
    operation: &str,
    command: &Value,
) -> Result<Value, AppError> {
    if !supervisor.config.pm.workflow.enabled
        || agent.sdlc_role != Some(domain::SdlcRole::ProjectManager)
        || record.reservation.identity.agent_id()? != agent.id
        || record.terminal_status.is_some()
    {
        return Err(AppError::Forbidden);
    }
    let original = record
        .reservation
        .runtime_binding
        .as_ref()
        .ok_or_else(|| AppError::Unavailable("PM original native binding required".into()))?;
    if pm_readback::capture(supervisor, agent).await? != *original {
        return Err(AppError::conflict("PM tool launch changed"));
    }
    let op = supervisor
        .repo
        .find_pm_creation_for_session(record.reservation.session_id)
        .await?;
    if op.execution_identity()? != record.reservation.identity {
        return Err(AppError::conflict("PM tool execution changed"));
    }
    let coordinator =
        crate::pm_credentials::PmCredentialCoordinator::configured(&supervisor.config)?
            .ok_or_else(|| AppError::Unavailable("PM credentials unavailable".into()))?;
    let credential = coordinator
        .prepare_credential(supervisor.repo.as_ref(), &op)
        .await?;
    let workflow = crate::pm_workflow::PmWorkflowClient::configured(&supervisor.config)?
        .ok_or_else(|| AppError::Unavailable("PM Workflow unavailable".into()))?;
    if !workflow.verify_active_run(record).await? {
        return Err(AppError::Unavailable("PM enrollment pending".into()));
    }
    if operation == "workflow" {
        return workflow.native_step(record, command).await;
    }
    if operation == "skills" {
        return skill(supervisor, agent, record, &workflow, command).await;
    }
    if operation == "checkpoint" {
        return checkpoint(supervisor, record, &workflow, &credential, command).await;
    }
    let mut url = crate::pm_credentials::configured_origin(&supervisor.config.tracker.url)?;
    let (method, suffix) = match operation {
        "context" => (reqwest::Method::GET, "context"),
        "question" => (reqwest::Method::POST, "clarifications"),
        "requirements" => (reqwest::Method::POST, "requirements"),
        _ => {
            return Err(AppError::Unavailable(
                "PM tool integration is not implemented".into(),
            ));
        }
    };
    if !command.is_object() {
        return Err(AppError::validation("structured PM command required"));
    }
    if method == reqwest::Method::GET && command.as_object().is_some_and(|v| !v.is_empty()) {
        return Err(AppError::validation("PM context command must be empty"));
    }
    url.set_path(&format!(
        "/api/v1/issues/{}/sdlc/{suffix}",
        record.reservation.identity.task_ref
    ));
    let request = supervisor
        .client
        .request(method.clone(), url)
        .timeout(Duration::from_secs(10))
        .header(reqwest::header::ACCEPT_ENCODING, "identity");
    let request = if method == reqwest::Method::POST {
        let mut body = command.clone();
        let fields = body
            .as_object_mut()
            .ok_or_else(|| AppError::validation("structured PM command required"))?;
        if fields.contains_key("fence") {
            return Err(AppError::Forbidden);
        }
        fields.insert(
            "fence".into(),
            serde_json::to_value(
                &op.execution_lease
                    .as_ref()
                    .ok_or_else(|| AppError::conflict("PM original lease missing"))?
                    .claim
                    .fence,
            )
            .map_err(AppError::internal)?,
        );
        request.json(&body)
    } else {
        request
    };
    let request = credential.authorize(request)?;
    let response = supervisor
        .client
        .execute(request)
        .await
        .map_err(|_| AppError::Unavailable("PM owner command outcome unknown".into()))?;
    let status = response.status();
    if !matches!(
        status,
        reqwest::StatusCode::OK | reqwest::StatusCode::CREATED
    ) {
        return Err(match status {
            reqwest::StatusCode::UNAUTHORIZED => AppError::Unauthorized,
            reqwest::StatusCode::FORBIDDEN => AppError::Forbidden,
            reqwest::StatusCode::CONFLICT => AppError::conflict("PM owner command is stale"),
            _ => AppError::Unavailable("PM owner command outcome unavailable".into()),
        });
    }
    let mut value = hermes_wire::read_json(response, status, 256 * 1024).await?;
    if operation == "context" {
        let questions = tracker_read(supervisor, record, &credential, "clarifications").await?;
        let revision = value["requirement_revision"].as_i64();
        let requirements = match revision {
            Some(revision) if (1..=9_007_199_254_740_991).contains(&revision) => {
                tracker_read(
                    supervisor,
                    record,
                    &credential,
                    &format!("requirements/{revision}"),
                )
                .await?
            }
            None if value["requirement_revision"].is_null() => Value::Null,
            _ => return Err(AppError::conflict("PM requirements cursor is malformed")),
        };
        value = json!({"task_context":value,"input":op.input.as_ref()
            .ok_or_else(|| AppError::conflict("PM original input missing"))?.input,
            "clarifications":questions,"requirements":requirements});
    }
    if pm_readback::capture(supervisor, agent).await? != *original {
        return Err(AppError::conflict("PM tool launch changed during command"));
    }
    Ok(value)
}

pub(super) async fn tracker_read(
    supervisor: &LocalRuntimeSupervisor,
    record: &domain::PmRunRecord,
    credential: &crate::pm_credentials::PmDelegatedCredential,
    resource: &str,
) -> Result<Value, AppError> {
    let mut url = crate::pm_credentials::configured_origin(&supervisor.config.tracker.url)?;
    url.set_path(&format!(
        "/api/v1/issues/{}/sdlc/{resource}",
        record.reservation.identity.task_ref
    ));
    let request = credential.authorize(
        supervisor
            .client
            .get(url)
            .timeout(Duration::from_secs(5))
            .header(reqwest::header::ACCEPT_ENCODING, "identity"),
    )?;
    let response = supervisor
        .client
        .execute(request)
        .await
        .map_err(|_| AppError::Unavailable("PM owner context unavailable".into()))?;
    match response.status() {
        reqwest::StatusCode::UNAUTHORIZED => return Err(AppError::Unauthorized),
        reqwest::StatusCode::FORBIDDEN => return Err(AppError::Forbidden),
        reqwest::StatusCode::CONFLICT => {
            return Err(AppError::conflict("PM owner context changed"));
        }
        reqwest::StatusCode::OK => (),
        _ => return Err(AppError::Unavailable("PM owner context unavailable".into())),
    }
    hermes_wire::read_json(response, reqwest::StatusCode::OK, 256 * 1024).await
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct SkillCommand {
    name: String,
    operation_key: String,
    expected_phase_code: String,
    expected_status: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointCommand {
    operation_key: String,
    question_id: Uuid,
}

async fn checkpoint(
    supervisor: &LocalRuntimeSupervisor,
    record: &domain::PmRunRecord,
    workflow: &crate::pm_workflow::PmWorkflowClient,
    credential: &crate::pm_credentials::PmDelegatedCredential,
    command: &Value,
) -> Result<Value, AppError> {
    let input: CheckpointCommand = serde_json::from_value(command.clone())
        .map_err(|_| AppError::validation("closed PM checkpoint command required"))?;
    if serde_json::to_value(&input).map_err(AppError::internal)? != *command
        || input.question_id.is_nil()
        || !domain::pm_draft::valid_key(&input.operation_key)
        || !input.operation_key.is_ascii()
    {
        return Err(AppError::validation("invalid PM checkpoint cursor"));
    }
    let mut url = crate::pm_credentials::configured_origin(&supervisor.config.tracker.url)?;
    url.set_path(&format!(
        "/api/v1/issues/{}/sdlc/clarifications",
        record.reservation.identity.task_ref
    ));
    let request = credential.authorize(
        supervisor
            .client
            .get(url)
            .timeout(Duration::from_secs(5))
            .header(reqwest::header::ACCEPT_ENCODING, "identity"),
    )?;
    let response = supervisor
        .client
        .execute(request)
        .await
        .map_err(|_| AppError::Unavailable("PM clarification source unavailable".into()))?;
    let raw = hermes_wire::read_json(response, reqwest::StatusCode::OK, 256 * 1024).await?;
    let questions: domain::TrackerClarifications = serde_json::from_value(raw)
        .map_err(|_| AppError::Unavailable("PM clarification source malformed".into()))?;
    let matches: Vec<_> = questions
        .questions
        .iter()
        .filter(|q| q.id == input.question_id)
        .collect();
    if matches.len() != 1 {
        return Err(AppError::conflict("PM clarification is not unique"));
    }
    let q = matches[0];
    let identity = &record.reservation.identity;
    if q.task_id.to_string() != identity.task_ref
        || q.root_task_id.to_string() != identity.root_ref
        || q.assignment_id.to_string() != identity.assignment_ref
        || q.execution_id.to_string() != identity.execution_ref
        || q.agent_id.to_string() != identity.agent_ref
        || q.assignment_version != identity.assignment_revision
        || q.author_subject != supervisor.config.pm.credentials.machine_subject
        || !matches!(
            q.state,
            domain::TrackerQuestionState::Open | domain::TrackerQuestionState::Answered
        )
    {
        return Err(AppError::conflict(
            "PM question belongs to another execution or is no longer current",
        ));
    }
    workflow
        .checkpoint(supervisor.repo.as_ref(), record, q, &input.operation_key)
        .await
}

async fn skill(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    record: &domain::PmRunRecord,
    workflow: &crate::pm_workflow::PmWorkflowClient,
    command: &Value,
) -> Result<Value, AppError> {
    let request: SkillCommand = serde_json::from_value(command.clone())
        .map_err(|_| AppError::validation("closed PM skill command required"))?;
    if serde_json::to_value(&request).map_err(AppError::internal)? != *command {
        return Err(AppError::validation("closed PM skill command required"));
    }
    let phase = workflow.native_step(record, &json!({"operation_key":request.operation_key,
        "report":null,"expected_phase_code":request.expected_phase_code,"expected_status":request.expected_status})).await?;
    if !phase["phase_contract"]["skills"]
        .as_array()
        .is_some_and(|names| names.contains(&json!(request.name)))
    {
        return Err(AppError::Forbidden);
    }
    let effective = supervisor
        .repo
        .get_effective_config_revision(agent.id)
        .await?
        .ok_or_else(|| AppError::Unavailable("PM effective skills unavailable".into()))?;
    crate::effective_configuration::verify(agent, &supervisor.config, &effective).await?;
    let package = crate::base_package::VerifiedRolePackage::read(
        std::path::Path::new(&supervisor.config.fleet.base_package_checkout),
        domain::SdlcRole::ProjectManager,
    )
    .await?;
    package.verify_snapshot(agent, &effective.snapshot)?;
    let skill = effective
        .snapshot
        .skills
        .iter()
        .find(|skill| skill.name == request.name && skill.state == domain::SkillState::Enabled)
        .ok_or(AppError::Forbidden)?;
    Ok(json!({"name":skill.name,"content":skill.content,
        "sha256":package.proof().skill_sha256.get(&skill.name).ok_or(AppError::Forbidden)?,
        "package_revision":package.proof().commit}))
}
