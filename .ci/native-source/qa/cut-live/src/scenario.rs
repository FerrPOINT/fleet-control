#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CutSaved {
    config: AppConfig,
    owner: Uuid,
    agents: Vec<Agent>,
    original: ContainerLaunch,
    peer: Value,
    revision: i64,
}

fn cut_hash(value: &impl Serialize) -> String {
    let mut value = serde_json::to_value(value).unwrap();
    value.sort_all_objects();
    let raw = serde_json::to_vec(&value).unwrap();
    // This QA fixture uses ASCII paths/config. Production's Unicode hash guard is unchanged.
    assert!(raw.is_ascii());
    hex::encode(Sha256::digest(raw))
}

async fn cut_private(path: &Path) -> Vec<u8> {
    let meta = tokio::fs::symlink_metadata(path).await.unwrap();
    assert!(meta.is_file() && !meta.file_type().is_symlink());
    assert_eq!(meta.mode() & 0o777, 0o600);
    assert_eq!(meta.uid(), 999);
    assert_eq!(meta.nlink(), 1);
    assert!(meta.len() <= 1_048_576);
    tokio::fs::read(path).await.unwrap()
}

async fn cut_peer(repo: &PostgresFleetRepository, a: &Agent) -> Value {
    let l = launch(repo, a.id).await;
    let physical = inspected(&l).await;
    json!({"launch":l,"pid":physical["State"]["Pid"],"started_at":physical["State"]["StartedAt"],
        "mounts":physical["Mounts"],"mount_specs":physical["HostConfig"]["Mounts"],
        "files_sha256":cut_hash(&files(a).await),"posts":native_posts(a).await,"ack_loss":native_ack_loss(a).await})
}

async fn cut_plan(a: &Agent, revision: i64) -> (String, Value) {
    let path = Path::new("/controller").join(format!("{}.{}.activation.json", a.id, revision));
    let raw = cut_private(&path).await;
    (
        hex::encode(Sha256::digest(&raw)),
        serde_json::from_slice(&raw).unwrap(),
    )
}

async fn cut_managed(a: &Agent, plan: &Value, rollback: bool) -> String {
    let expected = &plan[if rollback { "previous_files" } else { "files" }];
    for (name, body) in expected.as_object().unwrap() {
        let path = Path::new(name);
        assert!(path.starts_with(&a.paths.config) && path != Path::new(&a.paths.config));
        if body.is_null() {
            assert!(!tokio::fs::try_exists(path).await.unwrap());
        } else {
            let bytes: Vec<u8> = serde_json::from_value(body.clone()).unwrap();
            assert_eq!(cut_private(path).await, bytes);
        }
    }
    cut_hash(expected)
}

async fn cut_record(
    repo: &PostgresFleetRepository,
    db: &DatabaseConnection,
    runtime: &LocalRuntimeSupervisor,
    config: &AppConfig,
    a: &Agent,
    activation: &app::container_activation::Activation,
    root: &ContainerLaunch,
) -> Value {
    let (plan_sha256, plan) = cut_plan(a, activation.claim.revision).await;
    let effective = repo
        .get_effective_config_revision(a.id)
        .await
        .unwrap()
        .unwrap();
    assert!(!effective.draining);
    FilesystemProvisioner
        .verify_effective_configuration(a, config, &effective)
        .await
        .unwrap();
    let l = launch(repo, a.id).await;
    let generation = l.prepared.container.registration.generation;
    assert_eq!(
        activation.readiness.as_ref().unwrap().generation,
        generation
    );
    assert_eq!(l.prepared.configuration_revision, Some(effective.revision));
    let managed = cut_managed(a, &plan, activation.phase == Phase::RolledBack).await;
    assert_eq!(activation.readiness.as_ref().unwrap().files_sha256, managed);
    let mut recovery = None;
    for _ in 0..100 {
        let latest = repo
            .get_container_recovery(root.prepared.container.registration.generation)
            .await
            .unwrap()
            .unwrap();
        if latest.lease_valid && latest.receipt.is_some() && latest.lease_receipt.is_some() {
            recovery = Some(latest);
            break;
        }
        sleep(Duration::from_millis(100)).await;
    }
    let recovery = recovery.expect("Latest native recovery ACK/heartbeat missing");
    assert_eq!(
        recovery.lease.request.original_controller_id,
        root.controller_id
    );
    assert_ne!(recovery.lease.request.controller_id, root.controller_id);
    assert_eq!(
        recovery.lease.request.launch_id,
        root.prepared.container.registration.generation
    );
    assert_eq!(
        runtime
            .health(&repo.get_agent(a.id).await.unwrap())
            .await
            .unwrap()
            .status,
        AgentStatus::Running
    );
    let row = db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*)::bigint AS n FROM runtime_container_activation_authorities WHERE activation_id=$1 AND recovery_id=$2 AND controller_id=$3 AND plan_sha256=$4 AND claim=$5",
        [activation.claim.id.into(),recovery.lease.request.id.into(),recovery.lease.request.controller_id.into(),
         activation.claim.intent_sha256.clone().into(),serde_json::to_value(&activation.claim).unwrap().into()]))
        .await.unwrap().unwrap();
    let authority_count: i64 = row.try_get("", "n").unwrap();
    assert_eq!(authority_count, 1);
    json!({"activation":activation,"effective_revision":effective.revision,"launch_generation":generation,
        "plan_sha256":plan_sha256,"managed_sha256":managed,"lease":recovery.lease,
        "lease_receipt":recovery.lease_receipt,"lease_valid":recovery.lease_valid,
        "health_running":true,"authority_count":authority_count})
}

