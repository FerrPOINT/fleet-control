use app::FleetRepository;
use infra::{PostgresFleetRepository, connect_database, run_migrations};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use shared::DatabaseConfig;
use std::{collections::HashSet, sync::Arc};
use tokio::task::JoinSet;
use uuid::Uuid;

async fn fixture() -> (Arc<PostgresFleetRepository>, DatabaseConnection) {
    let config = DatabaseConfig {
        url: std::env::var("FLEET_TEST_DATABASE_URL")
            .expect("agent event regressions require isolated PostgreSQL"),
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
    (repo, db)
}

#[tokio::test]
async fn insert_event_returns_its_row_when_a_newer_event_already_exists() {
    let (repo, db) = fixture().await;
    let event_type = format!("event-race-{}", Uuid::new_v4());
    let competitor_id = Uuid::new_v4();
    let trigger = format!("event_race_{}", Uuid::new_v4().simple());
    // Force the competing write before INSERT returns, without timing assumptions.
    db.execute_unprepared(&format!(
        "CREATE FUNCTION {trigger}() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
           INSERT INTO agent_events(id, agent_id, event_type, message, payload, created_at)
           VALUES ('{competitor_id}', NULL, 'competitor', 'newer event', '{{}}'::jsonb,
                   NEW.created_at + interval '1 minute');
           RETURN NEW;
         END $$;
         CREATE TRIGGER {trigger} AFTER INSERT ON agent_events FOR EACH ROW
         WHEN (NEW.event_type = '{event_type}') EXECUTE FUNCTION {trigger}();"
    ))
    .await
    .unwrap();

    let payload = serde_json::json!({"source": "original", "sequence": 1});
    let result = repo
        .insert_event(None, &event_type, "original event", payload.clone())
        .await;
    db.execute_unprepared(&format!(
        "DROP TRIGGER {trigger} ON agent_events; DROP FUNCTION {trigger}();"
    ))
    .await
    .unwrap();

    let inserted = result.expect("a newer event must not hide a successfully inserted row");
    assert_ne!(inserted.id, competitor_id);
    assert_eq!(inserted.agent_id, None);
    assert_eq!(inserted.event_type, event_type);
    assert_eq!(inserted.message, "original event");
    assert_eq!(inserted.payload, payload);
    let persisted = repo.list_events(1000).await.unwrap();
    assert!(persisted.iter().any(|row| row.id == competitor_id));
    let row = persisted.iter().find(|row| row.id == inserted.id).unwrap();
    assert_eq!(row.agent_id, inserted.agent_id);
    assert_eq!(row.event_type, inserted.event_type);
    assert_eq!(row.message, inserted.message);
    assert_eq!(row.payload, inserted.payload);
    assert_eq!(row.created_at, inserted.created_at);
}

#[tokio::test]
async fn concurrent_event_writers_each_return_their_own_persisted_row() {
    let (repo, _) = fixture().await;
    let event_type = format!("event-writers-{}", Uuid::new_v4());
    let mut writers = JoinSet::new();
    for index in 0..64 {
        let repo = repo.clone();
        let event_type = event_type.clone();
        writers.spawn(async move {
            let message = format!("writer-{index}");
            let payload = serde_json::json!({"writer": index});
            let inserted = repo
                .insert_event(None, &event_type, &message, payload.clone())
                .await
                .unwrap();
            assert_eq!(inserted.agent_id, None);
            assert_eq!(inserted.event_type, event_type);
            assert_eq!(inserted.message, message);
            assert_eq!(inserted.payload, payload);
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
    let persisted = repo.list_events(1000).await.unwrap();
    for result in inserted {
        let row = persisted.iter().find(|row| row.id == result.id).unwrap();
        assert_eq!(row.event_type, result.event_type);
        assert_eq!(row.message, result.message);
        assert_eq!(row.payload, result.payload);
        assert_eq!(row.created_at, result.created_at);
    }
}
