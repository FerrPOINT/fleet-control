use super::*;
use app::runtime_launch::{RuntimeContainerBinding, RuntimeLaunchBinding, RuntimeLaunchRecord};
use container_control::{
    ContainerControl, ContainerLaunchFiles, ContainerObservation, ContainerReceipt,
    ContainerReceiptState, ControlSource,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::io::AsyncWriteExt;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PreparedContainer {
    agent_id: Uuid,
    paths: domain::AgentPaths,
    api_port: Option<i32>,
    configuration_revision: Option<i64>,
    configuration_sha256: Option<String>,
    container: RuntimeContainerBinding,
}

// Resolved runtime credentials are only persisted in private controller storage.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContainerCreationIntent {
    agent_id: Uuid,
    generation: Uuid,
    operation_id: Uuid,
    paths: domain::AgentPaths,
    api_port: Option<i32>,
    configuration_revision: Option<i64>,
    configuration_sha256: Option<String>,
    policy: Value,
    process: container_control::ContainerProcess,
    source_sha256: [String; 3],
    context: String,
}

async fn private_document(
    root: &Path,
    path: &Path,
    value: &impl Serialize,
) -> Result<(), AppError> {
    if path.parent() != Some(root) {
        return Err(held());
    }
    crate::reject_symlink_components(root, path)
        .await
        .map_err(|_| held())?;
    let bytes = serde_json::to_vec(value).map_err(|_| held())?;
    if bytes.len() > 64 * 1024 {
        return Err(held());
    }
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path).await.map_err(|_| held())?;
    file.write_all(&bytes).await.map_err(|_| held())?;
    file.sync_all().await.map_err(|_| held())?;
    tokio::fs::File::open(root)
        .await
        .map_err(|_| held())?
        .sync_all()
        .await
        .map_err(|_| held())
}

async fn read_private_document(root: &Path, path: &Path) -> Result<Option<Vec<u8>>, AppError> {
    crate::reject_symlink_components(root, path)
        .await
        .map_err(|_| held())?;
    let meta = match tokio::fs::symlink_metadata(path).await {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(held()),
    };
    if path.parent() != Some(root) || !meta.is_file() || meta.len() > 64 * 1024 {
        return Err(held());
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.mode() & 0o777 != 0o600
            || meta.nlink() != 1
            || meta.uid() != tokio::fs::metadata(root).await.map_err(|_| held())?.uid()
        {
            return Err(held());
        }
    }
    Ok(Some(tokio::fs::read(path).await.map_err(|_| held())?))
}

fn held() -> AppError {
    AppError::Unavailable(
        "original agent container requires reconciliation; no native fallback".into(),
    )
}

