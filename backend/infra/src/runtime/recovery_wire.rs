//! Original-key readback only. A missing witness never authorizes another run submission.
use super::*;
use reqwest::{StatusCode, header};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SOURCE: &str = "bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3";
const CAPABILITIES_PATH: &str = "/fleet/v1/recovery/capabilities";
const LOOKUP_PATH: &str = "/fleet/v1/recovery/lookup";
const MAX_RESPONSE: usize = 16_384;
const MAX_REQUEST: usize = 1024 * 1024;
const MAX_LOOKUP: usize = 2 * MAX_REQUEST + 4096;

#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Endpoint {
    method: String,
    path: String,
}

// Deliberately no Debug: persisted facts include a credential scope fingerprint.
#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Capabilities {
    object: String,
    contract_version: u32,
    store_id: String,
    profile: String,
    scope_fingerprint: String,
    native_source_revision: String,
    lookup: Endpoint,
    non_dispatch: bool,
    durable_witness: bool,
}

impl Capabilities {
    fn validate(&self) -> Result<(), AppError> {
        let store = Uuid::parse_str(&self.store_id).ok();
        if self.object != "fleet.hermes.recovery.capabilities"
            || self.contract_version != 1
            || store.is_none_or(|id| id.is_nil() || id.to_string() != self.store_id)
            || self.profile != "default"
            || !sha256_ref(&self.scope_fingerprint)
            || self.native_source_revision != SOURCE
            || self.lookup.method != "POST"
            || self.lookup.path != LOOKUP_PATH
            || !self.non_dispatch
            || !self.durable_witness
        {
            return Err(unavailable());
        }
        Ok(())
    }
}

fn unavailable() -> AppError {
    AppError::Unavailable("Hermes original-key recovery proof is not verified".into())
}

fn sha256_ref(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn scope(token: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"default\0");
    hash.update(token.as_bytes());
    format!("{:x}", hash.finalize())
}

fn original(facts: &Value) -> Result<Option<Capabilities>, AppError> {
    let Some(value) = facts.get("fleet_recovery") else {
        return Ok(None);
    };
    let caps: Capabilities = serde_json::from_value(value.clone()).map_err(|_| unavailable())?;
    caps.validate()?;
    Ok(Some(caps))
}

pub(crate) fn store_id(facts: &Value) -> Result<Option<String>, AppError> {
    Ok(original(facts)?.map(|caps| caps.store_id))
}

async fn current(
    client: &reqwest::Client,
    base: &str,
    token: &str,
) -> Result<Capabilities, AppError> {
    let response = client
        .get(format!("{base}{CAPABILITIES_PATH}"))
        .bearer_auth(token)
        .header(header::ACCEPT_ENCODING, "identity")
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|_| unavailable())?;
    let bytes = hermes_wire::read_body(response, StatusCode::OK, MAX_RESPONSE).await?;
    let caps: Capabilities = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
    caps.validate()?;
    if caps.scope_fingerprint != scope(token) {
        return Err(unavailable());
    }
    Ok(caps)
}

pub(super) async fn capabilities(
    client: &reqwest::Client,
    base: &str,
    token: &str,
) -> Result<Value, AppError> {
    serde_json::to_value(current(client, base, token).await?).map_err(|_| unavailable())
}

#[derive(Serialize)]
struct Lookup<'a> {
    contract_version: u32,
    store_id: &'a str,
    idempotency_key: &'a str,
    request_json: &'a str,
    request_sha256: &'a str,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Found {
    object: String,
    contract_version: u32,
    store_id: String,
    idempotency_key: String,
    request_sha256: String,
    scope_fingerprint: String,
    profile: String,
    found: bool,
    run_id: String,
}

