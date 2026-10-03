use super::*;
use axum::{Router, body::Body, http::Response, routing::get};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};

const SUBJECT: &str = "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee";
const TOKEN: &str = "sdlc_pat_test-only-configuration-reader-credential";

#[test]
fn sdlc_configuration_openapi_operation_ids_are_unique() {
    let document: serde_json::Value = serde_json::from_str(&crate::openapi_json()).unwrap();
    assert_eq!(
        document["paths"]["/internal/runtime/v1/agents/{agent_id}/configuration"]["get"]["operationId"],
        "read_sdlc_configuration"
    );
    let mut ids = BTreeSet::new();
    for path in document["paths"].as_object().unwrap().values() {
        for operation in path.as_object().unwrap().values() {
            if let Some(id) = operation
                .get("operationId")
                .and_then(serde_json::Value::as_str)
            {
                assert!(ids.insert(id), "duplicate operation ID: {id}");
            }
        }
    }
}

fn headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        format!("Bearer {TOKEN}").parse().unwrap(),
    );
    headers
}

fn principal() -> serde_json::Value {
    json!({"sub":SUBJECT,"email":"machine@example.test", "scopes":[
        "fleet-control:read"
    ]})
}

async fn service(
    status: StatusCode,
    body: String,
    encoding: Option<&str>,
) -> (SdlcConfig, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let encoding = encoding.map(str::to_owned);
    let app = Router::new().route(
        "/auth/tokens/introspect",
        get(move |headers: HeaderMap| {
            let body = body.clone();
            let encoding = encoding.clone();
            let counter = counter.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                assert_eq!(headers[header::AUTHORIZATION], format!("Bearer {TOKEN}"));
                assert_eq!(headers[header::ACCEPT_ENCODING], "identity");
                let mut response = Response::builder().status(status).header(
                    header::LOCATION,
                    format!("http://{address}/auth/tokens/introspect"),
                );
                if let Some(encoding) = encoding {
                    response = response.header(header::CONTENT_ENCODING, encoding);
                }
                response.body(Body::from(body)).unwrap()
            }
        }),
    );
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (
        SdlcConfig {
            configuration_readback_enabled: true,
            auth_url: format!("http://{address}"),
            configuration_reader_subject: SUBJECT.into(),
            configuration_reader_agent_ids: String::new(),
        },
        calls,
        task,
    )
}

async fn finish(task: tokio::task::JoinHandle<()>) {
    task.abort();
    let _ = task.await;
}

#[tokio::test]
async fn sdlc_configuration_authorization_is_fresh_exact_and_agent_scoped() {
    let id = Uuid::new_v4();
    let (mut config, calls, task) = service(StatusCode::OK, principal().to_string(), None).await;
    config.configuration_reader_agent_ids = id.to_string();
    authorize(&config, id, &headers()).await.unwrap();
    authorize(&config, id, &headers()).await.unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(matches!(
        authorize(&config, Uuid::new_v4(), &headers()).await,
        Err(AppError::Forbidden)
    ));
    finish(task).await;

    for mode in 0..6 {
        let mut value = principal();
        match mode {
            0 => value["sub"] = json!(Uuid::new_v4().to_string()),
            1 => value["email"] = json!(" "),
            2 => value["scopes"] = json!([]),
            3 => value["scopes"] = json!(["fleet-control:read", "fleet-control:*"]),
            4 => value["scopes"]
                .as_array_mut()
                .unwrap()
                .push(json!("fleet-control:write")),
            _ => value["scopes"]
                .as_array_mut()
                .unwrap()
                .push(json!("fleet-control:read")),
        }
        let (mut config, _, task) = service(StatusCode::OK, value.to_string(), None).await;
        config.configuration_reader_agent_ids = id.to_string();
        assert!(matches!(
            authorize(&config, id, &headers()).await,
            Err(AppError::Forbidden)
        ));
        finish(task).await;
    }
}

