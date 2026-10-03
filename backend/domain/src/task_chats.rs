use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TaskChatBinding {
    pub tracker_instance_id: String,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub root_task_id: Uuid,
    pub agent_id: Uuid,
    pub owner_subject: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreatePmDraftChat {
    pub binding: TaskChatBinding,
    pub title: String,
    pub task_key: String,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BindTaskChatRequest {
    pub task_id: Uuid,
    pub idempotency_key: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SessionTaskContext {
    pub binding: Option<TaskChatBinding>,
    pub tracker: Option<TrackerTaskContext>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageHistoryPage {
    pub items: Vec<crate::SessionMessage>,
    pub next_before: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChatControls {
    pub can_send: bool,
    pub can_steer: bool,
    pub can_stop: bool,
    pub active_run_id: Option<Uuid>,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ClarificationAnswerRequest {
    pub expected_question_version: i64,
    pub requirement_revision: i64,
    pub selected_option_ids: Vec<Uuid>,
    pub text: Option<String>,
    pub comment: Option<String>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfirmRequirementsRequest {
    pub content_hash: String,
    pub idempotency_key: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "routing_version"
    )]
    #[schema(minimum = 1, maximum = 9007199254740991i64)]
    pub expected_routing_policy_version: Option<i64>,
}

fn routing_version<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
    let value = Option::<i64>::deserialize(d)?;
    if value.is_some_and(|n| !(1..=9_007_199_254_740_991).contains(&n)) {
        return Err(serde::de::Error::custom("invalid routing policy version"));
    }
    Ok(value)
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum TrackerStage {
    Draft,
    Clarification,
    Backlog,
    Analysis,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TrackerQuestionMode {
    Single,
    Multiple,
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TrackerQuestionState {
    Open,
    Answered,
    Superseded,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrackerPmAssignment {
    pub assignment_id: Uuid,
    pub execution_id: Uuid,
    pub agent_id: Uuid,
    pub version: i64,
    pub machine_subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrackerPermissions {
    pub can_answer: bool,
    pub can_confirm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrackerTaskContext {
    pub contract_version: u8,
    pub tracker_instance_id: String,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub root_task_id: Uuid,
    pub owner_subject: String,
    pub stage: TrackerStage,
    pub requirement_revision: Option<i64>,
    pub waiting_reason: Option<String>,
    pub permissions: TrackerPermissions,
    pub assignment: Option<TrackerPmAssignment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrackerQuestionOption {
    pub id: Uuid,
    pub label: String,
    pub consequences: String,
    #[serde(default)]
    pub is_custom: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrackerAnswer {
    pub id: Uuid,
    pub question_id: Uuid,
    pub question_version: i64,
    pub requirement_revision: i64,
    pub selected_option_ids: Vec<Uuid>,
    pub text: Option<String>,
    pub comment: Option<String>,
    pub author_subject: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrackerQuestion {
    pub id: Uuid,
    pub request_id: Uuid,
    pub task_id: Uuid,
    pub root_task_id: Uuid,
    pub version: i64,
    pub requirement_revision: i64,
    pub requirement_reference: Option<String>,
    pub assignment_id: Uuid,
    pub execution_id: Uuid,
    pub agent_id: Uuid,
    pub assignment_version: i64,
    pub checkpoint_id: Uuid,
    pub text: String,
    pub rationale: String,
    pub required: bool,
    pub mode: TrackerQuestionMode,
    pub options: Vec<TrackerQuestionOption>,
    pub recommended_option_id: Option<Uuid>,
    pub state: TrackerQuestionState,
    pub answer: Option<TrackerAnswer>,
    pub author_subject: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrackerRequirementsRevision {
    pub revision: i64,
    pub content_hash: String,
    pub goal: String,
    pub scope: Vec<String>,
    pub exclusions: Vec<String>,
    pub scenarios: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub constraints: Vec<String>,
    pub dependencies: Vec<String>,
    pub assumptions: Vec<String>,
    pub checklist: Vec<String>,
    pub prerequisites: Vec<String>,
    pub author_subject: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrackerConfirmation {
    pub id: Uuid,
    pub task_id: Uuid,
    pub revision: i64,
    pub content_hash: String,
    pub owner_subject: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub stage: TrackerStage,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TrackerClarifications {
    pub questions: Vec<TrackerQuestion>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TrackerRequirements {
    pub revisions: Vec<TrackerRequirementsRevision>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn confirmation_routing_opt_in_is_explicit_and_preserves_legacy_wire() {
        let legacy = json!({"content_hash":"a".repeat(64),"idempotency_key":"confirm-fixture"});
        let parsed: ConfirmRequirementsRequest = serde_json::from_value(legacy.clone()).unwrap();
        assert!(parsed.expected_routing_policy_version.is_none());
        assert_eq!(serde_json::to_value(parsed).unwrap(), legacy);
        for value in [
            json!(1),
            json!(9_007_199_254_740_991i64),
            serde_json::Value::Null,
        ] {
            let mut raw = legacy.clone();
            raw["expected_routing_policy_version"] = value.clone();
            let parsed: ConfirmRequirementsRequest = serde_json::from_value(raw).unwrap();
            assert_eq!(parsed.expected_routing_policy_version, value.as_i64());
        }
        for value in [
            json!(0),
            json!(-1),
            json!(9_007_199_254_740_992i64),
            json!("1"),
            json!(1.0),
            json!(true),
        ] {
            let mut raw = legacy.clone();
            raw["expected_routing_policy_version"] = value;
            assert!(serde_json::from_value::<ConfirmRequirementsRequest>(raw).is_err());
        }
    }
}
