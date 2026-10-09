//! Controller-private original Docker identities. These are never public API DTOs.
use domain::AgentPaths;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerEngineIdentity {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "KernelVersion")]
    pub kernel_version: String,
    #[serde(rename = "ServerVersion")]
    pub server_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerRegistration {
    pub contract_version: u8,
    pub operation_id: Uuid,
    pub container_id: String,
    pub resource_id: Uuid,
    pub generation: Uuid,
    pub engine: ContainerEngineIdentity,
    pub policy_sha256: String,
    pub inventory_sha256: String,
    pub running_inventory_sha256: String,
    pub compose_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_sha256: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerBinding {
    pub registration: ContainerRegistration,
    pub policy: Value,
    pub compose: String,
    pub journal: String,
    pub stop_journal: String,
    pub source_sha256: [String; 3],
    pub context: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedContainer {
    pub agent_id: Uuid,
    pub paths: AgentPaths,
    pub api_port: Option<i32>,
    pub configuration_revision: Option<i64>,
    pub configuration_sha256: Option<String>,
    pub container: ContainerBinding,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerLaunch {
    pub prepared: PreparedContainer,
    pub controller_id: Uuid,
    pub state: String,
    pub snapshot: Option<Value>,
    pub origin: Option<String>,
    pub stop_id: Uuid,
}
