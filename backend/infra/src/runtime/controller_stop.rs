use super::{LocalRuntimeSupervisor, container_control};
use app::runtime_launch::{
    ControllerRecoveryCommand, ControllerRecoveryRecord, RuntimeLaunchRecord,
};
use domain::{Agent, AgentStatus, RuntimeOperationResponse};
use serde_json::json;
use shared::AppError;

fn held() -> AppError {
    AppError::Unavailable(
        "original namespace stop requires current controller reconciliation".into(),
    )
}

impl LocalRuntimeSupervisor {
    async fn current_stop_owner(
        &self,
        agent: &Agent,
    ) -> Result<(RuntimeLaunchRecord, ControllerRecoveryRecord), AppError> {
        if !self.config.fleet.controller_recovery_enabled {
            return Err(held());
        }
        let launch = self
            .repo
            .get_open_runtime_launch(agent.id)
            .await?
            .ok_or_else(held)?;
        let owner = self
            .repo
            .current_controller_recovery(launch.binding.id)
            .await?
            .ok_or_else(held)?;
        if launch.state != "gateway_started"
            || !launch.controller_recovery
            || launch.pid.is_none()
            || owner.request.controller_id != self.controller_id
            || owner.state != "acknowledged"
            || !owner.lease_valid
            || owner.native_receipt_sha256.is_none()
            || agent.kind != launch.binding.kind
            || agent.api_port != launch.binding.api_port
            || serde_json::to_value(&agent.paths).map_err(|_| held())?
                != serde_json::to_value(&launch.binding.paths).map_err(|_| held())?
        {
            return Err(held());
        }
        let saved = self
            .repo
            .read_controller_recovery_delivery(owner.request.id)
            .await?
            .ok_or_else(held)?;
        let receipt = saved.native_receipt.as_ref().ok_or_else(held)?;
        if !saved.dispatch_claimed
            || saved.command.request != owner.request
            || saved.command.epoch != owner.epoch
            || saved.native_receipt_sha256 != owner.native_receipt_sha256
            || owner.native_receipt_sha256.as_deref()
                != Some(container_control::canonical_hash(receipt)?.as_str())
        {
            return Err(held());
        }
        super::controller_recovery_wire::validate_receipt(receipt, &saved.command, &launch)?;
        Ok((launch, owner))
    }

    async fn stop_owner_readback(
        &self,
        agent: &Agent,
        launch: &RuntimeLaunchRecord,
        owner: &ControllerRecoveryRecord,
    ) -> Result<(), AppError> {
        let fresh_agent = self.repo.get_agent(agent.id).await?;
        let (fresh_launch, fresh_owner) = self.current_stop_owner(&fresh_agent).await?;
        if serde_json::to_value(&fresh_launch.binding).map_err(|_| held())?
            != serde_json::to_value(&launch.binding).map_err(|_| held())?
            || fresh_launch.pid != launch.pid
            || fresh_owner.request != owner.request
            || fresh_owner.epoch != owner.epoch
            || fresh_owner.lease_version != owner.lease_version
            || fresh_owner.lease_expires_at != owner.lease_expires_at
            || fresh_owner.native_receipt_sha256 != owner.native_receipt_sha256
        {
            return Err(held());
        }
        Ok(())
    }

    /// Namespace containment only: it never grants model, control or SDLC execution authority.
    pub(super) async fn stop_recovered_container_locked(
        &self,
        agent: &Agent,
    ) -> Result<RuntimeOperationResponse, AppError> {
        if !self.config.fleet.controller_recovery_enabled {
            return Err(held());
        }
        let original = self
            .repo
            .get_open_runtime_launch(agent.id)
            .await?
            .ok_or_else(held)?;
        if self
            .repo
            .read_controller_stop(original.binding.id)
            .await?
            .is_some_and(|delivery| delivery.dispatch_command.is_some())
        {
            // Historical readback needs no live effects permit and cannot issue another stop.
            if agent.kind != original.binding.kind
                || agent.api_port != original.binding.api_port
                || serde_json::to_value(&agent.paths).map_err(|_| held())?
                    != serde_json::to_value(&original.binding.paths).map_err(|_| held())?
            {
                return Err(held());
            }
            let container = original.binding.container.as_ref().ok_or_else(held)?;
            let witness = self
                .container_control(container)?
                .observe_controller_restart(
                    &self.container_files(container).await?,
                    &container.registration,
                )
                .await?;
            if witness.receipt.observation
                != container_control::ContainerObservation::NamespaceExited
            {
                return Err(held());
            }
            self.repo
                .settle_controller_stop(
                    original.binding.id,
                    &json!({"kind":"observe","receipt":witness.receipt}),
                )
                .await?;
            return Ok(self.confirmed_recovered_stop(agent).await);
        }
        let (launch, owner) = self.current_stop_owner(agent).await?;
        let command = ControllerRecoveryCommand {
            request: owner.request.clone(),
            epoch: owner.epoch,
            lease_version: owner.lease_version,
            lease_expires_at: owner.lease_expires_at.clone(),
        };
        let container = launch.binding.container.as_ref().ok_or_else(held)?;
        let files = self.container_files(container).await?;
        let control = self.container_control(container)?;
        let journal = files
            .journal
            .parent()
            .ok_or_else(held)?
            .join(format!("{}.controller-epochs.sqlite", launch.binding.id));
        // Native observe checks the exact live boot-clock lease and fresh physical controller.
        let observed = control
            .observe_with_owner(&files, &container.registration, &command, &journal)
            .await?;
        self.stop_owner_readback(agent, &launch, &owner).await?;
        let delivery = self.repo.retain_controller_stop(&command).await?;
        let snapshot_hash =
            container_control::canonical_hash(observed.snapshot.as_ref().ok_or_else(held)?)?;
        if observed.state != container_control::ContainerReceiptState::Observed
            || snapshot_hash != delivery.intent.snapshot_sha256
        {
            return Err(held());
        }
        let claimed = self.repo.claim_controller_stop(&command).await?;
        let outcome =
            if observed.observation == container_control::ContainerObservation::NamespaceExited {
                json!({"kind":"observe","receipt":observed})
            } else if claimed
                && observed.observation == container_control::ContainerObservation::Running
            {
                self.stop_owner_readback(agent, &launch, &owner).await?;
                // The durable claim survives a lost reply; retries are read-only, never a second kill.
                let receipt = control
                    .stop_with_owner(
                        &files,
                        &container.registration,
                        &command,
                        &journal,
                        &delivery.intent,
                    )
                    .await?;
                json!({"kind":"stop","receipt":receipt})
            } else {
                return Err(held());
            };
        self.repo
            .settle_controller_stop(launch.binding.id, &outcome)
            .await?;
        Ok(self.confirmed_recovered_stop(agent).await)
    }

    async fn confirmed_recovered_stop(&self, agent: &Agent) -> RuntimeOperationResponse {
        self.launches.lock().await.remove(&agent.id);
        RuntimeOperationResponse {
            agent_id: agent.id,
            status: AgentStatus::Stopped,
            message:
                "Original agent namespace exit confirmed; session reconciliation remains separate"
                    .into(),
        }
    }
}
