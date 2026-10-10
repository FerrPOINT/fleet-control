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
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Principal {
    sub: String,
    email: String,
    scopes: Vec<String>,
    #[serde(rename = "display_name")]
    _display_name: String,
}

fn unavailable() -> AppError {
    AppError::Unavailable("PM credential verification is unavailable".into())
}

impl PmCredentialCoordinator {
    /// Base owns idempotent delegation. Runtime leases are reconstructed in memory;
    /// no child bearer or native tool secret is persisted in Fleet.
    pub async fn runtime_credential(
        &self,
        operation: &PmDraftOperation,
    ) -> Result<PmDelegatedCredential, AppError> {
        let journal = operation.credentials.as_ref().ok_or_else(unavailable)?;
        if journal.intent != self.intent(operation)? {
            return Err(AppError::conflict("PM original credential context changed"));
        }
        self.principal(
            self.issuer.parent.clone(),
            &["task-tracker:read".into(), "task-tracker:write".into()],
        )
        .await?;
        let window = Utc::now()
            .timestamp()
            .div_euclid((self.ttl_seconds / 2).max(1));
        let command = PmCredentialCommand::tracker(
            &operation.execution_identity()?,
            format!("fleet-pm-tools:{}:{window}", operation.id),
            self.ttl_seconds,
        )?;
        let credential = self.issuer.issue(&command).await?;
        self.principal(credential.bearer.clone(), command.scopes())
            .await?;
        self.context(operation, &credential).await?;
        Ok(credential)
    }
    pub async fn tracker_call(
        &self,
        credential: &PmDelegatedCredential,
        method: reqwest::Method,
        suffix: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, AppError> {
        let mut url = self.issuer.tracker_origin.clone();
        url.set_path(&format!(
            "/api/v1/issues/{}/sdlc/{suffix}",
            credential.task_id
        ));
        let mut request = self
            .issuer
            .client
            .request(method, url)
            .header(header::ACCEPT_ENCODING, "identity")
            .header(header::CACHE_CONTROL, "no-cache, no-store");
        if let Some(body) = body {
            request = request.json(body);
        }
        let mut response = self
            .issuer
            .client
            .execute(credential.authorize(request)?)
            .await
            .map_err(|_| unavailable())?;
        match response.status() {
            StatusCode::OK | StatusCode::CREATED => (),
            StatusCode::UNAUTHORIZED => return Err(AppError::Unauthorized),
            StatusCode::FORBIDDEN => return Err(AppError::Forbidden),
            StatusCode::CONFLICT => {
                return Err(AppError::conflict(
                    "PM Tracker command is stale or conflicts",
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
                    .is_some_and(|m| m.trim().eq_ignore_ascii_case("application/json"))
            })
            || response
                .headers()
                .get(header::CONTENT_ENCODING)
                .is_some_and(|v| v != "identity")
        {
            return Err(unavailable());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
            if bytes.len().saturating_add(chunk.len()) > 262144 {
                return Err(unavailable());
            }
            bytes.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&bytes).map_err(|_| unavailable())
    }

    pub async fn machine_context(
        &self,
        operation: &PmDraftOperation,
        credential: &PmDelegatedCredential,
    ) -> Result<domain::TrackerTaskContext, AppError> {
        let context = canonical(
            self.tracker_call(credential, reqwest::Method::GET, "context", None)
                .await?,
        )?;
        domain::verify_pm_machine_context(operation, &context)?;
        Ok(context)
    }
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
        let mut response = self
            .issuer
            .client
            .get(url)
            .header(header::AUTHORIZATION, authorization)
            .header(header::ACCEPT_ENCODING, "identity")
            .send()
            .await
            .map_err(|_| unavailable())?;
        match response.status() {
            StatusCode::OK => (),
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
        self.principal(credential.bearer.clone(), command.scopes())
            .await?;
        self.context(&saved, &credential).await?;
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
        self.prepare_credential(repo, operation).await.map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn introspection_rejects_wrong_subject_and_non_exact_parent_or_child_scopes() {
        use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
        use serde_json::{Value, json};
        use std::sync::Arc;
        use tokio::sync::Mutex;

        const PARENT: &str = "sdlc_pat_parent-test-secret-1234567890";
        const CHILD: &str = "sdlc_pat_child-test-secret-1234567890";
        let subject = Uuid::new_v4().to_string();
        let response = Arc::new(Mutex::new(Value::Null));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = PmCredentialIssuer::new(
            &format!("http://{}/", listener.local_addr().unwrap()),
            "https://tracker/",
            PARENT,
        )
        .unwrap();
        let router = Router::new()
            .route(
                "/auth/tokens/introspect",
                get(
                    |State(response): State<Arc<Mutex<Value>>>, headers: HeaderMap| async move {
                        let authorization = headers["authorization"].to_str().unwrap();
                        assert!(
                            authorization == format!("Bearer {PARENT}")
                                || authorization == format!("Bearer {CHILD}")
                        );
                        Json(response.lock().await.clone())
                    },
                ),
            )
            .with_state(response.clone());
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let coordinator = PmCredentialCoordinator {
            issuer,
            subject: subject.clone(),
            ttl_seconds: 300,
        };
        let parent_scopes = vec![
            "task-tracker:read".to_string(),
            "task-tracker:write".to_string(),
        ];
        let mut child_scopes = parent_scopes.clone();
        child_scopes.push(format!(
            "task-tracker:sdlc:pm:{}:{}:{}:{}:1",
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4()
        ));
        child_scopes.sort_unstable();
        for (secret, scopes) in [(PARENT, parent_scopes), (CHILD, child_scopes)] {
            let valid = json!({"sub":subject,"email":"machine@example.test","display_name":"PM machine","scopes":scopes});
            let mut wrong_subject = valid.clone();
            wrong_subject["sub"] = json!(Uuid::new_v4());
            let mut extra = valid.clone();
            extra["scopes"]
                .as_array_mut()
                .unwrap()
                .push(json!("task-tracker:admin"));
            let mut invalid = vec![wrong_subject, extra];
            for (index, scope) in scopes.iter().enumerate() {
                let mut missing = valid.clone();
                missing["scopes"].as_array_mut().unwrap().remove(index);
                invalid.push(missing);
                let mut duplicate = valid.clone();
                duplicate["scopes"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!(scope));
                invalid.push(duplicate);
            }
            for value in invalid {
                // All negatives are valid closed DTOs, not deserialization failures.
                assert!(serde_json::from_value::<Principal>(value.clone()).is_ok());
                *response.lock().await = value;
                assert!(matches!(
                    coordinator
                        .principal(bearer(secret).unwrap(), &scopes)
                        .await,
                    Err(AppError::Forbidden)
                ));
            }
            let mut reordered = valid;
            reordered["scopes"].as_array_mut().unwrap().reverse();
            *response.lock().await = reordered;
            coordinator
                .principal(bearer(secret).unwrap(), &scopes)
                .await
                .unwrap();
        }
        server.abort();
    }

    #[test]
    fn introspection_accepts_the_pinned_base_dto_without_weakening_closed_fields() {
        let value = serde_json::json!({
            "sub": Uuid::new_v4().to_string(),
            "email": "machine@example.test",
            "scopes": ["task-tracker:read", "task-tracker:write"],
            "display_name": "PM machine"
        });
        assert!(serde_json::from_value::<Principal>(value.clone()).is_ok());
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove("display_name");
        assert!(serde_json::from_value::<Principal>(missing).is_err());
        let mut invalid = value.clone();
        invalid["display_name"] = serde_json::Value::Null;
        assert!(serde_json::from_value::<Principal>(invalid).is_err());
        let mut unknown = value;
        unknown["role"] = serde_json::json!("admin");
        assert!(serde_json::from_value::<Principal>(unknown).is_err());
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
