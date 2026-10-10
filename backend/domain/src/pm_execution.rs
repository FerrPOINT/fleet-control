use serde::{Deserialize, Serialize};
use shared::AppError;
use utoipa::ToSchema;
use uuid::Uuid;

/// Identity shared with Project Workflow; runtime-local IDs are never execution IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PmExecutionIdentity {
    pub task: String,
    pub execution_ref: String,
    pub tracker_instance_ref: String,
    pub tracker_project_ref: String,
    pub task_ref: String,
    pub root_ref: String,
    pub agent_ref: String,
    pub assignment_operation_key: String,
    pub assignment_ref: String,
    pub assignment_revision: i64,
}

impl PmExecutionIdentity {
    pub fn validate(&self) -> Result<(), AppError> {
        let ordinal = self
            .task
            .strip_prefix("SDLC-")
            .and_then(|v| v.parse::<u64>().ok());
        if !ordinal.is_some_and(|n| n > 0 && self.task == format!("SDLC-{n}"))
            || self.assignment_revision < 1
            || !valid_ref(&self.tracker_instance_ref, 128)
            || !valid_operation_key(&self.assignment_operation_key)
        {
            return Err(AppError::validation("invalid PM execution identity"));
        }
        for value in [
            &self.execution_ref,
            &self.tracker_project_ref,
            &self.task_ref,
            &self.root_ref,
            &self.agent_ref,
            &self.assignment_ref,
        ] {
            canonical_uuid(value)?;
        }
        Ok(())
    }

    pub fn agent_id(&self) -> Result<Uuid, AppError> {
        canonical_uuid(&self.agent_ref)
    }
}

pub fn valid_ref(value: &str, limit: usize) -> bool {
    !value.is_empty()
        && value.len() <= limit
        && !value.chars().any(|v| v.is_whitespace() || v.is_control())
}

fn valid_operation_key(value: &str) -> bool {
    valid_ref(value, 128) && value.bytes().all(|c| c.is_ascii_graphic())
}

