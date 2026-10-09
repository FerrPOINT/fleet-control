use super::*;

async fn lookup_snapshot(f: &ControlFixture) -> Value {
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT jsonb_build_object(
            'commands',(SELECT jsonb_agg(to_jsonb(c) ORDER BY c.id) FROM runtime_control_commands c WHERE c.session_id=$1),
            'events',(SELECT jsonb_agg(to_jsonb(e) ORDER BY e.sequence) FROM session_events e WHERE e.session_id=$1),
            'outbox',(SELECT jsonb_agg(to_jsonb(o) ORDER BY o.message_id) FROM message_dispatch_outbox o
                WHERE o.message_id IN (SELECT m.id FROM session_messages m WHERE m.session_id=$1)),
            'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY a.id) FROM audit_log a WHERE a.entity_id IN
                (SELECT c.id::text FROM runtime_control_commands c WHERE c.session_id=$1)),
            'run',(SELECT to_jsonb(r) FROM session_agent_runs r WHERE r.id=$2)) AS snapshot",
        [f.run.session_id.into(), f.run.id.into()],
    ))
    .await
    .unwrap()
    .unwrap()
    .try_get("", "snapshot")
    .unwrap()
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn original_key_lookup_is_scoped_read_only_and_not_limited_to_latest100() {
    let f = ledger_fixture(stop_ack()).await.unwrap();
    let actor = f.actor();
    let first = f
        .repo
        .reserve_runtime_control(&f.run, &actor, domain::RuntimeControlOperation::Stop, None)
        .await
        .unwrap()
        .receipt;
    f.repo
        .retire_runtime_control(first.id, false)
        .await
        .unwrap();
    for _ in 0..101 {
        let next = f
            .repo
            .reserve_runtime_control(
                &f.run,
                &f.actor(),
                domain::RuntimeControlOperation::Stop,
                None,
            )
            .await
            .unwrap()
            .receipt;
        f.repo.retire_runtime_control(next.id, false).await.unwrap();
    }
    let latest = f
        .repo
        .list_runtime_controls(f.run.session_id, f.run.id)
        .await
        .unwrap();
    assert_eq!(latest.len(), 100);
    assert!(latest.iter().all(|receipt| receipt.id != first.id));
    let before = lookup_snapshot(&f).await;
    for _ in 0..2 {
        let found = f
            .repo
            .find_runtime_control_by_key(f.run.session_id, f.run.id, &actor)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(found.id, first.id);
        assert_eq!(found.state, domain::RuntimeControlState::Rejected);
        let wire = serde_json::to_value(found).unwrap();
        assert!(wire.get("idempotency_key").is_none());
        assert!(wire.get("payload_sha256").is_none());
        assert!(!wire.to_string().contains(&actor.idempotency_key));
    }
    for (session, run, candidate) in [
        (Uuid::new_v4(), f.run.id, actor.clone()),
        (f.run.session_id, Uuid::new_v4(), actor.clone()),
        (
            f.run.session_id,
            f.run.id,
            domain::RuntimeControlActor {
                user_id: Uuid::new_v4(),
                idempotency_key: actor.idempotency_key.clone(),
            },
        ),
        (f.run.session_id, f.run.id, f.actor()),
    ] {
        assert!(
            f.repo
                .find_runtime_control_by_key(session, run, &candidate)
                .await
                .unwrap()
                .is_none()
        );
    }
    for key in ["", "bad key", &"x".repeat(129)] {
        assert!(
            f.repo
                .find_runtime_control_by_key(
                    f.run.session_id,
                    f.run.id,
                    &domain::RuntimeControlActor {
                        user_id: f.owner,
                        idempotency_key: key.into()
                    }
                )
                .await
                .is_err()
        );
    }
    assert_eq!(lookup_snapshot(&f).await, before);
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}

