use domain::{AgentKind, AgentPaths};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
}

#[derive(Debug, Clone)]
pub struct RuntimeLaunchRecord {
    pub binding: RuntimeLaunchBinding,
    pub state: String,
    pub pid: Option<i32>,
}
