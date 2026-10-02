use crate::{TaskChatBinding, TrackerStage};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use shared::AppError;
use uuid::Uuid;

pub const TRACKER_METADATA_BUDGET: usize = 262_144;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerMetadataPage {
    pub contract_version: u8,
    pub projection: String,
    pub after: String,
    pub next_after: String,
    pub has_more: bool,
    pub events: Vec<TrackerMetadataEvent>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerMetadataEvent {
    pub sequence: String,
    pub event_id: Uuid,
    pub task_id: Uuid,
    pub event_type: String,
    pub created_at: String,
    pub metadata_sha256: String,
    pub payload: TrackerMetadataPayload,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrackerMetadataPayload {
    pub tracker_instance_id: String,
    pub project_id: Uuid,
    pub root_task_id: Uuid,
    pub owner_subject: String,
    pub stage: TrackerStage,
    pub current_requirement_revision: Option<i64>,
    pub resource: Value,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputRef {
    snapshot_ref: Uuid,
    sha256: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CreatedResource {
    input: Option<InputRef>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundResource {}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MetadataFence {
    assignment_id: Uuid,
    execution_id: Uuid,
    agent_id: Uuid,
    assignment_version: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestionResource {
    question_id: Uuid,
    question_version: i64,
    request_id: Uuid,
    checkpoint_id: Uuid,
    requirement_revision: i64,
    state: String,
    fence: MetadataFence,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AnswerResource {
    answer_id: Uuid,
    question_id: Uuid,
    question_version: i64,
    request_id: Uuid,
    checkpoint_id: Uuid,
    requirement_revision: i64,
    fence: MetadataFence,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RevisionResource {
    requirement_revision: i64,
    content_hash: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidenceResource {
    requirement_revision: i64,
    content_hash: String,
    check_id_sha256: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmationResource {
    confirmation_id: Uuid,
    requirement_revision: i64,
    content_hash: String,
}

fn invalid() -> AppError {
    AppError::Unavailable("Tracker metadata response is invalid".into())
}

pub fn tracker_metadata_cursor(value: &str) -> Result<i64, AppError> {
    let number = value.parse::<i64>().map_err(|_| invalid())?;
    if number < 0 || number.to_string() != value {
        return Err(invalid());
    }
    Ok(number)
}

fn version(value: i64) -> Result<(), AppError> {
    if !(1..=9_007_199_254_740_991).contains(&value) {
        return Err(invalid());
    }
    Ok(())
}

fn strict<T: DeserializeOwned + Serialize>(raw: &Value) -> Result<T, AppError> {
    let typed: T = serde_json::from_value(raw.clone()).map_err(|_| invalid())?;
    // Roundtrip equality enforces required null fields and canonical UUID strings.
    if serde_json::to_value(&typed).map_err(|_| invalid())? != *raw {
        return Err(invalid());
    }
    Ok(typed)
}

fn validate_refs(value: &Value) -> Result<(), AppError> {
    let object = value.as_object().ok_or_else(invalid)?;
    for (name, field) in object {
        if name == "fence" {
            validate_refs(field)?;
        } else if name == "input" {
            if !field.is_null() {
                validate_refs(field)?;
            }
        } else if name.ends_with("_id") || name == "snapshot_ref" {
            let raw = field.as_str().ok_or_else(invalid)?;
            let id = Uuid::parse_str(raw).map_err(|_| invalid())?;
            if id.is_nil() || id.to_string() != raw {
                return Err(invalid());
            }
        } else if name.ends_with("_version") || name == "requirement_revision" {
            version(field.as_i64().ok_or_else(invalid)?)?;
        } else if name.ends_with("_hash") || name.ends_with("sha256") {
            let hash = field.as_str().ok_or_else(invalid)?;
            if hash.len() != 64
                || !hash
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

impl TrackerMetadataPage {
    pub fn decode(bytes: &[u8], binding: &TaskChatBinding, after: i64) -> Result<Self, AppError> {
        if bytes.len() > TRACKER_METADATA_BUDGET {
            return Err(invalid());
        }
        let raw: Value = serde_json::from_slice(bytes).map_err(|_| invalid())?;
        let page: Self = strict(&raw)?;
        page.validate(binding, after)?;
        Ok(page)
    }

    pub fn validate(&self, binding: &TaskChatBinding, after: i64) -> Result<(), AppError> {
        if self.contract_version != 1
            || self.projection != "metadata_v1"
            || tracker_metadata_cursor(&self.after)? != after
            || self.events.len() > 100
            || serde_json::to_vec(self).map_err(|_| invalid())?.len() > TRACKER_METADATA_BUDGET
            || (self.events.is_empty() && self.has_more)
        {
            return Err(invalid());
        }
        let mut previous = after;
        let mut ids = std::collections::HashSet::new();
        for event in &self.events {
            let sequence = tracker_metadata_cursor(&event.sequence)?;
            let p = &event.payload;
            if sequence <= previous
                || event.event_id.is_nil()
                || !ids.insert(event.event_id)
                || event.task_id != binding.task_id
                || p.tracker_instance_id != binding.tracker_instance_id
                || p.project_id != binding.project_id
                || p.root_task_id != binding.root_task_id
                || p.owner_subject != binding.owner_subject
            {
                return Err(invalid());
            }
            if let Some(revision) = p.current_requirement_revision {
                version(revision)?;
            }
            event.validate()?;
            previous = sequence;
        }
        if tracker_metadata_cursor(&self.next_after)? != previous {
            return Err(invalid());
        }
        Ok(())
    }
}

impl TrackerMetadataEvent {
    pub fn source_time(&self) -> Result<DateTime<Utc>, AppError> {
        let time = DateTime::parse_from_rfc3339(&self.created_at)
            .map_err(|_| invalid())?
            .with_timezone(&Utc);
        if time.to_rfc3339_opts(SecondsFormat::Nanos, true) != self.created_at {
            return Err(invalid());
        }
        Ok(time)
    }

    pub fn summary(&self) -> Result<&'static str, AppError> {
        crate::tracker_events::summary(&self.event_type)
    }

    fn validate(&self) -> Result<(), AppError> {
        self.source_time()?;
        let resource = &self.payload.resource;
        match self.event_type.as_str() {
            "task.created" => {
                strict::<CreatedResource>(resource)?;
            }
            "task.bound" => {
                strict::<BoundResource>(resource)?;
            }
            "pm.assigned" => {
                strict::<MetadataFence>(resource)?;
            }
            "clarification.published" | "clarification.cancelled" => {
                let question = strict::<QuestionResource>(resource)?;
                let expected = if self.event_type == "clarification.published" {
                    "open"
                } else {
                    "cancelled"
                };
                if question.state != expected {
                    return Err(invalid());
                }
            }
            "clarification.answered" => {
                strict::<AnswerResource>(resource)?;
            }
            "requirements.published" => {
                strict::<RevisionResource>(resource)?;
            }
            "requirements.evidence_recorded" => {
                strict::<EvidenceResource>(resource)?;
            }
            "requirements.confirmed" => {
                strict::<ConfirmationResource>(resource)?;
            }
            _ => return Err(invalid()),
        }
        validate_refs(resource)?;
        let mut raw = serde_json::to_value(self).map_err(|_| invalid())?;
        raw.as_object_mut()
            .ok_or_else(invalid)?
            .remove("metadata_sha256");
        let canonical = json!({"contract_version":1,"projection":"metadata_v1","event":raw});
        let hash = hex::encode(Sha256::digest(
            serde_json::to_vec(&canonical).map_err(|_| invalid())?,
        ));
        if hash != self.metadata_sha256 {
            return Err(invalid());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actual_tracker_postgres_http_snapshots_decode_without_rewriting_source_digests() {
        let mut types = std::collections::HashSet::new();
        for bytes in [
            include_bytes!("../tests/fixtures/tracker-metadata-created.http.json").as_slice(),
            include_bytes!("../tests/fixtures/tracker-metadata-all8.http.json").as_slice(),
        ] {
            let raw: Value = serde_json::from_slice(bytes).unwrap();
            let event = &raw["events"][0];
            let payload = &event["payload"];
            let b = TaskChatBinding {
                tracker_instance_id: payload["tracker_instance_id"].as_str().unwrap().into(),
                project_id: serde_json::from_value(payload["project_id"].clone()).unwrap(),
                task_id: serde_json::from_value(event["task_id"].clone()).unwrap(),
                root_task_id: serde_json::from_value(payload["root_task_id"].clone()).unwrap(),
                owner_subject: payload["owner_subject"].as_str().unwrap().into(),
                agent_id: Uuid::new_v4(),
            };
            let page = TrackerMetadataPage::decode(bytes, &b, 0).unwrap();
            for event in page.events {
                types.insert(event.event_type);
            }
        }
        assert_eq!(types.len(), 9);
    }

    fn binding() -> TaskChatBinding {
        TaskChatBinding {
            tracker_instance_id: "tracker-test".into(),
            project_id: Uuid::new_v4(),
            task_id: Uuid::new_v4(),
            root_task_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            owner_subject: "central-owner".into(),
        }
    }

    fn sign(event: &mut Value) {
        event.as_object_mut().unwrap().remove("metadata_sha256");
        let hash = hex::encode(Sha256::digest(
            serde_json::to_vec(&json!({
                "contract_version":1,"projection":"metadata_v1","event":event
            }))
            .unwrap(),
        ));
        event["metadata_sha256"] = json!(hash);
    }

    fn wire(binding: &TaskChatBinding, kind: &str, resource: Value) -> Value {
        let mut event = json!({"sequence":"9223372036854775807", "event_id":Uuid::new_v4(),
            "task_id":binding.task_id,"event_type":kind,"created_at":"2026-10-02T00:00:00.000000000Z",
            "payload":{"tracker_instance_id":binding.tracker_instance_id,"project_id":binding.project_id,
                "root_task_id":binding.root_task_id,"owner_subject":binding.owner_subject,"stage":"Draft",
                "current_requirement_revision":null,"resource":resource}});
        sign(&mut event);
        json!({"contract_version":1,"projection":"metadata_v1","after":"0",
            "next_after":"9223372036854775807","has_more":false,"events":[event]})
    }

    fn decode(value: &Value, binding: &TaskChatBinding) -> Result<TrackerMetadataPage, AppError> {
        TrackerMetadataPage::decode(&serde_json::to_vec(value).unwrap(), binding, 0)
    }

    #[test]
    fn metadata_discriminators_and_i64_cursors_are_strict_and_lossless() {
        let b = binding();
        let fence = json!({"assignment_id":Uuid::new_v4(),"execution_id":Uuid::new_v4(),
            "agent_id":Uuid::new_v4(),"assignment_version":1});
        let question = json!({"question_id":Uuid::new_v4(),"question_version":1,"request_id":Uuid::new_v4(),
            "checkpoint_id":Uuid::new_v4(),"requirement_revision":1,"state":"open","fence":fence});
        let mut cancelled = question.clone();
        cancelled["state"] = json!("cancelled");
        let mut answer = question.clone();
        answer.as_object_mut().unwrap().remove("state");
        answer["answer_id"] = json!(Uuid::new_v4());
        let revision = json!({"requirement_revision":1,"content_hash":"a".repeat(64)});
        let mut evidence = revision.clone();
        evidence["check_id_sha256"] = json!("b".repeat(64));
        let mut confirmation = revision.clone();
        confirmation["confirmation_id"] = json!(Uuid::new_v4());
        let resources = [
            ("task.created", json!({"input":null})),
            ("task.bound", json!({})),
            ("pm.assigned", fence),
            ("clarification.published", question),
            ("clarification.cancelled", cancelled),
            ("clarification.answered", answer),
            ("requirements.published", revision),
            ("requirements.evidence_recorded", evidence),
            ("requirements.confirmed", confirmation),
        ];
        for (kind, resource) in resources {
            let valid = wire(&b, kind, resource);
            let page = decode(&valid, &b).unwrap();
            assert_eq!(tracker_metadata_cursor(&page.next_after).unwrap(), i64::MAX);
            assert!(!page.events[0].summary().unwrap().contains("secret"));
            let mut changed = valid.clone();
            changed["events"][0]["payload"]["resource"]["body"] = json!("secret");
            sign(&mut changed["events"][0]);
            assert!(decode(&changed, &b).is_err(), "{kind}");
        }
        for cursor in ["-1", "00", "01", "+1", " 1", "9223372036854775808"] {
            assert!(tracker_metadata_cursor(cursor).is_err());
        }
    }

    #[test]
    fn metadata_missing_nulls_noncanonical_refs_hashes_and_page_forgery_fail_closed() {
        let b = binding();
        let valid = wire(
            &b,
            "task.created",
            json!({"input":{"snapshot_ref":Uuid::new_v4(),"sha256":"a".repeat(64)}}),
        );
        decode(&valid, &b).unwrap();
        for mutation in 0..13 {
            let mut changed = valid.clone();
            let e = &mut changed["events"][0];
            match mutation {
                0 => {
                    e["payload"]
                        .as_object_mut()
                        .unwrap()
                        .remove("current_requirement_revision");
                }
                1 => e["event_id"] = json!(Uuid::nil()),
                2 => e["event_id"] = json!(Uuid::new_v4().simple().to_string()),
                3 => e["payload"]["resource"]["input"]["snapshot_ref"] = json!(Uuid::nil()),
                4 => e["payload"]["resource"]["input"]["sha256"] = json!("A".repeat(64)),
                5 => e["created_at"] = json!("2026-10-02T00:00:00+00:00"),
                6 => e["payload"]["current_requirement_revision"] = json!(9_007_199_254_740_992i64),
                7 => e["payload"]["owner_subject"] = json!("foreign"),
                8 => e["sequence"] = json!(i64::MAX),
                9 => e["payload"]["resource"] = json!({}),
                10 => {
                    e["payload"]["resource"] =
                        json!({"requirement_revision":1,"content_hash":"a".repeat(64)})
                }
                11 => e["payload"]["current_requirement_revision"] = json!(0),
                _ => e["metadata_sha256"] = json!("b".repeat(64)),
            }
            if mutation != 12 {
                sign(e);
            }
            assert!(decode(&changed, &b).is_err(), "mutation {mutation}");
        }
        for field in ["after", "next_after", "projection", "contract_version"] {
            let mut changed = valid.clone();
            changed[field] = json!("wrong");
            assert!(decode(&changed, &b).is_err());
        }
        let mut changed = valid.clone();
        let duplicate = changed["events"][0].clone();
        changed["events"].as_array_mut().unwrap().push(duplicate);
        assert!(decode(&changed, &b).is_err());
        let empty = json!({"contract_version":1,"projection":"metadata_v1","after":"0","next_after":"0","has_more":false,"events":[]});
        decode(&empty, &b).unwrap();
        let mut impossible = empty;
        impossible["has_more"] = json!(true);
        assert!(decode(&impossible, &b).is_err());
        assert!(
            TrackerMetadataPage::decode(&vec![b' '; TRACKER_METADATA_BUDGET + 1], &b, 0).is_err()
        );
    }
}
