use app::{
    FleetRepository, RuntimeApprovalCreate, RuntimeSessionSnapshot, RuntimeStatePatch,
    RuntimeSupervisor,
};
use async_trait::async_trait;
use domain::{
    Agent, AgentKind, AgentProductRole, AgentSession, AgentStatus, DeploymentJob,
    DeploymentJobKind, DeploymentJobState, DesiredState, MessageAuthorType, MessageDeliveryState,
    MessageKind, ResolveRuntimeApprovalRequest, RuntimeOperationResponse,
    RuntimeRunControlResponse, SessionAgentRun, SessionMessage, SessionRunRole, SessionRunState,
    SteerSessionRunRequest,
};
use futures_util::StreamExt;
use serde::Serialize;
use serde_json::{Value, json};
use shared::{AppConfig, AppError, FleetEvent};
use std::{collections::HashMap, process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::{Mutex, broadcast},
    time::sleep,
};
use uuid::Uuid;
mod acceptance_readback;
mod approval_snapshot;
mod container_activation;
pub(crate) mod container_control;
mod container_lifecycle;
mod container_mapping;
mod container_preparation;
#[cfg(test)]
mod container_preparation_tests;
mod container_recovery;
mod container_replacement;
mod container_workers;
mod hermes_wire;
mod native_context;
mod pm_continuation;
mod pm_dispatch;
mod pm_events;
mod pm_readback;
mod pm_tools;
pub(crate) mod recovery_wire;
mod run_control;
mod sse_wire;
mod targeted_approval;

const HERMES_READY_TIMEOUT: Duration = Duration::from_secs(60);
const HERMES_READY_POLL: Duration = Duration::from_millis(500);
const RECONCILE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct LocalRuntimeSupervisor {
    config: Arc<AppConfig>,
    repo: Arc<dyn FleetRepository>,
    children: Arc<Mutex<HashMap<Uuid, Child>>>,
    controller_id: Uuid,
    container_operations: Arc<container_workers::ContainerOperations>,
    client: reqwest::Client,
    events: broadcast::Sender<FleetEvent>,
    alerts: Arc<app::RepositoryAlertService>,
}

#[derive(Debug, Serialize)]
struct HermesSteerRequest {
    input: String,
}

impl LocalRuntimeSupervisor {
    pub fn new(
        config: Arc<AppConfig>,
        repo: Arc<dyn FleetRepository>,
        events: broadcast::Sender<FleetEvent>,
    ) -> Self {
        let supervisor = Self {
            config,
            repo: repo.clone(),
            children: Arc::new(Mutex::new(HashMap::new())),
            controller_id: Uuid::new_v4(),
            container_operations: Arc::new(container_workers::ContainerOperations::default()),
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .no_proxy()
                .connect_timeout(Duration::from_secs(5))
                .build()
                .expect("runtime HTTP client configuration"),
            events,
            alerts: Arc::new(app::RepositoryAlertService {
                repository: repo.clone(),
            }),
        };
        supervisor.spawn_reconciler();
        supervisor.spawn_container_recovery();
        supervisor.spawn_message_dispatcher();
        supervisor.spawn_acceptance_readback();
        supervisor.spawn_config_activator();
        supervisor
    }

    fn spawn_message_dispatcher(&self) {
        let supervisor = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                loop {
                    match supervisor.repo.claim_message_dispatch().await {
                        Ok(Some(message)) => {
                            let result = async {
                                let session =
                                    supervisor.repo.get_session(message.session_id).await?;
                                let agent =
                                    supervisor.repo.get_agent(session.primary_agent_id).await?;
                                supervisor.send_message(&agent, &session, &message).await
                            }
                            .await;
                            let error = match result {
                                Ok(response) if response.status != AgentStatus::Failed => None,
                                Ok(response) => Some(response.message),
                                Err(error) => Some(crate::redact_text(&error.to_string())),
                            };
                            // Unknown acceptance never causes automatic redispatch.
                            let _ = supervisor
                                .repo
                                .finish_message_dispatch(message.id, error.is_some(), error)
                                .await;
                        }
                        Ok(None) => sleep(Duration::from_millis(250)).await,
                        Err(error) => {
                            tracing::warn!("message outbox failed: {error}");
                            sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
            });
        }
    }

