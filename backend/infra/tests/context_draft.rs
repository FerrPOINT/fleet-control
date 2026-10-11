use app::{FleetRepository, SessionListFilter};
use domain::execution_context::{CreateContextSessionRequest, ExecutionContextV2};
use domain::{
    AgentKind, AgentProductRole, AgentRole, CreateAgentRequest, CreateSessionMessageRequest,
    CreateSessionRequest, MessageKind,
};
use infra::{PostgresFleetRepository, connect_database, run_migrations};
use shared::{AppConfig, AppError, DatabaseConfig};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn unconfirmed_v2_draft_has_no_queue_and_original_key_replay_preserves_id() {
    let config = DatabaseConfig {
        url: std::env::var("FLEET_TEST_DATABASE_URL").unwrap(),
        max_connections: 10,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    };
    run_migrations(config.clone()).await.unwrap();
    let repo = PostgresFleetRepository::new(connect_database(config).await.unwrap());
    let subject = Uuid::new_v4();
    let user = repo
        .find_or_create_central_user(
            &subject.to_string(),
            &format!("{subject}@example.test"),
            "Context QA",
        )
        .await
        .unwrap();
    repo.ensure_runtime_templates().await.unwrap();
    let agent = repo
        .create_agent(
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: AgentProductRole::Executor,
                role: AgentRole::Developer,
                sdlc_role: None,
                display_name: "Context draft QA".into(),
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
    let context:ExecutionContextV2=serde_json::from_value(serde_json::json!({"schema_version":2,"operation_id":Uuid::new_v4(),"namespace":{"registry_instance_id":Uuid::new_v4(),"namespace_id":Uuid::new_v4()},"task":{"tracker_instance_id":Uuid::new_v4(),"task_id":Uuid::new_v4()},"repositories":[]})).unwrap();
    let request = CreateContextSessionRequest {
        primary_agent_id: agent.id,
        title: format!("Context {subject}"),
        context,
    };
    let err = repo
        .create_context_session(request.clone(), user.id)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Unavailable(_)));
    let rows = repo
        .list_sessions(SessionListFilter {
            user_ids: vec![user.id],
            private_user_id: Some(user.id),
            ..SessionListFilter::default()
        })
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    let id = rows[0].id;
    assert!(repo.list_session_agent_runs(id).await.unwrap().is_empty());
    assert!(repo.session_execution_context(id).await.unwrap().is_none());
    repo.create_context_session(request.clone(), user.id)
        .await
        .unwrap_err();
    let replay = repo
        .list_sessions(SessionListFilter {
            user_ids: vec![user.id],
            private_user_id: Some(user.id),
            ..SessionListFilter::default()
        })
        .await
        .unwrap();
    assert_eq!(replay.len(), 1);
    assert_eq!(replay[0].id, id);
    let denied = repo
        .create_session_message(
            id,
            CreateSessionMessageRequest {
                body: "Must stay inert".into(),
                author_agent_id: None,
                message_kind: Some(MessageKind::UserPrompt),
                runtime_message_id: None,
                idempotency_key: None,
            },
            user.id,
        )
        .await;
    assert!(matches!(denied, Err(AppError::Conflict(_))));
    assert!(repo.list_session_agent_runs(id).await.unwrap().is_empty());
    let legacy = repo
        .create_session(
            CreateSessionRequest {
                primary_agent_id: Some(agent.id),
                agent_id: None,
                title: "Neighbor legacy chat".into(),
                task_key: None,
                leader_agent_id: None,
                parent_session_id: None,
                namespace_id: None,
                idempotency_key: None,
            },
            user.id,
        )
        .await
        .unwrap();
    assert_eq!(
        repo.list_session_agent_runs(legacy.id).await.unwrap().len(),
        1
    );
    let forged = repo
        .create_session(
            CreateSessionRequest {
                primary_agent_id: Some(agent.id),
                agent_id: None,
                title: request.title,
                task_key: None,
                leader_agent_id: None,
                parent_session_id: None,
                namespace_id: None,
                idempotency_key: Some(format!("namespace-v2:{}", request.context.operation_id)),
            },
            user.id,
        )
        .await;
    assert!(matches!(forged, Err(AppError::Validation(_))));
}