impl LocalRuntimeSupervisor {
    async fn create_container(
        &self,
        agent: &Agent,
        root: &Path,
        prepared_path: &Path,
        intent_path: &Path,
        revision: Option<i64>,
        revision_hash: Option<String>,
    ) -> Result<(), AppError> {
        let config = self
            .config
            .fleet
            .container_control
            .as_ref()
            .ok_or_else(held)?;
        let provisioning = config.provisioning.as_ref().ok_or_else(held)?;
        if !crate::runtime_launches::valid_agent_container_project(&provisioning.project) {
            return Err(AppError::validation(
                "agent containers require sdlc1, sdlc2 or an owned temporary QA project",
            ));
        }
        let agents = crate::normalize_path(Path::new(&self.config.fleet.agents_root))?;
        let base = crate::normalize_path(Path::new(&config.base_root))?;
        if root.starts_with(&agents)
            || agents.starts_with(root)
            || base.starts_with(&agents)
            || agents.starts_with(&base)
        {
            return Err(held());
        }
        let previous = read_private_document(root, intent_path).await?;
        let previous: Option<ContainerCreationIntent> = previous
            .map(|bytes| serde_json::from_slice(&bytes).map_err(|_| held()))
            .transpose()?;
        let generation = previous
            .as_ref()
            .map_or_else(Uuid::new_v4, |value| value.generation);
        let operation_id = previous
            .as_ref()
            .map_or_else(Uuid::new_v4, |value| value.operation_id);
        if generation.is_nil() || operation_id.is_nil() {
            return Err(held());
        }
        let service = format!("{}-runtime-{}", agent.name, generation.simple());
        let port = agent
            .api_port
            .filter(|value| (1024..=65535).contains(value))
            .ok_or_else(held)?;
        let intent = ContainerCreationIntent {
            agent_id: agent.id,
            generation,
            operation_id,
            paths: agent.paths.clone(),
            api_port: agent.api_port,
            configuration_revision: revision,
            configuration_sha256: revision_hash,
            policy: json!({"contract_version":2, "project":provisioning.project,"service":service,
                "resource_id":agent.id,"generation":generation,"image_id":provisioning.image_id,
                "task":provisioning.task,"purpose":provisioning.purpose,
                "mounts":[
                    {"type":"bind","source":agent.paths.runtime,"destination":"/runtime","read_only":true},
                    {"type":"bind","source":agent.paths.config,"destination":"/config","read_only":false},
                    {"type":"bind","source":agent.paths.workspace,"destination":"/workspace","read_only":false},
                    {"type":"bind","source":agent.paths.logs,"destination":"/logs","read_only":false}],
                "network":{"id":"0".repeat(64),"name":format!("{}-{service}", provisioning.project),
                    "internal":provisioning.network_internal}}),
            process: container_control::ContainerProcess {
                user: provisioning.user.clone(),
                entrypoint: provisioning.entrypoint.clone(),
                command: vec![
                    "serve".into(),
                    "--host".into(),
                    "0.0.0.0".into(),
                    "--port".into(),
                    port.to_string(),
                ],
                environment: std::collections::BTreeMap::from([
                    ("HOME".into(), "/config".into()),
                    ("HERMES_HOME".into(), "/config".into()),
                    ("HERMES_SERVE_HEADLESS".into(), "1".into()),
                    ("API_SERVER_ENABLED".into(), "true".into()),
                    ("API_SERVER_HOST".into(), "0.0.0.0".into()),
                    ("API_SERVER_PORT".into(), port.to_string()),
                    (
                        "API_SERVER_KEY".into(),
                        crate::agent_runtime_token(&self.config, agent.id)?,
                    ),
                    (
                        "API_SERVER_CORS_ORIGINS".into(),
                        self.config.server.cors_allowed_origins.join(","),
                    ),
                    ("PYTHONDONTWRITEBYTECODE".into(), "1".into()),
                ]),
                working_dir: "/workspace".into(),
                pids_limit: provisioning.pids_limit,
                memory_bytes: provisioning.memory_bytes,
                nano_cpus: provisioning.nano_cpus,
            },
            source_sha256: config.source_sha256.clone(),
            context: config.context.clone(),
        };
        if let Some(previous) = previous {
            if serde_json::to_value(previous).map_err(|_| held())?
                != serde_json::to_value(&intent).map_err(|_| held())?
            {
                return Err(held());
            }
        } else {
            private_document(root, intent_path, &intent).await?;
        }
        let name = format!("{}.{}", agent.id, generation);
        let files = ContainerLaunchFiles {
            policy: intent.policy,
            compose: root.join(format!("{name}.compose.json")),
            journal: root.join(format!("{name}.launch.sqlite")),
            stop_journal: root.join(format!("{name}.stop.sqlite")),
        };
        let control = ContainerControl::new(
            config.python.clone().into(),
            ControlSource {
                root: config.base_root.clone().into(),
                sha256: config.source_sha256.clone(),
            },
            config.context.clone(),
        )?;
        let prepared = control
            .prepare(
                &files,
                &intent.process,
                operation_id,
                &root.join(format!("{name}.create.json")),
                &root.join(format!("{name}.create.sqlite")),
            )
            .await?;
        let document = PreparedContainer {
            agent_id: agent.id,
            paths: agent.paths.clone(),
            api_port: agent.api_port,
            configuration_revision: revision,
            configuration_sha256: intent.configuration_sha256,
            container: RuntimeContainerBinding {
                registration: prepared.registration,
                policy: prepared.policy,
                compose: files.compose.to_string_lossy().into_owned(),
                journal: files.journal.to_string_lossy().into_owned(),
                stop_journal: files.stop_journal.to_string_lossy().into_owned(),
                source_sha256: config.source_sha256.clone(),
                context: config.context.clone(),
            },
        };
        private_document(root, prepared_path, &document).await
    }

