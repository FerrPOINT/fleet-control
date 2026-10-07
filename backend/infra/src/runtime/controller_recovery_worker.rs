use super::{LocalRuntimeSupervisor, container_control};
use app::runtime_launch::{
    ControllerRecoveryCommand, ControllerRecoveryRecord, ControllerRecoveryRequest,
};
use shared::AppError;
use uuid::Uuid;

fn held() -> AppError {
    AppError::Unavailable("controller recovery requires original-key reconciliation".into())
}

impl LocalRuntimeSupervisor {
    /// Trusted recovery entry point, not an HTTP action or a model/SDLC permit.
    /// Readback persists history; restored live effects still require both current leases.
    pub async fn recover_container_controller(
        &self,
        agent_id: Uuid,
    ) -> Result<ControllerRecoveryRecord, AppError> {
        let lock = self.lifecycle_lock(agent_id).await;
        let _guard = lock.lock().await;
        let launch = self
            .repo
            .get_open_runtime_launch(agent_id)
            .await?
            .ok_or_else(held)?;
        if launch.binding.controller_id == self.controller_id
            || launch.state != "gateway_started"
            || launch.pid.is_none()
        {
            return Err(held());
        }
        let agent = self.repo.get_agent(agent_id).await?;
        if agent.kind != launch.binding.kind
            || serde_json::to_value(&agent.paths).map_err(|_| held())?
                != serde_json::to_value(&launch.binding.paths).map_err(|_| held())?
            || agent.api_port != launch.binding.api_port
        {
            return Err(held());
        }
        let container = launch.binding.container.as_ref().ok_or_else(held)?;
        let control = self.container_control(container)?;
        let files = self.container_files(container).await?;
        let journal = files
            .journal
            .parent()
            .ok_or_else(held)?
            .join(format!("{}.controller-epochs.sqlite", launch.binding.id));
        let witness = control
            .observe_controller_restart(&files, &container.registration)
            .await?;
        let previous = self
            .repo
            .current_controller_recovery(launch.binding.id)
            .await?;
        let mut predecessor = None;
        let record = if let Some(record) = previous {
            if record.request.controller_id == self.controller_id {
                if record.request.controller_snapshot != witness.current_controller_snapshot {
                    return Err(held());
                }
                Some(record)
            } else {
                let delivery = self
                    .repo
                    .read_controller_recovery_delivery(record.request.id)
                    .await?
                    .ok_or_else(held)?;
                if !delivery.dispatch_claimed {
                    return Err(held());
                }
                let receipt = control
                    .read_controller_recovery(&files, &launch, &delivery.command, &journal)
                    .await?;
                let settled = self
                    .repo
                    .settle_controller_recovery_outcome(record.request.id, &receipt)
                    .await?;
                if settled.lease_valid
                    || settled.request.controller_snapshot.started_at
                        == witness.current_controller_snapshot.started_at
                {
                    return Err(held());
                }
                predecessor = Some(settled.request.id);
                None
            }
        } else {
            None
        };
        let record = match record {
            Some(record) => record,
            None => {
                self.repo
                    .reserve_controller_recovery(&ControllerRecoveryRequest {
                        id: Uuid::new_v4(),
                        launch_id: launch.binding.id,
                        agent_id,
                        original_controller_id: launch.binding.controller_id,
                        controller_id: self.controller_id,
                        predecessor_id: predecessor,
                        launch_sha256: container_control::canonical_hash(&launch.binding)?,
                        mapping_sha256: container_control::canonical_hash(
                            container.mount_mapping.as_ref().ok_or_else(held)?,
                        )?,
                        registration_sha256: container_control::canonical_hash(
                            &container.registration,
                        )?,
                        controller_snapshot: witness.current_controller_snapshot,
                        agent_pid: launch.pid.ok_or_else(held)?,
                    })
                    .await?
            }
        };
        let delivery = match self
            .repo
            .read_controller_recovery_delivery(record.request.id)
            .await?
        {
            Some(delivery) => delivery,
            None => {
                self.repo
                    .retain_controller_recovery_command(&ControllerRecoveryCommand {
                        request: record.request.clone(),
                        epoch: record.epoch,
                        lease_version: record.lease_version,
                        lease_expires_at: record.lease_expires_at.clone(),
                    })
                    .await?
            }
        };
        if self
            .repo
            .claim_controller_recovery_dispatch(record.request.id, self.controller_id)
            .await?
        {
            // One committed claim precedes this sole native call. An error becomes GET, never a second recover.
            let _unknown_or_ack = control
                .recover_controller(&files, &launch, &delivery.command, &journal)
                .await;
        }
        let receipt = control
            .read_controller_recovery(&files, &launch, &delivery.command, &journal)
            .await?;
        self.repo
            .settle_controller_recovery_outcome(record.request.id, &receipt)
            .await
    }
}
