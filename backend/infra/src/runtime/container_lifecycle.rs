use super::*;
use app::container_runtime::{ContainerBinding, ContainerLaunch, PreparedContainer};
use container_control::{
    ContainerControl, ContainerLaunchFiles, ContainerObservation, ContainerReceiptState,
    ControlSource,
};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

// Base utility169 is independent of the accepted Rust/UI SDK pin.
pub(super) const UTILITY_SHA256: [&str; 3] = [
    "3893ed87d8cbfd815a60356392be523ce06ed735f66c2275fdc0f1c3970948aa",
    "49234f34db9089f74407dcf1c01ba4d920e2b72cef43df1f7392794f1314fdfc",
    "b079befbffc71053c6ae0398b1b2446c10d77c5f9accee54f2fdd642d30e4b05",
];

fn held() -> AppError {
    AppError::Unavailable(
        "Original container generation requires reconciliation; no native fallback".into(),
    )
}

async fn private_root(path: &str) -> Result<PathBuf, AppError> {
    if !cfg!(target_os = "linux") {
        return Err(held());
    }
    let root = crate::normalize_path(Path::new(path)).map_err(|_| held())?;
    if !root.is_absolute() || root != Path::new(path) {
        return Err(held());
    }
    crate::reject_symlink_components(Path::new("/"), &root)
        .await
        .map_err(|_| held())?;
    let metadata = tokio::fs::symlink_metadata(&root)
        .await
        .map_err(|_| held())?;
    if !metadata.is_dir() {
        return Err(held());
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o777 != 0o700
            || metadata.uid()
                != tokio::fs::metadata("/proc/self")
                    .await
                    .map_err(|_| held())?
                    .uid()
        {
            return Err(held());
        }
    }
    Ok(root)
}

async fn private_file(root: &Path, path: &Path, limit: u64) -> Result<Vec<u8>, AppError> {
    if !path.is_absolute() || path.parent() != Some(root) {
        return Err(held());
    }
    crate::reject_symlink_components(root, path)
        .await
        .map_err(|_| held())?;
    let before = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|_| held())?;
    if !before.is_file() || before.len() > limit {
        return Err(held());
    }
    let file = tokio::fs::File::open(path).await.map_err(|_| held())?;
    let metadata = file.metadata().await.map_err(|_| held())?;
    if !before.is_file() || !metadata.is_file() || metadata.len() > limit {
        return Err(held());
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o777 != 0o600
            || metadata.nlink() != 1
            || metadata.uid() != tokio::fs::metadata(root).await.map_err(|_| held())?.uid()
            || (before.dev(), before.ino()) != (metadata.dev(), metadata.ino())
        {
            return Err(held());
        }
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| held())?;
    if bytes.len() as u64 > limit || bytes.len() as u64 != metadata.len() {
        return Err(held());
    }
    Ok(bytes)
}

