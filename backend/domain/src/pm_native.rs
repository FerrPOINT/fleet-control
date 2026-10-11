//! Native PM request identity and effective configuration, never caller-issued readiness.
use crate::{PmRunRecord, valid_ref};
use serde::{Deserialize, Serialize};
use shared::AppError;
use utoipa::ToSchema;
use uuid::Uuid;

pub const PM_NATIVE_TOOLS: [&str; 6] = [
    "fleet_pm_checkpoint",
    "fleet_pm_context",
    "fleet_pm_question",
    "fleet_pm_requirements",
    "fleet_pm_skills",
    "fleet_pm_workflow",
];

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PmNativeAdmissionObservation {
    pub contract_version: u8,
    pub observation_ref: Uuid,
    pub observed_at: chrono::DateTime<chrono::Utc>,
    pub identity: crate::PmExecutionIdentity,
    pub session_run_id: Uuid,
    pub native_run_ref: String,
    pub native_session_ref: String,
    pub binding_ref: String,
    pub fence: i64,
    pub effective_config_revision: i64,
    pub configuration_sha256: String,
    pub native_configuration: PmNativeConfiguration,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PmNativeConfiguration {
    pub model: String,
    pub provider: String,
    pub api_mode: String,
    pub route_sha256: String,
    pub tools: Vec<String>,
    pub output_limit: i64,
}

impl PmNativeConfiguration {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.model.trim().is_empty()
            || self.model.len() > 512
            || self.provider.trim().is_empty()
            || self.provider.len() > 512
            || self.api_mode != "chat_completions"
            || self.output_limit <= 0
            || self.output_limit > 1_000_000
            || self.route_sha256.len() != 64
            || !self
                .route_sha256
                .bytes()
                .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
            || self.tools != PM_NATIVE_TOOLS.map(str::to_string)
        {
            return Err(AppError::conflict("PM native configuration is unsupported"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PmNativeAdmissionRequest {
    pub session_id: String,
    pub native_configuration: PmNativeConfiguration,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmNativeInventory {
    pub contract_version: u8,
    pub agent_id: Uuid,
    pub native_run_ref: String,
    pub session_id: String,
    pub model: String,
    pub provider: String,
    pub api_mode: String,
    pub context_limit: i64,
    pub output_limit: i64,
    pub turn_budget: i64,
    pub tools: Vec<String>,
    pub route_sha256: String,
    pub background_review_disabled: bool,
    pub memory_disabled: bool,
    pub automatic_titles_disabled: bool,
}

impl PmNativeInventory {
    pub fn verify(
        &self,
        record: &PmRunRecord,
        request: &PmNativeConfiguration,
        config: &serde_json::Value,
    ) -> Result<(), AppError> {
        request.validate()?;
        let model = &config["model"];
        let turns = config["agent"]["max_turns"].as_i64();
        if self.contract_version != 1
            || !self.background_review_disabled
            || !self.memory_disabled
            || !self.automatic_titles_disabled
            || self.agent_id != record.reservation.identity.agent_id()?
            || record.hermes_run_ref.as_deref() != Some(self.native_run_ref.as_str())
            || record.hermes_session_ref.as_deref() != Some(self.session_id.as_str())
            || self.model != request.model
            || self.provider != request.provider
            || self.api_mode != request.api_mode
            || self.output_limit != request.output_limit
            || self.route_sha256 != request.route_sha256
            || self.tools != request.tools
            || model["default"].as_str() != Some(self.model.as_str())
            || model["provider"].as_str() != Some(self.provider.as_str())
            || model["api_mode"].as_str() != Some("chat_completions")
            || model["context_length"].as_i64() != Some(self.context_limit)
            || self.context_limit <= 0
            || model["max_tokens"].as_i64() != Some(self.output_limit)
            || turns != Some(self.turn_budget)
            || self.turn_budget <= 0
            || !valid_ref(&self.native_run_ref, 512)
        {
            return Err(AppError::conflict(
                "PM native inventory differs from the original effective configuration",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_configuration_is_closed_and_requires_only_scoped_tools() {
        let original = serde_json::json!({"model":"pm-model","provider":"custom",
            "api_mode":"chat_completions","route_sha256":"a".repeat(64),
            "tools":PM_NATIVE_TOOLS,"output_limit":8192});
        let value: PmNativeConfiguration = serde_json::from_value(original.clone()).unwrap();
        value.validate().unwrap();
        for (field, value) in [
            ("output_limit", serde_json::json!(0)),
            ("api_mode", serde_json::json!("unknown")),
            ("tools", serde_json::json!(["terminal"])),
            ("route_sha256", serde_json::json!("unbound-route")),
        ] {
            let mut malformed = original.clone();
            malformed[field] = value;
            let parsed: PmNativeConfiguration = serde_json::from_value(malformed).unwrap();
            assert!(parsed.validate().is_err());
        }
        let mut unknown = original;
        unknown["allowed"] = serde_json::json!(true);
        assert!(serde_json::from_value::<PmNativeConfiguration>(unknown).is_err());
    }
}
