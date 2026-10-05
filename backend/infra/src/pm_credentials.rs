//! Server-only, assignment-scoped credentials. Never expose the parent PAT to a runtime.
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use reqwest::{Client, RequestBuilder, StatusCode, Url, header};
use serde::Deserialize;
use shared::AppError;
use std::time::Duration;
use uuid::Uuid;

const MAX_RESPONSE_BYTES: usize = 16_384;

fn acknowledgement_expiry_valid(
    expires_at: DateTime<Utc>,
    acknowledged: DateTime<Utc>,
    checked_at: DateTime<Utc>,
    ttl_seconds: i64,
) -> bool {
    expires_at > checked_at && expires_at <= acknowledged + ChronoDuration::seconds(ttl_seconds + 5)
}

pub use domain::PmCredentialCommand;

mod coordinator;
pub use coordinator::PmCredentialCoordinator;

pub struct PmCredentialIssuer {
    origin: Url,
    tracker_origin: Url,
    parent: header::HeaderValue,
    client: Client,
}

impl std::fmt::Debug for PmCredentialIssuer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PmCredentialIssuer")
            .field("parent", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IssuedToken {
    secret: String,
    token_id: Uuid,
    expires_at: DateTime<Utc>,
    scopes: Vec<String>,
}

/// Not serializable; credentials can authenticate requests, not enter JSON/logs or transcripts.
pub struct PmDelegatedCredential {
    bearer: header::HeaderValue,
    tracker_origin: Url,
    task_id: Uuid,
    token_id: Uuid,
    expires_at: DateTime<Utc>,
}

impl std::fmt::Debug for PmDelegatedCredential {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PmDelegatedCredential")
            .field("token_id", &self.token_id)
            .field("expires_at", &self.expires_at)
            .field("bearer", &"[REDACTED]")
            .finish()
    }
}

impl PmDelegatedCredential {
    pub fn token_id(&self) -> Uuid {
        self.token_id
    }

    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    /// Receiving services still perform fresh parent/subject/assignment authorization.
    pub fn authorize(&self, request: RequestBuilder) -> Result<reqwest::Request, AppError> {
        if self.expires_at <= Utc::now() {
            return Err(AppError::Unauthorized);
        }
        let mut request = request
            .build()
            .map_err(|_| AppError::validation("invalid Tracker request"))?;
        let url = request.url();
        if url.origin() != self.tracker_origin.origin()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
            || !permitted_task_request(request.method(), url.path(), self.task_id)
            || request.headers().contains_key(header::AUTHORIZATION)
        {
            return Err(AppError::Forbidden);
        }
        request
            .headers_mut()
            .insert(header::AUTHORIZATION, self.bearer.clone());
        Ok(request)
    }
}

fn permitted_task_request(method: &reqwest::Method, path: &str, task_id: Uuid) -> bool {
    let prefix = format!("/api/v1/issues/{task_id}/sdlc/");
    let Some(operation) = path.strip_prefix(&prefix) else {
        return false;
    };
    match *method {
        reqwest::Method::GET => {
            if matches!(
                operation,
                "context"
                    | "clarifications"
                    | "requirements"
                    | "requirements/revisions"
                    | "pm-draft-input"
                    | "pm-draft-execution-lease"
            ) {
                return true;
            }
            let Some(revision) = operation.strip_prefix("requirements/") else {
                return false;
            };
            let revision = revision.strip_suffix("/diff").unwrap_or(revision);
            revision.parse::<u64>().is_ok_and(|number| {
                (1..=9_007_199_254_740_991).contains(&number) && number.to_string() == revision
            })
        }
        reqwest::Method::POST => {
            if matches!(
                operation,
                "clarifications"
                    | "requirements"
                    | "pm-draft-execution-lease"
                    | "pm-draft-execution-lease/heartbeat"
            ) {
                return true;
            }
            operation
                .strip_prefix("clarifications/")
                .and_then(|value| value.strip_suffix("/cancel"))
                .is_some_and(|raw| {
                    Uuid::parse_str(raw).is_ok_and(|id| !id.is_nil() && id.to_string() == raw)
                })
        }
        _ => false,
    }
}

