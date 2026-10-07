use super::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

#[tokio::test]
async fn recovered_namespace_exit_cancels_only_accepted_generation_and_replays_without_events() {
    for unknown in [false, true] {
        let Some((repo, agent, user, config, root)) =
            lifecycle_tests::fixture(AgentKind::Hermes).await
        else {
            return;
        };
        let mut config = container_lifecycle_tests::with_mapping_controller(
            container_lifecycle_tests::fake_creation(&config, &agent, false).await,
        );
        config.fleet.controller_recovery_enabled = true;
        let first = lifecycle_tests::supervisor(Arc::new(config.clone()), repo.clone());
        let launch = first
            .prepared_container(&agent, LaunchPhase::Regular)
            .await
            .unwrap();
        repo.claim_runtime_launch(&launch).await.unwrap();
        repo.observe_runtime_launch(&launch, "gateway_started", Some(12345))
            .await
            .unwrap();
        repo.update_agent_status(agent.id, AgentStatus::Running)
            .await
            .unwrap();
        let origin = format!("http://172.18.0.2:{}", agent.api_port.unwrap());
        repo.record_container_endpoint(&launch, 12345, &origin)
            .await
            .unwrap();
        let session = repo
            .create_session(
                domain::CreateSessionRequest {
                    primary_agent_id: Some(agent.id),
                    agent_id: None,
                    title: "Owned namespace stop".into(),
                    task_key: None,
                    leader_agent_id: None,
                    parent_session_id: None,
                    namespace_id: None,
                    idempotency_key: Some(Uuid::new_v4().to_string()),
                },
                user,
            )
            .await
            .unwrap();
        let message = repo
            .create_session_message(
                session.id,
                domain::CreateSessionMessageRequest {
                    body: "Keep this original prompt".into(),
                    author_agent_id: None,
                    message_kind: Some(domain::MessageKind::UserPrompt),
                    runtime_message_id: None,
                    idempotency_key: Some(Uuid::new_v4().to_string()),
                },
                user,
            )
            .await
            .unwrap();
        repo.db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
                [message.id.into()],
            ))
            .await
            .unwrap();
        let requested = format!("fleet:{}:{}", session.id, agent.id);
        let caps = json!({"object":"hermes.api_server.capabilities","platform":"hermes-agent",
            "auth":{"type":"bearer","required":true},
            "runtime":{"mode":"server_agent","tool_execution":"server","split_runtime":false},
            "features":{"run_submission":true,"run_status":true,"run_events_sse":true,"run_stop":true,
                "runs_idempotency":{"supported":true,"durable":true,"retention_seconds":86400}},
            "endpoints":{"runs":{"method":"POST","path":"/v1/runs"},
                "run_status":{"method":"GET","path":"/v1/runs/{run_id}"},
                "run_events":{"method":"GET","path":"/v1/runs/{run_id}/events"},
                "run_stop":{"method":"POST","path":"/v1/runs/{run_id}/stop"}},
            "fleet_launch":{"version":1,"launch_id":launch.id}});
        let prepared = repo
            .prepare_hermes_dispatch(app::HermesDispatchDraft {
                message_id: message.id,
                session_id: session.id,
                agent_id: agent.id,
                run_role: SessionRunRole::Primary,
                requested_session_id: requested.clone(),
                input: message.body.clone(),
                origin: origin.clone(),
                credential_fingerprint: "a".repeat(64),
                capabilities: caps,
            })
            .await
            .unwrap();
        repo.claim_hermes_submission(message.id, origin, "a".repeat(64))
            .await
            .unwrap()
            .unwrap();
        let native = format!("run_{}", Uuid::new_v4().simple());
        repo.accept_hermes_run(message.id, prepared.run.id, native.clone())
            .await
            .unwrap();
        repo.pin_hermes_run_session(
            prepared.run.id,
            native.clone(),
            requested,
            "native-owned-session".into(),
        )
        .await
        .unwrap();
        let approval = repo
            .upsert_runtime_approval_request(app::RuntimeApprovalCreate {
                session_id: session.id,
                session_run_id: prepared.run.id,
                agent_id: agent.id,
                runtime_run_id: native.clone(),
                runtime_approval_id: Some("owned-request".into()),
                prompt: "Permission remains ungranted".into(),
                detail: json!({}),
            })
            .await
            .unwrap();
        let legacy = Uuid::new_v4();
        repo.db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO session_agent_runs(id,session_id,agent_id,run_role,state,runtime_run_id,runtime_session_id)
             VALUES($1,$2,$3,'primary','waiting','legacy-run','legacy-session')",
            [legacy.into(),session.id.into(),agent.id.into()])).await.unwrap();
        let mut expected_transcript = repo.list_session_messages(session.id).await.unwrap();
        let expected_prompt = expected_transcript
            .iter_mut()
            .find(|stored| stored.id == message.id)
            .unwrap();
        expected_prompt.delivery_state = domain::MessageDeliveryState::Completed;
        expected_prompt.delivery_error = None;
        for marker in [
            format!("{}.started", launch.id),
            "known-start".into(),
            "controller-restart".into(),
        ] {
            tokio::fs::write(root.join("controller").join(marker), "1")
                .await
                .unwrap();
        }
        let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
        runtime
            .recover_container_controller(agent.id)
            .await
            .unwrap();
        if unknown {
            tokio::fs::write(root.join("controller/stop-before-exit-unknown"), "1")
                .await
                .unwrap();
            assert!(runtime.stop_container_locked(&agent).await.is_err());
            assert_eq!(
                repo.get_session_agent_run(prepared.run.id)
                    .await
                    .unwrap()
                    .state,
                SessionRunState::Running
            );
            assert_eq!(
                repo.list_session_approvals(session.id).await.unwrap()[0].state,
                domain::RuntimeApprovalState::Pending
            );
            tokio::fs::write(
                root.join("controller")
                    .join(format!("{}.stopped", launch.id)),
                "1",
            )
            .await
            .unwrap();
        }
        runtime.stop_container_locked(&agent).await.unwrap();
        let stopped = repo.get_session_agent_run(prepared.run.id).await.unwrap();
        assert_eq!(stopped.state, SessionRunState::Cancelled);
        assert_eq!(stopped.runtime_run_id.as_deref(), Some(native.as_str()));
        assert_eq!(
            stopped.last_error.as_deref(),
            Some("Original runtime namespace exit confirmed")
        );
        assert_eq!(
            repo.get_session_agent_run(legacy).await.unwrap().state,
            SessionRunState::Waiting
        );
        let approvals = repo.list_session_approvals(session.id).await.unwrap();
        assert_eq!(approvals[0].id, approval.id);
        assert_eq!(approvals[0].state, domain::RuntimeApprovalState::Cancelled);
        assert!(approvals[0].resolved_by_user_id.is_none());
        let transcript = repo.list_session_messages(session.id).await.unwrap();
        assert_eq!(
            serde_json::to_value(&transcript).unwrap(),
            serde_json::to_value(expected_transcript).unwrap()
        );
        let before = repo.session_event_cursor(session.id).await.unwrap();
        let outcome = repo
            .read_controller_stop(launch.id)
            .await
            .unwrap()
            .unwrap()
            .native_outcome
            .unwrap();
        for _ in 0..3 {
            repo.settle_controller_stop(launch.id, &outcome)
                .await
                .unwrap();
        }
        let after = repo.session_event_cursor(session.id).await.unwrap();
        assert_eq!(before, after);
        let row = repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT count(*) AS count FROM audit_log WHERE action='runtime.namespace_exit.run_cancelled' AND entity_id=$1",
            [prepared.run.id.to_string().into()])).await.unwrap().unwrap();
        assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}

