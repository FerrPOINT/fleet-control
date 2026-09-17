use crate::middleware::{CurrentUser, require_operator};
use app::AppContext;
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use domain::{
    Agent, AgentConfig, AgentDirectoryItem, AgentSkill, AgentStatus, AgentStorageReport,
    AgentStorageReview, AgentStorageReviewItem, CreateAgentRequest, PurgeAgentFilesRequest,
    PurgeAgentFilesResponse, RuntimeOperationResponse, UpdateAgentConfigRequest,
    UpdateAgentRequest, UpdateSkillRequest,
};
use shared::{AppError, FleetEvent};
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(get, path = "/api/v1/agents", tag = "agents", responses((status = 200, body = Vec<Agent>)))]
pub async fn list_agents(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<Vec<Agent>>, AppError> {
    require_operator(&user)?;
    Ok(Json(ctx.repo.list_agents().await?))
}

#[utoipa::path(get, path = "/api/v1/agent-directory", tag = "agents", responses((status = 200, body = Vec<AgentDirectoryItem>)))]
pub async fn list_agent_directory(
    State(ctx): State<Arc<AppContext>>,
) -> Result<Json<Vec<AgentDirectoryItem>>, AppError> {
    Ok(Json(ctx.repo.list_agent_directory().await?))
}

#[utoipa::path(get, path = "/api/v1/agents/storage-review", tag = "agents", responses((status = 200, body = AgentStorageReview)))]
pub async fn get_agent_storage_review(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<AgentStorageReview>, AppError> {
    require_operator(&user)?;
    let agents = ctx.repo.list_agents().await?;
    let mut items = Vec::with_capacity(agents.len());
    for agent in agents {
        let storage = ctx.provisioner.storage_report(&agent, &ctx.config).await?;
        items.push(storage_review_item(&agent, storage));
    }
    Ok(Json(build_storage_review(
        items,
        shared::now().to_rfc3339(),
    )))
}

#[utoipa::path(post, path = "/api/v1/agents", tag = "agents", request_body = CreateAgentRequest, responses((status = 200, body = Agent)))]
pub async fn create_agent(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<Agent>, AppError> {
    require_operator(&user)?;
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let agent = ctx.repo.create_agent(req, &ctx.config).await?;
    ctx.provisioner.provision(&agent, &ctx.config).await?;
    let agent = ctx
        .repo
        .update_agent_status(agent.id, domain::AgentStatus::Ready)
        .await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "agent.create",
            "agent",
            Some(agent.id.to_string()),
            audit_payload,
        )
        .await?;
    ctx.emit(FleetEvent::AgentCreated {
        agent_id: agent.id.to_string(),
        name: agent.name.clone(),
    });
    Ok(Json(agent))
}

#[utoipa::path(get, path = "/api/v1/agents/{agent_id}", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = Agent)))]
pub async fn get_agent(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<Agent>, AppError> {
    require_operator(&user)?;
    Ok(Json(ctx.repo.get_agent(agent_id).await?))
}

#[utoipa::path(patch, path = "/api/v1/agents/{agent_id}", tag = "agents", params(("agent_id" = Uuid, Path)), request_body = UpdateAgentRequest, responses((status = 200, body = Agent)))]
pub async fn update_agent(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<UpdateAgentRequest>,
) -> Result<Json<Agent>, AppError> {
    require_operator(&user)?;
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let agent = ctx.repo.update_agent(agent_id, req).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "agent.update",
            "agent",
            Some(agent.id.to_string()),
            audit_payload,
        )
        .await?;
    ctx.emit(FleetEvent::AgentUpdated {
        agent_id: agent.id.to_string(),
        name: agent.name.clone(),
    });
    Ok(Json(agent))
}

#[utoipa::path(delete, path = "/api/v1/agents/{agent_id}", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = Agent)))]
pub async fn archive_agent(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<Agent>, AppError> {
    require_operator(&user)?;
    let agent = ctx.repo.get_agent(agent_id).await?;
    let _ = ctx.runtime.stop(&agent).await;
    let agent = ctx.repo.archive_agent(agent_id).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "agent.archive",
            "agent",
            Some(agent.id.to_string()),
            serde_json::json!({ "name": agent.name }),
        )
        .await?;
    Ok(Json(agent))
}

#[utoipa::path(get, path = "/api/v1/agents/{agent_id}/storage", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = AgentStorageReport)))]
pub async fn get_agent_storage(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<AgentStorageReport>, AppError> {
    require_operator(&user)?;
    let agent = ctx.repo.get_agent(agent_id).await?;
    Ok(Json(
        ctx.provisioner.storage_report(&agent, &ctx.config).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/agents/{agent_id}/purge-files", tag = "agents", params(("agent_id" = Uuid, Path)), request_body = PurgeAgentFilesRequest, responses((status = 200, body = PurgeAgentFilesResponse)))]
pub async fn purge_agent_files(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<PurgeAgentFilesRequest>,
) -> Result<Json<PurgeAgentFilesResponse>, AppError> {
    require_operator(&user)?;
    let agent = ctx.repo.get_agent(agent_id).await?;
    if req.confirmation != agent.name {
        return Err(AppError::validation(format!(
            "confirmation must exactly match {}",
            agent.name
        )));
    }
    if agent.status != domain::AgentStatus::Archived {
        return Err(AppError::validation(
            "agent files can only be purged after the agent is archived",
        ));
    }
    let _ = ctx.runtime.stop(&agent).await;
    let response = ctx.provisioner.purge_files(&agent, &ctx.config).await?;
    ctx.repo
        .insert_event(
            Some(agent.id),
            "agent.files_purged",
            "Agent files were purged by an operator",
            serde_json::json!({
                "name": agent.name.clone(),
                "path": response.purged_path.clone(),
                "files_deleted": response.files_deleted,
                "marker_verified": response.marker_verified
            }),
        )
        .await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "agent.files_purge",
            "agent",
            Some(agent.id.to_string()),
            serde_json::json!({
                "name": agent.name.clone(),
                "path": response.purged_path.clone(),
                "files_deleted": response.files_deleted,
                "marker_verified": response.marker_verified
            }),
        )
        .await?;
    ctx.emit(FleetEvent::AgentFilesPurged {
        agent_id: agent.id.to_string(),
        name: agent.name.clone(),
    });
    Ok(Json(response))
}

#[utoipa::path(post, path = "/api/v1/agents/{agent_id}/provision", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = Agent)))]
pub async fn provision_agent(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<Agent>, AppError> {
    require_operator(&user)?;
    let agent = ctx.repo.get_agent(agent_id).await?;
    ctx.provisioner.provision(&agent, &ctx.config).await?;
    let agent = ctx
        .repo
        .update_agent_status(agent_id, domain::AgentStatus::Ready)
        .await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "agent.provision",
            "agent",
            Some(agent.id.to_string()),
            serde_json::json!({ "name": agent.name, "kind": agent.kind.as_str() }),
        )
        .await?;
    Ok(Json(agent))
}

#[utoipa::path(post, path = "/api/v1/agents/{agent_id}/start", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = RuntimeOperationResponse)))]
pub async fn start_agent(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<RuntimeOperationResponse>, AppError> {
    require_operator(&user)?;
    let agent = ctx.repo.get_agent(agent_id).await?;
    let previous_status = Some(agent.status.as_str().to_string());
    let response = ctx.runtime.start(&agent).await?;
    record_health_transition(
        ctx.clone(),
        agent.id,
        previous_status,
        response.status.as_str(),
    );
    ctx.repo
        .insert_audit(
            Some(user.id),
            "agent.start",
            "agent",
            Some(agent.id.to_string()),
            serde_json::json!({ "status": response.status.as_str() }),
        )
        .await?;
    ctx.emit(FleetEvent::RuntimeChanged {
        agent_id: response.agent_id.to_string(),
        status: response.status.as_str().to_string(),
    });
    Ok(Json(response))
}

#[utoipa::path(post, path = "/api/v1/agents/{agent_id}/stop", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = RuntimeOperationResponse)))]
pub async fn stop_agent(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<RuntimeOperationResponse>, AppError> {
    require_operator(&user)?;
    let agent = ctx.repo.get_agent(agent_id).await?;
    let previous_status = Some(agent.status.as_str().to_string());
    let response = ctx.runtime.stop(&agent).await?;
    record_health_transition(
        ctx.clone(),
        agent.id,
        previous_status,
        response.status.as_str(),
    );
    ctx.repo
        .insert_audit(
            Some(user.id),
            "agent.stop",
            "agent",
            Some(agent.id.to_string()),
            serde_json::json!({ "status": response.status.as_str() }),
        )
        .await?;
    Ok(Json(response))
}

#[utoipa::path(post, path = "/api/v1/agents/{agent_id}/restart", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = RuntimeOperationResponse)))]
pub async fn restart_agent(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<RuntimeOperationResponse>, AppError> {
    require_operator(&user)?;
    let agent = ctx.repo.get_agent(agent_id).await?;
    let response = ctx.runtime.restart(&agent).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "agent.restart",
            "agent",
            Some(agent.id.to_string()),
            serde_json::json!({ "status": response.status.as_str() }),
        )
        .await?;
    Ok(Json(response))
}

#[utoipa::path(post, path = "/api/v1/agents/{agent_id}/health", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = RuntimeOperationResponse)))]
pub async fn agent_health(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<RuntimeOperationResponse>, AppError> {
    require_operator(&user)?;
    let agent = ctx.repo.get_agent(agent_id).await?;
    let previous_status = Some(agent.status.as_str().to_string());
    let response = ctx.runtime.health(&agent).await?;
    record_health_transition(
        ctx.clone(),
        agent.id,
        previous_status,
        response.status.as_str(),
    );
    ctx.repo
        .insert_audit(
            Some(user.id),
            "agent.health",
            "agent",
            Some(agent.id.to_string()),
            serde_json::json!({ "status": response.status.as_str() }),
        )
        .await?;
    Ok(Json(response))
}

#[utoipa::path(get, path = "/api/v1/agents/{agent_id}/config", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = AgentConfig)))]
pub async fn get_agent_config(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<AgentConfig>, AppError> {
    require_operator(&user)?;
    Ok(Json(ctx.repo.get_agent_config(agent_id).await?))
}

#[utoipa::path(put, path = "/api/v1/agents/{agent_id}/config", tag = "agents", params(("agent_id" = Uuid, Path)), request_body = UpdateAgentConfigRequest, responses((status = 200, body = AgentConfig)))]
pub async fn update_agent_config(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<UpdateAgentConfigRequest>,
) -> Result<Json<AgentConfig>, AppError> {
    require_operator(&user)?;
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let config = ctx.repo.update_agent_config(agent_id, req).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "agent_config.update",
            "agent_config",
            Some(agent_id.to_string()),
            audit_payload,
        )
        .await?;
    Ok(Json(config))
}

#[utoipa::path(get, path = "/api/v1/agents/{agent_id}/skills", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = Vec<AgentSkill>)))]
pub async fn list_agent_skills(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<Vec<AgentSkill>>, AppError> {
    require_operator(&user)?;
    Ok(Json(ctx.repo.list_agent_skills(agent_id).await?))
}

#[utoipa::path(put, path = "/api/v1/agents/{agent_id}/skills/{skill_name}", tag = "agents", params(("agent_id" = Uuid, Path), ("skill_name" = String, Path)), request_body = UpdateSkillRequest, responses((status = 200, body = AgentSkill)))]
pub async fn update_agent_skill(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path((agent_id, skill_name)): Path<(Uuid, String)>,
    Json(req): Json<UpdateSkillRequest>,
) -> Result<Json<AgentSkill>, AppError> {
    require_operator(&user)?;
    let audit_payload = serde_json::to_value(&req).map_err(AppError::internal)?;
    let skill = ctx
        .repo
        .update_agent_skill(agent_id, skill_name, req)
        .await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "agent_skill.update",
            "agent_skill",
            Some(format!("{}:{}", agent_id, skill.name)),
            audit_payload,
        )
        .await?;
    ctx.emit(FleetEvent::SkillChanged {
        agent_id: agent_id.to_string(),
        skill: skill.name.clone(),
    });
    Ok(Json(skill))
}

#[utoipa::path(get, path = "/api/v1/fleet-alerts", tag = "agents", params(("state" = Option<String>, Query)), responses((status = 200, body = [domain::FleetAlert])))]
pub async fn list_fleet_alerts(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<domain::FleetAlert>>, AppError> {
    require_operator(&user)?;
    let state = params.get("state").map(String::as_str);
    if let Some(state) = state {
        if !matches!(state, "open" | "acknowledged" | "resolved") {
            return Err(AppError::validation(
                "state must be open|acknowledged|resolved",
            ));
        }
    }
    Ok(Json(ctx.repo.list_fleet_alerts(state).await?))
}

#[utoipa::path(post, path = "/api/v1/fleet-alerts/{alert_id}/acknowledge", tag = "agents", params(("alert_id" = Uuid, Path)), responses((status = 200, body = domain::FleetAlert)))]
pub async fn acknowledge_fleet_alert(
    State(ctx): State<Arc<AppContext>>,
    Extension(user): Extension<CurrentUser>,
    Path(alert_id): Path<Uuid>,
) -> Result<Json<domain::FleetAlert>, AppError> {
    require_operator(&user)?;
    let alert = ctx.repo.acknowledge_fleet_alert(alert_id, user.id).await?;
    ctx.repo
        .insert_audit(
            Some(user.id),
            "fleet_alert.acknowledge",
            "fleet_alert",
            Some(alert.id.to_string()),
            serde_json::json!({ "kind": alert.kind, "agent_id": alert.agent_id }),
        )
        .await?;
    Ok(Json(alert))
}

/// Fires fleet-alert transitions without blocking the API response.
fn record_health_transition(
    ctx: std::sync::Arc<app::AppContext>,
    agent_id: uuid::Uuid,
    previous_status: Option<String>,
    current_status: &str,
) {
    let current_status = current_status.to_string();
    tokio::spawn(async move {
        let alerts = app::RepositoryAlertService {
            repository: ctx.repo.clone(),
        };
        if let Err(error) = alerts
            .record_health_transition(agent_id, previous_status, &current_status)
            .await
        {
            tracing::warn!(agent_id = %agent_id, %error, "failed to record health transition");
        }
    });
}

fn storage_review_item(agent: &Agent, storage: AgentStorageReport) -> AgentStorageReviewItem {
    AgentStorageReviewItem {
        agent_id: agent.id,
        agent_name: agent.name.clone(),
        display_name: agent.display_name.clone(),
        kind: agent.kind,
        product_role: agent.product_role,
        status: agent.status,
        total_bytes: storage.total_bytes,
        total_files: storage.total_files,
        root_exists: storage.root_exists,
        marker_verified: storage.marker_verified,
        purge_eligible: storage.retention.purge_eligible,
        retention_hint: storage.retention.retention_hint,
    }
}

fn build_storage_review(
    items: Vec<AgentStorageReviewItem>,
    reviewed_at: String,
) -> AgentStorageReview {
    let total_bytes = items.iter().map(|item| item.total_bytes).sum();
    let archived_bytes = items
        .iter()
        .filter(|item| item.status == AgentStatus::Archived)
        .map(|item| item.total_bytes)
        .sum();
    AgentStorageReview {
        reviewed_at,
        total_agents: items.len(),
        total_bytes,
        archived_agents: items
            .iter()
            .filter(|item| item.status == AgentStatus::Archived)
            .count(),
        archived_bytes,
        purge_eligible_agents: items.iter().filter(|item| item.purge_eligible).count(),
        missing_root_agents: items.iter().filter(|item| !item.root_exists).count(),
        marker_issue_agents: items
            .iter()
            .filter(|item| item.root_exists && !item.marker_verified)
            .count(),
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{AgentKind, AgentProductRole};

    fn review_item(
        agent_name: &str,
        status: AgentStatus,
        total_bytes: u64,
        root_exists: bool,
        marker_verified: bool,
        purge_eligible: bool,
    ) -> AgentStorageReviewItem {
        AgentStorageReviewItem {
            agent_id: Uuid::new_v4(),
            agent_name: agent_name.to_string(),
            display_name: agent_name.to_string(),
            kind: AgentKind::Hermes,
            product_role: AgentProductRole::Executor,
            status,
            total_bytes,
            total_files: 1,
            root_exists,
            marker_verified,
            purge_eligible,
            retention_hint: "hint".to_string(),
        }
    }

    #[test]
    fn storage_review_summarizes_retention_state() {
        let review = build_storage_review(
            vec![
                review_item("agent1", AgentStatus::Running, 100, true, true, false),
                review_item("agent2", AgentStatus::Archived, 200, true, true, true),
                review_item("agent3", AgentStatus::Archived, 300, true, false, false),
                review_item("agent4", AgentStatus::Archived, 400, false, false, false),
            ],
            "2026-09-03T10:00:00Z".to_string(),
        );

        assert_eq!(review.total_agents, 4);
        assert_eq!(review.total_bytes, 1000);
        assert_eq!(review.archived_agents, 3);
        assert_eq!(review.archived_bytes, 900);
        assert_eq!(review.purge_eligible_agents, 1);
        assert_eq!(review.marker_issue_agents, 1);
        assert_eq!(review.missing_root_agents, 1);
    }
}
