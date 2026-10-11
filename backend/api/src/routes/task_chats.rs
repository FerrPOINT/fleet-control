use crate::middleware::{CurrentUser, VerifiedCentralSubject};
use app::AppContext;
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
};
use domain::{
    BindTaskChatRequest, ClarificationAnswerRequest, ConfirmRequirementsRequest,
    MessageHistoryPage, SessionTaskContext, TaskChatBinding,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use shared::AppError;
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

pub(super) struct TrackerGateway {
    url: reqwest::Url,
    instance: String,
}

#[utoipa::path(get,path="/api/v1/sessions/{session_id}/chat-controls",tag="task-chats",params(("session_id"=Uuid,Path)),responses((status=200,body=domain::ChatControls)))]
pub async fn controls(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<domain::ChatControls>, AppError> {
    require_project_access(&ctx, &user, id, &headers).await?;
    let session = ctx.repo.get_session(id).await?;
    super::sessions::ensure_session_read_access(&session, &user)?;
    let agent = ctx.repo.get_agent(session.primary_agent_id).await?;
    let runs = ctx.repo.list_session_agent_runs(id).await?;
    let active = runs.iter().rev().find(|run| {
        (run.state != domain::SessionRunState::Pending
            || run.runtime_session_id.is_some()
            || run.runtime_run_id.is_some()
            || run.last_event_at.is_some())
            && matches!(
                run.state,
                domain::SessionRunState::Pending
                    | domain::SessionRunState::Running
                    | domain::SessionRunState::Waiting
                    | domain::SessionRunState::Stopping
            )
    });
    let owner = session.user_id == user.id;
    let bound = ctx.repo.get_task_chat_binding(id).await?.is_some();
    let pending = ctx.repo.has_pending_session_dispatch(id).await?;
    let supported = agent.kind == domain::AgentKind::Hermes;
    let live = agent.status == domain::AgentStatus::Running;
    let features = agent.runtime.last_capabilities_json.get("features");
    let supports = |name: &str| {
        features
            .and_then(|value| value.get(name))
            .is_some_and(|value| {
                value.as_bool() == Some(true)
                    || value.get("supported").and_then(Value::as_bool) == Some(true)
            })
    };
    let tracked = active.is_some_and(|run| run.runtime_run_id.is_some());
    Ok(Json(domain::ChatControls {
        can_send: owner && supported && !bound && active.is_none() && !pending,
        can_steer: owner && supported && live && !bound && tracked && supports("run_steer"),
        can_stop: owner && supported && live && tracked && supports("run_stop"),
        active_run_id: active.map(|run| run.id),
        blocked_reason: if !owner {
            Some("read_only".into())
        } else if bound {
            Some("workflow_assignment_required".into())
        } else if !supported {
            Some("java_chat_phase_2".into())
        } else if pending || (active.is_some() && !tracked) {
            Some("dispatch_pending_or_uncertain".into())
        } else if !live {
            Some("runtime_stopped_messages_queue".into())
        } else {
            None
        },
    }))
}

impl TrackerGateway {
    pub(super) fn configured(config: &shared::config::TrackerConfig) -> Result<Self, AppError> {
        if config.url.is_empty() || config.instance_id.is_empty() {
            return Err(AppError::Unavailable(
                "Task Tracker integration is not configured".into(),
            ));
        }
        Self::parse(&config.url, config.instance_id.clone())
    }
    fn parse(raw: &str, instance: String) -> Result<Self, AppError> {
        let url =
            reqwest::Url::parse(raw).map_err(|_| AppError::validation("invalid Tracker URL"))?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path() != "/"
            || instance.trim().is_empty()
            || instance.len() > 128
        {
            return Err(AppError::validation(
                "Tracker requires a root HTTP URL and stable instance ID",
            ));
        }
        Ok(Self { url, instance })
    }
    async fn request(
        &self,
        task: Uuid,
        segments: &[String],
        headers: &HeaderMap,
        body: Option<Value>,
    ) -> Result<(StatusCode, Value), AppError> {
        let task = task.to_string();
        let mut path = vec!["api", "v1", "issues", task.as_str(), "sdlc"];
        path.extend(segments.iter().map(String::as_str));
        self.request_path(&path, None, headers, body).await
    }
    pub(super) async fn request_path(
        &self,
        segments: &[&str],
        query: Option<(&str, &str)>,
        headers: &HeaderMap,
        body: Option<Value>,
    ) -> Result<(StatusCode, Value), AppError> {
        let token = headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;
        let mut url = self.url.clone();
        url.path_segments_mut()
            .map_err(|_| AppError::validation("invalid Tracker base"))?
            .extend(segments.iter().copied());
        if let Some((name, value)) = query {
            url.query_pairs_mut().append_pair(name, value);
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .no_proxy()
            .build()
            .map_err(AppError::internal)?;
        let request = match body {
            Some(body) => client.post(url).json(&body),
            None => client.get(url),
        };
        let mut response = request.bearer_auth(token).send().await.map_err(|_| {
            AppError::Unavailable(
                "Task Tracker request outcome is unknown; reconcile before retry".into(),
            )
        })?;
        let status = response.status();
        if status.is_redirection() {
            return Err(AppError::Unavailable(
                "Tracker redirects are not allowed".into(),
            ));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| AppError::Unavailable("Tracker response was interrupted".into()))?
        {
            if bytes.len() + chunk.len() > 1_048_576 {
                return Err(AppError::Unavailable(
                    "Tracker response exceeds limit".into(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        let value = match serde_json::from_slice(&bytes) {
            Ok(value) => value,
            Err(_) if !status.is_success() => serde_json::json!({"error":{
                "code":"tracker_rejection", "message":format!("Tracker rejected the request ({})",status.as_u16())}}),
            Err(_) => {
                return Err(AppError::Unavailable(
                    "Tracker returned an invalid response".into(),
                ));
            }
        };
        Ok((status, value))
    }
}

async fn authorized_binding(
    ctx: &AppContext,
    id: Uuid,
    user: &CurrentUser,
    write: bool,
) -> Result<TaskChatBinding, AppError> {
    let session = ctx.repo.get_session(id).await?;
    super::sessions::ensure_session_read_access(&session, user)?;
    if write && session.user_id != user.id {
        return Err(AppError::Forbidden);
    }
    ctx.repo
        .get_task_chat_binding(id)
        .await?
        .ok_or_else(|| AppError::not_found("task binding", id))
}

fn check_context(
    value: Value,
    gateway: &TrackerGateway,
    task: Uuid,
    owner: &str,
    agent: Uuid,
) -> Result<TaskChatBinding, AppError> {
    let _typed: domain::TrackerTaskContext = decode_response(value.clone())?;
    if value.get("contract_version").and_then(Value::as_u64) != Some(1)
        || value.get("owner_subject").and_then(Value::as_str) != Some(owner)
        || value.get("tracker_instance_id").and_then(Value::as_str)
            != Some(gateway.instance.as_str())
    {
        return Err(AppError::Forbidden);
    }
    let uuid = |field: &str| -> Result<Uuid, AppError> {
        value
            .get(field)
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or_else(|| AppError::Unavailable("Tracker context identity is invalid".into()))
    };
    if uuid("task_id")? != task {
        return Err(AppError::Forbidden);
    }
    Ok(TaskChatBinding {
        tracker_instance_id: gateway.instance.clone(),
        project_id: uuid("project_id")?,
        task_id: task,
        root_task_id: uuid("root_task_id")?,
        agent_id: agent,
        owner_subject: owner.to_string(),
    })
}

fn check_current_agent(value: &Value, agent: Uuid) -> Result<(), AppError> {
    if value
        .get("assignment")
        .and_then(|assignment| assignment.get("agent_id"))
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        != Some(agent)
    {
        return Err(AppError::conflict(
            "task chat no longer matches the concrete assigned PM agent",
        ));
    }
    Ok(())
}

fn context_error(status: StatusCode) -> AppError {
    match status {
        StatusCode::UNAUTHORIZED => AppError::Unauthorized,
        StatusCode::FORBIDDEN => AppError::Forbidden,
        StatusCode::NOT_FOUND => AppError::not_found("Tracker context", "task"),
        StatusCode::CONFLICT => AppError::conflict("Tracker context changed"),
        StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY => {
            AppError::validation("Tracker rejected task context")
        }
        _ => AppError::Unavailable("Tracker context is unavailable".into()),
    }
}

#[utoipa::path(post, path="/api/v1/sessions/{session_id}/task-binding", tag="task-chats", params(("session_id"=Uuid,Path)), request_body=BindTaskChatRequest, responses((status=200,body=TaskChatBinding)))]
pub async fn bind_task_chat(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    subject: Option<Extension<VerifiedCentralSubject>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
    Json(req): Json<BindTaskChatRequest>,
) -> Result<Json<TaskChatBinding>, AppError> {
    let Extension(subject) = subject.ok_or(AppError::Unauthorized)?;
    let session = ctx.repo.get_session(id).await?;
    if session.user_id != user.id {
        return Err(AppError::Forbidden);
    }
    let gateway = TrackerGateway::configured(&ctx.config.tracker)?;
    let (status, value) = gateway
        .request(req.task_id, &["context".into()], &headers, None)
        .await?;
    if status != StatusCode::OK {
        return Err(context_error(status));
    }
    check_current_agent(&value, session.primary_agent_id)?;
    let binding = check_context(
        value,
        &gateway,
        req.task_id,
        &subject.0,
        session.primary_agent_id,
    )?;
    Ok(Json(
        ctx.repo
            .bind_task_chat(id, binding, req.idempotency_key)
            .await?,
    ))
}

#[utoipa::path(get,path="/api/v1/sessions/{session_id}/task-context",tag="task-chats",params(("session_id"=Uuid,Path)),responses((status=200,body=SessionTaskContext)))]
pub async fn task_context(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<SessionTaskContext>, AppError> {
    Ok(Json(
        load_task_context(&ctx, &user, id, &headers, true).await?,
    ))
}

pub(super) async fn load_task_context(
    ctx: &Arc<AppContext>,
    user: &CurrentUser,
    id: Uuid,
    headers: &HeaderMap,
    require_current_agent: bool,
) -> Result<SessionTaskContext, AppError> {
    let session = ctx.repo.get_session(id).await?;
    super::sessions::ensure_session_read_access(&session, user)?;
    let Some(binding) = ctx.repo.get_task_chat_binding(id).await? else {
        return Ok(SessionTaskContext {
            binding: None,
            tracker: None,
        });
    };
    let gateway = TrackerGateway::configured(&ctx.config.tracker)?;
    if gateway.instance != binding.tracker_instance_id {
        return Err(AppError::Unavailable(
            "bound Tracker instance is not configured".into(),
        ));
    }
    let (status, value) = gateway
        .request(binding.task_id, &["context".into()], headers, None)
        .await?;
    if status != StatusCode::OK {
        return Err(context_error(status));
    }
    if require_current_agent {
        check_current_agent(&value, binding.agent_id)?;
    }
    let checked = check_context(
        value.clone(),
        &gateway,
        binding.task_id,
        &binding.owner_subject,
        binding.agent_id,
    )?;
    if checked != binding {
        return Err(AppError::conflict("Tracker binding identity changed"));
    }
    Ok(SessionTaskContext {
        binding: Some(binding),
        tracker: Some(decode_response(value)?),
    })
}

pub(super) async fn require_project_access(
    ctx: &Arc<AppContext>,
    user: &CurrentUser,
    id: Uuid,
    headers: &HeaderMap,
) -> Result<(), AppError> {
    // History belongs to the immutable binding, not the current assignment.
    load_task_context(ctx, user, id, headers, false).await?;
    Ok(())
}

#[utoipa::path(get,path="/api/v1/sessions/{session_id}/clarifications",tag="task-chats",params(("session_id"=Uuid,Path)),responses((status=200,body=domain::TrackerClarifications)))]
pub async fn clarifications(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Value>), AppError> {
    proxy(
        &ctx,
        id,
        &user,
        &headers,
        false,
        vec!["clarifications".into()],
        None,
    )
    .await
}

#[utoipa::path(post,path="/api/v1/sessions/{session_id}/clarifications/{question_id}/answers",tag="task-chats",params(("session_id"=Uuid,Path),("question_id"=Uuid,Path)),request_body=ClarificationAnswerRequest,responses((status=200,body=domain::TrackerAnswer)))]
pub async fn answer(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    subject: Option<Extension<VerifiedCentralSubject>>,
    Path((id, question)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    Json(req): Json<ClarificationAnswerRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let Extension(_) = subject.ok_or(AppError::Unauthorized)?;
    proxy(
        &ctx,
        id,
        &user,
        &headers,
        true,
        vec![
            "clarifications".into(),
            question.to_string(),
            "answers".into(),
        ],
        Some(serde_json::to_value(req).map_err(AppError::internal)?),
    )
    .await
}

#[utoipa::path(get,path="/api/v1/sessions/{session_id}/requirements",tag="task-chats",params(("session_id"=Uuid,Path)),responses((status=200,body=domain::TrackerRequirements)))]
pub async fn requirements(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Value>), AppError> {
    proxy(
        &ctx,
        id,
        &user,
        &headers,
        false,
        vec!["requirements".into(), "revisions".into()],
        None,
    )
    .await
}

#[utoipa::path(post,path="/api/v1/sessions/{session_id}/requirements/{revision}/confirm",tag="task-chats",params(("session_id"=Uuid,Path),("revision"=i64,Path)),request_body=ConfirmRequirementsRequest,responses((status=200,body=domain::TrackerConfirmation)))]
pub async fn confirm(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    subject: Option<Extension<VerifiedCentralSubject>>,
    Path((id, revision)): Path<(Uuid, i64)>,
    headers: HeaderMap,
    Json(req): Json<ConfirmRequirementsRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let Extension(_) = subject.ok_or(AppError::Unauthorized)?;
    if revision < 1 {
        return Err(AppError::validation("revision must be positive"));
    }
    proxy(
        &ctx,
        id,
        &user,
        &headers,
        true,
        vec![
            "requirements".into(),
            revision.to_string(),
            "confirm".into(),
        ],
        Some(serde_json::to_value(req).map_err(AppError::internal)?),
    )
    .await
}

async fn proxy(
    ctx: &AppContext,
    id: Uuid,
    user: &CurrentUser,
    headers: &HeaderMap,
    write: bool,
    path: Vec<String>,
    body: Option<Value>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    let binding = authorized_binding(ctx, id, user, write).await?;
    let gateway = TrackerGateway::configured(&ctx.config.tracker)?;
    if gateway.instance != binding.tracker_instance_id {
        return Err(AppError::Unavailable(
            "bound Tracker instance is not configured".into(),
        ));
    }
    let (context_status, context) = gateway
        .request(binding.task_id, &["context".into()], headers, None)
        .await?;
    if context_status != StatusCode::OK {
        return Ok((context_status, Json(context)));
    }
    check_current_agent(&context, binding.agent_id)?;
    if check_context(
        context,
        &gateway,
        binding.task_id,
        &binding.owner_subject,
        binding.agent_id,
    )? != binding
    {
        return Err(AppError::conflict("Tracker binding identity changed"));
    }
    let response_path = path.clone();
    let (status, value) = gateway
        .request(binding.task_id, &path, headers, body)
        .await?;
    let value = if status.is_success() {
        match response_path.last().map(String::as_str) {
            Some("clarifications") => typed_response::<domain::TrackerClarifications>(value)?,
            Some("answers") => typed_response::<domain::TrackerAnswer>(value)?,
            Some("revisions") => typed_response::<domain::TrackerRequirements>(value)?,
            Some("confirm") => typed_response::<domain::TrackerConfirmation>(value)?,
            _ => {
                return Err(AppError::Unavailable(
                    "unsupported Tracker response contract".into(),
                ));
            }
        }
    } else {
        value
    };
    Ok((status, Json(value)))
}

fn decode_response<T: DeserializeOwned>(value: Value) -> Result<T, AppError> {
    validate_versions(&value)?;
    serde_json::from_value(value)
        .map_err(|_| AppError::Unavailable("Tracker response does not match contract v1".into()))
}

fn validate_versions(value: &Value) -> Result<(), AppError> {
    match value {
        Value::Object(fields) => {
            for (name, value) in fields {
                let version = matches!(
                    name.as_str(),
                    "version"
                        | "revision"
                        | "question_version"
                        | "assignment_version"
                        | "requirement_revision"
                );
                if version
                    && !value.is_null()
                    && !value
                        .as_i64()
                        .is_some_and(|value| (1..=9_007_199_254_740_991).contains(&value))
                {
                    return Err(AppError::Unavailable(
                        "Tracker returned an unsafe version".into(),
                    ));
                }
                validate_versions(value)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                validate_versions(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn typed_response<T: DeserializeOwned + Serialize>(value: Value) -> Result<Value, AppError> {
    serde_json::to_value(decode_response::<T>(value)?).map_err(AppError::internal)
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub before: Option<Uuid>,
    pub limit: Option<u64>,
}

#[utoipa::path(get,path="/api/v1/sessions/{session_id}/history",tag="task-chats",params(("session_id"=Uuid,Path),("before"=Option<Uuid>,Query),("limit"=Option<u64>,Query)),responses((status=200,body=MessageHistoryPage)))]
pub async fn history(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(id): Path<Uuid>,
    Query(query): Query<HistoryQuery>,
    headers: HeaderMap,
) -> Result<Json<MessageHistoryPage>, AppError> {
    require_project_access(&ctx, &user, id, &headers).await?;
    let session = ctx.repo.get_session(id).await?;
    super::sessions::ensure_session_read_access(&session, &user)?;
    Ok(Json(
        ctx.repo
            .session_message_history(id, query.before, query.limit.unwrap_or(50))
            .await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replacement_pm_invalidates_old_binding_and_errors_keep_their_class() {
        let agent = Uuid::new_v4();
        let context = serde_json::json!({"assignment":{"agent_id":agent}});
        assert!(check_current_agent(&context, agent).is_ok());
        assert!(matches!(
            check_current_agent(&context, Uuid::new_v4()),
            Err(AppError::Conflict(_))
        ));
        assert!(matches!(
            context_error(StatusCode::UNAUTHORIZED),
            AppError::Unauthorized
        ));
        assert!(matches!(
            context_error(StatusCode::FORBIDDEN),
            AppError::Forbidden
        ));
        assert!(matches!(
            context_error(StatusCode::SERVICE_UNAVAILABLE),
            AppError::Unavailable(_)
        ));
    }
    #[test]
    fn gateway_rejects_credentials_query_and_non_http_urls() {
        for raw in [
            "file:///etc/passwd",
            "http://user:password@tracker/",
            "http://tracker/?token=secret",
            "http://tracker/#fragment",
            "http://tracker/api",
        ] {
            assert!(TrackerGateway::parse(raw, "tracker".into()).is_err());
        }
        assert!(TrackerGateway::parse("http://tracker:3456/", "tracker".into()).is_ok());
    }
    #[test]
    fn context_identity_cannot_be_spoofed_by_local_uuid_or_email() {
        let gateway = TrackerGateway::parse("http://tracker/", "tracker".into()).unwrap();
        let task = Uuid::new_v4();
        let value = serde_json::json!({"contract_version":1,"tracker_instance_id":"tracker","task_id":task,"project_id":Uuid::new_v4(),"root_task_id":task,"owner_subject":"subject-a","stage":"Draft","requirement_revision":null,"waiting_reason":null,"permissions":{"can_answer":true,"can_confirm":false},"assignment":null});
        assert!(check_context(value.clone(), &gateway, task, "subject-a", Uuid::new_v4()).is_ok());
        assert!(check_context(value, &gateway, task, "subject-b", Uuid::new_v4()).is_err());
    }
    #[test]
    fn invalid_contract_shape_and_unsafe_versions_never_return_success() {
        assert!(
            typed_response::<domain::TrackerClarifications>(
                serde_json::json!({"questions":[{"text":"missing identity"}]})
            )
            .is_err()
        );
        for version in [0i64, -1, 9_007_199_254_740_992] {
            assert!(
                validate_versions(&serde_json::json!({"questions":[{"version":version}]})).is_err()
            );
        }
        assert!(
            validate_versions(
                &serde_json::json!({"requirement_revision":null,"version":9_007_199_254_740_991i64})
            )
            .is_ok()
        );
    }
}
