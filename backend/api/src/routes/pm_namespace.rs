use domain::{Agent, PmDraftOperation};
use reqwest::{StatusCode, Url, header};
use serde::{Deserialize, Serialize};
use shared::{AppConfig, AppError};
use std::time::Duration;
use uuid::Uuid;

const MAX_BODY: usize = 16 * 1024;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Readback {
    ok: bool,
    result: Ownership,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Ownership {
    contract_version: u8,
    ownership_ref: String,
    namespace_id: i64,
    tracker_instance_ref: String,
    tracker_project_ref: String,
    authority_issuer: String,
    provisioner_subject: String,
    created_at: String,
}

fn unavailable() -> AppError {
    AppError::Unavailable("Project Workflow namespace ownership is unavailable or invalid".into())
}

fn canonical_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|id| !id.is_nil() && id.to_string() == value)
}

fn origin(raw: &str) -> Result<Url, AppError> {
    let url = root(raw)?;
    if url.as_str().trim_end_matches('/') != raw.trim_end_matches('/') {
        return Err(unavailable());
    }
    Ok(url)
}

fn root(raw: &str) -> Result<Url, AppError> {
    if raw.len() > 512
        || raw.trim() != raw
        || raw.contains('\\')
        || raw.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err(unavailable());
    }
    let url = Url::parse(raw).map_err(|_| unavailable())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(unavailable());
    }
    Ok(url)
}

fn issuer(raw: &str) -> Result<(), AppError> {
    root(raw)?;
    // Workflow retains the exact authority spelling, including explicit default ports.
    if raw
        .split_once("://")
        .is_none_or(|(_, authority)| authority.contains('/'))
    {
        return Err(unavailable());
    }
    Ok(())
}

fn namespace(value: Option<&str>) -> Result<i64, AppError> {
    let raw = value.ok_or_else(|| {
        AppError::conflict("PM agent requires a provisioned Project Workflow namespace")
    })?;
    let id = raw.parse::<i64>().ok().filter(|id| *id > 0);
    match id {
        Some(id) if id.to_string() == raw => Ok(id),
        _ => Err(AppError::conflict(
            "PM namespace ID must be a canonical positive integer",
        )),
    }
}

fn validate(
    value: serde_json::Value,
    config: &AppConfig,
    operation: &PmDraftOperation,
    namespace_id: i64,
) -> Result<(), AppError> {
    let readback: Readback = serde_json::from_value(value.clone()).map_err(|_| unavailable())?;
    if serde_json::to_value(&readback).map_err(|_| unavailable())? != value
        || !readback.ok
        || readback.result.contract_version != 1
        || !canonical_uuid(&readback.result.ownership_ref)
        || !canonical_uuid(&readback.result.tracker_project_ref)
        || !canonical_uuid(&readback.result.provisioner_subject)
        || readback.result.namespace_id != namespace_id
        || readback.result.authority_issuer != config.pm.namespace_authority_issuer
        || readback.result.provisioner_subject != config.pm.namespace_provisioner_subject
        || chrono::DateTime::parse_from_rfc3339(&readback.result.created_at).is_err()
    {
        return Err(unavailable());
    }
    if readback.result.tracker_instance_ref != operation.tracker_instance_id
        || readback.result.tracker_project_ref != operation.project_id.to_string()
    {
        return Err(AppError::conflict(
            "PM namespace belongs to another Tracker project",
        ));
    }
    Ok(())
}

/// Workflow revalidates the dedicated machine PAT at Base on every read.
/// This guard deliberately issues no bind, admission, workspace or runtime command.
pub(super) async fn verify(
    config: &AppConfig,
    operation: &PmDraftOperation,
    agent: &Agent,
) -> Result<(), AppError> {
    let namespace_id = namespace(agent.namespace_id.as_deref())?;
    if agent.id != operation.request.agent_id {
        return Err(unavailable());
    }
    read(config, operation, namespace_id).await
}

async fn read(
    config: &AppConfig,
    operation: &PmDraftOperation,
    namespace_id: i64,
) -> Result<(), AppError> {
    if namespace_id <= 0
        || operation.tracker_instance_id.is_empty()
        || operation.tracker_instance_id.len() > 128
        || operation
            .tracker_instance_id
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
        || operation.project_id.is_nil()
        || !canonical_uuid(&config.pm.namespace_provisioner_subject)
    {
        return Err(unavailable());
    }
    issuer(&config.pm.namespace_authority_issuer)?;
    let mut url = origin(
        config
            .fleet
            .project_workflow_url
            .as_deref()
            .ok_or_else(unavailable)?,
    )?;
    url.path_segments_mut().map_err(|_| unavailable())?.extend([
        "api",
        "pm",
        "namespace-ownership",
        &namespace_id.to_string(),
    ]);
    let pat = &config.pm.namespace_read_pat;
    if !pat.starts_with("sdlc_pat_")
        || !(32..=128).contains(&pat.len())
        || !pat.bytes().all(|byte| byte.is_ascii_graphic())
        || pat == &config.pm.readback_token
        || pat == &config.auth.jwt_secret
        || pat == &config.fleet.runtime_token_secret
        || config.fleet.project_workflow_catalog_token.as_ref() == Some(pat)
    {
        return Err(unavailable());
    }
    let mut bearer =
        header::HeaderValue::from_str(&format!("Bearer {pat}")).map_err(|_| unavailable())?;
    bearer.set_sensitive(true);
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
        .header(header::AUTHORIZATION, bearer)
        .header(header::CACHE_CONTROL, "no-cache, no-store")
        .header(header::ACCEPT_ENCODING, "identity")
        .send()
        .await
        .map_err(|_| unavailable())?;
    if response.status() != StatusCode::OK
        || response
            .headers()
            .get(header::CONTENT_ENCODING)
            .is_some_and(|value| value != "identity")
        || response
            .content_length()
            .is_some_and(|length| length > MAX_BODY as u64)
    {
        return Err(unavailable());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
        if bytes.len() + chunk.len() > MAX_BODY {
            return Err(unavailable());
        }
        bytes.extend_from_slice(&chunk);
    }
    validate(
        serde_json::from_slice(&bytes).map_err(|_| unavailable())?,
        config,
        operation,
        namespace_id,
    )
}

#[cfg(test)]
#[path = "pm_namespace_tests.rs"]
mod tests;