    fn spawn_config_activator(&self) {
        let supervisor = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let mut tasks = container_workers::AgentTasks::default();
                let mut claimed = Vec::<domain::AgentConfigRevision>::new();
                let mut after_agent = None;
                loop {
                    tasks.reap();
                    // A freshly claimed revision must not be dropped while its predecessor finishes.
                    for revision in std::mem::take(&mut claimed) {
                        if tasks.is_running(revision.agent_id) {
                            claimed.push(revision);
                        } else {
                            let supervisor = supervisor.clone();
                            tasks.spawn(revision.agent_id, async move {
                                supervisor.reconcile_container_revision(&revision).await;
                            });
                        }
                    }
                    // Discovery is read-only: interrupted claims are not new custody/permits.
                    if let Ok(pending) = supervisor
                        .repo
                        .pending_container_activations(after_agent)
                        .await
                    {
                        after_agent = pending.last().map(|revision| revision.agent_id);
                        for revision in pending {
                            let supervisor = supervisor.clone();
                            tasks.spawn(revision.agent_id, async move {
                                supervisor.reconcile_container_revision(&revision).await;
                            });
                        }
                    }
                    match supervisor.repo.claim_config_activation().await {
                        Ok(Some(revision)) => {
                            let container = match supervisor.repo.get_agent(revision.agent_id).await
                            {
                                Ok(agent) => {
                                    supervisor.container_mode(&agent).await.unwrap_or(true)
                                }
                                Err(_) => true,
                            };
                            if container {
                                claimed.push(revision);
                                continue;
                            }
                            // Owner readback happens before any runtime/file mutation. Its
                            // failure must not be confused with an unverified rollback.
                            let preflight = async {
                                let agent = supervisor.repo.get_agent(revision.agent_id).await?;
                                supervisor
                                    .verify_config_activation_binding(&agent, &revision)
                                    .await
                            }
                            .await;
                            let (result, reconciled) = match preflight {
                                Err(error) => (Err(error), true),
                                Ok(()) => {
                                    let result = supervisor.apply_config_revision(&revision).await;
                                    let reconciled =
                                        !matches!(&result, Err(AppError::Unavailable(_)));
                                    (result, reconciled)
                                }
                            };
                            let error = result
                                .err()
                                .map(|error| crate::redact_text(&error.to_string()));
                            if let Err(error) = supervisor
                                .repo
                                .finish_config_activation(
                                    revision.agent_id,
                                    revision.revision,
                                    error,
                                    reconciled,
                                )
                                .await
                            {
                                tracing::error!(
                                    "configuration activation requires reconciliation: {error}"
                                );
                            }
                        }
                        Ok(None) => sleep(Duration::from_secs(1)).await,
                        Err(error) => {
                            tracing::warn!("configuration activation queue failed: {error}");
                            sleep(Duration::from_secs(2)).await;
                        }
                    }
                }
            });
        }
    }

    async fn verify_config_activation_binding(
        &self,
        agent: &Agent,
        revision: &domain::AgentConfigRevision,
    ) -> Result<(), AppError> {
        app::sdlc_workflow::verify_revision_binding(
            &self.config.sdlc.workflow_binding,
            agent,
            revision,
        )
        .await?;
        if revision
            .snapshot
            .config
            .config_json
            .get("fleet_sdlc_package")
            .is_some()
        {
            self.repo
                .verify_base_package_revision(
                    agent.id,
                    revision.revision,
                    &self.config.fleet.base_package_checkout,
                )
                .await?;
        }
        Ok(())
    }

    async fn apply_config_revision(
        &self,
        revision: &domain::AgentConfigRevision,
    ) -> Result<(), AppError> {
        let agent = self.repo.get_agent(revision.agent_id).await?;
        if self.container_mode(&agent).await? {
            return Err(AppError::Unavailable("Container configuration replacement requires an original new preparation; activation remains drained".into()));
        }
        if agent.kind != AgentKind::Hermes {
            return Err(AppError::validation(
                "Java Agent config activation is not implemented",
            ));
        }
        let running = agent.status == AgentStatus::Running;
        if !matches!(
            agent.status,
            AgentStatus::Running | AgentStatus::Ready | AgentStatus::Stopped
        ) {
            return Err(AppError::conflict(
                "runtime must be running, ready or stopped before configuration activation",
            ));
        }
        if running && !self.children.lock().await.contains_key(&agent.id) {
            return Err(AppError::conflict(
                "untracked runtime must be reconciled before configuration activation",
            ));
        }
        let files = crate::configuration_files(&agent, &self.config, revision).await?;
        let mut backups = Vec::new();
        for (path, _) in &files {
            let old = match tokio::fs::read(path).await {
                Ok(content) => Some(content),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(AppError::internal(error)),
            };
            backups.push((path.clone(), old));
        }
        if running {
            self.stop(&agent).await?;
        }
        let applied = async {
            for (path, body) in &files {
                if body.is_empty() && path.file_name().is_some_and(|name| name == "SKILL.md") {
                    if tokio::fs::try_exists(path)
                        .await
                        .map_err(AppError::internal)?
                    {
                        tokio::fs::remove_file(path)
                            .await
                            .map_err(AppError::internal)?;
                    }
                } else {
                    // Configuration planning/readback is read-only. Only activation creates paths.
                    let parent = path
                        .parent()
                        .ok_or_else(|| AppError::validation("configuration path has no parent"))?;
                    crate::reject_symlink_components(
                        std::path::Path::new(&self.config.fleet.agents_root),
                        path,
                    )
                    .await?;
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(AppError::internal)?;
                    crate::write_configuration_file(path, body.as_bytes()).await?;
                }
            }
            for (path, body) in &files {
                if body.is_empty() && path.file_name().is_some_and(|name| name == "SKILL.md") {
                    if tokio::fs::try_exists(path)
                        .await
                        .map_err(AppError::internal)?
                    {
                        return Err(AppError::validation("disabled skill is still present"));
                    }
                } else if tokio::fs::read(path).await.map_err(AppError::internal)?
                    != body.as_bytes()
                {
                    return Err(AppError::validation(
                        "configuration readback did not match the revision",
                    ));
                }
            }
            if running && self.start(&agent).await?.status != AgentStatus::Running {
                return Err(AppError::validation(
                    "runtime did not pass readiness with the new configuration",
                ));
            }
            Ok::<_, AppError>(())
        }
        .await;
        if let Err(error) = applied {
            if running {
                let _ = self.stop(&agent).await;
            }
            let rollback = async {
                for (path, old) in backups {
                    match old {
                        Some(content) => crate::write_configuration_file(&path, &content).await?,
                        None => {
                            if tokio::fs::try_exists(&path)
                                .await
                                .map_err(AppError::internal)?
                            {
                                tokio::fs::remove_file(path)
                                    .await
                                    .map_err(AppError::internal)?;
                            }
                        }
                    }
                }
                if running && self.start(&agent).await?.status != AgentStatus::Running {
                    return Err(AppError::Unavailable(
                        "configuration rollback restored files but runtime readiness failed".into(),
                    ));
                }
                Ok::<_, AppError>(())
            }
            .await;
            if rollback.is_err() {
                return Err(AppError::Unavailable(
                    "configuration rollback could not be verified; agent remains drained".into(),
                ));
            }
            return Err(error);
        }
        if let Err(error) = self
            .repo
            .insert_event(
                Some(agent.id),
                "config_revision_activated",
                "Configuration applied and read back",
                json!({"revision": revision.revision}),
            )
            .await
        {
            tracing::warn!("configuration applied but event persistence failed: {error}");
        }
        Ok(())
    }

    fn spawn_reconciler(&self) {
        let supervisor = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                let mut tasks = container_workers::AgentTasks::default();
                loop {
                    sleep(RECONCILE_INTERVAL).await;
                    tasks.reap();
                    let Ok(agents) = supervisor.repo.list_agents().await else {
                        continue;
                    };
                    for agent in agents {
                        let supervisor = supervisor.clone();
                        tasks.spawn(agent.id, async move {
                            let Ok(agent) = supervisor.repo.get_agent(agent.id).await else {
                                return;
                            };
                            if supervisor
                                .repo
                                .agent_is_draining(agent.id)
                                .await
                                .unwrap_or(true)
                            {
                                return;
                            }
                            match app::reconcile_action(agent.status, agent.runtime.desired_state) {
                                app::ReconcileAction::Restart => {
                                    tracing::info!(
                                        "reconciler: restarting failed agent {} (desired=running)",
                                        agent.name
                                    );
                                    if let Err(err) = supervisor.restart(&agent).await {
                                        tracing::warn!(
                                            "reconciler restart failed for {}: {err}",
                                            agent.name
                                        );
                                    }
                                }
                                app::ReconcileAction::HealthCheck => {
                                    let _ = supervisor.health(&agent).await;
                                }
                                app::ReconcileAction::Stop | app::ReconcileAction::None => {}
                            }
                            if agent.kind == AgentKind::JavaAgent
                                && agent.status == AgentStatus::Running
                            {
                                let _ = supervisor.sync_java_agent_sessions(&agent).await;
                            }
                        });
                    }
                    let _ = supervisor.process_deployment_jobs().await;
                    let _ = supervisor.sync_project_workflow().await;
                    if let Err(err) = supervisor.alerts.record_heartbeat_freshness().await {
                        tracing::warn!("heartbeat freshness scan failed: {err}");
                    }
                }
            });
        }
    }

    fn hermes_command(&self, agent: &Agent) -> Result<Command, AppError> {
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        let mut command = Command::new(&self.config.fleet.hermes_command);
        command.env_clear();
        for name in [
            "PATH",
            "HOME",
            "USERPROFILE",
            "SYSTEMROOT",
            "WINDIR",
            "TEMP",
            "TMP",
            "LANG",
            "LC_ALL",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        command
            .arg("serve")
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(agent.api_port.unwrap_or_default().to_string())
            .env("HERMES_HOME", &agent.paths.config)
            .env("HERMES_SERVE_HEADLESS", "1")
            .env("API_SERVER_ENABLED", "true")
            .env("API_SERVER_KEY", token)
            .env(
                "API_SERVER_CORS_ORIGINS",
                self.config.server.cors_allowed_origins.join(","),
            )
            .current_dir(&agent.paths.workspace)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        Ok(command)
    }

    fn java_agent_command(&self, agent: &Agent) -> Result<Command, AppError> {
        // runtime/backend.jar is provisioned into the agent layout by the
        // build pipeline (gradle bootJar copied to agents/agentN/runtime).
        let jar = std::path::Path::new(&agent.paths.runtime).join("backend.jar");
        if !jar.exists() {
            return Err(AppError::validation(format!(
                "java agent jar is not provisioned: {}",
                jar.display()
            )));
        }
        let port = agent
            .api_port
            .ok_or_else(|| AppError::validation("agent api_port is required"))?;
        let mut command = Command::new(&self.config.fleet.java_agent_command);
        command
            .arg("-jar")
            .arg(&jar)
            .arg(format!("--server.port={port}"))
            .arg("--spring.profiles.active=noop")
            .env("AGENT_SERVER_PORT", port.to_string())
            .current_dir(&agent.paths.workspace)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        Ok(command)
    }

    /// Pull /api/v2/sessions from a running java agent into Fleet Control.
    async fn sync_java_agent_sessions(&self, agent: &Agent) -> Result<u64, AppError> {
        let base = self.hermes_base_url(agent).await?;
        let response = self
            .client
            .get(format!("{base}/api/v2/sessions?limit=100"))
            .send()
            .await
            .map_err(AppError::internal)?;
        if !response.status().is_success() {
            return Err(AppError::validation(format!(
                "java agent /api/v2/sessions returned {}",
                response.status()
            )));
        }
        let payload: Value = response.json().await.map_err(AppError::internal)?;
        let Some(items) = payload.get("data").and_then(Value::as_array) else {
            return Ok(0);
        };
        let snapshots = items
            .iter()
            .filter_map(|item| {
                let external_id = item.get("id").and_then(Value::as_str)?.to_string();
                let title = item
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let created_at = item
                    .get("createdAt")
                    .and_then(Value::as_str)
                    .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok());
                let updated_at = item
                    .get("updatedAt")
                    .and_then(Value::as_str)
                    .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok());
                Some(RuntimeSessionSnapshot {
                    external_id,
                    title,
                    created_at,
                    updated_at,
                })
            })
            .collect::<Vec<_>>();
        let applied = self.repo.sync_runtime_sessions(agent.id, snapshots).await?;
        Ok(applied)
    }

    /// Process queued deployment jobs: trigger a Forge pipeline per job and
    /// mirror its terminal status back onto the job.
    async fn process_deployment_jobs(&self) -> Result<u64, AppError> {
        let jobs = self.repo.list_deployment_jobs(20).await?;
        let queued: Vec<_> = jobs
            .into_iter()
            .filter(|job| {
                job.state == DeploymentJobState::Queued
                    && matches!(
                        job.job_kind,
                        DeploymentJobKind::Provision | DeploymentJobKind::RuntimeUpdate
                    )
            })
            .collect();
        let Some((api_url, token, project_name)) = self.forge_settings() else {
            // Forge integration not configured: leave jobs queued (visible in UI).
            return Ok(0);
        };
        let mut processed = 0u64;
        for job in queued {
            let running = self
                .repo
                .update_deployment_job_state(job.id, DeploymentJobState::Running, None, None)
                .await?;
            match self
                .trigger_forge_pipeline(&api_url, &token, &project_name, &running)
                .await
            {
                Ok(pipeline_id) => {
                    self.repo
                        .update_deployment_job_state(
                            job.id,
                            DeploymentJobState::Completed,
                            Some(json!({
                                "forge_pipeline_id": pipeline_id,
                                "forge_project": project_name,
                            })),
                            None,
                        )
                        .await?;
                }
                Err(err) => {
                    self.repo
                        .update_deployment_job_state(
                            job.id,
                            DeploymentJobState::Failed,
                            None,
                            Some(err.to_string()),
                        )
                        .await?;
                }
            }
            processed += 1;
        }
        processed += self.process_product_jobs(&api_url, &token).await?;
        Ok(processed)
    }

    async fn process_product_jobs(&self, api_url: &str, token: &str) -> Result<u64, AppError> {
        let jobs = self.repo.list_deployment_jobs(500).await?;
        let mut processed = 0;
        for job in jobs.into_iter().filter(|job| {
            matches!(
                job.job_kind,
                DeploymentJobKind::ProductDeploy | DeploymentJobKind::ProductRollback
            ) && matches!(
                job.state,
                DeploymentJobState::Queued | DeploymentJobState::Running
            )
        }) {
            let job = self.repo.get_deployment_job(job.id).await?;
            if !matches!(
                job.state,
                DeploymentJobState::Queued | DeploymentJobState::Running
            ) {
                continue;
            }
            if let Err(error) = self.process_product_job(api_url, token, &job).await {
                let current = self.repo.get_deployment_job(job.id).await?;
                if let Some(deployment_id) = current
                    .detail
                    .get("forge_deployment_id")
                    .and_then(Value::as_str)
                {
                    let _ = self
                        .client
                        .post(format!(
                            "{}/api/v1/deployments/{deployment_id}/cancel",
                            api_url.trim_end_matches('/')
                        ))
                        .bearer_auth(token)
                        .send()
                        .await;
                }
                self.repo
                    .update_deployment_job_state(
                        job.id,
                        DeploymentJobState::Failed,
                        None,
                        Some(error.to_string()),
                    )
                    .await?;
            }
            processed += 1;
        }
        Ok(processed)
    }

    async fn process_product_job(
        &self,
        api_url: &str,
        token: &str,
        job: &DeploymentJob,
    ) -> Result<(), AppError> {
        let running = if job.state == DeploymentJobState::Queued {
            self.repo
                .update_deployment_job_state(job.id, DeploymentJobState::Running, None, None)
                .await?
        } else {
            job.clone()
        };
        if running.state != DeploymentJobState::Running {
            return Ok(());
        }
        let base = api_url.trim_end_matches('/');
        let (deployment_id, environment_id) = match (
            running
                .detail
                .get("forge_deployment_id")
                .and_then(Value::as_str),
            running
                .detail
                .get("forge_environment_id")
                .and_then(Value::as_str),
        ) {
            (Some(deployment_id), Some(environment_id)) => {
                (deployment_id.to_owned(), environment_id.to_owned())
            }
            _ => {
                let (deployment_id, environment_id) =
                    self.start_product_release(base, token, &running).await?;
                if self.repo.get_deployment_job(job.id).await?.state != DeploymentJobState::Running
                {
                    let _ = self
                        .client
                        .post(format!("{base}/api/v1/deployments/{deployment_id}/cancel"))
                        .bearer_auth(token)
                        .send()
                        .await;
                    return Ok(());
                }
                self.repo
                    .update_deployment_job_state(
                        job.id,
                        DeploymentJobState::Running,
                        Some(json!({
                            "forge_deployment_id": deployment_id,
                            "forge_environment_id": environment_id,
                        })),
                        None,
                    )
                    .await?;
                (deployment_id, environment_id)
            }
        };
        let created =
            chrono::DateTime::parse_from_rfc3339(&job.created_at).map_err(AppError::internal)?;
        if chrono::Utc::now()
            .signed_duration_since(created)
            .num_minutes()
            >= 30
        {
            return Err(AppError::validation(
                "product deployment timed out after 30 minutes",
            ));
        }
        let deployments: Value = self
            .client
            .get(format!(
                "{base}/api/v1/environments/{environment_id}/deployments"
            ))
            .bearer_auth(token)
            .send()
            .await
            .map_err(AppError::internal)?
            .error_for_status()
            .map_err(AppError::internal)?
            .json()
            .await
            .map_err(AppError::internal)?;
        let deployment = deployments
            .as_array()
            .and_then(|items| {
                items.iter().find(|item| {
                    item.get("id").and_then(Value::as_str) == Some(deployment_id.as_str())
                })
            })
            .ok_or_else(|| AppError::validation("Forge deployment is missing"))?;
        let status = deployment
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if status == "failed" {
            return Err(AppError::validation("Forge deployment failed"));
        }
        let Some(pipeline_id) = deployment.get("pipeline_id").and_then(Value::as_str) else {
            return Ok(());
        };
        let pipeline: Value = self
            .client
            .get(format!("{base}/api/v1/pipelines/{pipeline_id}"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(AppError::internal)?
            .error_for_status()
            .map_err(AppError::internal)?
            .json()
            .await
            .map_err(AppError::internal)?;
        let pipeline_status = pipeline
            .get("pipeline")
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        if matches!(pipeline_status, "failed" | "canceled") {
            return Err(AppError::validation(format!(
                "Forge pipeline {pipeline_status}"
            )));
        }
        self.repo
            .update_deployment_job_state(
                job.id,
                DeploymentJobState::Running,
                Some(json!({ "forge_pipeline_id": pipeline_id })),
                None,
            )
            .await?;
        if pipeline_status != "success" || status != "success" {
            return Ok(());
        }
        let api_health = self
            .config
            .fleet
            .pulse_health_url
            .as_deref()
            .ok_or_else(|| AppError::validation("Pulse API health URL is not configured"))?;
        let ui_health = self
            .config
            .fleet
            .pulse_ui_url
            .as_deref()
            .ok_or_else(|| AppError::validation("Pulse UI health URL is not configured"))?;
        for (label, url) in [("API", api_health), ("UI", ui_health)] {
            let response = self
                .client
                .get(url)
                .send()
                .await
                .map_err(AppError::internal)?;
            if !response.status().is_success() {
                return Err(AppError::validation(format!(
                    "Pulse {label} health returned {}",
                    response.status()
                )));
            }
        }
        self.repo
            .update_deployment_job_state(
                job.id,
                DeploymentJobState::Completed,
                Some(json!({ "health_verified": true })),
                None,
            )
            .await?;
        Ok(())
    }

    async fn start_product_release(
        &self,
        base: &str,
        token: &str,
        job: &DeploymentJob,
    ) -> Result<(String, String), AppError> {
        let projects: Value = self
            .client
            .get(format!("{base}/api/v1/projects"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(AppError::internal)?
            .error_for_status()
            .map_err(AppError::internal)?
            .json()
            .await
            .map_err(AppError::internal)?;
        let project_id = projects
            .as_array()
            .and_then(|items| {
                items.iter().find(|item| {
                    item.get("repository_url")
                        .and_then(Value::as_str)
                        .is_some_and(|url| url.ends_with("/service-pulse.git"))
                })
            })
            .and_then(|item| item.get("id"))
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::validation("Service Pulse Forge project not found"))?;
        let environments: Value = self
            .client
            .get(format!("{base}/api/v1/projects/{project_id}/environments"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(AppError::internal)?
            .error_for_status()
            .map_err(AppError::internal)?
            .json()
            .await
            .map_err(AppError::internal)?;
        let environment_id = environments
            .as_array()
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item.get("name").and_then(Value::as_str) == Some("demo"))
            })
            .and_then(|item| item.get("id"))
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::validation("Forge demo environment not found"))?
            .to_owned();
        let request_key = job
            .detail
            .get("idempotency_key")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::validation("product job missing idempotency key"))?;
        let request = match job.job_kind {
            DeploymentJobKind::ProductDeploy => {
                let sha = job
                    .detail
                    .get("commit_sha")
                    .and_then(Value::as_str)
                    .ok_or_else(|| AppError::validation("product job missing commit SHA"))?;
                self.client
                    .post(format!(
                        "{base}/api/v1/environments/{environment_id}/deployments"
                    ))
                    .bearer_auth(token)
                    .json(&json!({ "git_ref": sha, "request_key": request_key }))
            }
            DeploymentJobKind::ProductRollback => {
                let previous = job
                    .detail
                    .get("previous_release_id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| AppError::validation("product job missing previous release"))?;
                self.client
                    .post(format!("{base}/api/v1/deployments/{previous}/rollback"))
                    .bearer_auth(token)
                    .json(&json!({ "request_key": request_key }))
            }
            _ => return Err(AppError::validation("not a product deployment job")),
        };
        let deployment: Value = request
            .send()
            .await
            .map_err(AppError::internal)?
            .error_for_status()
            .map_err(AppError::internal)?
            .json()
            .await
            .map_err(AppError::internal)?;
        let deployment_id = deployment
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::internal("Forge deployment response missing id"))?;
        Ok((deployment_id.to_owned(), environment_id))
    }

    /// Pull the project-workflow namespace/workflow catalog and refresh
    /// workflow binding statuses (Phase 3: project-workflow API sync).
    async fn sync_project_workflow(&self) -> Result<u64, AppError> {
        if self.config.fleet.project_workflow_url.is_none()
            || self.config.fleet.project_workflow_catalog_token.is_none()
        {
            return Ok(0);
        }
        let catalog = app::fetch_workflow_catalog(&self.config).await?;
        let known_namespaces: Vec<(String, String)> = catalog
            .namespaces
            .into_iter()
            .map(|item| (item.id, item.name))
            .collect();
        let known_workflows: Vec<(String, String)> = catalog
            .workflows
            .into_iter()
            .map(|item| (item.id, item.name))
            .collect();
        self.repo
            .refresh_workflow_bindings(known_namespaces, known_workflows)
            .await
    }

    fn forge_settings(&self) -> Option<(String, String, String)> {
        let fleet = &self.config.fleet;
        let api_url = fleet.forge_api_url.clone()?;
        let project_name = fleet.forge_project.clone()?;
        let token = fleet.forge_api_token.clone().unwrap_or_default();
        Some((api_url, token, project_name))
    }

    async fn trigger_forge_pipeline(
        &self,
        api_url: &str,
        token: &str,
        project_name: &str,
        job: &DeploymentJob,
    ) -> Result<String, AppError> {
        let base = api_url.trim_end_matches('/');
        let projects: Value = self
            .client
            .get(format!("{base}/api/v1/projects"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(AppError::internal)?
            .json()
            .await
            .map_err(AppError::internal)?;
        let project_id = projects
            .as_array()
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item.get("name").and_then(Value::as_str) == Some(project_name))
                    .and_then(|item| item.get("id"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .ok_or_else(|| {
                AppError::validation(format!("forge project {project_name} not found"))
            })?;
        let pipeline: Value = self
            .client
            .post(format!("{base}/api/v1/projects/{project_id}/pipelines"))
            .bearer_auth(token)
            .json(&serde_json::json!({
                "git_ref": "main",
                "variables": {
                    "FLEET_JOB_ID": job.id.to_string(),
                    "FLEET_JOB_KIND": job.job_kind.as_str(),
                }
            }))
            .send()
            .await
            .map_err(AppError::internal)?
            .json()
            .await
            .map_err(AppError::internal)?;
        pipeline
            .get("pipeline")
            .and_then(|p| p.get("id"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| AppError::internal("forge pipeline response missing id"))
    }

    async fn probe_java_agent(&self, agent: &Agent) -> Result<Value, AppError> {
        // Readiness is db-only per the java-agent contract; optional
        // components (browser CDP, model) may be DOWN while the runtime
        // still serves traffic, so probe readiness, not the aggregate.
        let base = self.hermes_base_url(agent).await?;
        let readiness = self
            .client
            .get(format!("{base}/actuator/health/readiness"))
            .send()
            .await
            .map_err(AppError::internal)?;
        if !readiness.status().is_success() {
            return Err(AppError::validation(format!(
                "java agent /actuator/health/readiness returned {}",
                readiness.status()
            )));
        }
        let readiness: Value = readiness.json().await.map_err(AppError::internal)?;
        if readiness.get("status").and_then(Value::as_str) != Some("UP") {
            return Err(AppError::validation(
                "java agent readiness status is not UP",
            ));
        }
        Ok(readiness)
    }

    async fn wait_for_java_agent_ready(&self, agent: &Agent) -> Result<Value, AppError> {
        let mut elapsed = Duration::ZERO;
        let mut last_error = "java agent readiness probe did not run".to_string();
        while elapsed < HERMES_READY_TIMEOUT {
            match self.probe_java_agent(agent).await {
                Ok(health) => return Ok(health),
                Err(err) => last_error = err.to_string(),
            }
            sleep(HERMES_READY_POLL).await;
            elapsed += HERMES_READY_POLL;
        }
        Err(AppError::validation(format!(
            "java agent did not become ready: {last_error}"
        )))
    }

    async fn spawn_log_reader<R>(&self, agent_id: Uuid, stream: &'static str, reader: R)
    where
        R: tokio::io::AsyncRead + Unpin + Send + 'static,
    {
        let repo = self.repo.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        let _ = repo
                            .insert_log(agent_id, stream, &crate::redact_text(&line))
                            .await;
                    }
                    Ok(None) => break,
                    Err(err) => {
                        let _ = repo
                            .insert_log(agent_id, stream, &format!("log reader failed: {err}"))
                            .await;
                        break;
                    }
                }
            }
        });
    }

    async fn hermes_base_url(&self, agent: &Agent) -> Result<String, AppError> {
        if self.container_mode(agent).await? {
            return self.container_origin(agent).await;
        }
        let port = agent
            .api_port
            .ok_or_else(|| AppError::validation("agent api_port is required"))?;
        Ok(format!("http://127.0.0.1:{port}"))
    }

    async fn run_base_url(&self, agent: &Agent, run: &SessionAgentRun) -> Result<String, AppError> {
        if self.container_mode(agent).await? {
            self.container_run_origin(agent, run).await
        } else {
            self.hermes_base_url(agent).await
        }
    }

    fn runtime_session_id(session: &AgentSession, agent: &Agent) -> String {
        format!("fleet:{}:{}", session.id, agent.id)
    }

    fn run_role(session: &AgentSession, agent: &Agent) -> SessionRunRole {
        if agent.product_role == AgentProductRole::Leader
            || session.leader_agent_id == Some(agent.id)
        {
            SessionRunRole::Leader
        } else if session.primary_agent_id == agent.id {
            SessionRunRole::Primary
        } else {
            SessionRunRole::Executor
        }
    }

    fn runtime_input(agent: &Agent, session: &AgentSession, message: &SessionMessage) -> String {
        if message.author_type == MessageAuthorType::Agent
            && message.author_agent_id != Some(agent.id)
        {
            return format!(
                "[Fleet Control]\nSession: {}\nTask: {}\nMessage from leader agent: {}\n\n{}",
                session.title,
                session.task_key.as_deref().unwrap_or("not set"),
                message
                    .author_agent_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                message.body
            );
        }
        message.body.clone()
    }

    async fn probe_hermes(&self, agent: &Agent) -> Result<Value, AppError> {
        let base = self.hermes_base_url(agent).await?;
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        self.probe_hermes_at(&base, &token).await
    }

    // Callers must derive this origin from original Base endpoint readback, never configuration.
    async fn probe_hermes_at(&self, base: &str, token: &str) -> Result<Value, AppError> {
        let health = self
            .client
            .get(format!("{base}/health"))
            .timeout(Duration::from_secs(3))
            .bearer_auth(token)
            .send()
            .await
            .map_err(AppError::internal)?;
        if !health.status().is_success() {
            return Err(AppError::validation(format!(
                "Hermes /health returned {}",
                health.status()
            )));
        }
        let capabilities = self
            .client
            .get(format!("{base}/v1/capabilities"))
            .header(reqwest::header::ACCEPT_ENCODING, "identity")
            .timeout(Duration::from_secs(3))
            .bearer_auth(token)
            .send()
            .await
            .map_err(AppError::internal)?;
        if !capabilities.status().is_success() {
            return Err(AppError::validation(format!(
                "Hermes /v1/capabilities returned {}",
                capabilities.status()
            )));
        }
        let capabilities =
            hermes_wire::read_json(capabilities, reqwest::StatusCode::OK, 262_144).await?;
        for feature in ["run_status", "run_events_sse", "run_stop"] {
            if capabilities
                .get("features")
                .and_then(|features| features.get(feature))
                .and_then(Value::as_bool)
                != Some(true)
            {
                return Err(AppError::validation(format!(
                    "Hermes capability {feature} is required"
                )));
            }
        }
        Ok(capabilities)
    }

    async fn wait_for_hermes_ready(&self, agent: &Agent) -> Result<Value, AppError> {
        let mut elapsed = Duration::ZERO;
        let mut last_error = "Hermes readiness probe did not run".to_string();
        while elapsed < HERMES_READY_TIMEOUT {
            match self.probe_hermes(agent).await {
                Ok(capabilities) => return Ok(capabilities),
                Err(err) => last_error = err.to_string(),
            }
            sleep(HERMES_READY_POLL).await;
            elapsed += HERMES_READY_POLL;
        }
        Err(AppError::validation(format!(
            "Hermes did not become ready: {last_error}"
        )))
    }

    async fn start_hermes_run(
        &self,
        agent: &Agent,
        session: &AgentSession,
        message: &SessionMessage,
    ) -> Result<(SessionAgentRun, String), AppError> {
        if let Some(binding) = self.repo.get_task_chat_binding(session.id).await? {
            if binding.agent_id != agent.id || agent.id != session.primary_agent_id {
                return Err(AppError::conflict(
                    "task runtime identity does not match binding",
                ));
            }
            let capabilities = self.probe_hermes(agent).await?;
            hermes_wire::task_protocol(&capabilities)?;
            return Err(AppError::Unavailable(
                "task runtime admission is not yet verified".into(),
            ));
        }
        let capabilities = self.probe_hermes(agent).await?;
        let mut capabilities = hermes_wire::dispatch_capabilities(&capabilities)?;
        self.bind_container_dispatch(agent, &mut capabilities)
            .await?;
        let base = self.hermes_base_url(agent).await?;
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        if self.config.fleet.hermes_recovery_extension_enabled {
            capabilities["fleet_recovery"] =
                recovery_wire::capabilities(&self.client, &base, &token).await?;
        }
        let fingerprint = hermes_wire::credential_fingerprint(&token);
        self.repo
            .prepare_hermes_dispatch(app::HermesDispatchDraft {
                message_id: message.id,
                session_id: session.id,
                agent_id: agent.id,
                run_role: Self::run_role(session, agent),
                requested_session_id: Self::runtime_session_id(session, agent),
                input: Self::runtime_input(agent, session, message),
                origin: base.clone(),
                credential_fingerprint: fingerprint.clone(),
                capabilities,
            })
            .await?;
        // Consume the durable permit before any network side effect. An unknown POST
        // may not be repeated merely because the native idempotency key was saved.
        let claimed = self
            .repo
            .claim_hermes_submission(message.id, base.clone(), fingerprint)
            .await?
            .ok_or_else(|| {
                AppError::Unavailable(
                    "Hermes submission was already attempted; reconciliation is required".into(),
                )
            })?;
        if claimed.message_id != message.id
            || claimed.run.agent_id != agent.id
            || claimed.run.session_id != session.id
        {
            return Err(AppError::conflict(
                "Hermes submission journal scope does not match",
            ));
        }
        hermes_wire::verify_intent(&claimed, &base, &token)?;
        self.verify_container_intent(agent, &claimed).await?;
        let runtime_run_id = hermes_wire::submit(
            &self.client,
            &base,
            &token,
            message.id,
            &claimed.request_body,
            recovery_wire::store_id(&claimed.capabilities)?.as_deref(),
        )
        .await?;
        let run = self
            .repo
            .accept_hermes_run(message.id, claimed.run.id, runtime_run_id.clone())
            .await?;
        // ACK is durable even if HTTP readback fails. The recovery worker only reads this run.
        let run = self
            .finish_acceptance_readback(agent, session, message, &run)
            .await
            .unwrap_or(run);
        Ok((run, runtime_run_id))
    }

    fn spawn_hermes_event_worker(
        &self,
        agent: Agent,
        session: AgentSession,
        message: SessionMessage,
        run: SessionAgentRun,
        runtime_run_id: String,
    ) {
        let supervisor = self.clone();
        tokio::spawn(async move {
            if let Err(err) = supervisor
                .follow_hermes_events(
                    agent.clone(),
                    session.clone(),
                    Some(message.clone()),
                    run.clone(),
                    runtime_run_id.clone(),
                )
                .await
            {
                let redacted = crate::redact_text(&err.to_string());
                let _ = supervisor
                    .repo
                    .update_session_message_delivery(
                        message.id,
                        MessageDeliveryState::Dispatched,
                        Some(runtime_run_id.clone()),
                        Some(redacted.clone()),
                    )
                    .await;
                if let Ok(updated) = supervisor
                    .repo
                    .update_session_agent_run_dispatch(
                        run.id,
                        Some(runtime_run_id.clone()),
                        SessionRunState::Waiting,
                        Some(redacted.clone()),
                    )
                    .await
                {
                    supervisor.emit_run(&updated);
                }
                let _ = supervisor
                    .repo
                    .insert_event(
                        Some(agent.id),
                        "hermes_event_stream_failed",
                        &redacted,
                        json!({ "session_id": session.id, "run_id": run.id, "runtime_run_id": runtime_run_id }),
                    )
                    .await;
            }
        });
    }

    async fn follow_hermes_events(
        &self,
        agent: Agent,
        session: AgentSession,
        message: Option<SessionMessage>,
        run: SessionAgentRun,
        runtime_run_id: String,
    ) -> Result<(), AppError> {
        let base;
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        if let Some(message) = &message {
            base = self.run_base_url(&agent, &run).await?;
            let intent = self
                .repo
                .get_hermes_dispatch_intent(message.id)
                .await?
                .ok_or_else(|| {
                    AppError::Unavailable("Hermes stream has no original dispatch context".into())
                })?;
            if intent.state != "accepted"
                || intent.run.id != run.id
                || intent.run.runtime_run_id.as_deref() != Some(runtime_run_id.as_str())
                || intent.run.runtime_session_id != run.runtime_session_id
            {
                return Err(AppError::conflict("Hermes stream identity changed"));
            }
            hermes_wire::verify_intent(&intent, &base, &token)?;
            if matches!(
                intent.run.state,
                SessionRunState::Completed | SessionRunState::Failed | SessionRunState::Cancelled
            ) {
                return Ok(());
            }
        } else {
            let (record, committed) = self.repo.pm_stream_context(run.id).await?;
            pm_events::verify_stream_pin(&record, &run, &runtime_run_id)?;
            let intent = self
                .repo
                .get_pm_dispatch(run.id)
                .await?
                .ok_or_else(pm_dispatch::unavailable)?;
            base = pm_dispatch::verify_context(self, &agent, &intent).await?;
            if committed {
                return Ok(());
            }
        }
        let response = tokio::time::timeout(
            Duration::from_secs(10),
            self.client
                .get(format!("{base}/v1/runs/{runtime_run_id}/events"))
                .header(reqwest::header::ACCEPT, "text/event-stream")
                .header(reqwest::header::ACCEPT_ENCODING, "identity")
                .bearer_auth(token)
                .send(),
        )
        .await
        .map_err(|_| AppError::Unavailable("Hermes stream headers timed out".into()))?
        .map_err(AppError::internal)?;
        sse_wire::verify_headers(&response)?;

        let mut stream = response.bytes_stream();
        let mut decoder = sse_wire::Decoder::default();
        let mut budget = sse_wire::Budget::new(tokio::time::Instant::now());
        let mut transcript = sse_wire::Transcript::default();
        let mut terminal_watch = tokio::time::interval(Duration::from_secs(5));

        loop {
            let chunk = tokio::select! {
                chunk = stream.next() => chunk,
                _ = tokio::time::sleep_until(budget.deadline()) => {
                    return Err(AppError::Unavailable("Hermes event stream deadline elapsed".into()));
                }
                _ = terminal_watch.tick() => {
                    let current = self.repo.get_session_agent_run(run.id).await?;
                    if matches!(current.state, SessionRunState::Completed | SessionRunState::Failed | SessionRunState::Cancelled) {
                        if message.is_some() || self.repo.pm_stream_context(run.id).await?.1 {
                            return Ok(());
                        }
                    }
                    continue;
                }
            };
            let Some(chunk) = chunk else {
                break;
            };
            let chunk = chunk
                .map_err(|_| AppError::Unavailable("Hermes event stream was interrupted".into()))?;
            let now = tokio::time::Instant::now();
            budget.receive(chunk.len(), now)?;
            for &byte in &chunk {
                let event = decoder.push(byte)?;
                budget.frame(decoder.pending(), now);
                if let Some(event) = event {
                    budget.event()?;
                    if self
                        .handle_hermes_event(
                            &agent,
                            &session,
                            message.as_ref(),
                            &run,
                            &runtime_run_id,
                            event.name,
                            event.data,
                            &mut transcript,
                        )
                        .await?
                    {
                        return Ok(());
                    }
                }
            }
        }

        // An incomplete final frame is never dispatched. EOF requires independent status proof.
        let response = self
            .client
            .get(format!("{base}/v1/runs/{runtime_run_id}"))
            .header(reqwest::header::ACCEPT_ENCODING, "identity")
            .timeout(Duration::from_secs(10))
            .bearer_auth(crate::agent_runtime_token(&self.config, agent.id)?)
            .send()
            .await
            .map_err(|_| AppError::Unavailable("Hermes terminal readback is unavailable".into()))?;
        let payload =
            hermes_wire::read_json(response, reqwest::StatusCode::OK, 1024 * 1024).await?;
        if hermes_wire::effective_session(&payload, &runtime_run_id)?
            != run.runtime_session_id.as_deref().unwrap_or_default()
        {
            return Err(AppError::Unavailable(
                "Hermes terminal session identity changed".into(),
            ));
        }
        let event = hermes_wire::terminal_readback(&payload, &runtime_run_id)?;
        self.handle_hermes_event(
            &agent,
            &session,
            message.as_ref(),
            &run,
            &runtime_run_id,
            Some(event.to_string()),
            payload.to_string(),
            &mut transcript,
        )
        .await?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_hermes_event(
        &self,
        agent: &Agent,
        session: &AgentSession,
        message: Option<&SessionMessage>,
        run: &SessionAgentRun,
        runtime_run_id: &str,
        event_name: Option<String>,
        data: String,
        transcript: &mut sse_wire::Transcript,
    ) -> Result<bool, AppError> {
        let payload: Value = serde_json::from_str(&data)
            .map_err(|_| AppError::Unavailable("Hermes event JSON is malformed".into()))?;
        if !payload.is_object()
            || payload.get("run_id").and_then(Value::as_str) != Some(runtime_run_id)
            || payload
                .get("session_id")
                .is_some_and(|id| id.as_str() != run.runtime_session_id.as_deref())
            || payload.get("event").is_some_and(|name| {
                name.as_str().is_none()
                    || event_name
                        .as_ref()
                        .is_some_and(|expected| name.as_str() != Some(expected.as_str()))
            })
        {
            return Err(AppError::Unavailable(
                "Hermes event identity does not match".into(),
            ));
        }
        let event_type = event_name
            .or_else(|| {
                payload
                    .get("event")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "message".to_string());
        let terminal_state = hermes_wire::terminal_event(&event_type, &payload, runtime_run_id)?;
        if message.is_none() {
            let (record, committed) = self.repo.pm_stream_context(run.id).await?;
            pm_events::verify_stream_pin(&record, run, runtime_run_id)?;
            let intent = self
                .repo
                .get_pm_dispatch(run.id)
                .await?
                .ok_or_else(pm_dispatch::unavailable)?;
            pm_dispatch::verify_context(self, agent, &intent).await?;
            if committed && terminal_state.is_none() {
                return Ok(true);
            }
        }
        if terminal_state.is_none() {
            let current = self.repo.get_session_agent_run(run.id).await?;
            if matches!(
                current.state,
                SessionRunState::Completed | SessionRunState::Failed | SessionRunState::Cancelled
            ) {
                return if message.is_none() && !self.repo.pm_stream_context(run.id).await?.1 {
                    Ok(false)
                } else {
                    Ok(true)
                };
            }
        }
        if message.is_some() && terminal_state.is_some() && self.container_mode(agent).await? {
            self.container_run_origin(agent, run).await?;
        }

        if event_type.contains("delta") {
            if let Some(delta) = pick_string(&payload, &["delta", "text", "output_text"]) {
                transcript.append(&delta)?;
                let mut secrets: Vec<String> = std::env::vars()
                    .filter(|(name, _)| name.starts_with("FLEET_CONTROL_SECRET__"))
                    .map(|(_, value)| value)
                    .collect();
                secrets.push(crate::agent_runtime_token(&self.config, agent.id)?);
                let text = crate::redact_stream_text(&transcript.text, &secrets);
                transcript.mirror(&text)?;
                self.repo
                    .append_session_event(
                        session.id,
                        "session_run_delta",
                        json!({
                            "type": "session_run_delta", "session_id": session.id, "run_id": run.id,
                            "runtime_run_id": runtime_run_id, "text": text,
                        }),
                    )
                    .await?;
            }
            return Ok(false);
        }

        if matches!(
            event_type.as_str(),
            "approval.request" | "approval.requested"
        ) {
            let prompt = pick_string(&payload, &["prompt", "description", "command", "message"])
                .unwrap_or_else(|| "Hermes approval requested".to_string());
            let runtime_approval_id = pick_string(&payload, &["approval_id", "request_id", "id"]);
            let approval = self
                .repo
                .upsert_runtime_approval_request(RuntimeApprovalCreate {
                    session_id: session.id,
                    session_run_id: run.id,
                    agent_id: agent.id,
                    runtime_run_id: runtime_run_id.to_string(),
                    runtime_approval_id,
                    prompt: prompt.clone(),
                    detail: payload.clone(),
                })
                .await?;
            let _ = self
                .repo
                .insert_session_message_mirror(
                    session.id,
                    Some(agent.id),
                    format!("Approval requested: {prompt}"),
                    MessageKind::ToolEvent,
                    Some(runtime_run_id.to_string()),
                )
                .await?;
            let _ = self.events.send(FleetEvent::RuntimeApprovalRequested {
                session_id: session.id.to_string(),
                run_id: run.id.to_string(),
                approval_id: approval.id.to_string(),
            });
            if let Ok(updated) = self
                .repo
                .update_session_agent_run_dispatch(
                    run.id,
                    Some(runtime_run_id.to_string()),
                    SessionRunState::Waiting,
                    None,
                )
                .await
            {
                self.emit_run(&updated);
            }
            return Ok(false);
        }

        if event_type.contains("tool") {
            let tool_text = pick_string(&payload, &["message", "name", "text"])
                .unwrap_or_else(|| format!("Hermes tool event: {event_type}"));
            let _ = self
                .repo
                .insert_session_message_mirror(
                    session.id,
                    Some(agent.id),
                    tool_text,
                    MessageKind::ToolEvent,
                    Some(runtime_run_id.to_string()),
                )
                .await?;
            return Ok(false);
        }

        if let Some(state) = terminal_state {
            let body = if state == SessionRunState::Completed {
                [
                    "final_response",
                    "output",
                    "output_text",
                    "content",
                    "response",
                    "message",
                    "text",
                ]
                .into_iter()
                .find_map(|key| payload.get(key).and_then(Value::as_str).map(str::to_owned))
                .or_else(|| (!transcript.text.trim().is_empty()).then(|| transcript.text.clone()))
            } else {
                None
            };
            let error = (state == SessionRunState::Failed).then(|| {
                pick_error(&payload).unwrap_or_else(|| format!("Hermes event {event_type}"))
            });
            let command = app::HermesTerminalCommit {
                message_id: message.map(|m| m.id).unwrap_or(run.id),
                run_id: run.id,
                runtime_run_id: runtime_run_id.to_owned(),
                runtime_session_id: run.runtime_session_id.clone().ok_or_else(|| {
                    AppError::Unavailable("Hermes effective session is not pinned".into())
                })?,
                state,
                body: body
                    .map(|text| self.redact_hermes_text(agent.id, &text))
                    .transpose()?,
                error: error
                    .map(|text| self.redact_hermes_text(agent.id, &text))
                    .transpose()?,
            };
            let (updated, assistant, first) = if message.is_some() {
                self.repo.commit_hermes_terminal(command).await?
            } else {
                let record = self.repo.get_pm_run(run.id).await?;
                let status = self.probe_pm_run(agent, &record).await?;
                self.repo.commit_pm_terminal(command, status).await?
            };
            if first {
                self.emit_run(&updated);
                if let Some(assistant) = assistant {
                    let _ = self.events.send(FleetEvent::SessionMessageChanged {
                        session_id: session.id.to_string(),
                        message_id: assistant.id.to_string(),
                        event: "message.completed".to_string(),
                    });
                }
            }
            return Ok(true);
        }

        Ok(false)
    }

    fn redact_hermes_text(&self, agent: Uuid, text: &str) -> Result<String, AppError> {
        let mut secrets: Vec<String> = std::env::vars()
            .filter(|(name, _)| name.starts_with("FLEET_CONTROL_SECRET__"))
            .map(|(_, value)| value)
            .collect();
        secrets.push(crate::agent_runtime_token(&self.config, agent)?);
        let mut redacted = crate::redact_text(text);
        for secret in secrets {
            if !secret.is_empty() {
                redacted = redacted.replace(&secret, "redacted");
            }
        }
        Ok(redacted)
    }

    fn emit_run(&self, run: &SessionAgentRun) {
        let _ = self.events.send(FleetEvent::SessionRunChanged {
            session_id: run.session_id.to_string(),
            run_id: run.id.to_string(),
            runtime_run_id: run.runtime_run_id.clone(),
            state: run.state.as_str().to_string(),
        });
    }
}

fn control_message(command: &domain::RuntimeControlReceipt) -> &'static str {
    match command.state {
        domain::RuntimeControlState::Acknowledged => {
            "Runtime acknowledged the command; terminal readback is independent"
        }
        domain::RuntimeControlState::Rejected => "Command was not dispatched",
        domain::RuntimeControlState::TerminalObserved => {
            "Run terminal observed; command acceptance remains unknown"
        }
        _ => "Command acceptance is unknown; do not resend with a new key",
    }
}

