use crate::{ClarificationAnswerRequest, TaskChatBinding, TrackerAnswer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::AppError;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ClarificationCommandActor {
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub subject: String,
    pub binding: TaskChatBinding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClarificationDeliveryState {
    Stored,
    Delivering,
    Uncertain,
    Delivered,
    Rejected,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClarificationContinuationState {
    #[default]
    NotRequired,
    Pending,
    Confirmed,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClarificationAnswerCommand {
    pub id: Uuid,
    pub session_id: Uuid,
    pub question_id: Uuid,
    pub request: ClarificationAnswerRequest,
    pub payload_sha256: String,
    pub state: ClarificationDeliveryState,
    #[serde(default)]
    pub continuation_state: ClarificationContinuationState,
    pub answer: Option<TrackerAnswer>,
    pub rejection_status: Option<u16>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct ClarificationDeliveryPermit {
    pub command: ClarificationAnswerCommand,
    pub attempt_id: Option<Uuid>,
}

pub enum ClarificationDeliveryOutcome {
    Delivered(TrackerAnswer),
    Rejected(u16),
    Uncertain,
}

pub fn canonical_answer_request(
    mut request: ClarificationAnswerRequest,
) -> Result<String, AppError> {
    if !crate::valid_ref(&request.idempotency_key, 128)
        || !(1..=9_007_199_254_740_991).contains(&request.expected_question_version)
        || !(1..=9_007_199_254_740_991).contains(&request.requirement_revision)
        || request.selected_option_ids.len() > 128
        || request.selected_option_ids.iter().any(Uuid::is_nil)
        || request.text.as_ref().is_some_and(|s| s.len() > 65_536)
        || request.comment.as_ref().is_some_and(|s| s.len() > 65_536)
    {
        return Err(AppError::validation("invalid clarification command"));
    }
    request.selected_option_ids.sort();
    if request
        .selected_option_ids
        .windows(2)
        .any(|ids| ids[0] == ids[1])
    {
        return Err(AppError::validation("duplicate clarification option"));
    }
    let body = serde_json::to_string(&request).map_err(AppError::internal)?;
    if body.len() > 300_000 {
        return Err(AppError::validation("clarification command exceeds limit"));
    }
    Ok(body)
}

pub fn answer_payload_hash(body: &str) -> String {
    hex::encode(Sha256::digest(body.as_bytes()))
}

pub fn answer_matches_command(
    command: &ClarificationAnswerCommand,
    answer: &TrackerAnswer,
    subject: &str,
) -> bool {
    let mut options = answer.selected_option_ids.clone();
    options.sort();
    !answer.id.is_nil()
        && answer.question_id == command.question_id
        && answer.author_subject == subject
        && answer.question_version == command.request.expected_question_version
        && answer.requirement_revision == command.request.requirement_revision
        && options == command.request.selected_option_ids
        && answer.text == command.request.text
        && answer.comment == command.request.comment
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ClarificationAnswerRequest {
        ClarificationAnswerRequest {
            expected_question_version: 1,
            requirement_revision: 2,
            selected_option_ids: vec![Uuid::from_u128(2), Uuid::from_u128(1)],
            text: Some("original Unicode: \u{442}\u{435}\u{43a}\u{441}\u{442}".into()),
            comment: None,
            idempotency_key: "original-key".into(),
        }
    }

    #[test]
    fn canonical_body_retains_original_key_and_text_but_sorts_option_set() {
        let first = canonical_answer_request(request()).unwrap();
        let mut reversed = request();
        reversed.selected_option_ids.reverse();
        assert_eq!(first, canonical_answer_request(reversed).unwrap());
        assert_eq!(
            serde_json::from_str::<ClarificationAnswerRequest>(&first)
                .unwrap()
                .text,
            request().text
        );
        let mut changed = request();
        changed.idempotency_key = "new-key".into();
        assert_ne!(
            answer_payload_hash(&first),
            answer_payload_hash(&canonical_answer_request(changed).unwrap())
        );
    }

    #[test]
    fn unsafe_versions_keys_and_duplicate_options_are_rejected() {
        for value in [0, -1, 9_007_199_254_740_992] {
            let mut invalid = request();
            invalid.expected_question_version = value;
            assert!(canonical_answer_request(invalid).is_err());
        }
        let mut duplicate = request();
        duplicate
            .selected_option_ids
            .push(duplicate.selected_option_ids[0]);
        assert!(canonical_answer_request(duplicate).is_err());
        let mut invalid = request();
        invalid.idempotency_key = " ".into();
        assert!(canonical_answer_request(invalid).is_err());
    }
}
