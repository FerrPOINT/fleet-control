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
}
