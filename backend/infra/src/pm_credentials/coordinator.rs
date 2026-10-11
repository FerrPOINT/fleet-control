use super::*;
use app::{FleetRepository, pm_draft::PmDraftCredentials};
use async_trait::async_trait;
use domain::{PmCredentialIntent, PmCredentialReceipt, PmDraftOperation, PmDraftProof};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

pub struct PmCredentialCoordinator {
    issuer: PmCredentialIssuer,
    subject: String,
    ttl_seconds: i64,
    workflow: Option<crate::pm_workflow::PmWorkflowClient>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Principal {
    sub: String,
    email: String,
    scopes: Vec<String>,
    // Auth's additive display field is not assignment or scope authority.
    #[serde(default, rename = "display_name")]
    _display_name: String,
}

fn unavailable() -> AppError {
    AppError::Unavailable("PM credential verification is unavailable".into())
}

impl PmCredentialCoordinator {
    pub fn configured(config: &shared::AppConfig) -> Result<Option<Self>, AppError> {
        let credentials = &config.pm.credentials;
        if !credentials.enabled {
            return Ok(None);
        }
        let subject = Uuid::parse_str(&credentials.machine_subject)
            .map_err(|_| AppError::validation("invalid PM machine subject"))?;
        if subject.is_nil()
            || subject.to_string() != credentials.machine_subject
            || !(1..=1800).contains(&credentials.ttl_seconds)
        {
            return Err(AppError::validation("invalid PM credential configuration"));
        }
        Ok(Some(Self {
            issuer: PmCredentialIssuer::new(
                &credentials.auth_url,
                &config.tracker.url,
                &credentials.parent_pat,
            )?,
            subject: credentials.machine_subject.clone(),
            ttl_seconds: credentials.ttl_seconds,
            workflow: crate::pm_workflow::PmWorkflowClient::configured(config)?,
        }))
    }

    fn intent(&self, operation: &PmDraftOperation) -> Result<PmCredentialIntent, AppError> {
        let command = serde_json::to_value(PmCredentialCommand::tracker(
            &operation.execution_identity()?,
            operation.credential_key(),
            self.ttl_seconds,
        )?)
        .map_err(AppError::internal)?;
        // Pin the actual high-entropy parent secret, not just its shared user subject.
        let mut fingerprint = Sha256::new();
        fingerprint.update(b"fleet-pm-parent-v1\0");
        fingerprint.update(self.issuer.parent.as_bytes());
        Ok(PmCredentialIntent {
            request_sha256: domain::pm_canonical_hash(&command),
            command,
            parent_fingerprint: hex::encode(fingerprint.finalize()),
            base_origin: self.issuer.origin.as_str().to_string(),
            tracker_origin: self.issuer.tracker_origin.as_str().to_string(),
            machine_subject: self.subject.clone(),
        })
    }

    async fn get(
        &self,
        url: Url,
        authorization: header::HeaderValue,
        limit: usize,
    ) -> Result<serde_json::Value, AppError> {
        let request = self
            .issuer
            .client
            .get(url)
            .header(header::AUTHORIZATION, authorization)
            .header(header::ACCEPT_ENCODING, "identity")
            .build()
            .map_err(|_| unavailable())?;
        self.read_json(request, limit).await
    }

