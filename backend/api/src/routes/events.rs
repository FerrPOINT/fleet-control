use app::AppContext;
use axum::{
    Extension, Json,
    extract::Query,
    extract::State,
    http::{HeaderMap, header},
    response::sse::{Event, KeepAlive, Sse},
};
use domain::AgentEvent;
use futures_util::stream::Stream;
use serde::Deserialize;
use shared::{AppError, FleetEvent};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    pub limit: Option<u64>,
}

#[utoipa::path(get, path = "/api/v1/events/recent", tag = "runtime", responses((status = 200, body = Vec<AgentEvent>)))]
pub async fn recent_events(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    Query(query): Query<EventsQuery>,
) -> Result<Json<Vec<AgentEvent>>, AppError> {
    crate::middleware::require_operator(&user)?;
    Ok(Json(
        ctx.repo
            .list_events(query.limit.unwrap_or(100).min(500))
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/events", tag = "runtime", responses((status = 200, description = "SSE event stream")))]
pub async fn events(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<crate::middleware::CurrentUser>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    crate::middleware::require_operator(&user)?;
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?
        .to_owned();
    let receiver = ctx.events.subscribe();
    let stream = futures_util::stream::unfold(
        (ctx, user.id, token, receiver),
        |(ctx, user_id, token, mut receiver)| async move {
            loop {
                let pending = tokio::time::timeout(Duration::from_secs(1), receiver.recv()).await;
                if matches!(pending, Ok(Err(RecvError::Closed))) {
                    return None;
                }
                // Validate after waiting: an event arriving after revocation must not escape.
                let central = stream_principal(&ctx, &token, user_id).await.ok()?;
                let Ok(Ok(event)) = pending else {
                    continue;
                };
                if let Some(session_id) = event_session_id(&event) {
                    let session = ctx.repo.get_session(session_id.parse().ok()?).await.ok()?;
                    if !session_event_visible(central, user_id, session.user_id, session.visibility)
                    {
                        continue;
                    }
                }
                let json = serde_json::to_string(&event).ok()?;
                return Some((
                    Ok::<_, Infallible>(Event::default().event("fleet").data(json)),
                    (ctx, user_id, token, receiver),
                ));
            }
        },
    );
    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(20))))
}

async fn stream_principal(
    ctx: &AppContext,
    token: &str,
    expected_user_id: Uuid,
) -> Result<bool, AppError> {
    use crate::middleware::central_auth::{CentralCheck, check_token};

    match check_token(token).await {
        CentralCheck::Validated(central, name) => {
            if !central.allows_service("fleet-control", "GET") {
                return Err(AppError::Forbidden);
            }
            let user =
                crate::middleware::resolve_central_user(ctx, &central, name.as_deref()).await?;
            ensure_stream_user(&user, expected_user_id, true)?;
            Ok(true)
        }
        CentralCheck::FallThrough
            if std::env::var_os("FLEET_CONTROL_AUTH__CENTRAL_JWKS_URI").is_none() =>
        {
            let claims = ctx.auth.validate_access_token(token).await?;
            if claims.sub.parse::<Uuid>().ok() != Some(expected_user_id) {
                return Err(AppError::Unauthorized);
            }
            let user = ctx
                .repo
                .find_user_by_id(expected_user_id)
                .await?
                .ok_or(AppError::Unauthorized)?;
            ensure_stream_user(&user, expected_user_id, false)?;
            Ok(false)
        }
        _ => Err(AppError::Unauthorized),
    }
}

