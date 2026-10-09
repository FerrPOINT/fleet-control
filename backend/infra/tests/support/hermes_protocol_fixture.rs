use axum::{Json, Router, http::HeaderMap, routing::get};
use serde_json::{Value, json};

pub fn capabilities() -> Value {
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

pub fn preflight<S: Clone + Send + Sync + 'static>(router: Router<S>) -> Router<S> {
    router
        .route("/health", get(|| async { Json(json!({"status":"ok"})) }))
        .route(
            "/v1/capabilities",
            get(|headers: HeaderMap| async move {
                assert!(
                    headers["authorization"]
                        .to_str()
                        .unwrap()
                        .starts_with("Bearer fc_")
                );
                assert_eq!(headers["accept-encoding"], "identity");
                Json(capabilities())
            }),
        )
}
