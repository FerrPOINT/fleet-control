//! Verified, immutable context projection. No user PAT crosses a product boundary.
use domain::execution_context::{ExecutionContextV2, SessionExecutionContext};
use sea_orm::{
    ConnectionTrait, DatabaseBackend, DatabaseConnection, EntityTrait, QuerySelect, Statement,
    TransactionTrait,
};
use shared::AppError;
use uuid::Uuid;

pub(super) async fn read(
    db: &DatabaseConnection,
    session: Uuid,
) -> Result<Option<SessionExecutionContext>, AppError> {
    let row=db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,"SELECT context,operation_id,verified_projection FROM session_execution_contexts WHERE session_id=$1",[session.into()])).await.map_err(AppError::database)?;
    row.map(|row| {
        let value: serde_json::Value = row
            .try_get("", "verified_projection")
            .map_err(AppError::database)?;
        let result: SessionExecutionContext = serde_json::from_value(value)
            .map_err(|_| AppError::Unavailable("invalid_execution_context_projection".into()))?;
        let original: serde_json::Value = row.try_get("", "context").map_err(AppError::database)?;
        let operation: Uuid = row
            .try_get("", "operation_id")
            .map_err(AppError::database)?;
        if !result.context.valid()
            || result.tracker_project_id.is_nil()
            || result.binding_generation <= 0
            || result.adapter_version != "namespace-context-v2/foundation-v1-disabled"
            || result.runtime_ready
            || result.dispatch_allowed
            || result.context.operation_id != operation
            || serde_json::to_value(&result.context).map_err(AppError::internal)? != original
        {
            return Err(AppError::Unavailable(
                "invalid_execution_context_projection".into(),
            ));
        }
        Ok(result)
    })
    .transpose()
}

async fn owner_read(
    key: &str,
    path: &str,
    context: &ExecutionContextV2,
) -> Result<serde_json::Value, AppError> {
    let raw = std::env::var(format!("FLEET_CONTROL_NAMESPACE__{key}_URL"))
        .map_err(|_| AppError::Unavailable("execution_context_reader_not_configured".into()))?;
    let base = reqwest::Url::parse(&raw)
        .map_err(|_| AppError::Unavailable("invalid_owner_endpoint".into()))?;
    if !matches!(base.scheme(), "http" | "https")
        || base.host_str().is_none()
        || !base.username().is_empty()
        || base.password().is_some()
        || base.path() != "/"
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return Err(AppError::Unavailable("invalid_owner_endpoint".into()));
    }
    let token_file = std::env::var(format!("FLEET_CONTROL_NAMESPACE__{key}_TOKEN_FILE"))
        .map_err(|_| AppError::Unavailable("execution_context_reader_not_configured".into()))?;
    let token = tokio::fs::read_to_string(token_file)
        .await
        .map_err(|_| AppError::Unavailable("execution_context_reader_not_configured".into()))?;
    if token.trim().is_empty() || token.trim().contains(['\r', '\n']) {
        return Err(AppError::Unavailable("invalid_owner_credential".into()));
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| AppError::Unavailable("owner_reader_unavailable".into()))?;
    let mut response = client
        .get(
            base.join(path)
                .map_err(|_| AppError::Unavailable("invalid_owner_endpoint".into()))?,
        )
        .query(&[
            (
                "registry_instance_id",
                context.namespace.registry_instance_id,
            ),
            ("namespace_id", context.namespace.namespace_id),
        ])
        .bearer_auth(token.trim())
        .send()
        .await
        .map_err(|_| AppError::Unavailable("owner_reader_unavailable".into()))?;
    if response.status() == reqwest::StatusCode::FORBIDDEN
        || response.status() == reqwest::StatusCode::NOT_FOUND
    {
        return Err(AppError::conflict("foreign_or_missing_context_resource"));
    }
    if !response.status().is_success() {
        return Err(AppError::Unavailable("owner_reader_unavailable".into()));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| AppError::Unavailable("owner_reader_unavailable".into()))?
    {
        if bytes.len() + chunk.len() > 65536 {
            return Err(AppError::Unavailable("invalid_owner_readback".into()));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| AppError::Unavailable("invalid_owner_readback".into()))
}