fn parse_domain_ts(value: &Option<domain::Timestamp>) -> Option<shared::Timestamp> {
    value
        .as_deref()
        .and_then(|timestamp| chrono::DateTime::parse_from_rfc3339(timestamp).ok())
}

fn pick_string(value: &Value, keys: &[&str]) -> Option<String> {
    if let Value::String(text) = value {
        return Some(text.clone());
    }
    pick_named_string(value, keys)
}

fn pick_named_string(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(found) = value.get(*key).and_then(Value::as_str) {
            return Some(found.to_string());
        }
    }
    match value {
        Value::Object(map) => map
            .values()
            .find_map(|value| pick_named_string(value, keys)),
        Value::Array(items) => items
            .iter()
            .find_map(|value| pick_named_string(value, keys)),
        _ => None,
    }
}

fn pick_error(value: &Value) -> Option<String> {
    value
        .get("error")
        .and_then(|error| {
            error.as_str().map(ToString::to_string).or_else(|| {
                error
                    .get("message")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
        })
        .or_else(|| pick_string(value, &["error_message", "message", "detail"]))
        .map(|error| crate::redact_text(&error))
}

impl LocalRuntimeSupervisor {
    /// Launches the provisioned java agent jar and waits for actuator UP.
    async fn start_java_agent(&self, agent: &Agent) -> Result<RuntimeOperationResponse, AppError> {
        if self.children.lock().await.contains_key(&agent.id) {
            let updated = self
                .repo
                .update_runtime_state(
                    agent.id,
                    RuntimeStatePatch {
                        status: AgentStatus::Running,
                        desired_state: DesiredState::Running,
                        pid: agent.runtime.pid,
                        health_status: Some("running".to_string()),
                        health_detail: Some("process is already tracked".to_string()),
                        last_capabilities_json: None,
                        startup_command_redacted: Some(self.command_preview(agent)),
                        started_at: parse_domain_ts(&agent.runtime.started_at),
                        stopped_at: None,
                    },
                )
                .await?;
            return Ok(RuntimeOperationResponse {
                agent_id: updated.id,
                status: updated.status,
                message: "java agent is already running".to_string(),
            });
        }

        let starting = self
            .repo
            .update_runtime_state(
                agent.id,
                RuntimeStatePatch {
                    status: AgentStatus::Starting,
                    desired_state: DesiredState::Running,
                    pid: None,
                    health_status: Some("starting".to_string()),
                    health_detail: Some("java agent launch requested".to_string()),
                    last_capabilities_json: None,
                    startup_command_redacted: Some(self.command_preview(agent)),
                    started_at: None,
                    stopped_at: None,
                },
            )
            .await?;
        let _ = self
            .repo
            .insert_log(starting.id, "system", "java agent start requested")
            .await;

        let mut command = self.java_agent_command(&starting)?;
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                let detail = format!("failed to spawn java agent: {err}");
                let updated = self
                    .repo
                    .update_runtime_state(
                        starting.id,
                        RuntimeStatePatch {
                            status: AgentStatus::Failed,
                            desired_state: DesiredState::Stopped,
                            pid: None,
                            health_status: Some("failed".to_string()),
                            health_detail: Some(detail.clone()),
                            last_capabilities_json: None,
                            startup_command_redacted: Some(self.command_preview(&starting)),
                            started_at: None,
                            stopped_at: Some(shared::now()),
                        },
                    )
                    .await?;
                let _ = self.repo.insert_log(updated.id, "stderr", &detail).await;
                return Ok(RuntimeOperationResponse {
                    agent_id: updated.id,
                    status: updated.status,
                    message: "failed to spawn java agent runtime".to_string(),
                });
            }
        };

        if let Some(stdout) = child.stdout.take() {
            self.spawn_log_reader(starting.id, "stdout", stdout).await;
        }
        if let Some(stderr) = child.stderr.take() {
            self.spawn_log_reader(starting.id, "stderr", stderr).await;
        }
        let pid = child.id().map(|id| id as i32);
        self.children.lock().await.insert(starting.id, child);

        match self.wait_for_java_agent_ready(&starting).await {
            Ok(health) => {
                let updated = self
                    .repo
                    .update_runtime_state(
                        starting.id,
                        RuntimeStatePatch {
                            status: AgentStatus::Running,
                            desired_state: DesiredState::Running,
                            pid,
                            health_status: Some("running".to_string()),
                            health_detail: Some("java agent actuator is UP".to_string()),
                            last_capabilities_json: Some(health),
                            startup_command_redacted: Some(self.command_preview(&starting)),
                            started_at: Some(shared::now()),
                            stopped_at: None,
                        },
                    )
                    .await?;
                Ok(RuntimeOperationResponse {
                    agent_id: updated.id,
                    status: updated.status,
                    message: "java agent runtime started and actuator is UP".to_string(),
                })
            }
            Err(err) => {
                if let Some(mut child) = self.children.lock().await.remove(&starting.id) {
                    let _ = child.kill().await;
                }
                let detail = crate::redact_text(&err.to_string());
                let updated = self
                    .repo
                    .update_runtime_state(
                        starting.id,
                        RuntimeStatePatch {
                            status: AgentStatus::Failed,
                            desired_state: DesiredState::Stopped,
                            pid: None,
                            health_status: Some("failed".to_string()),
                            health_detail: Some(detail.clone()),
                            last_capabilities_json: None,
                            startup_command_redacted: Some(self.command_preview(&starting)),
                            started_at: None,
                            stopped_at: Some(shared::now()),
                        },
                    )
                    .await?;
                let _ = self.repo.insert_log(updated.id, "stderr", &detail).await;
                Ok(RuntimeOperationResponse {
                    agent_id: updated.id,
                    status: updated.status,
                    message: detail,
                })
            }
        }
    }
}

