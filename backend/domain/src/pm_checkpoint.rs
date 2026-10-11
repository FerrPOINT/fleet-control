use crate::{PmRunRecord, pm_canonical_hash};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::AppError;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmCheckpointJournal {
    pub command: Value,
    pub request_sha256: String,
    pub receipt: Option<Value>,
}

impl PmCheckpointJournal {
    pub fn validate(&self, record: &PmRunRecord) -> Result<(), AppError> {
        let r = &record.reservation;
        r.validate()?;
        let invalid = || AppError::conflict("PM checkpoint differs from original run");
        let fields = self.command.as_object().ok_or_else(invalid)?;
        let identity = serde_json::to_value(&r.identity).map_err(AppError::internal)?;
        let identity = identity.as_object().ok_or_else(invalid)?;
        if fields.len() != identity.len() + 10
            || !record
                .hermes_run_ref
                .as_deref()
                .is_some_and(|v| crate::valid_ref(v, 512))
            || !record
                .hermes_session_ref
                .as_deref()
                .is_some_and(|v| crate::valid_ref(v, 512))
            || identity
                .iter()
                .any(|(key, value)| fields.get(key) != Some(value))
            || self.request_sha256 != pm_canonical_hash(&self.command)
            || self.command["binding_ref"] != r.binding_ref
            || self.command["hermes_run_ref"].as_str() != record.hermes_run_ref.as_deref()
            || self.command["session_run_id"] != json!(r.session_run_id)
            || self.command["expected_fence"].as_i64() != Some(r.fence)
            || self.command["expected_version"]
                .as_i64()
                .is_none_or(|v| v < 1 || v == i64::MAX)
            || !self.command["operation_key"]
                .as_str()
                .is_some_and(|v| crate::pm_draft::valid_key(v) && v.is_ascii())
            || ["clarification_version", "requirements_revision"]
                .into_iter()
                .any(|key| {
                    self.command[key]
                        .as_i64()
                        .is_none_or(|v| !(1..=9_007_199_254_740_991).contains(&v))
                })
        {
            return Err(invalid());
        }
        for field in ["checkpoint_ref", "clarification_request_ref"] {
            let raw = self.command[field].as_str().ok_or_else(invalid)?;
            let id = uuid::Uuid::parse_str(raw).map_err(|_| invalid())?;
            if id.is_nil() || id.to_string() != raw {
                return Err(invalid());
            }
        }
        if let Some(receipt) = &self.receipt {
            self.verify_receipt(record, receipt)?;
        }
        Ok(())
    }

    pub fn verify_receipt(&self, record: &PmRunRecord, receipt: &Value) -> Result<(), AppError> {
        let expected = json!({"contract_version":1,"identity":record.reservation.identity,
            "state":"waiting","version":self.command["expected_version"].as_i64().and_then(|v| v.checked_add(1)),
            "fence":record.reservation.fence,"session_run_id":record.reservation.session_run_id,
            "binding_ref":record.reservation.binding_ref,"hermes_run_ref":record.hermes_run_ref,
            "checkpoint":{"checkpoint_ref":self.command["checkpoint_ref"],
                "clarification_request_ref":self.command["clarification_request_ref"],
                "clarification_version":self.command["clarification_version"],
                "requirements_revision":self.command["requirements_revision"]},
            "resume_operation_key":null,"resume_session_run_id":null,"terminal_readback":null,
            "workflow_step_allowed":false,"resume_delivered":false});
        if *receipt != expected {
            return Err(AppError::conflict(
                "PM waiting receipt differs from original checkpoint",
            ));
        }
        Ok(())
    }
}
