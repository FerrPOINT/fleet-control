use crate::middleware::CurrentUser;
use app::AppContext;
use axum::{
    Extension, Json,
    extract::{Query, State},
    http::HeaderMap,
};
use domain::{ChatsDirectoryFilter, ChatsDirectoryPage};
use serde::Deserialize;
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatsDirectoryQuery {
    pub agent_id: Option<Uuid>,
    pub user_id: Option<String>,
    pub q: Option<String>,
    pub before: Option<Uuid>,
    pub limit: Option<u64>,
}

fn authorized_filter(
    query: ChatsDirectoryQuery,
    current: &CurrentUser,
) -> Result<ChatsDirectoryFilter, AppError> {
    // Match sessions::parse_user_filter: omitted means mine, empty/all requires read_all.
    let (mut user_ids, include_all_users) = match query.user_id.as_deref().map(str::trim) {
        None => (vec![current.id], false),
        Some(value) if value.is_empty() || value.eq_ignore_ascii_case("all") => {
            if !current.can_read_all_sessions() {
                return Err(AppError::Forbidden);
            }
            (Vec::new(), true)
        }
        Some(value) => {
            let ids = value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(|item| {
                    Uuid::parse_str(item).map_err(|_| AppError::validation("invalid user_id"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if ids.iter().any(|id| *id != current.id) && !current.can_read_all_sessions() {
                return Err(AppError::Forbidden);
            }
            (ids, false)
        }
    };
    user_ids.sort_unstable();
    user_ids.dedup();
    let filter = ChatsDirectoryFilter {
        agent_id: query.agent_id,
        user_ids,
        include_all_users,
        search: query.q.unwrap_or_default().trim().to_string(),
        before: query.before,
        limit: query.limit.unwrap_or(50),
        task_project_access: None,
    };
    filter.validate()?;
    Ok(filter)
}

#[utoipa::path(
    get, path = "/api/v1/chats/directory", tag = "task-chats",
    params(
        ("agent_id" = Option<Uuid>, Query, description = "Concrete non-archived agent"),
        ("user_id" = Option<String>, Query, description = "Omit for mine; all or comma-separated UUIDs requires read_all for other users"),
        ("q" = Option<String>, Query, description = "Literal case-insensitive title, task-key or owner search, at most 200 characters"),
        ("before" = Option<Uuid>, Query, description = "Session cursor in this agent/user/search scope; ordered by created_at DESC, id DESC"),
        ("limit" = Option<u64>, Query, description = "Page size 1..100, default 50")
    ),
    responses((status = 200, body = ChatsDirectoryPage), (status = 403, description = "Scope denied"), (status = 422, description = "Invalid filter or cursor"))
)]
pub async fn directory(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Query(query): Query<ChatsDirectoryQuery>,
    headers: HeaderMap,
) -> Result<Json<ChatsDirectoryPage>, AppError> {
    let mut filter = authorized_filter(query, &user)?;
    if !ctx.config.tracker.url.is_empty() || !ctx.config.tracker.instance_id.is_empty() {
        filter.task_project_access =
            Some(super::project_access::authorized_projects(&ctx, &headers).await?);
    }
    Ok(Json(ctx.repo.list_chats_directory(filter).await?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        routing::get,
    };
    use tower::ServiceExt;

    fn user(role: domain::SystemRole) -> CurrentUser {
        CurrentUser {
            id: Uuid::new_v4(),
            role,
            is_system_admin: false,
        }
    }

    #[test]
    fn mine_is_default_even_for_operators() {
        for role in [
            domain::SystemRole::User,
            domain::SystemRole::Operator,
            domain::SystemRole::Admin,
        ] {
            let current = user(role);
            let filter = authorized_filter(ChatsDirectoryQuery::default(), &current).unwrap();
            assert_eq!(filter.user_ids, vec![current.id]);
            assert!(!filter.include_all_users);
        }
    }

    #[test]
    fn explicit_own_scope_is_deduplicated_without_read_all() {
        let current = user(domain::SystemRole::User);
        let filter = authorized_filter(
            ChatsDirectoryQuery {
                user_id: Some(format!(" {}, {}, ", current.id, current.id)),
                q: Some("  literal%_  ".into()),
                ..Default::default()
            },
            &current,
        )
        .unwrap();
        assert_eq!(filter.user_ids, vec![current.id]);
        assert!(!filter.include_all_users);
        assert_eq!(filter.search, "literal%_");
    }

    #[test]
    fn read_all_is_required_for_all_and_multiuser_counts_and_list() {
        let mut current = user(domain::SystemRole::User);
        // Legacy is_system_admin cannot elevate the effective role.
        current.is_system_admin = true;
        for scope in [
            "all".to_string(),
            "".to_string(),
            Uuid::new_v4().to_string(),
            format!("{},{}", current.id, Uuid::new_v4()),
        ] {
            assert!(matches!(
                authorized_filter(
                    ChatsDirectoryQuery {
                        user_id: Some(scope),
                        ..Default::default()
                    },
                    &current
                ),
                Err(AppError::Forbidden)
            ));
        }
        for role in [domain::SystemRole::Operator, domain::SystemRole::Admin] {
            let current = user(role);
            let all = authorized_filter(
                ChatsDirectoryQuery {
                    user_id: Some("ALL".into()),
                    ..Default::default()
                },
                &current,
            )
            .unwrap();
            assert!(all.include_all_users);
            let other = Uuid::new_v4();
            let ids = authorized_filter(
                ChatsDirectoryQuery {
                    user_id: Some(format!("{other}, {}, {other}", current.id)),
                    ..Default::default()
                },
                &current,
            )
            .unwrap()
            .user_ids;
            assert_eq!(ids.len(), 2);
            assert!(ids.contains(&other) && ids.contains(&current.id));
        }
    }

    #[tokio::test]
    async fn http_query_rejects_bad_ids_limits_and_foreign_scope() {
        async fn validate(
            Extension(user): Extension<CurrentUser>,
            Query(query): Query<ChatsDirectoryQuery>,
        ) -> Result<Json<serde_json::Value>, AppError> {
            let filter = authorized_filter(query, &user)?;
            Ok(Json(
                serde_json::json!({"user_ids":filter.user_ids, "all":filter.include_all_users}),
            ))
        }
        let current = user(domain::SystemRole::User);
        let router = Router::new()
            .route("/", get(validate))
            .layer(Extension(current));
        for (query, expected) in [
            ("?agent_id=invalid".to_string(), StatusCode::BAD_REQUEST),
            ("?before=invalid".to_string(), StatusCode::BAD_REQUEST),
            ("?limit=101".to_string(), StatusCode::UNPROCESSABLE_ENTITY),
            ("?limit=0".to_string(), StatusCode::UNPROCESSABLE_ENTITY),
            (
                format!("?before={}", Uuid::new_v4()),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                format!("?q={}", "x".repeat(201)),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            ("?user_id=,%20,".to_string(), StatusCode::FORBIDDEN),
            (
                "?user_id=invalid".to_string(),
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            ("?user_id=all".to_string(), StatusCode::FORBIDDEN),
            ("?project_ids=ignored".to_string(), StatusCode::BAD_REQUEST),
            (
                "?tracker_instance_id=ignored".to_string(),
                StatusCode::BAD_REQUEST,
            ),
            (
                format!("?user_id={}", Uuid::new_v4()),
                StatusCode::FORBIDDEN,
            ),
        ] {
            let response = router
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/{query}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected, "{query}");
        }
    }
}