fn canonical_uuid(value: &str) -> Result<Uuid, AppError> {
    let id =
        Uuid::parse_str(value).map_err(|_| AppError::validation("invalid PM UUID reference"))?;
    if id.is_nil() || id.to_string() != value {
        return Err(AppError::validation(
            "PM UUID references must be canonical and non-nil",
        ));
    }
    Ok(id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PmRuntimeStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    Stopped,
}

impl PmRuntimeStatus {
    pub fn terminal(self) -> bool {
        self != Self::Running
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmRuntimeBinding {
    pub launch_id: Uuid,
    pub controller_id: Uuid,
    pub origin: String,
    pub credential_fingerprint: String,
}

impl PmRuntimeBinding {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.launch_id.is_nil()
            || self.controller_id.is_nil()
            || !valid_ref(&self.origin, 512)
            || self.credential_fingerprint.len() != 64
            || !self
                .credential_fingerprint
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(AppError::validation("invalid PM runtime binding"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmRunReservation {
    pub session_id: Uuid,
    pub session_run_id: Uuid,
    pub identity: PmExecutionIdentity,
    pub binding_ref: String,
    pub dispatch_operation_key: String,
    pub checkpoint_ref: Option<String>,
    pub fence: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_binding: Option<PmRuntimeBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_session_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_message_id: Option<Uuid>,
}

impl PmRunReservation {
    pub fn validate(&self) -> Result<(), AppError> {
        self.identity.validate()?;
        if let Some(binding) = &self.runtime_binding {
            binding.validate()?;
        }
        if let Some(key) = &self.native_session_key {
            let message_id = match self.native_message_id {
                Some(id) if !id.is_nil() => id,
                Some(_) => return Err(AppError::validation("PM native message identity is nil")),
                None => canonical_uuid(&self.dispatch_operation_key)?,
            };
            if self.runtime_binding.is_none()
                || *key
                    != pm_native_session_key(self.session_id, self.identity.agent_id()?, message_id)
            {
                return Err(AppError::validation(
                    "PM native session must belong to this dispatch",
                ));
            }
        }
        if self.session_id.is_nil()
            || self.session_run_id.is_nil()
            || self.fence < 1
            || !valid_ref(&self.binding_ref, 512)
            || !valid_operation_key(&self.dispatch_operation_key)
            || self
                .checkpoint_ref
                .as_ref()
                .is_some_and(|v| !valid_ref(v, 512))
        {
            return Err(AppError::validation("invalid PM run reservation"));
        }
        Ok(())
    }
    pub fn runtime_session_id(&self) -> String {
        self.native_session_key
            .clone()
            .unwrap_or_else(|| format!("fleet:{}:{}", self.session_id, self.identity.agent_ref))
    }
}

pub fn pm_native_session_key(session: Uuid, agent: Uuid, message: Uuid) -> String {
    format!("fleet-pm:{session}:{agent}:{message}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PmRunRecord {
    pub reservation: PmRunReservation,
    pub hermes_run_ref: Option<String>,
    pub hermes_session_ref: Option<String>,
    pub terminal_status: Option<PmRuntimeStatus>,
}

/// Exact callback wire shape consumed by Project Workflow (no API envelope).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PmRuntimeObservation {
    #[serde(flatten)]
    pub identity: PmExecutionIdentity,
    pub observation_ref: Uuid,
    pub binding_ref: String,
    pub hermes_run_ref: String,
    pub session_run_id: Uuid,
    pub status: PmRuntimeStatus,
    pub dispatch_operation_key: String,
    pub checkpoint_ref: Option<String>,
    pub fence: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> PmExecutionIdentity {
        let id = Uuid::new_v4().to_string();
        PmExecutionIdentity {
            task: "SDLC-42".into(),
            execution_ref: id.clone(),
            tracker_instance_ref: "tracker-test".into(),
            tracker_project_ref: id.clone(),
            task_ref: id.clone(),
            root_ref: id.clone(),
            agent_ref: id.clone(),
            assignment_operation_key: "assign-42".into(),
            assignment_ref: id,
            assignment_revision: 1,
        }
    }

    #[test]
    fn execution_identity_is_canonical_and_separate_from_display_key() {
        let mut value = identity();
        assert!(value.validate().is_ok());
        for task in ["TASK-42", "SDLC-0", "SDLC-042", "SDLC-+42", "SDLC--1"] {
            value.task = task.into();
            assert!(value.validate().is_err());
        }
        value = identity();
        value.assignment_revision = 0;
        assert!(value.validate().is_err());
        value = identity();
        value.agent_ref = Uuid::nil().to_string();
        assert!(value.validate().is_err());
        assert!(!valid_operation_key("dispatch-\u{e9}"));
        assert!(!valid_operation_key("dispatch key"));
        assert!(!valid_operation_key(&"x".repeat(129)));
        assert!(valid_operation_key("dispatch:pm/42"));
    }

    #[test]
    fn callback_is_flat_and_preserves_nullable_checkpoint() {
        let value = serde_json::to_value(PmRuntimeObservation {
            identity: identity(),
            observation_ref: Uuid::new_v4(),
            binding_ref: "binding-1".into(),
            hermes_run_ref: "run_local".into(),
            session_run_id: Uuid::new_v4(),
            status: PmRuntimeStatus::Cancelled,
            dispatch_operation_key: "dispatch-1".into(),
            checkpoint_ref: None,
            fence: 1,
        })
        .unwrap();
        assert!(value.get("identity").is_none());
        assert_eq!(value["task"], "SDLC-42");
        assert_eq!(value["status"], "cancelled");
        assert!(value["checkpoint_ref"].is_null());
        assert_eq!(value.as_object().unwrap().len(), 18);
    }

    #[test]
    fn runtime_binding_is_closed_and_rejects_invalid_fingerprints() {
        let binding = PmRuntimeBinding {
            launch_id: Uuid::new_v4(),
            controller_id: Uuid::new_v4(),
            origin: "http://127.0.0.1:29100".into(),
            credential_fingerprint: "a".repeat(64),
        };
        binding.validate().unwrap();
        for hash in ["a".repeat(63), "A".repeat(64), "z".repeat(64), "".into()] {
            let mut bad = binding.clone();
            bad.credential_fingerprint = hash;
            assert!(bad.validate().is_err());
        }
        for field in ["launch_id", "controller_id"] {
            let mut bad = serde_json::to_value(&binding).unwrap();
            bad[field] = serde_json::json!(Uuid::nil());
            assert!(
                serde_json::from_value::<PmRuntimeBinding>(bad)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
        let mut extended = serde_json::to_value(&binding).unwrap();
        extended["secret"] = serde_json::json!("untrusted");
        assert!(serde_json::from_value::<PmRuntimeBinding>(extended).is_err());
    }

    #[test]
    fn historical_reservation_is_readable_without_guessed_binding() {
        let reservation = PmRunReservation {
            session_id: Uuid::new_v4(),
            session_run_id: Uuid::new_v4(),
            identity: identity(),
            binding_ref: "binding".into(),
            dispatch_operation_key: "dispatch".into(),
            checkpoint_ref: None,
            fence: 1,
            runtime_binding: None,
            native_session_key: None,
            native_message_id: None,
        };
        let wire = serde_json::to_value(&reservation).unwrap();
        assert!(wire.get("runtime_binding").is_none());
        let historical: PmRunReservation = serde_json::from_value(wire).unwrap();
        assert!(historical.runtime_binding.is_none());
        assert_eq!(historical, reservation);
    }

    #[test]
    fn native_dispatch_sessions_are_distinct_with_the_same_task_execution_and_chat() {
        let mut reservation = PmRunReservation {
            session_id: Uuid::new_v4(),
            session_run_id: Uuid::new_v4(),
            identity: identity(),
            binding_ref: "binding".into(),
            dispatch_operation_key: Uuid::new_v4().to_string(),
            checkpoint_ref: None,
            fence: 1,
            runtime_binding: Some(PmRuntimeBinding {
                launch_id: Uuid::new_v4(),
                controller_id: Uuid::new_v4(),
                origin: "http://127.0.0.1:29002".into(),
                credential_fingerprint: "a".repeat(64),
            }),
            native_session_key: None,
            native_message_id: None,
        };
        let message = Uuid::parse_str(&reservation.dispatch_operation_key).unwrap();
        reservation.native_session_key = Some(pm_native_session_key(
            reservation.session_id,
            reservation.identity.agent_id().unwrap(),
            message,
        ));
        reservation.validate().unwrap();
        let mut resumed = reservation.clone();
        let next_message = Uuid::new_v4();
        resumed.dispatch_operation_key = next_message.to_string();
        assert!(resumed.validate().is_err()); // A late old-session call cannot adopt the new dispatch.
        resumed.native_session_key = Some(pm_native_session_key(
            resumed.session_id,
            resumed.identity.agent_id().unwrap(),
            next_message,
        ));
        resumed.validate().unwrap();
        assert_ne!(
            reservation.runtime_session_id(),
            resumed.runtime_session_id()
        );
        assert_eq!(reservation.identity, resumed.identity);
        assert_eq!(reservation.session_id, resumed.session_id);
    }
}
