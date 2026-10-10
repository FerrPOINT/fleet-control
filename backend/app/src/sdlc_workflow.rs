//! Fresh, bounded owner readback. Never falls back to the legacy catalog token.
use domain::{Agent, AgentConfigRevision, SdlcWorkflowBinding, canonical_workflow_id};
use reqwest::{Client, Url, header};
use serde::Deserialize;
use shared::{AppError, config::SdlcWorkflowConfig};
use std::time::Duration;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {
    ok: bool,
    binding: SdlcWorkflowBinding,
}

fn unavailable() -> AppError {
    AppError::Unavailable("Workflow namespace binding readback unavailable".into())
}

fn endpoint(config: &SdlcWorkflowConfig, agent: &Agent) -> Result<Url, AppError> {
    let id = agent
        .namespace_id
        .as_deref()
        .filter(|id| canonical_workflow_id(id))
        .ok_or_else(|| AppError::conflict("a persisted Workflow namespace ID is required"))?;
    let mut url = Url::parse(&config.url).map_err(|_| unavailable())?;
    if config.url.trim() != config.url
        || !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || config.read_pat.trim().is_empty()
    {
        return Err(unavailable());
    }
    url.set_path(&format!("/internal/runtime/base/namespace-bindings/{id}"));
    Ok(url)
}

pub async fn read_binding(
    config: &SdlcWorkflowConfig,
    agent: &Agent,
) -> Result<SdlcWorkflowBinding, AppError> {
    let url = endpoint(config, agent)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(3))
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .no_proxy()
        .build()
        .map_err(|_| unavailable())?;
    let mut response = client
        .get(url)
        .bearer_auth(&config.read_pat)
        .header(header::ACCEPT_ENCODING, "identity")
        .send()
        .await
        .map_err(|_| unavailable())?;
    if response.status() != reqwest::StatusCode::OK
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
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
        if bytes.len() + chunk.len() > 16_384 {
            return Err(unavailable());
        }
        bytes.extend_from_slice(&chunk);
    }
    let response: Response = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
    if !response.ok || !response.binding.matches_agent(agent) {
        return Err(AppError::conflict(
            "Workflow namespace binding differs from the agent",
        ));
    }
    Ok(response.binding)
}

pub async fn verify_revision_binding(
    config: &SdlcWorkflowConfig,
    agent: &Agent,
    revision: &AgentConfigRevision,
) -> Result<(), AppError> {
    if revision
        .snapshot
        .config
        .config_json
        .get("fleet_sdlc_package")
        .is_none()
    {
        return Ok(());
    }
    let frozen: SdlcWorkflowBinding = serde_json::from_value(
        revision
            .snapshot
            .config
            .config_json
            .get("fleet_sdlc_workflow_binding")
            .cloned()
            .ok_or_else(|| AppError::conflict("frozen Workflow namespace binding is missing"))?,
    )
    .map_err(|_| AppError::conflict("frozen Workflow namespace binding is invalid"))?;
    if read_binding(config, agent).await? != frozen {
        return Err(AppError::conflict(
            "Workflow namespace binding changed since configuration preparation",
        ));
    }
    Ok(())
}
