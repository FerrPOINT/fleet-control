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
