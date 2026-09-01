use app::{AppContext, auth};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use domain::{AuthResponse, LoginRequest, RegisterRequest};
use shared::AppError;
use std::sync::Arc;

#[utoipa::path(post, path = "/api/v1/auth/register", tag = "auth", request_body = RegisterRequest, responses((status = 200, body = AuthResponse)))]
pub async fn register(
    State(ctx): State<Arc<AppContext>>,
    jar: CookieJar,
    Json(req): Json<RegisterRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), AppError> {
    let req = auth::normalize_register(req)?;
    let existing = ctx.repo.find_user_by_email(&req.email).await?;
    if existing.is_some() {
        return Err(AppError::conflict("email is already registered"));
    }
    let is_first_user = ctx.repo.list_users().await?.is_empty();
    let password_hash = ctx.auth.hash_password(&req.password)?;
    let user = ctx
        .repo
        .create_user(req, password_hash, is_first_user)
        .await?;
    let tokens = ctx.auth.issue_tokens(&user)?;
    ctx.repo
        .update_refresh_hash(user.id, Some(tokens.refresh_hash.clone()))
        .await?;
    Ok((
        jar.add(refresh_cookie(&ctx, tokens.refresh_token)),
        Json(tokens.response),
    ))
}

#[utoipa::path(post, path = "/api/v1/auth/login", tag = "auth", request_body = LoginRequest, responses((status = 200, body = AuthResponse)))]
pub async fn login(
    State(ctx): State<Arc<AppContext>>,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), AppError> {
    let req = auth::normalize_login(req);
    let user = ctx
        .repo
        .find_user_by_email(&req.email)
        .await?
        .ok_or(AppError::Unauthorized)?;
    if !user.is_active {
        return Err(AppError::Forbidden);
    }
    ctx.auth
        .verify_password(&req.password, &user.password_hash)?;
    let tokens = ctx.auth.issue_tokens(&user)?;
    ctx.repo
        .update_refresh_hash(user.id, Some(tokens.refresh_hash.clone()))
        .await?;
    Ok((
        jar.add(refresh_cookie(&ctx, tokens.refresh_token)),
        Json(tokens.response),
    ))
}

pub async fn refresh(
    State(ctx): State<Arc<AppContext>>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<AuthResponse>), AppError> {
    let cookie = jar
        .get(&ctx.config.auth.refresh_cookie_name)
        .ok_or(AppError::Unauthorized)?;
    let refresh_hash = auth::hash_refresh_token(cookie.value());
    for user in ctx.repo.list_users().await? {
        let Some(record) = ctx.repo.find_user_by_id(user.id).await? else {
            continue;
        };
        if record.refresh_token_hash.as_deref() == Some(refresh_hash.as_str()) {
            let tokens = ctx.auth.issue_tokens(&record)?;
            ctx.repo
                .update_refresh_hash(record.id, Some(tokens.refresh_hash.clone()))
                .await?;
            return Ok((
                jar.add(refresh_cookie(&ctx, tokens.refresh_token)),
                Json(tokens.response),
            ));
        }
    }
    Err(AppError::Unauthorized)
}

#[utoipa::path(post, path = "/api/v1/auth/refresh", tag = "auth", responses((status = 200, body = AuthResponse), (status = 401)))]
pub async fn refresh_openapi() {}

pub async fn logout(State(ctx): State<Arc<AppContext>>, jar: CookieJar) -> impl IntoResponse {
    let cookie = Cookie::build((ctx.config.auth.refresh_cookie_name.clone(), ""))
        .path(ctx.config.auth.refresh_cookie_path.clone())
        .max_age(time::Duration::seconds(0))
        .build();
    (jar.add(cookie), StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/auth/logout", tag = "auth", responses((status = 204)))]
pub async fn logout_openapi() {}

fn refresh_cookie(ctx: &AppContext, value: String) -> Cookie<'static> {
    let same_site = match ctx.config.auth.refresh_cookie_same_site.as_str() {
        "Strict" => SameSite::Strict,
        "None" => SameSite::None,
        _ => SameSite::Lax,
    };
    let mut builder = Cookie::build((ctx.config.auth.refresh_cookie_name.clone(), value))
        .path(ctx.config.auth.refresh_cookie_path.clone())
        .http_only(true)
        .secure(ctx.config.auth.refresh_cookie_secure)
        .same_site(same_site);
    if let Some(domain) = &ctx.config.auth.refresh_cookie_domain {
        builder = builder.domain(domain.clone());
    }
    builder.build()
}
