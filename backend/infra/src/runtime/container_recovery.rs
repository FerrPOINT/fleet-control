use super::container_control::{ContainerControl, ContainerLaunchFiles, canonical_hash};
use super::*;
use app::container_runtime::{ContainerLaunch, ContainerRecoveryCommand, ContainerRecoveryRequest};

fn held() -> AppError {
    AppError::Unavailable("Original controller custody remains held".into())
}

fn expires() -> String {
    (chrono::Utc::now() + chrono::Duration::seconds(30))
        .to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
}

fn original_snapshot(
    launch: &ContainerLaunch,
    receipt: &container_control::ContainerReceipt,
) -> Result<(), AppError> {
    if serde_json::to_value(&receipt.snapshot).map_err(|_| held())?
        != launch.snapshot.clone().ok_or_else(held)?
    {
        return Err(held());
    }
    Ok(())
}

impl LocalRuntimeSupervisor {
    pub(super) async fn container_owner_files(
        &self,
        launch: &ContainerLaunch,
    ) -> Result<(ContainerControl, ContainerLaunchFiles), AppError> {
        let (control, mut files) = self.container_files(&launch.prepared.container).await?;
        let recovery = self
            .repo
            .get_container_recovery(launch.prepared.container.registration.generation)
            .await?;
        if let Some(record) = recovery {
            if record.command.request.controller_id != self.controller_id
                || !record.lease_valid
                || record.receipt.is_none()
                || record.lease_receipt.is_none()
            {
                return Err(held());
            }
            files.recovery = Some(record.lease);
            let receipt = control
                .observe(&files, &launch.prepared.container.registration)
                .await?;
            original_snapshot(launch, &receipt)?;
        } else if launch.controller_id != self.controller_id {
            return Err(held());
        }
        Ok((control, files))
    }

    pub(super) async fn advance_container(
        &self,
        launch: &ContainerLaunch,
        state: &str,
    ) -> Result<(), AppError> {
        let (_, files) = self.container_owner_files(launch).await?;
        if let Some(command) = files.recovery {
            self.repo
                .advance_recovered_container(launch, &command, state)
                .await
        } else {
            self.repo
                .advance_container_launch(
                    launch,
                    state,
                    launch.snapshot.clone(),
                    launch.origin.clone(),
                )
                .await
        }
    }

