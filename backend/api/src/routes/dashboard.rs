use app::AppContext;
use axum::{Extension, Json, extract::State};
use domain::FleetDashboard;
use shared::AppError;
use std::sync::Arc;

#[utoipa::path(get, path = "/api/v1/dashboard", tag = "agents", responses((status = 200, body = FleetDashboard)))]
pub async fn get_dashboard(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
) -> Result<Json<FleetDashboard>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(ctx.dashboard().await?))
}