    async fn read_json(
        &self,
        request: reqwest::Request,
        limit: usize,
    ) -> Result<serde_json::Value, AppError> {
        let allows_created = request.method() == reqwest::Method::POST;
        let mut response = self
            .issuer
            .client
            .execute(request)
            .await
            .map_err(|_| unavailable())?;
        match response.status() {
            StatusCode::OK => (),
            StatusCode::CREATED if allows_created => (),
            StatusCode::UNAUTHORIZED => return Err(AppError::Unauthorized),
            StatusCode::FORBIDDEN => return Err(AppError::Forbidden),
            StatusCode::CONFLICT => {
                return Err(AppError::conflict("PM credential assignment changed"));
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
            || response
                .content_length()
                .is_some_and(|size| size > limit as u64)
        {
            return Err(unavailable());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
            if bytes.len().saturating_add(chunk.len()) > limit {
                return Err(unavailable());
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| unavailable())
    }

    /// A fresh prerequisite observation, never lease ownership or model permission.
    pub async fn read_execution_lease(
        &self,
        operation: &PmDraftOperation,
        credential: &PmDelegatedCredential,
        original: Option<&domain::PmExecutionLeaseCommand>,
    ) -> Result<domain::PmExecutionLeaseReadback, AppError> {
        let identity = operation.identity()?;
        let journal = operation.credentials.as_ref().ok_or_else(unavailable)?;
        let receipt = journal.receipt.as_ref().ok_or_else(unavailable)?;
        if operation
            .reservation
            .as_ref()
            .ok_or_else(unavailable)?
            .assignment
            .machine_subject
            != self.subject
            || journal.intent != self.intent(operation)?
            || receipt.token_id != credential.token_id
            || receipt.expires_at != credential.expires_at
            || receipt.scopes
                != PmCredentialCommand::tracker(
                    &operation.execution_identity()?,
                    operation.credential_key(),
                    self.ttl_seconds,
                )?
                .scopes()
        {
            return Err(AppError::conflict("PM lease credential binding changed"));
        }
        self.principal(credential.bearer.clone(), &receipt.scopes)
            .await?;
        self.context(operation, credential).await?;
        let mut url = self.issuer.tracker_origin.clone();
        url.set_path(&format!(
            "/api/v1/issues/{}/sdlc/pm-draft-execution-lease",
            identity.task_id
        ));
        if let Some(command) = original {
            url.query_pairs_mut()
                .append_pair("idempotency_key", command.idempotency_key());
        }
        let request = credential.authorize(
            self.issuer
                .client
                .get(url)
                .timeout(Duration::from_secs(5))
                .header(header::CACHE_CONTROL, "no-cache, no-store")
                .header(header::ACCEPT_ENCODING, "identity"),
        )?;
        let value = self.read_json(request, MAX_RESPONSE_BYTES).await?;
        if credential.expires_at <= Utc::now() {
            return Err(AppError::Unauthorized);
        }
        domain::PmExecutionLeaseReadback::verified(
            value,
            operation.reservation.as_ref().ok_or_else(unavailable)?,
            original,
        )
    }

    /// Persist the original claim before mutation; unknown replies recover by keyed readback.
    /// An active lease is only a prerequisite and never authorizes a model/run by itself.
    pub async fn claim_execution_lease(
        &self,
        repo: &dyn FleetRepository,
        operation: &PmDraftOperation,
        credential: &PmDelegatedCredential,
    ) -> Result<domain::PmExecutionLeaseReadback, AppError> {
        let claim = operation.lease_claim()?;
        let saved = repo
            .record_pm_draft_proof(
                operation.id,
                operation.owner_user_id,
                PmDraftProof::LeaseIntent(claim.clone()),
            )
            .await?;
        let original = domain::PmExecutionLeaseCommand::Claim(claim.clone());
        let before = self
            .read_execution_lease(&saved, credential, Some(&original))
            .await?;
        let mut posted_receipt = None;
        if before.operation.is_none() {
            if before.state != domain::PmExecutionLeaseState::Unclaimed {
                return Err(AppError::conflict(
                    "PM lease claim requires original-key reconciliation",
                ));
            }
            let identity = saved.identity()?;
            let mut url = self.issuer.tracker_origin.clone();
            url.set_path(&format!(
                "/api/v1/issues/{}/sdlc/pm-draft-execution-lease",
                identity.task_id
            ));
            let request = credential.authorize(
                self.issuer
                    .client
                    .post(url)
                    .timeout(Duration::from_secs(5))
                    .header(header::ACCEPT_ENCODING, "identity")
                    .json(&claim),
            )?;
            let receipt: domain::PmExecutionLeaseReceipt =
                canonical(self.read_json(request, MAX_RESPONSE_BYTES).await?)?;
            receipt.verify_claim(saved.reservation.as_ref().ok_or_else(unavailable)?, &claim)?;
            posted_receipt = Some(receipt);
        }
        let current = self
            .read_execution_lease(&saved, credential, Some(&original))
            .await?;
        if current.state != domain::PmExecutionLeaseState::Active {
            return Err(AppError::conflict(
                "PM execution lease is not active; quiescence recovery required",
            ));
        }
        let receipt = current
            .operation
            .as_ref()
            .ok_or_else(unavailable)?
            .result
            .clone();
        if posted_receipt
            .as_ref()
            .is_some_and(|posted| posted != &receipt)
        {
            return Err(AppError::conflict(
                "PM lease acknowledgement differs from original-key readback",
            ));
        }
        repo.record_pm_draft_proof(
            saved.id,
            saved.owner_user_id,
            PmDraftProof::LeaseAcknowledged(receipt),
        )
        .await?;
        Ok(current)
    }

    /// Renew the same server-issued generation. The key is derived from its CAS
    /// cursor, so concurrent workers and a lost reply cannot extend it twice.
    pub async fn renew_execution_lease(
        &self,
        operation: &PmDraftOperation,
        credential: &PmDelegatedCredential,
    ) -> Result<domain::PmExecutionLeaseReadback, AppError> {
        let journal = operation.execution_lease.as_ref().ok_or_else(unavailable)?;
        let receipt = journal.receipt.as_ref().ok_or_else(unavailable)?;
        let original = domain::PmExecutionLeaseCommand::Claim(journal.claim.clone());
        let before = self
            .read_execution_lease(operation, credential, Some(&original))
            .await?;
        let lease = before.current.as_ref().ok_or_else(unavailable)?;
        if before.state != domain::PmExecutionLeaseState::Active
            || before.operation.is_none()
            || lease.lease_id != receipt.lease.lease_id
        {
            return Err(AppError::conflict("PM original lease is not active"));
        }
        if lease.expires_at - before.observed_at > ChronoDuration::seconds(20) {
            return Ok(before);
        }
        let command =
            domain::PmExecutionLeaseCommand::Heartbeat(domain::PmExecutionLeaseHeartbeat {
                expected_owner_version: journal.claim.expected_owner_version,
                fence: journal.claim.fence.clone(),
                lease_id: lease.lease_id,
                expected_lease_version: lease.version,
                idempotency_key: format!("fleet-pm-heartbeat:{}:{}", lease.lease_id, lease.version),
            });
        let mut url = self.issuer.tracker_origin.clone();
        url.set_path(&format!(
            "/api/v1/issues/{}/sdlc/pm-draft-execution-lease/heartbeat",
            operation.identity()?.task_id
        ));
        let request = credential.authorize(
            self.issuer
                .client
                .post(url)
                .timeout(Duration::from_secs(5))
                .header(header::ACCEPT_ENCODING, "identity")
                .json(&command.payload()),
        )?;
        // A transport error or concurrent CAS conflict is resolved only through
        // the original keyed read. Auth denials must remain denials.
        let posted = match self.read_json(request, MAX_RESPONSE_BYTES).await {
            Ok(value) => Some(canonical::<domain::PmExecutionLeaseReceipt>(value)?),
            Err(AppError::Unauthorized) => return Err(AppError::Unauthorized),
            Err(AppError::Forbidden) => return Err(AppError::Forbidden),
            Err(_) => None,
        };
        let current = self
            .read_execution_lease(operation, credential, Some(&command))
            .await?;
        let acknowledgement = current.operation.as_ref().ok_or_else(unavailable)?;
        if current.state != domain::PmExecutionLeaseState::Active
            || posted
                .as_ref()
                .is_some_and(|v| v != &acknowledgement.result)
        {
            return Err(AppError::conflict(
                "PM heartbeat acknowledgement is not current",
            ));
        }
        Ok(current)
    }

    async fn principal(
        &self,
        authorization: header::HeaderValue,
        scopes: &[String],
    ) -> Result<(), AppError> {
        let mut url = self.issuer.origin.clone();
        url.set_path("/auth/tokens/introspect");
        let principal: Principal =
            serde_json::from_value(self.get(url, authorization, MAX_RESPONSE_BYTES).await?)
                .map_err(|_| unavailable())?;
        let mut actual = principal.scopes;
        actual.sort_unstable();
        if principal.sub != self.subject || principal.email.trim().is_empty() || actual != scopes {
            return Err(AppError::Forbidden);
        }
        Ok(())
    }

    async fn context(
        &self,
        operation: &PmDraftOperation,
        credential: &PmDelegatedCredential,
    ) -> Result<(), AppError> {
        let identity = operation.identity()?;
        let mut url = self.issuer.tracker_origin.clone();
        url.set_path(&format!("/api/v1/issues/{}/sdlc/context", identity.task_id));
        let request = credential.authorize(self.issuer.client.get(url))?;
        let value = self
            .get(request.url().clone(), credential.bearer.clone(), 262_144)
            .await?;
        let context: domain::TrackerTaskContext = canonical(value)?;
        let assignment = context
            .assignment
            .ok_or_else(|| AppError::conflict("PM assignment is unavailable"))?;
        let expected = &operation
            .reservation
            .as_ref()
            .ok_or_else(unavailable)?
            .assignment;
        if context.contract_version != 1
            || context.tracker_instance_id != identity.tracker_instance_id
            || context.project_id != identity.project_id
            || context.task_id != identity.task_id
            || context.root_task_id != identity.root_task_id
            || context.owner_subject != identity.owner_subject
            || !matches!(
                context.stage,
                domain::TrackerStage::Draft | domain::TrackerStage::Clarification
            )
            || context.permissions.can_answer
            || context.permissions.can_confirm
            || assignment.assignment_id != expected.assignment_id
            || assignment.execution_id != expected.execution_id
            || assignment.agent_id != expected.agent_id
            || u64::try_from(assignment.version).ok() != Some(expected.version)
            || assignment.machine_subject != self.subject
        {
            return Err(AppError::conflict(
                "PM credential context is stale or inconsistent",
            ));
        }
        if credential.expires_at <= Utc::now() {
            return Err(AppError::Unauthorized);
        }
        Ok(())
    }

    /// Returns a memory-only credential for the later admission/tools coordinator.
    /// This operation alone does not claim a lease, bind a workflow or dispatch a model.
    pub async fn prepare_credential(
        &self,
        repo: &dyn FleetRepository,
        operation: &PmDraftOperation,
    ) -> Result<PmDelegatedCredential, AppError> {
        let saved = repo
            .record_pm_draft_proof(
                operation.id,
                operation.owner_user_id,
                PmDraftProof::CredentialIntent(self.intent(operation)?),
            )
            .await?;
        let journal = saved.credentials.as_ref().ok_or_else(unavailable)?;
        if journal
            .receipt
            .as_ref()
            .is_some_and(|receipt| receipt.expires_at <= Utc::now())
        {
            return Err(AppError::Unauthorized);
        }
        // Read authorization again on every replay, before reusing the original Base command.
        self.principal(
            self.issuer.parent.clone(),
            &["task-tracker:read".into(), "task-tracker:write".into()],
        )
        .await?;
        let command = PmCredentialCommand::tracker(
            &saved.execution_identity()?,
            saved.credential_key(),
            self.ttl_seconds,
        )?;
        let credential = self.issuer.issue(&command).await?;
        let receipt = PmCredentialReceipt {
            token_id: credential.token_id,
            expires_at: credential.expires_at,
            scopes: command.scopes().to_vec(),
        };
        // An acknowledged child survives later Tracker failure without storing its secret.
        let saved = repo
            .record_pm_draft_proof(
                saved.id,
                saved.owner_user_id,
                PmDraftProof::CredentialAcknowledged(receipt),
            )
            .await?;
        // Only a previously journaled original claim may be reconciled. Another
        // active or expired lease is never adopted by credential preparation.
        let original = saved
            .execution_lease
            .as_ref()
            .map(|journal| domain::PmExecutionLeaseCommand::Claim(journal.claim.clone()));
        let lease = self
            .read_execution_lease(&saved, &credential, original.as_ref())
            .await?;
        let known_active = original.is_some()
            && lease.state == domain::PmExecutionLeaseState::Active
            && lease.operation.is_some();
        if lease.state != domain::PmExecutionLeaseState::Unclaimed && !known_active {
            return Err(AppError::conflict(
                "PM execution lease requires original-key reconciliation",
            ));
        }
        Ok(credential)
    }
}

fn canonical<T: DeserializeOwned + Serialize>(value: serde_json::Value) -> Result<T, AppError> {
    let typed: T = serde_json::from_value(value.clone()).map_err(|_| unavailable())?;
    if serde_json::to_value(&typed).map_err(AppError::internal)? != value {
        return Err(unavailable());
    }
    Ok(typed)
}

#[async_trait]
impl PmDraftCredentials for PmCredentialCoordinator {
    async fn prepare(
        &self,
        repo: &dyn FleetRepository,
        operation: &PmDraftOperation,
    ) -> Result<(), AppError> {
        let credential = self.prepare_credential(repo, operation).await?;
        if let Some(workflow) = &self.workflow {
            let saved = repo
                .read_pm_draft_operation(operation.id, operation.owner_user_id)
                .await?;
            self.claim_execution_lease(repo, &saved, &credential)
                .await?;
            let saved = repo
                .read_pm_draft_operation(operation.id, operation.owner_user_id)
                .await?;
            workflow
                .prepare_assignment(repo, &saved, self, &credential)
                .await?;
            let session = saved.session_id.ok_or_else(unavailable)?;
            repo.create_session_message(
                session,
                domain::CreateSessionMessageRequest {
                    body: saved
                        .input
                        .as_ref()
                        .ok_or_else(unavailable)?
                        .input
                        .description
                        .clone(),
                    author_agent_id: None,
                    message_kind: Some(domain::MessageKind::UserPrompt),
                    runtime_message_id: None,
                    idempotency_key: Some(format!("fleet-pm-intake:{}", saved.id)),
                },
                saved.owner_user_id,
            )
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_auth_display_name_is_compatible_without_becoming_authority() {
        for display in [None, Some(serde_json::json!("PM service"))] {
            let mut value = serde_json::json!({"sub":"subject","email":"pm@example.test","scopes":["task-tracker:read"]});
            if let Some(display) = display {
                value["display_name"] = display;
            }
            let principal: Principal = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(principal.sub, "subject");
            assert_eq!(principal.scopes, ["task-tracker:read"]);
            value["dispatch_allowed"] = serde_json::json!(true);
            assert!(serde_json::from_value::<Principal>(value).is_err());
        }
        assert!(serde_json::from_value::<Principal>(serde_json::json!({"sub":"subject","email":"pm@example.test","scopes":[],"display_name":true})).is_err());
    }

    #[test]
    fn configuration_is_opt_in_and_requires_canonical_subject_and_fixed_origins() {
        let mut config = shared::AppConfig::default();
        assert!(
            PmCredentialCoordinator::configured(&config)
                .unwrap()
                .is_none()
        );
        config.pm.credentials = shared::PmCredentialsConfig {
            enabled: true,
            auth_url: "https://base/".into(),
            machine_subject: Uuid::new_v4().to_string(),
            parent_pat: "sdlc_pat_parent-test-secret-1234567890".into(),
            ttl_seconds: 300,
        };
        config.tracker.url = "https://tracker/".into();
        assert!(
            PmCredentialCoordinator::configured(&config)
                .unwrap()
                .is_some()
        );
        for subject in [
            "fleet-orchestrator",
            "00000000-0000-0000-0000-000000000000",
            "AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA",
        ] {
            let mut wrong = config.clone();
            wrong.pm.credentials.machine_subject = subject.into();
            assert!(PmCredentialCoordinator::configured(&wrong).is_err());
        }
        for ttl in [0, 1801] {
            let mut wrong = config.clone();
            wrong.pm.credentials.ttl_seconds = ttl;
            assert!(PmCredentialCoordinator::configured(&wrong).is_err());
        }
        for origin in [
            "https://base/path",
            "https://base/?query=1",
            "https://secret@base/",
        ] {
            let mut wrong = config.clone();
            wrong.pm.credentials.auth_url = origin.into();
            assert!(PmCredentialCoordinator::configured(&wrong).is_err());
        }
    }
}