    pub(super) fn spawn_container_recovery(&self) {
        if self.config.fleet.container_control.is_none() {
            return;
        }
        let supervisor = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                loop {
                    if let Ok(agents) = supervisor.repo.list_agents().await {
                        for agent in agents.into_iter().filter(|a| a.kind == AgentKind::Hermes) {
                            let _guard = supervisor.container_custody.lock().await;
                            // Neither a storage ACK nor this worker changes run capacity or runtime_ready.
                            if supervisor.reconcile_container_owner(&agent).await.is_err() {
                                tracing::debug!(agent_id=%agent.id, "Original container custody remains held");
                            }
                        }
                    }
                    sleep(Duration::from_secs(5)).await;
                }
            });
        }
    }

    async fn reconcile_container_owner(&self, agent: &Agent) -> Result<(), AppError> {
        let Some(launch) = self.repo.get_container_launch(agent.id).await? else {
            return Ok(());
        };
        if !matches!(launch.state.as_str(), "running" | "stopping")
            || launch.prepared.container.mapped.is_none()
        {
            return Ok(());
        }
        self.checked_prepared(agent, &launch.prepared).await?;
        let (control, mut files) = self.container_files(&launch.prepared.container).await?;
        let original = &launch.prepared.container.registration;
        let mut prior = self
            .repo
            .get_container_recovery(original.generation)
            .await?;
        if let Some(record) = &mut prior {
            if record.receipt.is_none() {
                files.recovery = Some(record.command.clone());
                // Same-owner retry is Base-idempotent. Another owner may only read the frozen ACK.
                let action = if record.command.request.controller_id == self.controller_id {
                    "recover_controller"
                } else {
                    "read_controller_recovery"
                };
                let ack = control.recovery_action(&files, original, action).await?;
                if ack["witness"]["receipt"]["snapshot"]
                    != launch.snapshot.clone().ok_or_else(held)?
                {
                    return Err(held());
                }
                self.repo
                    .acknowledge_container_recovery(&record.command, ack.clone())
                    .await?;
                record.receipt = Some(ack);
            }
            if record.command.request.controller_id == self.controller_id {
                if !record.lease_valid {
                    return Err(held());
                }
                // Claim the exact next heartbeat before invoking Base. Unknown delivery reuses it.
                if record.lease_receipt.is_some() {
                    let mut next = record.lease.clone();
                    next.lease_version = next.lease_version.checked_add(1).ok_or_else(held)?;
                    next.lease_expires_at = expires();
                    self.repo.claim_container_lease(record, &next).await?;
                    record.lease = next;
                    record.lease_receipt = None;
                }
                files.recovery = Some(record.lease.clone());
                let ack = control
                    .recovery_action(&files, original, "heartbeat_controller")
                    .await?;
                let receipt = control.observe(&files, original).await?;
                original_snapshot(&launch, &receipt)?;
                self.repo
                    .acknowledge_container_lease(
                        &record.lease,
                        json!({"ack":ack,"receipt":receipt}),
                    )
                    .await?;
                return self.finish_recovered_stop(agent, &launch).await;
            }
            if record.lease_receipt.is_none() {
                // Read the exact native acknowledgement, never invent a fresh deadline for a lost owner.
                files.recovery = Some(record.lease.clone());
                let ack = control
                    .recovery_action(&files, original, "heartbeat_controller")
                    .await?;
                let witness = control.controller_restart(&files, original).await?;
                original_snapshot(&launch, &witness.receipt)?;
                self.repo
                    .acknowledge_container_lease(
                        &record.lease,
                        json!({"ack":ack,"receipt":witness.receipt}),
                    )
                    .await?;
                return Ok(());
            }
            if record.lease_valid {
                return Err(held());
            }
        } else if launch.controller_id == self.controller_id {
            return Ok(());
        }
        files.recovery = None;
        let witness = control.controller_restart(&files, original).await?;
        original_snapshot(&launch, &witness.receipt)?;
        let mapping = &files.mapped.as_ref().ok_or_else(held)?.mapping;
        let command = ContainerRecoveryCommand {
            request: ContainerRecoveryRequest {
                id: Uuid::new_v4(),
                launch_id: original.generation,
                agent_id: agent.id,
                original_controller_id: launch.controller_id,
                controller_id: self.controller_id,
                predecessor_id: prior.as_ref().map(|r| r.command.request.id),
                launch_sha256: container_control::launch_hash(&launch)?,
                mapping_sha256: canonical_hash(mapping)?,
                registration_sha256: canonical_hash(original)?,
                controller_snapshot: witness.current_controller_snapshot,
                agent_pid: i32::try_from(
                    witness.receipt.snapshot.as_ref().ok_or_else(held)?.init_pid,
                )
                .map_err(|_| held())?,
            },
            epoch: prior.map_or(Ok(1), |r| r.command.epoch.checked_add(1).ok_or_else(held))?,
            lease_version: 1,
            lease_expires_at: expires(),
        };
        self.repo
            .claim_container_recovery(&launch, &command)
            .await?;
        files.recovery = Some(command.clone());
        let ack = control
            .recovery_action(&files, original, "recover_controller")
            .await?;
        if ack["witness"]["receipt"]["snapshot"] != launch.snapshot.clone().ok_or_else(held)? {
            return Err(held());
        }
        self.repo
            .acknowledge_container_recovery(&command, ack)
            .await?;
        let heartbeat = control
            .recovery_action(&files, original, "heartbeat_controller")
            .await?;
        let receipt = control.observe(&files, original).await?;
        original_snapshot(&launch, &receipt)?;
        self.repo
            .acknowledge_container_lease(&command, json!({"ack":heartbeat,"receipt":receipt}))
            .await?;
        self.finish_recovered_stop(agent, &launch).await
    }

    async fn finish_recovered_stop(
        &self,
        agent: &Agent,
        launch: &ContainerLaunch,
    ) -> Result<(), AppError> {
        if launch.state == "stopping" {
            let _guard = self.container_operations.lock().await;
            self.stop_container(agent).await?;
        }
        Ok(())
    }
}
