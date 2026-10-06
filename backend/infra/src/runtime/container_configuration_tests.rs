use super::*;
use app::runtime_launch::{RuntimeConfigurationClaim, RuntimeContainerPreparation};
use domain::{AgentConfigRevision, UpdateAgentConfigRequest};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use std::path::{Path, PathBuf};

async fn draft(
    repo: &crate::PostgresFleetRepository,
    agent: &Agent,
    owner: Uuid,
    soul: &str,
) -> AgentConfigRevision {
    let revision = repo
        .create_config_revision(
            agent.id,
            UpdateAgentConfigRequest {
                config_json: json!({"model":"test-model"}),
                soul_md: soul.into(),
                env_json: json!({}),
            },
            owner,
        )
        .await
        .unwrap();
    repo.validate_config_revision(agent.id, revision.revision, vec![])
        .await
        .unwrap();
    revision
}

async fn claim(repo: &crate::PostgresFleetRepository, revision: &AgentConfigRevision, owner: Uuid) {
    repo.request_config_activation(revision.agent_id, revision.revision, owner)
        .await
        .unwrap();
    let claimed = repo.claim_config_activation().await.unwrap().unwrap();
    assert_eq!(
        (claimed.agent_id, claimed.revision),
        (revision.agent_id, revision.revision)
    );
}

async fn apply(
    runtime: &LocalRuntimeSupervisor,
    revision: &AgentConfigRevision,
) -> Result<(), AppError> {
    let mut journal = None;
    let result = runtime.apply_config_revision(revision, &mut journal).await;
    let reconciled = !matches!(&result, Err(AppError::Unavailable(_)));
    runtime
        .repo
        .finish_config_activation(
            revision.agent_id,
            revision.revision,
            result
                .as_ref()
                .err()
                .map(|error| crate::redact_text(&error.to_string())),
            reconciled,
        )
        .await
        .unwrap();
    if reconciled && let Some(journal) = journal {
        journal.acknowledge().await.unwrap();
    }
    result
}

