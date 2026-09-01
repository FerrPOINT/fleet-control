use app::AppContext;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{
        HeaderValue, Method,
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    },
    middleware::from_fn_with_state,
    routing::{get, post, put},
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod middleware;
pub mod routes;

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::health::health,
        routes::auth::register,
        routes::auth::login,
        routes::auth::refresh_openapi,
        routes::auth::logout_openapi,
        routes::users::get_me,
        routes::users::list_users,
        routes::dashboard::get_dashboard,
        routes::agents::list_agents,
        routes::agents::create_agent,
        routes::agents::get_agent,
        routes::agents::update_agent,
        routes::agents::archive_agent,
        routes::agents::provision_agent,
        routes::agents::start_agent,
        routes::agents::stop_agent,
        routes::agents::restart_agent,
        routes::agents::agent_health,
        routes::agents::get_agent_config,
        routes::agents::update_agent_config,
        routes::agents::list_agent_skills,
        routes::agents::update_agent_skill,
        routes::sessions::list_sessions,
        routes::sessions::create_session,
        routes::sessions::get_session,
        routes::sessions::handoff_session,
        routes::workflows::list_workflow_bindings,
        routes::deployments::list_runtime_templates,
        routes::logs::list_logs,
        routes::events::events,
    ),
    components(schemas(
        domain::Agent,
        domain::AgentKind,
        domain::AgentRole,
        domain::AgentStatus,
        domain::DesiredState,
        domain::SkillState,
        domain::SessionState,
        domain::AgentPaths,
        domain::AgentRuntime,
        domain::AgentConfig,
        domain::AgentSkill,
        domain::AgentSession,
        domain::RuntimeTemplate,
        domain::WorkflowBinding,
        domain::AgentEvent,
        domain::AgentLogEntry,
        domain::FleetDashboard,
        domain::RegisterRequest,
        domain::LoginRequest,
        domain::AuthResponse,
        domain::UserResponse,
        domain::UserListResponse,
        domain::CreateAgentRequest,
        domain::UpdateAgentRequest,
        domain::UpdateAgentConfigRequest,
        domain::UpdateSkillRequest,
        domain::CreateSessionRequest,
        domain::HandoffSessionRequest,
        domain::RuntimeOperationResponse,
    )),
    tags(
        (name = "auth", description = "Authentication"),
        (name = "agents", description = "Agent fleet management"),
        (name = "sessions", description = "Cross-agent task sessions"),
        (name = "runtime", description = "Runtime templates and deployments")
    )
)]
pub struct ApiDoc;

pub fn router(ctx: Arc<AppContext>) -> Router<Arc<AppContext>> {
    let cors = cors_layer(&ctx.config.server.cors_allowed_origins);

    let protected = Router::new()
        .route("/api/v1/users/me", get(routes::users::get_me))
        .route("/api/v1/users", get(routes::users::list_users))
        .route("/api/v1/dashboard", get(routes::dashboard::get_dashboard))
        .route(
            "/api/v1/agents",
            get(routes::agents::list_agents).post(routes::agents::create_agent),
        )
        .route(
            "/api/v1/agents/{agent_id}",
            get(routes::agents::get_agent)
                .patch(routes::agents::update_agent)
                .delete(routes::agents::archive_agent),
        )
        .route(
            "/api/v1/agents/{agent_id}/provision",
            post(routes::agents::provision_agent),
        )
        .route(
            "/api/v1/agents/{agent_id}/start",
            post(routes::agents::start_agent),
        )
        .route(
            "/api/v1/agents/{agent_id}/stop",
            post(routes::agents::stop_agent),
        )
        .route(
            "/api/v1/agents/{agent_id}/restart",
            post(routes::agents::restart_agent),
        )
        .route(
            "/api/v1/agents/{agent_id}/health",
            post(routes::agents::agent_health),
        )
        .route(
            "/api/v1/agents/{agent_id}/config",
            get(routes::agents::get_agent_config).put(routes::agents::update_agent_config),
        )
        .route(
            "/api/v1/agents/{agent_id}/skills",
            get(routes::agents::list_agent_skills),
        )
        .route(
            "/api/v1/agents/{agent_id}/skills/{skill_name}",
            put(routes::agents::update_agent_skill),
        )
        .route(
            "/api/v1/sessions",
            get(routes::sessions::list_sessions).post(routes::sessions::create_session),
        )
        .route(
            "/api/v1/sessions/{session_id}",
            get(routes::sessions::get_session),
        )
        .route(
            "/api/v1/sessions/{session_id}/handoff",
            post(routes::sessions::handoff_session),
        )
        .route(
            "/api/v1/workflow-bindings",
            get(routes::workflows::list_workflow_bindings),
        )
        .route(
            "/api/v1/runtime-templates",
            get(routes::deployments::list_runtime_templates),
        )
        .route("/api/v1/logs", get(routes::logs::list_logs))
        .route("/api/v1/events", get(routes::events::events))
        .route_layer(from_fn_with_state(ctx.clone(), middleware::require_auth));

    Router::new()
        .route("/api/v1/health", get(routes::health::health))
        .route("/api/v1/auth/register", post(routes::auth::register))
        .route("/api/v1/auth/login", post(routes::auth::login))
        .route("/api/v1/auth/refresh", post(routes::auth::refresh))
        .route("/api/v1/auth/logout", post(routes::auth::logout))
        .merge(protected)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(cors)
}

fn cors_layer(allowed_origins: &[String]) -> CorsLayer {
    CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([ACCEPT, AUTHORIZATION, CONTENT_TYPE])
        .allow_credentials(true)
        .allow_origin(
            allowed_origins
                .iter()
                .filter_map(|origin| origin.parse::<HeaderValue>().ok())
                .collect::<Vec<_>>(),
        )
}

pub fn openapi_json() -> String {
    ApiDoc::openapi()
        .to_pretty_json()
        .expect("serialize OpenAPI")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{
            Request, StatusCode,
            header::{
                ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
                ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_REQUEST_HEADERS,
                ACCESS_CONTROL_REQUEST_METHOD, ORIGIN,
            },
        },
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn cors_preflight_supports_credentials_with_explicit_headers() {
        let app = Router::new()
            .route("/health", get(|| async { "ok" }))
            .layer(cors_layer(&["http://localhost:23802".to_string()]));

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::OPTIONS)
                    .uri("/health")
                    .header(ORIGIN, "http://localhost:23802")
                    .header(ACCESS_CONTROL_REQUEST_METHOD, "GET")
                    .header(ACCESS_CONTROL_REQUEST_HEADERS, "authorization,content-type")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_CREDENTIALS),
            Some(&HeaderValue::from_static("true"))
        );
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
            Some(&HeaderValue::from_static("http://localhost:23802"))
        );
        let allow_headers = response
            .headers()
            .get(ACCESS_CONTROL_ALLOW_HEADERS)
            .expect("allow headers")
            .to_str()
            .expect("allow headers value");
        assert!(allow_headers.contains("authorization"));
        assert!(allow_headers.contains("content-type"));
    }
}
