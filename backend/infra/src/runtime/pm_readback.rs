use super::*;
use domain::{PmRunRecord, PmRuntimeStatus};

pub(super) async fn capture(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
) -> Result<domain::PmRuntimeBinding, AppError> {
    let lock = supervisor.lifecycle_lock(agent.id).await;
    let _guard = lock.lock().await;
    let current = supervisor.repo.get_agent(agent.id).await?;
    let binding = domain::PmRuntimeBinding {
        launch_id: supervisor
            .gateway_launch_generation(agent.id)
            .await?
            .ok_or_else(|| {
                AppError::Unavailable("PM requires an acknowledged original launch".into())
            })?,
        controller_id: supervisor.controller_id,
        origin: supervisor.hermes_base_url(&current).await?,
        credential_fingerprint: hermes_wire::credential_fingerprint(&crate::agent_runtime_token(
            &supervisor.config,
            agent.id,
        )?),
    };
    verify_binding(supervisor, &current, &binding).await?;
    Ok(binding)
}

pub(super) async fn probe(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    record: &PmRunRecord,
) -> Result<PmRuntimeStatus, AppError> {
    record.reservation.validate()?;
    if agent.kind != AgentKind::Hermes || agent.id != record.reservation.identity.agent_id()? {
        return Err(AppError::Forbidden);
    }
    let binding =
        record.reservation.runtime_binding.as_ref().ok_or_else(|| {
            AppError::Unavailable("PM original runtime binding is missing".into())
        })?;
    let lock = supervisor.lifecycle_lock(agent.id).await;
    let _guard = lock.lock().await;
    verify_binding(supervisor, agent, binding).await?;
    let raw_id = record
        .hermes_run_ref
        .as_deref()
        .ok_or_else(|| AppError::Unavailable("PM acceptance is unknown".into()))?;
    if !crate::pm_execution::valid_hermes_ref(raw_id) {
        return Err(AppError::Unavailable(
            "PM runtime reference is invalid".into(),
        ));
    }
    let run = supervisor
        .repo
        .get_session_agent_run(record.reservation.session_run_id)
        .await?;
    if run.session_id != record.reservation.session_id
        || run.agent_id != agent.id
        || run.runtime_run_id.as_deref() != Some(raw_id)
        || run.runtime_session_id.as_deref()
            != Some(record.reservation.runtime_session_id().as_str())
    {
        return Err(AppError::Unavailable(
            "PM runtime mapping does not match reservation".into(),
        ));
    }
    // A fresh authenticated read, never a cached Fleet status or an SSE EOF.
    let response = supervisor
        .client
        .get(format!("{}/v1/runs/{raw_id}", binding.origin))
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .timeout(Duration::from_secs(10))
        .bearer_auth(crate::agent_runtime_token(&supervisor.config, agent.id)?)
        .send()
        .await
        .map_err(|_| AppError::Unavailable("Hermes PM readback is unavailable".into()))?;
    if !response.status().is_success() {
        return Err(AppError::Unavailable(
            "Hermes PM readback did not return a run".into(),
        ));
    }
    let payload = hermes_wire::read_json(response, reqwest::StatusCode::OK, 1024 * 1024).await?;
    // Hermes may resolve a Fleet session alias to its own persistent UUID.
    // Dispatch pins that effective ID; later probes cannot change it.
    let effective_session = record.hermes_session_ref.as_deref().ok_or_else(|| {
        AppError::Unavailable("PM effective session acceptance is unknown".into())
    })?;
    let status = status(&payload, raw_id, effective_session)?;
    verify_binding(supervisor, agent, binding).await?;
    // Keep lifecycle exclusion through the DB fence and terminal/capacity commit.
    supervisor
        .repo
        .observe_pm_run(record, status, supervisor)
        .await?;
    Ok(status)
}

async fn verify_binding(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    binding: &domain::PmRuntimeBinding,
) -> Result<(), AppError> {
    binding.validate()?;
    let current = supervisor.repo.get_agent(agent.id).await?;
    let token = crate::agent_runtime_token(&supervisor.config, agent.id)?;
    if binding.controller_id != supervisor.controller_id
        || supervisor.gateway_launch_generation(agent.id).await? != Some(binding.launch_id)
        || current.kind != AgentKind::Hermes
        || current.sdlc_role != Some(domain::SdlcRole::ProjectManager)
        || supervisor.hermes_base_url(&current).await? != binding.origin
        || hermes_wire::credential_fingerprint(&token) != binding.credential_fingerprint
    {
        return Err(AppError::Unavailable(
            "PM original runtime context changed".into(),
        ));
    }
    Ok(())
}

