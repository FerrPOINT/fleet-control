use app::AppContext;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    middleware::{from_fn, from_fn_with_state},
    routing::{get, patch, post, put},
};
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub mod middleware;
pub mod routes;

#[derive(OpenApi)]
#[openapi(
    paths(
        routes::health::health,
        routes::pm_runtime::readback,
        routes::sdlc_configuration::readback,
        routes::chats_directory::directory,
        routes::approvals::list,
        routes::approvals::read,
        routes::approvals::decide,
        routes::auth::register,
        routes::auth::login,
        routes::auth::refresh_openapi,
        routes::auth::logout_openapi,
        routes::users::get_me,
        routes::users::get_permissions,
        routes::users::list_users,
        routes::users::update_user_role,
        routes::dashboard::get_dashboard,
        routes::agents::list_agent_directory,
        routes::agents::list_agents,
        routes::agents::get_agent_storage_review,
        routes::agents::create_agent,
        routes::agents::get_agent,
        routes::agents::get_agent_storage,
        routes::agents::update_agent,
        routes::agents::archive_agent,
        routes::agents::purge_agent_files,
        routes::agents::provision_agent,
        routes::agents::start_agent,
        routes::agents::stop_agent,
        routes::agents::restart_agent,
        routes::agents::agent_health,
        routes::agents::list_fleet_alerts,
        routes::agents::acknowledge_fleet_alert,
        routes::agents::get_agent_config,
        routes::agents::update_agent_config,
        routes::agents::list_config_revisions,
        routes::agents::validate_config_revision,
        routes::agents::prepare_base_package,
        routes::agents::activate_config_revision,
        routes::agents::get_sdlc_readiness,
        routes::agents::list_agent_skills,
        routes::agents::update_agent_skill,
        routes::leaders::list_leaders,
        routes::leaders::list_leader_executors,
        routes::leaders::update_leader_executors,
        routes::executors::list_executors,
        routes::sessions::list_sessions,
        routes::sessions::create_session,
        routes::sessions::get_session,
        routes::sessions::handoff_session,
        routes::sessions::assign_session_leader,
        routes::sessions::list_session_messages,
        routes::sessions::create_session_message,
        routes::sessions::list_session_participants,
        routes::sessions::create_session_delegation,
        routes::sessions::list_session_agent_runs,
        routes::sessions::stream_session,
        routes::sessions::steer_session_run,
        routes::sessions::stop_session_run,
        routes::sessions::list_controls,
        routes::sessions::lookup_control,
        routes::sessions::read_control,
        routes::sessions::resolve_session_run_approval,
        routes::task_chats::bind_task_chat,
        routes::task_chats::task_context,
        routes::task_chats::clarifications,
        routes::task_chats::answer,
        routes::clarification_commands::store,
        routes::clarification_commands::pending,
        routes::clarification_commands::get,
        routes::clarification_commands::delivery,
        routes::task_chats::requirements,
        routes::task_chats::confirm,
        routes::task_chats::history,
        routes::task_chats::controls,
        routes::pm_drafts::create,
        routes::pm_drafts::projects,
        routes::pm_drafts::read,
        routes::pm_drafts::read_by_key,
        routes::pm_drafts::continue_operation,
        routes::workflows::list_workflow_bindings,
        routes::workflows::get_workflow_catalog,
        routes::workflows::rebind_workflow_binding,
        routes::deployments::list_runtime_templates,
        routes::deployments::list_deployment_jobs,
        routes::deployments::get_deployment_job,
        routes::deployments::create_deployment_job,
        routes::deployments::bulk_create_deployment_jobs,
        routes::deployments::cancel_deployment_job,
        routes::settings::get_runtime_settings,
        routes::settings::update_runtime_settings,
        routes::settings::get_port_settings,
        routes::settings::update_port_settings,
        routes::settings::get_integration_settings,
        routes::settings::update_integration_settings,
        routes::settings::get_auth_settings,
        routes::settings::update_auth_settings,
        routes::settings::get_managed_settings,
        routes::settings::preview_managed_settings,
        routes::settings::apply_managed_settings,
        routes::settings::list_managed_settings_versions,
        routes::settings::rollback_managed_settings,
        routes::settings::run_retention_review,
        routes::logs::list_logs,
        routes::logs::list_audit_log,
        routes::events::recent_events,
        routes::events::events,
    ),
    components(schemas(
        routes::settings::RetentionReviewOutcomeDto,
        domain::Agent,
        domain::SystemRole,
        domain::ApprovalChoice,
        domain::ApprovalDecisionState,
        domain::ApprovalDecisionRequest,
        domain::ApprovalDecision,
        domain::AgentKind,
        domain::AgentProductRole,
        domain::AgentRole,
        domain::SdlcRole,
        domain::AgentConfigRevision,
        domain::AgentConfigurationSnapshot,
        domain::AgentSdlcReadiness,
        domain::SdlcWorkflowBinding,
        domain::SessionEvent,
        domain::TaskChatBinding,
        domain::CreatePmDraftRequest,
        routes::pm_drafts::PmDraftOperationQuery,
        routes::pm_drafts::ContinuePmDraftRequest,
        routes::pm_drafts::PmDraftProjectDirectory,
        routes::pm_drafts::PmDraftProject,
        domain::PmDraftCreationResponse,
        domain::PmDraftCreationState,
        domain::PmDraftCreationStep,
        domain::BindTaskChatRequest,
        domain::SessionTaskContext,
        domain::TrackerTaskContext,
        domain::TrackerPermissions,
        domain::TrackerPmAssignment,
        domain::TrackerStage,
        domain::TrackerQuestionMode,
        domain::TrackerQuestionState,
        domain::TrackerQuestionOption,
        domain::TrackerQuestion,
        domain::TrackerAnswer,
        domain::TrackerRequirementsRevision,
        domain::TrackerConfirmation,
        domain::TrackerClarifications,
        domain::TrackerRequirements,
        domain::ClarificationAnswerRequest,
        domain::ConfirmRequirementsRequest,
        domain::MessageHistoryPage,
        domain::ChatControls,
        domain::PmExecutionIdentity,
        domain::PmRuntimeStatus,
        domain::PmRuntimeObservation,
        domain::ChatsDirectoryAgent,
        domain::ChatsDirectoryPage,
        domain::AgentStatus,
        domain::DesiredState,
        domain::SkillState,
        domain::SessionState,
        domain::SessionVisibility,
        domain::SessionParticipantType,
        domain::SessionRole,
        domain::MessageAuthorType,
        domain::MessageKind,
        domain::MessageDeliveryState,
        domain::SessionRunRole,
        domain::SessionRunState,
        domain::RuntimeApprovalState,
        domain::AgentPaths,
        domain::AgentRuntime,
        domain::AgentDirectoryItem,
        domain::AgentConfig,
        domain::AgentSkill,
        domain::AgentSession,
        domain::LeaderExecutor,
        domain::SessionParticipant,
        domain::SessionMessage,
        domain::SessionAgentRun,
        domain::RuntimeApprovalRequest,
        domain::AuditLogEntry,
        domain::AgentStorageArea,
        domain::AgentStorageReport,
        domain::AgentStorageReview,
        domain::AgentStorageReviewItem,
        domain::AgentRetentionReport,
        domain::PurgeAgentFilesRequest,
        domain::PurgeAgentFilesResponse,
        domain::DeploymentJobKind,
        domain::DeploymentJobState,
        domain::DeploymentJob,
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
        domain::UserPermissionsResponse,
        domain::UpdateUserRoleRequest,
        domain::CreateAgentRequest,
        domain::UpdateAgentRequest,
        domain::UpdateAgentConfigRequest,
        domain::UpdateSkillRequest,
        domain::CreateSessionRequest,
        domain::CreateSessionDelegationRequest,
        domain::HandoffSessionRequest,
        domain::AssignSessionLeaderRequest,
        domain::CreateSessionMessageRequest,
        domain::SteerSessionRunRequest,
        domain::ResolveRuntimeApprovalRequest,
        domain::RuntimeRunControlResponse,
        domain::RuntimeControlReceipt,
        domain::ClarificationAnswerCommand,
        domain::ClarificationDeliveryState,
        domain::RuntimeControlOperation,
        domain::RuntimeControlState,
        domain::UpdateLeaderExecutorsRequest,
        domain::CreateDeploymentJobRequest,
        domain::BulkDeploymentRequest,
        domain::BulkDeploymentResult,
        domain::RuntimeSettings,
        domain::PortSettings,
        domain::IntegrationSettings,
        domain::AuthSettings,
        domain::ManagedPortSettings,
        domain::ManagedIntegrationSettings,
        domain::ManagedRetentionSettings,
        domain::ManagedSettingsSnapshot,
        domain::ManagedSettingsVersion,
        domain::ManagedSettingsChange,
        domain::ManagedSettingsPreviewRequest,
        domain::ManagedSettingsState,
        domain::ManagedSettingsPreview,
        domain::ApplyManagedSettingsRequest,
        domain::RollbackManagedSettingsRequest,
        domain::ApplyManagedSettingsResponse,
        domain::RuntimeOperationResponse,
    )),
    tags(
        (name = "auth", description = "Authentication"),
        (name = "agents", description = "Agent fleet management"),
        (name = "leaders", description = "Leader agents and managed executor teams"),
        (name = "executors", description = "Executor agent inventory"),
        (name = "sessions", description = "Cross-agent task sessions"),
        (name = "runtime", description = "Runtime templates and deployments"),
        (name = "settings", description = "Fleet Control settings")
    )
)]
pub struct ApiDoc;

