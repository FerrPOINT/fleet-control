use super::*;
use domain::{AgentRole, CreateAgentRequest, SdlcRole, UpdateAgentConfigRequest};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait};
use std::path::PathBuf;
use tokio::time::timeout;

pub(super) async fn fixture(
    kind: AgentKind,
) -> Option<(
    Arc<crate::PostgresFleetRepository>,
    Agent,
    Uuid,
    Arc<AppConfig>,
    PathBuf,
)> {
    let Ok(url) = std::env::var("FLEET_TEST_DATABASE_URL") else {
        eprintln!("FLEET_TEST_DATABASE_URL not configured; lifecycle PostgreSQL tests skipped");
        return None;
    };
    let database = shared::DatabaseConfig {
        url,
        max_connections: 10,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    };
    crate::run_migrations(database.clone()).await.unwrap();
    let db = crate::connect_database(database).await.unwrap();
    let owner = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES ($1,$2,$3,'Lifecycle test','disabled',false,'user')",
        [owner.into(), format!("{owner}@example.test").into(), owner.to_string().into()]
    )).await.unwrap();
    let repo = Arc::new(crate::PostgresFleetRepository::new(db));
    repo.ensure_runtime_templates().await.unwrap();
    let root = std::env::temp_dir().join(format!("fleet-lifecycle-{}", Uuid::new_v4()));
    let mut config = AppConfig::default();
    config.fleet.agents_root = root.join("agents").to_string_lossy().into_owned();
    config.fleet.controller_root = root.join("controller").to_string_lossy().into_owned();
    tokio::fs::create_dir_all(&config.fleet.controller_root)
        .await
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(
            &config.fleet.controller_root,
            std::fs::Permissions::from_mode(0o700),
        )
        .await
        .unwrap();
    }
    config.fleet.runtime_token_secret = "lifecycle-test-only".into();
    let agent = repo
        .create_agent(
            CreateAgentRequest {
                kind,
                product_role: AgentProductRole::Executor,
                role: AgentRole::Developer,
                sdlc_role: Some(SdlcRole::Developer),
                display_name: "Lifecycle test".into(),
                description: None,
                namespace_id: None,
                namespace_name: None,
                workflow_id: None,
                workflow_name: None,
                executor_ids: vec![],
            },
            &config,
        )
        .await
        .unwrap();
    repo.update_agent_status(agent.id, AgentStatus::Ready)
        .await
        .unwrap();
    let agent = repo.get_agent(agent.id).await.unwrap();
    for skill in repo.list_agent_skills(agent.id).await.unwrap() {
        repo.update_agent_skill(
            agent.id,
            skill.name,
            domain::UpdateSkillRequest {
                state: domain::SkillState::Disabled,
                content: None,
            },
        )
        .await
        .unwrap();
    }
    tokio::fs::create_dir_all(&agent.paths.config)
        .await
        .unwrap();
    tokio::fs::write(
        PathBuf::from(&agent.paths.config)
            .parent()
            .unwrap()
            .join(".fleet-agent.json"),
        serde_json::to_vec(
            &json!({"id":agent.id,"name":agent.name,"ordinal":agent.ordinal,"kind":"hermes"}),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    Some((repo, agent, owner, Arc::new(config), root))
}

pub(super) async fn revision(
    repo: &crate::PostgresFleetRepository,
    agent: &Agent,
    owner: Uuid,
) -> domain::AgentConfigRevision {
    let revision = repo
        .create_config_revision(
            agent.id,
            UpdateAgentConfigRequest {
                config_json: json!({"model":"test-model"}),
                soul_md: "Updated lifecycle SOUL".into(),
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

pub(super) fn supervisor(
    config: Arc<AppConfig>,
    repo: Arc<crate::PostgresFleetRepository>,
) -> LocalRuntimeSupervisor {
    let (events, _) = broadcast::channel(32);
    LocalRuntimeSupervisor {
        config,
        repo: repo.clone(),
        children: Arc::new(Mutex::new(HashMap::new())),
        controller_id: Uuid::new_v4(),
        launches: Arc::new(Mutex::new(HashMap::new())),
        lifecycle_locks: Arc::new(Mutex::new(HashMap::new())),
        recovery_worker: Arc::new(super::controller_recovery_worker::RecoveryWorker::default()),
        client: reqwest::Client::new(),
        events,
        alerts: Arc::new(app::RepositoryAlertService { repository: repo }),
    }
}

fn journal_path(supervisor: &LocalRuntimeSupervisor, agent: &Agent) -> PathBuf {
    activation_journal::controller_journal_path(
        std::path::Path::new(&supervisor.config.fleet.controller_root),
        agent.id,
    )
}

#[tokio::test]
async fn delayed_start_rechecks_drain_after_acquiring_lifecycle_lock() {
    let Some((repo, agent, owner, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let revision = revision(&repo, &agent, owner).await;
    let supervisor = supervisor(config, repo.clone());
    assert!(!repo.agent_is_draining(agent.id).await.unwrap());
    let lock = supervisor.lifecycle_lock(agent.id).await;
    let guard = lock.lock().await;
    let worker = supervisor.clone();
    let stale = agent.clone();
    let start = tokio::spawn(async move { worker.start(&stale).await });
    sleep(Duration::from_millis(40)).await;
    assert!(!start.is_finished());
    repo.request_config_activation(agent.id, revision.revision, owner)
        .await
        .unwrap();
    repo.claim_config_activation().await.unwrap().unwrap();
    drop(guard);
    assert!(matches!(
        timeout(Duration::from_secs(2), start)
            .await
            .unwrap()
            .unwrap(),
        Err(AppError::Conflict(_))
    ));
    assert!(!supervisor.children.lock().await.contains_key(&agent.id));
    assert!(!journal_path(&supervisor, &agent).exists());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn activation_keeps_lifecycle_lock_through_file_readback_and_event_persistence() {
    let Some((repo, agent, owner, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let revision = revision(&repo, &agent, owner).await;
    crate::configuration_files(&agent, &config, &revision)
        .await
        .unwrap();
    repo.request_config_activation(agent.id, revision.revision, owner)
        .await
        .unwrap();
    let claimed = repo.claim_config_activation().await.unwrap().unwrap();
    assert_eq!(claimed.agent_id, agent.id);
    assert_eq!(claimed.revision, revision.revision);
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let transaction = db.begin().await.unwrap();
    transaction
        .execute(Statement::from_string(
            DatabaseBackend::Postgres,
            "LOCK TABLE agent_events IN SHARE MODE".to_owned(),
        ))
        .await
        .unwrap();
    let supervisor = supervisor(config, repo.clone());
    let worker = supervisor.clone();
    let mut activation = tokio::spawn(async move {
        let mut journal = None;
        let result = worker.apply_config_revision(&revision, &mut journal).await;
        (result, journal)
    });
    let soul = PathBuf::from(&agent.paths.config).join("SOUL.md");
    let file_readback = timeout(Duration::from_secs(5), async {
        loop {
            if activation.is_finished() {
                let (result, _journal) = (&mut activation).await.unwrap();
                panic!("activation exited before locked event persistence: {result:?}");
            }
            if tokio::fs::read(&soul).await.ok().as_deref() == Some(b"Updated lifecycle SOUL") {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    if file_readback.is_err() {
        let waits = repo
            .db
            .query_all(Statement::from_string(
                DatabaseBackend::Postgres,
                "SELECT state, wait_event_type, wait_event FROM pg_stat_activity
             WHERE datname=current_database() AND pid<>pg_backend_pid()"
                    .to_owned(),
            ))
            .await
            .unwrap();
        let waits = waits
            .into_iter()
            .map(|row| {
                (
                    row.try_get::<Option<String>>("", "state").unwrap(),
                    row.try_get::<Option<String>>("", "wait_event_type")
                        .unwrap(),
                    row.try_get::<Option<String>>("", "wait_event").unwrap(),
                )
            })
            .collect::<Vec<_>>();
        panic!(
            "activation readback deadline: journal_exists={}, soul_exists={}, postgres_waits={waits:?}",
            journal_path(&supervisor, &agent).exists(),
            soul.exists(),
        );
    }
    assert!(!activation.is_finished());
    let worker = supervisor.clone();
    let stale = agent.clone();
    let start = tokio::spawn(async move { worker.start(&stale).await });
    sleep(Duration::from_millis(60)).await;
    assert!(
        !start.is_finished(),
        "start must wait for the entire activation/file phase"
    );
    assert!(!supervisor.children.lock().await.contains_key(&agent.id));
    transaction.commit().await.unwrap();
    let (result, journal) = timeout(Duration::from_secs(5), activation)
        .await
        .unwrap()
        .unwrap();
    result.unwrap();
    assert!(journal.is_some());
    assert!(matches!(
        timeout(Duration::from_secs(2), start)
            .await
            .unwrap()
            .unwrap(),
        Err(AppError::Conflict(_))
    ));
    // No DB acknowledgement: the sensitive recovery journal must still exist.
    drop(journal);
    assert!(journal_path(&supervisor, &agent).exists());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn failed_directory_barrier_preserves_activation_drain_journal_and_effective_head() {
    let Some((repo, agent, owner, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let revision = revision(&repo, &agent, owner).await;
    repo.request_config_activation(agent.id, revision.revision, owner)
        .await
        .unwrap();
    let claimed = repo.claim_config_activation().await.unwrap().unwrap();
    assert_eq!(claimed.agent_id, agent.id);
    let supervisor = supervisor(config, repo.clone());
    let mut journal = None;
    let result = crate::configuration_disk::SYNC_FAILURE
        .scope(
            (PathBuf::from(&agent.paths.config), std::cell::Cell::new(2)),
            supervisor.apply_config_revision(&claimed, &mut journal),
        )
        .await;
    assert!(matches!(result, Err(AppError::Unavailable(_))));
    assert!(journal.is_some());
    assert!(
        PathBuf::from(&agent.paths.config)
            .join("config.yaml")
            .is_file()
    );
    repo.finish_config_activation(
        agent.id,
        claimed.revision,
        Some(crate::redact_text(&result.unwrap_err().to_string())),
        false,
    )
    .await
    .unwrap();
    assert!(repo.agent_is_draining(agent.id).await.unwrap());
    assert!(
        repo.get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    let observed = repo
        .get_config_revision(agent.id, claimed.revision)
        .await
        .unwrap();
    assert_ne!(observed.state, "active");
    assert!(!supervisor.children.lock().await.contains_key(&agent.id));
    drop(journal);
    assert!(journal_path(&supervisor, &agent).is_file());
    assert!(repo.claim_config_activation().await.unwrap().is_none());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn legacy_agent_journal_keeps_drain_and_blocks_before_configuration_effects() {
    let Some((repo, agent, owner, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let revision = revision(&repo, &agent, owner).await;
    let legacy = PathBuf::from(&agent.paths.config).join(".fleet-activation-journal.json");
    tokio::fs::write(&legacy, b"legacy-sensitive-recovery")
        .await
        .unwrap();
    let soul = PathBuf::from(&agent.paths.config).join("SOUL.md");
    tokio::fs::write(&soul, b"previous-soul").await.unwrap();
    repo.request_config_activation(agent.id, revision.revision, owner)
        .await
        .unwrap();
    let claimed = repo.claim_config_activation().await.unwrap().unwrap();
    let supervisor = supervisor(config, repo.clone());
    let mut journal = None;
    let outcome = supervisor
        .apply_config_revision(&claimed, &mut journal)
        .await;
    assert!(matches!(outcome, Err(AppError::Unavailable(_))));
    assert!(journal.is_none());
    assert!(!journal_path(&supervisor, &agent).exists());
    assert_eq!(
        tokio::fs::read(&legacy).await.unwrap(),
        b"legacy-sensitive-recovery"
    );
    assert_eq!(tokio::fs::read(&soul).await.unwrap(), b"previous-soul");
    repo.finish_config_activation(
        agent.id,
        claimed.revision,
        Some("legacy journal reconciliation required".into()),
        false,
    )
    .await
    .unwrap();
    assert!(repo.agent_is_draining(agent.id).await.unwrap());
    assert!(
        repo.get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(!supervisor.children.lock().await.contains_key(&agent.id));
    assert!(repo.claim_config_activation().await.unwrap().is_none());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn missing_java_jar_does_not_publish_an_untracked_starting_runtime() {
    let Some((repo, agent, _, config, root)) = fixture(AgentKind::JavaAgent).await else {
        return;
    };
    let supervisor = supervisor(config, repo.clone());
    for _ in 0..2 {
        assert!(
            supervisor
                .start(&agent)
                .await
                .unwrap_err()
                .to_string()
                .contains("not provisioned")
        );
        let observed = repo.get_agent(agent.id).await.unwrap();
        assert_eq!(observed.status, AgentStatus::Ready);
        assert_eq!(observed.runtime.pid, agent.runtime.pid);
        assert_eq!(observed.runtime.desired_state, agent.runtime.desired_state);
        assert_eq!(observed.runtime.started_at, agent.runtime.started_at);
        assert!(!supervisor.children.lock().await.contains_key(&agent.id));
    }
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn stopped_child_with_failed_metadata_commit_keeps_activation_journal_and_drain() {
    let Some((repo, agent, owner, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let revision = revision(&repo, &agent, owner).await;
    let soul = PathBuf::from(&agent.paths.config).join("SOUL.md");
    tokio::fs::write(&soul, "Previous lifecycle SOUL")
        .await
        .unwrap();
    let supervisor = supervisor(config, repo.clone());
    let child = Command::new("sleep")
        .arg("30")
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let pid = child.id().unwrap() as i32;
    supervisor.children.lock().await.insert(agent.id, child);
    repo.update_runtime_state(
        agent.id,
        RuntimeStatePatch {
            status: AgentStatus::Running,
            desired_state: DesiredState::Running,
            pid: Some(pid),
            health_status: None,
            health_detail: None,
            last_capabilities_json: None,
            startup_command_redacted: None,
            started_at: None,
            stopped_at: None,
        },
    )
    .await
    .unwrap();
    repo.request_config_activation(agent.id, revision.revision, owner)
        .await
        .unwrap();
    repo.claim_config_activation().await.unwrap().unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let name = format!("fleet_stop_failure_{}", agent.id.simple());
    db.execute(Statement::from_string(
        DatabaseBackend::Postgres,
        format!(
            "CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
         IF NEW.agent_id='{id}'::uuid AND NEW.desired_state='stopped' THEN
           RAISE EXCEPTION 'fixture metadata commit rejected'; END IF; RETURN NEW; END $$",
            id = agent.id
        ),
    ))
    .await
    .unwrap();
    db.execute(Statement::from_string(DatabaseBackend::Postgres, format!(
        "CREATE TRIGGER {name} BEFORE UPDATE ON agent_runtime FOR EACH ROW EXECUTE FUNCTION {name}()"
    ))).await.unwrap();
    let mut journal = None;
    let applied = supervisor
        .apply_config_revision(&revision, &mut journal)
        .await;
    db.execute(Statement::from_string(
        DatabaseBackend::Postgres,
        format!("DROP TRIGGER {name} ON agent_runtime"),
    ))
    .await
    .unwrap();
    db.execute(Statement::from_string(
        DatabaseBackend::Postgres,
        format!("DROP FUNCTION {name}()"),
    ))
    .await
    .unwrap();
    assert!(matches!(applied, Err(AppError::Unavailable(_))));
    assert!(journal.is_some());
    assert!(!supervisor.children.lock().await.contains_key(&agent.id));
    assert_eq!(
        tokio::fs::read_to_string(&soul).await.unwrap(),
        "Previous lifecycle SOUL"
    );
    let observed = repo.get_agent(agent.id).await.unwrap();
    assert_eq!(observed.status, AgentStatus::Running);
    assert_eq!(observed.runtime.pid, Some(pid));
    repo.finish_config_activation(
        agent.id,
        revision.revision,
        Some("metadata reconciliation required".into()),
        false,
    )
    .await
    .unwrap();
    assert!(repo.agent_is_draining(agent.id).await.unwrap());
    drop(journal);
    assert!(journal_path(&supervisor, &agent).exists());
    tokio::fs::remove_dir_all(root).await.unwrap();
}
