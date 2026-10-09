use app::FleetRepository;
use domain::{AgentKind, AgentProductRole, AgentRole, CreateAgentRequest};
use infra::{PostgresFleetRepository, connect_database, run_migrations};
use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, Statement};
use shared::{AppConfig, DatabaseConfig};
use std::{collections::HashSet, sync::Arc};
use tokio::task::JoinSet;
use uuid::Uuid;

async fn fixture() -> (Arc<PostgresFleetRepository>, DatabaseConnection, Uuid) {
    let config = DatabaseConfig {
        url: std::env::var("FLEET_TEST_DATABASE_URL")
            .expect("agent log regressions require isolated PostgreSQL"),
        max_connections: 10,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    };
    run_migrations(config.clone()).await.unwrap();
    let db = connect_database(config.clone()).await.unwrap();
    let repo = Arc::new(PostgresFleetRepository::new(
        connect_database(config).await.unwrap(),
    ));
    repo.ensure_runtime_templates().await.unwrap();
    let agent = repo
        .create_agent(
            CreateAgentRequest {
                kind: AgentKind::Hermes,
                product_role: AgentProductRole::Executor,
                role: AgentRole::Developer,
                sdlc_role: None,
                display_name: "Log regression".into(),
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
    (repo, db, agent.id)
}

#[tokio::test]
async fn insert_log_returns_its_row_when_a_newer_log_already_exists() {
    let (repo, db, agent_id) = fixture().await;
    let competitor_id = Uuid::new_v4();
    let trigger = format!("log_race_{}", Uuid::new_v4().simple());
    // Make the competing write happen before INSERT returns, without a scheduling race.
    db.execute_unprepared(&format!(
        "CREATE FUNCTION {trigger}() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
           INSERT INTO agent_logs(id, agent_id, stream, message, created_at)
           VALUES ('{competitor_id}', NEW.agent_id, 'competitor', 'newer log',
                   NEW.created_at + interval '1 minute');
           RETURN NEW;
         END $$;
         CREATE TRIGGER {trigger} AFTER INSERT ON agent_logs FOR EACH ROW
         WHEN (NEW.agent_id = '{agent_id}'::uuid AND NEW.stream = 'stdout')
         EXECUTE FUNCTION {trigger}();"
    ))
    .await
    .unwrap();

    let result = repo
        .insert_log(
            agent_id,
            "stdout",
            "original api_key=never-store-this-value",
        )
        .await;
    db.execute_unprepared(&format!(
        "DROP TRIGGER {trigger} ON agent_logs; DROP FUNCTION {trigger}();"
    ))
    .await
    .unwrap();

    let inserted = result.expect("a newer log must not hide the successfully inserted row");
    assert_ne!(inserted.id, competitor_id);
    assert_eq!(inserted.agent_id, agent_id);
    assert_eq!(inserted.stream, "stdout");
    assert_eq!(inserted.message, "original api_key=redacted");

    let logs = repo.list_logs(Some(agent_id), 2).await.unwrap();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].id, competitor_id);
    let persisted = &logs[1];
    assert_eq!(persisted.id, inserted.id);
    assert_eq!(persisted.agent_id, inserted.agent_id);
    assert_eq!(persisted.stream, inserted.stream);
    assert_eq!(persisted.message, inserted.message);
    assert_eq!(persisted.created_at, inserted.created_at);
}

#[tokio::test]
async fn concurrent_log_writers_each_return_their_own_redacted_row() {
    let (repo, _, agent_id) = fixture().await;
    let mut writers = JoinSet::new();
    for index in 0..64 {
        let repo = repo.clone();
        writers.spawn(async move {
            let stream = if index % 2 == 0 { "stdout" } else { "stderr" };
            let message = format!("writer-{index} token=never-store-this-value");
            let inserted = repo.insert_log(agent_id, stream, &message).await.unwrap();
            assert_eq!(inserted.agent_id, agent_id);
            assert_eq!(inserted.stream, stream);
            assert_eq!(inserted.message, format!("writer-{index} token=redacted"));
            inserted
        });
    }
    let mut inserted = Vec::new();
    while let Some(writer) = writers.join_next().await {
        inserted.push(writer.unwrap());
    }
    assert_eq!(inserted.len(), 64);
    assert_eq!(
        inserted
            .iter()
            .map(|row| row.id)
            .collect::<HashSet<_>>()
            .len(),
        64
    );
    let persisted = repo.list_logs(Some(agent_id), 100).await.unwrap();
    assert_eq!(persisted.len(), 64);
    for result in inserted {
        let row = persisted.iter().find(|row| row.id == result.id).unwrap();
        assert_eq!(row.agent_id, result.agent_id);
        assert_eq!(row.stream, result.stream);
        assert_eq!(row.message, result.message);
        assert_eq!(row.created_at, result.created_at);
    }
}

#[tokio::test]
async fn rejected_log_insert_returns_an_error_without_a_phantom_row() {
    let (repo, db, _) = fixture().await;
    let missing_agent_id = Uuid::new_v4();
    assert!(
        repo.insert_log(missing_agent_id, "stderr", "failed")
            .await
            .is_err()
    );
    let row = db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT count(*)::bigint AS count FROM agent_logs WHERE agent_id = $1",
            [missing_agent_id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 0);
}
