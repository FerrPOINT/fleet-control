use app::{FleetRepository, RuntimeStatePatch, RuntimeSupervisor};
use async_trait::async_trait;
use domain::{Agent, AgentKind, AgentStatus, DesiredState, RuntimeOperationResponse};
use shared::{AppConfig, AppError};
use std::{collections::HashMap, process::Stdio, sync::Arc};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::Mutex,
};
use uuid::Uuid;

#[derive(Clone)]
pub struct LocalRuntimeSupervisor {
    config: Arc<AppConfig>,
    repo: Arc<dyn FleetRepository>,
    children: Arc<Mutex<HashMap<Uuid, Child>>>,
}

impl LocalRuntimeSupervisor {
    pub fn new(config: Arc<AppConfig>, repo: Arc<dyn FleetRepository>) -> Self {
        Self {
            config,
            repo,
            children: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn hermes_command(&self, agent: &Agent) -> Command {
        let mut command = Command::new(&self.config.fleet.hermes_command);
        command
            .arg("dashboard")
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(agent.dashboard_port.unwrap_or_default().to_string())
            .env("HERMES_HOME", &agent.paths.config)
            .current_dir(&agent.paths.workspace)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        command
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
                        let _ = repo.insert_log(agent_id, stream, &line).await;
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
}

fn parse_domain_ts(value: &Option<domain::Timestamp>) -> Option<shared::Timestamp> {
    value
        .as_deref()
        .and_then(|timestamp| chrono::DateTime::parse_from_rfc3339(timestamp).ok())
}

#[async_trait]
impl RuntimeSupervisor for LocalRuntimeSupervisor {
    async fn start(&self, agent: &Agent) -> Result<RuntimeOperationResponse, AppError> {
        if agent.kind == AgentKind::JavaAgent {
            return Err(AppError::validation(
                "Java Agent runtime start is planned for phase 2",
            ));
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
                    started_at: None,
                    stopped_at: None,
                },
            )
            .await?;
        let _ = self
            .repo
            .insert_log(starting.id, "system", "Hermes start requested")
            .await;

        let mut child = match self.hermes_command(&starting).spawn() {
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
        let updated = self
            .repo
            .update_runtime_state(
                starting.id,
                RuntimeStatePatch {
                    status: AgentStatus::Running,
                    desired_state: DesiredState::Running,
                    pid,
                    health_status: Some("running".to_string()),
                    health_detail: Some("Hermes process started".to_string()),
                    started_at: Some(shared::now()),
                    stopped_at: None,
                },
            )
            .await?;
        Ok(RuntimeOperationResponse {
            agent_id: updated.id,
            status: updated.status,
            message: "Hermes runtime started".to_string(),
        })
    }

    async fn stop(&self, agent: &Agent) -> Result<RuntimeOperationResponse, AppError> {
        if agent.kind == AgentKind::JavaAgent {
            return Err(AppError::validation(
                "Java Agent runtime stop is planned for phase 2",
            ));
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
        let tracked = self.children.lock().await.contains_key(&agent.id);
        let (status, desired, health, detail) = if tracked {
            (
                AgentStatus::Running,
                DesiredState::Running,
                "running",
                "process is tracked in this Fleet Control instance",
            )
        } else if agent.kind == AgentKind::JavaAgent {
            (
                agent.status,
                agent.runtime.desired_state,
                "planned",
                "Java Agent health endpoint is reserved for phase 2",
            )
        } else {
            (
                AgentStatus::Stopped,
                DesiredState::Stopped,
                "stopped",
                "process is not tracked in this Fleet Control instance",
            )
        };
        let updated = self
            .repo
            .update_runtime_state(
                agent.id,
                RuntimeStatePatch {
                    status,
                    desired_state: desired,
                    pid: if tracked { agent.runtime.pid } else { None },
                    health_status: Some(health.to_string()),
                    health_detail: Some(detail.to_string()),
                    started_at: parse_domain_ts(&agent.runtime.started_at),
                    stopped_at: parse_domain_ts(&agent.runtime.stopped_at),
                },
            )
            .await?;
        Ok(RuntimeOperationResponse {
            agent_id: updated.id,
            status: updated.status,
            message: detail.to_string(),
        })
    }

    fn command_preview(&self, agent: &Agent) -> String {
        match agent.kind {
            AgentKind::Hermes => format!(
                "{} dashboard --host 127.0.0.1 --port {}",
                self.config.fleet.hermes_command,
                agent.dashboard_port.unwrap_or_default()
            ),
            AgentKind::JavaAgent => format!(
                "{} -jar runtime/backend.jar --server.port={}",
                self.config.fleet.java_agent_command,
                agent.api_port.unwrap_or_default()
            ),
        }
    }
}
