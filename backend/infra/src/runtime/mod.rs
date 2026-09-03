use app::{FleetRepository, RuntimeApprovalCreate, RuntimeStatePatch, RuntimeSupervisor};
use async_trait::async_trait;
use domain::{
    Agent, AgentKind, AgentProductRole, AgentSession, AgentStatus, DesiredState, MessageAuthorType,
    MessageDeliveryState, MessageKind, ResolveRuntimeApprovalRequest, RuntimeOperationResponse,
    RuntimeRunControlResponse, SessionAgentRun, SessionMessage, SessionRunRole, SessionRunState,
    SteerSessionRunRequest,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
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

const HERMES_READY_TIMEOUT: Duration = Duration::from_secs(30);
const HERMES_READY_POLL: Duration = Duration::from_millis(500);
const RECONCILE_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct LocalRuntimeSupervisor {
    config: Arc<AppConfig>,
    repo: Arc<dyn FleetRepository>,
    children: Arc<Mutex<HashMap<Uuid, Child>>>,
    client: reqwest::Client,
    events: broadcast::Sender<FleetEvent>,
}

#[derive(Debug, Serialize)]
struct HermesRunStartRequest {
    input: String,
    session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_options: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct HermesRunStartResponse {
    run_id: String,
}

#[derive(Debug, Serialize)]
struct HermesSteerRequest {
    input: String,
}

#[derive(Debug, Serialize)]
struct HermesApprovalRequest {
    choice: String,
    resolve_all: bool,
}

impl LocalRuntimeSupervisor {
    pub fn new(
        config: Arc<AppConfig>,
        repo: Arc<dyn FleetRepository>,
        events: broadcast::Sender<FleetEvent>,
    ) -> Self {
        let supervisor = Self {
            config,
            repo,
            children: Arc::new(Mutex::new(HashMap::new())),
            client: reqwest::Client::new(),
            events,
        };
        supervisor.spawn_reconciler();
        supervisor
    }

    fn spawn_reconciler(&self) {
        let supervisor = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                loop {
                    sleep(RECONCILE_INTERVAL).await;
                    let Ok(agents) = supervisor.repo.list_agents().await else {
                        continue;
                    };
                    for agent in agents {
                        if matches!(
                            agent.status,
                            AgentStatus::Running | AgentStatus::Starting | AgentStatus::Degraded
                        ) || agent.runtime.desired_state == DesiredState::Running
                        {
                            let _ = supervisor.health(&agent).await;
                        }
                    }
                }
            });
        }
    }

    fn hermes_command(&self, agent: &Agent) -> Result<Command, AppError> {
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        let mut command = Command::new(&self.config.fleet.hermes_command);
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

    fn hermes_base_url(agent: &Agent) -> Result<String, AppError> {
        let port = agent
            .api_port
            .ok_or_else(|| AppError::validation("agent api_port is required"))?;
        Ok(format!("http://127.0.0.1:{port}"))
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
        let base = Self::hermes_base_url(agent)?;
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        let health = self
            .client
            .get(format!("{base}/health"))
            .bearer_auth(&token)
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
        let capabilities: Value = capabilities.json().await.map_err(AppError::internal)?;
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
        let runtime_session_id = Self::runtime_session_id(session, agent);
        let run = self
            .repo
            .prepare_session_agent_run(
                session.id,
                agent.id,
                Self::run_role(session, agent),
                runtime_session_id.clone(),
            )
            .await?;
        let base = Self::hermes_base_url(agent)?;
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        let model_options = (!matches!(&run.model_options, Value::Object(map) if map.is_empty()))
            .then_some(run.model_options.clone());
        let response = self
            .client
            .post(format!("{base}/v1/runs"))
            .bearer_auth(token)
            .header("Idempotency-Key", message.id.to_string())
            .json(&HermesRunStartRequest {
                input: Self::runtime_input(agent, session, message),
                session_id: runtime_session_id,
                model: run.model.clone(),
                provider: run.provider.clone(),
                model_options,
            })
            .send()
            .await
            .map_err(AppError::internal)?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::validation(format!(
                "Hermes /v1/runs rejected dispatch with {status}: {}",
                crate::redact_text(&body)
            )));
        }
        let accepted: HermesRunStartResponse = response.json().await.map_err(AppError::internal)?;
        let run = self
            .repo
            .update_session_agent_run_dispatch(
                run.id,
                Some(accepted.run_id.clone()),
                SessionRunState::Running,
                None,
            )
            .await?;
        self.repo
            .update_session_message_delivery(
                message.id,
                MessageDeliveryState::Dispatched,
                Some(accepted.run_id.clone()),
                None,
            )
            .await?;
        self.emit_run(&run);
        let _ = self.events.send(FleetEvent::SessionMessageChanged {
            session_id: session.id.to_string(),
            message_id: message.id.to_string(),
            event: "message.dispatched".to_string(),
        });
        Ok((run, accepted.run_id))
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
                    message.clone(),
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
                        MessageDeliveryState::Failed,
                        Some(runtime_run_id.clone()),
                        Some(redacted.clone()),
                    )
                    .await;
                if let Ok(updated) = supervisor
                    .repo
                    .update_session_agent_run_dispatch(
                        run.id,
                        Some(runtime_run_id.clone()),
                        SessionRunState::Failed,
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
        message: SessionMessage,
        run: SessionAgentRun,
        runtime_run_id: String,
    ) -> Result<(), AppError> {
        let base = Self::hermes_base_url(&agent)?;
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        let response = self
            .client
            .get(format!("{base}/v1/runs/{runtime_run_id}/events"))
            .bearer_auth(token)
            .send()
            .await
            .map_err(AppError::internal)?;
        if !response.status().is_success() {
            return Err(AppError::validation(format!(
                "Hermes run events returned {}",
                response.status()
            )));
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut event_name: Option<String> = None;
        let mut data_lines: Vec<String> = Vec::new();
        let mut final_text = String::new();
        let mut terminal_seen = false;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(AppError::internal)?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(pos) = buffer.find('\n') {
                let mut line = buffer[..pos].to_string();
                if line.ends_with('\r') {
                    line.pop();
                }
                buffer = buffer[pos + 1..].to_string();
                if line.is_empty() {
                    if !data_lines.is_empty() {
                        terminal_seen |= self
                            .handle_hermes_event(
                                &agent,
                                &session,
                                &message,
                                &run,
                                &runtime_run_id,
                                event_name.take(),
                                data_lines.join("\n"),
                                &mut final_text,
                            )
                            .await?;
                        data_lines.clear();
                    }
                } else if let Some(name) = line.strip_prefix("event:") {
                    event_name = Some(name.trim().to_string());
                } else if let Some(data) = line.strip_prefix("data:") {
                    data_lines.push(data.trim_start().to_string());
                }
            }
        }

        if !data_lines.is_empty() {
            terminal_seen |= self
                .handle_hermes_event(
                    &agent,
                    &session,
                    &message,
                    &run,
                    &runtime_run_id,
                    event_name,
                    data_lines.join("\n"),
                    &mut final_text,
                )
                .await?;
        }

        if !terminal_seen {
            let body = if final_text.trim().is_empty() {
                "Hermes run completed without a final response payload".to_string()
            } else {
                final_text
            };
            self.persist_assistant_completion(
                &agent,
                &session,
                &message,
                &run,
                &runtime_run_id,
                body,
            )
            .await?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_hermes_event(
        &self,
        agent: &Agent,
        session: &AgentSession,
        message: &SessionMessage,
        run: &SessionAgentRun,
        runtime_run_id: &str,
        event_name: Option<String>,
        data: String,
        final_text: &mut String,
    ) -> Result<bool, AppError> {
        let payload = serde_json::from_str::<Value>(&data).unwrap_or(Value::String(data));
        let event_type = event_name
            .or_else(|| pick_string(&payload, &["event", "type", "object"]))
            .unwrap_or_else(|| "message".to_string());

        if event_type.contains("delta") {
            if let Some(delta) = pick_string(&payload, &["delta", "text", "output_text"]) {
                final_text.push_str(&delta);
                let _ = self.events.send(FleetEvent::SessionRunDelta {
                    session_id: session.id.to_string(),
                    run_id: run.id.to_string(),
                    runtime_run_id: Some(runtime_run_id.to_string()),
                    delta,
                });
            }
            return Ok(false);
        }

        if event_type.contains("approval") {
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

        if event_type.contains("cancel") || event_type.contains("stopped") {
            let updated = self
                .repo
                .update_session_agent_run_dispatch(
                    run.id,
                    Some(runtime_run_id.to_string()),
                    SessionRunState::Cancelled,
                    None,
                )
                .await?;
            self.emit_run(&updated);
            return Ok(true);
        }

        if event_type.contains("fail") || event_type.contains("error") {
            let error =
                pick_error(&payload).unwrap_or_else(|| format!("Hermes event {event_type}"));
            self.repo
                .update_session_message_delivery(
                    message.id,
                    MessageDeliveryState::Failed,
                    Some(runtime_run_id.to_string()),
                    Some(error.clone()),
                )
                .await?;
            let updated = self
                .repo
                .update_session_agent_run_dispatch(
                    run.id,
                    Some(runtime_run_id.to_string()),
                    SessionRunState::Failed,
                    Some(error),
                )
                .await?;
            self.emit_run(&updated);
            return Ok(true);
        }

        if event_type.contains("completed") || event_type.contains("done") {
            let body = pick_string(
                &payload,
                &[
                    "final_response",
                    "output_text",
                    "response",
                    "message",
                    "text",
                ],
            )
            .unwrap_or_else(|| {
                if final_text.trim().is_empty() {
                    "Hermes run completed".to_string()
                } else {
                    final_text.clone()
                }
            });
            self.persist_assistant_completion(agent, session, message, run, runtime_run_id, body)
                .await?;
            return Ok(true);
        }

        Ok(false)
    }

    async fn persist_assistant_completion(
        &self,
        agent: &Agent,
        session: &AgentSession,
        message: &SessionMessage,
        run: &SessionAgentRun,
        runtime_run_id: &str,
        body: String,
    ) -> Result<(), AppError> {
        let assistant = self
            .repo
            .insert_session_message_mirror(
                session.id,
                Some(agent.id),
                body,
                MessageKind::AssistantMessage,
                Some(runtime_run_id.to_string()),
            )
            .await?;
        self.repo
            .update_session_message_delivery(
                message.id,
                MessageDeliveryState::Completed,
                Some(runtime_run_id.to_string()),
                None,
            )
            .await?;
        let updated = self
            .repo
            .update_session_agent_run_dispatch(
                run.id,
                Some(runtime_run_id.to_string()),
                SessionRunState::Completed,
                None,
            )
            .await?;
        self.emit_run(&updated);
        let _ = self.events.send(FleetEvent::SessionMessageChanged {
            session_id: session.id.to_string(),
            message_id: assistant.id.to_string(),
            event: "message.completed".to_string(),
        });
        Ok(())
    }

    fn emit_run(&self, run: &SessionAgentRun) {
        let _ = self.events.send(FleetEvent::SessionRunChanged {
            session_id: run.session_id.to_string(),
            run_id: run.id.to_string(),
            runtime_run_id: run.runtime_run_id.clone(),
            state: run.state.as_str().to_string(),
        });
    }

    async fn post_run_control<T: Serialize + ?Sized>(
        &self,
        agent: &Agent,
        run: &SessionAgentRun,
        path: &str,
        body: Option<&T>,
    ) -> Result<Value, AppError> {
        let runtime_run_id = run
            .runtime_run_id
            .as_ref()
            .ok_or_else(|| AppError::validation("session run has no runtime_run_id"))?;
        let base = Self::hermes_base_url(agent)?;
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        let request = self
            .client
            .post(format!("{base}/v1/runs/{runtime_run_id}/{path}"))
            .bearer_auth(token);
        let request = match body {
            Some(body) => request.json(body),
            None => request,
        };
        let response = request.send().await.map_err(AppError::internal)?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::validation(format!(
                "Hermes run control {path} failed with {status}: {}",
                crate::redact_text(&body)
            )));
        }
        response.json().await.map_err(AppError::internal)
    }
}

