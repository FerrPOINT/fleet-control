use app::AppContext;
use axum::{
    Json,
    extract::{Path, State},
};
use domain::{
    Agent, AgentConfig, AgentSkill, CreateAgentRequest, RuntimeOperationResponse,
    UpdateAgentConfigRequest, UpdateAgentRequest, UpdateSkillRequest,
};
use shared::{AppError, FleetEvent};
use std::sync::Arc;
use uuid::Uuid;

#[utoipa::path(get, path = "/api/v1/agents", tag = "agents", responses((status = 200, body = Vec<Agent>)))]
pub async fn list_agents(State(ctx): State<Arc<AppContext>>) -> Result<Json<Vec<Agent>>, AppError> {
    Ok(Json(ctx.repo.list_agents().await?))
}

#[utoipa::path(post, path = "/api/v1/agents", tag = "agents", request_body = CreateAgentRequest, responses((status = 200, body = Agent)))]
pub async fn create_agent(
    State(ctx): State<Arc<AppContext>>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<Agent>, AppError> {
    let agent = ctx.repo.create_agent(req, &ctx.config).await?;
    ctx.provisioner.provision(&agent, &ctx.config).await?;
    let agent = ctx
        .repo
        .update_agent_status(agent.id, domain::AgentStatus::Ready)
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
    Path(agent_id): Path<Uuid>,
) -> Result<Json<Agent>, AppError> {
    Ok(Json(ctx.repo.get_agent(agent_id).await?))
}

#[utoipa::path(patch, path = "/api/v1/agents/{agent_id}", tag = "agents", params(("agent_id" = Uuid, Path)), request_body = UpdateAgentRequest, responses((status = 200, body = Agent)))]
pub async fn update_agent(
    State(ctx): State<Arc<AppContext>>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<UpdateAgentRequest>,
) -> Result<Json<Agent>, AppError> {
    let agent = ctx.repo.update_agent(agent_id, req).await?;
    ctx.emit(FleetEvent::AgentUpdated {
        agent_id: agent.id.to_string(),
        name: agent.name.clone(),
    });
    Ok(Json(agent))
}

#[utoipa::path(delete, path = "/api/v1/agents/{agent_id}", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = Agent)))]
pub async fn archive_agent(
    State(ctx): State<Arc<AppContext>>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<Agent>, AppError> {
    let agent = ctx.repo.get_agent(agent_id).await?;
    let _ = ctx.runtime.stop(&agent).await;
    Ok(Json(ctx.repo.archive_agent(agent_id).await?))
}

#[utoipa::path(post, path = "/api/v1/agents/{agent_id}/provision", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = Agent)))]
pub async fn provision_agent(
    State(ctx): State<Arc<AppContext>>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<Agent>, AppError> {
    let agent = ctx.repo.get_agent(agent_id).await?;
    ctx.provisioner.provision(&agent, &ctx.config).await?;
    Ok(Json(
        ctx.repo
            .update_agent_status(agent_id, domain::AgentStatus::Ready)
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/agents/{agent_id}/start", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = RuntimeOperationResponse)))]
pub async fn start_agent(
    State(ctx): State<Arc<AppContext>>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<RuntimeOperationResponse>, AppError> {
    let agent = ctx.repo.get_agent(agent_id).await?;
    let response = ctx.runtime.start(&agent).await?;
    ctx.emit(FleetEvent::RuntimeChanged {
        agent_id: response.agent_id.to_string(),
        status: response.status.as_str().to_string(),
    });
    Ok(Json(response))
}

#[utoipa::path(post, path = "/api/v1/agents/{agent_id}/stop", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = RuntimeOperationResponse)))]
pub async fn stop_agent(
    State(ctx): State<Arc<AppContext>>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<RuntimeOperationResponse>, AppError> {
    let agent = ctx.repo.get_agent(agent_id).await?;
    Ok(Json(ctx.runtime.stop(&agent).await?))
}

#[utoipa::path(post, path = "/api/v1/agents/{agent_id}/restart", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = RuntimeOperationResponse)))]
pub async fn restart_agent(
    State(ctx): State<Arc<AppContext>>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<RuntimeOperationResponse>, AppError> {
    let agent = ctx.repo.get_agent(agent_id).await?;
    Ok(Json(ctx.runtime.restart(&agent).await?))
}

#[utoipa::path(post, path = "/api/v1/agents/{agent_id}/health", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = RuntimeOperationResponse)))]
pub async fn agent_health(
    State(ctx): State<Arc<AppContext>>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<RuntimeOperationResponse>, AppError> {
    let agent = ctx.repo.get_agent(agent_id).await?;
    Ok(Json(ctx.runtime.health(&agent).await?))
}

#[utoipa::path(get, path = "/api/v1/agents/{agent_id}/config", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = AgentConfig)))]
pub async fn get_agent_config(
    State(ctx): State<Arc<AppContext>>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<AgentConfig>, AppError> {
    Ok(Json(ctx.repo.get_agent_config(agent_id).await?))
}

#[utoipa::path(put, path = "/api/v1/agents/{agent_id}/config", tag = "agents", params(("agent_id" = Uuid, Path)), request_body = UpdateAgentConfigRequest, responses((status = 200, body = AgentConfig)))]
pub async fn update_agent_config(
    State(ctx): State<Arc<AppContext>>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<UpdateAgentConfigRequest>,
) -> Result<Json<AgentConfig>, AppError> {
    Ok(Json(ctx.repo.update_agent_config(agent_id, req).await?))
}

#[utoipa::path(get, path = "/api/v1/agents/{agent_id}/skills", tag = "agents", params(("agent_id" = Uuid, Path)), responses((status = 200, body = Vec<AgentSkill>)))]
pub async fn list_agent_skills(
    State(ctx): State<Arc<AppContext>>,
    Path(agent_id): Path<Uuid>,
) -> Result<Json<Vec<AgentSkill>>, AppError> {
    Ok(Json(ctx.repo.list_agent_skills(agent_id).await?))
}

#[utoipa::path(put, path = "/api/v1/agents/{agent_id}/skills/{skill_name}", tag = "agents", params(("agent_id" = Uuid, Path), ("skill_name" = String, Path)), request_body = UpdateSkillRequest, responses((status = 200, body = AgentSkill)))]
pub async fn update_agent_skill(
    State(ctx): State<Arc<AppContext>>,
    Path((agent_id, skill_name)): Path<(Uuid, String)>,
    Json(req): Json<UpdateSkillRequest>,
) -> Result<Json<AgentSkill>, AppError> {
    let skill = ctx
        .repo
        .update_agent_skill(agent_id, skill_name, req)
        .await?;
    ctx.emit(FleetEvent::SkillChanged {
        agent_id: agent_id.to_string(),
        skill: skill.name.clone(),
    });
    Ok(Json(skill))
}