fn validate_recipe(
    agent: &Agent,
    p: &PreparedContainer,
    compose: &Value,
    token: &str,
) -> Result<(), AppError> {
    let b = &p.container;
    container_control::validate_registration(&b.registration)?;
    let policy = &b.policy;
    let service = policy["service"].as_str().ok_or_else(held)?;
    if p.agent_id != agent.id
        || p.api_port != agent.api_port
        || p.api_port.is_none_or(|v| !(1024..=65535).contains(&v))
        || serde_json::to_value(&p.paths).map_err(|_| held())?
            != serde_json::to_value(&agent.paths).map_err(|_| held())?
        || !matches!(b.registration.contract_version, 2 | 3)
        || b.registration.resource_id != agent.id
        || policy["contract_version"] != b.registration.contract_version
        || policy["resource_id"] != agent.id.to_string()
        || policy["generation"] != b.registration.generation.to_string()
        || !matches!(policy["project"].as_str(), Some("sdlc1" | "sdlc2"))
            && !policy["project"]
                .as_str()
                .is_some_and(|v| v.starts_with("sdlc-qa-"))
        || b.source_sha256 != UTILITY_SHA256.map(str::to_owned)
        || container_control::canonical_hash(policy)? != b.registration.policy_sha256
        || container_control::canonical_hash(compose)? != b.registration.compose_sha256
    {
        return Err(held());
    }
    let expected = json!([
        {"type":"bind","source":agent.paths.runtime,"destination":"/runtime","read_only":true},
        {"type":"bind","source":agent.paths.config,"destination":"/config","read_only":false},
        {"type":"bind","source":agent.paths.workspace,"destination":"/workspace","read_only":false},
        {"type":"bind","source":agent.paths.logs,"destination":"/logs","read_only":false}
    ]);
    let mounts = if let Some(mapped) = &b.mapped {
        let c = &mapped.mapping.controller;
        let local = super::container_mapping::local_policy(
            policy,
            &mapped.mapping,
            &shared::config::MappingControllerConfig {
                container_id: c.container_id.clone(),
                image_id: c.image_id.clone(),
                service: c.service.clone(),
            },
            &mapped.mapping.local_root,
        )?;
        if b.registration.mount_mapping_sha256.as_ref()
            != Some(&container_control::canonical_hash(&mapped.mapping)?)
            || b.registration.engine != mapped.mapping.engine
            || b.registration.container_id == mapped.mapping.controller.container_id
        {
            return Err(held());
        }
        local["mounts"].clone()
    } else {
        if b.registration.contract_version != 2 {
            return Err(held());
        }
        policy["mounts"].clone()
    };
    if mounts != expected
        || compose["name"] != policy["project"]
        || compose["services"]
            .as_object()
            .is_none_or(|services| services.len() != 1 || !services.contains_key(service))
    {
        return Err(held());
    }
    let spec = &compose["services"][service];
    let environment = spec["environment"].as_object().ok_or_else(held)?;
    for (key, value) in [
        ("HOME", "/config"),
        ("HERMES_HOME", "/config"),
        ("API_SERVER_ENABLED", "true"),
        ("API_SERVER_HOST", "0.0.0.0"),
        ("HERMES_SERVE_HEADLESS", "1"),
        ("API_SERVER_KEY", token),
    ] {
        // Compose escapes literal dollar signs in the protected recipe.
        if environment.get(key).and_then(Value::as_str) != Some(value.replace('$', "$$").as_str()) {
            return Err(held());
        }
    }
    if spec["working_dir"] != "/workspace"
        || spec["command"]
            != json!([
                "serve",
                "--host",
                "0.0.0.0",
                "--port",
                p.api_port.unwrap().to_string()
            ])
        || environment.get("API_SERVER_PORT") != Some(&json!(p.api_port.unwrap().to_string()))
        || environment
            .keys()
            .any(|key| key.starts_with("DOCKER_") || key == "CONTAINER_HOST")
    {
        return Err(held());
    }
    Ok(())
}

impl LocalRuntimeSupervisor {
    pub(super) async fn container_mode(&self, agent: &Agent) -> Result<bool, AppError> {
        Ok(agent.kind == AgentKind::Hermes
            && (self.config.fleet.container_control.is_some()
                || self.repo.get_container_launch(agent.id).await?.is_some()))
    }

