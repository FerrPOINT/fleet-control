use super::*;
use app::RuntimeSupervisor;
use domain::{PmExecutionIdentity, PmRunRecord, PmRunReservation, PmRuntimeStatus};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, TransactionTrait};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::{sync::Notify, time::timeout};

struct Fixture {
    repo: Arc<crate::PostgresFleetRepository>,
    runtime: LocalRuntimeSupervisor,
    agent: Agent,
    record: PmRunRecord,
    mode: Arc<AtomicUsize>,
    reads: Arc<AtomicUsize>,
    entered: Arc<Notify>,
    release: Arc<Notify>,
    server: tokio::task::JoinHandle<()>,
    root: std::path::PathBuf,
}

impl Fixture {
    async fn finish(self) {
        self.server.abort();
        if let Some(mut child) = self.runtime.children.lock().await.remove(&self.agent.id) {
            let _ = child.start_kill();
            child.wait().await.unwrap();
        }
        tokio::fs::remove_dir_all(self.root).await.unwrap();
    }

    async fn held(&self) {
        assert!(
            self.repo
                .get_pm_run(self.record.reservation.session_run_id)
                .await
                .unwrap()
                .terminal_status
                .is_none()
        );
        assert_eq!(
            self.repo
                .get_session_agent_run(self.record.reservation.session_run_id)
                .await
                .unwrap()
                .state,
            SessionRunState::Running
        );
        assert!(
            self.repo
                .list_audit_log(app::AuditLogFilter {
                    action: Some("pm.run.terminal_verified".into()),
                    entity_id: Some(self.record.reservation.session_run_id.to_string()),
                    ..Default::default()
                })
                .await
                .unwrap()
                .is_empty()
        );
    }

    async fn observation_snapshot(&self) -> Value {
        self.repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT jsonb_build_object('observed_at',p.observed_at,'updated_at',r.updated_at,
                'events',(SELECT count(*) FROM session_events WHERE session_id=$2),
                'cursor',(SELECT max(sequence) FROM session_events WHERE session_id=$2)) AS snapshot
             FROM pm_run_bindings p JOIN session_agent_runs r ON r.id=p.session_run_id WHERE r.id=$1",
            [self.record.reservation.session_run_id.into(),self.record.reservation.session_id.into()]))
            .await.unwrap().unwrap().try_get("","snapshot").unwrap()
    }
}

async fn fixture() -> Option<Fixture> {
    fixture_with_pool(10).await
}