#[tokio::test]
async fn sdlc_configuration_denies_browser_legacy_or_disabled_access_before_io() {
    let id = Uuid::new_v4();
    let (mut config, calls, task) = service(StatusCode::OK, principal().to_string(), None).await;
    config.configuration_reader_agent_ids = id.to_string();
    for token in [
        "Bearer local-admin-jwt",
        "Bearer runtime-secret",
        "bearer sdlc_pat_invalid",
        "Bearer sdlc_pat_short",
    ] {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, token.parse().unwrap());
        assert!(matches!(
            authorize(&config, id, &headers).await,
            Err(AppError::Unauthorized)
        ));
    }
    assert!(matches!(
        authorize(&config, id, &HeaderMap::new()).await,
        Err(AppError::Unauthorized)
    ));
    config.configuration_readback_enabled = false;
    assert!(matches!(
        authorize(&config, id, &headers()).await,
        Err(AppError::Unavailable(_))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    finish(task).await;
}

#[tokio::test]
async fn sdlc_configuration_denies_revoked_redirect_malformed_and_oversized_readbacks() {
    let id = Uuid::new_v4();
    for (status, body, encoding) in [
        (StatusCode::UNAUTHORIZED, "{}".into(), None),
        (StatusCode::TEMPORARY_REDIRECT, "{}".into(), None),
        (StatusCode::SERVICE_UNAVAILABLE, "{}".into(), None),
        (StatusCode::OK, "{".into(), None),
        (StatusCode::OK, "x".repeat(16_385), None),
        (StatusCode::OK, principal().to_string(), Some("gzip")),
    ] {
        let (mut config, calls, task) = service(status, body, encoding).await;
        config.configuration_reader_agent_ids = id.to_string();
        let error = authorize(&config, id, &headers()).await.unwrap_err();
        assert!(!error.to_string().contains(TOKEN));
        if status == StatusCode::UNAUTHORIZED {
            assert!(matches!(error, AppError::Unauthorized));
        } else {
            assert!(matches!(error, AppError::Unavailable(_)));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        finish(task).await;
    }
    let mut value = principal();
    value["role"] = json!("admin");
    let (mut config, _, task) = service(StatusCode::OK, value.to_string(), None).await;
    config.configuration_reader_agent_ids = id.to_string();
    assert!(matches!(
        authorize(&config, id, &headers()).await,
        Err(AppError::Unavailable(_))
    ));
    finish(task).await;
}

#[test]
fn sdlc_configuration_authority_must_be_fixed_and_canonical() {
    let mut config = SdlcConfig {
        configuration_readback_enabled: true,
        auth_url: "https://auth.example.test".into(),
        configuration_reader_subject: SUBJECT.into(),
        configuration_reader_agent_ids: Uuid::new_v4().to_string(),
    };
    assert_eq!(
        authority(&config).unwrap().as_str(),
        "https://auth.example.test/auth/tokens/introspect"
    );
    for root in [
        "",
        "file:///tmp/auth",
        "http://user:secret@localhost",
        "http://localhost/path",
        "http://localhost?token=x",
        "http://localhost#x",
        " http://localhost",
        "http://localhost\\",
    ] {
        config.auth_url = root.into();
        assert!(authority(&config).is_err());
    }
    config.auth_url = "https://auth.example.test".into();
    for subject in [
        "",
        "invalid",
        "00000000-0000-0000-0000-000000000000",
        "EEEEEEEE-EEEE-4EEE-8EEE-EEEEEEEEEEEE",
    ] {
        config.configuration_reader_subject = subject.into();
        assert!(authority(&config).is_err());
    }
}

#[test]
fn sdlc_configuration_resource_allowlist_is_closed_bounded_and_explicit() {
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let mut config = SdlcConfig::default();
    config.configuration_reader_agent_ids = format!("{first},{second}");
    assert_eq!(
        registered_agents(&config).unwrap(),
        BTreeSet::from([first, second])
    );
    for value in [
        String::new(),
        "*".into(),
        format!("{first},{first}"),
        format!("{first}, {second}"),
        format!("{first},"),
        Uuid::nil().to_string(),
        "x".repeat(4097),
        first.to_string().to_uppercase(),
    ] {
        config.configuration_reader_agent_ids = value;
        assert!(registered_agents(&config).is_err());
    }
}
