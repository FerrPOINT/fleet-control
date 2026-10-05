use super::*;
use domain::{ApprovalDecision, RuntimeApprovalRequest};

pub(super) async fn send(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    run: &SessionAgentRun,
    approval: &RuntimeApprovalRequest,
    decision: &ApprovalDecision,
) -> Result<ApprovalDecision, AppError> {
    if !supervisor.config.fleet.hermes_control_outcome_enabled {
        return Err(AppError::Unavailable(
            "original approval outcomes are disabled".into(),
        ));
    }
    if agent.kind != AgentKind::Hermes
        || agent.id != run.agent_id
        || approval.agent_id != agent.id
        || approval.session_id != run.session_id
        || approval.session_run_id != run.id
        || decision.session_id != run.session_id
        || decision.session_run_id != run.id
        || decision.approval_id != approval.id
    {
        return Err(AppError::conflict("original approval scope changed"));
    }
    // An existing context is history, never another POST permit or a new epoch.
    if let Some(saved) = supervisor.repo.get_approval_outcome(decision.id).await? {
        return Ok(saved.decision);
    }
    let accepted = targeted_approval::pending(supervisor, agent, run, approval).await?;
    let context = control_outcome_wire::prepare(
        &supervisor.client,
        &accepted.base,
        &accepted.token,
        decision.id,
        &approval.runtime_run_id,
        control_outcome_wire::Request::Approval {
            request_id: approval.runtime_approval_id.as_deref().unwrap(),
            choice: decision.choice,
        },
    )
    .await?;
    context.verify_approval(
        decision,
        approval,
        &accepted.base,
        &hermes_wire::credential_fingerprint(&accepted.token),
    )?;
    let saved = serde_json::to_value(&context).map_err(AppError::internal)?;
    if !supervisor
        .repo
        .claim_approval_outcome(decision.id, saved.clone())
        .await?
    {
        return supervisor
            .repo
            .approval_decision(decision.session_id, decision.approval_id)
            .await;
    }
    let ack = control_outcome_wire::send_approval(
        &supervisor.client,
        &context,
        &accepted.base,
        &accepted.token,
    )
    .await?;
    if ack != control_outcome_wire::Acknowledgement::ApprovalResolved {
        return Err(AppError::Unavailable(
            "original approval ACK is not verified".into(),
        ));
    }
    supervisor
        .repo
        .finish_approval_outcome(decision.id, saved)
        .await
}

impl LocalRuntimeSupervisor {
    pub(super) fn spawn_approval_outcome_readback(&self) {
        if !self.config.fleet.hermes_control_outcome_enabled {
            return;
        }
        let supervisor = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let mut after = None;
                loop {
                    match supervisor.repo.list_approval_outcomes(after).await {
                        Ok(records) => {
                            if records.is_empty() {
                                after = None;
                            }
                            for intent in records {
                                // Advance even on an invalid historical context, avoiding head-of-line starvation.
                                let decision_id = intent.decision.id;
                                after = Some(decision_id);
                                if supervisor.finish_approval_outcome_readback(intent).await.is_err() {
                                    tracing::warn!(%decision_id, "Approval outcome requires reconciliation");
                                }
                            }
                        }
                        Err(_) => tracing::warn!("Approval outcome queue is unavailable"),
                    }
                    sleep(Duration::from_secs(5)).await;
                }
            });
        }
    }

    async fn finish_approval_outcome_readback(
        &self,
        intent: app::ApprovalOutcomeIntent,
    ) -> Result<(), AppError> {
        let approval = &intent.approval;
        let agent = self.repo.get_agent(approval.agent_id).await?;
        if agent.status == AgentStatus::Archived {
            return Err(AppError::conflict(
                "archived approval outcome requires reconciliation",
            ));
        }
        let run = self
            .repo
            .get_session_agent_run(intent.decision.session_run_id)
            .await?;
        if run.session_id != approval.session_id
            || run.agent_id != approval.agent_id
            || run.runtime_run_id.as_deref() != Some(approval.runtime_run_id.as_str())
        {
            return Err(AppError::conflict("approval outcome scope changed"));
        }
        let accepted = native_context::accepted(self, &agent, &run).await?;
        let context: control_outcome_wire::Context = serde_json::from_value(intent.context.clone())
            .map_err(|_| AppError::conflict("invalid original approval context"))?;
        context.verify_approval(
            &intent.decision,
            approval,
            &accepted.base,
            &hermes_wire::credential_fingerprint(&accepted.token),
        )?;
        if control_outcome_wire::lookup(&self.client, &context, &accepted.base, &accepted.token)
            .await?
            == control_outcome_wire::Outcome::Acknowledged(
                control_outcome_wire::Acknowledgement::ApprovalResolved,
            )
        {
            self.repo
                .finish_approval_outcome(intent.decision.id, intent.context)
                .await?;
        }
        // Missing/uncertain witnesses leave the claim intact. Terminal state cannot prove a decision.
        Ok(())
    }
}
