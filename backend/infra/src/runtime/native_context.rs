use super::*;

pub(super) struct AcceptedContext {
    pub run: SessionAgentRun,
    pub base: String,
    pub token: String,
}

pub(super) async fn accepted(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    run: &SessionAgentRun,
) -> Result<AcceptedContext, AppError> {
    let current = supervisor.repo.get_session_agent_run(run.id).await?;
    let current_agent = supervisor.repo.get_agent(agent.id).await?;
    if agent.kind != AgentKind::Hermes
        || current_agent.kind != AgentKind::Hermes
        || current.agent_id != agent.id
        || current.session_id != run.session_id
        || current.runtime_run_id != run.runtime_run_id
        || current.runtime_session_id != run.runtime_session_id
        || supervisor.hermes_base_url(&current_agent).await?
            != supervisor.hermes_base_url(agent).await?
    {
        return Err(AppError::conflict("runtime control identity changed"));
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
    current
        .runtime_run_id
        .as_deref()
        .filter(|id| crate::pm_execution::valid_hermes_ref(id))
        .ok_or_else(|| AppError::conflict("verified native run identity is required"))?;
    current
        .runtime_session_id
        .as_deref()
        .filter(|id| domain::valid_ref(id, 512))
        .ok_or_else(|| AppError::conflict("verified native session identity is required"))?;
    let base = supervisor.run_base_url(&current_agent, &current).await?;
    let token = crate::agent_runtime_token(&supervisor.config, agent.id)?;
    let intent = supervisor
        .repo
        .get_accepted_hermes_context(run.id)
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
    Ok(AcceptedContext {
        run: current,
        base,
        token,
    })
}
