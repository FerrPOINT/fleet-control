use app::AppContext;
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use domain::execution_context::{ExecutionContextV2, SessionExecutionContext};
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;
#[utoipa::path(get,operation_id="fleet_get_execution_context_v2",path="/api/v2/sessions/{session_id}/execution-context",tag="sessions",params(("session_id"=Uuid,Path)),responses((status=200,body=Option<SessionExecutionContext>)))]
pub async fn get(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(session): Path<Uuid>,
) -> Result<Json<Option<SessionExecutionContext>>, AppError> {
    super::sessions::ensure_session_read_access(&ctx.repo.get_session(session).await?, &user)?;
    Ok(Json(ctx.repo.session_execution_context(session).await?))
}
#[utoipa::path(put,operation_id="fleet_bind_execution_context_v2",path="/api/v2/sessions/{session_id}/execution-context",tag="sessions",params(("session_id"=Uuid,Path)),request_body=ExecutionContextV2,responses((status=200,body=SessionExecutionContext),(status=409,description="Immutable context/replay conflict")))]
pub async fn bind(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Path(session): Path<Uuid>,
    Json(input): Json<ExecutionContextV2>,
) -> Result<Json<SessionExecutionContext>, AppError> {
    super::sessions::ensure_session_write_access(&ctx.repo.get_session(session).await?, &user)?;
    Ok(Json(
        ctx.repo
            .bind_session_execution_context(session, user.id, input)
            .await?,
    ))
}

#[utoipa::path(post, operation_id="fleet_create_context_session_v2", path="/api/v2/sessions", tag="sessions", request_body=domain::execution_context::CreateContextSessionRequest, responses((status=200,body=domain::execution_context::ContextSessionReceipt),(status=409,description="Original-key context conflict")))]
pub async fn create(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Json(input): Json<domain::execution_context::CreateContextSessionRequest>,
) -> Result<Json<domain::execution_context::ContextSessionReceipt>, AppError> {
    let receipt = ctx.repo.create_context_session(input, user.id).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "session.context_created",
            "session",
            Some(receipt.session.id.to_string()),
            serde_json::to_value(&receipt.execution_context).map_err(AppError::internal)?,
        )
        .await?;
    Ok(Json(receipt))
}
