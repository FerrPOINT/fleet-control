use app::AppContext;
use axum::{
    body::Body,
    extract::State,
    http::{Request, header},
    middleware::Next,
    response::Response,
};
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub id: Uuid,
}

pub async fn require_auth(
    State(ctx): State<Arc<AppContext>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;
    let claims = ctx.auth.validate_access_token(token)?;
    let user_id = claims
        .sub
        .parse::<Uuid>()
        .map_err(|_| AppError::Unauthorized)?;
    let user = ctx
        .repo
        .find_user_by_id(user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    if !user.is_active {
        return Err(AppError::Forbidden);
    }
    req.extensions_mut().insert(CurrentUser { id: user.id });
    Ok(next.run(req).await)
}