pub fn router(ctx: Arc<AppContext>) -> Router<Arc<AppContext>> {
    let cors = cors_layer(&ctx.config.server.cors_allowed_origins);

    let protected = Router::new()
        .route("/api/v1/users/me", get(routes::users::get_me))
        .route(
            "/api/v1/users/me/permissions",
            get(routes::users::get_permissions),
        )
        .route("/api/v1/users", get(routes::users::list_users))
        .route(
            "/api/v1/users/{user_id}/role",
            patch(routes::users::update_user_role),
        )
        .route("/api/v1/dashboard", get(routes::dashboard::get_dashboard))
        .route(
            "/api/v1/agent-directory",
            get(routes::agents::list_agent_directory),
        )
        .route(
            "/api/v1/agents",
            get(routes::agents::list_agents).post(routes::agents::create_agent),
        )
        .route(
            "/api/v1/agents/storage-review",
            get(routes::agents::get_agent_storage_review),
        )
        .route(
            "/api/v1/agents/{agent_id}",
            get(routes::agents::get_agent)
                .patch(routes::agents::update_agent)
                .delete(routes::agents::archive_agent),
        )
        .route(
            "/api/v1/agents/{agent_id}/storage",
            get(routes::agents::get_agent_storage),
        )
        .route(
            "/api/v1/agents/{agent_id}/purge-files",
            post(routes::agents::purge_agent_files),
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
            "/api/v1/fleet-alerts",
            get(routes::agents::list_fleet_alerts),
        )
        .route(
            "/api/v1/fleet-alerts/{alert_id}/acknowledge",
            post(routes::agents::acknowledge_fleet_alert),
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
            "/api/v1/agents/{agent_id}/config/revisions",
            get(routes::agents::list_config_revisions),
        )
        .route(
            "/api/v1/agents/{agent_id}/config/base-package",
            post(routes::agents::prepare_base_package),
        )
        .route(
            "/api/v1/agents/{agent_id}/config/revisions/{revision}/validate",
            post(routes::agents::validate_config_revision),
        )
        .route(
            "/api/v1/agents/{agent_id}/config/revisions/{revision}/activate",
            post(routes::agents::activate_config_revision),
        )
        .route(
            "/api/v1/agents/{agent_id}/readiness",
            get(routes::agents::get_sdlc_readiness),
        )
        .route(
            "/api/v1/agents/{agent_id}/skills/{skill_name}",
            put(routes::agents::update_agent_skill),
        )
        .route("/api/v1/leaders", get(routes::leaders::list_leaders))
        .route(
            "/api/v1/leaders/{leader_agent_id}/executors",
            get(routes::leaders::list_leader_executors)
                .put(routes::leaders::update_leader_executors),
        )
        .route("/api/v1/executors", get(routes::executors::list_executors))
        .route(
            "/api/v1/chats/directory",
            get(routes::chats_directory::directory),
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
            "/api/v1/sessions/{session_id}/messages",
            get(routes::sessions::list_session_messages)
                .post(routes::sessions::create_session_message),
        )
        .route(
            "/api/v1/projects/{project_id}/pm-drafts",
            post(routes::pm_drafts::create),
        )
        .route(
            "/api/v1/pm-drafts/projects",
            get(routes::pm_drafts::projects),
        )
        .route(
            "/api/v1/pm-drafts/operations/{operation_id}",
            get(routes::pm_drafts::read),
        )
        .route(
            "/api/v1/projects/{project_id}/pm-drafts/operation",
            get(routes::pm_drafts::read_by_key),
        )
        .route(
            "/api/v1/pm-drafts/operations/{operation_id}/continue",
            post(routes::pm_drafts::continue_operation),
        )
        .route(
            "/api/v1/sessions/{session_id}/task-binding",
            post(routes::task_chats::bind_task_chat),
        )
        .route(
            "/api/v1/sessions/{session_id}/task-context",
            get(routes::task_chats::task_context),
        )
        .route(
            "/api/v1/sessions/{session_id}/clarifications",
            get(routes::task_chats::clarifications),
        )
        .route(
            "/api/v1/sessions/{session_id}/clarifications/{question_id}/answers",
            post(routes::task_chats::answer),
        )
        .route(
            "/api/v1/sessions/{session_id}/clarifications/{question_id}/answer-commands",
            post(routes::clarification_commands::store),
        )
        .route(
            "/api/v1/sessions/{session_id}/clarification-answer-commands",
            get(routes::clarification_commands::pending),
        )
        .route(
            "/api/v1/sessions/{session_id}/clarification-answer-commands/{command_id}",
            get(routes::clarification_commands::get),
        )
        .route(
            "/api/v1/sessions/{session_id}/clarification-answer-commands/{command_id}/delivery",
            post(routes::clarification_commands::delivery),
        )
        .route(
            "/api/v1/sessions/{session_id}/requirements",
            get(routes::task_chats::requirements),
        )
        .route(
            "/api/v1/sessions/{session_id}/requirements/{revision}/confirm",
            post(routes::task_chats::confirm),
        )
        .route(
            "/api/v1/sessions/{session_id}/history",
            get(routes::task_chats::history),
        )
        .route(
            "/api/v1/sessions/{session_id}/chat-controls",
            get(routes::task_chats::controls),
        )
        .route(
            "/api/v1/sessions/{session_id}/participants",
            get(routes::sessions::list_session_participants),
        )
        .route(
            "/api/v1/sessions/{session_id}/delegations",
            post(routes::sessions::create_session_delegation),
        )
        .route(
            "/api/v1/sessions/{session_id}/leader",
            put(routes::sessions::assign_session_leader),
        )
        .route(
            "/api/v1/sessions/{session_id}/handoff",
            post(routes::sessions::handoff_session),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs",
            get(routes::sessions::list_session_agent_runs),
        )
        .route(
            "/api/v1/sessions/{session_id}/stream",
            get(routes::sessions::stream_session),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/steer",
            post(routes::sessions::steer_session_run),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/stop",
            post(routes::sessions::stop_session_run),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/controls",
            get(routes::sessions::list_controls),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/controls/lookup",
            get(routes::sessions::lookup_control),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/controls/{command_id}",
            get(routes::sessions::read_control),
        )
        .route(
            "/api/v1/sessions/{session_id}/runs/{run_id}/approval",
            post(routes::sessions::resolve_session_run_approval),
        )
        .route(
            "/api/v1/sessions/{session_id}/approvals",
            get(routes::approvals::list),
        )
        .route(
            "/api/v1/sessions/{session_id}/approvals/{approval_id}/decision",
            get(routes::approvals::read).post(routes::approvals::decide),
        )
        .route(
            "/api/v1/workflow-bindings",
            get(routes::workflows::list_workflow_bindings),
        )
        .route(
            "/api/v1/workflow-catalog",
            get(routes::workflows::get_workflow_catalog),
        )
        .route(
            "/api/v1/workflow-bindings/{agent_id}",
            put(routes::workflows::rebind_workflow_binding),
        )
        .route(
            "/api/v1/runtime-templates",
            get(routes::deployments::list_runtime_templates),
        )
        .route(
            "/api/v1/deployments/jobs",
            get(routes::deployments::list_deployment_jobs)
                .post(routes::deployments::create_deployment_job),
        )
        .route(
            "/api/v1/deployments/jobs/bulk",
            post(routes::deployments::bulk_create_deployment_jobs),
        )
        .route(
            "/api/v1/deployments/jobs/{job_id}",
            get(routes::deployments::get_deployment_job),
        )
        .route(
            "/api/v1/deployments/jobs/{job_id}/cancel",
            post(routes::deployments::cancel_deployment_job),
        )
        .route(
            "/api/v1/settings/runtime",
            get(routes::settings::get_runtime_settings)
                .put(routes::settings::update_runtime_settings),
        )
        .route(
            "/api/v1/settings/ports",
            get(routes::settings::get_port_settings).put(routes::settings::update_port_settings),
        )
        .route(
            "/api/v1/settings/integrations",
            get(routes::settings::get_integration_settings)
                .put(routes::settings::update_integration_settings),
        )
        .route(
            "/api/v1/settings/auth",
            get(routes::settings::get_auth_settings).put(routes::settings::update_auth_settings),
        )
        .route(
            "/api/v1/settings/managed",
            get(routes::settings::get_managed_settings),
        )
        .route(
            "/api/v1/settings/managed/preview",
            post(routes::settings::preview_managed_settings),
        )
        .route(
            "/api/v1/settings/managed/apply",
            post(routes::settings::apply_managed_settings),
        )
        .route(
            "/api/v1/settings/managed/versions",
            get(routes::settings::list_managed_settings_versions),
        )
        .route(
            "/api/v1/settings/managed/versions/{version}/rollback",
            post(routes::settings::rollback_managed_settings),
        )
        .route(
            "/settings/retention/review",
            post(routes::settings::run_retention_review),
        )
        .route("/api/v1/logs", get(routes::logs::list_logs))
        .route("/api/v1/audit-log", get(routes::logs::list_audit_log))
        .route("/api/v1/events/recent", get(routes::events::recent_events))
        .route("/api/v1/events", get(routes::events::events))
        .route_layer(from_fn_with_state(ctx.clone(), middleware::require_auth));

    let (metric_layer, metric_handle) = axum_prometheus::PrometheusMetricLayer::pair();
    Router::new()
        .route(
            "/metrics",
            get(move || {
                let handle = metric_handle.clone();
                async move { handle.render() }
            }),
        )
        .route("/health", get(routes::health::health))
        .route("/api/v1/health", get(routes::health::health))
        .route(
            "/internal/runtime/v1/pm/runs/{session_run_id}",
            get(routes::pm_runtime::readback),
        )
        .route(
            "/internal/runtime/v1/pm/agents/{agent_id}/mcp",
            post(routes::pm_tools::handle).layer(DefaultBodyLimit::max(262144)),
        )
        .route(
            "/internal/runtime/v1/agents/{agent_id}/configuration",
            get(routes::sdlc_configuration::readback),
        )
        .route("/api/v1/auth/register", post(routes::auth::register))
        .route("/api/v1/auth/login", post(routes::auth::login))
        .route("/api/v1/auth/refresh", post(routes::auth::refresh))
        .route("/api/v1/auth/logout", post(routes::auth::logout))
        .merge(protected)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(metric_layer)
        .layer(cors)
        .layer(from_fn(shared::telemetry::request_id_mw))
}

