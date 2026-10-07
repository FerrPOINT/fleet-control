use app::FleetRepository;
use domain::{
    AgentKind, AgentProductRole, AgentRole, AgentStatus, ChatsDirectoryFilter, ChatsDirectoryPage,
    CreateAgentRequest, CreateSessionRequest,
};
use infra::{PostgresFleetRepository, connect_database, run_migrations};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use shared::{AppConfig, AppError, DatabaseConfig};
use uuid::Uuid;

async fn sql(db: &DatabaseConnection, query: &str, values: Vec<sea_orm::Value>) {
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        query,
        values,
    ))
    .await
    .unwrap();
}
fn query(owner: Uuid, agent: Option<Uuid>, search: &str) -> ChatsDirectoryFilter {
    ChatsDirectoryFilter {
        agent_id: agent,
        user_ids: vec![owner],
        include_all_users: false,
        search: search.into(),
        before: None,
        limit: 50,
        task_project_access: None,
        private_user_id: None,
    }
}
fn count(page: &ChatsDirectoryPage, agent: Uuid) -> u64 {
    page.agents
        .iter()
        .find(|item| item.agent.id == agent)
        .unwrap()
        .matching_session_count
}
async fn agent(repo: &PostgresFleetRepository, title: &str) -> Uuid {
    repo.create_agent(
        CreateAgentRequest {
            kind: AgentKind::Hermes,
            product_role: AgentProductRole::Executor,
            role: AgentRole::Developer,
            sdlc_role: Some(domain::SdlcRole::Developer),
            display_name: title.into(),
            description: None,
            namespace_id: None,
            namespace_name: None,
            workflow_id: None,
            workflow_name: None,
            executor_ids: vec![],
        },
        &AppConfig::default(),
    )
    .await
    .unwrap()
    .id
}
async fn session(repo: &PostgresFleetRepository, owner: Uuid, agent: Uuid, title: &str) -> Uuid {
    repo.create_session(
        CreateSessionRequest {
            primary_agent_id: Some(agent),
            agent_id: None,
            title: title.into(),
            task_key: Some("DISPLAY-ONLY".into()),
            leader_agent_id: None,
            parent_session_id: None,
            namespace_id: None,
            idempotency_key: Some(Uuid::new_v4().to_string()),
        },
        owner,
    )
    .await
    .unwrap()
    .id
}

