use super::*;

async fn bind_namespace(repo: &PostgresFleetRepository, id: Uuid) {
    repo.update_agent(
        id,
        domain::UpdateAgentRequest {
            product_role: None,
            role: None,
            sdlc_role: None,
            display_name: None,
            description: None,
            namespace_id: Some("hermes-developer".into()),
            workflow_id: None,
            executor_ids: None,
        },
    )
    .await
    .unwrap();
}

async fn wait_for_preparation_lock(db: &sea_orm::DatabaseConnection, count: i64) {
    for _ in 0..200 {
        let row = db.query_one(Statement::from_string(DatabaseBackend::Postgres,
            "SELECT count(*) AS n FROM pg_stat_activity WHERE datname = current_database() AND wait_event_type = 'Lock' AND query LIKE 'SELECT id, kind, sdlc_role, namespace_id FROM agents%FOR UPDATE'"))
            .await.unwrap().unwrap();
        if row.try_get::<i64>("", "n").unwrap() >= count {
            return;
        }
        sleep(Duration::from_millis(50)).await;
    }
    panic!("preparation did not reach the guarded write boundary");
}

#[tokio::test]
async fn base_package_concurrent_preparation_fences_revision_and_identity() {
    let Some(checkout) = std::env::var("FLEET_TEST_BASE_PACKAGE_CHECKOUT").ok() else {
        return;
    };
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let id = agent(&repo).await;
    bind_namespace(&repo, id).await;
    repo.create_config_revision(id, configuration(), owner)
        .await
        .unwrap();
    let db = connect_database(DatabaseConfig {
        url: std::env::var("FLEET_TEST_DATABASE_URL").unwrap(),
        max_connections: 4,
        min_connections: 1,
        connect_timeout_seconds: 10,
        idle_timeout_seconds: 60,
    })
    .await
    .unwrap();
    let repo = Arc::new(repo);
    let lock = db.begin().await.unwrap();
    lock.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id = $1 FOR UPDATE",
        [id.into()],
    ))
    .await
    .unwrap();
    let left = {
        let repo = repo.clone();
        let checkout = checkout.clone();
        tokio::spawn(async move {
            repo.prepare_base_package_revision(id, &checkout, owner)
                .await
        })
    };
    let right = {
        let repo = repo.clone();
        let checkout = checkout.clone();
        tokio::spawn(async move {
            repo.prepare_base_package_revision(id, &checkout, owner)
                .await
        })
    };
    wait_for_preparation_lock(&db, 2).await;
    lock.commit().await.unwrap();
    let left = left.await.unwrap();
    let right = right.await.unwrap();
    assert_ne!(left.is_ok(), right.is_ok());
    let conflict = left.err().or_else(|| right.err()).unwrap();
    assert!(matches!(conflict, shared::AppError::Conflict(_)));
    assert_eq!(repo.list_config_revisions(id).await.unwrap().len(), 2);

    let lock = db.begin().await.unwrap();
    lock.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id = $1 FOR UPDATE",
        [id.into()],
    ))
    .await
    .unwrap();
    let pending = {
        let repo = repo.clone();
        tokio::spawn(async move {
            repo.prepare_base_package_revision(id, &checkout, owner)
                .await
        })
    };
    wait_for_preparation_lock(&db, 1).await;
    lock.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET sdlc_role = 'tester' WHERE id = $1",
        [id.into()],
    ))
    .await
    .unwrap();
    lock.commit().await.unwrap();
    assert!(matches!(
        pending.await.unwrap(),
        Err(shared::AppError::Conflict(_))
    ));
    assert_eq!(repo.list_config_revisions(id).await.unwrap().len(), 2);
}

