use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::AppError;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreatePmDraftRequest {
    pub agent_id: Uuid,
    pub title: String,
    pub description: String,
    pub idempotency_key: String,
}

impl CreatePmDraftRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.agent_id.is_nil()
            || self.title.trim().is_empty()
            || self.title.chars().count() > 500
            || self.title.chars().any(char::is_control)
            || self.description.chars().count() > 100_000
            || self.description.contains('\0')
            || !valid_key(&self.idempotency_key)
        {
            return Err(AppError::validation("invalid PM Draft request"));
        }
        Ok(())
    }
}

pub fn valid_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.chars().any(|c| c.is_whitespace() || c.is_control())
}

fn subject(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|id| !id.is_nil() && id.to_string() == value)
}

pub fn pm_input_hash(title: &str, description: &str) -> String {
    // Tracker's immutable input uses lexicographically ordered JSON keys and exact UTF-8.
    pm_canonical_hash(&serde_json::json!({"description":description,"title":title}))
}

pub fn pm_canonical_hash(value: &serde_json::Value) -> String {
    fn sort(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                let ordered: std::collections::BTreeMap<_, _> = map
                    .iter()
                    .map(|(key, value)| (key.clone(), sort(value)))
                    .collect();
                serde_json::to_value(ordered).expect("JSON object serialization cannot fail")
            }
            serde_json::Value::Array(items) => items.iter().map(sort).collect(),
            other => other.clone(),
        }
    }
    hex::encode(Sha256::digest(
        serde_json::to_vec(&sort(value)).expect("JSON serialization cannot fail"),
    ))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerCreatedDraft {
    pub tracker_instance_id: String,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub root_task_id: Uuid,
    pub task_key: String,
    pub owner_subject: String,
    pub stage: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerDraftIdentity {
    pub tracker_instance_id: String,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub root_task_id: Uuid,
    pub owner_subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerDraftInput {
    pub snapshot_ref: Uuid,
    pub title: String,
    pub description: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerDraftInputReceipt {
    pub contract_version: u8,
    pub tracker_instance_id: String,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub root_task_id: Uuid,
    pub owner_subject: String,
    pub input: TrackerDraftInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerOwnerCas {
    pub expected_version: u64,
    pub version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerDraftAssignment {
    pub assignment_id: Uuid,
    pub execution_id: Uuid,
    pub agent_id: Uuid,
    pub version: u64,
    pub machine_subject: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerDraftExecution {
    pub ordinal: String,
    pub key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerDraftInputRef {
    pub snapshot_ref: Uuid,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerPmDraftReservation {
    pub contract_version: u8,
    pub variant: String,
    pub binding: TrackerDraftIdentity,
    pub owner_cas: TrackerOwnerCas,
    pub assignment: TrackerDraftAssignment,
    pub execution: TrackerDraftExecution,
    pub input: TrackerDraftInputRef,
    pub assignment_operation_key: String,
    pub admission_state: String,
    pub dispatch_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerDraftReservationOperation {
    pub idempotency_key: String,
    pub request_sha256: String,
    pub result: TrackerPmDraftReservation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerDraftReservationReadback {
    pub contract_version: u8,
    pub binding: TrackerDraftIdentity,
    pub owner_version: u64,
    pub current: Option<TrackerPmDraftReservation>,
    pub operation: Option<TrackerDraftReservationOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmDraftOperation {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub owner_subject: String,
    pub tracker_instance_id: String,
    pub project_id: Uuid,
    pub request: CreatePmDraftRequest,
    pub draft: Option<TrackerCreatedDraft>,
    pub input: Option<TrackerDraftInputReceipt>,
    pub reservation: Option<TrackerPmDraftReservation>,
    pub session_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credentials: Option<crate::PmCredentialJournal>,
}

#[derive(Debug, Clone)]
pub enum PmDraftProof {
    Created(TrackerCreatedDraft),
    Input(TrackerDraftInputReceipt),
    Reserved(TrackerPmDraftReservation),
    Chat(Uuid),
    CredentialIntent(crate::PmCredentialIntent),
    CredentialAcknowledged(crate::PmCredentialReceipt),
}

impl PmDraftOperation {
    pub fn creation_key(&self) -> String {
        format!("fleet-pm-create:{}", self.id)
    }
    pub fn reservation_key(&self) -> String {
        format!("fleet-pm-reserve:{}", self.id)
    }
    pub fn chat_key(&self) -> String {
        format!("fleet-pm-chat:{}", self.id)
    }
    pub fn credential_key(&self) -> String {
        format!("fleet-pm-credential:{}", self.id)
    }
    pub fn execution_identity(&self) -> Result<crate::PmExecutionIdentity, AppError> {
        let reservation = self.reservation.as_ref().ok_or_else(inconsistent)?;
        let binding = self.identity()?;
        let identity = crate::PmExecutionIdentity {
            task: reservation.execution.key.clone(),
            execution_ref: reservation.assignment.execution_id.to_string(),
            tracker_instance_ref: binding.tracker_instance_id,
            tracker_project_ref: binding.project_id.to_string(),
            task_ref: binding.task_id.to_string(),
            root_ref: binding.root_task_id.to_string(),
            agent_ref: reservation.assignment.agent_id.to_string(),
            assignment_operation_key: reservation.assignment_operation_key.clone(),
            assignment_ref: reservation.assignment.assignment_id.to_string(),
            assignment_revision: i64::try_from(reservation.assignment.version)
                .map_err(|_| inconsistent())?,
        };
        identity.validate()?;
        Ok(identity)
    }
    pub fn identity(&self) -> Result<TrackerDraftIdentity, AppError> {
        let draft = self.draft.as_ref().ok_or_else(inconsistent)?;
        Ok(TrackerDraftIdentity {
            tracker_instance_id: self.tracker_instance_id.clone(),
            project_id: self.project_id,
            task_id: draft.task_id,
            root_task_id: draft.task_id,
            owner_subject: self.owner_subject.clone(),
        })
    }
    pub fn reservation_body(&self) -> serde_json::Value {
        serde_json::json!({"expected_owner_version":0,"expected_assignment_version":null,
            "requested_agent_id":self.request.agent_id,"idempotency_key":self.reservation_key()})
    }
    pub fn apply(&mut self, proof: PmDraftProof) -> Result<(), AppError> {
        match proof {
            PmDraftProof::Created(draft) => {
                if draft.tracker_instance_id != self.tracker_instance_id
                    || draft.project_id != self.project_id
                    || draft.owner_subject != self.owner_subject
                    || !subject(&draft.owner_subject)
                    || draft.task_id.is_nil()
                    || draft.root_task_id != draft.task_id
                    || draft.stage != "Draft"
                    || !valid_key(&draft.task_key)
                {
                    return Err(inconsistent());
                }
                retain(&mut self.draft, draft)?;
            }
            PmDraftProof::Input(input) => {
                let identity = self.identity()?;
                if input.contract_version != 1
                    || input.tracker_instance_id != identity.tracker_instance_id
                    || input.project_id != identity.project_id
                    || input.task_id != identity.task_id
                    || input.root_task_id != identity.root_task_id
                    || input.owner_subject != identity.owner_subject
                    || input.input.snapshot_ref.is_nil()
                    || input.input.title != self.request.title
                    || input.input.description != self.request.description
                    || input.input.sha256
                        != pm_input_hash(&self.request.title, &self.request.description)
                {
                    return Err(inconsistent());
                }
                retain(&mut self.input, input)?;
            }
            PmDraftProof::Reserved(reservation) => {
                let input = &self.input.as_ref().ok_or_else(inconsistent)?.input;
                let ordinal = reservation.execution.ordinal.parse::<i64>().ok();
                if reservation.contract_version != 1
                    || reservation.variant != "pm_draft_reserved"
                    || reservation.binding != self.identity()?
                    || reservation.owner_cas
                        != (TrackerOwnerCas {
                            expected_version: 0,
                            version: 1,
                        })
                    || reservation.assignment.assignment_id.is_nil()
                    || reservation.assignment.execution_id.is_nil()
                    || reservation.assignment.agent_id != self.request.agent_id
                    || reservation.assignment.version != 1
                    || reservation.assignment.machine_subject.trim().is_empty()
                    || reservation.assignment.machine_subject == self.owner_subject
                    || ordinal
                        .is_none_or(|n| n <= 0 || n.to_string() != reservation.execution.ordinal)
                    || reservation.execution.key
                        != format!("SDLC-{}", reservation.execution.ordinal)
                    || reservation.input.snapshot_ref != input.snapshot_ref
                    || reservation.input.sha256 != input.sha256
                    || reservation.assignment_operation_key
                        != format!("pm-draft:{}", reservation.assignment.assignment_id)
                    || reservation.admission_state != "reserved"
                    || reservation.dispatch_allowed
                {
                    return Err(inconsistent());
                }
                retain(&mut self.reservation, reservation)?;
            }
            PmDraftProof::Chat(id) => {
                if self.reservation.is_none() || id.is_nil() {
                    return Err(inconsistent());
                }
                retain(&mut self.session_id, id)?;
            }
            PmDraftProof::CredentialIntent(intent) => {
                let ttl = intent
                    .command
                    .get("expires_in_seconds")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(inconsistent)?;
                let command = crate::PmCredentialCommand::tracker(
                    &self.execution_identity()?,
                    self.credential_key(),
                    ttl,
                )?;
                if self.session_id.is_none()
                    || intent.command
                        != serde_json::to_value(command).map_err(AppError::internal)?
                    || intent.request_sha256 != pm_canonical_hash(&intent.command)
                    || intent.parent_fingerprint.len() != 64
                    || !intent
                        .parent_fingerprint
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    || !subject(&intent.machine_subject)
                    || intent.machine_subject
                        != self
                            .reservation
                            .as_ref()
                            .ok_or_else(inconsistent)?
                            .assignment
                            .machine_subject
                    || !crate::pm_execution::valid_ref(&intent.base_origin, 1024)
                    || !crate::pm_execution::valid_ref(&intent.tracker_origin, 1024)
                {
                    return Err(inconsistent());
                }
                match &self.credentials {
                    Some(old) if old.intent != intent => return Err(inconsistent()),
                    Some(_) => (),
                    None => {
                        self.credentials = Some(crate::PmCredentialJournal {
                            intent,
                            receipt: None,
                        })
                    }
                }
            }
            PmDraftProof::CredentialAcknowledged(receipt) => {
                let journal = self.credentials.as_mut().ok_or_else(inconsistent)?;
                if receipt.token_id.is_nil()
                    || serde_json::to_value(&receipt.scopes).map_err(AppError::internal)?
                        != journal.intent.command["scopes"]
                    || receipt.expires_at <= chrono::Utc::now()
                {
                    return Err(inconsistent());
                }
                retain(&mut journal.receipt, receipt)?;
            }
        }
        Ok(())
    }
    pub fn check_current(
        &self,
        readback: &TrackerDraftReservationReadback,
    ) -> Result<(), AppError> {
        if readback.contract_version != 1 || readback.binding != self.identity()? {
            return Err(inconsistent());
        }
        match (&readback.current, &readback.operation) {
            (None, None) if readback.owner_version == 0 && self.reservation.is_none() => Ok(()),
            (Some(current), Some(operation)) => {
                let mut candidate = self.clone();
                candidate.apply(PmDraftProof::Reserved(operation.result.clone()))?;
                if current != &operation.result
                    || readback.owner_version != current.owner_cas.version
                    || operation.idempotency_key != self.reservation_key()
                    || operation.request_sha256
                        != pm_canonical_hash(
                            &serde_json::json!({"operation":"reserve_pm_draft","payload":self.reservation_body()}),
                        )
                {
                    return Err(inconsistent());
                }
                Ok(())
            }
            _ => Err(inconsistent()),
        }
    }
    pub fn response(&self) -> PmDraftCreationResponse {
        PmDraftCreationResponse {
            operation_id: self.id,
            project_id: self.project_id,
            agent_id: self.request.agent_id,
            task_id: self.draft.as_ref().map(|draft| draft.task_id),
            session_id: self.session_id,
            state: if self.session_id.is_some() {
                PmDraftCreationState::AwaitingAdmission
            } else {
                PmDraftCreationState::Incomplete
            },
            next_step: if self.draft.is_none() {
                PmDraftCreationStep::Draft
            } else if self.input.is_none() {
                PmDraftCreationStep::Input
            } else if self.reservation.is_none() {
                PmDraftCreationStep::Reservation
            } else if self.session_id.is_none() {
                PmDraftCreationStep::Chat
            } else {
                PmDraftCreationStep::Admission
            },
            dispatch_allowed: false,
        }
    }
}

fn retain<T: PartialEq>(target: &mut Option<T>, value: T) -> Result<(), AppError> {
    if target.as_ref().is_some_and(|old| old != &value) {
        return Err(inconsistent());
    }
    *target = Some(value);
    Ok(())
}

fn inconsistent() -> AppError {
    AppError::conflict("PM Draft proof is stale or inconsistent; reconcile before continuing")
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PmDraftCreationState {
    Incomplete,
    AwaitingAdmission,
    AwaitingRuntimeAcceptance,
    RuntimeAccepted,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PmDraftCreationStep {
    Draft,
    Input,
    Reservation,
    Chat,
    Admission,
    Runtime,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PmDraftCreationResponse {
    pub operation_id: Uuid,
    pub project_id: Uuid,
    pub agent_id: Uuid,
    pub task_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub state: PmDraftCreationState,
    pub next_step: PmDraftCreationStep,
    pub dispatch_allowed: bool,
}
