use super::*;
use axum::{
    Router,
    body::Bytes,
    http::HeaderMap,
    response::IntoResponse,
    routing::{get, post},
};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const TOKEN: &str = "control-outcome-fixture-only";
const RUN: &str = "run_0123456789abcdef0123456789abcdef";

fn capabilities() -> Value {
    json!({"object":"fleet.hermes.controls.capabilities","contract_version":1,
        "store_id":"7cbbdbf8-c9a7-455b-b2c8-40dfc4c7d41b","scope_fingerprint":scope(TOKEN),
        "profile":"default","native_source_revision":SOURCE,"single_send":true,
        "non_dispatch_lookup":true,"lookup":{"method":"GET","path":LOOKUP},
        "operations":["steer","stop","approval"]})
}

fn context(operation: Operation) -> Context {
    let request_body = match operation {
        Operation::Stop => String::new(),
        Operation::Steer => "{\n  \"input\" : \"fixture guidance\"\n}".to_owned(),
        Operation::Approval => {
            r#"{"request_id":"request_fixture","choice":"deny","resolve_all":false}"#.to_owned()
        }
    };
    Context {
        origin: "http://127.0.0.1:23899".into(),
        credential_fingerprint: hermes_wire::credential_fingerprint(TOKEN),
        capabilities: serde_json::from_value(capabilities()).unwrap(),
        command_id: Uuid::new_v4(),
        run_id: RUN.into(),
        operation,
        request_sha256: digest(request_body.as_bytes()),
        request_body,
    }
}

fn found(context: &Context) -> Value {
    json!({"object":"fleet.hermes.controls.lookup","contract_version":1,
        "store_id":context.capabilities.store_id,"scope_fingerprint":scope(TOKEN),"profile":"default",
        "command_id":context.command_id.to_string(),"run_id":RUN,"operation":context.operation,
        "request_sha256":context.request_sha256,"state":"uncertain"})
}

fn acknowledged(context: &Context) -> Value {
    let mut payload = found(context);
    payload["state"] = json!("acknowledged");
    payload["ack"] = match context.operation {
        Operation::Steer => json!({"object":"hermes.run.steer","run_id":RUN,"accepted":true}),
        Operation::Stop => json!({"run_id":RUN,"status":"stopping"}),
        Operation::Approval => json!({"object":"hermes.run.approval_response","run_id":RUN,
            "request_id":"request_fixture","choice":"deny","resolved":1}),
    };
    payload
}

fn client() -> Client {
    Client::builder()
        .retry(reqwest::retry::never())
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .unwrap()
}

#[test]
fn capabilities_pin_version_source_default_scope_epoch_and_all_operations() {
    let good: Capabilities = serde_json::from_value(capabilities()).unwrap();
    good.validate(TOKEN).unwrap();
    for (path, value) in [
        ("/object", json!("hermes.run")),
        ("/contract_version", json!(2)),
        ("/store_id", json!(Uuid::nil().to_string())),
        ("/store_id", json!(good.store_id.to_uppercase())),
        ("/scope_fingerprint", json!(scope("rotated-fixture-only"))),
        ("/profile", json!("other")),
        ("/native_source_revision", json!("main")),
        ("/single_send", json!(false)),
        ("/non_dispatch_lookup", json!(false)),
        ("/lookup/method", json!("POST")),
        ("/lookup/path", json!("/v1/runs")),
        ("/operations", json!(["steer", "stop"])),
        ("/operations", json!(["steer", "stop", "stop"])),
        (
            "/operations",
            json!(["steer", "stop", "approval", "restart"]),
        ),
    ] {
        let mut bad = capabilities();
        *bad.pointer_mut(path).unwrap() = value;
        assert!(
            serde_json::from_value::<Capabilities>(bad)
                .unwrap()
                .validate(TOKEN)
                .is_err(),
            "{path}"
        );
    }
    assert!(good.validate("").is_err());
    assert!(good.validate("rotated-fixture-only").is_err());
}

