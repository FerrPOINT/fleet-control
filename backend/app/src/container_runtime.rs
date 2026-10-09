//! Controller-private original Docker identities. These are never public API DTOs.
use domain::AgentPaths;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Only hashes and non-secret identities enter PostgreSQL. Exact credentials stay
/// in the immutable 0600 intent outside all agent mounts.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerPreparationClaim {
    pub agent_id: Uuid,
    pub generation: Uuid,
    pub operation_id: Uuid,
    pub paths: AgentPaths,
    pub api_port: Option<i32>,
    pub configuration_revision: Option<i64>,
    pub configuration_sha256: Option<String>,
    pub intent_sha256: String,
}

#[derive(Clone)]
pub struct ContainerPreparation {
    pub claim: ContainerPreparationClaim,
    pub attempted: bool,
    pub receipt: Option<PreparedContainer>,
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mount_mapping_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingController {
    pub container_id: String,
    pub image_id: String,
    pub service: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerSnapshot {
    pub container_id: String,
    pub started_at: String,
    pub init_pid: u32,
    pub inventory_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectedMount {
    #[serde(rename = "type")]
    pub mount_type: String,
    pub source: String,
    pub destination: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerMapping {
    pub state: String,
    pub controller: MappingController,
    pub snapshot: ControllerSnapshot,
    pub engine: ContainerEngineIdentity,
    pub local_root: String,
    pub volume_name: String,
    pub volume_sha256: String,
    pub mounts: Vec<ProjectedMount>,
    pub input_policy_sha256: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappedContainer {
    pub mapping: ContainerMapping,
    pub mapping_file: String,
    pub attachment_journal: String,
    pub recovery_journal: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapped: Option<MappedContainer>,
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

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerRecoveryRequest {
    pub id: Uuid,
    pub launch_id: Uuid,
    pub agent_id: Uuid,
    pub original_controller_id: Uuid,
    pub controller_id: Uuid,
    pub predecessor_id: Option<Uuid>,
    pub launch_sha256: String,
    pub mapping_sha256: String,
    pub registration_sha256: String,
    pub controller_snapshot: ControllerSnapshot,
    pub agent_pid: i32,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerRecoveryCommand {
    pub request: ContainerRecoveryRequest,
    pub epoch: i64,
    pub lease_version: i64,
    pub lease_expires_at: String,
}

#[derive(Clone)]
pub struct ContainerRecovery {
    pub command: ContainerRecoveryCommand,
    pub receipt: Option<Value>,
    pub lease: ContainerRecoveryCommand,
    pub lease_receipt: Option<Value>,
    pub lease_valid: bool,
}