    pub(super) fn container_control(
        &self,
        binding: &RuntimeContainerBinding,
    ) -> Result<ContainerControl, AppError> {
        let config = self
            .config
            .fleet
            .container_control
            .as_ref()
            .ok_or_else(held)?;
        if binding.context != config.context || binding.source_sha256 != config.source_sha256 {
            return Err(held());
        }
        ContainerControl::new(
            config.python.clone().into(),
            ControlSource {
                root: config.base_root.clone().into(),
                sha256: config.source_sha256.clone(),
            },
            config.context.clone(),
        )
    }

    async fn container_files(
        &self,
        binding: &RuntimeContainerBinding,
    ) -> Result<ContainerLaunchFiles, AppError> {
        let root = activation_journal::private_controller_directory(Path::new(
            &self.config.fleet.controller_root,
        ))
        .await?;
        let agents = crate::normalize_path(Path::new(&self.config.fleet.agents_root))?;
        if root.starts_with(&agents) || agents.starts_with(&root) {
            return Err(held());
        }
        for value in [&binding.compose, &binding.journal, &binding.stop_journal] {
            let path = Path::new(value);
            if !path.is_absolute() || path.parent() != Some(root.as_path()) {
                return Err(held());
            }
            crate::reject_symlink_components(&root, path)
                .await
                .map_err(|_| held())?;
        }
        Ok(ContainerLaunchFiles {
            policy: binding.policy.clone(),
            compose: binding.compose.clone().into(),
            journal: binding.journal.clone().into(),
            stop_journal: binding.stop_journal.clone().into(),
        })
    }

    pub(super) async fn owned_container_launch(
        &self,
        agent: Uuid,
    ) -> Result<RuntimeLaunchRecord, AppError> {
        let persisted = self
            .repo
            .get_open_runtime_launch(agent)
            .await?
            .ok_or_else(held)?;
        let launches = self.launches.lock().await;
        let original = launches.get(&agent).ok_or_else(held)?;
        if persisted.binding.controller_id != self.controller_id
            || persisted.binding.container.is_none()
            || serde_json::to_value(&persisted.binding).map_err(AppError::internal)?
                != serde_json::to_value(&original.binding).map_err(AppError::internal)?
        {
            return Err(held());
        }
        Ok(persisted)
    }

    async fn observe_container(
        &self,
        launch: &RuntimeLaunchRecord,
    ) -> Result<ContainerReceipt, AppError> {
        let binding = launch.binding.container.as_ref().ok_or_else(held)?;
        self.container_control(binding)?
            .observe(&self.container_files(binding).await?, &binding.registration)
            .await
    }

    pub(super) async fn container_generation(&self, agent: Uuid) -> Result<Uuid, AppError> {
        let launch = self.owned_container_launch(agent).await?;
        let receipt = self.observe_container(&launch).await?;
        if receipt.state != ContainerReceiptState::Observed
            || receipt.observation != ContainerObservation::Running
            || launch.state != "gateway_started"
            || receipt
                .snapshot
                .as_ref()
                .and_then(|v| i32::try_from(v.init_pid).ok())
                != launch.pid
        {
            return Err(held());
        }
        Ok(launch.binding.id)
    }

