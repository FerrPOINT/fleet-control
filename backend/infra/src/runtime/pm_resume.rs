use super::*;

pub(super) async fn prepare_answers(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    operation: &domain::PmDraftOperation,
) -> Result<(), AppError> {
    let Some(session) = operation.session_id else {
        return Ok(());
    };
    let runs = supervisor.repo.waiting_pm_runs(session).await?;
    if runs.is_empty() {
        return Ok(());
    }
    let coordinator =
        crate::pm_credentials::PmCredentialCoordinator::configured(&supervisor.config)?
            .ok_or_else(|| AppError::Unavailable("PM credentials unavailable".into()))?;
    let credential = coordinator
        .prepare_credential(supervisor.repo.as_ref(), operation)
        .await?;
    let workflow = crate::pm_workflow::PmWorkflowClient::configured(&supervisor.config)?
        .ok_or_else(|| AppError::Unavailable("PM Workflow unavailable".into()))?;
    for run in runs {
        let old = supervisor.repo.get_pm_run(run).await?;
        let checkpoint = supervisor
            .repo
            .pm_checkpoint(run)
            .await?
            .ok_or_else(|| AppError::conflict("PM confirmed wait missing"))?;
        if old.reservation.identity != operation.execution_identity()? {
            return Err(AppError::conflict("PM resume execution changed"));
        }
        if let Some(journal) = supervisor.repo.pm_resume(run).await? {
            if journal.receipt.is_none() {
                pm_readback::probe(supervisor, agent, &old).await?;
            }
            let journal = workflow
                .resume_original(supervisor.repo.as_ref(), &old, &checkpoint)
                .await?;
            if workflow
                .resume_delivery_allowed(&old, &checkpoint, &journal)
                .await?
            {
                supervisor.repo.queue_pm_resume(run).await?;
            }
            continue;
        }
        if old.reservation.identity != operation.execution_identity()?
            || old.reservation.runtime_binding.as_ref()
                != pm_readback::capture(supervisor, agent).await.ok().as_ref()
        {
            return Err(AppError::conflict("PM original wait runtime changed"));
        }
        let raw = pm_tools::tracker_read(supervisor, &old, &credential, "clarifications").await?;
        let source: domain::TrackerClarifications = serde_json::from_value(raw)
            .map_err(|_| AppError::Unavailable("PM structured answers malformed".into()))?;
        let matching: Vec<_> = source
            .questions
            .iter()
            .filter(|q| {
                checkpoint.command["checkpoint_ref"] == json!(q.checkpoint_id)
                    && checkpoint.command["clarification_request_ref"] == json!(q.request_id)
                    && checkpoint.command["clarification_version"].as_i64() == Some(q.version)
                    && checkpoint.command["requirements_revision"].as_i64()
                        == Some(q.requirement_revision)
                    && matches!(q.state, domain::TrackerQuestionState::Answered)
                    && q.answer.is_some()
            })
            .collect();
        if matching.is_empty() {
            continue;
        }
        if matching.len() != 1 {
            return Err(AppError::conflict("PM answered wait is ambiguous"));
        }
        let question = matching[0];
        if question.author_subject != supervisor.config.pm.credentials.machine_subject {
            return Err(AppError::conflict("PM question author changed"));
        }
        let binding = supervisor
            .repo
            .get_task_chat_binding(session)
            .await?
            .ok_or_else(|| AppError::conflict("PM Task binding missing"))?;
        let reader = crate::tracker_event_poller::TrackerEventPoller::configured(
            &supervisor.config.tracker,
            supervisor.repo.clone(),
        )?
        .ok_or_else(|| {
            AppError::Unavailable("PM trusted answer event reader unavailable".into())
        })?;
        let Some(event) = reader.answer_event(&binding, question).await? else {
            continue;
        };
        let status = pm_readback::probe(supervisor, agent, &old).await?;
        if !status.terminal() {
            return Err(AppError::conflict(
                "PM original conversation is still active",
            ));
        }
        let intent = domain::PmResumeIntent::from_answer(
            &old,
            &checkpoint,
            question,
            &event,
            Uuid::new_v4(),
            Uuid::new_v4(),
        )?;
        supervisor
            .repo
            .save_pm_resume(
                &old,
                &checkpoint,
                domain::PmResumeJournal {
                    intent,
                    receipt: None,
                },
            )
            .await?;
        workflow
            .resume_original(supervisor.repo.as_ref(), &old, &checkpoint)
            .await?;
        let journal = supervisor
            .repo
            .pm_resume(run)
            .await?
            .ok_or_else(|| AppError::conflict("PM resume acknowledgement missing"))?;
        if workflow
            .resume_delivery_allowed(&old, &checkpoint, &journal)
            .await?
        {
            supervisor.repo.queue_pm_resume(run).await?;
        }
    }
    Ok(())
}
