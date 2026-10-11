use serde::{Deserialize, Serialize};
use shared::AppError;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalChoice {
    Once,
    Deny,
}

impl ApprovalChoice {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApprovalDecisionRequest {
    pub choice: ApprovalChoice,
    pub idempotency_key: String,
}

impl ApprovalDecisionRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.idempotency_key.is_empty()
            || self.idempotency_key.len() > 128
            || !self.idempotency_key.bytes().all(|b| b.is_ascii_graphic())
        {
            return Err(AppError::validation("invalid approval command key"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecisionState {
    Pending,
    Delivered,
    Uncertain,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApprovalDecision {
    pub id: Uuid,
    pub session_id: Uuid,
    pub approval_id: Uuid,
    pub session_run_id: Uuid,
    pub actor_user_id: Uuid,
    pub choice: ApprovalChoice,
    pub state: ApprovalDecisionState,
    pub created_at: crate::Timestamp,
}

pub struct ReservedApprovalDecision {
    pub decision: ApprovalDecision,
    pub approval: crate::RuntimeApprovalRequest,
    pub dispatch: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn targeted_choices_never_widen_permissions() {
        for choice in ["always", "session", "approve", "cancel"] {
            assert!(
                serde_json::from_value::<ApprovalDecisionRequest>(
                    serde_json::json!({"choice":choice,"idempotency_key":"command-1"})
                )
                .is_err()
            );
        }
        assert!(
            serde_json::from_value::<ApprovalDecisionRequest>(
                serde_json::json!({"choice":"once","idempotency_key":"key","resolve_all":true})
            )
            .is_err()
        );
        for key in ["", "a b", "a\nb", "ключ"] {
            assert!(
                ApprovalDecisionRequest {
                    choice: ApprovalChoice::Once,
                    idempotency_key: key.into()
                }
                .validate()
                .is_err()
            );
        }
    }
}