#[test]
fn context_roundtrip_keeps_raw_body_and_rejects_identity_or_body_changes() {
    for op in [Operation::Steer, Operation::Stop, Operation::Approval] {
        let good = context(op);
        let bytes = serde_json::to_vec(&good).unwrap();
        let restored: Context = serde_json::from_slice(&bytes).unwrap();
        restored.validate(&good.origin, TOKEN).unwrap();
        assert_eq!(
            restored.request_body.as_bytes(),
            good.request_body.as_bytes()
        );
        assert_eq!(restored.request_sha256, good.request_sha256);
        assert!(!String::from_utf8(bytes).unwrap().contains(TOKEN));
        assert!(restored.validate("http://127.0.0.1:23900", TOKEN).is_err());
        assert!(
            restored
                .validate(&good.origin, "rotated-fixture-only")
                .is_err()
        );
        for (field, value) in [
            ("command_id", json!(Uuid::nil())),
            ("run_id", json!("run_foreign")),
            ("credential_fingerprint", json!("bad")),
            ("request_sha256", json!("0".repeat(64))),
            ("request_body", json!("mutated guidance")),
        ] {
            let mut bad = serde_json::to_value(&good).unwrap();
            bad[field] = value;
            assert!(
                serde_json::from_value::<Context>(bad)
                    .unwrap()
                    .validate(&good.origin, TOKEN)
                    .is_err()
            );
        }
    }
}

