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
