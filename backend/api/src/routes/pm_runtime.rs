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
    ctx.repo.observe_pm_run(id, status).await?;
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
        checkpoint_ref: reservation.checkpoint_ref,
        fence: reservation.fence,
    }))
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