async fn fixture_with_pool(max_connections: u32) -> Option<Fixture> {
    let (repo, agent, owner, config, root) = lifecycle_tests::fixture(AgentKind::Hermes).await?;
    drop(repo);
    let mut options =
        sea_orm::ConnectOptions::new(std::env::var("FLEET_TEST_DATABASE_URL").unwrap());
    options
        .max_connections(max_connections)
        .min_connections(1)
        .acquire_timeout(Duration::from_secs(3));
    let repo = Arc::new(crate::PostgresFleetRepository::new(
        sea_orm::Database::connect(options).await.unwrap(),
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let subject = Uuid::new_v4().to_string();
    repo.db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE users SET central_sub=$2 WHERE id=$1",
            [owner.into(), subject.clone().into()],
        ))
        .await
        .unwrap();
    repo.db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agents SET api_port=$2,sdlc_role='project_manager' WHERE id=$1",
            [agent.id.into(), i32::from(port).into()],
        ))
        .await
        .unwrap();
    let agent = repo.get_agent(agent.id).await.unwrap();
    let runtime = lifecycle_tests::supervisor(config.clone(), repo.clone());
    let mut command = Command::new("sleep");
    command.arg("60").kill_on_drop(true);
    runtime
        .prepare_native_launch(&agent, &command, LaunchPhase::Regular)
        .await
        .unwrap();
    let child = command.spawn().unwrap();
    let pid = i32::try_from(child.id().unwrap()).unwrap();
    runtime.children.lock().await.insert(agent.id, child);
    runtime
        .record_native_spawn(agent.id, Some(pid))
        .await
        .unwrap();
    repo.update_agent_status(agent.id, AgentStatus::Running)
        .await
        .unwrap();
    let agent = repo.get_agent(agent.id).await.unwrap();
    let session = repo
        .create_session(
            domain::CreateSessionRequest {
                primary_agent_id: Some(agent.id),
                agent_id: None,
                title: "Original PM callback".into(),
                task_key: None,
                leader_agent_id: None,
                parent_session_id: None,
                namespace_id: None,
                idempotency_key: Some(Uuid::new_v4().to_string()),
            },
            owner,
        )
        .await
        .unwrap();
    let task = Uuid::new_v4();
    let project = Uuid::new_v4();
    repo.bind_task_chat(
        session.id,
        domain::TaskChatBinding {
            tracker_instance_id: "pm-original-fixture".into(),
            project_id: project,
            task_id: task,
            root_task_id: task,
            agent_id: agent.id,
            owner_subject: subject,
        },
        Uuid::new_v4().to_string(),
    )
    .await
    .unwrap();
    let runtime_binding = runtime.capture_pm_runtime_binding(&agent).await.unwrap();
    let reservation = PmRunReservation {
        session_id: session.id,
        session_run_id: Uuid::new_v4(),
        identity: PmExecutionIdentity {
            task: "SDLC-42".into(),
            execution_ref: Uuid::new_v4().to_string(),
            tracker_instance_ref: "pm-original-fixture".into(),
            tracker_project_ref: project.to_string(),
            task_ref: task.to_string(),
            root_ref: task.to_string(),
            agent_ref: agent.id.to_string(),
            assignment_operation_key: "pm-original-assignment".into(),
            assignment_ref: Uuid::new_v4().to_string(),
            assignment_revision: 1,
        },
        binding_ref: "pm-original-binding".into(),
        dispatch_operation_key: "pm-original-dispatch".into(),
        checkpoint_ref: None,
        fence: 1,
        runtime_binding: Some(runtime_binding),
    };
    repo.reserve_pm_run(reservation.clone()).await.unwrap();
    let effective_session = Uuid::new_v4().to_string();
    let record = repo
        .accept_pm_run(
            reservation.session_run_id,
            "run_pm_original".into(),
            effective_session.clone(),
        )
        .await
        .unwrap();
    let mode = Arc::new(AtomicUsize::new(0));
    let reads = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let state = (
        mode.clone(),
        reads.clone(),
        entered.clone(),
        release.clone(),
    );
    let expected_token = crate::agent_runtime_token(&config, agent.id).unwrap();
    let router = axum::Router::new().route("/v1/runs/run_pm_original", axum::routing::get(move |headers: axum::http::HeaderMap| {
        let (mode,reads,entered,release) = state.clone();
        let session = effective_session.clone();
        let token = expected_token.clone();
        async move {
            assert_eq!(headers.get("authorization").unwrap().to_str().unwrap(),format!("Bearer {token}"));
            reads.fetch_add(1,Ordering::SeqCst);
            let mode = mode.load(Ordering::SeqCst);
            if mode == 5 {
                entered.notify_one();
                release.notified().await;
            }
            axum::Json(json!({"object":"hermes.run","run_id":if mode==2 {"run_other"} else {"run_pm_original"},
                "session_id":session,"status":if mode==0 {"running"} else {"completed"},
                "completed":mode!=0,"partial":mode==3,"interrupted":false}))
        }
    }));
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    Some(Fixture {
        repo,
        runtime,
        agent,
        record,
        mode,
        reads,
        entered,
        release,
        server,
        root,
    })
}

#[tokio::test]
async fn pm_original_callback_persists_only_exact_complete_proof_once() {
    let Some(f) = fixture().await else {
        return;
    };
    assert_eq!(
        f.runtime.probe_pm_run(&f.agent, &f.record).await.unwrap(),
        PmRuntimeStatus::Running
    );
    f.held().await;
    for mode in [2, 3] {
        f.mode.store(mode, Ordering::SeqCst);
        assert!(f.runtime.probe_pm_run(&f.agent, &f.record).await.is_err());
        f.held().await;
    }
    f.mode.store(1, Ordering::SeqCst);
    assert_eq!(
        f.runtime.probe_pm_run(&f.agent, &f.record).await.unwrap(),
        PmRuntimeStatus::Completed
    );
    let before = f.observation_snapshot().await;
    assert_eq!(
        f.runtime.probe_pm_run(&f.agent, &f.record).await.unwrap(),
        PmRuntimeStatus::Completed
    );
    assert_eq!(f.observation_snapshot().await, before);
    assert_eq!(
        f.repo
            .list_audit_log(app::AuditLogFilter {
                action: Some("pm.run.terminal_verified".into()),
                entity_id: Some(f.record.reservation.session_run_id.to_string()),
                ..Default::default()
            })
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        f.repo
            .get_session_agent_run(f.record.reservation.session_run_id)
            .await
            .unwrap()
            .state,
        SessionRunState::Completed
    );
    f.finish().await;
}