#[async_trait]
impl RuntimeSupervisor for LocalRuntimeSupervisor {
    fn authorize_pm_tool(&self, agent: Uuid, bearer: &str) -> Result<(), AppError> {
        use hmac::Mac;
        let expected = crate::agent_runtime_token(&self.config, agent)?;
        let mut expected_mac = crate::HmacSha256::new_from_slice(expected.as_bytes())
            .map_err(|_| AppError::Unauthorized)?;
        let mut supplied_mac = crate::HmacSha256::new_from_slice(bearer.as_bytes())
            .map_err(|_| AppError::Unauthorized)?;
        expected_mac.update(b"fleet-pm-mcp-v1");
        supplied_mac.update(b"fleet-pm-mcp-v1");
        expected_mac
            .verify_slice(&supplied_mac.finalize().into_bytes())
            .map_err(|_| AppError::Unauthorized)
    }

    async fn call_pm_tool(
        &self,
        agent: Uuid,
        name: &str,
        call: domain::PmToolCall,
    ) -> Result<Value, AppError> {
        pm_tools::call(self, agent, name, call).await
    }

    async fn resume_pm_answer(
        &self,
        actor: &domain::ClarificationCommandActor,
        command: &domain::ClarificationAnswerCommand,
    ) -> Result<domain::PmContinuationOutcome, AppError> {
        match pm_continuation::resume(self, actor, command).await {
            Err(AppError::Unavailable(_)) => Ok(domain::PmContinuationOutcome::Pending),
            result => result,
        }
    }

