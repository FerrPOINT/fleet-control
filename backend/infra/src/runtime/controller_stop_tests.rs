use super::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

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
