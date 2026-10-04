use super::*;

impl LocalRuntimeSupervisor {
    pub(super) fn spawn_acceptance_readback(&self) {
        let supervisor = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let mut after = None;
                loop {
                    match supervisor.repo.list_pending_hermes_acceptances(after).await {
                        Ok(records) => {
                            if records.is_empty() { after = None; }
                            for (message, run) in records {
                                // Invalid old ACKs must not starve later pending readbacks.
                                after = Some(run.id);
                                let result = async {
                                    let session = supervisor.repo.get_session(run.session_id).await?;
                                    let agent = supervisor.repo.get_agent(run.agent_id).await?;
                                    supervisor.finish_acceptance_readback(&agent, &session, &message, &run).await
                                }.await;
                                if result.is_err() {
                                    // The persisted ACK and agent capacity remain held. Never POST here.
                                    tracing::warn!(run_id = %run.id, "Hermes acceptance readback requires reconciliation");
                                }
                            }
                        }
                        Err(_) => tracing::warn!("Hermes acceptance readback queue is unavailable"),
                    }
                    sleep(Duration::from_secs(5)).await;
                }
            });
        }
    }

    pub(super) async fn finish_acceptance_readback(
        &self,
        agent: &Agent,
        session: &AgentSession,
        message: &SessionMessage,
        run: &SessionAgentRun,
    ) -> Result<SessionAgentRun, AppError> {
        if run.agent_id != agent.id
            || run.session_id != session.id
            || session.primary_agent_id != agent.id
            || message.session_id != session.id
            || self.repo.get_task_chat_binding(session.id).await?.is_some()
        {
            return Err(AppError::conflict(
                "Hermes acceptance does not match its free chat",
            ));
        }
        let runtime_run_id = run
            .runtime_run_id
            .as_deref()
            .ok_or_else(|| AppError::Unavailable("Hermes acceptance is unknown".into()))?;
        let base = Self::hermes_base_url(agent)?;
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        let intent = self.repo.get_hermes_dispatch_intent(message.id).await?
            .ok_or_else(|| AppError::Unavailable("Legacy Hermes acceptance has no original dispatch context; reconciliation is required".into()))?;
        if intent.run.id != run.id
            || intent.run.runtime_run_id.as_deref() != Some(runtime_run_id)
            || intent.state != "accepted"
        {
            return Err(AppError::conflict(
                "Hermes journal acceptance does not match",
            ));
        }
        hermes_wire::verify_intent(&intent, &base, &token)?;
        let payload =
            hermes_wire::read_accepted_run(&self.client, &base, &token, runtime_run_id).await?;
        let effective = hermes_wire::effective_session(&payload, runtime_run_id)?;
        let (pinned, first) = self
            .repo
            .pin_hermes_run_session(
                run.id,
                runtime_run_id.to_owned(),
                Self::runtime_session_id(session, agent),
                effective,
            )
            .await?;
        if first {
            self.emit_run(&pinned);
            let _ = self.events.send(FleetEvent::SessionMessageChanged {
                session_id: session.id.to_string(),
                message_id: message.id.to_string(),
                event: "message.dispatched".to_string(),
            });
            self.spawn_hermes_event_worker(
                agent.clone(),
                session.clone(),
                message.clone(),
                pinned.clone(),
                runtime_run_id.to_owned(),
            );
        }
        Ok(pinned)
    }
}
