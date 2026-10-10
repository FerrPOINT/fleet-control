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

#[derive(Debug, Default)]
struct ActivationPollState {
    polls: u64,
    first_poll_after_ms: Option<u128>,
    last_poll_started_after_ms: Option<u128>,
    last_poll_finished_after_ms: Option<u128>,
    longest_poll_ms: u128,
    in_poll: bool,
    returned: bool,
}

struct ActivationPollTrace {
    queued_at: std::time::Instant,
    state: std::sync::Mutex<ActivationPollState>,
}

impl ActivationPollTrace {
    fn new() -> Self {
        Self {
            queued_at: std::time::Instant::now(),
            state: std::sync::Mutex::new(ActivationPollState::default()),
        }
    }

    fn before_poll(&self) -> std::time::Instant {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.queued_at).as_millis();
        let mut state = self.state.lock().unwrap();
        state.polls += 1;
        state.first_poll_after_ms.get_or_insert(elapsed);
        state.last_poll_started_after_ms = Some(elapsed);
        state.in_poll = true;
        now
    }

    fn after_poll(&self, started: std::time::Instant, returned: bool) {
        let mut state = self.state.lock().unwrap();
        state.last_poll_finished_after_ms = Some(self.queued_at.elapsed().as_millis());
        state.longest_poll_ms = state.longest_poll_ms.max(started.elapsed().as_millis());
        state.in_poll = false;
        state.returned = returned;
    }

    fn snapshot(&self) -> String {
        format!("{:?}", self.state.lock().unwrap())
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn separate_supervisors_cannot_activate_until_original_journal_is_settled() {
    let Some((repo, agent, owner, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let revision = revision(&repo, &agent, owner).await;
    repo.request_config_activation(agent.id, revision.revision, owner)
        .await
        .unwrap();
    let claimed = repo.claim_config_activation().await.unwrap().unwrap();
    let first = supervisor(config.clone(), repo.clone());
    let second = supervisor(config, repo.clone());
    let controller = PathBuf::from(&first.config.fleet.controller_root);
    let soul = PathBuf::from(&agent.paths.config).join("SOUL.md");
    tokio::fs::write(&soul, b"original-effective-soul")
        .await
        .unwrap();
    let original = activation_lock::ActivationLock::acquire(&controller, agent.id)
        .await
        .unwrap();
    let mut blocked_journal = None;
    assert!(matches!(
        second
            .apply_config_revision(&claimed, &mut blocked_journal)
            .await,
        Err(AppError::Unavailable(_))
    ));
    assert!(blocked_journal.is_none());
    assert_eq!(
        tokio::fs::read(&soul).await.unwrap(),
        b"original-effective-soul"
    );
    assert!(!journal_path(&first, &agent).exists());
    assert!(repo.agent_is_draining(agent.id).await.unwrap());
    drop(original);
    let mut journal = None;
    first
        .apply_config_revision(&claimed, &mut journal)
        .await
        .unwrap();
    let journal = journal.unwrap();
    assert!(
        journal
            .verified_backups()
            .await
            .unwrap()
            .contains(&(soul.clone(), Some(b"original-effective-soul".to_vec())))
    );
    assert!(
        activation_lock::ActivationLock::acquire(&controller, agent.id)
            .await
            .is_err()
    );
    repo.finish_config_activation(agent.id, claimed.revision, None, true)
        .await
        .unwrap();
    assert!(!repo.agent_is_draining(agent.id).await.unwrap());
    assert!(
        activation_lock::ActivationLock::acquire(&controller, agent.id)
            .await
            .is_err()
    );
    journal.acknowledge().await.unwrap();
    let successor = activation_lock::ActivationLock::acquire(&controller, agent.id)
        .await
        .unwrap();
    drop(successor);
    assert_eq!(
        tokio::fs::read(&soul).await.unwrap(),
        b"Updated lifecycle SOUL"
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
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
    let planning_started = std::time::Instant::now();
    let plan = crate::configuration_files(&agent, &config, &revision)
        .await
        .unwrap();
    let planning_ms = planning_started.elapsed().as_millis();
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
    let event_lock_pid: i32 = transaction
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT pg_backend_pid() AS pid".to_owned(),
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    let supervisor = supervisor(config, repo.clone());
    let worker = supervisor.clone();
    let trace = Arc::new(ActivationPollTrace::new());
    let worker_trace = trace.clone();
    eprintln!(
        "activation diagnostic setup: agent={}, revision={}, renderer={}, observer={}, planned_files={}, planning_ms={}, event_lock_pid={}",
        agent.id,
        revision.revision,
        revision.snapshot.renderer_version,
        revision
            .snapshot
            .config
            .config_json
            .get("fleet_request_observer")
            .is_some(),
        plan.len(),
        planning_ms,
        event_lock_pid,
    );
    let mut activation = tokio::spawn(async move {
        let mut journal = None;
        let result = {
            let mut apply = std::pin::pin!(worker.apply_config_revision(&revision, &mut journal));
            std::future::poll_fn(|context| {
                let started = worker_trace.before_poll();
                let result = std::future::Future::poll(apply.as_mut(), context);
                worker_trace.after_poll(started, result.is_ready());
                result
            })
            .await
        };
        (result, journal)
    });
    let soul = PathBuf::from(&agent.paths.config).join("SOUL.md");
    let file_readback = timeout(Duration::from_secs(5), async {
        loop {
            if activation.is_finished() {
                let (result, _journal) = (&mut activation).await.unwrap();
                panic!(
                    "activation exited before locked event persistence: {result:?}; poll_trace={}",
                    trace.snapshot(),
                );
            }
            if tokio::fs::read(&soul).await.ok().as_deref() == Some(b"Updated lifecycle SOUL") {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    if file_readback.is_err() {
        let poll_trace = trace.snapshot();
        let lifecycle_lock = match supervisor.lifecycle_locks.try_lock() {
            Err(_) => "registry_busy",
            Ok(locks) => match locks.get(&agent.id) {
                None => "agent_lock_not_created",
                Some(lock) => match lock.try_lock() {
                    Ok(_) => "agent_lock_free",
                    Err(_) => "agent_lock_held",
                },
            },
        };
        let controller_lock_exists = PathBuf::from(&supervisor.config.fleet.controller_root)
            .join(format!("{}.activation.lock", agent.id))
            .exists();
        let file_presence = plan
            .iter()
            .map(|(path, _)| {
                (
                    path.strip_prefix(&agent.paths.config)
                        .unwrap()
                        .to_path_buf(),
                    path.exists(),
                )
            })
            .collect::<Vec<_>>();
        eprintln!(
            "activation deadline snapshot: task_finished={}, poll_trace={}, lifecycle_lock={}, controller_lock_exists={}, journal_exists={}, file_presence={file_presence:?}",
            activation.is_finished(),
            poll_trace,
            lifecycle_lock,
            controller_lock_exists,
            journal_path(&supervisor, &agent).exists(),
        );
        // Use the independent diagnostic pool, not an acquisition on the worker's pool.
        // Report classifications only: raw SQL can contain private configuration values.
        let postgres = timeout(Duration::from_secs(2), db.query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT jsonb_build_object(
                'activity', (SELECT COALESCE(jsonb_agg(to_jsonb(a)), '[]'::jsonb) FROM (
                    SELECT pid, state, wait_event_type, wait_event,
                        pg_blocking_pids(pid) AS blocking_pids,
                        xact_start IS NOT NULL AS has_transaction,
                        CASE WHEN query ILIKE '%agent_events%' THEN 'agent_events'
                             WHEN query ILIKE '%runtime_launch%' THEN 'runtime_launch'
                             WHEN query ILIKE '%agent_config%' THEN 'agent_config'
                             WHEN query ILIKE '%agents%' THEN 'agent_lookup'
                             ELSE 'other' END AS query_phase
                    FROM pg_stat_activity
                    WHERE datname=current_database() AND pid<>pg_backend_pid()
                    ORDER BY (state='active') DESC, pid LIMIT 64
                ) a),
                'event_locks', (SELECT COALESCE(jsonb_agg(to_jsonb(l)), '[]'::jsonb) FROM (
                    SELECT locks.pid, locks.mode, locks.granted
                    FROM pg_locks locks JOIN pg_class relation ON relation.oid=locks.relation
                    WHERE locks.database=(SELECT oid FROM pg_database WHERE datname=current_database())
                        AND relation.relname='agent_events'
                    ORDER BY locks.granted, locks.pid LIMIT 64
                ) l)) AS snapshot".to_owned(),
        ))).await;
        let postgres = match postgres {
            Err(_) => "diagnostic_query_timeout_2s".into(),
            Ok(Err(_)) => "diagnostic_query_failed".into(),
            Ok(Ok(None)) => "diagnostic_query_missing_row".into(),
            Ok(Ok(Some(row))) => match row.try_get::<Value>("", "snapshot") {
                Ok(value) => value.to_string(),
                Err(_) => "diagnostic_query_invalid_snapshot".into(),
            },
        };
        panic!(
            "activation readback deadline (original 5s): poll_trace={poll_trace}, lifecycle_lock={lifecycle_lock}, controller_lock_exists={controller_lock_exists}, journal_exists={}, soul_exists={}, event_lock_pid={event_lock_pid}, postgres={postgres}",
            journal_path(&supervisor, &agent).exists(),
            soul.exists(),
        );
    }
    eprintln!(
        "activation SOUL observed under original 5s deadline: poll_trace={}, journal_exists={}",
        trace.snapshot(),
        journal_path(&supervisor, &agent).exists(),
    );
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
