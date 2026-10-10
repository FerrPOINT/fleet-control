use super::*;

pub(super) async fn preparation(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    session: &AgentSession,
    message: &SessionMessage,
) -> Result<
    (
        domain::PmDraftOperation,
        domain::PmRuntimeBinding,
        Option<domain::PmResumeJournal>,
    ),
    AppError,
> {
    if !supervisor.config.pm.workflow.enabled
        || agent.sdlc_role != Some(domain::SdlcRole::ProjectManager)
    {
        return Err(AppError::Unavailable(
            "task runtime admission is not yet verified".into(),
        ));
    }
    let op = supervisor
        .repo
        .find_pm_creation_for_session(session.id)
        .await?;
    let resume = supervisor.repo.pm_resume_by_message(message.id).await?;
    let valid_message = if let Some(journal) = &resume {
        message.author_type == MessageAuthorType::System
            && message.message_kind == MessageKind::Control
            && message.body == journal.intent.prompt
            && journal.intent.message_id == message.id
            && journal.receipt.is_some()
    } else {
        message.author_user_id == Some(op.owner_user_id)
            && message.message_kind == MessageKind::UserPrompt
            && message.body.trim()
                == op
                    .input
                    .as_ref()
                    .ok_or_else(|| AppError::conflict("PM input is missing"))?
                    .input
                    .description
                    .trim()
    };
    if op.request.agent_id != agent.id || op.session_id != Some(session.id) || !valid_message {
        return Err(AppError::conflict(
            "PM initial dispatch differs from immutable intake",
        ));
    }
    let coordinator =
        crate::pm_credentials::PmCredentialCoordinator::configured(&supervisor.config)?
            .ok_or_else(|| AppError::Unavailable("PM credentials are not configured".into()))?;
    let credential = coordinator
        .prepare_credential(supervisor.repo.as_ref(), &op)
        .await?;
    let workflow = crate::pm_workflow::PmWorkflowClient::configured(&supervisor.config)?
        .ok_or_else(|| AppError::Unavailable("PM Workflow is not configured".into()))?;
    workflow
        .prepare_assignment(supervisor.repo.as_ref(), &op, &coordinator, &credential)
        .await?;
    let binding = pm_readback::capture(supervisor, agent).await?;
    if let Some(journal) = &resume {
        let old = supervisor
            .repo
            .get_pm_run(journal.intent.old_session_run_id)
            .await?;
        let checkpoint = supervisor
            .repo
            .pm_checkpoint(journal.intent.old_session_run_id)
            .await?
            .ok_or_else(|| AppError::conflict("PM original checkpoint missing"))?;
        if old.reservation.identity != op.execution_identity()?
            || old.reservation.runtime_binding.as_ref() != Some(&binding)
            || !pm_readback::probe(supervisor, agent, &old)
                .await?
                .terminal()
            || !workflow
                .resume_delivery_allowed(&old, &checkpoint, journal)
                .await?
        {
            return Err(AppError::conflict(
                "PM original resume is no longer pending",
            ));
        }
    }
    let effective = supervisor
        .repo
        .get_effective_config_revision(agent.id)
        .await?
        .ok_or_else(|| AppError::Unavailable("PM effective configuration is missing".into()))?;
    let launch = supervisor
        .repo
        .get_runtime_launch(binding.launch_id)
        .await?
        .ok_or_else(|| AppError::Unavailable("PM original launch is missing".into()))?;
    if launch.binding.configuration_revision != Some(effective.revision)
        || launch.binding.configuration_sha256.as_deref()
            != Some(
                crate::runtime_launches::snapshot_hash(
                    &serde_json::to_value(&effective.snapshot).map_err(AppError::internal)?,
                )?
                .as_str(),
            )
    {
        return Err(AppError::conflict(
            "PM native launch is not using the effective snapshot",
        ));
    }
    crate::effective_configuration::verify(agent, &supervisor.config, &effective).await?;
    Ok((op, binding, resume))
}

pub(super) async fn accepted(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    run: &SessionAgentRun,
    native: &str,
) -> Result<(), AppError> {
    let record = supervisor.repo.get_pm_run(run.id).await?;
    let binding = record
        .reservation
        .runtime_binding
        .as_ref()
        .ok_or_else(|| AppError::conflict("PM original binding missing"))?;
    let payload = hermes_wire::read_accepted_run(
        &supervisor.client,
        &binding.origin,
        &crate::agent_runtime_token(&supervisor.config, agent.id)?,
        native,
    )
    .await?;
    let session = hermes_wire::effective_session(&payload, native)?;
    let record = supervisor
        .repo
        .accept_pm_run(run.id, native.into(), session)
        .await?;
    let workflow = crate::pm_workflow::PmWorkflowClient::configured(&supervisor.config)?
        .ok_or_else(|| AppError::Unavailable("PM Workflow is not configured".into()))?;
    let resume = match record.reservation.native_message_id {
        Some(message) => supervisor.repo.pm_resume_by_message(message).await?,
        None => None,
    };
    if let Some(journal) = resume {
        let old = supervisor
            .repo
            .get_pm_run(journal.intent.old_session_run_id)
            .await?;
        let checkpoint = supervisor
            .repo
            .pm_checkpoint(journal.intent.old_session_run_id)
            .await?
            .ok_or_else(|| AppError::conflict("PM original checkpoint missing"))?;
        workflow
            .rebind_original(&old, &record, &checkpoint, &journal)
            .await?;
    } else {
        workflow.bind_initial_run(&record).await?;
    }
    let current = supervisor.repo.get_session_agent_run(run.id).await?;
    supervisor.emit_run(&current);
    Ok(())
}
