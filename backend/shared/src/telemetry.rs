//! Telemetry compatibility layer aligned with `services-base`.
//!
//! Keep this API shape close to `sdlc-telemetry` so Fleet Control can switch to
//! the shared crate once private repository access is available in WSL and CI.

use axum::extract::Request;
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;

pub const REQUEST_ID_HEADER: HeaderName = HeaderName::from_static("x-request-id");

pub fn init_tracing(service: &str) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let json = std::env::var("SDLC_LOG_JSON")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false);

    if json {
        builder.json().with_current_span(false).init();
    } else {
        builder.compact().init();
    }
    tracing::info!(service, "tracing initialized");
}

pub async fn request_id_mw(mut request: Request, next: Next) -> Response {
    let started = Instant::now();
    let request_id = request
        .headers()
        .get(&REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.trim().is_empty() && value.len() <= 128)
        .map(str::to_owned)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let mut response = next.run(request).await;
    let elapsed_ms = started.elapsed().as_millis();

    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert(&REQUEST_ID_HEADER, value);
    }
    tracing::info!(
        request_id = %request_id,
        method = %method,
        path = %path,
        status = response.status().as_u16(),
        elapsed_ms,
        "request"
    );
    response
}

#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn of(extensions: &axum::http::Extensions) -> Option<&str> {
        extensions
            .get::<RequestId>()
            .map(|request_id| request_id.0.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::get;
    use tower::ServiceExt;

    #[tokio::test]
    async fn assigns_request_id_and_sets_response_header() {
        let app = Router::new()
            .route("/ping", get(|| async { "pong" }))
            .layer(axum::middleware::from_fn(request_id_mw));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/ping")
                    .body(axum::body::Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), 200);
        assert!(
            response
                .headers()
                .get(&REQUEST_ID_HEADER)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| !value.is_empty())
        );
    }

    #[tokio::test]
    async fn propagates_inbound_request_id() {
        let app = Router::new()
            .route("/ping", get(|| async { "pong" }))
            .layer(axum::middleware::from_fn(request_id_mw));

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/ping")
                    .header(&REQUEST_ID_HEADER, "client-id-42")
                    .body(axum::body::Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(
            response
                .headers()
                .get(&REQUEST_ID_HEADER)
                .and_then(|value| value.to_str().ok()),
            Some("client-id-42")
        );
    }

    #[tokio::test]
    async fn extensions_carry_request_id_for_error_envelopes() {
        let app = Router::new()
            .route(
                "/echo-id",
                get(|ext: axum::extract::Extension<RequestId>| async move { ext.0.0 }),
            )
            .layer(axum::middleware::from_fn(
                |mut request: Request, next: Next| async move {
                    request
                        .extensions_mut()
                        .insert(RequestId("fixed-1".to_string()));
                    next.run(request).await
                },
            ));

        let body = axum::body::to_bytes(
            app.oneshot(
                axum::http::Request::builder()
                    .uri("/echo-id")
                    .body(axum::body::Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
            .into_body(),
            1024,
        )
        .await
        .expect("body");

        assert_eq!(&body[..], b"fixed-1");
    }
}