fn parse_domain_ts(value: &Option<domain::Timestamp>) -> Option<shared::Timestamp> {
    value
        .as_deref()
        .and_then(|timestamp| chrono::DateTime::parse_from_rfc3339(timestamp).ok())
}

fn pick_string(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(found) = value.get(*key).and_then(Value::as_str) {
            return Some(found.to_string());
        }
    }
    match value {
        Value::Object(map) => map.values().find_map(|value| pick_string(value, keys)),
        Value::Array(items) => items.iter().find_map(|value| pick_string(value, keys)),
        Value::String(text) => Some(text.clone()),
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
            let detail = "Java Agent health endpoint is reserved for phase 2".to_string();
            let updated = self
                .repo
                .update_runtime_state(
                    agent.id,
                    RuntimeStatePatch {
                        status: agent.status,
                        desired_state: agent.runtime.desired_state,
                        pid: agent.runtime.pid,
                        health_status: Some("planned".to_string()),
                        health_detail: Some(detail.clone()),
                        last_capabilities_json: None,
                        startup_command_redacted: Some(self.command_preview(agent)),
                        started_at: parse_domain_ts(&agent.runtime.started_at),
                        stopped_at: parse_domain_ts(&agent.runtime.stopped_at),
                    },
                )
                .await?;
            return Ok(RuntimeOperationResponse {
                agent_id: updated.id,
                status: updated.status,
                message: detail,
            });
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
                let status = if tracked || agent.runtime.desired_state == DesiredState::Running {
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
                self.spawn_hermes_event_worker(
                    agent.clone(),
                    session.clone(),
                    message.clone(),
                    run,
                    runtime_run_id.clone(),
                );
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
                    message: "Hermes run started".to_string(),
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
    ) -> Result<RuntimeRunControlResponse, AppError> {
        if agent.kind == AgentKind::JavaAgent {
            return Err(AppError::validation(
                "Java Agent runtime steer is planned for phase 2",
            ));
        }
        if req.input.trim().is_empty() {
            return Err(AppError::validation("steer input is required"));
        }
        self.post_run_control(
            agent,
            run,
            "steer",
            Some(&HermesSteerRequest {
                input: req.input.trim().to_string(),
            }),
        )
        .await?;
        let updated = self
            .repo
            .update_session_agent_run_dispatch(
                run.id,
                run.runtime_run_id.clone(),
                SessionRunState::Running,
                None,
            )
            .await?;
        self.emit_run(&updated);
        Ok(RuntimeRunControlResponse {
            session_id: run.session_id,
            run_id: run.id,
            runtime_run_id: run.runtime_run_id.clone(),
            accepted: true,
            state: updated.state,
            message: "Hermes run steer accepted".to_string(),
        })
    }

    async fn stop_run(
        &self,
        agent: &Agent,
        run: &SessionAgentRun,
    ) -> Result<RuntimeRunControlResponse, AppError> {
        if agent.kind == AgentKind::JavaAgent {
            return Err(AppError::validation(
                "Java Agent runtime stop is planned for phase 2",
            ));
        }
        self.post_run_control::<Value>(agent, run, "stop", None)
            .await?;
        let updated = self
            .repo
            .update_session_agent_run_dispatch(
                run.id,
                run.runtime_run_id.clone(),
                SessionRunState::Stopping,
                None,
            )
            .await?;
        self.emit_run(&updated);
        Ok(RuntimeRunControlResponse {
            session_id: run.session_id,
            run_id: run.id,
            runtime_run_id: run.runtime_run_id.clone(),
            accepted: true,
            state: updated.state,
            message: "Hermes run stop accepted".to_string(),
        })
    }

    async fn resolve_approval(
        &self,
        agent: &Agent,
        run: &SessionAgentRun,
        req: ResolveRuntimeApprovalRequest,
    ) -> Result<RuntimeRunControlResponse, AppError> {
        if agent.kind == AgentKind::JavaAgent {
            return Err(AppError::validation(
                "Java Agent runtime approval is planned for phase 2",
            ));
        }
        self.post_run_control(
            agent,
            run,
            "approval",
            Some(&HermesApprovalRequest {
                choice: req.choice,
                resolve_all: req.resolve_all,
            }),
        )
        .await?;
        let updated = self
            .repo
            .update_session_agent_run_dispatch(
                run.id,
                run.runtime_run_id.clone(),
                SessionRunState::Running,
                None,
            )
            .await?;
        self.emit_run(&updated);
        Ok(RuntimeRunControlResponse {
            session_id: run.session_id,
            run_id: run.id,
            runtime_run_id: run.runtime_run_id.clone(),
            accepted: true,
            state: updated.state,
            message: "Hermes run approval accepted".to_string(),
        })
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

    fn agent(id: Uuid, product_role: AgentProductRole) -> Agent {
        Agent {
            id,
            ordinal: 1,
            name: "agent1".to_string(),
            kind: AgentKind::Hermes,
            product_role,
            role: domain::AgentRole::Developer,
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
}
