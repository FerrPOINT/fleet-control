use super::{LocalRuntimeSupervisor, container_control};
use app::runtime_launch::{
    ControllerRecoveryCommand, ControllerRecoveryRecord, ControllerRecoveryRequest,
};
use shared::AppError;
use std::{collections::HashSet, time::Duration};
use uuid::Uuid;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
const RECOVERY_CYCLE_TIMEOUT: Duration = Duration::from_secs(8);

fn held() -> AppError {
    AppError::Unavailable("controller recovery requires original-key reconciliation".into())
}

impl LocalRuntimeSupervisor {
    pub(super) fn spawn_controller_recovery(&self) {
        if !self.config.fleet.controller_recovery_enabled
            || self
                .config
                .fleet
                .container_control
                .as_ref()
                .and_then(|control| control.bridge_controller.as_ref())
                .is_none()
        {
            return;
        }
        let supervisor = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let mut ticks = tokio::time::interval(HEARTBEAT_INTERVAL);
                ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                let mut jobs = tokio::task::JoinSet::new();
                let mut active = HashSet::new();
                loop {
                    tokio::select! {
                        result = jobs.join_next(), if !jobs.is_empty() => {
                            match result {
                                Some(Ok((id, true))) => { active.remove(&id); }
                                Some(Ok((id, false))) => {
                                    active.remove(&id);
                                    tracing::warn!(agent_id=%id, "controller recovery cycle remains held");
                                }
                                _ => tracing::warn!("controller recovery task failed; custody remains held"),
                            }
                        }
                        _ = ticks.tick() => {
                            let Ok(agents) = supervisor.repo.list_agents().await else { continue; };
                            for agent in agents {
                                if agent.kind != domain::AgentKind::Hermes || !active.insert(agent.id) {
                                    continue;
                                }
                                let worker = supervisor.clone();
                                jobs.spawn(async move {
                                    let result = tokio::time::timeout(RECOVERY_CYCLE_TIMEOUT,
                                        worker.reconcile_controller_recovery(agent.id)).await;
                                    (agent.id, matches!(result, Ok(Ok(()))))
                                });
                            }
                        }
                    }
                }
            });
        }
    }

    pub(super) async fn reconcile_controller_recovery(
        &self,
        agent_id: Uuid,
    ) -> Result<(), AppError> {
        if !self.config.fleet.controller_recovery_enabled {
            return Ok(());
        }
        let Some(launch) = self.repo.get_open_runtime_launch(agent_id).await? else {
            return Ok(());
        };
        if launch.binding.controller_id == self.controller_id || launch.binding.container.is_none()
        {
            return Ok(());
        }
        let record = self.recover_container_controller(agent_id).await?;
        if record.state != "acknowledged" || !record.lease_valid {
            return Err(held());
        }
        self.heartbeat_container_controller(agent_id)
            .await
            .map(|_| ())
    }

    /// Reconcile the last native version before renewing PostgreSQL; never skip an uncertain heartbeat.
    pub async fn heartbeat_container_controller(
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
        let record = self
            .repo
            .current_controller_recovery(launch.binding.id)
            .await?
            .ok_or_else(held)?;
        if record.request.controller_id != self.controller_id
            || record.state != "acknowledged"
            || !record.lease_valid
            || record.native_receipt_sha256.is_none()
        {
            return Err(held());
        }
        let agent = self.repo.get_agent(agent_id).await?;
        if agent.kind != launch.binding.kind
            || agent.api_port != launch.binding.api_port
            || serde_json::to_value(&agent.paths).map_err(|_| held())?
                != serde_json::to_value(&launch.binding.paths).map_err(|_| held())?
        {
            return Err(held());
        }
        let delivery = self
            .repo
            .read_controller_recovery_delivery(record.request.id)
            .await?
            .ok_or_else(held)?;
        let receipt = delivery.native_receipt.as_ref().ok_or_else(held)?;
        if !delivery.dispatch_claimed
            || delivery.command.request != record.request
            || delivery.command.epoch != record.epoch
            || delivery.native_receipt_sha256 != record.native_receipt_sha256
            || record.native_receipt_sha256.as_deref()
                != Some(container_control::canonical_hash(receipt)?.as_str())
        {
            return Err(held());
        }
        super::controller_recovery_wire::validate_receipt(receipt, &delivery.command, &launch)?;
        let container = launch.binding.container.as_ref().ok_or_else(held)?;
        let control = self.container_control(container)?;
        let files = self.container_files(container).await?;
        let journal = files
            .journal
            .parent()
            .ok_or_else(held)?
            .join(format!("{}.controller-epochs.sqlite", launch.binding.id));
        let current = ControllerRecoveryCommand {
            request: record.request.clone(),
            epoch: record.epoch,
            lease_version: record.lease_version,
            lease_expires_at: record.lease_expires_at.clone(),
        };
        // Exact replay catches up a native write whose reply or DB acknowledgement was lost.
        control
            .heartbeat_controller(&files, &launch, &current, &journal)
            .await?;
        control
            .verify_live_controller(&files, &launch, &current, &journal, receipt)
            .await?;
        let renewed = self
            .repo
            .heartbeat_controller_recovery(
                record.request.id,
                self.controller_id,
                record.lease_version,
            )
            .await?;
        let next = ControllerRecoveryCommand {
            request: renewed.request.clone(),
            epoch: renewed.epoch,
            lease_version: renewed.lease_version,
            lease_expires_at: renewed.lease_expires_at.clone(),
        };
        control
            .heartbeat_controller(&files, &launch, &next, &journal)
            .await?;
        control
            .verify_live_controller(&files, &launch, &next, &journal, receipt)
            .await?;
        let latest = self
            .repo
            .current_controller_recovery(launch.binding.id)
            .await?
            .ok_or_else(held)?;
        if latest.request != renewed.request
            || latest.epoch != renewed.epoch
            || latest.state != "acknowledged"
            || !latest.lease_valid
            || latest.lease_version != renewed.lease_version
            || latest.lease_expires_at != renewed.lease_expires_at
            || latest.native_receipt_sha256 != renewed.native_receipt_sha256
        {
            return Err(held());
        }
        Ok(latest)
    }

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
