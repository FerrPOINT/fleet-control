use super::*;
use app::runtime_launch::{RuntimeLaunchBinding, RuntimeLaunchRecord};

#[derive(Clone, Copy)]
pub(super) enum LaunchPhase {
    Regular,
    Activation(i64),
    Rollback,
}

impl LocalRuntimeSupervisor {
    pub(super) async fn verify_gateway_launch(&self, agent: Uuid) -> Result<(), AppError> {
        self.gateway_launch_generation(agent).await.map(|_| ())
    }

    pub(super) async fn gateway_launch_generation(
        &self,
        agent: Uuid,
    ) -> Result<Option<Uuid>, AppError> {
        let child_pid = self
            .children
            .lock()
            .await
            .get(&agent)
            .and_then(Child::id)
            .and_then(|pid| i32::try_from(pid).ok());
        let Some(persisted) = self.repo.get_open_runtime_launch(agent).await? else {
            // Existing unjournaled runtimes remain legacy, not boundary/admission evidence.
            if self.launches.lock().await.contains_key(&agent) {
                return Err(AppError::Unavailable(
                    "original gateway launch journal is missing".into(),
                ));
            }
            return Ok(None);
        };
        let mut launches = self.launches.lock().await;
        let original = launches.get_mut(&agent).ok_or_else(|| {
            AppError::Unavailable("gateway launch is not owned by this controller".into())
        })?;
        if persisted.state != "gateway_started"
            || !matches!(original.state.as_str(), "claimed" | "gateway_started")
            || child_pid.is_none()
            || child_pid != original.pid
            || persisted.pid != original.pid
            || serde_json::to_value(&persisted.binding).map_err(AppError::internal)?
                != serde_json::to_value(&original.binding).map_err(AppError::internal)?
        {
            return Err(AppError::Unavailable(
                "gateway launch acknowledgement is unresolved".into(),
            ));
        }
        // Reconcile a committed DB ACK only with our retained original child, never a new PID.
        original.state = "gateway_started".into();
        Ok(Some(original.binding.id))
    }

    pub(super) async fn verify_dispatch_launch(
        &self,
        agent: Uuid,
        capabilities: &Value,
    ) -> Result<(), AppError> {
        let expected = crate::runtime_launches::dispatch_launch_id(capabilities)?;
        if self.gateway_launch_generation(agent).await? != expected {
            return Err(AppError::Unavailable(
                "original dispatch gateway generation changed".into(),
            ));
        }
        Ok(())
    }

    pub(super) async fn prepare_native_launch(
        &self,
        agent: &Agent,
        command: &Command,
        phase: LaunchPhase,
    ) -> Result<(), AppError> {
        let (phase, revision) = match phase {
            LaunchPhase::Regular => (
                "regular",
                self.repo.get_effective_config_revision(agent.id).await?,
            ),
            LaunchPhase::Activation(revision) => (
                "activation",
                Some(self.repo.get_config_revision(agent.id, revision).await?),
            ),
            LaunchPhase::Rollback => (
                "rollback",
                self.repo.get_effective_config_revision(agent.id).await?,
            ),
        };
        let command = command.as_std();
        let text = |value: &std::ffi::OsStr| {
            value.to_str().map(str::to_owned).ok_or_else(|| {
                AppError::validation("runtime launch requires UTF-8 command identity")
            })
        };
        let args = command
            .get_args()
            .map(text)
            .collect::<Result<Vec<_>, _>>()?;
        let env = command
            .get_envs()
            .map(|(key, value)| Ok((text(key)?, value.map(text).transpose()?)))
            .collect::<Result<std::collections::BTreeMap<_, _>, AppError>>()?;
        let command_sha256 = crate::runtime_launches::snapshot_hash(&json!({
            "program": text(command.get_program())?, "args": args, "env": env,
            "cwd": command.get_current_dir().map(|path| text(path.as_os_str())).transpose()?,
        }))?;
        let binding = RuntimeLaunchBinding {
            id: Uuid::new_v4(),
            agent_id: agent.id,
            controller_id: self.controller_id,
            kind: agent.kind,
            paths: agent.paths.clone(),
            api_port: agent.api_port,
            phase: phase.into(),
            configuration_revision: revision.as_ref().map(|revision| revision.revision),
            configuration_sha256: revision
                .map(|revision| {
                    let snapshot =
                        serde_json::to_value(revision.snapshot).map_err(AppError::internal)?;
                    crate::runtime_launches::snapshot_hash(&snapshot)
                })
                .transpose()?,
            command_sha256,
        };
        // No process may execute unless this authoritative transaction has committed.
        self.repo.claim_runtime_launch(&binding).await?;
        self.launches.lock().await.insert(
            agent.id,
            RuntimeLaunchRecord {
                binding,
                state: "claimed".into(),
                pid: None,
            },
        );
        Ok(())
    }

    pub(super) async fn record_native_spawn(
        &self,
        agent: Uuid,
        pid: Option<i32>,
    ) -> Result<(), AppError> {
        let mut launches = self.launches.lock().await;
        let launch = launches
            .get_mut(&agent)
            .ok_or_else(|| AppError::Unavailable("original launch identity is missing".into()))?;
        // Preserve the actual child identity even if the database acknowledgement fails.
        launch.pid = pid;
        self.repo
            .observe_runtime_launch(&launch.binding, "gateway_started", pid)
            .await?;
        launch.state = "gateway_started".into();
        Ok(())
    }

    pub(super) async fn record_native_end(
        &self,
        agent: Uuid,
        spawned: bool,
    ) -> Result<(), AppError> {
        let mut launches = self.launches.lock().await;
        if let Some(launch) = launches.get(&agent) {
            self.repo
                .observe_runtime_launch(
                    &launch.binding,
                    if spawned {
                        "gateway_exited"
                    } else {
                        "spawn_failed"
                    },
                    if spawned { launch.pid } else { None },
                )
                .await?;
            launches.remove(&agent);
        } else if self.repo.get_open_runtime_launch(agent).await?.is_some() {
            return Err(AppError::Unavailable(
                "original launch is not owned by this controller".into(),
            ));
        }
        Ok(())
    }
}