#[async_trait::async_trait]
impl app::PmRuntimeCustody for LocalRuntimeSupervisor {
    async fn verify(
        &self,
        binding: &domain::PmRuntimeBinding,
        launch: &app::runtime_launch::RuntimeLaunchBinding,
        pid: i32,
    ) -> Result<(), AppError> {
        binding.validate()?;
        let agent_id = launch.agent_id;
        let held = || AppError::Unavailable("PM original runtime custody changed".into());
        // Locked provenance comes from the caller; nested repository reads could exhaust its pool.
        let original = self
            .launches
            .try_lock()
            .map_err(|_| held())?
            .get(&agent_id)
            .cloned()
            .ok_or_else(held)?;
        if binding.controller_id != self.controller_id
            || binding.launch_id != launch.id
            || launch.controller_id != self.controller_id
            || launch.kind != AgentKind::Hermes
            || original.controller_recovery
            || !matches!(original.state.as_str(), "claimed" | "gateway_started")
            || serde_json::to_value(&original.binding).map_err(AppError::internal)?
                != serde_json::to_value(launch).map_err(AppError::internal)?
            || hermes_wire::credential_fingerprint(&crate::agent_runtime_token(
                &self.config,
                agent_id,
            )?) != binding.credential_fingerprint
        {
            return Err(held());
        }
        if launch.container.is_some() {
            let record = app::runtime_launch::RuntimeLaunchRecord {
                binding: launch.clone(),
                state: "gateway_started".into(),
                pid: Some(pid),
                controller_recovery: false,
            };
            let receipt = self.observe_container(&record).await?;
            if receipt.state != container_control::ContainerReceiptState::Observed
                || receipt.observation != container_control::ContainerObservation::Running
                || receipt
                    .snapshot
                    .as_ref()
                    .and_then(|snapshot| i32::try_from(snapshot.init_pid).ok())
                    != Some(pid)
            {
                return Err(held());
            }
        } else {
            let mut children = self.children.try_lock().map_err(|_| held())?;
            let child = children.get_mut(&agent_id).ok_or_else(held)?;
            if child.try_wait().map_err(AppError::internal)?.is_some()
                || child.id().and_then(|id| i32::try_from(id).ok()) != Some(pid)
                || original.pid != Some(pid)
            {
                return Err(held());
            }
        }
        Ok(())
    }
}

fn status(payload: &Value, raw_id: &str, session_id: &str) -> Result<PmRuntimeStatus, AppError> {
    if payload.get("object").and_then(Value::as_str) != Some("hermes.run")
        || payload.get("run_id").and_then(Value::as_str) != Some(raw_id)
        || payload.get("session_id").and_then(Value::as_str) != Some(session_id)
    {
        return Err(AppError::Unavailable(
            "Hermes PM readback identity mismatch".into(),
        ));
    }
    match payload.get("status").and_then(Value::as_str) {
        Some("queued" | "started" | "running" | "waiting_for_approval" | "stopping") => {
            Ok(PmRuntimeStatus::Running)
        }
        Some("completed") => {
            hermes_wire::terminal_readback(payload, raw_id)?;
            Ok(PmRuntimeStatus::Completed)
        }
        Some("failed" | "interrupted") => Ok(PmRuntimeStatus::Failed),
        Some("cancelled") => Ok(PmRuntimeStatus::Cancelled),
        Some("stopped") => Ok(PmRuntimeStatus::Stopped),
        _ => Err(AppError::Unavailable(
            "Hermes PM run status is unknown".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn value(state: &str) -> Value {
        json!({"object":"hermes.run","run_id":"run_test","session_id":"fleet:test:agent","status":state,
            "completed":state == "completed","partial":false,"interrupted":state == "interrupted"})
    }
    #[test]
    fn only_runtime_terminal_status_is_proof() {
        for state in [
            "queued",
            "started",
            "running",
            "waiting_for_approval",
            "stopping",
        ] {
            assert_eq!(
                status(&value(state), "run_test", "fleet:test:agent").unwrap(),
                PmRuntimeStatus::Running
            );
        }
        assert_eq!(
            status(&value("interrupted"), "run_test", "fleet:test:agent").unwrap(),
            PmRuntimeStatus::Failed
        );
        assert_eq!(
            status(&value("completed"), "run_test", "fleet:test:agent").unwrap(),
            PmRuntimeStatus::Completed
        );
        for state in ["eof", "success", "succeeded", "unknown", ""] {
            assert!(status(&value(state), "run_test", "fleet:test:agent").is_err());
        }
    }
    #[test]
    fn wrong_or_missing_run_identity_fails_closed() {
        let payload = value("completed");
        assert!(status(&payload, "run_other", "fleet:test:agent").is_err());
        assert!(status(&payload, "run_test", "fleet:other:agent").is_err());
        let mut missing = payload.clone();
        missing.as_object_mut().unwrap().remove("session_id");
        assert!(status(&missing, "run_test", "fleet:test:agent").is_err());
        assert!(status(&Value::Null, "run_test", "fleet:test:agent").is_err());
    }

    #[test]
    fn pm_completion_cannot_hide_partial_or_interrupted_output() {
        for (field, bad) in [
            ("completed", json!(false)),
            ("partial", json!(true)),
            ("interrupted", json!(true)),
        ] {
            let mut payload = value("completed");
            payload[field] = bad;
            assert!(status(&payload, "run_test", "fleet:test:agent").is_err());
        }
        let mut payload = value("completed");
        payload.as_object_mut().unwrap().remove("completed");
        assert!(status(&payload, "run_test", "fleet:test:agent").is_err());
    }
}