fn bearer(secret: &str) -> Result<header::HeaderValue, AppError> {
    if !secret.starts_with("sdlc_pat_")
        || !(32..=128).contains(&secret.len())
        || !secret.bytes().all(|c| c.is_ascii_graphic())
    {
        return Err(AppError::validation("invalid machine credential"));
    }
    let mut value = header::HeaderValue::from_str(&format!("Bearer {secret}"))
        .map_err(|_| AppError::validation("invalid machine credential"))?;
    value.set_sensitive(true);
    Ok(value)
}

impl PmCredentialIssuer {
    /// The configured Base root origin and parent PAT are operator-managed server secrets.
    pub fn new(origin: &str, tracker_origin: &str, parent: &str) -> Result<Self, AppError> {
        let origin = configured_origin(origin)?;
        let tracker_origin = configured_origin(tracker_origin)?;
        Ok(Self {
            origin,
            tracker_origin,
            parent: bearer(parent)?,
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .no_proxy()
                .build()
                .map_err(|_| AppError::Unavailable("credential issuer is unavailable".into()))?,
        })
    }

    /// One HTTP attempt only. An unknown outcome must replay this persisted command/key.
    pub async fn issue(
        &self,
        command: &PmCredentialCommand,
    ) -> Result<PmDelegatedCredential, AppError> {
        let mut url = self.origin.clone();
        url.set_path("/auth/tokens/delegate");
        let mut response = self
            .client
            .post(url)
            .header(header::AUTHORIZATION, self.parent.clone())
            .json(command)
            .send()
            .await
            .map_err(|_| {
                AppError::Unavailable(
                    "credential issue outcome is unknown; reconcile the original key".into(),
                )
            })?;
        // Base starts the lifetime after its database locks, not at HTTP request start.
        let acknowledged = Utc::now();
        match response.status() {
            StatusCode::OK | StatusCode::CREATED => (),
            StatusCode::UNAUTHORIZED => return Err(AppError::Unauthorized),
            StatusCode::FORBIDDEN => return Err(AppError::Forbidden),
            StatusCode::CONFLICT => {
                return Err(AppError::conflict("credential operation payload conflict"));
            }
            _ => {
                return Err(AppError::Unavailable(
                    "credential issuer did not acknowledge the command".into(),
                ));
            }
        }
        let no_store = response
            .headers()
            .get_all(header::CACHE_CONTROL)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(','))
            .any(|value| value.trim().eq_ignore_ascii_case("no-store"));
        if !no_store {
            return Err(AppError::Unavailable(
                "credential issuer omitted no-store protection".into(),
            ));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| {
            AppError::Unavailable("credential issue acknowledgement was interrupted".into())
        })? {
            if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
                return Err(AppError::Unavailable(
                    "credential acknowledgement exceeds limit".into(),
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        let token: IssuedToken = serde_json::from_slice(&bytes)
            .map_err(|_| AppError::Unavailable("credential acknowledgement is invalid".into()))?;
        if token.token_id.is_nil()
            || token.scopes != command.scopes()
            || !acknowledgement_expiry_valid(
                token.expires_at,
                acknowledged,
                Utc::now(),
                command.ttl_seconds(),
            )
        {
            return Err(AppError::Unavailable(
                "credential acknowledgement scope or expiry mismatch".into(),
            ));
        }
        let bearer = bearer(&token.secret).map_err(|_| {
            AppError::Unavailable("credential acknowledgement contains an invalid secret".into())
        })?;
        Ok(PmDelegatedCredential {
            bearer,
            tracker_origin: self.tracker_origin.clone(),
            task_id: command.task_id(),
            token_id: token.token_id,
            expires_at: token.expires_at,
        })
    }

    pub async fn revoke(&self, credential: &PmDelegatedCredential) -> Result<(), AppError> {
        let mut url = self.origin.clone();
        url.set_path(&format!("/auth/tokens/{}", credential.token_id));
        let response = self
            .client
            .delete(url)
            .header(header::AUTHORIZATION, self.parent.clone())
            .send()
            .await
            .map_err(|_| {
                AppError::Unavailable("credential revocation outcome is unknown".into())
            })?;
        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            StatusCode::UNAUTHORIZED => Err(AppError::Unauthorized),
            StatusCode::FORBIDDEN => Err(AppError::Forbidden),
            _ => Err(AppError::Unavailable(
                "credential revocation was not acknowledged".into(),
            )),
        }
    }
}

fn configured_origin(raw: &str) -> Result<Url, AppError> {
    let origin = Url::parse(raw).map_err(|_| AppError::validation("invalid credential origin"))?;
    if !matches!(origin.scheme(), "http" | "https")
        || origin.host_str().is_none()
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.path() != "/"
        || origin.query().is_some()
        || origin.fragment().is_some()
    {
        return Err(AppError::validation("invalid credential origin"));
    }
    Ok(origin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        extract::State,
        http::HeaderMap,
        routing::{delete, post},
    };
    use serde_json::{Value, json};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    const PARENT: &str = "sdlc_pat_parent-test-secret-1234567890";
    const CHILD: &str = "sdlc_pat_child-test-secret-1234567890";

    fn identity() -> domain::PmExecutionIdentity {
        domain::PmExecutionIdentity {
            task: "SDLC-1".into(),
            execution_ref: Uuid::new_v4().to_string(),
            tracker_instance_ref: "tracker-test".into(),
            tracker_project_ref: Uuid::new_v4().to_string(),
            task_ref: Uuid::new_v4().to_string(),
            root_ref: Uuid::new_v4().to_string(),
            agent_ref: Uuid::new_v4().to_string(),
            assignment_operation_key: "assign-key".into(),
            assignment_ref: Uuid::new_v4().to_string(),
            assignment_revision: 1,
        }
    }

    #[test]
    fn only_server_derived_assignment_scopes_and_bounded_commands() {
        let mut identity = identity();
        let request = PmCredentialCommand::tracker(&identity, "persisted-key".into(), 900).unwrap();
        let body = serde_json::to_value(&request).unwrap();
        assert_eq!(body.as_object().unwrap().len(), 5);
        assert!(body.get("task_id").is_none());
        assert_eq!(body["service"], "task-tracker");
        assert_eq!(body["scopes"].as_array().unwrap().len(), 3);
        assert!(request.scopes().contains(&format!(
            "task-tracker:sdlc:pm:{}:{}:{}:{}:1",
            identity.task_ref, identity.assignment_ref, identity.execution_ref, identity.agent_ref,
        )));
        for key in ["", "unsafe key", "\u{e9}"] {
            assert!(PmCredentialCommand::tracker(&identity, key.into(), 900).is_err());
        }
        assert!(PmCredentialCommand::tracker(&identity, "x".repeat(129), 900).is_err());
        assert!(PmCredentialCommand::tracker(&identity, "x".repeat(128), 900).is_ok());
        for ttl in [0, -1, 1801, i64::MAX] {
            assert!(PmCredentialCommand::tracker(&identity, "key".into(), ttl).is_err());
        }
        identity.assignment_ref = Uuid::nil().to_string();
        assert!(PmCredentialCommand::tracker(&identity, "key".into(), 900).is_err());
        identity.assignment_ref = Uuid::new_v4().to_string();
        identity.assignment_revision = 9_007_199_254_740_992;
        assert!(PmCredentialCommand::tracker(&identity, "key".into(), 900).is_err());
    }

    #[test]
    fn origins_and_secrets_are_strict_and_debug_is_redacted() {
        for origin in [
            "file:///tmp/base",
            "https://user:secret@base/",
            "https://base/auth",
            "https://base/?secret=x",
            "https://base/#x",
        ] {
            assert!(PmCredentialIssuer::new(origin, "https://tracker/", PARENT).is_err());
            assert!(PmCredentialIssuer::new("https://base/", origin, PARENT).is_err());
        }
        for secret in [
            "browser-jwt",
            "sdlc_pat_short",
            "sdlc_pat_unsafe secret-1234567890",
        ] {
            assert!(PmCredentialIssuer::new("https://base/", "https://tracker/", secret).is_err());
        }
        let issuer = PmCredentialIssuer::new("https://base/", "https://tracker/", PARENT).unwrap();
        assert!(!format!("{issuer:?}").contains(PARENT));
        assert!(issuer.parent.is_sensitive());
    }

    #[test]
    fn credentials_allow_only_bound_pm_operations_not_owner_or_legacy_actions() {
        let task = Uuid::new_v4();
        let path = |suffix: &str| format!("/api/v1/issues/{task}/sdlc/{suffix}");
        for operation in [
            "context",
            "clarifications",
            "requirements",
            "requirements/revisions",
            "requirements/1",
            "requirements/9007199254740991/diff",
            "pm-draft-input",
            "pm-draft-execution-lease",
        ] {
            assert!(permitted_task_request(
                &reqwest::Method::GET,
                &path(operation),
                task
            ));
        }
        let cancel = format!("clarifications/{}/cancel", Uuid::new_v4());
        for operation in [
            "clarifications",
            "requirements",
            "pm-draft-execution-lease",
            "pm-draft-execution-lease/heartbeat",
            &cancel,
        ] {
            assert!(permitted_task_request(
                &reqwest::Method::POST,
                &path(operation),
                task
            ));
        }
        for operation in [
            "",
            "context/",
            "events",
            "binding",
            "assignment",
            "evidence",
            "requirements/0",
            "requirements/01",
            "requirements/+1",
            "requirements/9007199254740992",
            "requirements/1/confirm",
            "clarifications/00000000-0000-0000-0000-000000000000/cancel",
            "clarifications/11111111-ABCD-4111-8111-111111111111/cancel",
            "clarifications/11111111-abcd-4111-8111-111111111111/answers",
        ] {
            for method in [reqwest::Method::GET, reqwest::Method::POST] {
                assert!(!permitted_task_request(&method, &path(operation), task));
            }
        }
        assert!(!permitted_task_request(
            &reqwest::Method::GET,
            &format!("/api/v1/issues/{}/sdlc/context", Uuid::new_v4()),
            task
        ));
    }

    #[derive(Clone, Copy)]
    struct IssuedTimes {
        issued_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
    }

    #[derive(Clone)]
    struct TestState {
        mode: Arc<AtomicUsize>,
        calls: Arc<AtomicUsize>,
        redirected: Arc<AtomicUsize>,
        token_id: Uuid,
        issued_times: Arc<std::sync::Mutex<Option<IssuedTimes>>>,
    }

    async fn issue(
        State(state): State<TestState>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> (StatusCode, HeaderMap, String) {
        assert_eq!(headers[header::AUTHORIZATION], format!("Bearer {PARENT}"));
        assert_eq!(body["idempotency_key"], "persisted-key");
        state.calls.fetch_add(1, Ordering::SeqCst);
        let mut headers = HeaderMap::new();
        headers.insert(header::CACHE_CONTROL, "private, no-store".parse().unwrap());
        let mode = state.mode.load(Ordering::SeqCst);
        if mode == 17 || mode == 18 {
            tokio::time::sleep(Duration::from_secs(6)).await;
        }
        let issued_at = Utc::now();
        let mut token = json!({"secret":CHILD,"token_id":state.token_id,"expires_at":issued_at+ChronoDuration::seconds(100),"scopes":body["scopes"]});
        let mut status = StatusCode::CREATED;
        match mode {
            1 => status = StatusCode::OK,
            2 => token["scopes"] = json!(["task-tracker:write"]),
            3 => token["token_id"] = json!(Uuid::nil()),
            4 => token["expires_at"] = json!(Utc::now() - ChronoDuration::seconds(1)),
            5 => token["expires_at"] = json!(Utc::now() + ChronoDuration::seconds(1801)),
            6 => token["secret"] = json!("unexpected secret"),
            7 => {
                headers.remove(header::CACHE_CONTROL);
            }
            8 => return (status, headers, "not-json".into()),
            9 => return (status, headers, "x".repeat(MAX_RESPONSE_BYTES + 1)),
            10 => {
                headers.insert(header::LOCATION, "/redirect-target".parse().unwrap());
                status = StatusCode::TEMPORARY_REDIRECT;
            }
            11 => status = StatusCode::UNAUTHORIZED,
            12 => status = StatusCode::FORBIDDEN,
            13 => status = StatusCode::CONFLICT,
            14 => status = StatusCode::INTERNAL_SERVER_ERROR,
            15 => token["untrusted"] = json!(true),
            16 => {
                token["scopes"] = json!([
                    "task-tracker:read",
                    "task-tracker:sdlc:pm:other",
                    "task-tracker:write"
                ])
            }
            18 => {
                token["expires_at"] = json!(
                    Utc::now()
                        + ChronoDuration::seconds(
                            body["expires_in_seconds"].as_i64().unwrap() + 60
                        )
                )
            }
            _ => (),
        }
        *state.issued_times.lock().unwrap() = Some(IssuedTimes {
            issued_at,
            expires_at: serde_json::from_value(token["expires_at"].clone()).unwrap(),
        });
        (status, headers, token.to_string())
    }

    #[tokio::test]
    async fn exact_scope_replay_and_revoke_never_follow_redirects_or_retry() {
        let state = TestState {
            mode: Arc::new(AtomicUsize::new(0)),
            calls: Arc::new(AtomicUsize::new(0)),
            redirected: Arc::new(AtomicUsize::new(0)),
            token_id: Uuid::new_v4(),
            issued_times: Arc::new(std::sync::Mutex::new(None)),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = PmCredentialIssuer::new(
            &format!("http://{}", listener.local_addr().unwrap()),
            "https://tracker/",
            PARENT,
        )
        .unwrap();
        let router = Router::new()
            .route("/auth/tokens/delegate", post(issue))
            .route(
                "/redirect-target",
                post(|State(state): State<TestState>| async move {
                    state.redirected.fetch_add(1, Ordering::SeqCst);
                    StatusCode::OK
                }),
            )
            .route(
                "/auth/tokens/{id}",
                delete(
                    |State(state): State<TestState>,
                     axum::extract::Path(id): axum::extract::Path<Uuid>,
                     headers: HeaderMap| async move {
                        assert_eq!(id, state.token_id);
                        assert_eq!(headers[header::AUTHORIZATION], format!("Bearer {PARENT}"));
                        if state.mode.load(Ordering::SeqCst) == 14 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::NO_CONTENT
                        }
                    },
                ),
            )
            .with_state(state.clone());
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let request =
            PmCredentialCommand::tracker(&identity(), "persisted-key".into(), 900).unwrap();
        let credential = issuer.issue(&request).await.unwrap();
        assert!(!format!("{credential:?}").contains(CHILD));
        assert!(credential.bearer.is_sensitive());
        let task_url = format!(
            "https://tracker/api/v1/issues/{}/sdlc/context",
            request.task_id()
        );
        let authorized = credential.authorize(Client::new().get(&task_url)).unwrap();
        assert_eq!(
            authorized.headers()[header::AUTHORIZATION],
            format!("Bearer {CHILD}")
        );
        assert!(!format!("{authorized:?}").contains(CHILD));
        for url in [
            "https://other/api/v1/issues",
            "http://tracker/api/v1/issues",
            "https://tracker:9443/api/v1/issues",
            "https://user@tracker/api/v1/issues",
            "https://tracker/auth/tokens",
            "https://tracker/api/v1/issues#x",
            "https://tracker/api/v1/issues",
            "https://tracker/api/v1/sdlc/project-directory",
        ] {
            assert!(
                matches!(
                    credential.authorize(Client::new().get(url)),
                    Err(AppError::Forbidden)
                ),
                "unexpected authorization result for {url}"
            );
        }
        for url in [
            format!(
                "https://tracker/api/v1/issues/{}/sdlc/context",
                Uuid::new_v4()
            ),
            format!(
                "https://tracker/api/v1/issues/{}/sdlc/../comments",
                request.task_id()
            ),
            format!("{task_url}#secret"),
            task_url.replace("https://tracker/", "https://tracker:9443/"),
        ] {
            assert!(matches!(
                credential.authorize(Client::new().get(url)),
                Err(AppError::Forbidden)
            ));
        }
        let publish_url = task_url.replace("/context", "/clarifications");
        assert!(
            credential
                .authorize(Client::new().post(&publish_url))
                .is_ok()
        );
        assert!(matches!(
            credential.authorize(Client::new().post(&task_url)),
            Err(AppError::Forbidden)
        ));
        for method in [
            reqwest::Method::PUT,
            reqwest::Method::PATCH,
            reqwest::Method::DELETE,
        ] {
            assert!(matches!(
                credential.authorize(Client::new().request(method, &task_url)),
                Err(AppError::Forbidden)
            ));
        }
        assert!(matches!(
            credential.authorize(Client::new().get(&task_url).bearer_auth(PARENT)),
            Err(AppError::Forbidden)
        ));
        state.mode.store(1, Ordering::SeqCst);
        assert_eq!(
            issuer.issue(&request).await.unwrap().token_id,
            credential.token_id
        );
        issuer.revoke(&credential).await.unwrap();
        for mode in 2..=16 {
            state.mode.store(mode, Ordering::SeqCst);
            let result = issuer.issue(&request).await;
            match mode {
                11 => assert!(matches!(result, Err(AppError::Unauthorized))),
                12 => assert!(matches!(result, Err(AppError::Forbidden))),
                13 => assert!(matches!(result, Err(AppError::Conflict(_)))),
                _ => assert!(matches!(result, Err(AppError::Unavailable(_)))),
            }
        }
        state.mode.store(14, Ordering::SeqCst);
        assert!(issuer.revoke(&credential).await.is_err());
        assert_eq!(state.calls.load(Ordering::SeqCst), 17);
        assert_eq!(state.redirected.load(Ordering::SeqCst), 0);
        let expired = PmDelegatedCredential {
            bearer: bearer(CHILD).unwrap(),
            tracker_origin: Url::parse("https://tracker/").unwrap(),
            task_id: request.task_id(),
            token_id: Uuid::new_v4(),
            expires_at: Utc::now() - ChronoDuration::seconds(1),
        };
        assert!(matches!(
            expired.authorize(Client::new().get(&task_url)),
            Err(AppError::Unauthorized)
        ));
        server.abort();
    }

    #[tokio::test]
    async fn issuer_lock_delay_preserves_bounded_ttl_and_one_attempt() {
        let state = TestState {
            mode: Arc::new(AtomicUsize::new(17)),
            calls: Arc::new(AtomicUsize::new(0)),
            redirected: Arc::new(AtomicUsize::new(0)),
            token_id: Uuid::new_v4(),
            issued_times: Arc::new(std::sync::Mutex::new(None)),
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let issuer = PmCredentialIssuer::new(
            &format!("http://{}", listener.local_addr().unwrap()),
            "https://tracker/",
            PARENT,
        )
        .unwrap();
        let router = Router::new()
            .route("/auth/tokens/delegate", post(issue))
            .with_state(state.clone());
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let command =
            PmCredentialCommand::tracker(&identity(), "persisted-key".into(), 100).unwrap();
        let started = std::time::Instant::now();
        let credential = issuer.issue(&command).await.unwrap();
        assert!(started.elapsed() >= Duration::from_secs(6));
        let IssuedTimes {
            issued_at,
            expires_at,
        } = state.issued_times.lock().unwrap().unwrap();
        assert_eq!(expires_at - issued_at, ChronoDuration::seconds(100));
        assert_eq!(credential.expires_at(), expires_at);
        assert_eq!(credential.token_id(), state.token_id);
        assert_eq!(state.calls.load(Ordering::SeqCst), 1);
        state.mode.store(18, Ordering::SeqCst);
        assert!(matches!(
            issuer.issue(&command).await,
            Err(AppError::Unavailable(_))
        ));
        assert_eq!(state.calls.load(Ordering::SeqCst), 2);
        server.abort();
    }

    #[test]
    fn acknowledgement_expiry_bounds_are_exact_and_do_not_spend_issuer_lock_delay() {
        let started = DateTime::<Utc>::from_timestamp(1_000_000, 0).unwrap();
        let acknowledged = started + ChronoDuration::seconds(6);
        let expires_at = acknowledged + ChronoDuration::seconds(100);
        assert!(acknowledgement_expiry_valid(
            expires_at,
            acknowledged,
            acknowledged,
            100
        ));
        assert!(!acknowledgement_expiry_valid(
            expires_at,
            started,
            acknowledged,
            100
        ));
        let ceiling = acknowledged + ChronoDuration::seconds(105);
        assert!(acknowledgement_expiry_valid(
            ceiling,
            acknowledged,
            acknowledged,
            100
        ));
        assert!(!acknowledgement_expiry_valid(
            ceiling + ChronoDuration::nanoseconds(1),
            acknowledged,
            acknowledged,
            100
        ));
        assert!(!acknowledgement_expiry_valid(
            acknowledged,
            acknowledged,
            acknowledged,
            100
        ));
        assert!(!acknowledgement_expiry_valid(
            expires_at,
            acknowledged,
            expires_at,
            100
        ));
    }

    #[tokio::test]
    async fn lost_issuer_is_unknown_not_a_new_command_and_never_echoes_secrets() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        let issuer = PmCredentialIssuer::new(&origin, "https://tracker/", PARENT).unwrap();
        let command =
            PmCredentialCommand::tracker(&identity(), "persisted-key".into(), 900).unwrap();
        let before = serde_json::to_value(&command).unwrap();
        let error = issuer.issue(&command).await.unwrap_err();
        assert!(matches!(error, AppError::Unavailable(_)));
        assert!(error.to_string().contains("original key"));
        assert!(!error.to_string().contains(PARENT));
        assert_eq!(serde_json::to_value(&command).unwrap(), before);
    }
}