fn cors_layer(allowed_origins: &[String]) -> tower_http::cors::CorsLayer {
    sdlc_shared::cors::cors_layer(allowed_origins)
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
            HeaderValue, Method, Request, StatusCode,
            header::{
                ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
                ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_REQUEST_HEADERS,
                ACCESS_CONTROL_REQUEST_METHOD, ORIGIN,
            },
        },
    };
    use tower::ServiceExt;

    #[test]
    fn runtime_control_lookup_openapi_requires_key_and_returns_one_receipt() {
        let spec: serde_json::Value = serde_json::from_str(&openapi_json()).unwrap();
        let collection = "/api/v1/sessions/{session_id}/runs/{run_id}/controls";
        let operation = &spec["paths"][format!("{collection}/lookup")]["get"];
        let keys: Vec<_> = operation["parameters"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|parameter| parameter["name"] == "Idempotency-Key")
            .collect();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0]["in"], "header");
        assert_eq!(keys[0]["required"], true);
        assert_eq!(keys[0]["schema"]["type"], "string");
        assert_eq!(
            operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/RuntimeControlReceipt"
        );
        assert!(operation["responses"]["404"].is_object());
        assert_eq!(
            spec["paths"][collection]["get"]["parameters"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            spec["paths"][collection]["get"]["responses"]["200"]["content"]["application/json"]["schema"]
                ["type"],
            "array"
        );
    }

    #[tokio::test]
    async fn runtime_control_lookup_literal_route_cannot_be_an_old_uuid_receipt() {
        let uuid_route = "/controls/{command_id}";
        let old = Router::new().route(
            uuid_route,
            get(|axum::extract::Path(_): axum::extract::Path<uuid::Uuid>| async { "receipt" }),
        );
        let request = || {
            Request::builder()
                .uri("/controls/lookup")
                .body(Body::empty())
                .unwrap()
        };
        assert_eq!(
            old.clone().oneshot(request()).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
        let current = old.route("/controls/lookup", get(|| async { "lookup" }));
        let response = current.oneshot(request()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            axum::body::to_bytes(response.into_body(), 64)
                .await
                .unwrap(),
            "lookup"
        );
    }

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
                    .header(
                        ACCESS_CONTROL_REQUEST_HEADERS,
                        "authorization,content-type,last-event-id,idempotency-key",
                    )
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
        assert!(allow_headers.contains("last-event-id"));
        assert!(allow_headers.contains("idempotency-key"));
    }

    #[tokio::test]
    async fn metrics_endpoint_exposes_prometheus_counters() {
        // M1: the /metrics route pattern used by router() must render the
        // Prometheus exposition after the metric layer recorded a request.
        let (metric_layer, metric_handle) = axum_prometheus::PrometheusMetricLayer::pair();
        let app = Router::new()
            .route(
                "/metrics",
                get(move || {
                    let handle = metric_handle.clone();
                    async move { handle.render() }
                }),
            )
            .route("/healthz", get(|| async { "ok" }))
            .layer(metric_layer);
        // generate one recorded request
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("warmup response");
        let metrics_response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("metrics response");
        assert_eq!(metrics_response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(metrics_response.into_body(), usize::MAX)
            .await
            .expect("body");
        let text = String::from_utf8_lossy(&body);
        assert!(
            text.contains("axum_http_requests_total"),
            "prometheus exposition must contain request counters, got: {text}"
        );
    }
}
