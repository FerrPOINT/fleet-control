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
    assert_eq!(
        reqwest::Url::parse(&url).unwrap().path(),
        "/fleet_chats_directory_test",
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
}
