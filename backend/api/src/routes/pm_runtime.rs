use app::AppContext;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, header},
};
use domain::PmRuntimeObservation;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use shared::AppError;
use std::sync::Arc;
use uuid::Uuid;

fn authorize(headers: &HeaderMap, expected: &str) -> Result<(), AppError> {
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return Err(AppError::Unauthorized);
    }
    if expected.len() < 32
        || expected.len() > 512
        || !expected.bytes().all(|c| c.is_ascii_graphic())
    {
        return Err(AppError::Unavailable(
            "PM readback credential is not configured".into(),
        ));
    }
    let supplied = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .filter(|v| v.len() <= 512)
        .ok_or(AppError::Unauthorized)?;
    let mut expected_mac =
        Hmac::<Sha256>::new_from_slice(expected.as_bytes()).map_err(AppError::internal)?;
    let mut supplied_mac =
        Hmac::<Sha256>::new_from_slice(supplied.as_bytes()).map_err(AppError::internal)?;
    expected_mac.update(b"fleet-pm-readback-v1");
    supplied_mac.update(b"fleet-pm-readback-v1");
    expected_mac
        .verify_slice(&supplied_mac.finalize().into_bytes())
        .map_err(|_| AppError::Unauthorized)
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct PmNativeAdmissionResponse {
    pub ok: bool,
    pub allowed: bool,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PmNativeToolRequest {
    pub session_id: String,
    pub arguments: serde_json::Value,
}

#[utoipa::path(post,path="/internal/runtime/v1/pm/agents/{agent_id}/tools/{operation}",tag="pm-runtime",
    params(("agent_id"=Uuid,Path),("operation"=String,Path)),request_body=PmNativeToolRequest,
    responses((status=200),(status=401),(status=409),(status=503)))]
pub async fn tool(
    State(ctx): State<Arc<AppContext>>,
    Path((agent_id, operation)): Path<(Uuid, String)>,
    headers: HeaderMap,
    Json(request): Json<PmNativeToolRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !ctx.config.pm.workflow.enabled {
        return Err(AppError::Unavailable("PM tools are disabled".into()));
    }
    authorize(
        &headers,
        &app::agent_runtime_credential(&ctx.config, agent_id)?,
    )?;
    if !domain::valid_ref(&request.session_id, 512) {
        return Err(AppError::validation("native session context required"));
    }
    let args = request
        .arguments
        .as_object()
        .filter(|v| v.len() == 1)
        .and_then(|v| v.get("command"))
        .ok_or_else(|| AppError::validation("closed PM command envelope required"))?;
    let agent = ctx.repo.get_agent(agent_id).await?;
    let record = ctx
        .repo
        .find_pm_native_run(agent_id, &request.session_id)
        .await?
        .ok_or_else(|| AppError::Unavailable("PM native run not admitted".into()))?;
    let result = ctx
        .runtime
        .pm_native_tool(&agent, &record, &operation, args)
        .await?;
    Ok(Json(serde_json::json!({"ok":true,"result":result})))
}

#[utoipa::path(post,path="/internal/runtime/v1/pm/agents/{agent_id}/admit",tag="pm-runtime",
    params(("agent_id"=Uuid,Path)),request_body=domain::PmNativeAdmissionRequest,
    responses((status=200,body=PmNativeAdmissionResponse),(status=401),(status=409),(status=503)))]
pub async fn admit(
    State(ctx): State<Arc<AppContext>>,
    Path(agent_id): Path<Uuid>,
    headers: HeaderMap,
    Json(request): Json<domain::PmNativeAdmissionRequest>,
) -> Result<Json<PmNativeAdmissionResponse>, AppError> {
    if !ctx.config.pm.workflow.enabled || !ctx.config.pm.credentials.enabled {
        return Err(AppError::Unavailable(
            "PM native admission is not enabled".into(),
        ));
    }
    authorize(
        &headers,
        &app::agent_runtime_credential(&ctx.config, agent_id)?,
    )?;
    request.native_configuration.validate()?;
    if !domain::valid_ref(&request.session_id, 512) {
        return Err(AppError::validation("native session context required"));
    }
    let agent = ctx.repo.get_agent(agent_id).await?;
    if agent.kind != domain::AgentKind::Hermes
        || agent.sdlc_role != Some(domain::SdlcRole::ProjectManager)
    {
        return Err(AppError::Forbidden);
    }
    // A first native call can precede the accepted mapping. Wait for the existing
    // run event instead of polling or inventing an early permission receipt.
    let mut events = ctx.events.subscribe();
    let record = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            if let Some(record) = ctx
                .repo
                .find_pm_native_run(agent_id, &request.session_id)
                .await?
            {
                if ctx
                    .runtime
                    .admit_pm_native_configuration(&agent, &record, &request.native_configuration)
                    .await?
                {
                    return Ok::<_, AppError>(record);
                }
            }
            events
                .recv()
                .await
                .map_err(|_| AppError::Unavailable("PM admission event is unavailable".into()))?;
        }
    })
    .await
    .map_err(|_| AppError::Unavailable("PM native binding is pending".into()))??;
    let _ = record;
    Ok(Json(PmNativeAdmissionResponse {
        ok: true,
        allowed: true,
    }))
}

