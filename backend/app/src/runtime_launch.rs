use domain::{AgentKind, AgentPaths};
use serde::{Deserialize, Serialize};
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

/// Private controller provenance, never a user-supplied runtime endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeContainerBinding {
    pub registration: ContainerRegistration,
    pub policy: serde_json::Value,
    pub compose: String,
    pub journal: String,
    pub stop_journal: String,
    pub source_sha256: [String; 3],
    pub context: String,
}

/// Controller-only launch identity. A gateway exit is not a boundary-empty receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLaunchBinding {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub controller_id: Uuid,
    pub kind: AgentKind,
    pub paths: AgentPaths,
    pub api_port: Option<i32>,
    pub phase: String,
    pub configuration_revision: Option<i64>,
    pub configuration_sha256: Option<String>,
    pub command_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<RuntimeContainerBinding>,
}

#[derive(Debug, Clone)]
pub struct RuntimeLaunchRecord {
    pub binding: RuntimeLaunchBinding,
    pub state: String,
    pub pid: Option<i32>,
}
