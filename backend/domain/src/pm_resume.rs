//! A trusted answer reserves the next run of the same execution, never a new Task.
use crate::{
    PmCheckpointJournal, PmRunRecord, TrackerMetadataEvent, TrackerQuestion, pm_canonical_hash,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shared::AppError;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmResumeIntent {
    pub old_session_run_id: Uuid,
    pub new_session_run_id: Uuid,
    pub message_id: Uuid,
    pub source_event_id: Uuid,
    pub source_answer_id: Uuid,
    pub command: Value,
    pub request_sha256: String,
    pub prompt: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PmResumeJournal {
    pub intent: PmResumeIntent,
    pub receipt: Option<Value>,
}

impl PmResumeIntent {
    pub fn from_answer(
        old: &PmRunRecord,
        checkpoint: &PmCheckpointJournal,
        question: &TrackerQuestion,
        event: &TrackerMetadataEvent,
        next_run: Uuid,
        message: Uuid,
    ) -> Result<Self, AppError> {
        checkpoint.validate(old)?;
        let conflict = || AppError::conflict("PM answer differs from the original wait");
        let receipt = checkpoint.receipt.as_ref().ok_or_else(conflict)?;
        let answer = question.answer.as_ref().ok_or_else(conflict)?;
        let identity = &old.reservation.identity;
        if !old.terminal_status.is_some_and(|status| status.terminal())
            || old.reservation.native_session_key.is_none()
            || next_run.is_nil()
            || next_run == old.reservation.session_run_id
            || message.is_nil()
            || !event.matches_pm_answer(question)?
            || event.event_id.is_nil()
            || answer.id.is_nil()
            || event.payload.tracker_instance_id != identity.tracker_instance_ref
            || event.payload.project_id.to_string() != identity.tracker_project_ref
            || question.task_id.to_string() != identity.task_ref
            || question.root_task_id.to_string() != identity.root_ref
            || question.assignment_id.to_string() != identity.assignment_ref
            || question.execution_id.to_string() != identity.execution_ref
            || question.agent_id.to_string() != identity.agent_ref
            || question.assignment_version != identity.assignment_revision
            || checkpoint.command["checkpoint_ref"] != json!(question.checkpoint_id)
            || checkpoint.command["clarification_request_ref"] != json!(question.request_id)
            || checkpoint.command["clarification_version"].as_i64() != Some(question.version)
            || checkpoint.command["requirements_revision"].as_i64()
                != Some(question.requirement_revision)
        {
            return Err(conflict());
        }
        let mut selected = Vec::new();
        for selected_id in &answer.selected_option_ids {
            let option = question
                .options
                .iter()
                .find(|option| option.id == *selected_id)
                .ok_or_else(conflict)?;
            selected.push(
                json!({"id":option.id,"label":option.label,"consequences":option.consequences}),
            );
        }
        let mut command = checkpoint.command.clone();
        let fields = command.as_object_mut().ok_or_else(conflict)?;
        fields.insert(
            "operation_key".into(),
            json!(format!(
                "fleet-pm-resume:{}:{}",
                old.reservation.session_run_id, event.event_id
            )),
        );
        fields.insert("expected_version".into(), receipt["version"].clone());
        fields.insert("answer_event_ref".into(), json!(event.event_id));
        fields.insert("new_session_run_id".into(), json!(next_run));
        let prompt = format!("Continue the same PM execution after the confirmed clarification answer. Read the current Workflow phase and Task context before reporting.\n{}",
            serde_json::to_string(&json!({"source_event_id":event.event_id,"answer_id":answer.id,
                "question_id":question.id,"question":question.text,"selected_options":selected,
                "text":answer.text,"comment":answer.comment,"author_subject":answer.author_subject,
                "question_version":question.version,"requirements_revision":question.requirement_revision,
                "checkpoint_ref":question.checkpoint_id})).map_err(AppError::internal)?);
        let intent = Self {
            old_session_run_id: old.reservation.session_run_id,
            new_session_run_id: next_run,
            message_id: message,
            source_event_id: event.event_id,
            source_answer_id: answer.id,
            request_sha256: pm_canonical_hash(&command),
            command,
            prompt,
        };
        intent.validate(old, checkpoint)?;
        Ok(intent)
    }

    pub fn validate(
        &self,
        old: &PmRunRecord,
        checkpoint: &PmCheckpointJournal,
    ) -> Result<(), AppError> {
        checkpoint.validate(old)?;
        let invalid = || AppError::conflict("PM original resume intent is inconsistent");
        let receipt = checkpoint.receipt.as_ref().ok_or_else(invalid)?;
        let mut expected = checkpoint.command.clone();
        let fields = expected.as_object_mut().ok_or_else(invalid)?;
        fields.insert(
            "operation_key".into(),
            json!(format!(
                "fleet-pm-resume:{}:{}",
                old.reservation.session_run_id, self.source_event_id
            )),
        );
        fields.insert("expected_version".into(), receipt["version"].clone());
        fields.insert("answer_event_ref".into(), json!(self.source_event_id));
        fields.insert("new_session_run_id".into(), json!(self.new_session_run_id));
        if self.old_session_run_id != old.reservation.session_run_id
            || !old.terminal_status.is_some_and(|status| status.terminal())
            || old.reservation.native_session_key.is_none()
            || [
                self.new_session_run_id,
                self.message_id,
                self.source_event_id,
                self.source_answer_id,
            ]
            .iter()
            .any(Uuid::is_nil)
            || self.new_session_run_id == self.old_session_run_id
            || self.command != expected
            || self.request_sha256 != pm_canonical_hash(&expected)
            || self.prompt.trim().is_empty()
            || self.prompt.len() > 256 * 1024
        {
            return Err(invalid());
        }
        Ok(())
    }

    pub fn verify_receipt(
        &self,
        old: &PmRunRecord,
        checkpoint: &PmCheckpointJournal,
        response: &Value,
    ) -> Result<(), AppError> {
        self.validate(old, checkpoint)?;
        let result = &response["result"];
        if response["ok"] != true
            || result["contract_version"].as_i64() != Some(1)
            || result["identity"] != json!(old.reservation.identity)
            || result["state"] != "resume_pending"
            || result["session_run_id"] != json!(old.reservation.session_run_id)
            || result["binding_ref"] != old.reservation.binding_ref
            || result["hermes_run_ref"] != json!(old.hermes_run_ref)
            || result["fence"].as_i64() != old.reservation.fence.checked_add(1)
            || result["version"].as_i64()
                != self.command["expected_version"]
                    .as_i64()
                    .and_then(|v| v.checked_add(1))
            || result["resume_session_run_id"] != json!(self.new_session_run_id)
            || result["resume_operation_key"] != self.command["operation_key"]
            || result["checkpoint"]
                != checkpoint
                    .receipt
                    .as_ref()
                    .ok_or_else(|| AppError::conflict("PM checkpoint ACK missing"))?["checkpoint"]
            || result["workflow_step_allowed"] != false
            || result["resume_delivered"] != false
            || !matches!(
                result["terminal_readback"]["status"].as_str(),
                Some("completed" | "failed" | "cancelled" | "stopped")
            )
        {
            return Err(AppError::conflict(
                "PM resume acknowledgement differs from the original answer",
            ));
        }
        let terminal = &result["terminal_readback"];
        let identity = json!(old.reservation.identity);
        if identity.as_object().is_none_or(|fields| {
            fields
                .iter()
                .any(|(key, value)| terminal.get(key) != Some(value))
        }) || terminal["binding_ref"] != old.reservation.binding_ref
            || terminal["hermes_run_ref"] != json!(old.hermes_run_ref)
            || terminal["session_run_id"] != json!(self.old_session_run_id)
            || terminal["dispatch_operation_key"] != old.reservation.dispatch_operation_key
            || terminal["checkpoint_ref"] != checkpoint.command["checkpoint_ref"]
            || terminal["status"] != json!(old.terminal_status)
            || terminal["fence"].as_i64() != Some(old.reservation.fence)
        {
            return Err(AppError::conflict(
                "PM resume terminal proof belongs to another run",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    fn fixture() -> (
        PmRunRecord,
        PmCheckpointJournal,
        TrackerQuestion,
        TrackerMetadataEvent,
    ) {
        let identity = PmExecutionIdentity {
            task: "SDLC-9".into(),
            execution_ref: Uuid::new_v4().to_string(),
            tracker_instance_ref: "tracker:fixture".into(),
            tracker_project_ref: Uuid::new_v4().to_string(),
            task_ref: Uuid::new_v4().to_string(),
            root_ref: Uuid::new_v4().to_string(),
            agent_ref: Uuid::new_v4().to_string(),
            assignment_operation_key: "original-assignment".into(),
            assignment_ref: Uuid::new_v4().to_string(),
            assignment_revision: 1,
        };
        let session = Uuid::new_v4();
        let message = Uuid::new_v4();
        let old = PmRunRecord {
            reservation: PmRunReservation {
                session_id: session,
                session_run_id: Uuid::new_v4(),
                identity: identity.clone(),
                binding_ref: "original-binding".into(),
                dispatch_operation_key: message.to_string(),
                checkpoint_ref: None,
                fence: 1,
                runtime_binding: Some(PmRuntimeBinding {
                    launch_id: Uuid::new_v4(),
                    controller_id: Uuid::new_v4(),
                    origin: "http://127.0.0.1:29002".into(),
                    credential_fingerprint: "a".repeat(64),
                }),
                native_session_key: Some(pm_native_session_key(
                    session,
                    identity.agent_id().unwrap(),
                    message,
                )),
                native_message_id: Some(message),
            },
            hermes_run_ref: Some("run_original".into()),
            hermes_session_ref: Some(Uuid::new_v4().to_string()),
            terminal_status: Some(PmRuntimeStatus::Stopped),
        };
        let checkpoint_id = Uuid::new_v4();
        let request = Uuid::new_v4();
        let question_id = Uuid::new_v4();
        let answer_id = Uuid::new_v4();
        let question:TrackerQuestion=serde_json::from_value(json!({"id":question_id,"request_id":request,"task_id":identity.task_ref,
            "root_task_id":identity.root_ref,"version":2,"requirement_revision":3,"requirement_reference":null,
            "assignment_id":identity.assignment_ref,"execution_id":identity.execution_ref,"agent_id":identity.agent_ref,
            "assignment_version":1,"checkpoint_id":checkpoint_id,"text":"Which scope?","rationale":"Need exact scope",
            "required":true,"mode":"text","options":[],"recommended_option_id":null,"state":"answered",
            "answer":{"id":answer_id,"question_id":question_id,"question_version":2,"requirement_revision":3,
                "selected_option_ids":[],"text":"Owner-selected scope","comment":null,"author_subject":Uuid::new_v4(),"created_at":"2026-10-09T00:00:00Z"},
            "author_subject":Uuid::new_v4(),"created_at":"2026-10-09T00:00:00Z"})).unwrap();
        let mut command = json!(identity);
        command.as_object_mut().unwrap().extend([
            ("operation_key".into(), json!("checkpoint:original")),
            ("expected_version".into(), json!(1)),
            ("expected_fence".into(), json!(1)),
            ("binding_ref".into(), json!(old.reservation.binding_ref)),
            ("hermes_run_ref".into(), json!(old.hermes_run_ref)),
            (
                "session_run_id".into(),
                json!(old.reservation.session_run_id),
            ),
            ("checkpoint_ref".into(), json!(checkpoint_id)),
            ("clarification_request_ref".into(), json!(request)),
            ("clarification_version".into(), json!(2)),
            ("requirements_revision".into(), json!(3)),
        ]);
        let receipt = json!({"contract_version":1,"identity":identity,"state":"waiting","version":2,"fence":1,
            "session_run_id":old.reservation.session_run_id,"binding_ref":old.reservation.binding_ref,"hermes_run_ref":old.hermes_run_ref,
            "checkpoint":{"checkpoint_ref":checkpoint_id,"clarification_request_ref":request,"clarification_version":2,"requirements_revision":3},
            "resume_operation_key":null,"resume_session_run_id":null,"terminal_readback":null,"workflow_step_allowed":false,"resume_delivered":false});
        let checkpoint = PmCheckpointJournal {
            request_sha256: pm_canonical_hash(&command),
            command,
            receipt: Some(receipt),
        };
        let event:TrackerMetadataEvent=serde_json::from_value(json!({"sequence":"8","event_id":Uuid::new_v4(),"task_id":identity.task_ref,
            "event_type":"clarification.answered","created_at":"2026-10-09T00:00:00.000000000Z","metadata_sha256":"a".repeat(64),
            "payload":{"tracker_instance_id":identity.tracker_instance_ref,"project_id":identity.tracker_project_ref,"root_task_id":identity.root_ref,
                "owner_subject":Uuid::new_v4(),"stage":"Clarification","current_requirement_revision":3,"resource":{"answer_id":answer_id,
                "question_id":question_id,"question_version":2,"request_id":request,"checkpoint_id":checkpoint_id,"requirement_revision":3,
                "fence":{"assignment_id":identity.assignment_ref,"execution_id":identity.execution_ref,"agent_id":identity.agent_ref,"assignment_version":1}}}})).unwrap();
        (old, checkpoint, question, event)
    }
    #[test]
    fn pm_resume_uses_actual_event_and_preserves_execution_with_distinct_native_run() {
        let (old, checkpoint, question, event) = fixture();
        let intent = PmResumeIntent::from_answer(
            &old,
            &checkpoint,
            &question,
            &event,
            Uuid::new_v4(),
            Uuid::new_v4(),
        )
        .unwrap();
        intent.validate(&old, &checkpoint).unwrap();
        assert_eq!(intent.command["answer_event_ref"], json!(event.event_id));
        assert_ne!(
            intent.command["answer_event_ref"],
            json!(question.answer.as_ref().unwrap().id)
        );
        assert_eq!(
            intent.command["execution_ref"],
            old.reservation.identity.execution_ref
        );
        assert_eq!(intent.command["task"], "SDLC-9");
        assert_eq!(intent.command["expected_version"], 2);
        assert!(intent.prompt.contains("Owner-selected scope"));
        for field in [
            "agent_ref",
            "assignment_ref",
            "task_ref",
            "root_ref",
            "answer_event_ref",
            "new_session_run_id",
        ] {
            let mut forged = intent.clone();
            forged.command[field] = json!(Uuid::new_v4());
            forged.request_sha256 = pm_canonical_hash(&forged.command);
            assert!(forged.validate(&old, &checkpoint).is_err(), "{field}");
        }
    }
    #[test]
    fn pm_resume_rejects_stale_foreign_unanswered_and_unconfirmed_wait() {
        let (old, checkpoint, question, event) = fixture();
        for field in [
            "answer_id",
            "question_id",
            "request_id",
            "checkpoint_id",
            "requirement_revision",
            "question_version",
        ] {
            let mut foreign = event.clone();
            foreign.payload.resource[field] =
                if field.ends_with("revision") || field.ends_with("version") {
                    json!(99)
                } else {
                    json!(Uuid::new_v4())
                };
            assert!(
                PmResumeIntent::from_answer(
                    &old,
                    &checkpoint,
                    &question,
                    &foreign,
                    Uuid::new_v4(),
                    Uuid::new_v4()
                )
                .is_err(),
                "{field}"
            );
        }
        let mut pending = question.clone();
        pending.answer = None;
        assert!(
            PmResumeIntent::from_answer(
                &old,
                &checkpoint,
                &pending,
                &event,
                Uuid::new_v4(),
                Uuid::new_v4()
            )
            .is_err()
        );
        let mut running = old.clone();
        running.terminal_status = None;
        assert!(
            PmResumeIntent::from_answer(
                &running,
                &checkpoint,
                &question,
                &event,
                Uuid::new_v4(),
                Uuid::new_v4()
            )
            .is_err()
        );
        let mut unknown = checkpoint;
        unknown.receipt = None;
        assert!(
            PmResumeIntent::from_answer(
                &old,
                &unknown,
                &question,
                &event,
                Uuid::new_v4(),
                Uuid::new_v4()
            )
            .is_err()
        );
    }
}