#[test]
fn persisted_request_rejects_empty_steer_bulk_approval_stop_json_and_invalid_ids() {
    for (op, raw) in [
        (Operation::Steer, r#"{"input":" "}"#),
        (Operation::Steer, r#"{"input":"\u001c\u001f"}"#),
        (Operation::Steer, r#"{"input":"one","input":"two"}"#),
        (Operation::Steer, r#"{"input":"one","extra":true}"#),
        (Operation::Stop, "{}"),
        (
            Operation::Approval,
            r#"{"request_id":"request_fixture","choice":"always","resolve_all":false}"#,
        ),
        (
            Operation::Approval,
            r#"{"request_id":"request_fixture","choice":"once","resolve_all":true}"#,
        ),
        (
            Operation::Approval,
            r#"{"request_id":" request_fixture","choice":"once","resolve_all":false}"#,
        ),
        (
            Operation::Approval,
            r#"{"request_id":"request\nfixture","choice":"once","resolve_all":false}"#,
        ),
    ] {
        let mut bad = context(op);
        bad.request_body = raw.into();
        bad.request_sha256 = digest(raw.as_bytes());
        assert!(bad.validate(&bad.origin, TOKEN).is_err());
    }
    for id in [
        "",
        "run_a",
        "run_0123456789abcdef0123456789ABCDEF",
        "../run",
    ] {
        assert!(!native_run(id));
    }
    for origin in [
        "http://fixture/path",
        "http://fixture/",
        "http://u:p@fixture",
        "http://fixture?secret=x",
        "http://fixture#fragment",
        "file:///fixture",
        "http://fixture:80",
    ] {
        assert!(validate_origin(origin).is_err());
    }
}

#[test]
fn lookup_requires_every_original_identity_field() {
    let good = context(Operation::Steer);
    for (field, value) in [
        ("object", json!("hermes.run")),
        ("contract_version", json!(2)),
        ("profile", json!("foreign")),
        ("scope_fingerprint", json!("0".repeat(64))),
        ("store_id", json!(Uuid::new_v4())),
        ("command_id", json!(Uuid::new_v4())),
        ("run_id", json!("run_ffffffffffffffffffffffffffffffff")),
        ("operation", json!("stop")),
        ("request_sha256", json!("0".repeat(64))),
    ] {
        let mut bad = acknowledged(&good);
        bad[field] = value;
        assert!(
            outcome(&serde_json::to_vec(&bad).unwrap(), &good).is_err(),
            "{field}"
        );
    }
}

#[test]
fn exact_acknowledgements_are_not_terminal_stage_or_safe_stop_evidence() {
    for (op, expected) in [
        (Operation::Steer, Acknowledgement::Steered),
        (Operation::Stop, Acknowledgement::Stopping),
        (Operation::Approval, Acknowledgement::ApprovalResolved),
    ] {
        let good = context(op);
        assert_eq!(
            outcome(&serde_json::to_vec(&acknowledged(&good)).unwrap(), &good).unwrap(),
            Outcome::Acknowledged(expected)
        );
        assert_eq!(
            outcome(&serde_json::to_vec(&found(&good)).unwrap(), &good).unwrap(),
            Outcome::Uncertain
        );
        let mut uncertain_with_ack = acknowledged(&good);
        uncertain_with_ack["state"] = json!("uncertain");
        assert!(outcome(&serde_json::to_vec(&uncertain_with_ack).unwrap(), &good).is_err());
        for value in [
            Value::Null,
            json!({"run_id":RUN,"status":"completed"}),
            json!({"object":"hermes.run","run_id":RUN,"status":"completed","completed":true}),
            json!({"object":"hermes.run.steer","run_id":RUN,"accepted":"true"}),
            json!({"object":"hermes.run.approval_response","run_id":RUN,
                "request_id":"request_fixture","choice":"deny","resolved":true}),
        ] {
            let mut bad = acknowledged(&good);
            bad["ack"] = value;
            assert!(outcome(&serde_json::to_vec(&bad).unwrap(), &good).is_err());
        }
    }
}

#[test]
fn approval_ack_identifies_exact_request_choice_and_single_resolution() {
    let good = context(Operation::Approval);
    for (field, value) in [
        ("object", json!("hermes.run")),
        ("run_id", json!("run_ffffffffffffffffffffffffffffffff")),
        ("request_id", json!("foreign")),
        ("choice", json!("once")),
        ("resolved", json!(0)),
        ("resolved", json!(2)),
        ("extra", json!(true)),
    ] {
        let mut bad = acknowledged(&good);
        bad["ack"][field] = value;
        assert!(
            outcome(&serde_json::to_vec(&bad).unwrap(), &good).is_err(),
            "{field}"
        );
    }
}

#[test]
fn duplicate_unknown_or_missing_json_fields_do_not_become_a_witness() {
    let good = context(Operation::Steer);
    let raw = acknowledged(&good).to_string();
    for bad in [
        raw.replacen("\"state\":", "\"state\":\"uncertain\",\"state\":", 1),
        raw.replacen("\"accepted\":", "\"accepted\":false,\"accepted\":", 1),
        raw.replacen("\"object\":", "\"extra\":true,\"object\":", 1),
        raw.replacen("\"request_sha256\":", "\"missing_field\":", 1),
    ] {
        assert!(outcome(bad.as_bytes(), &good).is_err(), "{bad}");
    }
    let caps = capabilities().to_string();
    assert!(
        serde_json::from_str::<Capabilities>(&caps.replacen(
            "\"store_id\":",
            "\"store_id\":\"foreign\",\"store_id\":",
            1
        ))
        .is_err()
    );
}

async fn server(router: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    (origin, task)
}

#[tokio::test]
async fn send_keeps_saved_bytes_original_uuid_epoch_and_empty_stop_body() {
    for op in [Operation::Steer, Operation::Stop] {
        let mut original = context(op);
        let expected_body = original.request_body.clone();
        let expected_id = original.command_id.to_string();
        let expected_epoch = original.capabilities.store_id.clone();
        let ack = acknowledged(&original)["ack"].clone();
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let router = Router::new().route(
            &format!("/v1/runs/{RUN}/{}", op.as_str()),
            post(move |headers: HeaderMap, bytes: Bytes| {
                assert_eq!(headers["authorization"], format!("Bearer {TOKEN}"));
                assert_eq!(headers["accept-encoding"], "identity");
                assert_eq!(headers["idempotency-key"], expected_id);
                assert_eq!(headers["x-fleet-control-store-id"], expected_epoch);
                assert_eq!(bytes.as_ref(), expected_body.as_bytes());
                observed.fetch_add(1, Ordering::SeqCst);
                let ack = ack.clone();
                async { axum::Json(ack) }
            }),
        );
        let (origin, task) = server(router).await;
        original.origin = origin.clone();
        let restored: Context =
            serde_json::from_value(serde_json::to_value(&original).unwrap()).unwrap();
        assert!(
            send(&client(), &restored, &origin, "rotated-fixture-only")
                .await
                .is_err()
        );
        assert!(
            send(&client(), &restored, "http://127.0.0.1:23900", TOKEN)
                .await
                .is_err()
        );
        let ack = send(&client(), &restored, &origin, TOKEN).await.unwrap();
        assert_eq!(
            ack,
            if op == Operation::Steer {
                Acknowledgement::Steered
            } else {
                Acknowledgement::Stopping
            }
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        task.abort();
        let _ = task.await;
    }
    let approval = context(Operation::Approval);
    assert!(
        send(&client(), &approval, &approval.origin, TOKEN)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn send_unknown_negative_duplicate_or_terminal_ack_never_retries() {
    for (code, mime, encoding, body) in [
        (
            StatusCode::CONFLICT,
            "application/json",
            "identity",
            r#"{"run_id":"run_0123456789abcdef0123456789abcdef","status":"stopping"}"#.to_owned(),
        ),
        (StatusCode::OK, "text/plain", "identity", "{}".to_owned()),
        (StatusCode::OK, "application/json", "gzip", "{}".to_owned()),
        (
            StatusCode::OK,
            "application/json",
            "identity",
            format!(r#"{{"run_id":"{RUN}","status":"stopping","status":"stopping"}}"#),
        ),
        (
            StatusCode::OK,
            "application/json",
            "identity",
            format!(r#"{{"run_id":"{RUN}","status":"stopping","unexpected":true}}"#),
        ),
        (
            StatusCode::OK,
            "application/json",
            "identity",
            format!(
                r#"{{"object":"hermes.run","run_id":"{RUN}","status":"completed","completed":true,"partial":false,"interrupted":false}}"#
            ),
        ),
        (
            StatusCode::OK,
            "application/json",
            "identity",
            r#"{"run_id":"run_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","status":"stopping"}"#.to_string(),
        ),
        (
            StatusCode::OK,
            "application/json",
            "identity",
            "x".repeat(MAX_BODY + 1),
        ),
    ] {
        let mut original = context(Operation::Stop);
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let router = Router::new().route(
            &format!("/v1/runs/{RUN}/stop"),
            post(move || {
                observed.fetch_add(1, Ordering::SeqCst);
                let body = body.clone();
                async move {
                    (
                        code,
                        [
                            (header::CONTENT_TYPE, mime),
                            (header::CONTENT_ENCODING, encoding),
                        ],
                        body,
                    )
                }
            }),
        );
        let (origin, task) = server(router).await;
        original.origin = origin.clone();
        assert!(send(&client(), &original, &origin, TOKEN).await.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        task.abort();
        let _ = task.await;
    }
}

#[tokio::test]
async fn authenticated_lookup_is_get_only_and_uses_original_epoch_body_hash_and_command() {
    let mut original = context(Operation::Steer);
    let expected = found(&original);
    let reply = acknowledged(&original).to_string();
    let reads = Arc::new(AtomicUsize::new(0));
    let observed = reads.clone();
    let router = Router::new().route(
        LOOKUP,
        get(
            move |headers: HeaderMap,
                  axum::extract::Query(query): axum::extract::Query<
                std::collections::HashMap<String, String>,
            >,
                  body: Bytes| {
                observed.fetch_add(1, Ordering::SeqCst);
                assert_eq!(headers["authorization"], format!("Bearer {TOKEN}"));
                assert_eq!(headers["accept-encoding"], "identity");
                assert!(body.is_empty());
                assert_eq!(query.len(), 5);
                for key in [
                    "store_id",
                    "command_id",
                    "run_id",
                    "operation",
                    "request_sha256",
                ] {
                    assert_eq!(query[key], expected[key].as_str().unwrap());
                }
                let reply = reply.clone();
                async move { ([(header::CONTENT_TYPE, "application/json")], reply) }
            },
        ),
    );
    let (origin, task) = server(router).await;
    original.origin = origin.clone();
    let restored: Context =
        serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();
    assert_eq!(
        lookup(&client(), &restored, &origin, TOKEN).await.unwrap(),
        Outcome::Acknowledged(Acknowledgement::Steered)
    );
    assert_eq!(
        lookup(&client(), &restored, &origin, TOKEN).await.unwrap(),
        Outcome::Acknowledged(Acknowledgement::Steered)
    );
    assert!(
        lookup(&client(), &restored, &origin, "rotated-fixture-only")
            .await
            .is_err()
    );
    assert!(
        lookup(&client(), &restored, "http://127.0.0.1:23900", TOKEN)
            .await
            .is_err()
    );
    assert_eq!(reads.load(Ordering::SeqCst), 2);
    task.abort();
    let _ = task.await;
}

#[tokio::test]
async fn prepare_is_get_only_serializes_once_and_bounds_encoded_request_before_http() {
    let reads = Arc::new(AtomicUsize::new(0));
    let observed = reads.clone();
    let router = Router::new().route(
        CAPABILITIES,
        get(move |headers: HeaderMap, body: Bytes| {
            observed.fetch_add(1, Ordering::SeqCst);
            assert_eq!(headers["authorization"], format!("Bearer {TOKEN}"));
            assert!(body.is_empty());
            async { axum::Json(capabilities()) }
        }),
    );
    let (origin, task) = server(router).await;
    for request in [
        Request::Steer("actual fixture"),
        Request::Stop,
        Request::Approval {
            request_id: "request_fixture",
            choice: ApprovalChoice::Once,
        },
    ] {
        let prepared = prepare(&client(), &origin, TOKEN, Uuid::new_v4(), RUN, request)
            .await
            .unwrap();
        prepared.validate(&origin, TOKEN).unwrap();
        assert_eq!(
            prepared.request_sha256,
            digest(prepared.request_body.as_bytes())
        );
        if prepared.operation == Operation::Stop {
            assert!(prepared.request_body.is_empty());
        }
    }
    for request in [
        Request::Steer(" "),
        Request::Steer("\u{1c}\u{1f}"),
        Request::Steer(&"\"\n".repeat(MAX_BODY / 3)),
        Request::Approval {
            request_id: " invalid",
            choice: ApprovalChoice::Once,
        },
    ] {
        assert!(
            prepare(&client(), &origin, TOKEN, Uuid::new_v4(), RUN, request)
                .await
                .is_err()
        );
    }
    assert!(
        prepare(&client(), &origin, TOKEN, Uuid::nil(), RUN, Request::Stop)
            .await
            .is_err()
    );
    assert_eq!(reads.load(Ordering::SeqCst), 3);
    task.abort();
    let _ = task.await;
}

#[tokio::test]
async fn missing_error_redirect_encoding_mime_and_oversize_never_authorize_recovery() {
    for (code, mime, encoding, body) in [
        (
            StatusCode::NOT_FOUND,
            "application/json",
            "identity",
            "fixture-private".into(),
        ),
        (
            StatusCode::CONFLICT,
            "application/json",
            "identity",
            "fixture-private".into(),
        ),
        (
            StatusCode::SERVICE_UNAVAILABLE,
            "application/json",
            "identity",
            "fixture-private".into(),
        ),
        (
            StatusCode::FOUND,
            "application/json",
            "identity",
            "fixture-private".into(),
        ),
        (
            StatusCode::OK,
            "text/plain",
            "identity",
            "fixture-private".into(),
        ),
        (
            StatusCode::OK,
            "application/json",
            "gzip",
            "fixture-private".into(),
        ),
        (
            StatusCode::OK,
            "application/json",
            "identity",
            "x".repeat(MAX_BODY + 1),
        ),
    ] {
        let mut original = context(Operation::Stop);
        let reads = Arc::new(AtomicUsize::new(0));
        let observed = reads.clone();
        let router = Router::new().route(
            LOOKUP,
            get(move || {
                observed.fetch_add(1, Ordering::SeqCst);
                let body = body.clone();
                async move {
                    (
                        code,
                        [
                            (header::CONTENT_TYPE, mime),
                            (header::CONTENT_ENCODING, encoding),
                        ],
                        body,
                    )
                }
            }),
        );
        let (origin, task) = server(router).await;
        original.origin = origin.clone();
        let error = lookup(&client(), &original, &origin, TOKEN)
            .await
            .unwrap_err();
        assert!(!error.to_string().contains("fixture-private"));
        assert!(!error.to_string().contains(TOKEN));
        assert_eq!(reads.load(Ordering::SeqCst), 1);
        task.abort();
        let _ = task.await;
    }
}

#[tokio::test]
async fn truncated_or_chunked_overbound_lookup_remains_unverified() {
    for truncated in [true, false] {
        let mut original = context(Operation::Stop);
        let router = Router::new().route(
            LOOKUP,
            get(move || async move {
                let chunks = if truncated {
                    vec![Ok::<_, std::io::Error>("{\"ack\":".to_owned())]
                } else {
                    vec![Ok("x".repeat(MAX_BODY)), Ok("more".into())]
                };
                let mut response =
                    axum::body::Body::from_stream(futures_util::stream::iter(chunks))
                        .into_response();
                response
                    .headers_mut()
                    .insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
                response
            }),
        );
        let (origin, task) = server(router).await;
        original.origin = origin.clone();
        assert!(lookup(&client(), &original, &origin, TOKEN).await.is_err());
        task.abort();
        let _ = task.await;
    }
}
