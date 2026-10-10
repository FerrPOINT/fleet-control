use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(target_os = "linux")]
#[path = "pm_recovery_pg_tests.rs"]
mod pg;

fn capabilities() -> Value {
    json!({"object":"hermes.api_server.capabilities","platform":"hermes-agent",
        "auth":{"type":"bearer","required":true},
        "runtime":{"mode":"server_agent","tool_execution":"server","split_runtime":false},
        "features":{"runs_idempotency":{"supported":true,"durable":true,"retention_seconds":86400},
            "run_submission":true,"run_status":true,"run_events_sse":true,"run_stop":true},
        "endpoints":{"runs":{"method":"POST","path":"/v1/runs"},
            "run_status":{"method":"GET","path":"/v1/runs/{run_id}"},
            "run_events":{"method":"GET","path":"/v1/runs/{run_id}/events"},
            "run_stop":{"method":"POST","path":"/v1/runs/{run_id}/stop"}}})
}

fn fixture(base: &str) -> (PmDispatchIntent, app::container_runtime::ContainerLaunch) {
    let key = Uuid::new_v4();
    let generation = Uuid::new_v4();
    let now = chrono::Utc::now();
    let launch: app::container_runtime::ContainerLaunch = serde_json::from_value(json!({
        "prepared":{"agent_id":Uuid::new_v4(),"paths":{"runtime":"/agents/a/runtime",
            "config":"/agents/a/config","workspace":"/agents/a/workspace","logs":"/agents/a/logs"},
            "api_port":24003,"configuration_revision":1,"configuration_sha256":"c".repeat(64),
            "container":{"registration":{"contract_version":2,"operation_id":Uuid::new_v4(),
                "container_id":"a".repeat(64),"resource_id":Uuid::new_v4(),"generation":generation,
                "engine":{"ID":"engine","KernelVersion":"kernel","ServerVersion":"29"},
                "policy_sha256":"b".repeat(64),"inventory_sha256":"c".repeat(64),
                "running_inventory_sha256":"d".repeat(64),"compose_sha256":"e".repeat(64)},
                "policy":{},"compose":"/private/compose.json","journal":"/private/start.sqlite",
                "stop_journal":"/private/stop.sqlite","source_sha256":["f".repeat(64),"f".repeat(64),"f".repeat(64)],
                "context":"fixture"}},
        "controller_id":Uuid::new_v4(),"state":"running",
        "snapshot":{"init_pid":42,"started_at":"2026-10-10T00:00:00Z"},
        "origin":base,"stop_id":Uuid::new_v4()
    })).unwrap();
    let body = r#"{"input":"original fixture","session_id":"fleet:fixture"}"#.to_owned();
    let proof = Proof {
        version: 1,
        captured_at: now,
        deadline: now + chrono::Duration::seconds(HORIZON_SECONDS),
        key,
        request_sha256: digest(&body),
        capabilities: hermes_wire::dispatch_capabilities(&capabilities()).unwrap(),
        generation,
        launch_sha256: container_control::launch_hash(&launch).unwrap(),
        snapshot: launch.snapshot.clone().unwrap(),
    };
    let intent = PmDispatchIntent {
        session_run_id: key,
        origin: launch.origin.clone().unwrap(),
        credential_fingerprint: hermes_wire::credential_fingerprint("fixture-bearer"),
        request_body: body,
        workflow_assignment: json!({"original":"assignment"}),
        workflow_origin: "http://workflow.fixture".into(),
        workflow_credential_fingerprint: "f".repeat(64),
        runtime_context: json!({"fleet_container_generation":generation, (PROOF):proof}),
        submitted: true,
        hermes_run_ref: None,
    };
    (intent, launch)
}

#[test]
fn replay_proof_reload_preserves_original_key_body_context_and_horizon() {
    let (intent, launch) = fixture("http://127.0.0.1:24003");
    let reloaded: PmDispatchIntent =
        serde_json::from_slice(&serde_json::to_vec(&intent).unwrap()).unwrap();
    assert!(reloaded == intent);
    validate(&reloaded, &launch, &capabilities(), chrono::Utc::now()).unwrap();
    assert_eq!(
        context(&reloaded.runtime_context).unwrap(),
        json!({"fleet_container_generation":launch.prepared.container.registration.generation})
    );
    assert_eq!(
        proof(&reloaded).unwrap().deadline,
        proof(&intent).unwrap().deadline
    );
}