#[tokio::test]
async fn configuration_admission_precedes_create_and_rechecks_exact_claimed_revision() {
    let Some((repo, agent, owner, config, root)) =
        lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = container_lifecycle_tests::fake_creation(&config, &agent, false).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let revision = draft(&repo, &agent, owner, "Candidate").await;
    repo.request_config_activation(agent.id, revision.revision, owner)
        .await
        .unwrap();
    assert!(matches!(
        runtime
            .prepared_container(&agent, LaunchPhase::Regular)
            .await,
        Err(AppError::Conflict(_))
    ));
    let preparation = RuntimeContainerPreparation {
        agent_id: agent.id,
        ordinal: 0,
        controller_id: runtime.controller_id,
        generation: Uuid::new_v4(),
        operation_id: Uuid::new_v4(),
        intent_sha256: "a".repeat(64),
    };
    let configuration = RuntimeConfigurationClaim {
        phase: "activation".into(),
        revision: Some(revision.revision),
        sha256: Some(
            crate::runtime_launches::snapshot_hash(
                &serde_json::to_value(&revision.snapshot).unwrap(),
            )
            .unwrap(),
        ),
    };
    assert!(
        repo.claim_container_preparation(&preparation, &configuration)
            .await
            .is_err()
    );
    assert!(
        !repo
            .has_pending_container_preparation(agent.id)
            .await
            .unwrap()
    );
    assert!(!root.join("controller/prepare-effect").exists());
    assert!(
        !root
            .join(format!("controller/{}.container-creation.json", agent.id))
            .exists()
    );
    assert_eq!(
        repo.claim_config_activation()
            .await
            .unwrap()
            .unwrap()
            .agent_id,
        agent.id
    );
    for field in ["phase", "revision", "hash"] {
        let mut changed = configuration.clone();
        match field {
            "phase" => changed.phase = "regular".into(),
            "revision" => changed.revision = Some(revision.revision + 1),
            _ => changed.sha256 = Some("b".repeat(64)),
        }
        assert!(
            repo.claim_container_preparation(&preparation, &changed)
                .await
                .is_err(),
            "{field}"
        );
        assert!(
            !repo
                .has_pending_container_preparation(agent.id)
                .await
                .unwrap()
        );
    }
    repo.claim_container_preparation(&preparation, &configuration)
        .await
        .unwrap();
    assert!(
        repo.has_pending_container_preparation(agent.id)
            .await
            .unwrap()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn stopped_container_configuration_activates_without_spawning_or_native_paths() {
    let Some((repo, agent, owner, config, root)) =
        lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = container_lifecycle_tests::fake_creation(&config, &agent, false).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let revision = draft(&repo, &agent, owner, "Stopped candidate").await;
    claim(&repo, &revision, owner).await;
    apply(&runtime, &revision).await.unwrap();
    assert_eq!(
        repo.get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        revision.revision
    );
    let yaml = tokio::fs::read_to_string(Path::new(&agent.paths.config).join("config.yaml"))
        .await
        .unwrap();
    assert!(yaml.contains("/workspace") && yaml.contains("0.0.0.0"));
    assert!(!yaml.contains(&agent.paths.workspace));
    assert_eq!(
        tokio::fs::read_to_string(Path::new(&agent.paths.config).join("SOUL.md"))
            .await
            .unwrap(),
        "Stopped candidate"
    );
    assert!(!repo.agent_is_draining(agent.id).await.unwrap());
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(!root.join("controller/start-effect").exists());
    assert!(runtime.children.lock().await.is_empty());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn container_creation_requires_dotenv_bytes_from_the_claimed_configuration_revision() {
    let Some((repo, agent, owner, config, root)) =
        lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = container_lifecycle_tests::fake_creation(&config, &agent, false).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let revision = draft(&repo, &agent, owner, "Environment snapshot candidate").await;
    claim(&repo, &revision, owner).await;
    apply(&runtime, &revision).await.unwrap();
    let envfile = Path::new(&agent.paths.config).join(".env");
    let original = tokio::fs::read(&envfile).await.unwrap();
    tokio::fs::write(&envfile, "API_SERVER_KEY=foreign\n")
        .await
        .unwrap();
    assert!(
        runtime
            .prepared_container(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert!(
        !repo
            .has_pending_container_preparation(agent.id)
            .await
            .unwrap()
    );
    assert!(!root.join("controller/prepare-calls").exists());
    tokio::fs::write(&envfile, &original).await.unwrap();
    runtime
        .prepared_container(&agent, LaunchPhase::Regular)
        .await
        .unwrap();
    let intent: Value = serde_json::from_slice(
        &tokio::fs::read(root.join(format!("controller/{}.container-creation.json", agent.id)))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        intent["environment_snapshot"]["dotenv"],
        String::from_utf8(original).unwrap()
    );
    assert_eq!(intent["configuration_revision"], revision.revision);
    assert!(!root.join("controller/start-effect").exists());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn missing_pending_intent_prevents_configuration_files_and_journal_mutation() {
    let Some((repo, agent, owner, config, root)) =
        lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = container_lifecycle_tests::fake_creation(&config, &agent, true).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let soul = Path::new(&agent.paths.config).join("SOUL.md");
    tokio::fs::write(&soul, "Original").await.unwrap();
    assert!(
        runtime
            .prepared_container(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    tokio::fs::remove_file(root.join(format!("controller/{}.container-creation.json", agent.id)))
        .await
        .unwrap();
    let revision = draft(&repo, &agent, owner, "Must not apply").await;
    claim(&repo, &revision, owner).await;
    assert!(matches!(
        apply(&runtime, &revision).await,
        Err(AppError::Unavailable(_))
    ));
    assert_eq!(tokio::fs::read_to_string(soul).await.unwrap(), "Original");
    assert!(
        !root
            .join(format!("controller/{}.activation.json", agent.id))
            .exists()
    );
    assert!(!root.join("controller/start-effect").exists());
    assert!(repo.agent_is_draining(agent.id).await.unwrap());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

async fn ready_runtime(
    repo: Arc<crate::PostgresFleetRepository>,
    agent: &Agent,
    owner: Uuid,
    config: &AppConfig,
) -> (
    LocalRuntimeSupervisor,
    AgentConfigRevision,
    tokio::task::JoinHandle<()>,
) {
    use axum::{
        Json, Router,
        http::{HeaderMap, StatusCode},
        routing::get,
    };
    let config = container_lifecycle_tests::fake_creation(config, agent, false).await;
    let private = Path::new(&config.fleet.controller_root);
    let host = tokio::net::lookup_host((std::env::var("HOSTNAME").unwrap().as_str(), 0))
        .await
        .unwrap()
        .find_map(|address| match address.ip() {
            std::net::IpAddr::V4(ip) if ip.is_private() && !ip.is_loopback() => Some(ip),
            _ => None,
        })
        .unwrap();
    let listener = tokio::net::TcpListener::bind((host, 0)).await.unwrap();
    repo.db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agents SET api_port=$2 WHERE id=$1",
            [
                agent.id.into(),
                i32::from(listener.local_addr().unwrap().port()).into(),
            ],
        ))
        .await
        .unwrap();
    let agent = repo.get_agent(agent.id).await.unwrap();
    tokio::fs::write(private.join("endpoint-host"), host.to_string())
        .await
        .unwrap();
    tokio::fs::write(private.join("known-start"), "1")
        .await
        .unwrap();
    let token = format!(
        "Bearer {}",
        crate::agent_runtime_token(&config, agent.id).unwrap()
    );
    let expected = token.clone();
    let soul = PathBuf::from(&agent.paths.config).join("SOUL.md");
    let routes = Router::new().route("/health", get(move |headers: HeaderMap| {
        let expected = expected.clone();
        async move { if headers.get("authorization").and_then(|v| v.to_str().ok()) == Some(expected.as_str()) { StatusCode::OK } else { StatusCode::UNAUTHORIZED } }
    })).route("/v1/capabilities", get(move |headers: HeaderMap| {
        let token = token.clone(); let soul = soul.clone();
        async move {
            let valid = headers.get("authorization").and_then(|v| v.to_str().ok()) == Some(token.as_str());
            let ready = tokio::fs::read_to_string(soul).await.unwrap() != "Reject candidate";
            (if valid { StatusCode::OK } else { StatusCode::UNAUTHORIZED }, Json(json!({"features":{"run_status":ready,"run_events_sse":ready,"run_stop":ready}})))
        }
    }));
    let server = tokio::spawn(async move {
        axum::serve(listener, routes).await.unwrap();
    });
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let initial = draft(&repo, &agent, owner, "Initial active").await;
    claim(&repo, &initial, owner).await;
    apply(&runtime, &initial).await.unwrap();
    assert_eq!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .unwrap()
            .status,
        AgentStatus::Running
    );
    (runtime, initial, server)
}

async fn history(
    repo: &crate::PostgresFleetRepository,
    agent: Uuid,
) -> Vec<(Uuid, String, String)> {
    repo.db.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT id,state,binding->>'phase' AS phase FROM runtime_launches WHERE agent_id=$1 ORDER BY created_at,id",
        [agent.into()])).await.unwrap().into_iter().map(|row| (row.try_get("", "id").unwrap(), row.try_get("", "state").unwrap(), row.try_get("", "phase").unwrap())).collect()
}

#[tokio::test]
async fn running_configuration_replaces_only_confirmed_namespace_with_fresh_generation() {
    let Some((repo, agent, owner, config, root)) =
        lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let (runtime, initial, server) = ready_runtime(repo.clone(), &agent, owner, &config).await;
    let original = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    let revision = draft(&repo, &agent, owner, "New active").await;
    claim(&repo, &revision, owner).await;
    assert_eq!(
        repo.get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        initial.revision
    );
    apply(&runtime, &revision).await.unwrap();
    let replacement = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(replacement.binding.id, original.binding.id);
    assert_eq!(replacement.binding.phase, "activation");
    assert_eq!(
        replacement.binding.configuration_revision,
        Some(revision.revision)
    );
    assert_eq!(
        history(&repo, agent.id).await,
        vec![
            (
                original.binding.id,
                "gateway_exited".into(),
                "regular".into()
            ),
            (
                replacement.binding.id,
                "gateway_started".into(),
                "activation".into()
            )
        ]
    );
    assert_eq!(
        repo.get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        revision.revision
    );
    assert!(!repo.agent_is_draining(agent.id).await.unwrap());
    assert!(runtime.children.lock().await.is_empty());
    runtime.stop_locked(&agent).await.unwrap();
    server.abort();
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn failed_candidate_readiness_restores_previous_files_and_new_rollback_generation() {
    let Some((repo, agent, owner, config, root)) =
        lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let (runtime, initial, server) = ready_runtime(repo.clone(), &agent, owner, &config).await;
    let original = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    let revision = draft(&repo, &agent, owner, "Reject candidate").await;
    claim(&repo, &revision, owner).await;
    assert!(matches!(
        apply(&runtime, &revision).await,
        Err(AppError::Validation(_))
    ));
    let launches = history(&repo, agent.id).await;
    assert_eq!(launches.len(), 3);
    assert_eq!(
        launches[0],
        (
            original.binding.id,
            "gateway_exited".into(),
            "regular".into()
        )
    );
    assert_eq!(
        (&launches[1].1, &launches[1].2),
        (&"gateway_exited".into(), &"activation".into())
    );
    assert_eq!(
        (&launches[2].1, &launches[2].2),
        (&"gateway_started".into(), &"rollback".into())
    );
    assert_ne!(launches[1].0, launches[2].0);
    assert_eq!(
        repo.get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        initial.revision
    );
    let failed = repo
        .get_config_revision(agent.id, revision.revision)
        .await
        .unwrap();
    assert_eq!(failed.state, "failed");
    assert!(!repo.agent_is_draining(agent.id).await.unwrap());
    assert_eq!(
        tokio::fs::read_to_string(Path::new(&agent.paths.config).join("SOUL.md"))
            .await
            .unwrap(),
        "Initial active"
    );
    runtime.stop_locked(&agent).await.unwrap();
    server.abort();
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn unknown_replacement_start_keeps_candidate_files_and_drain_without_rollback_spawn() {
    let Some((repo, agent, owner, config, root)) =
        lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let (runtime, initial, server) = ready_runtime(repo.clone(), &agent, owner, &config).await;
    tokio::fs::write(root.join("controller/hold-next-start"), "1")
        .await
        .unwrap();
    let revision = draft(&repo, &agent, owner, "Uncertain candidate").await;
    claim(&repo, &revision, owner).await;
    assert!(matches!(
        apply(&runtime, &revision).await,
        Err(AppError::Unavailable(_))
    ));
    assert_eq!(
        repo.get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        initial.revision
    );
    assert!(repo.agent_is_draining(agent.id).await.unwrap());
    assert_eq!(
        tokio::fs::read_to_string(Path::new(&agent.paths.config).join("SOUL.md"))
            .await
            .unwrap(),
        "Uncertain candidate"
    );
    let launches = history(&repo, agent.id).await;
    assert_eq!(launches.len(), 2);
    assert_eq!(launches[0].1, "gateway_exited");
    assert_eq!(launches[1].1, "claimed");
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/start-calls"))
            .await
            .unwrap()
            .lines()
            .count(),
        2
    );
    assert!(
        root.join(format!("controller/{}.activation.json", agent.id))
            .exists()
    );
    assert!(runtime.children.lock().await.is_empty());
    server.abort();
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn unknown_replacement_prepare_retains_candidate_and_never_spawns_rollback() {
    let Some((repo, agent, owner, config, root)) =
        lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let (runtime, initial, server) = ready_runtime(repo.clone(), &agent, owner, &config).await;
    tokio::fs::write(root.join("controller/prepare-unknown"), "1")
        .await
        .unwrap();
    let revision = draft(&repo, &agent, owner, "Uncertain preparation").await;
    claim(&repo, &revision, owner).await;
    assert!(matches!(
        apply(&runtime, &revision).await,
        Err(AppError::Unavailable(_))
    ));
    assert_eq!(
        repo.get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        initial.revision
    );
    assert!(repo.agent_is_draining(agent.id).await.unwrap());
    assert!(
        repo.has_pending_container_preparation(agent.id)
            .await
            .unwrap()
    );
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        tokio::fs::read_to_string(Path::new(&agent.paths.config).join("SOUL.md"))
            .await
            .unwrap(),
        "Uncertain preparation"
    );
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/start-calls"))
            .await
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert!(
        root.join(format!("controller/{}.activation.json", agent.id))
            .exists()
    );
    server.abort();
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn foreign_controller_cannot_stop_or_edit_original_container_for_activation() {
    let Some((repo, agent, owner, config, root)) =
        lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let (original, initial, server) = ready_runtime(repo.clone(), &agent, owner, &config).await;
    let foreign = lifecycle_tests::supervisor(original.config.clone(), repo.clone());
    let revision = draft(&repo, &agent, owner, "Foreign must not apply").await;
    claim(&repo, &revision, owner).await;
    assert!(matches!(
        apply(&foreign, &revision).await,
        Err(AppError::Unavailable(_))
    ));
    assert_eq!(
        repo.get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .unwrap()
            .revision,
        initial.revision
    );
    assert_eq!(
        tokio::fs::read_to_string(Path::new(&agent.paths.config).join("SOUL.md"))
            .await
            .unwrap(),
        "Initial active"
    );
    assert!(
        !root
            .join(format!("controller/{}.activation.json", agent.id))
            .exists()
    );
    let launches = history(&repo, agent.id).await;
    assert_eq!(launches.len(), 1);
    assert_eq!(launches[0].1, "gateway_started");
    original.stop_locked(&agent).await.unwrap();
    server.abort();
    tokio::fs::remove_dir_all(root).await.unwrap();
}