#[utoipa::path(get,path="/internal/runtime/v1/pm/runs/{session_run_id}",tag="pm-runtime",params(("session_run_id"=Uuid,Path)),responses((status=200,body=PmRuntimeObservation),(status=401),(status=409),(status=503)))]
pub async fn readback(
    State(ctx): State<Arc<AppContext>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<PmRuntimeObservation>, AppError> {
    if !separate_credential(&ctx.config) {
        return Err(AppError::Unavailable(
            "PM readback requires a separate credential".into(),
        ));
    }
    authorize(&headers, &ctx.config.pm.readback_token)?;
    let record = ctx.repo.get_pm_run(id).await?;
    record.reservation.validate()?;
    let binding = ctx
        .repo
        .get_task_chat_binding(record.reservation.session_id)
        .await?
        .ok_or_else(|| AppError::Unavailable("PM task binding is missing".into()))?;
    let identity = &record.reservation.identity;
    if binding.agent_id.to_string() != identity.agent_ref
        || binding.task_id.to_string() != identity.task_ref
        || binding.root_task_id.to_string() != identity.root_ref
        || binding.project_id.to_string() != identity.tracker_project_ref
        || binding.tracker_instance_id != identity.tracker_instance_ref
    {
        return Err(AppError::Unavailable(
            "PM task binding no longer matches execution".into(),
        ));
    }
    let agent = ctx.repo.get_agent(identity.agent_id()?).await?;
    let status = ctx.runtime.probe_pm_run(&agent, &record).await?;
    let reservation = record.reservation;
    Ok(Json(PmRuntimeObservation {
        identity: reservation.identity,
        observation_ref: Uuid::new_v4(),
        binding_ref: reservation.binding_ref,
        hermes_run_ref: record
            .hermes_run_ref
            .ok_or_else(|| AppError::Unavailable("PM runtime acceptance is unknown".into()))?,
        session_run_id: reservation.session_run_id,
        status,
        dispatch_operation_key: reservation.dispatch_operation_key,
        checkpoint_ref: if reservation.native_session_key.is_some() {
            ctx.repo
                .pm_checkpoint(reservation.session_run_id)
                .await?
                .filter(|journal| journal.receipt.is_some())
                .and_then(|journal| {
                    journal.command["checkpoint_ref"]
                        .as_str()
                        .map(str::to_owned)
                })
                .or(reservation.checkpoint_ref)
        } else {
            reservation.checkpoint_ref
        },
        fence: reservation.fence,
    }))
}

#[utoipa::path(get,path="/internal/runtime/v1/pm/executions/{execution_id}/admission",tag="pm-runtime",
    params(("execution_id"=Uuid,Path)),responses((status=200,body=domain::PmNativeAdmissionObservation),(status=401),(status=409),(status=503)))]
pub async fn execution_admission(
    State(ctx): State<Arc<AppContext>>,
    Path(execution): Path<Uuid>,
    headers: HeaderMap,
) -> Result<Json<domain::PmNativeAdmissionObservation>, AppError> {
    if execution.is_nil() || !separate_credential(&ctx.config) {
        return Err(AppError::Unavailable(
            "PM admission reader is not configured".into(),
        ));
    }
    authorize(&headers, &ctx.config.pm.readback_token)?;
    let record = ctx.repo.current_pm_execution_run(execution).await?;
    record.reservation.validate()?;
    let agent = ctx
        .repo
        .get_agent(record.reservation.identity.agent_id()?)
        .await?;
    Ok(Json(
        ctx.runtime
            .observe_pm_native_admission(&agent, &record)
            .await?,
    ))
}

fn separate_credential(config: &shared::AppConfig) -> bool {
    config.pm.readback_token != config.fleet.runtime_token_secret
        && config.pm.readback_token != config.auth.jwt_secret
        && config.pm.readback_token != config.pm.namespace_read_pat
        && config.fleet.project_workflow_catalog_token.as_deref()
            != Some(config.pm.readback_token.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_authorization_requires_exactly_one_bearer_header() {
        let expected = "native-test-only-credential-123456789";
        let mut headers = HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {expected}").parse().unwrap(),
        );
        authorize(&headers, expected).unwrap();
        headers.append(header::AUTHORIZATION, "Bearer foreign".parse().unwrap());
        assert!(matches!(
            authorize(&headers, expected),
            Err(AppError::Unauthorized)
        ));
    }
    #[test]
    fn callback_requires_a_distinct_configured_machine_credential() {
        let secret = "readback-test-only-credential-123456789";
        let mut config = shared::AppConfig::default();
        config.pm.readback_token = secret.into();
        assert!(separate_credential(&config));
        config.pm.namespace_read_pat = secret.into();
        assert!(!separate_credential(&config));
        config.pm.namespace_read_pat.clear();
        assert!(separate_credential(&config));
        config.fleet.project_workflow_catalog_token = Some(secret.into());
        assert!(!separate_credential(&config));
        config.fleet.project_workflow_catalog_token = None;
        config.auth.jwt_secret = secret.into();
        assert!(!separate_credential(&config));
        config.auth.jwt_secret.clear();
        config.fleet.runtime_token_secret = secret.into();
        assert!(!separate_credential(&config));
        let mut headers = HeaderMap::new();
        assert!(authorize(&headers, "").is_err());
        assert!(authorize(&headers, secret).is_err());
        headers.insert(
            header::AUTHORIZATION,
            format!("Bearer {secret}").parse().unwrap(),
        );
        assert!(authorize(&headers, secret).is_ok());
        assert!(authorize(&headers, "other-test-only-credential-123456789").is_err());
        headers.insert(
            header::AUTHORIZATION,
            format!("bearer {secret}").parse().unwrap(),
        );
        assert!(authorize(&headers, secret).is_err());
    }
}