#[tokio::test]
async fn recovered_stop_has_one_concurrent_claim_and_immutable_redacted_outcome() {
    let Some((repo, agent, _, owner, root)) =
        container_lifecycle_tests::controller_heartbeat_fixture().await
    else {
        return;
    };
    let command = container_lifecycle_tests::recovery_command(&owner);
    let retained = repo.retain_controller_stop(&command).await.unwrap();
    assert!(retained.dispatch_command.is_none());
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..12 {
        let repo = repo.clone();
        let command = command.clone();
        tasks.spawn(async move { repo.claim_controller_stop(&command).await.unwrap() });
    }
    let mut winners = 0;
    while let Some(result) = tasks.join_next().await {
        winners += u32::from(result.unwrap());
    }
    assert_eq!(winners, 1);
    for sql in [
        "DELETE FROM runtime_controller_stop_deliveries WHERE launch_id=$1",
        "UPDATE runtime_controller_stop_deliveries SET intent_sha256=repeat('f',64) WHERE launch_id=$1",
        "UPDATE runtime_controller_stop_deliveries SET dispatch_command=NULL,dispatch_command_sha256=NULL WHERE launch_id=$1",
    ] {
        assert!(
            repo.db
                .execute(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    sql,
                    [owner.request.launch_id.into()]
                ))
                .await
                .is_err()
        );
    }
    assert!(
        repo.db
            .execute_unprepared("TRUNCATE runtime_controller_stop_deliveries")
            .await
            .is_err()
    );
    let launch = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    let registration = &launch.binding.container.as_ref().unwrap().registration;
    let outcome = json!({"kind":"stop","receipt":{
        "contract_version":registration.contract_version,"operation_id":launch.binding.id,
        "container_id":registration.container_id,"resource_id":registration.resource_id,
        "generation":registration.generation,"snapshot_sha256":retained.intent.snapshot_sha256,
        "state":"observed","observation":"namespace_exited"}});
    for drift in [
        "generation",
        "operation_id",
        "snapshot_sha256",
        "observation",
        "extra",
    ] {
        let mut changed = outcome.clone();
        match drift {
            "generation" | "operation_id" => changed["receipt"][drift] = json!(Uuid::new_v4()),
            "snapshot_sha256" => changed["receipt"][drift] = json!("f".repeat(64)),
            "observation" => changed["receipt"][drift] = json!("running"),
            _ => changed["receipt"]["secret"] = json!("must-never-be-retained"),
        }
        assert!(
            repo.settle_controller_stop(launch.binding.id, &changed)
                .await
                .is_err(),
            "{drift}"
        );
    }
    repo.settle_controller_stop(launch.binding.id, &outcome)
        .await
        .unwrap();
    repo.settle_controller_stop(launch.binding.id, &outcome)
        .await
        .unwrap();
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    let fresh = repo.get_agent(agent.id).await.unwrap();
    assert_eq!(fresh.status, AgentStatus::Stopped);
    assert_eq!(fresh.runtime.pid, None);
    let row = repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*) AS count,min(payload::text) AS payload FROM audit_log WHERE action='runtime.controller_stop.outcome' AND entity_id=$1",
        [launch.binding.id.to_string().into()])).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    assert!(
        !row.try_get::<String>("", "payload")
            .unwrap()
            .contains("must-never")
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn recovered_stop_rejects_drift_before_claim_and_late_new_claim() {
    let Some((repo, _, _, owner, root)) =
        container_lifecycle_tests::controller_heartbeat_fixture().await
    else {
        return;
    };
    let command = container_lifecycle_tests::recovery_command(&owner);
    repo.retain_controller_stop(&command).await.unwrap();
    for drift in ["owner", "epoch", "version", "deadline"] {
        let mut changed = command.clone();
        match drift {
            "owner" => changed.request.controller_id = Uuid::new_v4(),
            "epoch" => changed.epoch += 1,
            "version" => changed.lease_version += 1,
            _ => changed.lease_expires_at = "2030-01-01T00:00:00+00:00".into(),
        }
        assert!(
            repo.claim_controller_stop(&changed).await.is_err(),
            "{drift}"
        );
    }
    tokio::time::sleep(std::time::Duration::from_secs(31)).await;
    assert!(repo.claim_controller_stop(&command).await.is_err());
    assert!(
        repo.read_controller_stop(owner.request.launch_id)
            .await
            .unwrap()
            .unwrap()
            .dispatch_command
            .is_none()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn recovered_stop_checks_native_lease_and_current_owner_before_dispatch() {
    let Some((repo, agent, runtime, owner, root)) =
        container_lifecycle_tests::controller_heartbeat_fixture().await
    else {
        return;
    };
    let foreign = lifecycle_tests::supervisor(runtime.config.clone(), repo.clone());
    assert!(foreign.stop_container_locked(&agent).await.is_err());
    assert!(
        repo.read_controller_stop(owner.request.launch_id)
            .await
            .unwrap()
            .is_none()
    );
    tokio::fs::write(root.join("controller/native-lease-expired"), "1")
        .await
        .unwrap();
    assert!(runtime.stop_container_locked(&agent).await.is_err());
    assert!(!root.join("controller/owner-stop-calls").exists());
    assert!(
        repo.read_controller_stop(owner.request.launch_id)
            .await
            .unwrap()
            .is_none()
    );
    tokio::fs::remove_file(root.join("controller/native-lease-expired"))
        .await
        .unwrap();
    let result = runtime.stop_container_locked(&agent).await.unwrap();
    assert_eq!(result.status, AgentStatus::Stopped);
    let stopped = repo.get_agent(agent.id).await.unwrap();
    let before = serde_json::to_value(&stopped.runtime).unwrap();
    for _ in 0..3 {
        // Deliberately pass the old running snapshot as the background watcher does.
        let health = app::RuntimeSupervisor::health(&runtime, &agent)
            .await
            .unwrap();
        assert_eq!(health.status, AgentStatus::Stopped);
        let observed = repo.get_agent(agent.id).await.unwrap();
        assert_eq!(
            observed.runtime.health_status.as_deref(),
            Some("gateway_exited")
        );
        assert_eq!(serde_json::to_value(&observed.runtime).unwrap(), before);
    }
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/owner-stop-calls"))
            .await
            .unwrap()
            .lines()
            .count(),
        1
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn recovered_stop_lost_reply_settles_by_observation_without_second_kill() {
    let Some((repo, agent, runtime, owner, root)) =
        container_lifecycle_tests::controller_heartbeat_fixture().await
    else {
        return;
    };
    tokio::fs::write(root.join("controller/stop-after-exit-unknown"), "1")
        .await
        .unwrap();
    assert!(runtime.stop_container_locked(&agent).await.is_err());
    let saved = repo
        .read_controller_stop(owner.request.launch_id)
        .await
        .unwrap()
        .unwrap();
    assert!(saved.dispatch_command.is_some() && saved.native_outcome.is_none());
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        runtime.stop_container_locked(&agent).await.unwrap().status,
        AgentStatus::Stopped
    );
    let saved = repo
        .read_controller_stop(owner.request.launch_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved.native_outcome.unwrap()["kind"], "observe");
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/owner-stop-calls"))
            .await
            .unwrap()
            .lines()
            .count(),
        1
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn recovered_stop_unknown_acceptance_keeps_running_namespace_held() {
    let Some((repo, agent, runtime, owner, root)) =
        container_lifecycle_tests::controller_heartbeat_fixture().await
    else {
        return;
    };
    tokio::fs::write(root.join("controller/stop-before-exit-unknown"), "1")
        .await
        .unwrap();
    assert!(runtime.stop_container_locked(&agent).await.is_err());
    tokio::fs::remove_file(root.join("controller/stop-before-exit-unknown"))
        .await
        .unwrap();
    assert!(runtime.stop_container_locked(&agent).await.is_err());
    assert!(runtime.gateway_launch_generation(agent.id).await.is_err());
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        repo.read_controller_stop(owner.request.launch_id)
            .await
            .unwrap()
            .unwrap()
            .native_outcome
            .is_none()
    );
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/owner-stop-calls"))
            .await
            .unwrap()
            .lines()
            .count(),
        1
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn recovered_stop_late_exit_readback_never_renews_expired_custody() {
    let Some((repo, agent, runtime, owner, root)) =
        container_lifecycle_tests::controller_heartbeat_fixture().await
    else {
        return;
    };
    tokio::fs::write(root.join("controller/stop-after-exit-unknown"), "1")
        .await
        .unwrap();
    assert!(runtime.stop_container_locked(&agent).await.is_err());
    let before = repo
        .read_controller_stop(owner.request.launch_id)
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(31)).await;
    assert_eq!(
        runtime.stop_container_locked(&agent).await.unwrap().status,
        AgentStatus::Stopped
    );
    let after = repo
        .read_controller_stop(owner.request.launch_id)
        .await
        .unwrap()
        .unwrap();
    assert!(before.intent == after.intent && before.dispatch_command == after.dispatch_command);
    assert_eq!(after.native_outcome.unwrap()["kind"], "observe");
    let latest = repo
        .current_controller_recovery(owner.request.launch_id)
        .await
        .unwrap()
        .unwrap();
    assert!(!latest.lease_valid);
    assert_eq!(latest.lease_version, owner.lease_version);
    assert_eq!(latest.lease_expires_at, owner.lease_expires_at);
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/owner-stop-calls"))
            .await
            .unwrap()
            .lines()
            .count(),
        1
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}