    async fn dispatch_pm_draft(
        &self,
        operation: &domain::PmDraftOperation,
        tracker: &dyn app::pm_draft::PmDraftTracker,
    ) -> Result<(), AppError> {
        pm_dispatch::dispatch(self, operation, tracker).await
    }
    async fn resolve_targeted_approval(
        &self,
        agent: &Agent,
        run: &SessionAgentRun,
        approval: &domain::RuntimeApprovalRequest,
        choice: domain::ApprovalChoice,
    ) -> Result<(), AppError> {
        targeted_approval::resolve(self, agent, run, approval, choice).await
    }
    async fn probe_pm_run(
        &self,
        agent: &Agent,
        record: &domain::PmRunRecord,
    ) -> Result<domain::PmRuntimeStatus, AppError> {
        pm_readback::probe(self, agent, record).await
    }
    async fn start(&self, agent: &Agent) -> Result<RuntimeOperationResponse, AppError> {
        if self.container_mode(agent).await? {
            let _guard = self.container_operations.lock(agent.id).await;
            return self.start_container(agent).await;
        }
        if agent.kind == AgentKind::JavaAgent {
            return self.start_java_agent(agent).await;
        }
        if self.children.lock().await.contains_key(&agent.id) {
            let updated = self
                .repo
                .update_runtime_state(
                    agent.id,
                    RuntimeStatePatch {
                        status: AgentStatus::Running,
                        desired_state: DesiredState::Running,
                        pid: agent.runtime.pid,
                        health_status: Some("running".to_string()),
                        health_detail: Some("process is already tracked".to_string()),
                        last_capabilities_json: None,
                        startup_command_redacted: Some(self.command_preview(agent)),
                        started_at: parse_domain_ts(&agent.runtime.started_at),
                        stopped_at: None,
                    },
                )
                .await?;
            return Ok(RuntimeOperationResponse {
                agent_id: updated.id,
                status: updated.status,
                message: "agent is already running".to_string(),
            });
        }

        let starting = self
            .repo
            .update_runtime_state(
                agent.id,
                RuntimeStatePatch {
                    status: AgentStatus::Starting,
                    desired_state: DesiredState::Running,
                    pid: None,
                    health_status: Some("starting".to_string()),
                    health_detail: Some("launch requested".to_string()),
                    last_capabilities_json: None,
                    startup_command_redacted: Some(self.command_preview(agent)),
                    started_at: None,
                    stopped_at: None,
                },
            )
            .await?;
        let _ = self
            .repo
            .insert_log(starting.id, "system", "Hermes start requested")
            .await;

        let mut command = self.hermes_command(&starting)?;
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                let updated = self
                    .repo
                    .update_runtime_state(
                        starting.id,
                        RuntimeStatePatch {
                            status: AgentStatus::Failed,
                            desired_state: DesiredState::Stopped,
                            pid: None,
                            health_status: Some("failed".to_string()),
                            health_detail: Some(format!("failed to spawn Hermes: {err}")),
                            last_capabilities_json: None,
                            startup_command_redacted: Some(self.command_preview(&starting)),
                            started_at: None,
                            stopped_at: Some(shared::now()),
                        },
                    )
                    .await?;
                let _ = self
                    .repo
                    .insert_log(
                        updated.id,
                        "stderr",
                        &format!("failed to spawn Hermes: {err}"),
                    )
                    .await;
                return Ok(RuntimeOperationResponse {
                    agent_id: updated.id,
                    status: updated.status,
                    message: "failed to spawn Hermes runtime".to_string(),
                });
            }
        };

        if let Some(stdout) = child.stdout.take() {
            self.spawn_log_reader(starting.id, "stdout", stdout).await;
        }
        if let Some(stderr) = child.stderr.take() {
            self.spawn_log_reader(starting.id, "stderr", stderr).await;
        }
        let pid = child.id().map(|id| id as i32);
        self.children.lock().await.insert(starting.id, child);

        match self.wait_for_hermes_ready(&starting).await {
            Ok(capabilities) => {
                let updated = self
                    .repo
                    .update_runtime_state(
                        starting.id,
                        RuntimeStatePatch {
                            status: AgentStatus::Running,
                            desired_state: DesiredState::Running,
                            pid,
                            health_status: Some("running".to_string()),
                            health_detail: Some("Hermes API is ready".to_string()),
                            last_capabilities_json: Some(capabilities),
                            startup_command_redacted: Some(self.command_preview(&starting)),
                            started_at: Some(shared::now()),
                            stopped_at: None,
                        },
                    )
                    .await?;
                Ok(RuntimeOperationResponse {
                    agent_id: updated.id,
                    status: updated.status,
                    message: "Hermes runtime started and API is ready".to_string(),
                })
            }
            Err(err) => {
                if let Some(mut child) = self.children.lock().await.remove(&starting.id) {
                    let _ = child.kill().await;
                }
                let detail = crate::redact_text(&err.to_string());
                let updated = self
                    .repo
                    .update_runtime_state(
                        starting.id,
                        RuntimeStatePatch {
                            status: AgentStatus::Failed,
                            desired_state: DesiredState::Stopped,
                            pid: None,
                            health_status: Some("failed".to_string()),
                            health_detail: Some(detail.clone()),
                            last_capabilities_json: None,
                            startup_command_redacted: Some(self.command_preview(&starting)),
                            started_at: None,
                            stopped_at: Some(shared::now()),
                        },
                    )
                    .await?;
                let _ = self.repo.insert_log(updated.id, "stderr", &detail).await;
                Ok(RuntimeOperationResponse {
                    agent_id: updated.id,
                    status: updated.status,
                    message: detail,
                })
            }
        }
    }

    async fn stop(&self, agent: &Agent) -> Result<RuntimeOperationResponse, AppError> {
        if self.container_mode(agent).await? {
            let _guard = self.container_operations.lock(agent.id).await;
            return self.stop_container(agent).await;
        }
        let mut children = self.children.lock().await;
        if let Some(mut child) = children.remove(&agent.id) {
            let _ = child.kill().await;
        }
        drop(children);
        let updated = self
            .repo
            .update_runtime_state(
                agent.id,
                RuntimeStatePatch {
                    status: AgentStatus::Stopped,
                    desired_state: DesiredState::Stopped,
                    pid: None,
                    health_status: Some("stopped".to_string()),
                    health_detail: Some("runtime stopped by Fleet Control".to_string()),
                    last_capabilities_json: None,
                    startup_command_redacted: Some(self.command_preview(agent)),
                    started_at: parse_domain_ts(&agent.runtime.started_at),
                    stopped_at: Some(shared::now()),
                },
            )
            .await?;
        let _ = self
            .repo
            .insert_log(updated.id, "system", "runtime stopped")
            .await;
        Ok(RuntimeOperationResponse {
            agent_id: updated.id,
            status: updated.status,
            message: "runtime stopped".to_string(),
        })
    }

    async fn restart(&self, agent: &Agent) -> Result<RuntimeOperationResponse, AppError> {
        self.stop(agent).await?;
        let refreshed = self.repo.get_agent(agent.id).await?;
        self.start(&refreshed).await
    }

    async fn health(&self, agent: &Agent) -> Result<RuntimeOperationResponse, AppError> {
        if self.container_mode(agent).await? {
            let _guard = self.container_operations.lock(agent.id).await;
            return self.health_container(agent).await;
        }
        let mut children = self.children.lock().await;
        let finished = match children.get_mut(&agent.id) {
            Some(child) => child.try_wait().map_err(AppError::internal)?,
            None => None,
        };
        if finished.is_some() {
            children.remove(&agent.id);
        }
        let tracked = children.contains_key(&agent.id);
        drop(children);

        if let Some(exit) = finished {
            let detail = format!("process exited with {exit}");
            let updated = self
                .repo
                .update_runtime_state(
                    agent.id,
                    RuntimeStatePatch {
                        status: AgentStatus::Failed,
                        desired_state: DesiredState::Stopped,
                        pid: None,
                        health_status: Some("exited".to_string()),
                        health_detail: Some(detail.clone()),
                        last_capabilities_json: None,
                        startup_command_redacted: Some(self.command_preview(agent)),
                        started_at: parse_domain_ts(&agent.runtime.started_at),
                        stopped_at: Some(shared::now()),
                    },
                )
                .await?;
            return Ok(RuntimeOperationResponse {
                agent_id: updated.id,
                status: updated.status,
                message: detail,
            });
        }

        if agent.kind == AgentKind::JavaAgent {
            return match self.probe_java_agent(agent).await {
                Ok(health) => {
                    let (status, detail) = if tracked {
                        (
                            AgentStatus::Running,
                            "java agent actuator is UP".to_string(),
                        )
                    } else {
                        (
                            AgentStatus::Degraded,
                            "java agent is UP but process is not tracked by this Fleet Control instance"
                                .to_string(),
                        )
                    };
                    let updated = self
                        .repo
                        .update_runtime_state(
                            agent.id,
                            RuntimeStatePatch {
                                status,
                                desired_state: DesiredState::Running,
                                pid: if tracked { agent.runtime.pid } else { None },
                                health_status: Some("healthy".to_string()),
                                health_detail: Some(detail.clone()),
                                last_capabilities_json: Some(health),
                                startup_command_redacted: Some(self.command_preview(agent)),
                                started_at: parse_domain_ts(&agent.runtime.started_at),
                                stopped_at: parse_domain_ts(&agent.runtime.stopped_at),
                            },
                        )
                        .await?;
                    Ok(RuntimeOperationResponse {
                        agent_id: updated.id,
                        status: updated.status,
                        message: detail,
                    })
                }
                Err(err) => {
                    let detail = crate::redact_text(&err.to_string());
                    let updated = self
                        .repo
                        .update_runtime_state(
                            agent.id,
                            RuntimeStatePatch {
                                status: if tracked {
                                    AgentStatus::Degraded
                                } else {
                                    AgentStatus::Stopped
                                },
                                desired_state: agent.runtime.desired_state,
                                pid: agent.runtime.pid,
                                health_status: Some("unhealthy".to_string()),
                                health_detail: Some(detail.clone()),
                                last_capabilities_json: None,
                                startup_command_redacted: Some(self.command_preview(agent)),
                                started_at: parse_domain_ts(&agent.runtime.started_at),
                                stopped_at: parse_domain_ts(&agent.runtime.stopped_at),
                            },
                        )
                        .await?;
                    Ok(RuntimeOperationResponse {
                        agent_id: updated.id,
                        status: updated.status,
                        message: detail,
                    })
                }
            };
        }

        match self.probe_hermes(agent).await {
            Ok(capabilities) => {
                let (status, detail) = if tracked {
                    (AgentStatus::Running, "Hermes API is healthy".to_string())
                } else {
                    (
                        AgentStatus::Degraded,
                        "Hermes API is healthy but process is not tracked by this Fleet Control instance"
                            .to_string(),
                    )
                };
                let updated = self
                    .repo
                    .update_runtime_state(
                        agent.id,
                        RuntimeStatePatch {
                            status,
                            desired_state: DesiredState::Running,
                            pid: if tracked { agent.runtime.pid } else { None },
                            health_status: Some("healthy".to_string()),
                            health_detail: Some(detail.clone()),
                            last_capabilities_json: Some(capabilities),
                            startup_command_redacted: Some(self.command_preview(agent)),
                            started_at: parse_domain_ts(&agent.runtime.started_at),
                            stopped_at: parse_domain_ts(&agent.runtime.stopped_at),
                        },
                    )
                    .await?;
                Ok(RuntimeOperationResponse {
                    agent_id: updated.id,
                    status: updated.status,
                    message: detail,
                })
            }
            Err(err) => {
                let detail = crate::redact_text(&err.to_string());
                // Preserve desired_state, but allow the reconciler to restart
                // an absent process after Fleet itself was restarted.
                let status = if tracked {
                    AgentStatus::Degraded
                } else {
                    AgentStatus::Stopped
                };
                let desired_state = if tracked {
                    DesiredState::Running
                } else {
                    agent.runtime.desired_state
                };
                let updated = self
                    .repo
                    .update_runtime_state(
                        agent.id,
                        RuntimeStatePatch {
                            status,
                            desired_state,
                            pid: if tracked { agent.runtime.pid } else { None },
                            health_status: Some("unhealthy".to_string()),
                            health_detail: Some(detail.clone()),
                            last_capabilities_json: None,
                            startup_command_redacted: Some(self.command_preview(agent)),
                            started_at: parse_domain_ts(&agent.runtime.started_at),
                            stopped_at: parse_domain_ts(&agent.runtime.stopped_at),
                        },
                    )
                    .await?;
                Ok(RuntimeOperationResponse {
                    agent_id: updated.id,
                    status: updated.status,
                    message: detail,
                })
            }
        }
    }

    async fn send_message(
        &self,
        agent: &Agent,
        session: &AgentSession,
        message: &SessionMessage,
    ) -> Result<RuntimeOperationResponse, AppError> {
        if agent.kind == AgentKind::JavaAgent {
            return Err(AppError::validation(
                "Java Agent runtime chat is planned for phase 2",
            ));
        }
        if !matches!(
            message.message_kind,
            MessageKind::UserPrompt | MessageKind::Control
        ) {
            return Ok(RuntimeOperationResponse {
                agent_id: agent.id,
                status: agent.status,
                message: "message mirrored without runtime dispatch".to_string(),
            });
        }

        match self.start_hermes_run(agent, session, message).await {
            Ok((run, runtime_run_id)) => {
                let _ = self
                    .repo
                    .insert_log(
                        agent.id,
                        "system",
                        &format!(
                            "Hermes /v1/runs dispatch accepted for Fleet session {} as {}",
                            session.id, runtime_run_id
                        ),
                    )
                    .await;
                Ok(RuntimeOperationResponse {
                    agent_id: agent.id,
                    status: AgentStatus::Running,
                    message: if run.state == SessionRunState::Pending {
                        "Hermes accepted the run; session readback is pending"
                    } else {
                        "Hermes run started"
                    }
                    .to_string(),
                })
            }
            Err(err) => {
                let detail = crate::redact_text(&err.to_string());
                self.repo
                    .update_session_message_delivery(
                        message.id,
                        MessageDeliveryState::Failed,
                        None,
                        Some(detail.clone()),
                    )
                    .await?;
                let _ = self
                    .repo
                    .insert_event(
                        Some(agent.id),
                        "runtime_message_dispatch_failed",
                        &detail,
                        json!({ "session_id": session.id, "message_id": message.id }),
                    )
                    .await;
                Ok(RuntimeOperationResponse {
                    agent_id: agent.id,
                    status: AgentStatus::Failed,
                    message: detail,
                })
            }
        }
    }

    async fn steer_run(
        &self,
        agent: &Agent,
        run: &SessionAgentRun,
        req: SteerSessionRunRequest,
        actor: domain::RuntimeControlActor,
    ) -> Result<RuntimeRunControlResponse, AppError> {
        if agent.kind == AgentKind::JavaAgent {
            return Err(AppError::validation(
                "Java Agent runtime steer is planned for phase 2",
            ));
        }
        if req.input.trim().is_empty() {
            return Err(AppError::validation("steer input is required"));
        }
        let command = run_control::send(
            self,
            agent,
            run,
            run_control::Operation::Steer,
            Some(&HermesSteerRequest {
                input: req.input.trim().to_string(),
            }),
            &actor,
            Some(req.input.trim()),
        )
        .await?;
        // Guidance acknowledgement cannot regress a concurrent waiting/stopping/terminal run.
        let updated = self.repo.get_session_agent_run(run.id).await?;
        Ok(RuntimeRunControlResponse {
            session_id: run.session_id,
            run_id: run.id,
            runtime_run_id: run.runtime_run_id.clone(),
            accepted: command.state == domain::RuntimeControlState::Acknowledged,
            state: updated.state,
            message: control_message(&command).to_string(),
            command: Some(command),
        })
    }

    async fn stop_run(
        &self,
        agent: &Agent,
        run: &SessionAgentRun,
        actor: domain::RuntimeControlActor,
    ) -> Result<RuntimeRunControlResponse, AppError> {
        if agent.kind == AgentKind::JavaAgent {
            return Err(AppError::validation(
                "Java Agent runtime stop is planned for phase 2",
            ));
        }
        let command = run_control::send::<Value>(
            self,
            agent,
            run,
            run_control::Operation::Stop,
            None,
            &actor,
            None,
        )
        .await?;
        let updated = self.repo.get_session_agent_run(run.id).await?;
        self.emit_run(&updated);
        Ok(RuntimeRunControlResponse {
            session_id: run.session_id,
            run_id: run.id,
            runtime_run_id: run.runtime_run_id.clone(),
            accepted: command.state == domain::RuntimeControlState::Acknowledged,
            state: updated.state,
            message: control_message(&command).to_string(),
            command: Some(command),
        })
    }

    async fn resolve_approval(
        &self,
        _agent: &Agent,
        _run: &SessionAgentRun,
        _req: ResolveRuntimeApprovalRequest,
    ) -> Result<RuntimeRunControlResponse, AppError> {
        Err(AppError::conflict(
            "use the exact approval request decision endpoint",
        ))
    }

    fn command_preview(&self, agent: &Agent) -> String {
        match agent.kind {
            AgentKind::Hermes => format!(
                "{} serve --host 127.0.0.1 --port {}",
                self.config.fleet.hermes_command,
                agent.api_port.unwrap_or_default()
            ),
            AgentKind::JavaAgent => format!(
                "{} -jar runtime/backend.jar --server.port={}",
                self.config.fleet.java_agent_command,
                agent.api_port.unwrap_or_default()
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn agent(id: Uuid, product_role: AgentProductRole) -> Agent {
        Agent {
            id,
            ordinal: 1,
            name: "agent1".to_string(),
            kind: AgentKind::Hermes,
            product_role,
            role: domain::AgentRole::Developer,
            sdlc_role: Some(domain::SdlcRole::Developer),
            status: AgentStatus::Running,
            display_name: "Agent".to_string(),
            description: None,
            namespace_id: None,
            workflow_id: None,
            runtime_version: None,
            dashboard_port: Some(29002),
            api_port: Some(29001),
            paths: domain::AgentPaths {
                runtime: "agents/agent1/runtime".to_string(),
                config: "agents/agent1/config".to_string(),
                workspace: "agents/agent1/workspace".to_string(),
                logs: "agents/agent1/logs".to_string(),
            },
            runtime: domain::AgentRuntime {
                desired_state: DesiredState::Running,
                pid: None,
                health_status: Some("healthy".to_string()),
                health_detail: None,
                command_preview: "hermes serve".to_string(),
                env_preview: json!({}),
                last_capabilities_json: json!({}),
                startup_command_redacted: None,
                started_at: None,
                stopped_at: None,
                last_health_at: None,
            },
            created_at: "2026-09-01T00:00:00Z".to_string(),
            updated_at: "2026-09-01T00:00:00Z".to_string(),
        }
    }

    fn session(primary_agent_id: Uuid, leader_agent_id: Option<Uuid>) -> AgentSession {
        AgentSession {
            id: Uuid::new_v4(),
            agent_id: primary_agent_id,
            primary_agent_id,
            agent_name: "agent1".to_string(),
            primary_agent_name: "agent1".to_string(),
            user_id: Uuid::new_v4(),
            user_email: "user@example.com".to_string(),
            user_username: "user".to_string(),
            user_display_name: "User".to_string(),
            leader_agent_id,
            leader_agent_name: None,
            parent_session_id: None,
            created_by_leader_agent_id: leader_agent_id,
            visibility: if leader_agent_id.is_some() {
                domain::SessionVisibility::LeaderScoped
            } else {
                domain::SessionVisibility::Private
            },
            title: "Task".to_string(),
            task_key: Some("CARD-1".to_string()),
            state: domain::SessionState::Active,
            namespace_id: None,
            external_session_id: None,
            last_message_preview: None,
            pending_delivery: None,
            created_at: "2026-09-01T00:00:00Z".to_string(),
            updated_at: "2026-09-01T00:00:00Z".to_string(),
        }
    }

    fn message(author_agent_id: Option<Uuid>) -> SessionMessage {
        SessionMessage {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            author_type: if author_agent_id.is_some() {
                MessageAuthorType::Agent
            } else {
                MessageAuthorType::User
            },
            author_user_id: None,
            author_agent_id,
            author_display_name: "Author".to_string(),
            body: "please test this".to_string(),
            message_kind: MessageKind::UserPrompt,
            runtime_message_id: None,
            delivery_state: MessageDeliveryState::Pending,
            delivery_error: None,
            replayed: false,
            request_payload_hash: None,
            created_at: "2026-09-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn leader_authored_executor_message_gets_fleet_envelope() {
        let executor_id = Uuid::new_v4();
        let leader_id = Uuid::new_v4();
        let executor = agent(executor_id, AgentProductRole::Executor);
        let session = session(executor_id, Some(leader_id));
        let message = message(Some(leader_id));

        let input = LocalRuntimeSupervisor::runtime_input(&executor, &session, &message);

        assert!(input.contains("[Fleet Control]"));
        assert!(input.contains("Message from leader agent"));
        assert!(input.contains("please test this"));
    }

    #[test]
    fn primary_leader_run_is_classified_as_leader() {
        let leader_id = Uuid::new_v4();
        let leader = agent(leader_id, AgentProductRole::Leader);
        let session = session(leader_id, Some(leader_id));

        assert_eq!(
            LocalRuntimeSupervisor::run_role(&session, &leader),
            SessionRunRole::Leader
        );
    }

    #[test]
    fn pick_error_redacts_secret_like_payloads() {
        let payload = json!({ "error": { "message": "api_key=super-secret" } });

        assert_eq!(pick_error(&payload).as_deref(), Some("api_key=redacted"));
    }

    #[test]
    fn hermes_completion_reads_output_instead_of_event_name() {
        let payload =
            json!({"event": "run.completed", "run_id": "run-1", "output": "FLEET_HERMES_OK"});
        assert_eq!(
            pick_string(&payload, &["final_response", "output", "text"]).as_deref(),
            Some("FLEET_HERMES_OK")
        );
    }

    #[test]
    fn missing_text_does_not_become_a_runtime_identifier() {
        let payload =
            json!({"event": "run.completed", "run_id": "run-1", "usage": {"model": "model-1"}});
        assert_eq!(pick_string(&payload, &["output", "text"]), None);
    }

    #[test]
    fn nested_delta_ignores_unrelated_event_fields() {
        let payload =
            json!({"event": "message.delta", "data": {"delta": "reply"}, "run_id": "run-1"});
        assert_eq!(
            pick_string(&payload, &["delta", "text"]).as_deref(),
            Some("reply")
        );
    }
}
