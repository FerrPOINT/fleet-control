//! Ownership TTL maintenance is independent of the model and SSE connection.
use super::*;

pub(super) fn spawn(supervisor: &LocalRuntimeSupervisor) {
    if !supervisor.config.pm.workflow.enabled {
        return;
    }
    let supervisor = supervisor.clone();
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    handle.spawn(async move {
        let mut ticks = tokio::time::interval(Duration::from_secs(5));
        ticks.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticks.tick().await;
            let operations = match supervisor.repo.list_pm_lease_operations().await {
                Ok(operations) => operations,
                Err(_) => {
                    tracing::warn!("PM lease maintenance inventory unavailable");
                    continue;
                }
            };
            futures_util::stream::iter(operations).for_each_concurrent(Some(8), |operation| {
                let supervisor = &supervisor;
                async move {
                    let Ok(agent) = supervisor.repo.get_agent(operation.request.agent_id).await else { return };
                    let Some(session) = operation.session_id else { return };
                    let Ok(runs) = supervisor.repo.active_pm_runs(session).await else { return };
                    let Ok(identity) = operation.execution_identity() else { return };
                    if runs.iter().any(|run| run.reservation.identity != identity) { return }
                    let original = runs.first().and_then(|run| run.reservation.runtime_binding.as_ref());
                    if let Some(original) = original {
                        // An old task must never stop a later task using this agent.
                        if pm_readback::capture(supervisor, &agent).await.ok().as_ref() != Some(original) { return }
                    }
                    let outcome = tokio::time::timeout(Duration::from_secs(10), async {
                        let coordinator = crate::pm_credentials::PmCredentialCoordinator::configured(&supervisor.config)?
                            .ok_or_else(|| AppError::Unavailable("PM credentials unavailable".into()))?;
                        let current = supervisor.repo.read_pm_draft_operation(operation.id, operation.owner_user_id).await?;
                        let credential = coordinator.prepare_credential(supervisor.repo.as_ref(), &current).await?;
                        coordinator.renew_execution_lease(&current, &credential).await?;
                        Ok::<_, AppError>(())
                    }).await;
                    if matches!(outcome, Ok(Ok(()))) {
                        if let Some(session) = operation.session_id {
                            let recovery = async {
                                let workflow = crate::pm_workflow::PmWorkflowClient::configured(&supervisor.config)?
                                    .ok_or_else(|| AppError::Unavailable("PM Workflow unavailable".into()))?;
                                for run in supervisor.repo.pending_pm_checkpoints(session).await? {
                                    let record = supervisor.repo.get_pm_run(run).await?;
                                    if record.reservation.runtime_binding.as_ref() != pm_readback::capture(supervisor, &agent).await.ok().as_ref()
                                        || record.reservation.identity != operation.execution_identity()? {
                                        return Err(AppError::conflict("PM checkpoint recovery context changed"));
                                    }
                                    workflow.reconcile_checkpoint(supervisor.repo.as_ref(), &record).await?;
                                }
                                pm_resume::prepare_answers(supervisor,&agent,&operation).await?;
                                Ok::<_, AppError>(())
                            };
                            if !matches!(tokio::time::timeout(Duration::from_secs(10), recovery).await, Ok(Ok(()))) {
                                tracing::warn!(agent_id=%agent.id, "PM original checkpoint outcome requires reconciliation");
                            }
                        }
                        return;
                    }
                    let Some(binding) = original else {
                        tracing::warn!(agent_id=%agent.id, "PM lease unavailable; no original active run to stop");
                        return;
                    };
                    let lock = supervisor.lifecycle_lock(agent.id).await;
                    let _guard = lock.lock().await;
                    let Ok(current_runs) = supervisor.repo.active_pm_runs(session).await else { return };
                    if current_runs.len() != 1
                        || current_runs[0].reservation.identity != identity
                        || current_runs[0].reservation.runtime_binding.as_ref() != Some(binding) {
                        return;
                    }
                    if supervisor.gateway_launch_generation(agent.id).await.ok().flatten() != Some(binding.launch_id) {
                        return;
                    }
                    // Process/container stop already proves descendant quiescence. A
                    // failed stop retains the reservation; it is never a terminal ACK.
                    if supervisor.stop_locked(&agent).await.is_err() {
                        tracing::warn!(agent_id=%agent.id, "PM lease lost; original runtime stop unconfirmed");
                    } else {
                        tracing::warn!(agent_id=%agent.id, "PM lease lost; original runtime stopped");
                    }
                }
            }).await;
        }
    });
}
