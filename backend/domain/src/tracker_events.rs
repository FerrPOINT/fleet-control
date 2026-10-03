use crate::{TaskChatBinding, TrackerStage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::AppError;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerOutboxEvent {
    pub sequence: i64,
    pub event_id: Uuid,
    pub task_id: Uuid,
    pub event_type: String,
    pub payload: TrackerEventPayload,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerEventPayload {
    pub contract_version: u8,
    pub tracker_instance_id: String,
    pub project_id: Uuid,
    pub task_id: Uuid,
    pub root_task_id: Uuid,
    pub owner_subject: String,
    pub stage: TrackerStage,
    pub requirement_revision: Option<i64>,
    pub result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerOutboxPage {
    pub events: Vec<TrackerOutboxEvent>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TrackerProjectionReceipt {
    pub cursor: i64,
    pub projected: usize,
}

impl TrackerOutboxPage {
    pub fn validate(&self, binding: &TaskChatBinding, after: i64) -> Result<(), AppError> {
        if after < 0 || self.events.len() > 100 {
            return Err(AppError::validation("invalid Tracker event page bounds"));
        }
        if serde_json::to_vec(self).map_err(AppError::internal)?.len() > 1_048_576 {
            return Err(AppError::validation("Tracker event page exceeds limit"));
        }
        let mut previous = after;
        let mut identities = std::collections::HashSet::new();
        for event in &self.events {
            let payload = &event.payload;
            if event.sequence <= previous
                || event.event_id.is_nil()
                || !identities.insert(event.event_id)
                || event.task_id != binding.task_id
                || payload.contract_version != 1
                || payload.tracker_instance_id != binding.tracker_instance_id
                || payload.project_id != binding.project_id
                || payload.task_id != binding.task_id
                || payload.root_task_id != binding.root_task_id
                || payload.owner_subject != binding.owner_subject
                || payload.requirement_revision.is_some_and(|value| value <= 0)
            {
                return Err(AppError::conflict(
                    "Tracker event identity or cursor mismatch",
                ));
            }
            event.summary()?;
            previous = event.sequence;
        }
        Ok(())
    }
}

impl TrackerOutboxEvent {
    // Transcript projections never copy an arbitrary result, answer or credential.
    pub fn summary(&self) -> Result<&'static str, AppError> {
        summary(&self.event_type)
    }
}

pub(crate) fn summary(event_type: &str) -> Result<&'static str, AppError> {
    match event_type {
        "task.created" => Ok("Tracker draft created."),
        "task.bound" => Ok("Tracker task binding saved."),
        "pm.assigned" => Ok("Project Manager assignment saved."),
        "clarification.published" => Ok("Clarification published in Tracker."),
        "clarification.answered" => {
            Ok("Clarification answer saved in Tracker; PM delivery is separate.")
        }
        "clarification.cancelled" => Ok("Clarification cancelled in Tracker."),
        "requirements.published" => Ok("Requirements revision published in Tracker."),
        "requirements.evidence_recorded" => Ok("Requirements evidence recorded in Tracker."),
        "requirements.confirmed" => Ok("Requirements revision confirmed by its owner."),
        "analysis.intent_created" => {
            Ok("Analysis queued in Tracker; runtime admission is separate.")
        }
        "analysis.assignment_reserved" => {
            Ok("Analysis assignment reserved in Tracker; runtime dispatch is not allowed yet.")
        }
        _ => Err(AppError::Unavailable(
            "unsupported Tracker event type".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> (TaskChatBinding, TrackerOutboxPage) {
        let task = Uuid::new_v4();
        let binding = TaskChatBinding {
            tracker_instance_id: "tracker-test".into(),
            project_id: Uuid::new_v4(),
            task_id: task,
            root_task_id: task,
            agent_id: Uuid::new_v4(),
            owner_subject: "central-owner".into(),
        };
        let event = TrackerOutboxEvent {
            sequence: 7,
            event_id: Uuid::new_v4(),
            task_id: task,
            event_type: "clarification.answered".into(),
            payload: TrackerEventPayload {
                contract_version: 1,
                tracker_instance_id: binding.tracker_instance_id.clone(),
                project_id: binding.project_id,
                task_id: task,
                root_task_id: task,
                owner_subject: binding.owner_subject.clone(),
                stage: TrackerStage::Draft,
                requirement_revision: Some(1),
                result: serde_json::json!({"text":"sk-private-answer"}),
            },
            created_at: Utc::now(),
        };
        (
            binding,
            TrackerOutboxPage {
                events: vec![event],
            },
        )
    }

    #[test]
    fn page_accepts_global_sequence_gaps_but_rejects_reordering_and_identity_changes() {
        let (binding, page) = sample();
        page.validate(&binding, 1).unwrap();
        assert!(page.validate(&binding, 7).is_err());
        assert!(page.validate(&binding, -1).is_err());
        let mut duplicate = page.clone();
        duplicate.events.push(duplicate.events[0].clone());
        assert!(duplicate.validate(&binding, 0).is_err());
        for mutation in 0..7 {
            let mut changed = page.clone();
            let event = &mut changed.events[0];
            match mutation {
                0 => event.event_id = Uuid::nil(),
                1 => event.task_id = Uuid::new_v4(),
                2 => event.payload.tracker_instance_id = "foreign".into(),
                3 => event.payload.project_id = Uuid::new_v4(),
                4 => event.payload.owner_subject = "foreign".into(),
                5 => event.payload.requirement_revision = Some(0),
                _ => event.event_type = "requirements.automatically_confirmed".into(),
            }
            assert!(changed.validate(&binding, 0).is_err());
        }
    }

    #[test]
    fn projection_summary_never_claims_runtime_delivery_or_copies_result() {
        let (_, page) = sample();
        let summary = page.events[0].summary().unwrap();
        assert!(summary.contains("delivery is separate"));
        assert!(!summary.contains("sk-private-answer"));
    }

    #[test]
    fn unknown_wire_fields_and_large_pages_fail_closed() {
        let (binding, mut page) = sample();
        let mut value = serde_json::to_value(&page).unwrap();
        value["cursor"] = serde_json::json!(9);
        assert!(serde_json::from_value::<TrackerOutboxPage>(value).is_err());
        page.events[0].payload.result = Value::String("x".repeat(1_048_576));
        assert!(page.validate(&binding, 0).is_err());
    }
}