#[tokio::test]
async fn pm_terminal_proof_with_single_connection_pool_does_not_wait_for_itself() {
    let Some(f) = fixture_with_pool(1).await else {
        return;
    };
    f.mode.store(1, Ordering::SeqCst);
    assert_eq!(
        timeout(
            Duration::from_secs(5),
            f.runtime.probe_pm_run(&f.agent, &f.record)
        )
        .await
        .unwrap()
        .unwrap(),
        PmRuntimeStatus::Completed
    );
    f.finish().await;
}

#[tokio::test]
async fn pm_custody_busy_global_mutex_does_not_wait_while_holding_transaction() {
    let Some(f) = fixture_with_pool(1).await else {
        return;
    };
    let launch = f
        .repo
        .get_open_runtime_launch(f.agent.id)
        .await
        .unwrap()
        .unwrap();
    let txn = f.repo.db.begin().await.unwrap();
    let binding = f.record.reservation.runtime_binding.as_ref().unwrap();
    {
        let _busy = f.runtime.launches.lock().await;
        assert!(
            timeout(
                Duration::from_millis(500),
                app::PmRuntimeCustody::verify(
                    &f.runtime,
                    binding,
                    &launch.binding,
                    launch.pid.unwrap()
                )
            )
            .await
            .unwrap()
            .is_err()
        );
    }
    {
        let _busy = f.runtime.children.lock().await;
        assert!(
            timeout(
                Duration::from_millis(500),
                app::PmRuntimeCustody::verify(
                    &f.runtime,
                    binding,
                    &launch.binding,
                    launch.pid.unwrap()
                )
            )
            .await
            .unwrap()
            .is_err()
        );
    }
    txn.rollback().await.unwrap();
    f.held().await;
    f.mode.store(1, Ordering::SeqCst);
    assert_eq!(
        timeout(
            Duration::from_secs(5),
            f.runtime.probe_pm_run(&f.agent, &f.record)
        )
        .await
        .unwrap()
        .unwrap(),
        PmRuntimeStatus::Completed
    );
    f.finish().await;
}

#[tokio::test]
async fn pm_missing_foreign_and_changed_context_never_reads_current_listener() {
    let Some(f) = fixture().await else {
        return;
    };
    for changed in 0..5 {
        let mut record = f.record.clone();
        let binding = record.reservation.runtime_binding.as_mut().unwrap();
        match changed {
            0 => record.reservation.runtime_binding = None,
            1 => binding.launch_id = Uuid::new_v4(),
            2 => binding.controller_id = Uuid::new_v4(),
            3 => binding.origin = "http://127.0.0.1:1024".into(),
            _ => binding.credential_fingerprint = "0".repeat(64),
        }
        assert!(f.runtime.probe_pm_run(&f.agent, &record).await.is_err());
        f.held().await;
    }
    let foreign = lifecycle_tests::supervisor(f.runtime.config.clone(), f.repo.clone());
    assert!(foreign.probe_pm_run(&f.agent, &f.record).await.is_err());
    assert_eq!(f.reads.load(Ordering::SeqCst), 0);
    f.finish().await;
}

#[tokio::test]
async fn pm_child_exit_during_get_retains_unresolved_capacity() {
    let Some(f) = fixture().await else {
        return;
    };
    f.mode.store(5, Ordering::SeqCst);
    let runtime = f.runtime.clone();
    let agent = f.agent.clone();
    let record = f.record.clone();
    let probe = tokio::spawn(async move { runtime.probe_pm_run(&agent, &record).await });
    timeout(Duration::from_secs(5), f.entered.notified())
        .await
        .unwrap();
    {
        let mut children = f.runtime.children.lock().await;
        let child = children.get_mut(&f.agent.id).unwrap();
        child.start_kill().unwrap();
        child.wait().await.unwrap();
    }
    f.release.notify_one();
    assert!(
        timeout(Duration::from_secs(5), probe)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    f.held().await;
    f.finish().await;
}

#[tokio::test]
async fn pm_terminal_commit_rechecks_custody_after_waiting_for_agent_lock() {
    lock_wait_changes_custody(false).await;
}

#[tokio::test]
async fn pm_child_exit_while_waiting_for_agent_lock_retains_unresolved_capacity() {
    lock_wait_changes_custody(true).await;
}

async fn lock_wait_changes_custody(exit: bool) {
    let Some(f) = fixture().await else {
        return;
    };
    f.mode.store(5, Ordering::SeqCst);
    let runtime = f.runtime.clone();
    let agent = f.agent.clone();
    let record = f.record.clone();
    let probe = tokio::spawn(async move { runtime.probe_pm_run(&agent, &record).await });
    timeout(Duration::from_secs(5), f.entered.notified())
        .await
        .unwrap();
    let txn = f.repo.db.begin().await.unwrap();
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id=$1 FOR UPDATE",
        [f.agent.id.into()],
    ))
    .await
    .unwrap();
    let blocker: i32 = txn
        .query_one(Statement::from_string(
            DatabaseBackend::Postgres,
            "SELECT pg_backend_pid() AS pid",
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "pid")
        .unwrap();
    f.release.notify_one();
    timeout(Duration::from_secs(5), async {
        loop {
            let blocked: bool = f.repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid))) AS blocked",
                [blocker.into()])).await.unwrap().unwrap().try_get("","blocked").unwrap();
            if blocked { break; }
            sleep(Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    if exit {
        let mut children = f.runtime.children.lock().await;
        let child = children.get_mut(&f.agent.id).unwrap();
        child.start_kill().unwrap();
        child.wait().await.unwrap();
    } else {
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agent_runtime SET desired_state='stopped' WHERE agent_id=$1",
            [f.agent.id.into()],
        ))
        .await
        .unwrap();
    }
    txn.commit().await.unwrap();
    assert!(
        timeout(Duration::from_secs(5), probe)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    f.held().await;
    f.finish().await;
}