async fn lookup_principal(
    f: &ControlFixture,
    ctx: &app::AppContext,
    role: domain::SystemRole,
) -> (Uuid, String) {
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let id = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
         VALUES($1,$2,$3,'Lookup principal','disabled',$4,$5)",
        [id.into(), format!("{id}@example.test").into(), id.to_string().into(),
         role.is_admin().into(), role.to_string().into()])).await.unwrap();
    let token = ctx
        .auth
        .issue_tokens(&f.repo.find_user_by_id(id).await.unwrap().unwrap())
        .unwrap()
        .response
        .access_token;
    (id, token)
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn lookup_http_auth_header_scope_and_single_receipt_never_fall_back_to_list() {
    let f = ledger_fixture(stop_ack()).await.unwrap();
    let actor = f.actor();
    let original = f
        .repo
        .reserve_runtime_control(&f.run, &actor, domain::RuntimeControlOperation::Stop, None)
        .await
        .unwrap()
        .receipt;
    f.repo
        .retire_runtime_control(original.id, false)
        .await
        .unwrap();
    let ctx = control_http_context(&f);
    let owner = ctx
        .auth
        .issue_tokens(&f.repo.find_user_by_id(f.owner).await.unwrap().unwrap())
        .unwrap()
        .response
        .access_token;
    let (_, foreign_token) = lookup_principal(&f, &ctx, domain::SystemRole::User).await;
    let (operator, operator_token) = lookup_principal(&f, &ctx, domain::SystemRole::Operator).await;
    let foreign_session = f
        .repo
        .create_session(chat(f.agent.id, "lookup-other-session"), f.owner)
        .await
        .unwrap();
    let router = control_http_routes()
        .route_layer(axum::middleware::from_fn_with_state(
            ctx.clone(),
            api::middleware::require_auth,
        ))
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    let collection = format!(
        "{base}/api/v1/sessions/{}/runs/{}/controls",
        f.run.session_id, f.run.id
    );
    let url = format!("{collection}/lookup");
    for (token, expected) in [
        (None, StatusCode::UNAUTHORIZED),
        (Some("invalid-token"), StatusCode::UNAUTHORIZED),
        (Some(foreign_token.as_str()), StatusCode::FORBIDDEN),
    ] {
        let mut request = client
            .get(&url)
            .header("Idempotency-Key", &actor.idempotency_key);
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        assert_eq!(request.send().await.unwrap().status(), expected);
    }
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(&owner)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    for key in ["", "bad key", &"x".repeat(129)] {
        assert_eq!(
            client
                .get(&url)
                .bearer_auth(&owner)
                .header("Idempotency-Key", key)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    for (first, second) in [("one", "two"), ("one", "one")] {
        assert_eq!(
            client
                .get(&url)
                .bearer_auth(&owner)
                .header("Idempotency-Key", first)
                .header("Idempotency-Key", second)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let before = lookup_snapshot(&f).await;
    let found = client
        .get(&url)
        .bearer_auth(&owner)
        .header("Idempotency-Key", &actor.idempotency_key)
        .send()
        .await
        .unwrap();
    assert_eq!(found.status(), StatusCode::OK);
    let found = found.json::<Value>().await.unwrap();
    assert!(found.is_object());
    assert_eq!(found["id"], original.id.to_string());
    assert_eq!(found["actor_user_id"], f.owner.to_string());
    assert!(found.get("idempotency_key").is_none());
    assert!(found.get("payload_sha256").is_none());
    assert!(!found.to_string().contains(&actor.idempotency_key));
    let concurrent_reads = futures_util::future::join_all((0..12).map(|index| {
        let request = if index % 2 == 0 {
            client.get(&url).bearer_auth(&owner)
        } else {
            client.get(&url).bearer_auth(&operator_token)
        };
        request
            .header("Idempotency-Key", &actor.idempotency_key)
            .send()
    }))
    .await;
    for (index, response) in concurrent_reads.into_iter().enumerate() {
        let response = response.unwrap();
        if index % 2 == 0 {
            assert_eq!(response.status(), StatusCode::OK);
            let receipt = response
                .json::<domain::RuntimeControlReceipt>()
                .await
                .unwrap();
            assert_eq!(receipt.id, original.id);
            assert_eq!(receipt.actor_user_id, f.owner);
        } else {
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
    }
    for (token, key) in [
        (&owner, "new-unknown-key"),
        (&operator_token, actor.idempotency_key.as_str()),
    ] {
        let missing = client
            .get(&url)
            .bearer_auth(token)
            .header("Idempotency-Key", key)
            .send()
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        assert!(!missing.text().await.unwrap().contains(key));
    }
    let list = client
        .get(&collection)
        .bearer_auth(&owner)
        .send()
        .await
        .unwrap()
        .json::<Vec<domain::RuntimeControlReceipt>>()
        .await
        .unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].id, original.id);
    assert_eq!(
        client
            .get(format!("{collection}/{}", original.id))
            .bearer_auth(&owner)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    for (session, run) in [
        (foreign_session.id, f.run.id),
        (f.run.session_id, Uuid::new_v4()),
    ] {
        assert_eq!(
            client
                .get(format!(
                    "{base}/api/v1/sessions/{session}/runs/{run}/controls/lookup"
                ))
                .bearer_auth(&owner)
                .header("Idempotency-Key", &actor.idempotency_key)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(lookup_snapshot(&f).await, before);
    let operator_receipt = f
        .repo
        .reserve_runtime_control(
            &f.run,
            &domain::RuntimeControlActor {
                user_id: operator,
                idempotency_key: actor.idempotency_key.clone(),
            },
            domain::RuntimeControlOperation::Stop,
            None,
        )
        .await
        .unwrap()
        .receipt;
    let own = client
        .get(&url)
        .bearer_auth(&operator_token)
        .header("Idempotency-Key", &actor.idempotency_key)
        .send()
        .await
        .unwrap();
    assert_eq!(own.status(), StatusCode::OK);
    let own = own.json::<domain::RuntimeControlReceipt>().await.unwrap();
    assert_eq!(own.id, operator_receipt.id);
    assert_eq!(own.actor_user_id, operator);
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE users SET is_active=false WHERE id=$1",
        [f.owner.into()],
    ))
    .await
    .unwrap();
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(&owner)
            .header("Idempotency-Key", &actor.idempotency_key)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    server.abort();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn lookup_http_sessionless_principal_and_forged_header_are_denied() {
    let f = ledger_fixture(stop_ack()).await.unwrap();
    let router = control_http_routes()
        .layer(axum::Extension(api::middleware::CurrentUser {
            id: f.owner,
            role: domain::SystemRole::Admin,
            is_system_admin: true,
            central_write: None,
        }))
        .with_state(control_http_context(&f));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let before = lookup_snapshot(&f).await;
    for (session, run) in [
        (f.run.session_id, f.run.id),
        (Uuid::new_v4(), Uuid::new_v4()),
    ] {
        assert_eq!(
            reqwest::Client::new()
                .get(format!(
                    "{base}/api/v1/sessions/{session}/runs/{run}/controls/lookup"
                ))
                .header("Idempotency-Key", "forged-human-key")
                .header("X-Verified-Human-Session", "true")
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(lookup_snapshot(&f).await, before);
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    server.abort();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn lookup_http_existing_project_guard_denies_unavailable_binding() {
    let f = ledger_fixture(stop_ack()).await.unwrap();
    let actor = f.actor();
    f.repo
        .reserve_runtime_control(&f.run, &actor, domain::RuntimeControlOperation::Stop, None)
        .await
        .unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let task = Uuid::new_v4();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
         VALUES($1,'lookup-fixture',$2,$3,$3,$4,'fixture-owner','lookup-bound')",
        [f.run.session_id.into(), Uuid::new_v4().into(), task.into(), f.agent.id.into()])).await.unwrap();
    let ctx = control_http_context(&f);
    let owner = ctx
        .auth
        .issue_tokens(&f.repo.find_user_by_id(f.owner).await.unwrap().unwrap())
        .unwrap()
        .response
        .access_token;
    let router = control_http_routes()
        .route_layer(axum::middleware::from_fn_with_state(
            ctx.clone(),
            api::middleware::require_auth,
        ))
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let before = lookup_snapshot(&f).await;
    assert_eq!(
        reqwest::Client::new()
            .get(format!(
                "{base}/api/v1/sessions/{}/runs/{}/controls/lookup",
                f.run.session_id, f.run.id
            ))
            .bearer_auth(owner)
            .header("Idempotency-Key", actor.idempotency_key)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(lookup_snapshot(&f).await, before);
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
    server.abort();
}

#[tokio::test]
#[ignore = "requires isolated FLEET_TEST_DATABASE_URL"]
async fn concurrent_original_key_reads_race_claim_without_mutation_or_new_permit() {
    let f = ledger_fixture(stop_ack()).await.unwrap();
    let actor = f.actor();
    let id = f
        .repo
        .reserve_runtime_control(&f.run, &actor, domain::RuntimeControlOperation::Stop, None)
        .await
        .unwrap()
        .receipt
        .id;
    let reads = futures_util::future::join_all((0..16).map(|_| {
        f.repo
            .find_runtime_control_by_key(f.run.session_id, f.run.id, &actor)
    }));
    let (reads, claimed) = tokio::join!(reads, f.repo.claim_runtime_control(id));
    assert!(claimed.unwrap());
    for receipt in reads {
        let receipt = receipt.unwrap().unwrap();
        assert_eq!(receipt.id, id);
        assert_eq!(receipt.actor_user_id, f.owner);
        assert!(matches!(
            receipt.state,
            domain::RuntimeControlState::Reserved | domain::RuntimeControlState::Submitted
        ));
    }
    let before = lookup_snapshot(&f).await;
    let stable = futures_util::future::join_all((0..16).map(|_| {
        f.repo
            .find_runtime_control_by_key(f.run.session_id, f.run.id, &actor)
    }))
    .await;
    for receipt in stable {
        let receipt = receipt.unwrap().unwrap();
        assert_eq!(receipt.id, id);
        assert_eq!(receipt.state, domain::RuntimeControlState::Submitted);
    }
    assert_eq!(lookup_snapshot(&f).await, before);
    assert!(!f.repo.claim_runtime_control(id).await.unwrap());
    assert!(matches!(
        f.repo
            .reserve_runtime_control(
                &f.run,
                &f.actor(),
                domain::RuntimeControlOperation::Stop,
                None
            )
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert_eq!(f.calls.load(Ordering::SeqCst), 0);
}