#[test]
fn replay_proof_rejects_legacy_request_capability_context_and_expiry_changes() {
    let (intent, launch) = fixture("http://127.0.0.1:24003");
    for field in [
        "missing",
        "noncontainer",
        "body",
        "key",
        "origin",
        "generation",
        "pid",
        "start",
        "launch",
        "configuration",
        "stopped",
        "durable",
        "retention",
        "endpoint",
        "expired",
        "budget",
        "future",
        "extended",
        "extra",
    ] {
        let mut intent = intent.clone();
        let mut launch = launch.clone();
        let mut caps = capabilities();
        let mut now = chrono::Utc::now();
        match field {
            "missing" => {
                intent
                    .runtime_context
                    .as_object_mut()
                    .unwrap()
                    .remove(PROOF);
            }
            "noncontainer" => {
                intent
                    .runtime_context
                    .as_object_mut()
                    .unwrap()
                    .remove("fleet_container_generation");
            }
            "body" => intent.request_body.push(' '),
            "key" => intent.session_run_id = Uuid::new_v4(),
            "origin" => intent.origin.push_str("/foreign"),
            "generation" => launch.prepared.container.registration.generation = Uuid::new_v4(),
            "pid" => launch.snapshot.as_mut().unwrap()["init_pid"] = json!(43),
            "start" => {
                launch.snapshot.as_mut().unwrap()["started_at"] = json!("2026-10-10T01:00:00Z")
            }
            "launch" => launch.controller_id = Uuid::new_v4(),
            "configuration" => launch.prepared.configuration_revision = Some(2),
            "stopped" => launch.state = "stopping".into(),
            "durable" => caps["features"]["runs_idempotency"]["durable"] = json!(false),
            "retention" => caps["features"]["runs_idempotency"]["retention_seconds"] = json!(86401),
            "endpoint" => caps["endpoints"]["runs"]["path"] = json!("/foreign"),
            "expired" => now = proof(&intent).unwrap().deadline,
            "budget" => now = proof(&intent).unwrap().deadline - chrono::Duration::seconds(30),
            "future" => now = proof(&intent).unwrap().captured_at - chrono::Duration::seconds(1),
            "extended" => {
                intent.runtime_context[PROOF]["deadline"] = json!(now + chrono::Duration::days(2))
            }
            "extra" => intent.runtime_context[PROOF]["extra"] = json!(true),
            _ => unreachable!(),
        }
        assert!(validate(&intent, &launch, &caps, now).is_err(), "{field}");
    }
}

// Controlled HTTP peer implements the pinned native scope/key/fingerprint
// reservation contract. This is not actual Hermes/model or PostgreSQL evidence.
async fn peer() -> (
    String,
    Arc<AtomicUsize>,
    Arc<AtomicUsize>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let effects = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let ledger = Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::<
        String,
        (Value, String),
    >::new()));
    let counted = effects.clone();
    let requested = calls.clone();
    let router = axum::Router::new().route(
        "/v1/runs",
        axum::routing::post(move |headers: axum::http::HeaderMap, body: String| {
            let ledger = ledger.clone();
            let counted = counted.clone();
            let requested = requested.clone();
            async move {
                use axum::response::IntoResponse;
                assert!(headers["authorization"] == "Bearer fixture-bearer");
                let key = headers["idempotency-key"].to_str().unwrap().to_owned();
                let call = requested.fetch_add(1, Ordering::SeqCst);
                let mut saved = ledger.lock().await;
                let body: Value = serde_json::from_str(&body).unwrap();
                let replay = saved.contains_key(&key);
                let run = if let Some((old_body, run)) = saved.get(&key) {
                    if *old_body != body {
                        return reqwest::StatusCode::CONFLICT.into_response();
                    }
                    run.clone()
                } else {
                    let effect = counted.fetch_add(1, Ordering::SeqCst);
                    let run = if effect == 0 {
                        "run_original".into()
                    } else {
                        format!("run_{effect}")
                    };
                    saved.insert(key, (body, run.clone()));
                    run
                };
                if call == 0 {
                    // Native effect happened, but Fleet receives no usable ACK.
                    return reqwest::StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
                (
                    reqwest::StatusCode::ACCEPTED,
                    axum::Json(json!({
                        "run_id":run, "status":"started", "replayed":replay
                    })),
                )
                    .into_response()
            }
        }),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (base, effects, calls, server)
}

#[tokio::test]
async fn original_key_lost_ack_then_reload_replay_returns_same_native_run() {
    let (base, effects, calls, server) = peer().await;
    let (intent, launch) = fixture(&base);
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    assert!(
        post(&client, &base, "fixture-bearer", &intent)
            .await
            .is_err()
    );
    let reloaded: PmDispatchIntent =
        serde_json::from_str(&serde_json::to_string(&intent).unwrap()).unwrap();
    validate(&reloaded, &launch, &capabilities(), chrono::Utc::now()).unwrap();
    assert_eq!(
        post(&client, &base, "fixture-bearer", &reloaded)
            .await
            .unwrap(),
        "run_original"
    );
    assert_eq!(
        post(&client, &base, "fixture-bearer", &reloaded)
            .await
            .unwrap(),
        "run_original"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    server.abort();
}

#[tokio::test]
async fn concurrent_original_and_same_key_replay_have_one_native_effect() {
    let (base, effects, calls, server) = peer().await;
    let (intent, launch) = fixture(&base);
    validate(&intent, &launch, &capabilities(), chrono::Utc::now()).unwrap();
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let (first, second) = tokio::join!(
        post(&client, &base, "fixture-bearer", &intent),
        post(&client, &base, "fixture-bearer", &intent)
    );
    assert_ne!(first.is_ok(), second.is_ok());
    assert_eq!(first.or(second).unwrap(), "run_original");
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    server.abort();
}

#[tokio::test]
async fn original_key_rejects_changed_payload_without_another_native_effect() {
    let (base, effects, calls, server) = peer().await;
    let (intent, launch) = fixture(&base);
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    assert!(
        post(&client, &base, "fixture-bearer", &intent)
            .await
            .is_err()
    );
    let mut changed = intent.clone();
    changed.request_body = r#"{"input":"changed","session_id":"fleet:fixture"}"#.into();
    assert!(validate(&changed, &launch, &capabilities(), chrono::Utc::now()).is_err());
    // Also characterize the controlled native peer's fingerprint conflict guard.
    assert!(
        post(&client, &base, "fixture-bearer", &changed)
            .await
            .is_err()
    );
    assert_eq!(
        post(&client, &base, "fixture-bearer", &intent)
            .await
            .unwrap(),
        "run_original"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(effects.load(Ordering::SeqCst), 1);
    server.abort();
}
