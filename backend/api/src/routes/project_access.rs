use app::AppContext;
use axum::http::{HeaderMap, StatusCode, header};
use domain::chats_directory::TaskProjectAccess;
use serde::Deserialize;
use shared::AppError;
use std::time::Duration;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectAccessResponse {
    contract_version: u32,
    tracker_instance_id: String,
    project_ids: Vec<Uuid>,
}

fn endpoint(config: &shared::config::TrackerConfig) -> Result<reqwest::Url, AppError> {
    let mut url = reqwest::Url::parse(&config.url)
        .map_err(|_| AppError::Unavailable("Tracker project access is not configured".into()))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
        || config.instance_id.trim().is_empty()
        || config.instance_id.len() > 128
    {
        return Err(AppError::Unavailable(
            "invalid Tracker project access configuration".into(),
        ));
    }
    url.set_path("/api/v1/sdlc/project-access");
    Ok(url)
}

fn decode(bytes: &[u8], instance: &str) -> Result<TaskProjectAccess, AppError> {
    let response: ProjectAccessResponse = serde_json::from_slice(bytes)
        .map_err(|_| AppError::Unavailable("Tracker project access response is invalid".into()))?;
    if response.contract_version != 1
        || response.tracker_instance_id != instance
        || response.project_ids.iter().any(Uuid::is_nil)
    {
        return Err(AppError::Unavailable(
            "Tracker project access identity mismatch".into(),
        ));
    }
    let mut project_ids = response.project_ids;
    project_ids.sort_unstable();
    project_ids.dedup();
    Ok(TaskProjectAccess {
        tracker_instance_id: response.tracker_instance_id,
        project_ids,
    })
}

/// One uncached request under the caller's verified bearer. Fleet roles cannot expand this scope.
pub async fn authorized_projects(
    ctx: &AppContext,
    headers: &HeaderMap,
) -> Result<TaskProjectAccess, AppError> {
    fetch(&ctx.config.tracker, headers).await
}

async fn fetch(
    config: &shared::config::TrackerConfig,
    headers: &HeaderMap,
) -> Result<TaskProjectAccess, AppError> {
    let url = endpoint(config)?;
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty())
        .ok_or(AppError::Unauthorized)?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(AppError::internal)?;
    let mut response = client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| AppError::Unavailable("Tracker project access is unavailable".into()))?;
    match response.status() {
        StatusCode::OK => (),
        StatusCode::UNAUTHORIZED => return Err(AppError::Unauthorized),
        StatusCode::FORBIDDEN => return Err(AppError::Forbidden),
        _ => {
            return Err(AppError::Unavailable(
                "Tracker project access was not authorized".into(),
            ));
        }
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| {
        AppError::Unavailable("Tracker project access response was interrupted".into())
    })? {
        if bytes.len().saturating_add(chunk.len()) > 1_048_576 {
            return Err(AppError::Unavailable(
                "Tracker project access response exceeds limit".into(),
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    decode(&bytes, &config.instance_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn configured_origin_and_scope_identity_are_strict() {
        for url in [
            "file:///tmp/tracker",
            "https://user:secret@tracker.test/",
            "https://tracker.test/path",
            "https://tracker.test/?q=x",
            "https://tracker.test/#x",
        ] {
            assert!(
                endpoint(&shared::config::TrackerConfig {
                    url: url.into(),
                    instance_id: "tracker".into()
                })
                .is_err()
            );
        }
        let id = Uuid::new_v4();
        let valid = serde_json::json!({"contract_version":1,"tracker_instance_id":"tracker","project_ids":[id]});
        assert_eq!(
            decode(&serde_json::to_vec(&valid).unwrap(), "tracker")
                .unwrap()
                .project_ids,
            vec![id]
        );
        for (key, value) in [
            ("contract_version", serde_json::json!(2)),
            ("tracker_instance_id", serde_json::json!("foreign")),
            ("project_ids", serde_json::json!([Uuid::nil()])),
            ("allowed_projects", serde_json::json!([])),
        ] {
            let mut wrong = valid.clone();
            wrong[key] = value;
            assert!(decode(&serde_json::to_vec(&wrong).unwrap(), "tracker").is_err());
        }
    }

    #[tokio::test]
    async fn scope_is_uncached_bearer_scoped_and_rejects_redirects_and_bad_responses() {
        let mode = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let redirected = Arc::new(AtomicUsize::new(0));
        let project = Uuid::new_v4();
        let state = (mode.clone(), calls.clone());
        let redirect_calls = redirected.clone();
        let router = axum::Router::new()
            .route("/api/v1/sdlc/project-access", axum::routing::get(move |headers: HeaderMap| {
                let (mode, calls) = state.clone();
                async move {
                    assert_eq!(headers.get(header::AUTHORIZATION).unwrap(), "Bearer current-user-token");
                    calls.fetch_add(1, Ordering::SeqCst);
                    let mut headers = HeaderMap::new();
                    let (status, body) = match mode.load(Ordering::SeqCst) {
                        0 => (StatusCode::OK, serde_json::json!({"contract_version":1,"tracker_instance_id":"tracker","project_ids":[project]}).to_string()),
                        1 => (StatusCode::OK, serde_json::json!({"contract_version":1,"tracker_instance_id":"tracker","project_ids":[]}).to_string()),
                        2 => { headers.insert(header::LOCATION, "/redirect-target".parse().unwrap()); (StatusCode::FOUND, String::new()) },
                        3 => (StatusCode::SERVICE_UNAVAILABLE, String::new()),
                        4 => (StatusCode::OK, "x".repeat(1_048_577)),
                        5 => (StatusCode::OK, "not-json".into()),
                        6 => (StatusCode::OK, serde_json::json!({"contract_version":1,"tracker_instance_id":"wrong","project_ids":[project]}).to_string()),
                        7 => (StatusCode::UNAUTHORIZED, String::new()),
                        _ => (StatusCode::FORBIDDEN, String::new()),
                    };
                    (status, headers, body)
                }
            }))
            .route("/redirect-target", axum::routing::get(move || {
                let calls = redirect_calls.clone();
                async move { calls.fetch_add(1, Ordering::SeqCst); "must never receive bearer" }
            }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let config = shared::config::TrackerConfig {
            url: format!("http://{}", listener.local_addr().unwrap()),
            instance_id: "tracker".into(),
        };
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let mut headers = HeaderMap::new();
        assert!(matches!(
            fetch(&config, &headers).await,
            Err(AppError::Unauthorized)
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        headers.insert(
            header::AUTHORIZATION,
            "Bearer current-user-token".parse().unwrap(),
        );
        assert_eq!(
            fetch(&config, &headers).await.unwrap().project_ids,
            vec![project]
        );
        mode.store(1, Ordering::SeqCst);
        assert!(
            fetch(&config, &headers)
                .await
                .unwrap()
                .project_ids
                .is_empty()
        );
        for value in 2..=8 {
            mode.store(value, Ordering::SeqCst);
            let result = fetch(&config, &headers).await;
            match value {
                7 => assert!(matches!(result, Err(AppError::Unauthorized))),
                8 => assert!(matches!(result, Err(AppError::Forbidden))),
                _ => assert!(matches!(result, Err(AppError::Unavailable(_)))),
            }
        }
        assert_eq!(calls.load(Ordering::SeqCst), 9);
        assert_eq!(redirected.load(Ordering::SeqCst), 0);
        server.abort();
    }
}