pub(super) async fn lookup(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    intent: &app::HermesDispatchIntent,
) -> Result<String, AppError> {
    hermes_wire::verify_intent(intent, base, token)?;
    if !intent.recovery_allowed
        || intent.state != "submitted"
        || intent.run.runtime_run_id.is_some()
        || intent.request_body.len() > MAX_REQUEST
    {
        return Err(unavailable());
    }
    let original = original(&intent.capabilities)?.ok_or_else(unavailable)?;
    if original.scope_fingerprint != scope(token) || current(client, base, token).await? != original
    {
        return Err(unavailable());
    }
    let body = serde_json::to_vec(&Lookup {
        contract_version: 1,
        store_id: &original.store_id,
        idempotency_key: &intent.idempotency_key,
        request_json: &intent.request_body,
        request_sha256: &intent.request_hash,
    })
    .map_err(|_| unavailable())?;
    if body.len() > MAX_LOOKUP {
        return Err(unavailable());
    }
    let response = client
        .post(format!("{base}{LOOKUP_PATH}"))
        .bearer_auth(token)
        .header(header::ACCEPT_ENCODING, "identity")
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|_| unavailable())?;
    let bytes = hermes_wire::read_body(response, StatusCode::OK, MAX_RESPONSE).await?;
    let found: Found = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
    if found.object != "fleet.hermes.recovery.lookup"
        || found.contract_version != 1
        || found.store_id != original.store_id
        || found.idempotency_key != intent.idempotency_key
        || found.request_sha256 != intent.request_hash
        || found.scope_fingerprint != original.scope_fingerprint
        || found.profile != "default"
        || !found.found
        || !crate::pm_execution::valid_hermes_ref(&found.run_id)
    {
        return Err(unavailable());
    }
    Ok(found.run_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> Value {
        json!({"fleet_recovery": {
            "object":"fleet.hermes.recovery.capabilities", "contract_version":1,
            "store_id":Uuid::new_v4().to_string(), "profile":"default",
            "scope_fingerprint":scope("recovery-fixture-only"), "native_source_revision":SOURCE,
            "lookup":{"method":"POST","path":LOOKUP_PATH}, "non_dispatch":true,"durable_witness":true
        }})
    }

    #[test]
    fn recovery_facts_are_closed_pinned_and_not_retroactively_added() {
        assert!(store_id(&json!({})).unwrap().is_none());
        let good = facts();
        assert!(store_id(&good).unwrap().is_some());
        for (key, value) in [
            ("object", json!("foreign.capabilities")),
            ("contract_version", json!(2)),
            ("store_id", json!(Uuid::nil().to_string())),
            ("profile", json!("room")),
            ("scope_fingerprint", json!("bad")),
            ("native_source_revision", json!("foreign")),
            ("non_dispatch", json!(false)),
            ("durable_witness", json!(false)),
            ("lookup", json!({"method":"POST","path":"/v1/runs"})),
            ("extra", json!(true)),
        ] {
            let mut bad = good.clone();
            bad["fleet_recovery"][key] = value;
            assert!(store_id(&bad).is_err(), "{key}");
        }
        assert!(store_id(&json!({"fleet_recovery":null})).is_err());
    }

    fn intent(base: &str) -> app::HermesDispatchIntent {
        let timestamp = chrono::Utc::now().fixed_offset();
        let message_id = Uuid::new_v4();
        let body = r#"{"input":"recovery fixture","session_id":"fleet:fixture:agent"}"#.to_owned();
        app::HermesDispatchIntent {
            message_id,
            run: SessionAgentRun {
                id: Uuid::new_v4(),
                session_id: Uuid::new_v4(),
                agent_id: Uuid::new_v4(),
                agent_name: "fixture".into(),
                runtime_session_id: Some("fleet:fixture:agent".into()),
                runtime_run_id: None,
                run_role: SessionRunRole::Primary,
                state: SessionRunState::Pending,
                last_error: None,
                last_event_at: None,
                model: None,
                provider: None,
                model_options: json!({}),
                created_at: timestamp.to_rfc3339(),
                updated_at: timestamp.to_rfc3339(),
            },
            request_hash: format!("{:x}", Sha256::digest(body.as_bytes())),
            request_body: body,
            idempotency_key: message_id.to_string(),
            origin: base.into(),
            credential_fingerprint: hermes_wire::credential_fingerprint("recovery-fixture-only"),
            capabilities: facts(),
            state: "submitted".into(),
            submission_attempted: true,
            submitted_at: Some(timestamp),
            recovery_deadline: timestamp,
            recovery_allowed: true,
        }
    }

    #[tokio::test]
    async fn unknown_lookup_requires_exact_closed_original_receipt_and_never_submits() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        for bad in [
            None,
            Some("object"),
            Some("contract_version"),
            Some("store_id"),
            Some("idempotency_key"),
            Some("request_sha256"),
            Some("scope_fingerprint"),
            Some("profile"),
            Some("found"),
            Some("run_id"),
            Some("extra"),
            Some("http404"),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let intent = intent(&base);
            let caps = intent.capabilities["fleet_recovery"].clone();
            let mut found = json!({"object":"fleet.hermes.recovery.lookup","contract_version":1,
                "store_id":caps["store_id"],"idempotency_key":intent.idempotency_key,
                "request_sha256":intent.request_hash,"scope_fingerprint":caps["scope_fingerprint"],
                "profile":"default","found":true,"run_id":"run_original"});
            if let Some(field) = bad {
                found[field] = Value::Null;
            }
            let lookup_count = Arc::new(AtomicUsize::new(0));
            let native_count = Arc::new(AtomicUsize::new(0));
            let counted = lookup_count.clone();
            let forbidden = native_count.clone();
            let expected = intent.request_body.clone();
            let router = axum::Router::new()
                .route(
                    CAPABILITIES_PATH,
                    axum::routing::get(move || async move { axum::Json(caps) }),
                )
                .route(
                    LOOKUP_PATH,
                    axum::routing::post(
                        move |headers: axum::http::HeaderMap,
                              axum::Json(body): axum::Json<Value>| {
                            counted.fetch_add(1, Ordering::SeqCst);
                            assert_eq!(headers["authorization"], "Bearer recovery-fixture-only");
                            assert_eq!(headers["accept-encoding"], "identity");
                            assert_eq!(body["request_json"], expected);
                            async move {
                                (
                                    if bad == Some("http404") {
                                        StatusCode::NOT_FOUND
                                    } else {
                                        StatusCode::OK
                                    },
                                    axum::Json(found),
                                )
                            }
                        },
                    ),
                )
                .route(
                    "/v1/runs",
                    axum::routing::post(move || {
                        forbidden.fetch_add(1, Ordering::SeqCst);
                        async { StatusCode::ACCEPTED }
                    }),
                );
            let server = tokio::spawn(async move {
                axum::serve(listener, router).await.unwrap();
            });
            let client = reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap();
            let result = lookup(&client, &base, "recovery-fixture-only", &intent).await;
            assert_eq!(result.is_ok(), bad.is_none(), "{bad:?}");
            assert_eq!(lookup_count.load(Ordering::SeqCst), 1);
            assert_eq!(native_count.load(Ordering::SeqCst), 0);
            server.abort();
        }
    }

    #[tokio::test]
    async fn unknown_lookup_expiry_identity_and_epoch_denials_are_non_dispatch() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let mut intent = intent(&base);
        let mut current = intent.capabilities["fleet_recovery"].clone();
        current["store_id"] = json!(Uuid::new_v4().to_string());
        let reads = Arc::new(AtomicUsize::new(0));
        let writes = Arc::new(AtomicUsize::new(0));
        let reading = reads.clone();
        let writing = writes.clone();
        let router = axum::Router::new()
            .route(
                CAPABILITIES_PATH,
                axum::routing::get(move || {
                    reading.fetch_add(1, Ordering::SeqCst);
                    async move { axum::Json(current) }
                }),
            )
            .route(
                LOOKUP_PATH,
                axum::routing::post(move || {
                    writing.fetch_add(1, Ordering::SeqCst);
                    async { StatusCode::ACCEPTED }
                }),
            );
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        intent.recovery_allowed = false;
        assert!(
            lookup(&client, &base, "recovery-fixture-only", &intent)
                .await
                .is_err()
        );
        intent.recovery_allowed = true;
        assert!(
            lookup(&client, &base, "rotated-fixture-only", &intent)
                .await
                .is_err()
        );
        for field in ["origin", "body", "hash", "key", "credential", "state"] {
            let mut altered = self::intent(&base);
            match field {
                "origin" => altered.origin.push_str("/foreign"),
                "body" => altered.request_body.push(' '),
                "hash" => altered.request_hash = "0".repeat(64),
                "key" => altered.idempotency_key = Uuid::new_v4().to_string(),
                "credential" => altered.credential_fingerprint = "0".repeat(64),
                "state" => altered.state = "prepared".into(),
                _ => unreachable!(),
            }
            assert!(
                lookup(&client, &base, "recovery-fixture-only", &altered)
                    .await
                    .is_err(),
                "{field}"
            );
        }
        assert_eq!(reads.load(Ordering::SeqCst), 0);
        assert!(
            lookup(&client, &base, "recovery-fixture-only", &intent)
                .await
                .is_err()
        );
        assert_eq!(reads.load(Ordering::SeqCst), 1);
        assert_eq!(writes.load(Ordering::SeqCst), 0);
        server.abort();
    }
}