#[tokio::test]
async fn base_package_draft_uses_real_pin_preserves_effective_and_does_not_install() {
    let Some(checkout) = std::env::var("FLEET_TEST_BASE_PACKAGE_CHECKOUT").ok() else {
        return;
    };
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let id = agent(&repo).await;
    bind_namespace(&repo, id).await;
    let previous = repo
        .create_config_revision(id, configuration(), owner)
        .await
        .unwrap();
    let package = repo
        .prepare_base_package_revision(id, &checkout, owner)
        .await
        .unwrap();
    assert_eq!(package.state, "draft");
    assert!(package.is_desired);
    assert!(!package.is_effective);
    assert!(!package.draining);
    assert_eq!(package.revision, previous.revision + 1);
    assert_eq!(
        package.snapshot.config.config_json["fleet_sdlc_package"]["commit"],
        infra::base_package::BASE_PACKAGE_COMMIT
    );
    assert_eq!(
        package.snapshot.config.config_json["model"],
        previous.snapshot.config.config_json["model"]
    );
    assert_eq!(
        package.snapshot.config.env_json,
        previous.snapshot.config.env_json
    );
    let enabled: Vec<_> = package
        .snapshot
        .skills
        .iter()
        .filter(|skill| skill.state == domain::SkillState::Enabled)
        .collect();
    assert_eq!(enabled.len(), 7);
    for skill in enabled {
        assert_eq!(skill.agent_id, id);
        assert!(skill.content.is_some());
    }
    assert!(!package.snapshot.config.soul_md.is_empty());
    assert_eq!(repo.list_config_revisions(id).await.unwrap().len(), 2);
    assert!(!repo.agent_is_draining(id).await.unwrap());
    // Skills in a draft do not update the installed/database inventory before activation.
    assert!(
        repo.list_agent_skills(id)
            .await
            .unwrap()
            .iter()
            .all(|skill| !skill.source.starts_with("base-sdlc:"))
    );
    assert!(repo.claim_config_activation().await.unwrap().is_none());
    repo.verify_base_package_revision(id, package.revision, &checkout)
        .await
        .unwrap();
    repo.validate_config_revision(id, package.revision, vec![])
        .await
        .unwrap();
    repo.update_agent(
        id,
        domain::UpdateAgentRequest {
            product_role: None,
            role: None,
            sdlc_role: Some(SdlcRole::Tester),
            display_name: None,
            description: None,
            namespace_id: None,
            workflow_id: None,
            executor_ids: None,
        },
    )
    .await
    .unwrap();
    assert!(
        repo.request_config_activation(id, package.revision, owner)
            .await
            .is_err()
    );
    assert!(!repo.agent_is_draining(id).await.unwrap());
}

#[tokio::test]
async fn base_package_draft_rejects_wrong_identity_and_drain_without_new_revision() {
    let Some(checkout) = std::env::var("FLEET_TEST_BASE_PACKAGE_CHECKOUT").ok() else {
        return;
    };
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let id = agent(&repo).await;
    let previous = repo
        .create_config_revision(id, configuration(), owner)
        .await
        .unwrap();
    assert!(
        repo.prepare_base_package_revision(id, &checkout, owner)
            .await
            .is_err()
    );
    assert_eq!(repo.list_config_revisions(id).await.unwrap().len(), 1);
    bind_namespace(&repo, id).await;
    repo.validate_config_revision(id, previous.revision, vec![])
        .await
        .unwrap();
    repo.request_config_activation(id, previous.revision, owner)
        .await
        .unwrap();
    assert!(
        repo.update_agent(
            id,
            domain::UpdateAgentRequest {
                product_role: None,
                role: None,
                sdlc_role: Some(SdlcRole::Tester),
                display_name: None,
                description: None,
                namespace_id: None,
                workflow_id: None,
                executor_ids: None
            }
        )
        .await
        .is_err()
    );
    assert!(
        repo.prepare_base_package_revision(id, &checkout, owner)
            .await
            .is_err()
    );
    assert_eq!(repo.list_config_revisions(id).await.unwrap().len(), 1);
    repo.claim_config_activation().await.unwrap().unwrap();
    repo.finish_config_activation(id, previous.revision, None, true)
        .await
        .unwrap();
}

#[tokio::test]
async fn base_package_http_requires_operator_and_server_owned_checkout() {
    let Some((repo, owner, _)) = fixture().await else {
        return;
    };
    let id = agent(&repo).await;
    let repo = Arc::new(repo);
    let config = Arc::new(AppConfig::default());
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
        events,
        restart_tx,
    ));
    let endpoint = axum::Router::new().route(
        "/agents/{id}/config/base-package",
        axum::routing::post(api::routes::agents::prepare_base_package),
    );
    let router = axum::Router::new()
        .nest(
            "/operator",
            endpoint
                .clone()
                .layer(axum::Extension(api::middleware::CurrentUser {
                    id: owner,
                    role: domain::SystemRole::Operator,
                    is_system_admin: false,
                })),
        )
        .nest(
            "/user",
            endpoint.layer(axum::Extension(api::middleware::CurrentUser {
                id: owner,
                role: domain::SystemRole::User,
                is_system_admin: false,
            })),
        )
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    assert_eq!(
        client
            .post(format!("{base}/user/agents/{id}/config/base-package"))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::FORBIDDEN
    );
    let response = client
        .post(format!("{base}/operator/agents/{id}/config/base-package"))
        .json(&serde_json::json!({"checkout": "/client/path"}))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
    assert!(repo.list_config_revisions(id).await.unwrap().is_empty());
    server.abort();
}
