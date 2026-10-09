//! Bounded Hermes HTTP wire checks; these do not attest native config or admit an assignment.
use super::*;
use reqwest::{Response, StatusCode, header};
use sha2::{Digest, Sha256};

pub(super) fn credential_fingerprint(token: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"fleet-hermes-default-profile-v1\0");
    hash.update(token.as_bytes());
    format!("{:x}", hash.finalize())
}

pub(super) fn verify_intent(
    intent: &app::HermesDispatchIntent,
    origin: &str,
    token: &str,
) -> Result<(), AppError> {
    if intent.origin != origin
        || intent.credential_fingerprint != credential_fingerprint(token)
        || intent.idempotency_key != intent.message_id.to_string()
        || intent.request_hash != format!("{:x}", Sha256::digest(intent.request_body.as_bytes()))
    {
        return Err(AppError::Unavailable(
            "Hermes original dispatch identity is not verified".into(),
        ));
    }
    Ok(())
}

pub(super) async fn read_json(
    response: Response,
    expected: StatusCode,
    limit: usize,
) -> Result<Value, AppError> {
    let body = read_body(response, expected, limit).await?;
    serde_json::from_slice(&body)
        .map_err(|_| AppError::Unavailable("Hermes protocol response is malformed".into()))
}

pub(super) async fn read_body(
    mut response: Response,
    expected: StatusCode,
    limit: usize,
) -> Result<Vec<u8>, AppError> {
    if response.status() != expected {
        return Err(AppError::Unavailable(format!(
            "Hermes protocol returned HTTP {}",
            response.status()
        )));
    }
    if response
        .headers()
        .get(header::CONTENT_ENCODING)
        .is_some_and(|value| value != "identity")
        || response
            .content_length()
            .is_some_and(|size| size > limit as u64)
    {
        return Err(AppError::Unavailable(
            "Hermes protocol response exceeds its bounds".into(),
        ));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| AppError::Unavailable("Hermes protocol response was interrupted".into()))?
    {
        if body.len().saturating_add(chunk.len()) > limit {
            return Err(AppError::Unavailable(
                "Hermes protocol response exceeds its bounds".into(),
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub(super) fn task_protocol(capabilities: &Value) -> Result<(), AppError> {
    let valid = capabilities["object"] == "hermes.api_server.capabilities"
        && capabilities["platform"] == "hermes-agent"
        && capabilities["auth"]["type"] == "bearer"
        && capabilities["auth"]["required"] == true
        && capabilities["runtime"]["mode"] == "server_agent"
        && capabilities["runtime"]["tool_execution"] == "server"
        && capabilities["runtime"]["split_runtime"] == false
        && capabilities["features"]["runs_idempotency"]["supported"] == true
        && capabilities["features"]["runs_idempotency"]["durable"] == true
        && capabilities["features"]["runs_idempotency"]["retention_seconds"].as_u64()
            == Some(86_400);
    if !valid {
        return Err(AppError::Unavailable(
            "Hermes durable task protocol is not verified".into(),
        ));
    }
    for (feature, endpoint, method, path) in [
        ("run_submission", "runs", "POST", "/v1/runs"),
        ("run_status", "run_status", "GET", "/v1/runs/{run_id}"),
        (
            "run_events_sse",
            "run_events",
            "GET",
            "/v1/runs/{run_id}/events",
        ),
        ("run_stop", "run_stop", "POST", "/v1/runs/{run_id}/stop"),
    ] {
        if capabilities["features"][feature] != true
            || capabilities["endpoints"][endpoint]["method"] != method
            || capabilities["endpoints"][endpoint]["path"] != path
        {
            return Err(AppError::Unavailable(
                "Hermes task protocol endpoints do not match".into(),
            ));
        }
    }
    Ok(())
}

pub(super) fn dispatch_capabilities(capabilities: &Value) -> Result<Value, AppError> {
    task_protocol(capabilities)?;
    // Persist only the verified wire facts, never arbitrary upstream metadata/secrets.
    let mut features = json!({"runs_idempotency": {
        "supported":true,"durable":true,"retention_seconds":86400
    }});
    let mut endpoints = json!({});
    for (feature, endpoint) in [
        ("run_submission", "runs"),
        ("run_status", "run_status"),
        ("run_events_sse", "run_events"),
        ("run_stop", "run_stop"),
    ] {
        features[feature] = Value::Bool(true);
        endpoints[endpoint] = json!({
            "method":capabilities["endpoints"][endpoint]["method"],
            "path":capabilities["endpoints"][endpoint]["path"]
        });
    }
    Ok(json!({
        "object":"hermes.api_server.capabilities","platform":"hermes-agent",
        "auth":{"type":"bearer","required":true},
        "runtime":{"mode":"server_agent","tool_execution":"server","split_runtime":false},
        "features":features,"endpoints":endpoints
    }))
}

pub(super) fn terminal_event(
    event: &str,
    payload: &Value,
    run_id: &str,
) -> Result<Option<SessionRunState>, AppError> {
    let state = match event {
        "run.completed" => SessionRunState::Completed,
        "run.failed" | "run.interrupted" => SessionRunState::Failed,
        "run.cancelled" | "run.stopped" => SessionRunState::Cancelled,
        _ => return Ok(None),
    };
    if !crate::pm_execution::valid_hermes_ref(run_id)
        || payload.get("run_id").and_then(Value::as_str) != Some(run_id)
        || payload
            .get("event")
            .is_some_and(|name| name.as_str() != Some(event))
        || (state == SessionRunState::Completed
            && (payload.get("completed").and_then(Value::as_bool) != Some(true)
                || payload.get("partial").and_then(Value::as_bool) != Some(false)
                || payload.get("interrupted").and_then(Value::as_bool) != Some(false)))
    {
        return Err(AppError::Unavailable(
            "Hermes terminal run evidence does not match".into(),
        ));
    }
    Ok(Some(state))
}

pub(super) fn terminal_readback(payload: &Value, run_id: &str) -> Result<&'static str, AppError> {
    if payload.get("object").and_then(Value::as_str) != Some("hermes.run") {
        return Err(AppError::Unavailable(
            "Hermes run status response does not match".into(),
        ));
    }
    let event = match payload.get("status").and_then(Value::as_str) {
        Some("completed") => "run.completed",
        Some("failed") => "run.failed",
        Some("interrupted") => "run.interrupted",
        Some("cancelled") => "run.cancelled",
        Some("stopped") => "run.stopped",
        _ => {
            return Err(AppError::Unavailable(
                "Hermes run status is not terminal; reconciliation is required".into(),
            ));
        }
    };
    terminal_event(event, payload, run_id)?;
    Ok(event)
}

fn accepted_run(payload: &Value) -> Result<String, AppError> {
    let run_id = payload.get("run_id").and_then(Value::as_str);
    let replayed = payload.get("replayed").and_then(Value::as_bool);
    let status = payload.get("status").and_then(Value::as_str);
    let valid_state = match replayed {
        Some(false) => status == Some("started"),
        Some(true) => matches!(
            status,
            Some(
                "queued"
                    | "started"
                    | "running"
                    | "waiting_for_approval"
                    | "stopping"
                    | "completed"
                    | "failed"
                    | "cancelled"
                    | "interrupted"
            )
        ),
        None => false,
    };
    if !valid_state || run_id.is_none_or(|id| !crate::pm_execution::valid_hermes_ref(id)) {
        return Err(AppError::Unavailable(
            "Hermes run acceptance is not verified".into(),
        ));
    }
    Ok(run_id.expect("validated run ID").into())
}

pub(super) fn effective_session(payload: &Value, run_id: &str) -> Result<String, AppError> {
    let session_id = payload.get("session_id").and_then(Value::as_str);
    if payload.get("object").and_then(Value::as_str) != Some("hermes.run")
        || payload.get("run_id").and_then(Value::as_str) != Some(run_id)
        || session_id.is_none_or(|id| !domain::valid_ref(id, 512))
        || !matches!(
            payload.get("status").and_then(Value::as_str),
            Some(
                "queued"
                    | "started"
                    | "running"
                    | "waiting_for_approval"
                    | "stopping"
                    | "completed"
                    | "failed"
                    | "cancelled"
                    | "interrupted"
                    | "stopped"
            )
        )
    {
        return Err(AppError::Unavailable(
            "Hermes accepted run readback does not match".into(),
        ));
    }
    // Completion cannot be inferred from an incomplete or inconsistent status document.
    if payload.get("status").and_then(Value::as_str) == Some("completed") {
        terminal_readback(payload, run_id)?;
    }
    Ok(session_id.expect("validated effective session").to_owned())
}

pub(super) async fn read_accepted_run(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    run_id: &str,
) -> Result<Value, AppError> {
    if !crate::pm_execution::valid_hermes_ref(run_id) {
        return Err(AppError::Unavailable(
            "Hermes accepted run reference is invalid".into(),
        ));
    }
    let response = client
        .get(format!("{base}/v1/runs/{run_id}"))
        .bearer_auth(token)
        .header(header::ACCEPT_ENCODING, "identity")
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|_| AppError::Unavailable("Hermes accepted run readback is unavailable".into()))?;
    let payload = read_json(response, StatusCode::OK, 1024 * 1024).await?;
    effective_session(&payload, run_id)?;
    Ok(payload)
}

pub(super) async fn submit(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    message_id: Uuid,
    body: &str,
    store_id: Option<&str>,
) -> Result<String, AppError> {
    let mut request = client
        .post(format!("{base}/v1/runs"))
        .bearer_auth(token)
        .header("Idempotency-Key", message_id.to_string())
        .header(header::ACCEPT_ENCODING, "identity")
        .timeout(Duration::from_secs(30))
        .header(header::CONTENT_TYPE, "application/json")
        .body(body.to_owned());
    if let Some(store_id) = store_id {
        request = request.header("X-Fleet-Recovery-Store-Id", store_id);
    }
    let response = request
        .send()
        .await
        .map_err(|_| AppError::Unavailable("Hermes run acceptance is unknown".into()))?;
    let payload = read_json(response, StatusCode::ACCEPTED, 16_384).await?;
    accepted_run(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn capabilities() -> Value {
        json!({
            "object":"hermes.api_server.capabilities", "platform":"hermes-agent",
            "auth":{"type":"bearer","required":true},
            "runtime":{"mode":"server_agent","tool_execution":"server","split_runtime":false},
            "features":{"run_submission":true,"run_status":true,"run_events_sse":true,"run_stop":true,
                "runs_idempotency":{"supported":true,"durable":true,"retention_seconds":86400}},
            "endpoints":{"runs":{"method":"POST","path":"/v1/runs"},
                "run_status":{"method":"GET","path":"/v1/runs/{run_id}"},
                "run_events":{"method":"GET","path":"/v1/runs/{run_id}/events"},
                "run_stop":{"method":"POST","path":"/v1/runs/{run_id}/stop"}}
        })
    }

    #[test]
    fn task_protocol_requires_durable_authenticated_exact_endpoints() {
        let good = capabilities();
        task_protocol(&good).unwrap();
        for pointer in [
            "/object",
            "/platform",
            "/auth/type",
            "/auth/required",
            "/runtime/mode",
            "/runtime/tool_execution",
            "/runtime/split_runtime",
            "/features/runs_idempotency/supported",
            "/features/runs_idempotency/durable",
            "/features/runs_idempotency/retention_seconds",
            "/features/run_submission",
            "/features/run_status",
            "/features/run_events_sse",
            "/features/run_stop",
            "/endpoints/runs/method",
            "/endpoints/runs/path",
            "/endpoints/run_status/path",
            "/endpoints/run_events/path",
            "/endpoints/run_stop/path",
        ] {
            let mut bad = good.clone();
            *bad.pointer_mut(pointer).unwrap() = Value::Null;
            assert!(task_protocol(&bad).is_err(), "{pointer}");
        }
        for bad in [
            json!(false),
            json!("true"),
            json!(true),
            json!({"supported":true}),
        ] {
            let mut payload = good.clone();
            payload["features"]["runs_idempotency"] = bad;
            assert!(task_protocol(&payload).is_err());
        }
        for retention in [json!(0), json!(1), json!(86400.0), json!("86400")] {
            let mut payload = good.clone();
            payload["features"]["runs_idempotency"]["retention_seconds"] = retention;
            assert!(task_protocol(&payload).is_err());
        }
    }

    #[test]
    fn acceptance_is_not_inferred_from_arbitrary_json_or_eof() {
        let good = json!({"run_id":"run_0123","status":"started","replayed":false});
        assert_eq!(accepted_run(&good).unwrap(), "run_0123");
        for (field, bad) in [
            ("run_id", json!("../foreign")),
            ("run_id", json!("")),
            ("status", json!("completed")),
            ("status", json!("success")),
            ("replayed", Value::Null),
            ("replayed", json!("false")),
        ] {
            let mut payload = good.clone();
            payload[field] = bad;
            assert!(accepted_run(&payload).is_err(), "{field}");
        }
        let replay = json!({"run_id":"run_original","status":"interrupted","replayed":true});
        assert_eq!(accepted_run(&replay).unwrap(), "run_original");
    }

    #[test]
    fn terminal_evidence_requires_exact_run_event_and_completed_flags() {
        let good = json!({"object":"hermes.run","run_id":"run_original","status":"completed",
            "completed":true,"partial":false,"interrupted":false,"output":"result"});
        assert_eq!(
            terminal_readback(&good, "run_original").unwrap(),
            "run.completed"
        );
        assert_eq!(
            terminal_event("run.completed", &good, "run_original").unwrap(),
            Some(SessionRunState::Completed)
        );
        for (field, value) in [
            ("run_id", json!("run_foreign")),
            ("object", json!("hermes.response")),
            ("status", json!("succeeded")),
            ("completed", json!(false)),
            ("completed", json!("true")),
            ("partial", json!(true)),
            ("interrupted", json!(true)),
        ] {
            let mut bad = good.clone();
            bad[field] = value;
            assert!(terminal_readback(&bad, "run_original").is_err(), "{field}");
        }
        for field in [
            "run_id",
            "object",
            "status",
            "completed",
            "partial",
            "interrupted",
        ] {
            let mut bad = good.clone();
            bad.as_object_mut().unwrap().remove(field);
            assert!(terminal_readback(&bad, "run_original").is_err(), "{field}");
        }
        for event in [
            "response.completed",
            "message.done",
            "run.cancellation_requested",
            "tool.failed",
        ] {
            assert_eq!(terminal_event(event, &good, "run_original").unwrap(), None);
        }
        let mismatch = json!({"run_id":"run_original","event":"run.failed","completed":true,"partial":false,"interrupted":false});
        assert!(terminal_event("run.completed", &mismatch, "run_original").is_err());
        for (status, state) in [
            ("failed", SessionRunState::Failed),
            ("interrupted", SessionRunState::Failed),
            ("cancelled", SessionRunState::Cancelled),
            ("stopped", SessionRunState::Cancelled),
        ] {
            let payload = json!({"object":"hermes.run","run_id":"run_original","status":status});
            let event = terminal_readback(&payload, "run_original").unwrap();
            assert_eq!(
                terminal_event(event, &payload, "run_original").unwrap(),
                Some(state)
            );
        }
    }

    #[tokio::test]
    async fn submit_is_one_authenticated_post_and_does_not_expose_rejection_body() {
        for status in [
            StatusCode::ACCEPTED,
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::OK,
            StatusCode::FOUND,
        ] {
            let calls = Arc::new(AtomicUsize::new(0));
            let count = calls.clone();
            let id = Uuid::new_v4();
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let router = axum::Router::new().route("/v1/runs", axum::routing::post(move |headers: axum::http::HeaderMap, axum::Json(body): axum::Json<Value>| {
                count.fetch_add(1, Ordering::SeqCst);
                assert_eq!(headers["authorization"], "Bearer wire-fixture-only");
                assert_eq!(headers["idempotency-key"], id.to_string());
                assert_eq!(body["session_id"], "fleet:fixture:agent");
                async move { (status, axum::Json(json!({"run_id":"run_fixture","status":"started","replayed":false,"diagnostic":"wire-fixture-private"}))) }
            }));
            let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
            let client = reqwest::Client::builder()
                .retry(reqwest::retry::never())
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .build()
                .unwrap();
            let result = submit(
                &client,
                &base,
                "wire-fixture-only",
                id,
                r#"{"input":"fixture","session_id":"fleet:fixture:agent"}"#,
                None,
            )
            .await;
            assert_eq!(result.is_ok(), status == StatusCode::ACCEPTED);
            if let Err(error) = result {
                assert!(!error.to_string().contains("wire-fixture-private"));
            }
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            server.abort();
            let _ = server.await;
        }
    }

    #[test]
    fn accepted_session_is_native_readback_not_the_requested_alias() {
        for state in [
            "queued",
            "started",
            "running",
            "waiting_for_approval",
            "stopping",
            "completed",
            "failed",
            "cancelled",
            "interrupted",
            "stopped",
        ] {
            let payload = json!({"object":"hermes.run","run_id":"run_original",
                "session_id":"native-session","status":state,
                "completed":state == "completed","partial":false,"interrupted":state == "interrupted"});
            assert_eq!(
                effective_session(&payload, "run_original").unwrap(),
                "native-session"
            );
        }
        let good = json!({"object":"hermes.run","run_id":"run_original",
            "session_id":"native-session","status":"running"});
        for (field, value) in [
            ("object", json!("hermes.response")),
            ("run_id", json!("foreign")),
            ("session_id", Value::Null),
            ("session_id", json!("")),
            ("session_id", json!("has space")),
            ("session_id", json!("x".repeat(513))),
            ("status", json!("succeeded")),
        ] {
            let mut bad = good.clone();
            bad[field] = value;
            assert!(effective_session(&bad, "run_original").is_err(), "{field}");
        }
        let partial = json!({"object":"hermes.run","run_id":"run_original",
            "session_id":"native-session","status":"completed","completed":true,"partial":true,"interrupted":false});
        assert!(effective_session(&partial, "run_original").is_err());
    }

    #[test]
    fn credential_fingerprint_is_stable_profile_scoped_and_not_the_token() {
        let original = credential_fingerprint("wire-fixture-only");
        assert_eq!(original.len(), 64);
        assert_eq!(original, credential_fingerprint("wire-fixture-only"));
        assert_ne!(original, credential_fingerprint("rotated-fixture-only"));
        assert_ne!(
            original,
            format!("{:x}", Sha256::digest(b"wire-fixture-only"))
        );
        assert!(!original.contains("wire-fixture-only"));
    }

    #[test]
    fn journal_capabilities_exclude_untrusted_metadata_and_pin_verified_protocol_only() {
        let mut upstream = capabilities();
        upstream["diagnostic"] = json!({"token":"fixture-upstream-private"});
        upstream["auth"]["private"] = json!("fixture-upstream-private");
        upstream["endpoints"]["runs"]["private"] = json!("fixture-upstream-private");
        let pinned = dispatch_capabilities(&upstream).unwrap();
        assert_eq!(pinned, capabilities());
        assert!(!pinned.to_string().contains("fixture-upstream-private"));
        upstream["features"]["runs_idempotency"]["durable"] = json!(false);
        assert!(dispatch_capabilities(&upstream).is_err());
    }

    #[tokio::test]
    async fn submit_preserves_journal_bytes_instead_of_reserializing_mutable_input() {
        let expected =
            "{\n  \"input\":\"immutable fixture\", \"session_id\":\"fleet:fixture:agent\"\n}";
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let id = Uuid::new_v4();
        let router = axum::Router::new().route(
            "/v1/runs",
            axum::routing::post(
                move |headers: axum::http::HeaderMap, body: axum::body::Bytes| async move {
                    assert_eq!(headers["content-type"], "application/json");
                    assert_eq!(headers["idempotency-key"], id.to_string());
                    assert_eq!(body.as_ref(), expected.as_bytes());
                    (
                        StatusCode::ACCEPTED,
                        axum::Json(
                            json!({"run_id":"run_exact_bytes","status":"started","replayed":false}),
                        ),
                    )
                },
            ),
        );
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let client = reqwest::Client::builder()
            .retry(reqwest::retry::never())
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .build()
            .unwrap();
        assert_eq!(
            submit(&client, &base, "wire-fixture-only", id, expected, None)
                .await
                .unwrap(),
            "run_exact_bytes"
        );
        server.abort();
        let _ = server.await;
    }

    #[tokio::test]
    async fn accepted_readback_is_get_only_bounded_and_authenticated() {
        for code in [
            StatusCode::OK,
            StatusCode::NOT_FOUND,
            StatusCode::FOUND,
            StatusCode::INTERNAL_SERVER_ERROR,
        ] {
            let calls = Arc::new(AtomicUsize::new(0));
            let count = calls.clone();
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let base = format!("http://{}", listener.local_addr().unwrap());
            let router = axum::Router::new().route("/v1/runs/run_fixture", axum::routing::get(move |headers: axum::http::HeaderMap| {
                count.fetch_add(1, Ordering::SeqCst);
                assert_eq!(headers["authorization"], "Bearer readback-fixture-only");
                assert_eq!(headers["accept-encoding"], "identity");
                async move { (code, axum::Json(json!({"object":"hermes.run","run_id":"run_fixture",
                    "session_id":"native-session","status":"running","private":"fixture-body-private"}))) }
            }));
            let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
            let client = reqwest::Client::builder()
                .retry(reqwest::retry::never())
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .build()
                .unwrap();
            let result =
                read_accepted_run(&client, &base, "readback-fixture-only", "run_fixture").await;
            assert_eq!(result.is_ok(), code == StatusCode::OK);
            if let Err(error) = result {
                assert!(!error.to_string().contains("fixture-body-private"));
            }
            assert!(
                read_accepted_run(&client, &base, "readback-fixture-only", "../bad")
                    .await
                    .is_err()
            );
            assert_eq!(calls.load(Ordering::SeqCst), 1);
            server.abort();
            let _ = server.await;
        }
    }

    #[tokio::test]
    async fn bounded_json_rejects_declared_and_streamed_oversize_or_encoding() {
        use axum::response::IntoResponse;
        for (chunked, encoded) in [(false, false), (true, false), (false, true)] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let url = format!("http://{}/fixture", listener.local_addr().unwrap());
            let router = axum::Router::new().route(
                "/fixture",
                axum::routing::get(move || async move {
                    let mut response = if chunked {
                        let chunks = futures_util::stream::iter([
                            Ok::<_, std::io::Error>("x".repeat(128)),
                            Ok("y".repeat(128)),
                        ]);
                        axum::body::Body::from_stream(chunks).into_response()
                    } else {
                        "x".repeat(256).into_response()
                    };
                    if encoded {
                        response.headers_mut().insert(
                            header::CONTENT_ENCODING,
                            header::HeaderValue::from_static("gzip"),
                        );
                    }
                    response
                }),
            );
            let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
            let response = reqwest::Client::new().get(url).send().await.unwrap();
            assert!(read_json(response, StatusCode::OK, 64).await.is_err());
            server.abort();
            let _ = server.await;
        }
    }
}