    pub(super) async fn container_files(
        &self,
        b: &ContainerBinding,
    ) -> Result<(ContainerControl, ContainerLaunchFiles), AppError> {
        let config = self
            .config
            .fleet
            .container_control
            .as_ref()
            .ok_or_else(held)?;
        let root = private_root(&config.controller_root).await?;
        let agents =
            crate::normalize_path(Path::new(&self.config.fleet.agents_root)).map_err(|_| held())?;
        let source = crate::normalize_path(Path::new(&config.base_root)).map_err(|_| held())?;
        if root.starts_with(&agents)
            || agents.starts_with(&root)
            || source.starts_with(&agents)
            || agents.starts_with(&source)
            || source.starts_with(&root)
            || root.starts_with(&source)
            || b.context != config.context
            || b.source_sha256 != UTILITY_SHA256.map(str::to_owned)
            || [
                b.compose.as_str(),
                b.journal.as_str(),
                b.stop_journal.as_str(),
            ]
            .iter()
            .any(|v| Path::new(v).parent() != Some(root.as_path()))
            || b.compose == b.journal
            || b.compose == b.stop_journal
            || b.journal == b.stop_journal
        {
            return Err(held());
        }
        for path in [&b.compose, &b.journal, &b.stop_journal] {
            crate::reject_symlink_components(&root, Path::new(path))
                .await
                .map_err(|_| held())?;
        }
        if let Some(mapped) = &b.mapped {
            let paths = [
                &b.compose,
                &b.journal,
                &b.stop_journal,
                &mapped.mapping_file,
                &mapped.attachment_journal,
                &mapped.recovery_journal,
            ];
            for (i, path) in paths.iter().enumerate() {
                if Path::new(path).parent() != Some(root.as_path()) || paths[..i].contains(path) {
                    return Err(held());
                }
                crate::reject_symlink_components(&root, Path::new(path))
                    .await
                    .map_err(|_| held())?;
            }
            let bytes = private_file(&root, Path::new(&mapped.mapping_file), 65_536).await?;
            if serde_json::from_slice::<Value>(&bytes).map_err(|_| held())? != json!(mapped.mapping)
            {
                return Err(held());
            }
            super::container_mapping::local_policy(
                &b.policy,
                &mapped.mapping,
                config.mapping_controller.as_ref().ok_or_else(held)?,
                &self.config.fleet.agents_root,
            )?;
        }
        let control = ContainerControl::new(
            config.python.clone().into(),
            ControlSource {
                root: source,
                sha256: b.source_sha256.clone(),
            },
            config.context.clone(),
        )?;
        Ok((
            control,
            ContainerLaunchFiles {
                policy: b.policy.clone(),
                compose: b.compose.clone().into(),
                journal: b.journal.clone().into(),
                stop_journal: b.stop_journal.clone().into(),
                mapped: b.mapped.clone(),
                recovery: None,
            },
        ))
    }

