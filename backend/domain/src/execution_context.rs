use serde::{Deserialize, Serialize};
pub use shared::resource_context::ExecutionContextV2;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionExecutionContext {
    pub context: ExecutionContextV2,
    pub tracker_project_id: Uuid,
    pub binding_generation: i64,
    pub runtime_ready: bool,
    pub dispatch_allowed: bool,
    pub adapter_version: String,
}

/// A v2 draft chat has no execution queue until a compatible foundation enables it.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateContextSessionRequest {
    pub primary_agent_id: Uuid,
    pub title: String,
    pub context: ExecutionContextV2,
}
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ContextSessionReceipt {
    pub session: crate::AgentSession,
    pub execution_context: SessionExecutionContext,
}
