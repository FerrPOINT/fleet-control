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
pub struct MountMappingController {
    pub container_id: String,
    pub image_id: String,
    pub service: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountMappingSnapshot {
    pub container_id: String,
    pub started_at: String,
    pub init_pid: u32,
    pub inventory_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectedAgentMount {
    #[serde(rename = "type")]
    pub mount_type: String,
    pub source: String,
    pub destination: String,
    pub read_only: bool,
}

/// Original controller/Engine proof; projected bind paths are not launch mounts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerMountMapping {
    pub state: String,
    pub controller: MountMappingController,
    pub snapshot: MountMappingSnapshot,
    pub engine: ContainerEngineIdentity,
    pub local_root: String,
    pub volume_name: String,
    pub volume_sha256: String,
    pub mounts: Vec<ProjectedAgentMount>,
    pub input_policy_sha256: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mount_mapping: Option<ContainerMountMapping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping_file: Option<String>,
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
    /// An ownership saga fences original-owner effects, even when its lease expires.
    pub controller_recovery: bool,
}

/// Controller-private request. Hashes refer to original witnesses, never raw secrets.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerRecoveryRequest {
    pub id: Uuid,
    pub launch_id: Uuid,
    pub agent_id: Uuid,
    pub original_controller_id: Uuid,
    pub controller_id: Uuid,
    pub predecessor_id: Option<Uuid>,
    pub launch_sha256: String,
    pub mapping_sha256: String,
    pub registration_sha256: String,
    pub controller_snapshot: MountMappingSnapshot,
    pub agent_pid: i32,
}

/// Storage acknowledgement is not a native owner permit or SDLC readiness.
#[derive(Clone)]
pub struct ControllerRecoveryRecord {
    pub request: ControllerRecoveryRequest,
    pub epoch: i64,
    pub state: String,
    pub lease_version: i64,
    pub lease_expires_at: String,
    pub lease_valid: bool,
    pub native_receipt_sha256: Option<String>,
}

/// Durable pre-create fence; the private intent (including credentials) stays on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeContainerPreparation {
    pub agent_id: Uuid,
    pub ordinal: i64,
    pub controller_id: Uuid,
    pub generation: Uuid,
    pub operation_id: Uuid,
    pub intent_sha256: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeConfigurationClaim {
    pub phase: String,
    pub revision: Option<i64>,
    pub sha256: Option<String>,
}
