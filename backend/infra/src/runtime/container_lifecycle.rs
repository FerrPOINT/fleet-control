use super::*;
use app::runtime_launch::{RuntimeContainerBinding, RuntimeLaunchBinding, RuntimeLaunchRecord};
use container_control::{
    ContainerControl, ContainerLaunchFiles, ContainerObservation, ContainerReceipt,
    ContainerReceiptState, ControlSource,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bridge_controller: Option<shared::config::BridgeControllerConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mount_mapping: Option<container_control::ContainerMountMapping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mapping_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    environment_snapshot: Option<container_environment::ContainerEnvironmentSnapshot>,
}

fn apply_mapping(
    intent: &mut ContainerCreationIntent,
    root: &Path,
    mapping: container_control::ContainerMountMapping,
) -> Result<(), AppError> {
    intent.policy = container_control::mapped_policy(&intent.policy, &mapping)?;
    intent.mapping_file = Some(
        root.join(format!(
            "{}.{}.mapping.json",
            intent.agent_id, intent.generation
        ))
        .to_string_lossy()
        .into_owned(),
    );
    intent.mount_mapping = Some(mapping);
    Ok(())
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
    async fn claim_container_intent(
        &self,
        intent: &ContainerCreationIntent,
        ordinal: i64,
        configuration: &app::runtime_launch::RuntimeConfigurationClaim,
    ) -> Result<(), AppError> {
        if serde_json::to_vec(intent).map_err(|_| held())?.len() > 64 * 1024 {
            return Err(held());
        }
        self.repo
            .claim_container_preparation(
                &app::runtime_launch::RuntimeContainerPreparation {
                    agent_id: intent.agent_id,
                    ordinal,
                    controller_id: self.controller_id,
                    generation: intent.generation,
                    operation_id: intent.operation_id,
                    intent_sha256: crate::runtime_launches::snapshot_hash(
                        &serde_json::to_value(intent).map_err(AppError::internal)?,
                    )?,
                },
                configuration,
            )
            .await
    }

    fn container_intent(
        &self,
        agent: &Agent,
        generation: Uuid,
        operation_id: Uuid,
        revision: Option<i64>,
        revision_hash: Option<String>,
    ) -> Result<ContainerCreationIntent, AppError> {
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
        if generation.is_nil() || operation_id.is_nil() {
            return Err(held());
        }
        let service = format!("{}-runtime-{}", agent.name, generation.simple());
        let port = agent
            .api_port
            .filter(|value| (1024..=65535).contains(value))
            .ok_or_else(held)?;
        Ok(ContainerCreationIntent {
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
            bridge_controller: config.bridge_controller.clone(),
            mount_mapping: None,
            mapping_file: None,
            environment_snapshot: None,
        })
    }

    async fn capture_container_environment(
        &self,
        intent: &mut ContainerCreationIntent,
    ) -> Result<(), AppError> {
        let expected = if let Some(revision) = intent.configuration_revision {
            let revision = self
                .repo
                .get_config_revision(intent.agent_id, revision)
                .await?;
            let hash = crate::runtime_launches::snapshot_hash(
                &serde_json::to_value(&revision.snapshot).map_err(|_| held())?,
            )?;
            if intent.configuration_sha256.as_ref() != Some(&hash) {
                return Err(held());
            }
            let agent = self.repo.get_agent(intent.agent_id).await?;
            let files = crate::configuration_files(&agent, &self.config, &revision).await?;
            Some(
                files
                    .into_iter()
                    .find(|(path, _)| path.file_name().is_some_and(|name| name == ".env"))
                    .ok_or_else(held)?
                    .1,
            )
        } else {
            None
        };
        intent.environment_snapshot = Some(
            container_environment::capture(
                Path::new(&self.config.fleet.agents_root),
                Path::new(&intent.paths.config),
                expected.as_deref(),
            )
            .await?,
        );
        Ok(())
    }

    async fn create_container(
        &self,
        agent: &Agent,
        root: &Path,
        prepared_path: &Path,
        intent_path: &Path,
        ordinal: i64,
        configuration: &app::runtime_launch::RuntimeConfigurationClaim,
    ) -> Result<(), AppError> {
        let config = self
            .config
            .fleet
            .container_control
            .as_ref()
            .ok_or_else(held)?;
        let agents = crate::normalize_path(Path::new(&self.config.fleet.agents_root))?;
        let base = crate::normalize_path(Path::new(&config.base_root))?;
        if root.starts_with(&agents)
            || agents.starts_with(root)
            || base.starts_with(&agents)
            || agents.starts_with(&base)
        {
            return Err(held());
        }
        let previous: Option<ContainerCreationIntent> = read_private_document(root, intent_path)
            .await?
            .map(|bytes| serde_json::from_slice(&bytes).map_err(|_| held()))
            .transpose()?;
        let generation = previous
            .as_ref()
            .map_or_else(Uuid::new_v4, |value| value.generation);
        let operation_id = previous
            .as_ref()
            .map_or_else(Uuid::new_v4, |value| value.operation_id);
        let mut intent = self.container_intent(
            agent,
            generation,
            operation_id,
            configuration.revision,
            configuration.sha256.clone(),
        )?;
        // Legacy intents retain their original hash; never backfill rotated input.
        if previous
            .as_ref()
            .is_none_or(|value| value.environment_snapshot.is_some())
        {
            self.capture_container_environment(&mut intent).await?;
        }
        let name = format!("{}.{}", agent.id, generation);
        let control = ContainerControl::new(
            config.python.clone().into(),
            ControlSource {
                root: config.base_root.clone().into(),
                sha256: config.source_sha256.clone(),
            },
            config.context.clone(),
        )?;
        if let Some(controller) = &config.bridge_controller {
            let local_files = ContainerLaunchFiles {
                policy: intent.policy.clone(),
                compose: root.join(format!("{name}.compose.json")),
                journal: root.join(format!("{name}.launch.sqlite")),
                stop_journal: root.join(format!("{name}.stop.sqlite")),
                mount_mapping: None,
                mapping_file: None,
            };
            let fresh = control
                .resolve_mounts(&local_files, controller, agents.to_str().ok_or_else(held)?)
                .await?;
            if previous
                .as_ref()
                .is_some_and(|previous| previous.mount_mapping.as_ref() != Some(&fresh))
            {
                return Err(held());
            }
            apply_mapping(&mut intent, root, fresh)?;
        }
        if let Some(previous) = previous.as_ref() {
            if serde_json::to_value(previous).map_err(|_| held())?
                != serde_json::to_value(&intent).map_err(|_| held())?
            {
                return Err(held());
            }
        }
        // Commit before both private-file creation and Docker create. Missing files
        // must not grant a new generation after an unknown physical effect.
        self.claim_container_intent(&intent, ordinal, configuration)
            .await?;
        if previous.is_none() {
            private_document(root, intent_path, &intent).await?;
        }
        let files = ContainerLaunchFiles {
            policy: intent.policy,
            compose: root.join(format!("{name}.compose.json")),
            journal: root.join(format!("{name}.launch.sqlite")),
            stop_journal: root.join(format!("{name}.stop.sqlite")),
            mount_mapping: intent.mount_mapping.clone(),
            mapping_file: intent.mapping_file.as_ref().map(PathBuf::from),
        };
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
            configuration_revision: configuration.revision,
            configuration_sha256: intent.configuration_sha256,
            container: RuntimeContainerBinding {
                registration: prepared.registration,
                policy: prepared.policy,
                compose: files.compose.to_string_lossy().into_owned(),
                journal: files.journal.to_string_lossy().into_owned(),
                stop_journal: files.stop_journal.to_string_lossy().into_owned(),
                source_sha256: config.source_sha256.clone(),
                context: config.context.clone(),
                mount_mapping: intent.mount_mapping,
                mapping_file: intent.mapping_file,
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
        if let Some(mapping) = &binding.mount_mapping {
            let controller = self
                .config
                .fleet
                .container_control
                .as_ref()
                .and_then(|config| config.bridge_controller.as_ref())
                .ok_or_else(held)?;
            let local = container_control::local_mapping_policy(&binding.policy, mapping)?;
            container_control::validate_mount_mapping(
                mapping,
                &local,
                controller,
                agents.to_str().ok_or_else(held)?,
            )?;
        }
        for value in [&binding.compose, &binding.journal, &binding.stop_journal]
            .into_iter()
            .chain(binding.mapping_file.as_ref())
        {
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
            mount_mapping: binding.mount_mapping.clone(),
            mapping_file: binding.mapping_file.as_ref().map(PathBuf::from),
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
        self.attach_container_controller(binding).await?;
        let host = self
            .container_control(binding)?
            .endpoint(&self.container_files(binding).await?, &binding.registration)
            .await?;
        let port = agent
            .api_port
            .filter(|port| (1..=65535).contains(port))
            .ok_or_else(held)?;
        let origin = format!("http://{host}:{port}");
        self.repo
            .record_container_endpoint(&launch.binding, launch.pid.ok_or_else(held)?, &origin)
            .await?;
        Ok(origin)
    }

    async fn validate_container_agent_paths(&self, agent: &Agent) -> Result<PathBuf, AppError> {
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
        Ok(agents_root)
    }

    pub(super) async fn verify_container_configuration_transition(
        &self,
        agent: &Agent,
    ) -> Result<(), AppError> {
        let control = self
            .config
            .fleet
            .container_control
            .as_ref()
            .ok_or_else(held)?;
        if control.provisioning.is_none() || self.children.lock().await.contains_key(&agent.id) {
            return Err(held());
        }
        self.validate_container_agent_paths(agent).await?;
        let running = self.repo.get_open_runtime_launch(agent.id).await?.is_some();
        if running {
            // Only the original observed namespace can be stopped before replacing files.
            self.container_generation(agent.id).await?;
        } else if unconfirmed_runtime(agent) {
            return Err(held());
        }
        if self
            .repo
            .has_pending_container_preparation(agent.id)
            .await?
        {
            return Err(held());
        }
        if running {
            return Ok(());
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
        for extension in ["container-creation.json", "container-prepared.json"] {
            if read_private_document(&root, &root.join(format!("{suffix}.{extension}")))
                .await?
                .is_some()
            {
                return Err(held());
            }
        }
        Ok(())
    }

    pub(super) async fn verify_container_configuration_quiescent(
        &self,
        agent: &Agent,
    ) -> Result<(), AppError> {
        let fresh = self.repo.get_agent(agent.id).await?;
        if self.repo.get_open_runtime_launch(agent.id).await?.is_some()
            || unconfirmed_runtime(&fresh)
        {
            return Err(held());
        }
        self.verify_container_configuration_transition(&fresh).await
    }

    pub(super) async fn prepared_container(
        &self,
        agent: &Agent,
        phase: LaunchPhase,
    ) -> Result<RuntimeLaunchBinding, AppError> {
        let agents_root = self.validate_container_agent_paths(agent).await?;
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
        let configuration = app::runtime_launch::RuntimeConfigurationClaim {
            phase: phase.into(),
            revision: revision_number,
            sha256: revision_hash.clone(),
        };
        if read_private_document(&root, &path).await?.is_none() {
            self.create_container(agent, &root, &path, &intent_path, ordinal, &configuration)
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
        let automatic = self
            .config
            .fleet
            .container_control
            .as_ref()
            .ok_or_else(held)?
            .provisioning
            .is_some();
        let intent_bytes = read_private_document(&root, &intent_path).await?;
        if automatic || intent_bytes.is_some() {
            // A prepared receipt is not permission to start an obsolete recipe or token.
            let intent: ContainerCreationIntent =
                serde_json::from_slice(&intent_bytes.ok_or_else(held)?).map_err(|_| held())?;
            let mut expected = self.container_intent(
                agent,
                intent.generation,
                intent.operation_id,
                revision_number,
                revision_hash.clone(),
            )?;
            if intent.environment_snapshot.is_some() {
                self.capture_container_environment(&mut expected).await?;
            }
            if let Some(mapping) = intent.mount_mapping.clone() {
                let controller = expected.bridge_controller.as_ref().ok_or_else(held)?;
                container_control::validate_mount_mapping(
                    &mapping,
                    &expected.policy,
                    controller,
                    agents_root.to_str().ok_or_else(held)?,
                )?;
                apply_mapping(&mut expected, &root, mapping)?;
            } else if expected.bridge_controller.is_some() {
                return Err(held());
            }
            if serde_json::to_value(&intent).map_err(|_| held())?
                != serde_json::to_value(expected).map_err(|_| held())?
            {
                return Err(held());
            }
            container_control::validate_preparation(
                &container_control::ContainerPreparation {
                    state: "prepared".into(),
                    policy: prepared.container.policy.clone(),
                    registration: prepared.container.registration.clone(),
                },
                &intent.policy,
                intent.operation_id,
                intent.mount_mapping.as_ref(),
            )?;
            if prepared.container.mount_mapping != intent.mount_mapping
                || prepared.container.mapping_file != intent.mapping_file
            {
                return Err(held());
            }
            self.claim_container_intent(&intent, ordinal, &configuration)
                .await?;
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
            if existing.binding.controller_id != self.controller_id {
                return Err(held());
            }
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
        self.attach_container_controller(container).await?;
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

    async fn attach_container_controller(
        &self,
        binding: &RuntimeContainerBinding,
    ) -> Result<(), AppError> {
        let config = self
            .config
            .fleet
            .container_control
            .as_ref()
            .ok_or_else(held)?;
        let Some(controller) = config.bridge_controller.as_ref() else {
            return Ok(());
        };
        let files = self.container_files(binding).await?;
        let journal = files.journal.parent().ok_or_else(held)?.join(format!(
            "{}.{}.attachment.sqlite",
            binding.registration.resource_id, binding.registration.generation
        ));
        self.container_control(binding)?
            .attach_controller(&files, &binding.registration, controller, &journal)
            .await
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
        let persisted = self
            .repo
            .get_open_runtime_launch(agent.id)
            .await?
            .ok_or_else(held)?;
        if persisted.binding.controller_id != self.controller_id {
            let binding = persisted.binding.container.as_ref().ok_or_else(held)?;
            if persisted.state != "gateway_started" || persisted.pid.is_none() {
                return Err(held());
            }
            let proof = self
                .container_control(binding)?
                .observe_controller_restart(
                    &self.container_files(binding).await?,
                    &binding.registration,
                )
                .await?;
            let fresh = self
                .repo
                .get_open_runtime_launch(agent.id)
                .await?
                .ok_or_else(held)?;
            if fresh.state != persisted.state
                || fresh.pid != persisted.pid
                || serde_json::to_value(&fresh.binding).map_err(AppError::internal)?
                    != serde_json::to_value(&persisted.binding).map_err(AppError::internal)?
                || proof
                    .receipt
                    .snapshot
                    .as_ref()
                    .and_then(|snapshot| i32::try_from(snapshot.init_pid).ok())
                    != persisted.pid
            {
                return Err(held());
            }
            // Observation cannot grant new custody, mutate the old ACK or release capacity.
            return Ok(RuntimeOperationResponse {
                agent_id: agent.id,
                status: AgentStatus::Degraded,
                message: "Controller restart observed for original agent namespace; ownership transfer remains required".into(),
            });
        }
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
