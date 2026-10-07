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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeControlLookupQuery {
    pub operation: RuntimeControlOperation,
    pub payload_sha256: String,
}

impl RuntimeControlLookupQuery {
    pub fn is_valid(&self) -> bool {
        self.payload_sha256.len() == 64
            && self
                .payload_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }
}

/// Hash of UTF8 canonical JSON with sorted keys: {"input":...,"operation":...}.
pub fn runtime_control_payload_sha256(
    operation: RuntimeControlOperation,
    input: Option<&str>,
) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(
        serde_json::to_vec(&serde_json::json!({"operation":operation,"input":input}))
            .expect("runtime control strings serialize"),
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_identity_matches_canonical_utf8_without_trimming_or_ascii_expansion() {
        for (operation, input, expected) in [
            (
                RuntimeControlOperation::Stop,
                None,
                "ea123901799860e917ce433b72c621c4afffe4c866497c1bfb38507816f8048f",
            ),
            (
                RuntimeControlOperation::Steer,
                Some("keep scope"),
                "836755e924913fa3776aeec3253eb2f9ba7c4d473e44deb16e87bbdd93f9f1b2",
            ),
            (
                RuntimeControlOperation::Steer,
                Some("  \u{0434}\u{0430}\n\"\\\t  "),
                "1ef66b6c19cae8637433011f49a3e26e994693ef2c26bff7dbdaff0ff3a75565",
            ),
        ] {
            assert_eq!(runtime_control_payload_sha256(operation, input), expected);
        }
    }

    #[test]
    fn lookup_requires_closed_scope_and_canonical_digest() {
        let mut query = RuntimeControlLookupQuery {
            operation: RuntimeControlOperation::Steer,
            payload_sha256: "a".repeat(64),
        };
        assert!(query.is_valid());
        for hash in [
            "".into(),
            "a".repeat(63),
            "a".repeat(65),
            "A".repeat(64),
            "g".repeat(64),
        ] {
            query.payload_sha256 = hash;
            assert!(!query.is_valid());
        }
        for body in [
            serde_json::json!({"operation":"steer"}),
            serde_json::json!({"operation":"approval","payload_sha256":"a".repeat(64)}),
            serde_json::json!({"operation":"steer","payload_sha256":"a".repeat(64),"actor_user_id":Uuid::new_v4()}),
        ] {
            assert!(serde_json::from_value::<RuntimeControlLookupQuery>(body).is_err());
        }
    }
}
