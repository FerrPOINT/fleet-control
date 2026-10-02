use super::*;
use domain::CreatePmDraftRequest;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn fixture() -> (AppConfig, PmDraftOperation, Value) {
    let mut config = AppConfig::default();
    config.pm.namespace_read_pat = "sdlc_pat_test_only_namespace_read_credential".into();
    config.pm.namespace_authority_issuer = "http://authority.example.test".into();
    config.pm.namespace_provisioner_subject = "22222222-2222-4222-8222-222222222222".into();
    let operation = PmDraftOperation {
        id: Uuid::new_v4(),
        owner_user_id: Uuid::new_v4(),
        owner_subject: Uuid::new_v4().to_string(),
        tracker_instance_id: "tracker-test".into(),
        project_id: Uuid::new_v4(),
        request: CreatePmDraftRequest {
            agent_id: Uuid::new_v4(),
            title: "PM Draft".into(),
            description: String::new(),
            idempotency_key: "test-only".into(),
        },
        draft: None,
        input: None,
        reservation: None,
        session_id: None,
    };
    let value = json!({"ok":true,"result":{
        "contract_version":1,"ownership_ref":"11111111-1111-4111-8111-111111111111",
        "namespace_id":7,"tracker_instance_ref":operation.tracker_instance_id,
        "tracker_project_ref":operation.project_id.to_string(),
        "authority_issuer":config.pm.namespace_authority_issuer,
        "provisioner_subject":config.pm.namespace_provisioner_subject,
        "created_at":"2026-10-02T00:00:00Z"}});
    (config, operation, value)
}

#[test]
fn namespace_and_origin_are_exact_not_normalized() {
    for value in [
        "http://authority.example.test:80",
        "https://authority.example.test:443",
    ] {
        issuer(value).unwrap();
    }
    for value in [
        "http://authority.example.test/",
        "http://authority.example.test/../",
    ] {
        assert!(issuer(value).is_err());
    }
    assert_eq!(namespace(Some("7")).unwrap(), 7);
    for value in [
        None,
        Some(""),
        Some("07"),
        Some("+7"),
        Some(" 7"),
        Some("7 "),
        Some("0"),
        Some("-7"),
        Some("7/foreign"),
        Some("9223372036854775808"),
    ] {
        assert!(namespace(value).is_err(), "{value:?}");
    }
    assert!(origin("http://workflow.example.test:8811").is_ok());
    for value in [
        "http://user:secret@workflow.test",
        "http://workflow.test/api",
        "file:///tmp/workflow",
        " http://workflow.test",
        "http://workflow.test?q=1",
        "http://workflow.test#fragment",
        "http://workflow.test\\foreign",
        "http://workflow.test/../",
        "http://workflow.test/\t",
    ] {
        assert!(origin(value).is_err(), "{value}");
    }
}

#[test]
fn strict_identity_and_authority_readback_does_not_accept_metadata_drift() {
    let (config, operation, value) = fixture();
    validate(value.clone(), &config, &operation, 7).unwrap();
    for spelling in [
        "http://authority.example.test:80",
        "https://authority.example.test:443",
    ] {
        let mut config = config.clone();
        config.pm.namespace_authority_issuer = spelling.into();
        let mut value = value.clone();
        value["result"]["authority_issuer"] = json!(spelling);
        issuer(spelling).unwrap();
        validate(value, &config, &operation, 7).unwrap();
    }
    for (pointer, replacement) in [
        ("/ok", json!(false)),
        ("/result/contract_version", json!(2)),
        ("/result/contract_version", json!(1.0)),
        ("/result/namespace_id", json!("7")),
        ("/result/namespace_id", json!(8)),
        ("/result/authority_issuer", json!("http://foreign.test")),
        (
            "/result/provisioner_subject",
            json!(Uuid::new_v4().to_string()),
        ),
        ("/result/ownership_ref", json!(Uuid::nil().to_string())),
        (
            "/result/ownership_ref",
            json!("AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA"),
        ),
        ("/result/created_at", json!("not-a-timestamp")),
    ] {
        let mut bad = value.clone();
        *bad.pointer_mut(pointer).unwrap() = replacement;
        assert!(validate(bad, &config, &operation, 7).is_err(), "{pointer}");
    }
    for field in value["result"].as_object().unwrap().keys() {
        let mut bad = value.clone();
        bad["result"].as_object_mut().unwrap().remove(field);
        assert!(
            validate(bad, &config, &operation, 7).is_err(),
            "missing {field}"
        );
    }
    for pointer in [
        "/result/tracker_instance_ref",
        "/result/tracker_project_ref",
    ] {
        let mut bad = value.clone();
        *bad.pointer_mut(pointer).unwrap() = json!(if pointer.ends_with("project_ref") {
            Uuid::new_v4().to_string()
        } else {
            "another-instance".into()
        });
        assert!(matches!(
            validate(bad, &config, &operation, 7),
            Err(AppError::Conflict(_))
        ));
    }
    let mut bad = value;
    bad["result"]["dispatch_allowed"] = json!(true);
    assert!(validate(bad, &config, &operation, 7).is_err());
}

