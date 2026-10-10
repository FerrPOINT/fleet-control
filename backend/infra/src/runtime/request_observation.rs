//! Partial native request evidence. Never dispatch, reload or promote readiness.
use super::*;
use serde::{Deserialize, Serialize};

const CONTRACT: &str = "fleet-native-request-observation/v1";
const SOURCE: &str = "bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3";
const CAPS: &str = "/fleet/v1/request-observations/capabilities";
const FACT: &str = "fleet_request_observer";
const BLOCKERS: [&str; 3] = [
    "configuration_revision_unverified",
    "source_inventory_unverified",
    "post_builder_transformations_unverified",
];

fn unavailable() -> AppError {
    AppError::Unavailable("original native request observation is unavailable".into())
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Capabilities {
    contract: String,
    native_source_revision: String,
    incarnation: Uuid,
    observational_read: bool,
    digest_only: bool,
    complete: bool,
    runtime_ready: bool,
}

impl Capabilities {
    fn validate(&self) -> Result<(), AppError> {
        if self.contract != CONTRACT
            || self.native_source_revision != SOURCE
            || self.incarnation.is_nil()
            || self.incarnation.get_version_num() != 4
            || self.incarnation.get_variant() != uuid::Variant::RFC4122
            || !self.observational_read
            || !self.digest_only
            || self.complete
            || self.runtime_ready
        {
            return Err(unavailable());
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    contract: String,
    incarnation: Uuid,
    run_id: String,
    session_id: String,
    observed_stage: String,
    api_mode: String,
    system_hmac_sha256: String,
    tools_hmac_sha256: String,
    frozen_prompt_hmac_sha256: String,
    home_hmac_sha256: String,
    cwd_hmac_sha256: String,
    model_hmac_sha256: String,
    complete: bool,
    runtime_ready: bool,
    blockers: Vec<String>,
    request_sequence: u64,
}

impl Observation {
    fn validate(&self, caps: &Capabilities, run: &SessionAgentRun) -> Result<(), AppError> {
        let digest = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        };
        if self.contract != CONTRACT
            || self.incarnation != caps.incarnation
            || Some(self.run_id.as_str()) != run.runtime_run_id.as_deref()
            || Some(self.session_id.as_str()) != run.runtime_session_id.as_deref()
            || self.observed_stage != "native_request_builder"
            || self.api_mode != "chat_completions"
            || !(1..=64).contains(&self.request_sequence)
            || self.complete
            || self.runtime_ready
            || self.blockers != BLOCKERS.map(str::to_owned)
            || [
                &self.system_hmac_sha256,
                &self.tools_hmac_sha256,
                &self.frozen_prompt_hmac_sha256,
                &self.home_hmac_sha256,
                &self.cwd_hmac_sha256,
                &self.model_hmac_sha256,
            ]
            .into_iter()
            .any(|value| !digest(value))
        {
            return Err(unavailable());
        }
        Ok(())
    }
}

async fn get(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    path: &str,
) -> Result<Vec<u8>, AppError> {
    let response = client
        .get(format!("{base}{path}"))
        .bearer_auth(token)
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .timeout(Duration::from_secs(3))
        .send()
        .await
        .map_err(|_| unavailable())?;
    if !response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|mime| {
            mime.split(';')
                .next()
                .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
        })
    {
        return Err(unavailable());
    }
    hermes_wire::read_body(response, reqwest::StatusCode::OK, 16_384)
        .await
        .map_err(|_| unavailable())
}

pub(super) async fn capabilities(
    client: &reqwest::Client,
    base: &str,
    token: &str,
) -> Result<Value, AppError> {
    let value = get(client, base, token, CAPS).await?;
    let caps: Capabilities = serde_json::from_slice(&value).map_err(|_| unavailable())?;
    caps.validate()?;
    serde_json::to_value(caps).map_err(|_| unavailable())
}

impl LocalRuntimeSupervisor {
    pub(super) async fn observer_for_phase(
        &self,
        agent: &Agent,
        phase: LaunchPhase,
    ) -> Result<bool, AppError> {
        let revision = match phase {
            LaunchPhase::Activation(revision) => {
                Some(self.repo.get_config_revision(agent.id, revision).await?)
            }
            LaunchPhase::Regular | LaunchPhase::Rollback => {
                self.repo.get_effective_config_revision(agent.id).await?
            }
        };
        match revision {
            Some(revision) => self.verify_observer_files(agent, &revision).await,
            None => Ok(false),
        }
    }

    async fn verify_observer_files(
        &self,
        agent: &Agent,
        revision: &domain::AgentConfigRevision,
    ) -> Result<bool, AppError> {
        let enabled = crate::request_observer_package::enabled(
            &revision.snapshot.config.config_json,
            revision.snapshot.renderer_version,
        )?;
        if enabled.is_none() {
            return Ok(false);
        }
        let files = crate::configuration_files(agent, &self.config, revision).await?;
        for (path, body) in files
            .iter()
            .filter(|(path, _)| crate::request_observer_package::observer_path(path))
        {
            let actual = activation_journal::read_backup(
                std::path::Path::new(&self.config.fleet.agents_root),
                path,
            )
            .await?;
            if if crate::request_observer_package::absent(path, body) {
                actual.is_some()
            } else {
                actual.as_deref() != Some(body.as_bytes())
            } {
                return Err(unavailable());
            }
        }
        Ok(enabled == Some(true))
    }

    pub(super) async fn observer_for_launch(&self, agent: &Agent) -> Result<bool, AppError> {
        let Some(launch) = self.repo.get_open_runtime_launch(agent.id).await? else {
            return Ok(false);
        };
        let Some(number) = launch.binding.configuration_revision else {
            return Ok(false);
        };
        let revision = self.repo.get_config_revision(agent.id, number).await?;
        if crate::request_observer_package::enabled(
            &revision.snapshot.config.config_json,
            revision.snapshot.renderer_version,
        )?
        .is_none()
        {
            return Ok(false);
        }
        if launch.binding.controller_id != self.controller_id
            || launch.binding.api_port != agent.api_port
            || serde_json::to_value(&launch.binding.paths).map_err(|_| unavailable())?
                != serde_json::to_value(&agent.paths).map_err(|_| unavailable())?
            || launch.binding.configuration_sha256.as_deref()
                != Some(
                    crate::runtime_launches::snapshot_hash(
                        &serde_json::to_value(&revision.snapshot).map_err(|_| unavailable())?,
                    )?
                    .as_str(),
                )
        {
            return Err(unavailable());
        }
        self.verify_observer_files(agent, &revision).await
    }

    pub(super) async fn original_request_observation(
        &self,
        agent: &Agent,
        run: &SessionAgentRun,
    ) -> Result<Value, AppError> {
        tokio::time::timeout(Duration::from_secs(15), async {
            let lock = self.lifecycle_lock(agent.id).await;
            let _guard = lock.lock().await;
            let context = native_context::accepted(self, agent, run).await?;
            let intent = self.repo.get_hermes_dispatch_intent_for_run(run.id).await?.ok_or_else(unavailable)?;
            let caps: Capabilities = serde_json::from_value(intent.capabilities.get(FACT)
                .cloned().ok_or_else(unavailable)?).map_err(|_| unavailable())?;
            caps.validate()?;
            // An unjournaled child may not acquire proof by querying a coincidentally live listener.
            crate::runtime_launches::dispatch_launch_id(&intent.capabilities)?.ok_or_else(unavailable)?;
            self.verify_dispatch_launch(agent.id, &intent.capabilities).await?;
            if !self.observer_for_launch(agent).await? { return Err(unavailable()); }
            let run_id = context.run.runtime_run_id.as_deref().ok_or_else(unavailable)?;
            if run_id.len() != 36 || !run_id.starts_with("run_")
                || !run_id[4..].bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            { return Err(unavailable()); }
            if capabilities(&self.client, &context.base, &context.token).await? != serde_json::to_value(&caps).map_err(|_| unavailable())?
            { return Err(unavailable()); }
            let value = get(&self.client, &context.base, &context.token,
                &format!("/fleet/v1/request-observations/{run_id}?incarnation={}", caps.incarnation)).await?;
            let observation: Observation = serde_json::from_slice(&value).map_err(|_| unavailable())?;
            observation.validate(&caps, &context.run)?;
            let after = native_context::accepted(self, agent, &context.run).await?;
            self.verify_dispatch_launch(agent.id, &intent.capabilities).await?;
            let current = self.repo.get_hermes_dispatch_intent_for_run(run.id).await?.ok_or_else(unavailable)?;
            if after.base != context.base || after.token != context.token
                || current.capabilities != intent.capabilities || current.origin != intent.origin
                || capabilities(&self.client, &context.base, &context.token).await? != serde_json::to_value(&caps).map_err(|_| unavailable())?
                || !self.observer_for_launch(agent).await?
            { return Err(unavailable()); }
            // Final physical custody check brackets the last HTTP as well.
            native_context::accepted(self, agent, &context.run).await?;
            self.verify_dispatch_launch(agent.id, &intent.capabilities).await?;
            Ok(json!({"contract":"fleet-managed-request-observation/v1", "agent_id":agent.id,
                "session_id":run.session_id,"run_id":run.id,"observation":observation,
                "complete":false,"runtime_ready":false,
                "blockers":["runtime_skill_inventory_not_verified","workflow_assignment_protocol_not_verified"]}))
        }).await.map_err(|_| unavailable())?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps() -> Value {
        json!({"contract":CONTRACT,"native_source_revision":SOURCE,"incarnation":Uuid::new_v4(),
            "observational_read":true,"digest_only":true,"complete":false,"runtime_ready":false})
    }

    #[test]
    fn capabilities_never_accept_readiness_or_foreign_protocol() {
        let good = caps();
        serde_json::from_value::<Capabilities>(good.clone())
            .unwrap()
            .validate()
            .unwrap();
        for (key, value) in [
            ("complete", json!(true)),
            ("runtime_ready", json!(true)),
            ("observational_read", json!(false)),
            ("digest_only", json!(false)),
            ("native_source_revision", json!("HEAD")),
            ("incarnation", json!(Uuid::nil())),
        ] {
            let mut bad = good.clone();
            bad[key] = value;
            assert!(
                serde_json::from_value::<Capabilities>(bad)
                    .unwrap()
                    .validate()
                    .is_err()
            );
        }
        let mut extra = good;
        extra["prompt"] = json!("foreign");
        assert!(serde_json::from_value::<Capabilities>(extra).is_err());
        let raw = serde_json::to_string(&caps()).unwrap();
        let duplicate = raw.replacen('{', "{\"complete\":false,", 1);
        assert!(serde_json::from_str::<Capabilities>(&duplicate).is_err());
    }

    fn run() -> SessionAgentRun {
        SessionAgentRun {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            agent_name: "fixture".into(),
            runtime_session_id: Some("fleet:fixture:agent".into()),
            runtime_run_id: Some(format!("run_{}", Uuid::new_v4().simple())),
            run_role: SessionRunRole::Primary,
            state: SessionRunState::Completed,
            last_error: None,
            last_event_at: None,
            model: None,
            provider: None,
            model_options: json!({}),
            created_at: shared::now().to_rfc3339(),
            updated_at: shared::now().to_rfc3339(),
        }
    }

    #[test]
    fn observation_binds_original_run_session_incarnation_and_partial_scope() {
        let caps: Capabilities = serde_json::from_value(caps()).unwrap();
        let run = run();
        let mut value = json!({"contract":CONTRACT,"incarnation":caps.incarnation,
            "run_id":run.runtime_run_id,"session_id":run.runtime_session_id,
            "observed_stage":"native_request_builder","api_mode":"chat_completions",
            "request_sequence":1,"complete":false,"runtime_ready":false,"blockers":BLOCKERS});
        for key in [
            "system_hmac_sha256",
            "tools_hmac_sha256",
            "frozen_prompt_hmac_sha256",
            "home_hmac_sha256",
            "cwd_hmac_sha256",
            "model_hmac_sha256",
        ] {
            value[key] = json!("a".repeat(64));
        }
        serde_json::from_value::<Observation>(value.clone())
            .unwrap()
            .validate(&caps, &run)
            .unwrap();
        for (key, changed) in [
            ("run_id", json!(format!("run_{}", Uuid::new_v4().simple()))),
            ("session_id", json!("foreign")),
            ("incarnation", json!(Uuid::new_v4())),
            ("request_sequence", json!(0)),
            ("request_sequence", json!(65)),
            ("complete", json!(true)),
            ("runtime_ready", json!(true)),
            ("blockers", json!([])),
            ("observed_stage", json!("wire")),
            ("model_hmac_sha256", json!("A".repeat(64))),
        ] {
            let mut bad = value.clone();
            bad[key] = changed;
            assert!(
                serde_json::from_value::<Observation>(bad)
                    .unwrap()
                    .validate(&caps, &run)
                    .is_err()
            );
        }
        let mut extra = value;
        extra["prompt"] = json!("do not expose");
        assert!(serde_json::from_value::<Observation>(extra).is_err());
    }

    #[tokio::test]
    async fn capabilities_http_is_authenticated_get_and_rejects_oversize_or_redirect() {
        use axum::{Router, routing::get};
        use std::sync::atomic::{AtomicUsize, Ordering};
        let calls = Arc::new(AtomicUsize::new(0));
        let count = calls.clone();
        let good = caps();
        let app = Router::new()
            .route(
                CAPS,
                get(move |headers: axum::http::HeaderMap| {
                    let value = good.clone();
                    let count = count.clone();
                    async move {
                        if headers
                            .get(reqwest::header::AUTHORIZATION)
                            .is_none_or(|header| header != "Bearer observer-test-only")
                        {
                            return Err(axum::http::StatusCode::UNAUTHORIZED);
                        }
                        assert_eq!(headers[reqwest::header::ACCEPT_ENCODING], "identity");
                        count.fetch_add(1, Ordering::SeqCst);
                        Ok(axum::Json(value))
                    }
                }),
            )
            .route(
                "/large",
                get(|| async { ([("content-type", "application/json")], "x".repeat(16_385)) }),
            )
            .route("/mime", get(|| async { "{}" }))
            .route(
                "/encoding",
                get(|| async {
                    (
                        [
                            ("content-type", "application/json"),
                            ("content-encoding", "gzip"),
                        ],
                        "{}",
                    )
                }),
            )
            .route(
                "/slow",
                get(|| async {
                    sleep(Duration::from_secs(4)).await;
                    axum::Json(json!({}))
                }),
            )
            .route(
                "/redirect",
                get(|| async { axum::response::Redirect::temporary(CAPS) }),
            );
        let socket = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", socket.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(socket, app).await.unwrap();
        });
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .no_zstd()
            .no_proxy()
            .build()
            .unwrap();
        capabilities(&client, &base, "observer-test-only")
            .await
            .unwrap();
        capabilities(&client, &base, "observer-test-only")
            .await
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(
            capabilities(&client, &base, "foreign-observer-test")
                .await
                .is_err()
        );
        assert!(
            super::get(&client, &base, "observer-test-only", "/large")
                .await
                .is_err()
        );
        assert!(
            super::get(&client, &base, "observer-test-only", "/redirect")
                .await
                .is_err()
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        for path in ["/mime", "/encoding", "/slow"] {
            assert!(
                super::get(&client, &base, "observer-test-only", path)
                    .await
                    .is_err()
            );
        }
        server.abort();
    }
}
