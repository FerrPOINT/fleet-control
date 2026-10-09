use crate::{SessionRunState, Timestamp};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeControlOperation {
    Steer,
    Stop,
}

impl RuntimeControlOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Steer => "steer",
            Self::Stop => "stop",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeControlState {
    Reserved,
    Submitted,
    Acknowledged,
    Uncertain,
    Rejected,
    TerminalObserved,
}

/// Derived by the authenticated HTTP boundary, never deserialized from a user body.
#[derive(Debug, Clone)]
pub struct RuntimeControlActor {
    pub user_id: Uuid,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuntimeControlReceipt {
    pub id: Uuid,
    pub session_id: Uuid,
    pub session_run_id: Uuid,
    pub agent_id: Uuid,
    pub actor_user_id: Uuid,
    pub operation: RuntimeControlOperation,
    pub state: RuntimeControlState,
    pub acknowledgement: Option<String>,
    pub observed_run_state: Option<SessionRunState>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Debug, Clone)]
pub struct RuntimeControlReservation {
    pub receipt: RuntimeControlReceipt,
    pub dispatch: bool,
}
