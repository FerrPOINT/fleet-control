//! Read-only Tracker lease evidence, matching task-tracker 357caa7.
use crate::{TrackerDraftIdentity, TrackerPmDraftReservation, pm_canonical_hash, valid_key};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use serde_json::{Value, json};
use shared::AppError;
use uuid::Uuid;

const MAX_SAFE_VERSION: u64 = 9_007_199_254_740_991;
const TTL_SECONDS: i64 = 30;
const HEARTBEAT_SECONDS: i64 = 10;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmExecutionLeaseFence {
    #[serde(deserialize_with = "canonical_uuid")]
    pub assignment_id: Uuid,
    #[serde(deserialize_with = "canonical_uuid")]
    pub execution_id: Uuid,
    #[serde(deserialize_with = "canonical_uuid")]
    pub agent_id: Uuid,
    #[serde(deserialize_with = "safe_version")]
    pub assignment_version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmExecutionLeaseClaim {
    #[serde(deserialize_with = "safe_version")]
    pub expected_owner_version: u64,
    pub fence: PmExecutionLeaseFence,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmExecutionLeaseHeartbeat {
    #[serde(deserialize_with = "safe_version")]
    pub expected_owner_version: u64,
    pub fence: PmExecutionLeaseFence,
    #[serde(deserialize_with = "canonical_uuid")]
    pub lease_id: Uuid,
    #[serde(deserialize_with = "safe_version")]
    pub expected_lease_version: u64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PmExecutionLeaseCommand {
    Claim(PmExecutionLeaseClaim),
    Heartbeat(PmExecutionLeaseHeartbeat),
}

impl PmExecutionLeaseCommand {
    pub fn idempotency_key(&self) -> &str {
        match self {
            Self::Claim(c) => &c.idempotency_key,
            Self::Heartbeat(c) => &c.idempotency_key,
        }
    }

    pub fn payload(&self) -> Value {
        match self {
            Self::Claim(c) => json!(c),
            Self::Heartbeat(c) => json!(c),
        }
    }

    pub fn request_sha256(&self) -> String {
        let operation = match self {
            Self::Claim(_) => "claim_pm_execution_lease",
            Self::Heartbeat(_) => "heartbeat_pm_execution_lease",
        };
        pm_canonical_hash(&json!({"operation": operation, "payload": self.payload()}))
    }

    fn validate(&self, owner: u64, fence: &PmExecutionLeaseFence) -> Result<(), AppError> {
        let (expected_owner, expected_fence) = match self {
            Self::Claim(c) => (c.expected_owner_version, &c.fence),
            Self::Heartbeat(c) => {
                if c.lease_id.is_nil() || !positive_safe(c.expected_lease_version) {
                    return Err(inconsistent());
                }
                (c.expected_owner_version, &c.fence)
            }
        };
        if expected_owner != owner || expected_fence != fence || !valid_key(self.idempotency_key())
        {
            return Err(inconsistent());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PmExecutionLeaseState {
    Unclaimed,
    Active,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmExecutionLease {
    #[serde(deserialize_with = "canonical_uuid")]
    pub lease_id: Uuid,
    #[serde(deserialize_with = "safe_version")]
    pub version: u64,
    pub holder_subject: String,
    #[serde(serialize_with = "utc_nanos")]
    pub claimed_at: DateTime<Utc>,
    #[serde(serialize_with = "utc_nanos")]
    pub heartbeat_at: DateTime<Utc>,
    #[serde(serialize_with = "utc_nanos")]
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmExecutionLeaseReceipt {
    pub contract_version: u8,
    pub binding: TrackerDraftIdentity,
    #[serde(deserialize_with = "safe_version")]
    pub owner_version: u64,
    pub fence: PmExecutionLeaseFence,
    pub lease: PmExecutionLease,
    pub ttl_seconds: i64,
    pub heartbeat_seconds: i64,
    #[serde(deserialize_with = "no_dispatch")]
    pub dispatch_allowed: bool,
}

/// An immutable operation result; it is not evidence of a current renewal or authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmExecutionLeaseOperation {
    pub idempotency_key: String,
    pub request_sha256: String,
    pub result: PmExecutionLeaseReceipt,
}

/// Original server-owned claim and its immutable acknowledgement; never dispatch authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmExecutionLeaseJournal {
    pub claim: PmExecutionLeaseClaim,
    pub request_sha256: String,
    pub receipt: Option<PmExecutionLeaseReceipt>,
}

impl PmExecutionLeaseReceipt {
    pub fn verify_claim(
        &self,
        reservation: &TrackerPmDraftReservation,
        claim: &PmExecutionLeaseClaim,
    ) -> Result<(), AppError> {
        let fence = PmExecutionLeaseFence {
            assignment_id: reservation.assignment.assignment_id,
            execution_id: reservation.assignment.execution_id,
            agent_id: reservation.assignment.agent_id,
            assignment_version: reservation.assignment.version,
        };
        PmExecutionLeaseCommand::Claim(claim.clone())
            .validate(reservation.owner_cas.version, &fence)?;
        validate_lease(&self.lease, reservation, self.lease.heartbeat_at)?;
        if self.contract_version != 1
            || self.binding != reservation.binding
            || self.owner_version != reservation.owner_cas.version
            || self.fence != fence
            || self.lease.version != 1
            || self.ttl_seconds != TTL_SECONDS
            || self.heartbeat_seconds != HEARTBEAT_SECONDS
            || self.dispatch_allowed
        {
            return Err(inconsistent());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmExecutionLeaseReadback {
    pub contract_version: u8,
    pub binding: TrackerDraftIdentity,
    #[serde(deserialize_with = "safe_version")]
    pub owner_version: u64,
    pub fence: PmExecutionLeaseFence,
    #[serde(serialize_with = "utc_nanos")]
    pub observed_at: DateTime<Utc>,
    pub state: PmExecutionLeaseState,
    #[serde(deserialize_with = "required_nullable")]
    pub current: Option<PmExecutionLease>,
    #[serde(deserialize_with = "required_nullable")]
    pub operation: Option<PmExecutionLeaseOperation>,
    #[serde(deserialize_with = "no_dispatch")]
    pub dispatch_allowed: bool,
}

impl PmExecutionLeaseReadback {
    pub fn verified(
        value: Value,
        reservation: &TrackerPmDraftReservation,
        expected_operation: Option<&PmExecutionLeaseCommand>,
    ) -> Result<Self, AppError> {
        let readback: Self = serde_json::from_value(value.clone()).map_err(|_| inconsistent())?;
        // Also rejects missing nullable fields and normalized UUID/timestamp spellings.
        if serde_json::to_value(&readback).map_err(|_| inconsistent())? != value {
            return Err(inconsistent());
        }
        let fence = PmExecutionLeaseFence {
            assignment_id: reservation.assignment.assignment_id,
            execution_id: reservation.assignment.execution_id,
            agent_id: reservation.assignment.agent_id,
            assignment_version: reservation.assignment.version,
        };
        if reservation.contract_version != 1
            || reservation.variant != "pm_draft_reserved"
            || reservation.admission_state != "reserved"
            || reservation.dispatch_allowed
            || !positive_safe(reservation.owner_cas.version)
            || !positive_safe(fence.assignment_version)
            || [fence.assignment_id, fence.execution_id, fence.agent_id]
                .iter()
                .any(Uuid::is_nil)
            || [
                reservation.binding.project_id,
                reservation.binding.task_id,
                reservation.binding.root_task_id,
            ]
            .iter()
            .any(Uuid::is_nil)
            || reservation.binding.tracker_instance_id.is_empty()
            || reservation.binding.owner_subject.is_empty()
            || reservation.assignment.machine_subject.trim().is_empty()
            || reservation.assignment.machine_subject == reservation.binding.owner_subject
            || readback.contract_version != 1
            || readback.binding != reservation.binding
            || readback.owner_version != reservation.owner_cas.version
            || readback.fence != fence
            || readback.dispatch_allowed
            || !valid_timestamp(readback.observed_at)
        {
            return Err(inconsistent());
        }
        if let Some(command) = expected_operation {
            command.validate(readback.owner_version, &fence)?;
        }
        let actual_state = match &readback.current {
            None => PmExecutionLeaseState::Unclaimed,
            Some(current) => {
                validate_lease(current, reservation, readback.observed_at)?;
                if current.expires_at > readback.observed_at {
                    PmExecutionLeaseState::Active
                } else {
                    PmExecutionLeaseState::Expired
                }
            }
        };
        if readback.state != actual_state {
            return Err(inconsistent());
        }
        match (&readback.operation, expected_operation) {
            (None, _) => (),
            (Some(_), None) => return Err(inconsistent()),
            (Some(operation), Some(command)) => {
                let receipt = &operation.result;
                let current = readback.current.as_ref().ok_or_else(inconsistent)?;
                validate_lease(&receipt.lease, reservation, readback.observed_at)?;
                if operation.idempotency_key != command.idempotency_key()
                    || operation.request_sha256 != command.request_sha256()
                    || receipt.contract_version != 1
                    || receipt.binding != readback.binding
                    || receipt.owner_version != readback.owner_version
                    || receipt.fence != fence
                    || receipt.ttl_seconds != TTL_SECONDS
                    || receipt.heartbeat_seconds != HEARTBEAT_SECONDS
                    || receipt.dispatch_allowed
                    || receipt.lease.lease_id != current.lease_id
                    || receipt.lease.claimed_at != current.claimed_at
                    || receipt.lease.version > current.version
                    || receipt.lease.heartbeat_at > current.heartbeat_at
                    || (receipt.lease.version == current.version && receipt.lease != *current)
                {
                    return Err(inconsistent());
                }
                match command {
                    PmExecutionLeaseCommand::Claim(_) if receipt.lease.version != 1 => {
                        return Err(inconsistent());
                    }
                    PmExecutionLeaseCommand::Heartbeat(c)
                        if receipt.lease.lease_id != c.lease_id
                            || c.expected_lease_version.checked_add(1)
                                != Some(receipt.lease.version) =>
                    {
                        return Err(inconsistent());
                    }
                    _ => (),
                }
            }
        }
        Ok(readback)
    }
}

fn validate_lease(
    lease: &PmExecutionLease,
    reservation: &TrackerPmDraftReservation,
    observed: DateTime<Utc>,
) -> Result<(), AppError> {
    if lease.lease_id.is_nil()
        || !positive_safe(lease.version)
        || lease.holder_subject != reservation.assignment.machine_subject
        || ![lease.claimed_at, lease.heartbeat_at, lease.expires_at]
            .into_iter()
            .all(valid_timestamp)
        || lease.claimed_at > lease.heartbeat_at
        || lease.heartbeat_at > observed
        || (lease.version == 1 && lease.heartbeat_at != lease.claimed_at)
        || lease
            .heartbeat_at
            .checked_add_signed(Duration::seconds(TTL_SECONDS))
            != Some(lease.expires_at)
    {
        return Err(inconsistent());
    }
    Ok(())
}

fn valid_timestamp(value: DateTime<Utc>) -> bool {
    // PostgreSQL clock_timestamp does not produce chrono's leap-second representation.
    value.timestamp_subsec_nanos() < 1_000_000_000
}

fn positive_safe(value: u64) -> bool {
    (1..=MAX_SAFE_VERSION).contains(&value)
}

fn safe_version<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let value = u64::deserialize(d)?;
    if positive_safe(value) {
        Ok(value)
    } else {
        Err(D::Error::custom("invalid execution lease version"))
    }
}

fn canonical_uuid<'de, D: Deserializer<'de>>(d: D) -> Result<Uuid, D::Error> {
    let raw = String::deserialize(d)?;
    let id = Uuid::parse_str(&raw).map_err(D::Error::custom)?;
    if id.is_nil() || id.to_string() != raw {
        Err(D::Error::custom("invalid execution lease reference"))
    } else {
        Ok(id)
    }
}

fn required_nullable<'de, D, T>(d: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
}

fn utc_nanos<S: Serializer>(value: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&value.to_rfc3339_opts(SecondsFormat::Nanos, true))
}

fn no_dispatch<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    if bool::deserialize(d)? {
        Err(D::Error::custom(
            "execution lease cannot authorize dispatch",
        ))
    } else {
        Ok(false)
    }
}

fn inconsistent() -> AppError {
    AppError::conflict(
        "PM execution lease proof is stale or inconsistent; reconcile before continuing",
    )
}

#[cfg(test)]
#[path = "pm_execution_lease_tests.rs"]
mod tests;
