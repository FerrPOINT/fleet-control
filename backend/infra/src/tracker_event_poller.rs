use app::FleetRepository;
use domain::{TRACKER_METADATA_BUDGET, TrackerMetadataPage, TrackerProjectionTarget};
use futures_util::{StreamExt, stream};
use reqwest::{
    Client, StatusCode, Url,
    header::{AUTHORIZATION, HeaderValue},
};
use serde::{Deserialize, de::DeserializeOwned};
use shared::{AppError, TrackerConfig};
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

/// Metadata projection only. This worker cannot dispatch a PM or transition a task.
pub struct TrackerEventPoller {
    repo: Arc<dyn FleetRepository>,
    http: Client,
    tracker: Url,
    introspection: Url,
    authorization: HeaderValue,
    instance: String,
    machine_subject: String,
    interval: Duration,
    after_session: Option<Uuid>,
}

#[derive(Default, Debug)]
pub struct PollReport {
    pub considered: usize,
    pub projected: usize,
    pub blocked: usize,
    pub blocked_reason: Option<&'static str>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Introspection {
    sub: String,
    email: String,
    scopes: Vec<String>,
}

impl Introspection {
    fn require_read_only(&self, subject: &str) -> Result<(), AppError> {
        if self.sub != subject
            || self.email.trim().is_empty()
            || self.scopes != ["task-tracker:read"]
        {
            return Err(AppError::Forbidden);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectScope {
    contract_version: u8,
    tracker_instance_id: String,
    project_ids: Vec<Uuid>,
}

fn unavailable() -> AppError {
    AppError::Unavailable("Tracker metadata synchronization is unavailable".into())
}

fn error_code(error: &AppError) -> &'static str {
    match error {
        AppError::Unauthorized => "credential_revoked_or_expired",
        AppError::Forbidden => "subject_scope_or_project_access_denied",
        AppError::Conflict(_) => "source_or_projection_reconciliation_required",
        AppError::Database(_) => "projection_database_unavailable",
        _ => "dependency_or_contract_unavailable",
    }
}

fn origin(raw: &str) -> Result<Url, AppError> {
    let url =
        Url::parse(raw).map_err(|_| AppError::validation("invalid metadata integration origin"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(AppError::validation(
            "metadata integrations require root HTTP origins",
        ));
    }
    Ok(url)
}

impl TrackerEventPoller {
    pub fn configured(
        config: &TrackerConfig,
        repo: Arc<dyn FleetRepository>,
    ) -> Result<Option<Self>, AppError> {
        let events = &config.events;
        if !events.enabled {
            return Ok(None);
        }
        let sub = Uuid::parse_str(&events.machine_subject)
            .map_err(|_| AppError::validation("metadata machine subject is invalid"))?;
        if sub.is_nil()
            || sub.to_string() != events.machine_subject
            || config.instance_id.trim().is_empty()
            || config.instance_id.len() > 128
            || !(1..=300).contains(&events.poll_interval_seconds)
            || !events.read_pat.starts_with("sdlc_pat_")
            || !(41..=136).contains(&events.read_pat.len())
            || !events.read_pat.bytes().all(|b| b.is_ascii_graphic())
        {
            return Err(AppError::validation(
                "invalid metadata polling configuration",
            ));
        }
        let mut introspection = origin(&events.auth_url)?;
        introspection.set_path("/auth/tokens/introspect");
        let mut authorization = HeaderValue::from_str(&format!("Bearer {}", events.read_pat))
            .map_err(|_| AppError::validation("invalid metadata credential"))?;
        authorization.set_sensitive(true);
        Ok(Some(Self {
            repo,
            tracker: origin(&config.url)?,
            introspection,
            authorization,
            instance: config.instance_id.clone(),
            machine_subject: events.machine_subject.clone(),
            interval: Duration::from_secs(events.poll_interval_seconds),
            after_session: None,
            http: Client::builder()
                .timeout(Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .build()
                .map_err(|_| unavailable())?,
        }))
    }

    async fn get(&self, url: Url, max_bytes: usize) -> Result<Vec<u8>, AppError> {
        let mut response = self
            .http
            .get(url)
            .header(AUTHORIZATION, self.authorization.clone())
            .send()
            .await
            .map_err(|_| unavailable())?;
        match response.status() {
            StatusCode::OK => (),
            StatusCode::UNAUTHORIZED => return Err(AppError::Unauthorized),
            StatusCode::FORBIDDEN => return Err(AppError::Forbidden),
            StatusCode::CONFLICT => {
                return Err(AppError::conflict(
                    "Tracker metadata source requires reconciliation",
                ));
            }
            _ => return Err(unavailable()),
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| unavailable())? {
            if bytes.len().saturating_add(chunk.len()) > max_bytes {
                return Err(unavailable());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }

    fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, AppError> {
        serde_json::from_slice(bytes).map_err(|_| unavailable())
    }

    async fn projects(&self) -> Result<Vec<Uuid>, AppError> {
        let identity: Introspection =
            Self::decode(&self.get(self.introspection.clone(), 16_384).await?)?;
        identity.require_read_only(&self.machine_subject)?;
        let mut url = self.tracker.clone();
        url.set_path("/api/v1/sdlc/project-access");
        let scope: ProjectScope = Self::decode(&self.get(url, 1_048_576).await?)?;
        if scope.contract_version != 1
            || scope.tracker_instance_id != self.instance
            || scope.project_ids.iter().any(Uuid::is_nil)
        {
            return Err(unavailable());
        }
        let mut ids = scope.project_ids;
        ids.sort_unstable();
        ids.dedup();
        Ok(ids)
    }

    async fn project(&self, target: TrackerProjectionTarget) -> Result<usize, AppError> {
        let after = self.repo.tracker_metadata_cursor(target.session_id).await?;
        let mut url = self.tracker.clone();
        url.path_segments_mut().map_err(|_| unavailable())?.extend([
            "api",
            "v1",
            "issues",
            &target.binding.task_id.to_string(),
            "sdlc",
            "events",
        ]);
        url.query_pairs_mut()
            .append_pair("projection", "metadata_v1")
            .append_pair("after", &after.to_string())
            .append_pair("limit", "100")
            .append_pair("max_bytes", &TRACKER_METADATA_BUDGET.to_string());
        let bytes = self.get(url, TRACKER_METADATA_BUDGET).await?;
        let page = TrackerMetadataPage::decode(&bytes, &target.binding, after)?;
        Ok(self
            .repo
            .project_tracker_metadata(target.session_id, target.binding, after, page)
            .await?
            .projected)
    }

    pub async fn poll_once(&mut self) -> Result<PollReport, AppError> {
        // Each cycle rechecks Base and Tracker. Every event fetch also checks Tracker ACLs.
        let projects = self.projects().await?;
        let targets = self
            .repo
            .tracker_projection_targets(&self.instance, &projects, self.after_session)
            .await?;
        let next = if targets.len() == 100 {
            targets.last().map(|t| t.session_id)
        } else {
            None
        };
        let considered = targets.len();
        let outcomes = stream::iter(targets)
            .map(|target| self.project(target))
            .buffer_unordered(2)
            .collect::<Vec<_>>()
            .await;
        self.after_session = next;
        let mut report = PollReport {
            considered,
            ..Default::default()
        };
        for result in outcomes {
            match result {
                Ok(projected) => report.projected += projected,
                Err(error) => {
                    report.blocked += 1;
                    report.blocked_reason.get_or_insert(error_code(&error));
                }
            }
        }
        Ok(report)
    }

    pub async fn run(mut self) {
        let mut ticker = tokio::time::interval(self.interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            match self.poll_once().await {
                Ok(report) if report.blocked > 0 => tracing::warn!(
                    blocked = report.blocked,
                    reason = report.blocked_reason,
                    "Tracker metadata bindings require reconciliation"
                ),
                Ok(_) => (),
                Err(error) => tracing::warn!(
                    reason = error_code(&error),
                    "Tracker metadata polling authorization or dependency is unavailable"
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_scope_never_accepts_write_wildcard_or_another_subject() {
        let subject = Uuid::new_v4().to_string();
        for scopes in [
            vec!["*"],
            vec!["task-tracker:write"],
            vec!["task-tracker:read", "task-tracker:write"],
            vec!["task-tracker:read", "task-tracker:read"],
            vec![],
        ] {
            let identity = Introspection {
                sub: subject.clone(),
                email: "machine@example.test".into(),
                scopes: scopes.into_iter().map(str::to_string).collect(),
            };
            assert!(identity.require_read_only(&subject).is_err());
        }
        let valid = Introspection {
            sub: subject.clone(),
            email: "machine@example.test".into(),
            scopes: vec!["task-tracker:read".into()],
        };
        valid.require_read_only(&subject).unwrap();
        assert!(
            valid
                .require_read_only(&Uuid::new_v4().to_string())
                .is_err()
        );
        assert!(
            TrackerEventPoller::decode::<Introspection>(
                br#"{"sub":"owner","email":"x","scopes":["task-tracker:read"],"role":"admin"}"#
            )
            .is_err()
        );
    }

    #[test]
    fn metadata_origins_reject_credentials_paths_queries_and_non_http() {
        for url in [
            "file:///secret",
            "http://user:secret@tracker/",
            "http://tracker/path",
            "http://tracker/?secret=x",
            "http://tracker/#secret",
        ] {
            assert!(origin(url).is_err());
        }
        assert!(origin("http://tracker:3456/").is_ok());
    }
}