#[tokio::test]
async fn pm_container_custody_under_observation_locks_does_not_register_endpoint() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    repo.db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agents SET sdlc_role='project_manager' WHERE id=$1",
            [agent.id.into()],
        ))
        .await
        .unwrap();
    let agent = repo.get_agent(agent.id).await.unwrap();
    let (config, mut launch) = container_lifecycle_tests::fake_control(
        &config,
        container_lifecycle_tests::binding(&agent, Uuid::new_v4()),
        false,
    )
    .await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    launch.controller_id = runtime.controller_id;
    repo.claim_runtime_launch(&launch).await.unwrap();
    runtime.launches.lock().await.insert(
        agent.id,
        app::runtime_launch::RuntimeLaunchRecord {
            binding: launch.clone(),
            state: "claimed".into(),
            pid: None,
            controller_recovery: false,
        },
    );
    let container = launch.container.as_ref().unwrap();
    let files = container_control::ContainerLaunchFiles {
        policy: container.policy.clone(),
        compose: container.compose.clone().into(),
        journal: container.journal.clone().into(),
        stop_journal: container.stop_journal.clone().into(),
        mount_mapping: None,
        mapping_file: None,
    };
    let receipt = runtime
        .container_control(container)
        .unwrap()
        .start(&files, &container.registration)
        .await
        .unwrap();
    let pid = i32::try_from(receipt.snapshot.unwrap().init_pid).unwrap();
    repo.observe_runtime_launch(&launch, "gateway_started", Some(pid))
        .await
        .unwrap();
    repo.update_agent_status(agent.id, AgentStatus::Running)
        .await
        .unwrap();
    let origin = format!("http://172.18.0.2:{}", agent.api_port.unwrap());
    repo.record_container_endpoint(&launch, pid, &origin)
        .await
        .unwrap();
    let binding = domain::PmRuntimeBinding {
        launch_id: launch.id,
        controller_id: runtime.controller_id,
        origin: origin.clone(),
        credential_fingerprint: hermes_wire::credential_fingerprint(
            &crate::agent_runtime_token(&runtime.config, agent.id).unwrap(),
        ),
    };
    let txn = repo.db.begin().await.unwrap();
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id=$1 FOR NO KEY UPDATE",
        [agent.id.into()],
    ))
    .await
    .unwrap();
    txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT l.id FROM runtime_launches l JOIN agent_runtime r ON r.agent_id=l.agent_id WHERE l.id=$1 FOR UPDATE OF l,r",
        [launch.id.into()])).await.unwrap();
    timeout(
        Duration::from_secs(5),
        app::PmRuntimeCustody::verify(&runtime, &binding, &launch, pid),
    )
    .await
    .unwrap()
    .unwrap();
    let persisted: String = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT origin FROM runtime_launch_endpoints WHERE launch_id=$1",
            [launch.id.into()],
        ))
        .await
        .unwrap()
        .unwrap()
        .try_get("", "origin")
        .unwrap();
    assert_eq!(persisted, origin);
    txn.rollback().await.unwrap();
    // The Base utility above is a controlled Python producer, not an actual Docker container.
    tokio::fs::remove_dir_all(root).await.unwrap();
}
