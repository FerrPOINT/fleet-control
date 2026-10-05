use super::*;

impl LocalRuntimeSupervisor {
    pub(super) fn spawn_acceptance_readback(&self) {
        let supervisor = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let mut after = None;
                loop {
                    match supervisor.repo.list_recoverable_hermes_acceptances(after).await {
                        Ok(records) => {
                            if records.is_empty() { after = None; }
                            for (message, run) in records {
                                // Invalid old ACKs must not starve later pending or pinned readbacks.
                                after = Some(run.id);
                                let result = async {
                                    let session = supervisor.repo.get_session(run.session_id).await?;
                                    let agent = supervisor.repo.get_agent(run.agent_id).await?;
                                    supervisor.finish_acceptance_readback(&agent, &session, &message, &run).await
                                }.await;
                                if result.is_err() {
                                    // Capacity remains held. Never submit a run in recovery.
                                    tracing::warn!(run_id = %run.id, "Hermes acceptance readback requires reconciliation");
                                }
                            }
                        }
                        Err(_) => tracing::warn!("Hermes acceptance readback queue is unavailable"),
                    }
                    if supervisor.repo.reconcile_runtime_controls().await.is_err() {
                        tracing::warn!("Runtime control terminal readback is unavailable");
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
        let base = Self::hermes_base_url(agent)?;
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        let mut intent = self.repo.get_hermes_dispatch_intent(message.id).await?
            .ok_or_else(|| AppError::Unavailable("Legacy Hermes acceptance has no original dispatch context; reconciliation is required".into()))?;
        if intent.run.id != run.id
            || intent.run.agent_id != agent.id
            || intent.run.session_id != session.id
            || (run.runtime_run_id.is_some() && intent.run.runtime_run_id != run.runtime_run_id)
        {
            return Err(AppError::conflict(
                "Hermes journal acceptance does not match",
            ));
        }
        hermes_wire::verify_intent(&intent, &base, &token)?;
        if intent.run.runtime_run_id.is_none() {
            if !self.config.fleet.hermes_recovery_extension_enabled {
                return Err(AppError::Unavailable("Hermes acceptance is unknown".into()));
            }
            let recovered = recovery_wire::lookup(&self.client, &base, &token, &intent).await?;
            self.repo
                .accept_recovered_hermes_run(
                    message.id,
                    run.id,
                    recovered,
                    intent.capabilities.clone(),
                )
                .await?;
            intent = self
                .repo
                .get_hermes_dispatch_intent(message.id)
                .await?
                .ok_or_else(|| AppError::Unavailable("Hermes recovery journal missing".into()))?;
            hermes_wire::verify_intent(&intent, &base, &token)?;
        }
        let runtime_run_id = intent
            .run
            .runtime_run_id
            .clone()
            .ok_or_else(|| AppError::Unavailable("Hermes acceptance is unknown".into()))?;
        if intent.state != "accepted" {
            return Err(AppError::conflict(
                "Hermes journal acceptance does not match",
            ));
        }
        if matches!(
            intent.run.state,
            SessionRunState::Completed | SessionRunState::Failed | SessionRunState::Cancelled
        ) {
            return Ok(intent.run);
        }
        let payload =
            hermes_wire::read_accepted_run(&self.client, &base, &token, &runtime_run_id).await?;
        let effective = hermes_wire::effective_session(&payload, &runtime_run_id)?;
        let (pinned, first) = if intent.run.state == SessionRunState::Pending {
            self.repo
                .pin_hermes_run_session(
                    run.id,
                    runtime_run_id.to_owned(),
                    Self::runtime_session_id(session, agent),
                    effective,
                )
                .await?
        } else {
            if intent.run.runtime_session_id.as_deref() != Some(effective.as_str()) {
                return Err(AppError::Unavailable(
                    "Hermes terminal session identity changed".into(),
                ));
            }
            (intent.run, false)
        };
        if first {
            self.emit_run(&pinned);
            let _ = self.events.send(FleetEvent::SessionMessageChanged {
                session_id: session.id.to_string(),
                message_id: message.id.to_string(),
                event: "message.dispatched".to_string(),
            });
        }
        if matches!(
            payload.get("status").and_then(Value::as_str),
            Some("completed" | "failed" | "cancelled" | "interrupted" | "stopped")
        ) {
            let event = hermes_wire::terminal_readback(&payload, &runtime_run_id)?;
            self.handle_hermes_event(
                agent,
                session,
                message,
                &pinned,
                &runtime_run_id,
                Some(event.to_owned()),
                payload.to_string(),
                &mut sse_wire::Transcript::default(),
            )
            .await?;
            return self.repo.get_session_agent_run(pinned.id).await;
        }
        if payload["status"] == "waiting_for_approval" {
            let caps = self.probe_hermes(agent).await?;
            targeted_approval::verify_capability(&caps)?;
            let request = approval_snapshot::request(&payload, &pinned)?;
            let (approval, created) = self
                .repo
                .recover_hermes_approval(
                    request,
                    pinned
                        .runtime_session_id
                        .clone()
                        .ok_or_else(|| AppError::conflict("missing pinned native session"))?,
                    base,
                    hermes_wire::credential_fingerprint(&token),
                )
                .await?;
            if created {
                let _ = self.events.send(FleetEvent::RuntimeApprovalRequested {
                    session_id: session.id.to_string(),
                    run_id: pinned.id.to_string(),
                    approval_id: approval.id.to_string(),
                });
            }
        }
        if first {
            self.spawn_hermes_event_worker(
                agent.clone(),
                session.clone(),
                message.clone(),
                pinned.clone(),
                runtime_run_id.to_owned(),
            );
        }
        self.repo.get_session_agent_run(pinned.id).await
    }
}