    pub(super) async fn container_base_url(&self, agent: &Agent) -> Result<String, AppError> {
        self.container_generation(agent.id).await?;
        let launch = self.owned_container_launch(agent.id).await?;
        if launch.binding.api_port != agent.api_port {
            return Err(held());
        }
        let binding = launch.binding.container.as_ref().ok_or_else(held)?;
        let host = self
            .container_control(binding)?
            .endpoint(&self.container_files(binding).await?, &binding.registration)
            .await?;
        let port = agent
            .api_port
            .filter(|port| (1..=65535).contains(port))
            .ok_or_else(held)?;
        Ok(format!("http://{host}:{port}"))
    }

    async fn prepared_container(
        &self,
        agent: &Agent,
        phase: LaunchPhase,
    ) -> Result<RuntimeLaunchBinding, AppError> {
        let agents_root =
            crate::normalize_path(Path::new(&self.config.fleet.agents_root)).map_err(|_| held())?;
        let agent_root = crate::safe_agent_root(&agents_root, &agent.name).map_err(|_| held())?;
        for (area, original) in [
            ("runtime", &agent.paths.runtime),
            ("config", &agent.paths.config),
            ("workspace", &agent.paths.workspace),
            ("logs", &agent.paths.logs),
        ] {
            let path = Path::new(original);
            if !path.is_absolute()
                || crate::normalize_path(path).map_err(|_| held())? != agent_root.join(area)
            {
                return Err(held());
            }
            crate::reject_symlink_components(&agents_root, path)
                .await
                .map_err(|_| held())?;
            if !tokio::fs::symlink_metadata(path)
                .await
                .map_err(|_| held())?
                .is_dir()
            {
                return Err(held());
            }
        }
        if crate::inspect_agent_marker(&agent_root, agent)
            .await
            .map_err(|_| held())?
            != (true, true)
        {
            return Err(held());
        }
        let root = activation_journal::private_controller_directory(Path::new(
            &self.config.fleet.controller_root,
        ))
        .await?;
        let ordinal = self.repo.next_container_launch_ordinal(agent.id).await?;
        let suffix = if ordinal == 0 {
            agent.id.to_string()
        } else {
            format!("{}.{ordinal}", agent.id)
        };
        let path = root.join(format!("{suffix}.container-prepared.json"));
        let intent_path = root.join(format!("{suffix}.container-creation.json"));
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
        let revision_number = revision.as_ref().map(|revision| revision.revision);
        let revision_hash = revision
            .map(|revision| {
                crate::runtime_launches::snapshot_hash(
                    &serde_json::to_value(revision.snapshot).map_err(AppError::internal)?,
                )
            })
            .transpose()?;
        if read_private_document(&root, &path).await?.is_none() {
            if phase != "regular" {
                return Err(held());
            }
            self.create_container(
                agent,
                &root,
                &path,
                &intent_path,
                revision_number,
                revision_hash.clone(),
            )
            .await?;
        }
        crate::reject_symlink_components(&root, &path)
            .await
            .map_err(|_| held())?;
        let metadata = tokio::fs::symlink_metadata(&path)
            .await
            .map_err(|_| held())?;
        if !metadata.is_file() || metadata.len() > 16_384 {
            return Err(held());
        }
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.mode() & 0o777 != 0o600
                || metadata.uid() != tokio::fs::metadata(&root).await.map_err(|_| held())?.uid()
            {
                return Err(held());
            }
        }
        let prepared: PreparedContainer =
            serde_json::from_slice(&tokio::fs::read(path).await.map_err(|_| held())?)
                .map_err(|_| held())?;
        if prepared.agent_id != agent.id
            || serde_json::to_value(&prepared.paths).map_err(AppError::internal)?
                != serde_json::to_value(&agent.paths).map_err(AppError::internal)?
            || prepared.api_port != agent.api_port
            || prepared.configuration_revision != revision_number
            || prepared.configuration_sha256 != revision_hash
        {
            return Err(held());
        }
        let command_sha256 = crate::runtime_launches::snapshot_hash(
            &serde_json::to_value(&prepared.container).map_err(AppError::internal)?,
        )?;
        let binding = RuntimeLaunchBinding {
            id: prepared.container.registration.generation,
            agent_id: agent.id,
            controller_id: self.controller_id,
            kind: agent.kind,
            paths: agent.paths.clone(),
            api_port: agent.api_port,
            phase: phase.into(),
            configuration_revision: revision_number,
            configuration_sha256: revision_hash,
            command_sha256,
            container: Some(prepared.container),
        };
        crate::runtime_launches::validate_container_binding(&binding)?;
        let container = binding.container.as_ref().ok_or_else(held)?;
        let observed = self
            .container_control(container)?
            .observe(
                &self.container_files(container).await?,
                &container.registration,
            )
            .await?;
        if observed.state != ContainerReceiptState::Registered
            || observed.observation != ContainerObservation::NeverStarted
        {
            return Err(held());
        }
        Ok(binding)
    }

    pub(super) async fn start_container_locked(
        &self,
        agent: &Agent,
        phase: LaunchPhase,
    ) -> Result<RuntimeOperationResponse, AppError> {
        if let Some(existing) = self.repo.get_open_runtime_launch(agent.id).await? {
            if existing.binding.container.is_some() {
                return self.health_container_locked(agent).await;
            }
            return Err(held());
        }
        if unconfirmed_runtime(agent) || self.children.lock().await.contains_key(&agent.id) {
            return Err(held());
        }
        let binding = self.prepared_container(agent, phase).await?;
        let container = binding.container.as_ref().ok_or_else(held)?;
        let control = self.container_control(container)?;
        let files = self.container_files(container).await?;
        // This transaction is the last prerequisite before the single protected Docker start.
        self.repo.claim_runtime_launch(&binding).await?;
        self.launches.lock().await.insert(
            agent.id,
            RuntimeLaunchRecord {
                binding: binding.clone(),
                state: "claimed".into(),
                pid: None,
            },
        );
        self.repo
            .update_runtime_state(
                agent.id,
                RuntimeStatePatch {
                    status: AgentStatus::Starting,
                    desired_state: DesiredState::Running,
                    pid: None,
                    health_status: Some("starting".into()),
                    health_detail: Some(
                        "Original container launch committed; start acknowledgement pending".into(),
                    ),
                    last_capabilities_json: None,
                    startup_command_redacted: Some("docker compose start <agent-service>".into()),
                    started_at: None,
                    stopped_at: None,
                },
            )
            .await?;
        let receipt = control.start(&files, &container.registration).await?;
        self.ack_container(&binding, &receipt).await?;
        let capabilities = match self.wait_for_hermes_ready(agent).await {
            Ok(capabilities) => capabilities,
            Err(error) => {
                self.stop_container_locked(agent).await?;
                return Err(error);
            }
        };
        self.container_generation(agent.id).await?;
        let pid = receipt
            .snapshot
            .and_then(|snapshot| i32::try_from(snapshot.init_pid).ok());
        self.repo
            .update_runtime_state(
                agent.id,
                RuntimeStatePatch {
                    status: AgentStatus::Running,
                    desired_state: DesiredState::Running,
                    pid,
                    health_status: Some("running".into()),
                    health_detail: Some("Original agent container and Hermes API are ready".into()),
                    last_capabilities_json: Some(capabilities),
                    startup_command_redacted: Some("docker compose start <agent-service>".into()),
                    started_at: Some(shared::now()),
                    stopped_at: None,
                },
            )
            .await?;
        Ok(RuntimeOperationResponse {
            agent_id: agent.id,
            status: AgentStatus::Running,
            message: "Hermes container started and API is ready".into(),
        })
    }

    async fn ack_container(
        &self,
        binding: &RuntimeLaunchBinding,
        receipt: &ContainerReceipt,
    ) -> Result<(), AppError> {
        if receipt.state != ContainerReceiptState::Observed
            || receipt.observation != ContainerObservation::Running
        {
            return Err(held());
        }
        let pid = receipt
            .snapshot
            .as_ref()
            .and_then(|snapshot| i32::try_from(snapshot.init_pid).ok())
            .ok_or_else(held)?;
        self.repo
            .observe_runtime_launch(binding, "gateway_started", Some(pid))
            .await?;
        let mut launches = self.launches.lock().await;
        let original = launches.get_mut(&binding.agent_id).ok_or_else(held)?;
        original.state = "gateway_started".into();
        original.pid = Some(pid);
        Ok(())
    }

    pub(super) async fn stop_container_locked(
        &self,
        agent: &Agent,
    ) -> Result<RuntimeOperationResponse, AppError> {
        let launch = self.owned_container_launch(agent.id).await?;
        let original = self.observe_container(&launch).await?;
        let snapshot = original.snapshot.as_ref().ok_or_else(held)?;
        if original.state != ContainerReceiptState::Observed {
            return Err(held());
        }
        let pid = i32::try_from(snapshot.init_pid).map_err(|_| held())?;
        if launch.state == "gateway_started" && launch.pid != Some(pid) {
            return Err(held());
        }
        if original.observation == ContainerObservation::Running {
            let container = launch.binding.container.as_ref().ok_or_else(held)?;
            // Generation gives one stable stop identity across retries; Base never repeats an uncertain kill.
            self.container_control(container)?
                .stop(
                    &self.container_files(container).await?,
                    &container.registration,
                    launch.binding.id,
                )
                .await?;
        } else if original.observation != ContainerObservation::NamespaceExited {
            return Err(held());
        }
        self.repo
            .observe_runtime_launch(&launch.binding, "gateway_exited", Some(pid))
            .await?;
        self.launches.lock().await.remove(&agent.id);
        Ok(RuntimeOperationResponse {
            agent_id: agent.id,
            status: AgentStatus::Stopped,
            message: "Original agent container namespace has exited".into(),
        })
    }

    pub(super) async fn health_container_locked(
        &self,
        agent: &Agent,
    ) -> Result<RuntimeOperationResponse, AppError> {
        let launch = self.owned_container_launch(agent.id).await?;
        let receipt = self.observe_container(&launch).await?;
        if receipt.state != ContainerReceiptState::Observed {
            return Err(held());
        }
        if receipt.observation == ContainerObservation::NamespaceExited {
            return self.stop_container_locked(agent).await;
        }
        if receipt.observation != ContainerObservation::Running {
            return Err(held());
        }
        if launch.state == "claimed" {
            // Recover only Base's original durable ACK, not a newly discovered running container.
            self.ack_container(&launch.binding, &receipt).await?;
        }
        self.container_generation(agent.id).await?;
        let capabilities = self.probe_hermes(agent).await;
        let (status, detail, capabilities): (AgentStatus, String, Option<Value>) =
            match capabilities {
                Ok(capabilities) => (
                    AgentStatus::Running,
                    "Original agent container and Hermes API are healthy".into(),
                    Some(capabilities),
                ),
                Err(_) => (
                    AgentStatus::Degraded,
                    "Original agent container is running; Hermes readiness is unavailable".into(),
                    None,
                ),
            };
        let pid = receipt
            .snapshot
            .and_then(|snapshot| i32::try_from(snapshot.init_pid).ok());
        self.repo
            .update_runtime_state(
                agent.id,
                RuntimeStatePatch {
                    status,
                    desired_state: DesiredState::Running,
                    pid,
                    health_status: Some(
                        if status == AgentStatus::Running {
                            "healthy"
                        } else {
                            "unhealthy"
                        }
                        .into(),
                    ),
                    health_detail: Some(detail.clone()),
                    last_capabilities_json: capabilities,
                    startup_command_redacted: Some("docker compose start <agent-service>".into()),
                    started_at: parse_domain_ts(&agent.runtime.started_at),
                    stopped_at: None,
                },
            )
            .await?;
        Ok(RuntimeOperationResponse {
            agent_id: agent.id,
            status,
            message: detail,
        })
    }
}
