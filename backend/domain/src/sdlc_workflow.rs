use crate::Agent;
use serde::{Deserialize, Serialize};

/// Workflow-owned persisted mapping, not a native Hermes profile attestation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SdlcWorkflowBinding {
    pub schema: String,
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    pub namespace_id: String,
    pub namespace_name: String,
    #[schema(pattern = "^[1-9][0-9]*$", max_length = 19)]
    pub workflow_id: String,
    pub workflow_key: String,
    pub role_key: String,
    pub profile: String,
    pub catalog_version: u32,
    pub catalog_sha256: String,
    pub skills_revision: String,
    pub runtime_ready: bool,
}

pub fn canonical_workflow_id(value: &str) -> bool {
    value
        .parse::<i64>()
        .is_ok_and(|id| id > 0 && id.to_string() == value)
}

impl SdlcWorkflowBinding {
    pub fn matches_agent(&self, agent: &Agent) -> bool {
        let role = agent.sdlc_role.map(|role| {
            if role == crate::SdlcRole::DevOps {
                "devops"
            } else {
                role.as_str()
            }
        });
        self.schema == "base-sdlc/workflow-binding/v1"
            && canonical_workflow_id(&self.namespace_id)
            && canonical_workflow_id(&self.workflow_id)
            && agent.namespace_id.as_deref() == Some(self.namespace_id.as_str())
            && agent.workflow_id.as_deref() == Some(self.workflow_id.as_str())
            && role == Some(self.role_key.as_str())
            && self.workflow_key == format!("hermes-sdlc:{}", self.role_key)
            && self.catalog_version == 3
            && self.catalog_sha256.len() == 64
            && self
                .catalog_sha256
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            && !self.runtime_ready
    }
}