pub(super) async fn bind(
    db: &DatabaseConnection,
    session: Uuid,
    actor: Uuid,
    input: ExecutionContextV2,
) -> Result<SessionExecutionContext, AppError> {
    if !input.valid() {
        return Err(AppError::validation("invalid_execution_context_v2"));
    }
    // Original-key readback precedes all cross-product requests. An outage cannot break replay.
    if let Some(old) = read(db, session).await? {
        if old.context == input {
            return Ok(old);
        }
        return Err(AppError::conflict("execution_context_already_bound"));
    }
    let task = owner_read(
        "TRACKER",
        &format!("api/v1/namespace-tasks/{}", input.task.task_id),
        &input,
    )
    .await?;
    if task["namespace"] != serde_json::to_value(&input.namespace).map_err(AppError::internal)?
        || task["task_id"] != input.task.task_id.to_string()
        || task["tracker_instance_id"] != input.task.tracker_instance_id.to_string()
        || task["state"] != "active"
    {
        return Err(AppError::conflict("invalid_task_context"));
    }
    let project = task["project_id"]
        .as_str()
        .and_then(|value| value.parse::<Uuid>().ok())
        .filter(|id| !id.is_nil())
        .ok_or_else(|| AppError::Unavailable("invalid_owner_readback".into()))?;
    let generation = task["generation"]
        .as_i64()
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::Unavailable("invalid_owner_readback".into()))?;
    for repository in &input.repositories {
        let verified = owner_read(
            "FORGE",
            &format!("api/v1/namespace-repositories/{}", repository.repository_id),
            &input,
        )
        .await?;
        if verified["namespace"]
            != serde_json::to_value(&input.namespace).map_err(AppError::internal)?
            || verified["repository_id"] != repository.repository_id.to_string()
            || verified["forge_instance_id"] != repository.forge_instance_id.to_string()
        {
            return Err(AppError::conflict("invalid_repository_context"));
        }
    }
    let result = SessionExecutionContext {
        context: input.clone(),
        tracker_project_id: project,
        binding_generation: generation,
        runtime_ready: false,
        dispatch_allowed: false,
        adapter_version: "namespace-context-v2/foundation-v1-disabled".into(),
    };
    let tx = db.begin().await.map_err(AppError::database)?;
    let saved_session = crate::entities::agent_session::Entity::find_by_id(session)
        .lock_exclusive()
        .one(&tx)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("agent_session", session))?;
    let saved_actor = crate::entities::user::Entity::find_by_id(actor)
        .lock_shared()
        .one(&tx)
        .await
        .map_err(AppError::database)?
        .ok_or(AppError::Unauthorized)?;
    let can_write_other = if saved_actor.central_sub.is_some() {
        saved_session.visibility == domain::SessionVisibility::LeaderScoped.as_str()
    } else {
        super::parse_system_role(&saved_actor.system_role, saved_actor.is_system_admin)
            .can_operate_fleet()
    };
    if !saved_actor.is_active || (actor != saved_session.user_id && !can_write_other) {
        return Err(AppError::Forbidden);
    }
    let pending=tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,"SELECT EXISTS(SELECT 1 FROM session_agent_runs WHERE session_id=$1 AND state IN ('pending','running','waiting','stopping')) OR EXISTS(SELECT 1 FROM message_dispatch_outbox o JOIN session_messages m ON m.id=o.message_id WHERE m.session_id=$1 AND o.state IN ('pending','dispatching','uncertain')) AS busy",[session.into()])).await.map_err(AppError::database)?.ok_or_else(|| AppError::Unavailable("context_readback_missing".into()))?;
    if pending
        .try_get::<bool>("", "busy")
        .map_err(AppError::database)?
    {
        return Err(AppError::conflict(
            "session_must_be_drained_before_context_binding",
        ));
    }
    tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,"INSERT INTO session_execution_contexts(session_id,operation_id,context,verified_projection,created_by_user_id) VALUES($1,$2,$3,$4,$5) ON CONFLICT(session_id) DO NOTHING",[session.into(),input.operation_id.into(),serde_json::to_value(&input).map_err(AppError::internal)?.into(),serde_json::to_value(&result).map_err(AppError::internal)?.into(),actor.into()])).await.map_err(AppError::database)?;
    tx.commit().await.map_err(AppError::database)?;
    let saved = read(db, session)
        .await?
        .ok_or_else(|| AppError::Unavailable("context_readback_missing".into()))?;
    if saved.context != input {
        return Err(AppError::conflict("execution_context_already_bound"));
    }
    Ok(saved)
}
