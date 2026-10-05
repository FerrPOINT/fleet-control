use super::*;

impl LocalRuntimeSupervisor {
    pub(super) fn spawn_control_outcome_readback(&self) {
        if !self.config.fleet.hermes_control_outcome_enabled {
            return;
        }
        let supervisor = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let mut after = None;
                loop {
                    match supervisor.repo.list_runtime_control_outcomes(after).await {
                        Ok(records) => {
                            if records.is_empty() {
                                after = None;
                            }
                            for intent in records {
                                // Invalid/missing old witnesses cannot starve later UUID-keyset pages.
                                after = Some(intent.receipt.id);
                                let command_id = intent.receipt.id;
                                if supervisor
                                    .finish_control_outcome_readback(intent)
                                    .await
                                    .is_err()
                                {
                                    tracing::warn!(
                                        %command_id,
                                        "Runtime control outcome requires reconciliation"
                                    );
                                }
                            }
                        }
                        Err(_) => tracing::warn!("Runtime control outcome queue is unavailable"),
                    }
                    sleep(Duration::from_secs(5)).await;
                }
            });
        }
    }

    pub(super) async fn finish_control_outcome_readback(
        &self,
        intent: app::RuntimeControlOutcomeIntent,
    ) -> Result<(), AppError> {
        let receipt = &intent.receipt;
        let agent = self.repo.get_agent(receipt.agent_id).await?;
        if agent.status == AgentStatus::Archived {
            return Err(AppError::conflict(
                "archived runtime outcome requires reconciliation",
            ));
        }
        let run = self
            .repo
            .get_session_agent_run(receipt.session_run_id)
            .await?;
        if run.session_id != receipt.session_id || run.agent_id != receipt.agent_id {
            return Err(AppError::conflict("runtime control scope changed"));
        }
        let accepted = native_context::accepted(self, &agent, &run).await?;
        let context: control_outcome_wire::Context = serde_json::from_value(intent.context.clone())
            .map_err(|_| AppError::conflict("invalid original control context"))?;
        context.verify_receipt(
            receipt,
            accepted.run.runtime_run_id.as_deref().unwrap_or(""),
        )?;
        let outcome =
            control_outcome_wire::lookup(&self.client, &context, &accepted.base, &accepted.token)
                .await?;
        if let control_outcome_wire::Outcome::Acknowledged(ack) = outcome {
            self.repo
                .finish_runtime_control_outcome(receipt.id, intent.context, ack.control_ack()?)
                .await?;
        }
        // Missing/uncertain outcome never releases a hold or creates another dispatch permit.
        Ok(())
    }
}
