use app::{FleetRepository, SessionListFilter};
use domain::{
    AgentKind, AgentProductRole, AgentRole, CreateAgentRequest, CreateSessionMessageRequest,
    CreateSessionRequest, MessageKind, SessionVisibility, SystemRole,
};
use infra::{PostgresFleetRepository, connect_database, run_migrations};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use shared::{AppConfig, AppError, DatabaseConfig};
use uuid::Uuid;

#[tokio::test]
async fn central_shared_messages_and_private_owner_filters_ignore_historical_roles() {
    let config = DatabaseConfig {
        url: std::env::var("FLEET_TEST_DATABASE_URL").expect("isolated PostgreSQL is required"),
        max_connections: 10,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    };
    run_migrations(config.clone()).await.unwrap();
    let db = connect_database(config.clone()).await.unwrap();
    let repo = PostgresFleetRepository::new(connect_database(config).await.unwrap());
    let mut users = Vec::new();
    for name in ["Owner", "Other"] {
        let subject = Uuid::new_v4();
        let user = repo
            .find_or_create_central_user(
                &subject.to_string(),
                &format!("{subject}@example.test"),
                name,
            )
            .await
            .unwrap();
        assert_eq!(user.system_role, SystemRole::User);
        assert!(!user.is_system_admin);
        users.push(user.id);
    }
    repo.ensure_runtime_templates().await.unwrap();
    let mut agents = Vec::new();
    for role in [AgentProductRole::Executor, AgentProductRole::Leader] {
        let agent = repo
            .create_agent(
                CreateAgentRequest {
                    kind: AgentKind::Hermes,
                    product_role: role,
                    role: if role == AgentProductRole::Leader {
                        AgentRole::ItLead
                    } else {
                        AgentRole::Developer
                    },
                    sdlc_role: None,
                    display_name: "Permissions QA".into(),
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
            .unwrap();
        agents.push(agent.id);
    }
    let mut sessions = Vec::new();
    for (agent, user) in [
        (agents[0], users[0]),
        (agents[0], users[1]),
        (agents[1], users[0]),
    ] {
        let session = repo
            .create_session(
                CreateSessionRequest {
                    primary_agent_id: Some(agent),
                    agent_id: None,
                    title: "Permissions QA".into(),
                    task_key: None,
                    leader_agent_id: None,
                    parent_session_id: None,
                    namespace_id: None,
                    idempotency_key: None,
                },
                user,
            )
            .await
            .unwrap();
        sessions.push(session);
    }
    assert_eq!(sessions[0].visibility, SessionVisibility::Private);
    assert_eq!(sessions[2].visibility, SessionVisibility::LeaderScoped);
    // Hidden private rows must not consume the list's 200-row limit.
    for _ in 0..205 {
        repo.create_session(
            CreateSessionRequest {
                primary_agent_id: Some(agents[0]),
                agent_id: None,
                title: "Hidden private permissions QA".into(),
                task_key: None,
                leader_agent_id: None,
                parent_session_id: None,
                namespace_id: None,
                idempotency_key: None,
            },
            users[0],
        )
        .await
        .unwrap();
    }
    let visible = repo
        .list_sessions(SessionListFilter {
            user_ids: users.clone(),
            include_all_users: true,
            private_user_id: Some(users[1]),
            ..SessionListFilter::default()
        })
        .await
        .unwrap();
    assert!(!visible.iter().any(|session| session.id == sessions[0].id));
    assert!(visible.iter().any(|session| session.id == sessions[1].id));
    assert!(visible.iter().any(|session| session.id == sessions[2].id));
    let directory_filter = |agent_id| domain::ChatsDirectoryFilter {
        agent_id: Some(agent_id),
        user_ids: users.clone(),
        include_all_users: true,
        private_user_id: Some(users[1]),
        search: "Permissions QA".into(),
        before: None,
        limit: 1,
        task_project_access: None,
    };
    let directory = repo
        .chats_directory(directory_filter(agents[0]))
        .await
        .unwrap();
    assert_eq!(directory.items.len(), 1);
    assert_eq!(directory.items[0].id, sessions[1].id);
    assert!(directory.next_before.is_none());
    assert_eq!(
        directory
            .agents
            .iter()
            .find(|row| row.agent.id == agents[0])
            .unwrap()
            .matching_session_count,
        1
    );
    let mut foreign_cursor = directory_filter(agents[0]);
    foreign_cursor.before = Some(sessions[0].id);
    assert!(matches!(
        repo.chats_directory(foreign_cursor).await,
        Err(AppError::Validation(_))
    ));
    let mut multi_user = directory_filter(agents[0]);
    multi_user.include_all_users = false;
    assert_eq!(
        repo.chats_directory(multi_user).await.unwrap().items[0].id,
        sessions[1].id
    );
    let shared = repo
        .chats_directory(directory_filter(agents[1]))
        .await
        .unwrap();
    assert_eq!(shared.items.len(), 1);
    assert_eq!(shared.items[0].id, sessions[2].id);
    let message = || CreateSessionMessageRequest {
        body: "Permissions QA".into(),
        author_agent_id: None,
        message_kind: Some(MessageKind::UserPrompt),
        runtime_message_id: None,
        idempotency_key: None,
    };
    assert!(matches!(
        repo.create_session_message(sessions[0].id, message(), users[1])
            .await,
        Err(AppError::Forbidden)
    ));
    assert!(
        repo.create_session_message(sessions[1].id, message(), users[1])
            .await
            .is_ok()
    );
    assert!(
        repo.create_session_message(sessions[2].id, message(), users[1])
            .await
            .is_ok()
    );
    assert_eq!(
        repo.find_user_by_id(users[1])
            .await
            .unwrap()
            .unwrap()
            .system_role,
        SystemRole::User
    );
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active = false WHERE id = $1",
        [users[1].into()],
    ))
    .await
    .unwrap();
    assert!(matches!(
        repo.create_session_message(sessions[2].id, message(), users[1])
            .await,
        Err(AppError::Forbidden)
    ));

    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET system_role='operator' WHERE id=$1",
        [users[0].into()],
    ))
    .await
    .unwrap();
    let project = Uuid::new_v4();
    let task = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
         SELECT $1,'tracker',$2,$3,$3,$4,central_sub,$5 FROM users WHERE id=$6",
        [sessions[0].id.into(),project.into(),task.into(),agents[0].into(),Uuid::new_v4().to_string().into(),users[0].into()],
    ))
    .await
    .unwrap();
    let mode = Arc::new(AtomicUsize::new(0));
    let requests = Arc::new(AtomicUsize::new(0));
    let tracker_state = (mode.clone(), requests.clone());
    let tracker = axum::Router::new().route(
        "/api/v1/sdlc/project-access",
        axum::routing::get(move |headers: axum::http::HeaderMap| {
            let (mode, requests) = tracker_state.clone();
            async move {
                assert!(headers.get("authorization").is_some());
                requests.fetch_add(1, Ordering::SeqCst);
                let projects = if mode.load(Ordering::SeqCst) == 0 {
                    vec![project]
                } else {
                    vec![]
                };
                axum::Json(serde_json::json!({"contract_version":1,"tracker_instance_id":"tracker","project_ids":projects}))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut config = AppConfig::default();
    config.tracker.url = format!("http://{}", listener.local_addr().unwrap());
    config.tracker.instance_id = "tracker".into();
    config.auth.jwt_secret = Uuid::new_v4().to_string();
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
        repo.clone(),
        Arc::new(infra::FilesystemProvisioner),
        runtime,
        events.clone(),
        restart_tx,
    ));
    let owner = repo.find_user_by_id(users[0]).await.unwrap().unwrap();
    let token = ctx.auth.issue_tokens(&owner).unwrap().response.access_token;
    let private_reads = axum::Router::new()
        .route(
            "/sessions/{id}/history",
            axum::routing::get(api::routes::task_chats::history),
        )
        .route(
            "/sessions/{id}/task-context",
            axum::routing::get(api::routes::task_chats::task_context),
        )
        .route(
            "/sessions/{id}/chat-controls",
            axum::routing::get(api::routes::task_chats::controls),
        )
        .layer(axum::Extension(api::middleware::CurrentUser {
            id: users[1],
            role: SystemRole::User,
            is_system_admin: false,
            central_write: Some(false),
        }));
    let router = axum::Router::new()
        .route("/events", axum::routing::get(api::routes::events::events))
        .layer(axum::Extension(api::middleware::CurrentUser {
            id: owner.id,
            role: SystemRole::Operator,
            is_system_admin: false,
            central_write: None,
        }))
        .nest("/foreign", private_reads)
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let fleet_server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    for suffix in ["history", "task-context", "chat-controls"] {
        let response = client
            .get(format!(
                "{url}/foreign/sessions/{}/{suffix}",
                sessions[0].id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 403);
    }
    assert_eq!(requests.load(Ordering::SeqCst), 0);
    let mut stream = client
        .get(format!("{url}/events"))
        .bearer_auth(token)
        .send()
        .await
        .unwrap();
    assert_eq!(stream.status(), 200);
    let delta = |body: &str| shared::FleetEvent::SessionRunDelta {
        session_id: sessions[0].id.to_string(),
        run_id: Uuid::new_v4().to_string(),
        runtime_run_id: None,
        delta: body.into(),
    };
    events.send(delta("ALLOWED_TASK_DELTA")).unwrap();
    let allowed = tokio::time::timeout(std::time::Duration::from_secs(5), stream.chunk())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(String::from_utf8_lossy(&allowed).contains("ALLOWED_TASK_DELTA"));
    mode.store(1, Ordering::SeqCst);
    events.send(delta("REVOKED_TASK_DELTA")).unwrap();
    events
        .send(shared::FleetEvent::RuntimeChanged {
            agent_id: agents[0].to_string(),
            status: "READ_ONLY_RUNTIME_EVENT".into(),
        })
        .unwrap();
    let after_revocation = tokio::time::timeout(std::time::Duration::from_secs(5), stream.chunk())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let after_revocation = String::from_utf8_lossy(&after_revocation);
    assert!(!after_revocation.contains("REVOKED_TASK_DELTA"));
    assert!(after_revocation.contains("READ_ONLY_RUNTIME_EVENT"));
    assert_eq!(requests.load(Ordering::SeqCst), 2);
    drop(stream);
    fleet_server.abort();
    tracker_server.abort();
}
