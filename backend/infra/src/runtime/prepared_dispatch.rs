use super::*;

impl LocalRuntimeSupervisor {
    pub(super) fn spawn_prepared_dispatch(&self) {
        let supervisor = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let mut after = None;
                loop {
                    match supervisor.repo.list_prepared_hermes_dispatches(after).await {
                        Ok(records) => {
                            if records.is_empty() { after = None; }
                            for (message, intent) in records {
                                after = Some(intent.run.id);
                                let run_id = intent.run.id;
                                if supervisor.resume_prepared_dispatch(message, intent).await.is_err() {
                                    // A failed read/preflight neither consumes a permit nor resets history.
                                    tracing::warn!(%run_id, "Prepared Hermes dispatch requires reconciliation");
                                }
                            }
                        }
                        Err(_) => tracing::warn!("Prepared Hermes dispatch queue is unavailable"),
                    }
                    sleep(Duration::from_secs(5)).await;
                }
            });
        }
    }

    async fn resume_prepared_dispatch(
        &self,
        message: SessionMessage,
        intent: app::HermesDispatchIntent,
    ) -> Result<(), AppError> {
        if intent.state != "prepared" || intent.submission_attempted || !intent.recovery_allowed {
            return Ok(());
        }
        let session = self.repo.get_session(intent.run.session_id).await?;
        let agent = self.repo.get_agent(intent.run.agent_id).await?;
        if session.primary_agent_id != agent.id
            || self.repo.get_task_chat_binding(session.id).await?.is_some()
        {
            return Err(AppError::conflict(
                "Prepared Hermes dispatch is not its free chat",
            ));
        }
        let base = self.hermes_base_url(&agent).await?;
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        hermes_wire::verify_intent(&intent, &base, &token)?;
        self.verify_dispatch_launch(agent.id, &intent.capabilities)
            .await?;
        let mut current = hermes_wire::dispatch_capabilities(&self.probe_hermes(&agent).await?)?;
        // The private Fleet binding is not advertised by Hermes. Retain it only
        // after proving this controller still owns that exact original child.
        if let Some(launch) = intent.capabilities.get("fleet_launch") {
            current["fleet_launch"] = launch.clone();
        }
        if recovery_wire::store_id(&intent.capabilities)?.is_some() {
            if !self.config.fleet.hermes_recovery_extension_enabled {
                return Err(AppError::Unavailable(
                    "Original Hermes recovery configuration changed".into(),
                ));
            }
            current["fleet_recovery"] =
                recovery_wire::capabilities(&self.client, &base, &token).await?;
        }
        if current != intent.capabilities {
            return Err(AppError::Unavailable(
                "Prepared Hermes protocol facts changed".into(),
            ));
        }
        if message.id != intent.message_id || message.session_id != session.id {
            return Err(AppError::conflict(
                "Prepared Hermes message identity changed",
            ));
        }
        let Some(claimed) = self
            .repo
            .claim_hermes_submission(
                intent.message_id,
                base,
                hermes_wire::credential_fingerprint(&token),
            )
            .await?
        else {
            // A concurrent original dispatcher/recovery already consumed the only permit.
            return Ok(());
        };
        let outcome = self
            .submit_hermes_intent(&agent, &session, &message, &claimed, &token)
            .await;
        let error = outcome
            .err()
            .map(|error| crate::redact_text(&error.to_string()));
        if let Some(detail) = &error {
            // Once submitted, the native outcome is unknown; keep pending and capacity held.
            self.repo
                .update_session_message_delivery(
                    message.id,
                    MessageDeliveryState::Pending,
                    None,
                    Some(detail.clone()),
                )
                .await?;
        }
        self.repo
            .finish_message_dispatch(message.id, error.is_some(), error)
            .await
    }
}