#[tokio::test]
#[ignore = "requires own disposable FLEET_CHATS_DIRECTORY_TEST_DATABASE_URL database"]
async fn postgres_directory_scope_search_counts_cursor_and_stable_order() {
    let url = std::env::var("FLEET_CHATS_DIRECTORY_TEST_DATABASE_URL")
        .expect("own test database required");
    let parsed = reqwest::Url::parse(&url).unwrap();
    assert!(
        parsed.path() == "/fleet_chats_directory_test"
            || parsed.path().starts_with("/fleet_chats_directory_test_"),
        "never run against shared Fleet databases"
    );
    let config = DatabaseConfig {
        url,
        max_connections: 5,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    };
    run_migrations(config.clone()).await.unwrap();
    let db = connect_database(config.clone()).await.unwrap();
    let repo = PostgresFleetRepository::new(connect_database(config).await.unwrap());
    repo.ensure_runtime_templates().await.unwrap();
    let owner = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let prefix = Uuid::new_v4().to_string();
    for (id, display) in [
        (owner, format!("Owner {prefix}")),
        (foreign, format!("Foreign {prefix}")),
    ] {
        sql(&db, "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role) VALUES($1,$2,$3,$4,'!',false,'user')", vec![id.into(), format!("{id}@example.test").into(), id.to_string().into(), display.into()]).await;
    }
    let dev = agent(&repo, "Directory developer").await;
    let tester = agent(&repo, "Directory tester").await;
    let archived = agent(&repo, "Archived directory agent").await;
    let foreign_only = agent(&repo, "Foreign-only agent").await;
    let mut mine = Vec::new();
    for n in 0..3 {
        let id = session(&repo, owner, dev, &format!("{prefix} login {n}")).await;
        sql(
            &db,
            "UPDATE agent_sessions SET created_at='2026-10-01T12:00:00Z' WHERE id=$1",
            vec![id.into()],
        )
        .await;
        mine.push(id);
    }
    let test_chat = session(&repo, owner, tester, &format!("{prefix} login QA")).await;
    let literal = session(&repo, owner, dev, &format!("{prefix} 50%_\\ literal")).await;
    let foreign_chat = session(&repo, foreign, dev, &format!("{prefix} login foreign")).await;
    session(
        &repo,
        foreign,
        foreign_only,
        &format!("{prefix} login hidden"),
    )
    .await;
    session(&repo, owner, archived, &format!("{prefix} login archived")).await;
    repo.update_agent_status(archived, AgentStatus::Archived)
        .await
        .unwrap();

    let all_mine = repo
        .chats_directory(query(owner, None, &prefix))
        .await
        .unwrap();
    assert_eq!(all_mine.selected_agent_id, Some(dev));
    assert_eq!(count(&all_mine, dev), 4);
    assert_eq!(count(&all_mine, tester), 1);
    assert_eq!(count(&all_mine, foreign_only), 0);
    assert!(!all_mine.agents.iter().any(|item| item.agent.id == archived));
    assert!(
        all_mine
            .items
            .iter()
            .all(|item| item.user_id == owner && item.primary_agent_id == dev)
    );
    assert!(!all_mine.items.iter().any(|item| item.id == foreign_chat));
    let matching = repo
        .chats_directory(query(owner, Some(dev), &format!("{prefix} LOGIN")))
        .await
        .unwrap();
    assert_eq!(count(&matching, dev), 3);
    assert_eq!(count(&matching, tester), 1);
    assert_eq!(matching.items.len(), 3);
    let qa = repo
        .chats_directory(query(owner, Some(tester), &prefix))
        .await
        .unwrap();
    assert_eq!(qa.items[0].id, test_chat);
    assert_eq!(qa.items[0].task_key, matching.items[0].task_key);
    let literal_page = repo
        .chats_directory(query(owner, Some(dev), "50%_\\"))
        .await
        .unwrap();
    assert_eq!(
        literal_page
            .items
            .iter()
            .map(|item| item.id)
            .collect::<Vec<_>>(),
        vec![literal]
    );
    let owner_search = repo
        .chats_directory(query(owner, Some(dev), &format!("Owner {prefix}")))
        .await
        .unwrap();
    assert_eq!(count(&owner_search, dev), 4);

    let mut all = query(owner, Some(dev), &prefix);
    all.user_ids.clear();
    all.include_all_users = true;
    let all_page = repo.chats_directory(all.clone()).await.unwrap();
    assert_eq!(count(&all_page, dev), 5);
    assert!(all_page.items.iter().any(|item| item.id == foreign_chat));
    let mut central = all.clone();
    central.private_user_id = Some(owner);
    let central_page = repo.chats_directory(central.clone()).await.unwrap();
    assert_eq!(count(&central_page, dev), 4);
    assert_eq!(count(&central_page, foreign_only), 0);
    assert!(
        !central_page
            .items
            .iter()
            .any(|item| item.id == foreign_chat)
    );
    central.before = Some(foreign_chat);
    assert!(matches!(
        repo.chats_directory(central).await,
        Err(AppError::Validation(_))
    ));
    let mut foreign_selected = all.clone();
    foreign_selected.user_ids = vec![foreign];
    foreign_selected.include_all_users = false;
    foreign_selected.private_user_id = Some(owner);
    let denied = repo.chats_directory(foreign_selected).await.unwrap();
    assert_eq!(count(&denied, dev), 0);
    assert!(denied.items.is_empty());
    all.include_all_users = false;
    assert!(matches!(
        repo.chats_directory(all).await,
        Err(AppError::Forbidden)
    ));
    let mut both = query(owner, Some(dev), &prefix);
    both.user_ids.push(foreign);
    assert_eq!(count(&repo.chats_directory(both).await.unwrap(), dev), 5);
    for invalid in [archived, Uuid::new_v4()] {
        assert!(matches!(
            repo.chats_directory(query(owner, Some(invalid), &prefix))
                .await,
            Err(AppError::Validation(_))
        ));
    }
    let mut current = query(owner, Some(dev), &format!("{prefix} login"));
    current.limit = 2;
    let first = repo.chats_directory(current.clone()).await.unwrap();
    mine.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(
        first.items.iter().map(|item| item.id).collect::<Vec<_>>(),
        mine[..2]
    );
    assert_eq!(first.next_before, Some(mine[1]));
    // Activity timestamps cannot move an unseen item ahead of an already-issued cursor.
    sql(
        &db,
        "UPDATE agent_sessions SET updated_at='2099-01-01T00:00:00Z' WHERE id=$1",
        vec![mine[2].into()],
    )
    .await;
    current.before = first.next_before;
    let last = repo.chats_directory(current.clone()).await.unwrap();
    assert_eq!(last.items[0].id, mine[2]);
    assert!(last.next_before.is_none());
    assert_eq!(count(&last, dev), 3);
    for invalid in [foreign_chat, test_chat, literal, Uuid::new_v4()] {
        current.before = Some(invalid);
        assert!(matches!(
            repo.chats_directory(current.clone()).await,
            Err(AppError::Validation(_))
        ));
    }
    sql(&db, "INSERT INTO agent_sessions(id,agent_id,user_id,title,state,visibility,created_at,updated_at) SELECT gen_random_uuid(),$1,$2,$3,'draft','private','2026-01-01'::timestamptz+n*interval '1 second',now() FROM generate_series(1,205) n", vec![dev.into(),owner.into(),format!("{prefix} bulk").into()]).await;
    let mut bulk = query(owner, Some(dev), &format!("{prefix} bulk"));
    bulk.limit = 100;
    let mut ids = std::collections::HashSet::new();
    for expected in [100, 100, 5] {
        let result = repo.chats_directory(bulk.clone()).await.unwrap();
        assert_eq!(result.items.len(), expected);
        assert_eq!(count(&result, dev), 205);
        for item in result.items {
            assert!(ids.insert(item.id));
        }
        bulk.before = result.next_before;
    }
    assert_eq!(ids.len(), 205);
    assert!(bulk.before.is_none());

    let project = Uuid::new_v4();
    let denied_project = Uuid::new_v4();
    let subject = owner.to_string();
    sql(
        &db,
        "UPDATE users SET central_sub=$2 WHERE id=$1",
        vec![owner.into(), subject.clone().into()],
    )
    .await;
    let free = session(&repo, owner, dev, &format!("{prefix} bounded free")).await;
    let mut visible = Vec::new();
    let mut hidden = Vec::new();
    for (instance, project_id, allowed) in [
        ("tracker", project, true),
        ("tracker", project, true),
        ("tracker", denied_project, false),
        ("foreign-tracker", project, false),
    ] {
        let id = session(&repo, owner, dev, &format!("{prefix} bounded task")).await;
        let task = Uuid::new_v4();
        repo.bind_task_chat(
            id,
            domain::TaskChatBinding {
                tracker_instance_id: instance.into(),
                project_id,
                task_id: task,
                root_task_id: task,
                agent_id: dev,
                owner_subject: subject.clone(),
            },
            Uuid::new_v4().to_string(),
        )
        .await
        .unwrap();
        sql(
            &db,
            "UPDATE agent_sessions SET last_message_preview='project-private-preview' WHERE id=$1",
            vec![id.into()],
        )
        .await;
        if allowed {
            visible.push(id);
        } else {
            hidden.push(id);
        }
    }
    let mut bounded = query(owner, Some(dev), &format!("{prefix} bounded"));
    let no_proof = repo.chats_directory(bounded.clone()).await.unwrap();
    assert_eq!(count(&no_proof, dev), 1);
    assert_eq!(no_proof.items[0].id, free);
    bounded.task_project_access = Some(domain::TaskProjectAccess {
        tracker_instance_id: "tracker".into(),
        project_ids: vec![project],
    });
    bounded.limit = 2;
    let allowed = repo.chats_directory(bounded.clone()).await.unwrap();
    assert_eq!(count(&allowed, dev), 3);
    assert!(allowed.items.iter().all(|item| !hidden.contains(&item.id)));
    let cursor = allowed.next_before.unwrap();
    bounded.before = Some(cursor);
    let remaining = repo.chats_directory(bounded.clone()).await.unwrap();
    assert_eq!(remaining.items.len(), 1);
    assert_eq!(count(&remaining, dev), 3);
    let mut seen: Vec<_> = allowed
        .items
        .iter()
        .chain(&remaining.items)
        .map(|item| item.id)
        .collect();
    seen.sort_unstable();
    visible.push(free);
    visible.sort_unstable();
    assert_eq!(seen, visible);
    for id in hidden {
        bounded.before = Some(id);
        assert!(matches!(
            repo.chats_directory(bounded.clone()).await,
            Err(AppError::Validation(_))
        ));
    }
    bounded.before = None;
    bounded
        .task_project_access
        .as_mut()
        .unwrap()
        .project_ids
        .clear();
    let revoked = repo.chats_directory(bounded.clone()).await.unwrap();
    assert_eq!(count(&revoked, dev), 1);
    assert_eq!(revoked.items[0].id, free);
    bounded.before = Some(visible.iter().copied().find(|id| *id != free).unwrap());
    assert!(matches!(
        repo.chats_directory(bounded).await,
        Err(AppError::Validation(_))
    ));

    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let mode = Arc::new(AtomicUsize::new(0));
    let requests = Arc::new(AtomicUsize::new(0));
    let state = (mode.clone(), requests.clone());
    let tracker = axum::Router::new().route("/api/v1/sdlc/project-access", axum::routing::get(move |headers: axum::http::HeaderMap| {
        let (mode, requests) = state.clone();
        async move {
            assert_eq!(headers.get("authorization").unwrap(), "Bearer directory-owner-fixture");
            requests.fetch_add(1, Ordering::SeqCst);
            match mode.load(Ordering::SeqCst) {
                0 => (axum::http::StatusCode::OK, axum::Json(serde_json::json!({"contract_version":1,"tracker_instance_id":"tracker","project_ids":[project]}))),
                1 => (axum::http::StatusCode::OK, axum::Json(serde_json::json!({"contract_version":1,"tracker_instance_id":"tracker","project_ids":[]}))),
                _ => (axum::http::StatusCode::SERVICE_UNAVAILABLE, axum::Json(serde_json::json!({"error":"unavailable"}))),
            }
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut config = AppConfig::default();
    config.tracker.url = format!("http://{}", listener.local_addr().unwrap());
    config.tracker.instance_id = "tracker".into();
    let tracker_server = tokio::spawn(async move { axum::serve(listener, tracker).await.unwrap() });
    let repo = Arc::new(repo);
    let config = Arc::new(config);
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = Arc::new(infra::runtime::LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    let (restart_tx, _) = tokio::sync::mpsc::channel(1);
    let ctx = Arc::new(app::AppContext::new(
        config,
        repo,
        Arc::new(infra::FilesystemProvisioner),
        runtime,
        events,
        restart_tx,
    ));
    let router = axum::Router::new()
        .route(
            "/directory",
            axum::routing::get(api::routes::chats_directory::directory),
        )
        .layer(axum::Extension(api::middleware::CurrentUser {
            id: owner,
            role: domain::SystemRole::Operator,
            is_system_admin: true,
            central_write: None,
        }))
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/directory", listener.local_addr().unwrap());
    let fleet_server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    let search = format!("{prefix} bounded");
    let agent_id = dev.to_string();
    for (mode_value, expected_count) in [(0, 3), (1, 1)] {
        mode.store(mode_value, Ordering::SeqCst);
        let response = client
            .get(&url)
            .bearer_auth("directory-owner-fixture")
            .query(&[
                ("agent_id", agent_id.as_str()),
                ("q", search.as_str()),
                ("user_id", "all"),
            ])
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let page: ChatsDirectoryPage = response.json().await.unwrap();
        assert_eq!(count(&page, dev), expected_count);
        assert_eq!(page.items.len(), expected_count as usize);
        if mode_value == 1 {
            assert_eq!(page.items[0].id, free);
        }
    }
    mode.store(2, Ordering::SeqCst);
    assert_eq!(
        client
            .get(&url)
            .bearer_auth("directory-owner-fixture")
            .send()
            .await
            .unwrap()
            .status(),
        503
    );
    assert_eq!(
        requests.load(Ordering::SeqCst),
        3,
        "exactly one scope request per directory call"
    );
    assert_eq!(
        client
            .get(&url)
            .bearer_auth("directory-owner-fixture")
            .query(&[("project_ids", project.to_string())])
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    assert_eq!(
        requests.load(Ordering::SeqCst),
        3,
        "client authorization fields are rejected before the gateway"
    );
    fleet_server.abort();
    tracker_server.abort();
}