async fn cut_initial(
    repo: Arc<PostgresFleetRepository>,
    config: Arc<AppConfig>,
    proof: &Proof,
    runtime: &LocalRuntimeSupervisor,
    model: &Model,
    owner: Uuid,
) {
    let mut agents = Vec::new();
    for index in 0..2 {
        let a = create(&repo, &config, proof, index).await;
        assert_eq!(
            runtime.start(&a).await.unwrap().status,
            AgentStatus::Running
        );
        isolation(&repo, &config, proof, &a).await;
        let prompt = format!("cut-isolation-{index}-{}", Uuid::new_v4());
        let (session, _, _) = send(&repo, a.id, owner, &prompt).await;
        answer(
            &repo,
            model,
            session,
            &prompt,
            if index == 0 {
                "LIVE_SOUL_A"
            } else {
                "LIVE_SOUL_B"
            },
            if index == 0 {
                "LIVE_SOUL_B"
            } else {
                "LIVE_SOUL_A"
            },
        )
        .await;
        agents.push(a);
    }
    let a = &agents[0];
    let original = launch(&repo, a.id).await;
    let before = files(a).await;
    let peer = cut_peer(&repo, &agents[1]).await;
    let cut_proof: Value =
        serde_json::from_slice(&tokio::fs::read("/proof/cut.json").await.unwrap()).unwrap();
    let arm = json!({"source_commit":proof.source_commit,"resource_id":a.id,
        "container_id":original.prepared.container.registration.container_id,
        "generation":original.prepared.container.registration.generation,"stop_id":original.stop_id,
        "registration_sha256":cut_hash(&original.prepared.container.registration),
        "snapshot_sha256":cut_hash(original.snapshot.as_ref().unwrap()),"loader_sha256":cut_proof["loader_sha256"]});
    put(
        Path::new("/controller/qa-cut-arm.json"),
        &serde_json::to_vec(&arm).unwrap(),
    )
    .await;
    let prompt = format!("hold-live-cut-{}", Uuid::new_v4());
    let (session, _, _) = send(&repo, a.id, owner, &prompt).await;
    timeout(Duration::from_secs(60), async {
        while !model.calls.lock().await.contains_key(&prompt) {
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    let revision = request(
        &repo,
        a.id,
        owner,
        configuration(&proof.model_host, "CUT_RECOVERED_A"),
    )
    .await;
    sleep(Duration::from_secs(4)).await;
    assert!(repo.agent_is_draining(a.id).await.unwrap());
    assert_eq!(files(a).await, before);
    assert_eq!(
        serde_json::to_value(launch(&repo, a.id).await).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    let saved = CutSaved {
        config: (*config).clone(),
        owner,
        agents: agents.clone(),
        original: original.clone(),
        peer: peer.clone(),
        revision,
    };
    put(
        Path::new("/controller/qa-cut-state.json"),
        &serde_json::to_vec(&saved).unwrap(),
    )
    .await;
    model.release.notify_one();
    answer(
        &repo,
        model,
        session,
        &prompt,
        "LIVE_SOUL_A",
        "CUT_RECOVERED_A",
    )
    .await;
    timeout(Duration::from_secs(30), async {
        while !tokio::fs::try_exists("/evidence/cut-stop-ack.json")
            .await
            .unwrap()
        {
            sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("No original durable stop ACK at the bounded cut");
    let activation = repo
        .get_container_activation(a.id, revision)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(activation.phase, Phase::StoppingPrevious);
    assert!(
        activation.previous_stop.is_none()
            && activation.candidate.is_none()
            && activation.rollback.is_none()
            && activation.readiness.is_none()
    );
    let (plan_sha256, _) = cut_plan(a, revision).await;
    assert_eq!(files(a).await, before);
    assert_eq!(inspected(&original).await["State"]["Running"], false);
    assert_eq!(cut_peer(&repo, &agents[1]).await, peer);
    let cut = json!({"phase":activation.phase,"previous_stop":activation.previous_stop,"candidate":activation.candidate,
        "rollback":activation.rollback,"readiness":activation.readiness,"claim":activation.claim,
        "plan_sha256":plan_sha256,"physical_exit":true,"draining":repo.agent_is_draining(a.id).await.unwrap()});
    // Only the cut's completed boundary is atomic; the original nine writer is unchanged.
    let mut publisher = tokio::process::Command::new("python3")
        .args(["-B", "/qa/cut_files.py", "publish-ready"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    {
        use tokio::io::AsyncWriteExt;
        let mut stdin = publisher.stdin.take().unwrap();
        stdin
            .write_all(&serde_json::to_vec(&json!({"cut":cut,"arm":arm,"peer":peer})).unwrap())
            .await
            .unwrap();
    }
    assert!(
        timeout(Duration::from_secs(5), publisher.wait())
            .await
            .unwrap()
            .unwrap()
            .success()
    );
    // Keep the original transport unreturned; only a physical controller restart continues.
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

async fn cut_recovered(
    repo: Arc<PostgresFleetRepository>,
    db: &DatabaseConnection,
    config: Arc<AppConfig>,
    proof: &Proof,
    runtime: &LocalRuntimeSupervisor,
    model: &Model,
    saved: CutSaved,
) {
    let a = &saved.agents[0];
    let ready: Value =
        serde_json::from_slice(&tokio::fs::read("/evidence/cut-ready.json").await.unwrap())
            .unwrap();
    let root = settled(&repo, a.id, saved.revision, Phase::Committed).await;
    assert_eq!(
        serde_json::to_value(&root.claim).unwrap(),
        ready["cut"]["claim"]
    );
    assert_eq!(
        cut_plan(a, saved.revision).await.0,
        ready["cut"]["plan_sha256"].as_str().unwrap()
    );
    let gate: Value = serde_json::from_slice(
        &tokio::fs::read("/evidence/cut-stop-ack.json")
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        root.previous_stop.as_ref().unwrap(),
        &gate["event"]["ack"]["result"]
    );
    assert_eq!(inspected(&saved.original).await["State"]["Running"], false);
    let root_record = cut_record(&repo, db, runtime, &config, a, &root, &saved.original).await;
    isolation(&repo, &config, proof, a).await;
    assert_eq!(cut_peer(&repo, &saved.agents[1]).await, saved.peer);
    let prompt = format!("cut-recovered-chat-{}", Uuid::new_v4());
    let (session, _, _) = send(&repo, a.id, saved.owner, &prompt).await;
    answer(
        &repo,
        model,
        session,
        &prompt,
        "CUT_RECOVERED_A",
        "LIVE_SOUL_A",
    )
    .await;
    let revision = request(
        &repo,
        a.id,
        saved.owner,
        configuration(&proof.model_host, "CUT_NEXT_B"),
    )
    .await;
    let second = settled(&repo, a.id, revision, Phase::Committed).await;
    let lineage = second
        .claim
        .lineage
        .as_ref()
        .expect("Next activation lost original recovered lineage");
    assert_eq!(lineage.family_id, root.claim.id);
    assert_eq!(lineage.predecessor_activation_id, root.claim.id);
    assert_eq!(second.claim.previous_revision, Some(root.claim.revision));
    let second_record = cut_record(&repo, db, runtime, &config, a, &second, &saved.original).await;
    let current_files = files(a).await;
    let current_launch = launch(&repo, a.id).await;
    assert_eq!(cut_peer(&repo, &saved.agents[1]).await, saved.peer);
    let prompt = format!("cut-next-chat-{}", Uuid::new_v4());
    let (session, _, _) = send(&repo, a.id, saved.owner, &prompt).await;
    answer(
        &repo,
        model,
        session,
        &prompt,
        "CUT_NEXT_B",
        "CUT_RECOVERED_A",
    )
    .await;
    let failed_revision = request(
        &repo,
        a.id,
        saved.owner,
        configuration(&proof.model_host, "QA_READINESS_DELAY_18"),
    )
    .await;
    let start = tokio::time::Instant::now();
    let failed = settled(&repo, a.id, failed_revision, Phase::RolledBack).await;
    assert!(start.elapsed() >= Duration::from_secs(60));
    assert_eq!(failed.claim.previous_revision, Some(revision));
    assert_eq!(
        failed
            .claim
            .lineage
            .as_ref()
            .unwrap()
            .predecessor_activation_id,
        second.claim.id
    );
    assert_eq!(files(a).await, current_files);
    assert_eq!(inspected(&current_launch).await["State"]["Running"], false);
    assert_eq!(
        inspected(failed.candidate.as_ref().unwrap()).await["State"]["Running"],
        false
    );
    let failed_record = cut_record(&repo, db, runtime, &config, a, &failed, &saved.original).await;
    isolation(&repo, &config, proof, a).await;
    let peer_after = cut_peer(&repo, &saved.agents[1]).await;
    assert_eq!(peer_after, saved.peer);
    let prompt = format!("cut-rollback-chat-{}", Uuid::new_v4());
    let (session, _, _) = send(&repo, a.id, saved.owner, &prompt).await;
    answer(
        &repo,
        model,
        session,
        &prompt,
        "CUT_NEXT_B",
        "QA_READINESS_DELAY_18",
    )
    .await;
    for agent in &saved.agents {
        assert_eq!(
            runtime
                .stop(&repo.get_agent(agent.id).await.unwrap())
                .await
                .unwrap()
                .status,
            AgentStatus::Stopped
        );
        assert_eq!(
            inspected(&launch(&repo, agent.id).await).await["State"]["Running"],
            false
        );
    }
    evidence(
        "cut-stop-report.json",
        &json!({"state":"original_namespaces_exited","agents":2,"runtime_ready":false}),
    )
    .await;
    evidence("cut-live-report.json",&json!({"state":"scoped_cut_matrix_passed","source_commit":proof.source_commit,
        "matrix":{"original_stop_ack_before_phase_cas":"accepted","recovered_original_stop_readback":"accepted",
        "recovered_effective_child":"accepted","next_recovered_activation":"accepted",
        "next_failure_exact_current_rollback":"accepted","peer_isolation":"accepted"},
        "cut":ready["cut"],"records":[root_record,second_record,failed_record],"peer_before":saved.peer,
        "peer_after":peer_after,"runtime_ready":false,"sdlc_completion":false})).await;
}

#[tokio::main]
pub async fn cut_entry() {
    assert_eq!(std::env::var("QA_NATIVE_EXECUTE").as_deref(), Ok("1"));
    let mode = std::env::args().nth(1).unwrap();
    assert!(matches!(mode.as_str(), "cut-initial" | "cut-recover"));
    let proof: Proof =
        serde_json::from_slice(&tokio::fs::read("/proof/controller.json").await.unwrap()).unwrap();
    assert_eq!(
        proof.source_commit,
        std::env::var("QA_SOURCE_COMMIT").unwrap()
    );
    let database = DatabaseConfig {
        url: std::env::var("FLEET_TEST_DATABASE_URL").unwrap(),
        max_connections: 10,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    };
    infra::run_migrations(database.clone()).await.unwrap();
    let db = infra::connect_database(database.clone()).await.unwrap();
    let repo = Arc::new(PostgresFleetRepository::new(
        infra::connect_database(database).await.unwrap(),
    ));
    repo.ensure_runtime_templates().await.unwrap();
    let saved: Option<CutSaved> = if mode == "cut-recover" {
        Some(
            serde_json::from_slice(&cut_private(Path::new("/controller/qa-cut-state.json")).await)
                .unwrap(),
        )
    } else {
        None
    };
    let owner = saved.as_ref().map(|s| s.owner).unwrap_or_else(Uuid::new_v4);
    if saved.is_none() {
        db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role) VALUES($1,$2,$3,'Native QA owner','disabled',false,'user')",
            [owner.into(),format!("{owner}@example.invalid").into(),owner.to_string().into()])).await.unwrap();
    }
    let mut config = saved.as_ref().map(|s| s.config.clone()).unwrap_or_default();
    if saved.is_none() {
        config.fleet.agents_root = "/agents".into();
        config.fleet.runtime_token_secret = Uuid::new_v4().to_string();
        config.fleet.hermes_source = "genuine-bbaf7af-immutable-image".into();
        config.fleet.agent_port_base = 29100;
        config.fleet.agent_port_stride = 5;
        config.fleet.project_workflow_url = None;
        config.fleet.project_workflow_catalog_token = None;
        let mut control = proof.control.clone();
        assert!(control.recovered_activation);
        control.python = "/compiled/cut-transport".into();
        config.fleet.container_control = Some(control);
    }
    let config = Arc::new(config);
    let model = Arc::new(Model::default());
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", MODEL_PORT))
        .await
        .unwrap();
    let router = Router::new()
        .route("/v1/chat/completions", post(inference))
        .with_state(model.clone());
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let (events, _) = tokio::sync::broadcast::channel(64);
    let runtime = LocalRuntimeSupervisor::new(config.clone(), repo.clone(), events);
    if let Some(saved) = saved {
        cut_recovered(repo, &db, config, &proof, &runtime, &model, saved).await;
    } else {
        cut_initial(repo, config, &proof, &runtime, &model, owner).await;
    }
}
