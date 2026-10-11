//! Workflow's assignment credential stays in Fleet; preparation never starts a model.
use app::FleetRepository;
use domain::{
    PmDraftOperation, PmDraftProof, PmWorkflowAssignedDraft, PmWorkflowAssignmentIntent,
    PmWorkflowAssignmentReceipt, PmWorkflowCompatibility,
};
use reqwest::{Client, Url, header};
use serde_json::Value;
use sha2::{Digest, Sha256};
use shared::{AppConfig, AppError};
use std::time::Duration;

use crate::pm_credentials::{PmCredentialCoordinator, PmDelegatedCredential};

pub struct PmWorkflowClient {
    origin: Url,
    assignment: header::HeaderValue,
    runtime: header::HeaderValue,
    fingerprint: String,
    client: Client,
}

fn unavailable() -> AppError {
    AppError::Unavailable(
        "PM Workflow evidence is unavailable; reconcile the original operation".into(),
    )
}

impl PmWorkflowClient {
    pub fn configured(config: &AppConfig) -> Result<Option<Self>, AppError> {
        let settings = &config.pm.workflow;
        if !settings.enabled {
            return Ok(None);
        }
        let secret = &settings.assignment_token;
        if !(32..=4096).contains(&secret.len())
            || !secret.bytes().all(|v| v.is_ascii_graphic())
            || !(32..=4096).contains(&settings.runtime_token.len())
            || !settings.runtime_token.bytes().all(|v| v.is_ascii_graphic())
            || secret == &settings.runtime_token
            || secret == &config.pm.readback_token
            || settings.runtime_token == config.pm.readback_token
        {
            return Err(AppError::validation(
                "invalid PM Workflow credential configuration",
            ));
        }
        let origin = crate::pm_credentials::configured_origin(
            config
                .fleet
                .project_workflow_url
                .as_deref()
                .ok_or_else(|| AppError::validation("PM Workflow origin required"))?,
        )?;
        let mut assignment = header::HeaderValue::from_str(&format!("Bearer {secret}"))
            .map_err(|_| AppError::validation("invalid PM Workflow credential"))?;
        assignment.set_sensitive(true);
        let mut runtime =
            header::HeaderValue::from_str(&format!("Bearer {}", settings.runtime_token))
                .map_err(|_| AppError::validation("invalid PM Workflow credential"))?;
        runtime.set_sensitive(true);
        let mut hash = Sha256::new();
        hash.update(b"fleet-pm-workflow-assignment-v1\0");
        hash.update(secret.as_bytes());
        Ok(Some(Self {
            origin,
            assignment,
            runtime,
            fingerprint: hex::encode(hash.finalize()),
            client: Client::builder()
                .connect_timeout(Duration::from_secs(3))
                .timeout(Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .no_proxy()
                .no_gzip()
                .no_brotli()
                .no_deflate()
                .no_zstd()
                .build()
                .map_err(|_| unavailable())?,
        }))
    }

    async fn request(&self, path: &str, body: Option<&Value>) -> Result<Value, AppError> {
        self.request_with_pending(path, body, false)
            .await?
            .ok_or_else(unavailable)
    }

    async fn request_with_pending(
        &self,
        path: &str,
        body: Option<&Value>,
        pending_readback: bool,
    ) -> Result<Option<Value>, AppError> {
        self.authorized_request(path, body, pending_readback, None)
            .await
    }

    async fn authorized_request(
        &self,
        path: &str,
        body: Option<&Value>,
        pending_readback: bool,
        runtime_scope: Option<&str>,
    ) -> Result<Option<Value>, AppError> {
        let mut url = self.origin.clone();
        url.set_path(path);
        let request = match body {
            Some(body) => self.client.post(url).json(body),
            None => self.client.get(url),
        }
        .header(
            header::AUTHORIZATION,
            if runtime_scope.is_some() {
                self.runtime.clone()
            } else {
                self.assignment.clone()
            },
        )
        .header(header::ACCEPT_ENCODING, "identity")
        .header(header::CACHE_CONTROL, "no-cache, no-store");
        let request = if let Some(scope) = runtime_scope {
            let mut header = header::HeaderValue::from_str(scope).map_err(|_| unavailable())?;
            header.set_sensitive(true);
            request.header("X-Workflow-Execution-Token", header)
        } else {
            request
        };
        let mut response = request.send().await.map_err(|_| unavailable())?;
        match response.status() {
            reqwest::StatusCode::OK => (),
            reqwest::StatusCode::NOT_FOUND if pending_readback => return Ok(None),
            reqwest::StatusCode::UNAUTHORIZED => return Err(AppError::Unauthorized),
            reqwest::StatusCode::FORBIDDEN => return Err(AppError::Forbidden),
            reqwest::StatusCode::CONFLICT => {
                return Err(AppError::conflict(
                    "PM Workflow original operation conflicts",
                ));
            }
            _ => return Err(unavailable()),
        }
        if !response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| {
                v.split(';')
                    .next()
                    .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"))
            })
            || response
                .headers()
                .get(header::CONTENT_ENCODING)
                .is_some_and(|v| v != "identity")
            || response.content_length().is_some_and(|v| v > 64 * 1024)
        {
            return Err(unavailable());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
            if bytes.len().saturating_add(chunk.len()) > 64 * 1024 {
                return Err(unavailable());
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| unavailable())
    }

    async fn compatibility(&self) -> Result<PmWorkflowCompatibility, AppError> {
        let value = self.request("/internal/runtime/capabilities", None).await?;
        if value["ok"] != true
            || value["role_key"] != "project_manager"
            || value["credential_kind"] != "assignment"
            || value["readiness"]
                != serde_json::json!({"service":"ready","schema":"ready","catalog":"ready"})
            || value["pm_continuation"]["contract_version"].as_i64() != Some(1)
            || value["pm_continuation"]["base_path"] != "/internal/runtime/v1/pm"
            || value["pm_continuation"]["dispatch_owner"] != "fleet"
            || value["pm_continuation"]["terminal_proof"] != "configured-runtime-readback"
            || !value["pm_continuation"]["commands"]
                .as_array()
                .is_some_and(|v| {
                    ["assign", "bind", "resume", "rebind", "readback"]
                        .iter()
                        .all(|command| v.contains(&Value::from(*command)))
                })
        {
            return Err(unavailable());
        }
        let raw = value["runtimeCompatibility"].clone();
        let compatibility: PmWorkflowCompatibility =
            serde_json::from_value(raw.clone()).map_err(|_| unavailable())?;
        if serde_json::to_value(&compatibility).map_err(AppError::internal)? != raw {
            return Err(unavailable());
        }
        compatibility.validate()?;
        Ok(compatibility)
    }

    /// Original command is durable before POST; explicit replay reuses the owner's idempotency key.
    /// Current Tracker/lease and packaged Workflow facts are prerequisites, never native admission.
    pub async fn prepare_assignment(
        &self,
        repo: &dyn FleetRepository,
        operation: &PmDraftOperation,
        coordinator: &PmCredentialCoordinator,
        credential: &PmDelegatedCredential,
    ) -> Result<PmWorkflowAssignmentReceipt, AppError> {
        let claim = operation.execution_lease.as_ref().ok_or_else(unavailable)?;
        if claim.receipt.is_none() {
            return Err(unavailable());
        }
        let original = domain::PmExecutionLeaseCommand::Claim(claim.claim.clone());
        let lease = coordinator
            .read_execution_lease(operation, credential, Some(&original))
            .await?;
        if lease.state != domain::PmExecutionLeaseState::Active || lease.operation.is_none() {
            return Err(AppError::conflict(
                "PM Workflow preparation requires the current original lease",
            ));
        }
        let compatibility = self.compatibility().await?;
        let command = domain::pm_workflow::assignment_command(operation, &compatibility)?;
        let intent = PmWorkflowAssignmentIntent {
            request_sha256: domain::pm_canonical_hash(&command),
            command,
            workflow_origin: self.origin.as_str().into(),
            credential_fingerprint: self.fingerprint.clone(),
        };
        let saved = repo
            .record_pm_draft_proof(
                operation.id,
                operation.owner_user_id,
                PmDraftProof::WorkflowIntent(intent),
            )
            .await?;
        let journal = saved.workflow_assignment.as_ref().ok_or_else(unavailable)?;
        let (receipt, saved) = if let Some(receipt) = &journal.receipt {
            receipt.validate()?;
            (receipt.clone(), saved)
        } else {
            let value = self
                .request(
                    "/internal/runtime/v1/pm/assign",
                    Some(&journal.intent.command),
                )
                .await?;
            if value.as_object().map(|v| v.len()) != Some(3)
                || value["ok"] != true
                || value["exit_code"].as_i64() != Some(0)
            {
                return Err(unavailable());
            }
            let receipt =
                PmWorkflowAssignedDraft::verified(value["result"].clone(), &journal.intent)?;
            // Retain the original ACK even when the subsequent current-state check fails.
            let saved = repo
                .record_pm_draft_proof(
                    saved.id,
                    saved.owner_user_id,
                    PmDraftProof::WorkflowAcknowledged(receipt.clone()),
                )
                .await?;
            (receipt, saved)
        };
        let current = coordinator
            .read_execution_lease(&saved, credential, Some(&original))
            .await?;
        if current.state != domain::PmExecutionLeaseState::Active || current.operation.is_none() {
            return Err(AppError::conflict(
                "PM lease expired after Workflow acknowledgement",
            ));
        }
        Ok(receipt)
    }

    pub async fn verify_active_run(&self, record: &domain::PmRunRecord) -> Result<bool, AppError> {
        Ok(self.active_snapshot(record).await?.is_some())
    }

    async fn active_snapshot(
        &self,
        record: &domain::PmRunRecord,
    ) -> Result<Option<Value>, AppError> {
        let Some(value) = self.execution_snapshot(record).await? else {
            return Ok(None);
        };
        if value["result"]["state"] != "active" || value["result"]["workflow_step_allowed"] != true
        {
            return Err(AppError::conflict(
                "PM Workflow execution is not currently admitted",
            ));
        }
        Ok(Some(value))
    }

    pub async fn execution_snapshot(
        &self,
        record: &domain::PmRunRecord,
    ) -> Result<Option<Value>, AppError> {
        let body =
            serde_json::to_value(&record.reservation.identity).map_err(AppError::internal)?;
        let Some(value) = self
            .request_with_pending("/internal/runtime/v1/pm/readback", Some(&body), true)
            .await?
        else {
            return Ok(None);
        };
        let result = &value["result"];
        if value["ok"] == true
            && result["contract_version"].as_i64() == Some(1)
            && result["identity"] == body
            && result["state"] == "resume_pending"
            && result["resume_session_run_id"]
                == serde_json::json!(record.reservation.session_run_id)
            && result["resume_operation_key"] == record.reservation.dispatch_operation_key
            && result["fence"].as_i64() == Some(record.reservation.fence)
            && result["workflow_step_allowed"] == false
        {
            return Ok(None);
        }
        if value["ok"] != true
            || result["contract_version"].as_i64() != Some(1)
            || result["identity"] != body
            || !matches!(
                result["state"].as_str(),
                Some("active" | "waiting" | "resume_pending")
            )
            || result["fence"].as_i64() != Some(record.reservation.fence)
            || result["session_run_id"].as_str()
                != Some(record.reservation.session_run_id.to_string().as_str())
            || result["binding_ref"].as_str() != Some(record.reservation.binding_ref.as_str())
            || result["hermes_run_ref"].as_str() != record.hermes_run_ref.as_deref()
        {
            return Err(AppError::conflict(
                "PM Workflow execution is not currently admitted",
            ));
        }
        if result["version"].as_i64().is_none_or(|v| v < 1) {
            return Err(unavailable());
        }
        Ok(Some(value))
    }

    /// Fleet supplies immutable execution fields; the model supplies only the
    /// report and its observed phase/status cursor. Secrets never enter output.
    pub async fn native_step(
        &self,
        record: &domain::PmRunRecord,
        command: &Value,
    ) -> Result<Value, AppError> {
        let snapshot = self
            .active_snapshot(record)
            .await?
            .ok_or_else(unavailable)?;
        let scope = snapshot["execution_token"]
            .as_str()
            .filter(|v| (32..=4096).contains(&v.len()) && v.bytes().all(|c| c.is_ascii_graphic()))
            .ok_or_else(unavailable)?;
        let body = domain::pm_workflow::native_step_command(record, command)?;
        let response = self
            .authorized_request("/internal/runtime/step", Some(&body), false, Some(scope))
            .await?
            .ok_or_else(unavailable)?;
        if !matches!(response["exit_code"].as_i64(), Some(0 | 1))
            || response["ok"].as_bool() != response["exit_code"].as_i64().map(|v| v == 0)
            || !response["result"].is_object()
        {
            return Err(unavailable());
        }
        response.get("result").cloned().ok_or_else(unavailable)
    }

    pub async fn checkpoint(
        &self,
        repo: &dyn FleetRepository,
        record: &domain::PmRunRecord,
        question: &domain::TrackerQuestion,
        operation_key: &str,
    ) -> Result<Value, AppError> {
        let snapshot = self
            .active_snapshot(record)
            .await?
            .ok_or_else(unavailable)?;
        let scope = snapshot["execution_token"]
            .as_str()
            .ok_or_else(unavailable)?;
        let r = &record.reservation;
        let mut body = serde_json::to_value(&r.identity).map_err(AppError::internal)?;
        body.as_object_mut().ok_or_else(unavailable)?.extend([
            ("operation_key".into(), serde_json::json!(operation_key)),
            (
                "expected_version".into(),
                snapshot["result"]["version"].clone(),
            ),
            ("expected_fence".into(), serde_json::json!(r.fence)),
            ("binding_ref".into(), serde_json::json!(r.binding_ref)),
            (
                "hermes_run_ref".into(),
                serde_json::json!(record.hermes_run_ref),
            ),
            ("session_run_id".into(), serde_json::json!(r.session_run_id)),
            (
                "checkpoint_ref".into(),
                serde_json::json!(question.checkpoint_id),
            ),
            (
                "clarification_request_ref".into(),
                serde_json::json!(question.request_id),
            ),
            (
                "clarification_version".into(),
                serde_json::json!(question.version),
            ),
            (
                "requirements_revision".into(),
                serde_json::json!(question.requirement_revision),
            ),
        ]);
        let mut journal = repo
            .save_pm_checkpoint(
                record,
                domain::PmCheckpointJournal {
                    request_sha256: domain::pm_canonical_hash(&body),
                    command: body,
                    receipt: None,
                },
            )
            .await?;
        if let Some(receipt) = journal.receipt {
            return Ok(receipt);
        }
        let response = self
            .authorized_request(
                "/internal/runtime/v1/pm/checkpoint",
                Some(&journal.command),
                false,
                Some(scope),
            )
            .await?
            .ok_or_else(unavailable)?;
        if response["ok"] != true {
            return Err(unavailable());
        }
        let receipt = response["result"].clone();
        journal.verify_receipt(record, &receipt)?;
        journal.receipt = Some(receipt.clone());
        repo.save_pm_checkpoint(record, journal).await?;
        Ok(receipt)
    }

    /// A lost checkpoint response is recovered by its original operation read;
    /// this method never posts or allocates another checkpoint/run.
    pub async fn reconcile_checkpoint(
        &self,
        repo: &dyn FleetRepository,
        record: &domain::PmRunRecord,
    ) -> Result<(), AppError> {
        let Some(mut journal) = repo
            .pm_checkpoint(record.reservation.session_run_id)
            .await?
        else {
            return Ok(());
        };
        journal.validate(record)?;
        if journal.receipt.is_some() {
            return Ok(());
        }
        let mut body =
            serde_json::to_value(&record.reservation.identity).map_err(AppError::internal)?;
        body.as_object_mut().ok_or_else(unavailable)?.insert(
            "operation_key".into(),
            journal.command["operation_key"].clone(),
        );
        let response = self
            .request("/internal/runtime/v1/pm/readback", Some(&body))
            .await?;
        let operation = &response["result"]["operation"];
        if response["ok"] != true
            || operation["kind"] != "checkpoint"
            || operation["operation_key"] != journal.command["operation_key"]
            || operation["request_sha256"] != journal.request_sha256
        {
            return Err(AppError::conflict("PM original checkpoint outcome unknown"));
        }
        let receipt = operation["result"].clone();
        journal.verify_receipt(record, &receipt)?;
        journal.receipt = Some(receipt);
        repo.save_pm_checkpoint(record, journal).await?;
        Ok(())
    }

    pub async fn resume_original(
        &self,
        repo: &dyn FleetRepository,
        old: &domain::PmRunRecord,
        checkpoint: &domain::PmCheckpointJournal,
    ) -> Result<domain::PmResumeJournal, AppError> {
        let mut journal = repo
            .pm_resume(old.reservation.session_run_id)
            .await?
            .ok_or_else(unavailable)?;
        journal.intent.validate(old, checkpoint)?;
        if let Some(receipt) = &journal.receipt {
            journal.intent.verify_receipt(
                old,
                checkpoint,
                &serde_json::json!({"ok":true,"result":receipt}),
            )?;
            return Ok(journal);
        }
        let mut query =
            serde_json::to_value(&old.reservation.identity).map_err(AppError::internal)?;
        query.as_object_mut().ok_or_else(unavailable)?.insert(
            "operation_key".into(),
            journal.intent.command["operation_key"].clone(),
        );
        let current = self
            .request("/internal/runtime/v1/pm/readback", Some(&query))
            .await?;
        let operation = &current["result"]["operation"];
        let response = if !operation.is_null() {
            if current["ok"] != true
                || operation["kind"] != "resume"
                || operation["operation_key"] != journal.intent.command["operation_key"]
                || operation["request_sha256"] != journal.intent.request_sha256
            {
                return Err(AppError::conflict("PM original resume outcome conflicts"));
            }
            serde_json::json!({"ok":true,"result":operation["result"]})
        } else {
            let result = &current["result"];
            if current["ok"] != true
                || result["identity"] != serde_json::json!(old.reservation.identity)
                || result["state"] != "waiting"
                || result["version"] != journal.intent.command["expected_version"]
                || result["fence"] != journal.intent.command["expected_fence"]
                || result["session_run_id"] != serde_json::json!(old.reservation.session_run_id)
                || result["binding_ref"] != old.reservation.binding_ref
                || result["hermes_run_ref"] != serde_json::json!(old.hermes_run_ref)
            {
                return Err(AppError::conflict("PM original wait changed before resume"));
            }
            // Original IDs and command were committed before this sole owner POST.
            self.request(
                "/internal/runtime/v1/pm/resume",
                Some(&journal.intent.command),
            )
            .await?
        };
        journal.intent.verify_receipt(old, checkpoint, &response)?;
        journal.receipt = Some(response["result"].clone());
        repo.save_pm_resume(old, checkpoint, journal).await
    }

    pub async fn resume_delivery_allowed(
        &self,
        old: &domain::PmRunRecord,
        checkpoint: &domain::PmCheckpointJournal,
        journal: &domain::PmResumeJournal,
    ) -> Result<bool, AppError> {
        let response = self
            .request(
                "/internal/runtime/v1/pm/readback",
                Some(&serde_json::to_value(&old.reservation.identity).map_err(AppError::internal)?),
            )
            .await?;
        let mut current = response["result"].clone();
        current
            .as_object_mut()
            .ok_or_else(unavailable)?
            .remove("operation");
        if current["state"] == "active"
            && current["identity"] == serde_json::json!(old.reservation.identity)
            && current["session_run_id"] == serde_json::json!(journal.intent.new_session_run_id)
            && current["fence"].as_i64() == old.reservation.fence.checked_add(1)
        {
            return Ok(false);
        }
        journal.intent.verify_receipt(
            old,
            checkpoint,
            &serde_json::json!({"ok":response["ok"],"result":current}),
        )?;
        Ok(true)
    }

    pub async fn rebind_original(
        &self,
        old: &domain::PmRunRecord,
        new: &domain::PmRunRecord,
        checkpoint: &domain::PmCheckpointJournal,
        journal: &domain::PmResumeJournal,
    ) -> Result<(), AppError> {
        let receipt = journal.receipt.as_ref().ok_or_else(unavailable)?;
        journal.intent.verify_receipt(
            old,
            checkpoint,
            &serde_json::json!({"ok":true,"result":receipt}),
        )?;
        let native = new.hermes_run_ref.as_ref().ok_or_else(unavailable)?;
        if new.reservation.identity != old.reservation.identity
            || new.reservation.session_id != old.reservation.session_id
            || new.reservation.session_run_id != journal.intent.new_session_run_id
            || new.reservation.native_message_id != Some(journal.intent.message_id)
            || new.reservation.dispatch_operation_key
                != journal.intent.command["operation_key"]
                    .as_str()
                    .ok_or_else(unavailable)?
            || new.reservation.fence != receipt["fence"].as_i64().ok_or_else(unavailable)?
            || new.reservation.checkpoint_ref.as_deref()
                != checkpoint.command["checkpoint_ref"].as_str()
        {
            return Err(AppError::conflict(
                "PM accepted next run differs from original resume",
            ));
        }
        if !self
            .resume_delivery_allowed(old, checkpoint, journal)
            .await?
        {
            if !self.verify_active_run(new).await? {
                return Err(unavailable());
            }
            return Ok(());
        }
        let mut command = journal.intent.command.clone();
        let fields = command.as_object_mut().ok_or_else(unavailable)?;
        for field in [
            "answer_event_ref",
            "clarification_request_ref",
            "clarification_version",
            "requirements_revision",
        ] {
            fields.remove(field);
        }
        fields.insert(
            "operation_key".into(),
            serde_json::json!(format!(
                "fleet-pm-rebind:{}",
                new.reservation.session_run_id
            )),
        );
        fields.insert("expected_version".into(), receipt["version"].clone());
        fields.insert("expected_fence".into(), receipt["fence"].clone());
        fields.insert(
            "resume_operation_key".into(),
            journal.intent.command["operation_key"].clone(),
        );
        fields.insert(
            "new_binding_ref".into(),
            serde_json::json!(new.reservation.binding_ref),
        );
        fields.insert("new_hermes_run_ref".into(), serde_json::json!(native));
        let response = self
            .request("/internal/runtime/v1/pm/rebind", Some(&command))
            .await?;
        if response["ok"] != true || !self.verify_active_run(new).await? {
            return Err(AppError::conflict("PM new binding is not currently active"));
        }
        Ok(())
    }

    pub async fn bind_initial_run(&self, record: &domain::PmRunRecord) -> Result<(), AppError> {
        let r = &record.reservation;
        let native = record.hermes_run_ref.as_ref().ok_or_else(unavailable)?;
        let ordinary = serde_json::json!({"task":r.identity.task,"bind_operation_key":format!("pm-native-bind:{}",r.session_run_id),
            "assignment_operation_key":r.identity.assignment_operation_key,"assignment_revision":r.identity.assignment_revision,
            "assignment_ref":r.identity.assignment_ref,"binding_ref":r.binding_ref,"hermes_run_ref":native,
            "mode_key":"draft","cycle_number":0,"attempt_number":1,"expected_binding_state":"unbound",
            "concrete_agent_ref":r.identity.agent_ref});
        let value = self
            .request("/internal/runtime/bind", Some(&ordinary))
            .await?;
        let result = &value["result"];
        if value["ok"] != true
            || result["binding_ref"] != r.binding_ref
            || result["hermes_run_ref"] != *native
            || result["concrete_agent_ref"] != r.identity.agent_ref
            || result["binding_state"] != "bound"
        {
            return Err(AppError::conflict(
                "PM ordinary binding acknowledgement differs from native run",
            ));
        }
        let mut pm = serde_json::to_value(&r.identity).map_err(AppError::internal)?;
        let body = pm.as_object_mut().ok_or_else(unavailable)?;
        body.extend([
            (
                "operation_key".into(),
                serde_json::json!(r.dispatch_operation_key),
            ),
            ("expected_version".into(), serde_json::json!(0)),
            ("binding_ref".into(), serde_json::json!(r.binding_ref)),
            ("hermes_run_ref".into(), serde_json::json!(native)),
            ("session_run_id".into(), serde_json::json!(r.session_run_id)),
        ]);
        let value = self
            .request("/internal/runtime/v1/pm/bind", Some(&pm))
            .await?;
        if value["ok"] != true || !self.verify_active_run(record).await? {
            return Err(AppError::conflict(
                "PM execution binding acknowledgement is not current",
            ));
        }
        Ok(())
    }
}