fn ensure_stream_user(
    user: &app::auth::UserRecord,
    expected_user_id: Uuid,
    central: bool,
) -> Result<(), AppError> {
    if user.id != expected_user_id || !user.is_active {
        return Err(AppError::Unauthorized);
    }
    if !central && !user.system_role.can_operate_fleet() {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

fn event_session_id(event: &FleetEvent) -> Option<&str> {
    match event {
        FleetEvent::SessionChanged { session_id, .. }
        | FleetEvent::SessionMessageChanged { session_id, .. }
        | FleetEvent::SessionRunChanged { session_id, .. }
        | FleetEvent::SessionRunDelta { session_id, .. }
        | FleetEvent::RuntimeApprovalRequested { session_id, .. } => Some(session_id),
        _ => None,
    }
}

fn session_event_visible(
    central: bool,
    user_id: Uuid,
    owner_id: Uuid,
    visibility: domain::SessionVisibility,
) -> bool {
    !central || owner_id == user_id || visibility == domain::SessionVisibility::LeaderScoped
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{SessionVisibility, SystemRole};

    fn user() -> app::auth::UserRecord {
        app::auth::UserRecord {
            id: Uuid::new_v4(),
            email: "stream@example.test".into(),
            username: "stream".into(),
            display_name: "Stream".into(),
            password_hash: "!".into(),
            refresh_token_hash: None,
            system_role: SystemRole::User,
            is_system_admin: false,
            is_active: true,
        }
    }

    #[test]
    fn central_stream_revalidation_never_depends_on_historical_role() {
        let mut user = user();
        for role in [SystemRole::User, SystemRole::Operator, SystemRole::Admin] {
            user.system_role = role;
            assert!(ensure_stream_user(&user, user.id, true).is_ok());
        }
    }

    #[test]
    fn stream_principal_must_stay_active_and_bound_to_the_original_user() {
        let mut user = user();
        assert!(ensure_stream_user(&user, Uuid::new_v4(), true).is_err());
        user.is_active = false;
        for central in [true, false] {
            assert!(ensure_stream_user(&user, user.id, central).is_err());
        }
    }

    #[test]
    fn standalone_stream_rechecks_current_role() {
        let mut user = user();
        assert!(ensure_stream_user(&user, user.id, false).is_err());
        for role in [SystemRole::Operator, SystemRole::Admin] {
            user.system_role = role;
            assert!(ensure_stream_user(&user, user.id, false).is_ok());
        }
    }

    #[test]
    fn private_session_events_are_owner_only_and_shared_events_remain_visible() {
        let owner = Uuid::new_v4();
        let other = Uuid::new_v4();
        assert!(session_event_visible(
            true,
            owner,
            owner,
            SessionVisibility::Private
        ));
        assert!(!session_event_visible(
            true,
            other,
            owner,
            SessionVisibility::Private
        ));
        assert!(session_event_visible(
            true,
            other,
            owner,
            SessionVisibility::LeaderScoped
        ));
        assert!(session_event_visible(
            false,
            other,
            owner,
            SessionVisibility::Private
        ));
    }

    #[test]
    fn every_session_event_variant_uses_the_privacy_boundary() {
        let session_id = Uuid::new_v4().to_string();
        let events = [
            FleetEvent::SessionChanged {
                session_id: session_id.clone(),
                agent_id: "agent".into(),
            },
            FleetEvent::SessionMessageChanged {
                session_id: session_id.clone(),
                message_id: "message".into(),
                event: "updated".into(),
            },
            FleetEvent::SessionRunChanged {
                session_id: session_id.clone(),
                run_id: "run".into(),
                runtime_run_id: None,
                state: "running".into(),
            },
            FleetEvent::SessionRunDelta {
                session_id: session_id.clone(),
                run_id: "run".into(),
                runtime_run_id: None,
                delta: "private text".into(),
            },
            FleetEvent::RuntimeApprovalRequested {
                session_id: session_id.clone(),
                run_id: "run".into(),
                approval_id: "approval".into(),
            },
        ];
        for event in events {
            assert_eq!(event_session_id(&event), Some(session_id.as_str()));
        }
        assert_eq!(
            event_session_id(&FleetEvent::AgentCreated {
                agent_id: "agent".into(),
                name: "agent".into()
            }),
            None
        );
    }
}