    pub(super) async fn checked_prepared(
        &self,
        agent: &Agent,
        p: &PreparedContainer,
    ) -> Result<(), AppError> {
        let config = self
            .config
            .fleet
            .container_control
            .as_ref()
            .ok_or_else(held)?;
        let root = private_root(&config.controller_root).await?;
        let agents =
            crate::normalize_path(Path::new(&self.config.fleet.agents_root)).map_err(|_| held())?;
        let agent_root = crate::safe_agent_root(&agents, &agent.name).map_err(|_| held())?;
        crate::reject_symlink_components(Path::new("/"), &agents)
            .await
            .map_err(|_| held())?;
        for (area, value) in [
            ("runtime", &agent.paths.runtime),
            ("config", &agent.paths.config),
            ("workspace", &agent.paths.workspace),
            ("logs", &agent.paths.logs),
        ] {
            if Path::new(value) != agent_root.join(area) {
                return Err(held());
            }
            crate::reject_symlink_components(&agents, Path::new(value))
                .await
                .map_err(|_| held())?;
            if !tokio::fs::symlink_metadata(value)
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
        let compose: Value = serde_json::from_slice(
            &private_file(&root, Path::new(&p.container.compose), 1_048_576).await?,
        )
        .map_err(|_| held())?;
        validate_recipe(
            agent,
            p,
            &compose,
            &crate::agent_runtime_token(&self.config, agent.id)?,
        )?;
        let revision = self.repo.get_container_configuration(agent.id).await?;
        let number = revision.as_ref().map(|r| r.revision);
        let hash = revision
            .map(|r| container_control::canonical_hash(&r.snapshot))
            .transpose()?;
        if p.configuration_revision != number || p.configuration_sha256 != hash {
            return Err(held());
        }
        Ok(())
    }

    async fn owned_container(&self, agent: &Agent) -> Result<ContainerLaunch, AppError> {
        let launch = self
            .repo
            .get_container_launch(agent.id)
            .await?
            .ok_or_else(held)?;
        if launch.prepared.agent_id != agent.id {
            return Err(held());
        }
        self.checked_prepared(agent, &launch.prepared).await?;
        self.container_owner_files(&launch).await?;
        Ok(launch)
    }

    pub(super) async fn container_origin(&self, agent: &Agent) -> Result<String, AppError> {
        let launch = self.owned_container(agent).await?;
        if launch.state != "running" {
            return Err(held());
        }
        let (control, files) = self.container_owner_files(&launch).await?;
        let original = &launch.prepared.container.registration;
        let receipt = control.observe(&files, original).await?;
        if receipt.state != ContainerReceiptState::Observed
            || receipt.observation != ContainerObservation::Running
            || serde_json::to_value(&receipt.snapshot).map_err(|_| held())?
                != launch.snapshot.clone().ok_or_else(held)?
        {
            return Err(held());
        }
        let host = control.endpoint(&files, original).await?;
        let origin = format!("http://{host}:{}", agent.api_port.ok_or_else(held)?);
        if launch.origin.as_ref() != Some(&origin) {
            return Err(held());
        }
        Ok(origin)
    }

    pub(super) async fn bind_container_dispatch(
        &self,
        agent: &Agent,
        caps: &mut Value,
    ) -> Result<(), AppError> {
        if self.container_mode(agent).await? {
            self.container_origin(agent).await?;
            let launch = self.owned_container(agent).await?;
            caps["fleet_container_generation"] =
                json!(launch.prepared.container.registration.generation);
        }
        Ok(())
    }

    pub(super) async fn verify_container_intent(
        &self,
        agent: &Agent,
        intent: &app::HermesDispatchIntent,
    ) -> Result<(), AppError> {
        if self.container_mode(agent).await? {
            let origin = self.container_origin(agent).await?;
            let launch = self.owned_container(agent).await?;
            if intent.origin != origin
                || intent.capabilities["fleet_container_generation"]
                    != launch
                        .prepared
                        .container
                        .registration
                        .generation
                        .to_string()
            {
                return Err(held());
            }
        } else if intent
            .capabilities
            .get("fleet_container_generation")
            .is_some()
        {
            return Err(held());
        }
        Ok(())
    }

    pub(super) async fn container_run_origin(
        &self,
        agent: &Agent,
        run: &SessionAgentRun,
    ) -> Result<String, AppError> {
        let intent = self
            .repo
            .get_hermes_run_intent(run.id)
            .await?
            .ok_or_else(held)?;
        if intent.run.agent_id != agent.id
            || intent.run.id != run.id
            || intent.run.runtime_run_id != run.runtime_run_id
            || intent.state != "accepted"
        {
            return Err(held());
        }
        self.verify_container_intent(agent, &intent).await?;
        hermes_wire::verify_intent(
            &intent,
            &intent.origin,
            &crate::agent_runtime_token(&self.config, agent.id)?,
        )?;
        Ok(intent.origin)
    }

    pub(super) async fn start_container(
        &self,
        agent: &Agent,
    ) -> Result<RuntimeOperationResponse, AppError> {
        if let Some(existing) = self.repo.get_container_launch(agent.id).await? {
            if existing.state != "exited" {
                return self.health_container(agent).await;
            }
        }
        if self.children.lock().await.contains_key(&agent.id)
            || agent.runtime.pid.is_some()
            || !matches!(
                agent.status,
                AgentStatus::Ready | AgentStatus::Stopped | AgentStatus::Failed
            )
        {
            return Err(held());
        }
        let config = self
            .config
            .fleet
            .container_control
            .as_ref()
            .ok_or_else(held)?;
        let root = private_root(&config.controller_root).await?;
        let p: PreparedContainer = serde_json::from_slice(
            &private_file(
                &root,
                &root.join(format!("{}.container-prepared.json", agent.id)),
                65_536,
            )
            .await?,
        )
        .map_err(|_| held())?;
        self.checked_prepared(agent, &p).await?;
        let (control, files) = self.container_files(&p.container).await?;
        let observed = control.observe(&files, &p.container.registration).await?;
        if observed.state != ContainerReceiptState::Registered
            || observed.observation != ContainerObservation::NeverStarted
        {
            return Err(held());
        }
        let launch = ContainerLaunch {
            prepared: p,
            controller_id: self.controller_id,
            state: "claimed".into(),
            snapshot: None,
            origin: None,
            stop_id: Uuid::new_v4(),
        };
        self.repo.claim_container_launch(&launch).await?;
        if files.mapped.is_some() {
            control
                .attach(&files, &launch.prepared.container.registration)
                .await?;
        }
        self.container_state(
            agent,
            AgentStatus::Starting,
            DesiredState::Running,
            None,
            "Original container start committed",
            None,
        )
        .await?;
        let receipt = control
            .start(&files, &launch.prepared.container.registration)
            .await?;
        if receipt.state != ContainerReceiptState::Observed
            || receipt.observation != ContainerObservation::Running
        {
            return Err(held());
        }
        let snapshot = serde_json::to_value(receipt.snapshot.as_ref().ok_or_else(held)?)
            .map_err(|_| held())?;
        let host = control
            .endpoint(&files, &launch.prepared.container.registration)
            .await?;
        let origin = format!("http://{host}:{}", agent.api_port.ok_or_else(held)?);
        self.repo
            .advance_container_launch(
                &launch,
                "running",
                Some(snapshot.clone()),
                Some(origin.clone()),
            )
            .await?;
        let capabilities =
            match tokio::time::timeout(HERMES_READY_TIMEOUT, self.wait_for_hermes_ready(agent))
                .await
            {
                Ok(Ok(value)) => value,
                _ => {
                    self.stop_container(agent).await?;
                    return Err(AppError::Unavailable(
                        "Container readiness failed after verified stop".into(),
                    ));
                }
            };
        self.container_origin(agent).await?;
        self.container_state(
            agent,
            AgentStatus::Running,
            DesiredState::Running,
            receipt
                .snapshot
                .and_then(|v| i32::try_from(v.init_pid).ok()),
            "Original Docker namespace and Hermes API are healthy; admission remains blocked",
            Some(capabilities),
        )
        .await
    }

    pub(super) async fn stop_container(
        &self,
        agent: &Agent,
    ) -> Result<RuntimeOperationResponse, AppError> {
        let mut launch = self.owned_container(agent).await?;
        if launch.state == "exited" {
            return self
                .container_state(
                    agent,
                    AgentStatus::Stopped,
                    DesiredState::Stopped,
                    None,
                    "Original namespace exited",
                    None,
                )
                .await;
        }
        if launch.state == "running" {
            self.advance_container(&launch, "stopping").await?;
            launch.state = "stopping".into();
        }
        if launch.state != "stopping" {
            return Err(held());
        }
        let (control, files) = self.container_owner_files(&launch).await?;
        let original = &launch.prepared.container.registration;
        let observed = control.observe(&files, original).await?;
        if serde_json::to_value(&observed.snapshot).map_err(|_| held())?
            != launch.snapshot.clone().ok_or_else(held)?
        {
            return Err(held());
        }
        control.stop(&files, original, launch.stop_id).await?;
        self.advance_container(&launch, "exited").await?;
        self.container_state(
            agent,
            AgentStatus::Stopped,
            DesiredState::Stopped,
            None,
            "Original namespace exit verified",
            None,
        )
        .await
    }

    pub(super) async fn health_container(
        &self,
        agent: &Agent,
    ) -> Result<RuntimeOperationResponse, AppError> {
        let launch = self.owned_container(agent).await?;
        if launch.state != "running" {
            return Err(held());
        }
        let (control, files) = self.container_owner_files(&launch).await?;
        let receipt = control
            .observe(&files, &launch.prepared.container.registration)
            .await?;
        if receipt.state != ContainerReceiptState::Observed
            || serde_json::to_value(&receipt.snapshot).map_err(|_| held())?
                != launch.snapshot.clone().ok_or_else(held)?
        {
            return Err(held());
        }
        if receipt.observation == ContainerObservation::NamespaceExited {
            self.advance_container(&launch, "exited").await?;
            return self
                .container_state(
                    agent,
                    AgentStatus::Stopped,
                    DesiredState::Stopped,
                    None,
                    "Original namespace exited; run capacity remains held",
                    None,
                )
                .await;
        }
        self.container_origin(agent).await?;
        match self.probe_hermes(agent).await {
            Ok(caps) => {
                self.container_origin(agent).await?;
                let pid = launch
                    .snapshot
                    .as_ref()
                    .and_then(|v| v["init_pid"].as_i64())
                    .and_then(|v| i32::try_from(v).ok());
                self.container_state(
                    agent,
                    AgentStatus::Running,
                    DesiredState::Running,
                    pid,
                    "Original namespace and API healthy; admission remains blocked",
                    Some(caps),
                )
                .await
            }
            Err(_) => {
                self.container_state(
                    agent,
                    AgentStatus::Degraded,
                    agent.runtime.desired_state,
                    agent.runtime.pid,
                    "Original namespace API is unhealthy",
                    None,
                )
                .await
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn container_state(
        &self,
        agent: &Agent,
        status: AgentStatus,
        desired: DesiredState,
        pid: Option<i32>,
        detail: &str,
        caps: Option<Value>,
    ) -> Result<RuntimeOperationResponse, AppError> {
        let updated = self
            .repo
            .update_runtime_state(
                agent.id,
                RuntimeStatePatch {
                    status,
                    desired_state: desired,
                    pid,
                    health_status: Some(status.as_str().into()),
                    health_detail: Some(detail.into()),
                    last_capabilities_json: caps,
                    startup_command_redacted: Some(
                        "docker compose start <original-agent-service>".into(),
                    ),
                    started_at: if status == AgentStatus::Running {
                        Some(shared::now())
                    } else {
                        parse_domain_ts(&agent.runtime.started_at)
                    },
                    stopped_at: if status == AgentStatus::Stopped {
                        Some(shared::now())
                    } else {
                        None
                    },
                },
            )
            .await?;
        Ok(RuntimeOperationResponse {
            agent_id: agent.id,
            status: updated.status,
            message: detail.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app::container_runtime::{ContainerEngineIdentity, ContainerRegistration};

    fn fixture() -> (Agent, PreparedContainer, Value) {
        let a = super::super::tests::agent(Uuid::new_v4(), AgentProductRole::Executor);
        let generation = Uuid::new_v4();
        let policy = json!({"contract_version":2,"project":"sdlc1","service":"agent1-runtime","resource_id":a.id,"generation":generation,
            "mounts":[{"type":"bind","source":a.paths.runtime,"destination":"/runtime","read_only":true},
            {"type":"bind","source":a.paths.config,"destination":"/config","read_only":false},
            {"type":"bind","source":a.paths.workspace,"destination":"/workspace","read_only":false},
            {"type":"bind","source":a.paths.logs,"destination":"/logs","read_only":false}]});
        let compose = json!({"name":"sdlc1","services":{"agent1-runtime":{"working_dir":"/workspace","command":["serve","--host","0.0.0.0","--port","29001"],
            "environment":{"HOME":"/config","HERMES_HOME":"/config","API_SERVER_ENABLED":"true","API_SERVER_HOST":"0.0.0.0","HERMES_SERVE_HEADLESS":"1","API_SERVER_KEY":"fixture","API_SERVER_PORT":"29001"}}}});
        let p = PreparedContainer {
            agent_id: a.id,
            paths: a.paths.clone(),
            api_port: a.api_port,
            configuration_revision: None,
            configuration_sha256: None,
            container: ContainerBinding {
                registration: ContainerRegistration {
                    contract_version: 2,
                    operation_id: Uuid::new_v4(),
                    container_id: "a".repeat(64),
                    resource_id: a.id,
                    generation,
                    engine: ContainerEngineIdentity {
                        id: "engine".into(),
                        kernel_version: "kernel".into(),
                        server_version: "29".into(),
                    },
                    policy_sha256: container_control::canonical_hash(&policy).unwrap(),
                    inventory_sha256: "c".repeat(64),
                    running_inventory_sha256: "d".repeat(64),
                    compose_sha256: container_control::canonical_hash(&compose).unwrap(),
                    network_sha256: Some("f".repeat(64)),
                    mount_mapping_sha256: None,
                },
                policy,
                compose: "/private/compose.json".into(),
                journal: "/private/start.sqlite".into(),
                stop_journal: "/private/stop.sqlite".into(),
                source_sha256: UTILITY_SHA256.map(str::to_owned),
                context: "protected".into(),
                mapped: None,
            },
        };
        (a, p, compose)
    }

    #[test]
    fn original_recipe_requires_isolated_home_workdir_and_original_token() {
        let (a, p, c) = fixture();
        validate_recipe(&a, &p, &c, "fixture").unwrap();
        assert!(validate_recipe(&a, &p, &c, "rotated").is_err());
        for (key, value) in [
            ("HOME", "/shared"),
            ("HERMES_HOME", "/workspace"),
            ("DOCKER_HOST", "tcp://foreign:2375"),
        ] {
            let mut c = c.clone();
            c["services"]["agent1-runtime"]["environment"][key] = json!(value);
            let mut p = p.clone();
            p.container.registration.compose_sha256 =
                container_control::canonical_hash(&c).unwrap();
            assert!(validate_recipe(&a, &p, &c, "fixture").is_err());
        }
    }

    #[test]
    fn foreign_agent_generation_mount_and_sources_cannot_be_adopted() {
        let (a, p, c) = fixture();
        let mut wrong = p.clone();
        wrong.agent_id = Uuid::new_v4();
        assert!(validate_recipe(&a, &wrong, &c, "fixture").is_err());
        let mut wrong = p.clone();
        wrong.container.registration.generation = Uuid::new_v4();
        assert!(validate_recipe(&a, &wrong, &c, "fixture").is_err());
        let mut wrong = p.clone();
        wrong.container.policy["mounts"][1]["source"] = json!("/foreign/config");
        wrong.container.registration.policy_sha256 =
            container_control::canonical_hash(&wrong.container.policy).unwrap();
        assert!(validate_recipe(&a, &wrong, &c, "fixture").is_err());
        let mut wrong = p;
        wrong.container.source_sha256[0] = "0".repeat(64);
        assert!(validate_recipe(&a, &wrong, &c, "fixture").is_err());
    }

    #[test]
    fn shared_infrastructure_and_build_projects_are_not_agent_runtimes() {
        let (a, p, c) = fixture();
        for name in [
            "sdlc-common",
            "sdlc-demo",
            "sdlc-build-runtime-123456789abc",
            "",
        ] {
            let mut p = p.clone();
            p.container.policy["project"] = json!(name);
            p.container.registration.policy_sha256 =
                container_control::canonical_hash(&p.container.policy).unwrap();
            let mut c = c.clone();
            c["name"] = json!(name);
            p.container.registration.compose_sha256 =
                container_control::canonical_hash(&c).unwrap();
            assert!(validate_recipe(&a, &p, &c, "fixture").is_err());
        }
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn private_storage_rejects_links_shared_modes_and_oversized_documents() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("fleet-container-private-{}", Uuid::new_v4()));
        tokio::fs::create_dir(&root).await.unwrap();
        tokio::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
            .await
            .unwrap();
        let path = root.join("prepared.json");
        tokio::fs::write(&path, b"{}").await.unwrap();
        tokio::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .await
            .unwrap();
        assert_eq!(private_file(&root, &path, 16).await.unwrap(), b"{}");
        assert!(private_file(&root, &path, 1).await.is_err());
        let link = root.join("link.json");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(private_file(&root, &link, 16).await.is_err());
        tokio::fs::remove_file(&link).await.unwrap();
        tokio::fs::hard_link(&path, &link).await.unwrap();
        assert!(private_file(&root, &path, 16).await.is_err());
        tokio::fs::remove_file(&link).await.unwrap();
        tokio::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .await
            .unwrap();
        assert!(private_file(&root, &path, 16).await.is_err());
        tokio::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755))
            .await
            .unwrap();
        assert!(private_root(root.to_str().unwrap()).await.is_err());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