#[tokio::test]
async fn fresh_bounded_read_uses_machine_identity_and_rejects_revocation_redirects_and_bad_bodies()
{
    let (mut config, operation, value) = fixture();
    let mode = Arc::new(AtomicUsize::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let state = (mode.clone(), calls.clone(), value);
    let router = axum::Router::new().fallback(
        move |uri: axum::http::Uri, headers: axum::http::HeaderMap| {
            let (mode, calls, value) = state.clone();
            async move {
                assert_eq!(uri.to_string(), "/api/pm/namespace-ownership/7");
                assert_eq!(
                    headers[header::AUTHORIZATION],
                    "Bearer sdlc_pat_test_only_namespace_read_credential"
                );
                assert_eq!(headers[header::CACHE_CONTROL], "no-cache, no-store");
                assert_eq!(headers[header::ACCEPT_ENCODING], "identity");
                calls.fetch_add(1, Ordering::SeqCst);
                let mut headers = axum::http::HeaderMap::new();
                let (status, body) = match mode.load(Ordering::SeqCst) {
                    0 => (StatusCode::OK, value.to_string()),
                    1 => (StatusCode::FORBIDDEN, "private rejection detail".into()),
                    2 => {
                        headers.insert(
                            header::LOCATION,
                            "http://127.0.0.1:1/credential-leak".parse().unwrap(),
                        );
                        (StatusCode::TEMPORARY_REDIRECT, String::new())
                    }
                    3 => (StatusCode::OK, "x".repeat(MAX_BODY + 1)),
                    4 => (StatusCode::OK, "invalid-json".into()),
                    5 => {
                        headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
                        (StatusCode::OK, value.to_string())
                    }
                    7 => {
                        let mut value = value;
                        value["result"]["tracker_instance_ref"] = json!("я".repeat(64));
                        (StatusCode::OK, value.to_string())
                    }
                    _ => (StatusCode::NOT_FOUND, "private rejection detail".into()),
                };
                (status, headers, body)
            }
        },
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    config.fleet.project_workflow_url = Some(format!("http://{}", listener.local_addr().unwrap()));
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    read(&config, &operation, 7).await.unwrap();
    read(&config, &operation, 7).await.unwrap();
    for current in 1..=6 {
        mode.store(current, Ordering::SeqCst);
        let error = read(&config, &operation, 7).await.unwrap_err();
        assert!(matches!(error, AppError::Unavailable(_)));
        assert!(!error.to_string().contains("private rejection detail"));
        assert!(!error.to_string().contains(&config.pm.namespace_read_pat));
    }
    assert_eq!(calls.load(Ordering::SeqCst), 8);
    mode.store(7, Ordering::SeqCst);
    let mut multibyte = operation.clone();
    multibyte.tracker_instance_id = "я".repeat(64);
    read(&config, &multibyte, 7).await.unwrap();
    multibyte.tracker_instance_id.push('x');
    assert!(read(&config, &multibyte, 7).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 9);
    config.fleet.project_workflow_catalog_token = Some(config.pm.namespace_read_pat.clone());
    assert!(read(&config, &operation, 7).await.is_err());
    config.fleet.project_workflow_catalog_token = None;
    config.pm.readback_token = config.pm.namespace_read_pat.clone();
    assert!(read(&config, &operation, 7).await.is_err());
    config.pm.readback_token.clear();
    config.auth.jwt_secret = config.pm.namespace_read_pat.clone();
    assert!(read(&config, &operation, 7).await.is_err());
    config.auth.jwt_secret.clear();
    config.fleet.runtime_token_secret = config.pm.namespace_read_pat.clone();
    assert!(read(&config, &operation, 7).await.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 9);
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn chunked_body_limit_and_stalled_body_deadline_do_not_depend_on_content_length() {
    use axum::{body::Body, response::IntoResponse};
    use futures_util::{StreamExt, stream};
    let (mut config, operation, _) = fixture();
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let router = axum::Router::new().fallback(move || {
        let count = count.clone();
        async move {
            let response = if count.fetch_add(1, Ordering::SeqCst) == 0 {
                let chunks = stream::iter([
                    Ok::<_, std::io::Error>(vec![b'x'; MAX_BODY / 2]),
                    Ok(vec![b'x'; MAX_BODY / 2 + 1]),
                ]);
                Body::from_stream(chunks).into_response()
            } else {
                let stalled = stream::once(async { Ok::<_, std::io::Error>(vec![b'{']) })
                    .chain(stream::pending());
                Body::from_stream(stalled).into_response()
            };
            assert!(response.headers().get(header::CONTENT_LENGTH).is_none());
            response
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    config.fleet.project_workflow_url = Some(format!("http://{}", listener.local_addr().unwrap()));
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    assert!(read(&config, &operation, 7).await.is_err());
    let result = tokio::time::timeout(Duration::from_secs(7), read(&config, &operation, 7)).await;
    assert!(matches!(result, Ok(Err(AppError::Unavailable(_)))));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    server.abort();
    let _ = server.await;
}
