use crate::PmExecutionIdentity;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::AppError;
use uuid::Uuid;

#[derive(Clone, Serialize)]
pub struct PmCredentialCommand {
    #[serde(skip)]
    task_id: Uuid,
    service: &'static str,
    label: String,
    scopes: Vec<String>,
    idempotency_key: String,
    expires_in_seconds: i64,
}

impl PmCredentialCommand {
    pub fn tracker(
        identity: &PmExecutionIdentity,
        operation_key: String,
        ttl_seconds: i64,
    ) -> Result<Self, AppError> {
        identity.validate()?;
        if identity.assignment_revision > 9_007_199_254_740_991
            || operation_key.is_empty()
            || operation_key.len() > 128
            || !operation_key.bytes().all(|c| c.is_ascii_graphic())
            || !(1..=1800).contains(&ttl_seconds)
        {
            return Err(AppError::validation("invalid PM credential command"));
        }
        let mut scopes = vec![
            "task-tracker:read".into(),
            "task-tracker:write".into(),
            format!(
                "task-tracker:sdlc:pm:{}:{}:{}:{}:{}",
                identity.task_ref,
                identity.assignment_ref,
                identity.execution_ref,
                identity.agent_ref,
                identity.assignment_revision,
            ),
        ];
        scopes.sort_unstable();
        Ok(Self {
            task_id: Uuid::parse_str(&identity.task_ref)
                .map_err(|_| AppError::validation("invalid PM task reference"))?,
            service: "task-tracker",
            label: format!("PM assignment {}", identity.assignment_ref),
            scopes,
            idempotency_key: operation_key,
            expires_in_seconds: ttl_seconds,
        })
    }

    pub fn task_id(&self) -> Uuid {
        self.task_id
    }

    pub fn scopes(&self) -> &[String] {
        &self.scopes
    }

    pub fn ttl_seconds(&self) -> i64 {
        self.expires_in_seconds
    }
}

/// Private operation journal. No parent/child bearer or human headers may enter it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmCredentialIntent {
    pub command: serde_json::Value,
    pub request_sha256: String,
    pub parent_fingerprint: String,
    pub base_origin: String,
    pub tracker_origin: String,
    pub machine_subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmCredentialReceipt {
    pub token_id: Uuid,
    pub expires_at: DateTime<Utc>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmCredentialJournal {
    pub intent: PmCredentialIntent,
    pub receipt: Option<PmCredentialReceipt>,
}
