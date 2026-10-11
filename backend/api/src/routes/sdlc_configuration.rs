use app::AppContext;
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, header},
};
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use shared::{AppError, config::SdlcConfig};
use std::{collections::BTreeSet, sync::Arc, time::Duration};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Serialize, ToSchema)]
pub struct ConfigurationObservation {
    pub contract_version: u8,
    pub observation_ref: Uuid,
    pub agent_id: Uuid,
    pub sdlc_role: domain::SdlcRole,
    pub effective_revision: i64,
    pub package: serde_json::Value,
    pub workflow_binding: domain::SdlcWorkflowBinding,
    pub observed_at: String,
    pub managed_files_verified: bool,
    pub runtime_ready: bool,
    pub blockers: Vec<String>,
}

fn observation(
    agent_id: Uuid,
    sdlc_role: domain::SdlcRole,
    revision: &domain::AgentConfigRevision,
    package: &serde_json::Value,
) -> Result<ConfigurationObservation, AppError> {
    let workflow_binding = revision
        .snapshot
        .config
        .config_json
        .get("fleet_sdlc_workflow_binding")
        .cloned()
        .ok_or_else(unavailable)?;
    Ok(ConfigurationObservation {
        contract_version: 1,
        observation_ref: Uuid::new_v4(),
        agent_id,
        sdlc_role,
        effective_revision: revision.revision,
        package: package.clone(),
        workflow_binding: serde_json::from_value(workflow_binding).map_err(|_| unavailable())?,
        observed_at: shared::now().to_rfc3339(),
        managed_files_verified: true,
        runtime_ready: false,
        blockers: vec![
            "runtime_skill_inventory_not_verified".into(),
            "workflow_assignment_protocol_not_verified".into(),
        ],
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Principal {
    sub: String,
    email: String,
    scopes: Vec<String>,
    #[serde(rename = "display_name")]
    _display_name: String,
}

fn unavailable() -> AppError {
    AppError::Unavailable("SDLC configuration observation is unavailable".into())
}

fn authority(config: &SdlcConfig) -> Result<Url, AppError> {
    let raw = &config.auth_url;
    let subject =
        Uuid::parse_str(&config.configuration_reader_subject).map_err(|_| unavailable())?;
    if !config.configuration_readback_enabled
        || subject.is_nil()
        || subject.to_string() != config.configuration_reader_subject
        || raw.is_empty()
        || raw.len() > 512
        || raw.contains('\\')
        || raw.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err(unavailable());
    }
    registered_agents(config)?;
    let mut url = Url::parse(raw).map_err(|_| unavailable())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || url.as_str().trim_end_matches('/') != raw.trim_end_matches('/')
    {
        return Err(unavailable());
    }
    url.set_path("/auth/tokens/introspect");
    Ok(url)
}

fn registered_agents(config: &SdlcConfig) -> Result<BTreeSet<Uuid>, AppError> {
    let raw = &config.configuration_reader_agent_ids;
    if raw.is_empty() || raw.len() > 4096 {
        return Err(unavailable());
    }
    let mut agents = BTreeSet::new();
    for value in raw.split(',') {
        let id = Uuid::parse_str(value).map_err(|_| unavailable())?;
        if id.is_nil() || id.to_string() != value || !agents.insert(id) {
            return Err(unavailable());
        }
    }
    Ok(agents)
}

/// Use Base's fresh PAT introspection contract, never browser/local role fallback.
async fn authorize(config: &SdlcConfig, id: Uuid, headers: &HeaderMap) -> Result<(), AppError> {
    let url = authority(config)?;
    if headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return Err(AppError::Unauthorized);
    }
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| {
            value.starts_with("sdlc_pat_")
                && (41..=136).contains(&value.len())
                && value.bytes().all(|b| b.is_ascii_graphic())
        })
        .ok_or(AppError::Unauthorized)?;
    let mut authorization = header::HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|_| AppError::Unauthorized)?;
    authorization.set_sensitive(true);
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .no_proxy()
        .build()
        .map_err(|_| unavailable())?;
    let mut response = client
        .get(url)
        .header(header::AUTHORIZATION, authorization)
        .header(header::CACHE_CONTROL, "no-cache, no-store")
        .header(header::ACCEPT_ENCODING, "identity")
        .send()
        .await
        .map_err(|_| unavailable())?;
    if response.status() == StatusCode::UNAUTHORIZED {
        return Err(AppError::Unauthorized);
    }
    if response.status() != StatusCode::OK
        || response
            .headers()
            .get(header::CONTENT_ENCODING)
            .is_some_and(|value| value != "identity")
        || response
            .content_length()
            .is_some_and(|length| length > 16_384)
    {
        return Err(unavailable());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
        if body.len().saturating_add(chunk.len()) > 16_384 {
            return Err(unavailable());
        }
        body.extend_from_slice(&chunk);
    }
    let principal: Principal = serde_json::from_slice(&body).map_err(|_| unavailable())?;
    let expected = BTreeSet::from(["fleet-control:read".to_string()]);
    let scopes: BTreeSet<_> = principal.scopes.iter().cloned().collect();
    if principal.sub != config.configuration_reader_subject
        || principal.email.trim().is_empty()
        || scopes != expected
        || scopes.len() != principal.scopes.len()
        || !registered_agents(config)?.contains(&id)
    {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

#[utoipa::path(get, path = "/internal/runtime/v1/agents/{agent_id}/configuration", operation_id = "read_sdlc_configuration", tag = "sdlc-runtime", params(("agent_id" = Uuid, Path)), responses((status = 200, body = ConfigurationObservation), (status = 401), (status = 403), (status = 404), (status = 409), (status = 503)))]
pub async fn readback(
    State(ctx): State<Arc<AppContext>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Result<(HeaderMap, Json<ConfigurationObservation>), AppError> {
    authorize(&ctx.config.sdlc, id, &headers).await?;
    let agent = ctx.repo.get_agent(id).await?;
    let revision = ctx
        .repo
        .get_effective_config_revision(id)
        .await?
        .ok_or_else(|| AppError::conflict("SDLC effective configuration is missing"))?;
    if revision.draining {
        return Err(AppError::conflict("SDLC configuration is draining"));
    }
    let role = agent
        .sdlc_role
        .ok_or_else(|| AppError::conflict("SDLC role is not assigned"))?;
    let package = revision
        .snapshot
        .config
        .config_json
        .get("fleet_sdlc_package")
        .ok_or_else(|| AppError::conflict("SDLC pinned package is not applied"))?;
    // Both checks are needed: a repository implementation and a provisioner fail
    // closed independently, and a client-editable package field grants no rights.
    ctx.repo
        .verify_base_package_revision(
            id,
            revision.revision,
            &ctx.config.fleet.base_package_checkout,
        )
        .await
        .map_err(|_| unavailable())?;
    ctx.provisioner
        .verify_effective_configuration(&agent, &ctx.config, &revision)
        .await
        .map_err(|_| unavailable())?;
    app::sdlc_workflow::verify_revision_binding(
        &ctx.config.sdlc.workflow_binding,
        &agent,
        &revision,
    )
    .await
    .map_err(|_| unavailable())?;
    let current_agent = ctx.repo.get_agent(id).await?;
    let current = ctx.repo.get_effective_config_revision(id).await?;
    if current.as_ref().is_none_or(|value| value.draining)
        || serde_json::to_value(&current_agent).map_err(|_| unavailable())?
            != serde_json::to_value(&agent).map_err(|_| unavailable())?
        || current
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(|_| unavailable())?
            != Some(serde_json::to_value(&revision).map_err(|_| unavailable())?)
    {
        return Err(AppError::conflict(
            "SDLC configuration changed during observation",
        ));
    }
    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    Ok((
        response_headers,
        Json(observation(id, role, &revision, package)?),
    ))
}

#[cfg(test)]
#[path = "sdlc_configuration_tests.rs"]
mod tests;
