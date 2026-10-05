use super::lifecycle_tests::{fixture, supervisor};
use super::*;
use app::runtime_launch::RuntimeLaunchBinding;
use domain::UpdateAgentConfigRequest;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};

fn command() -> Command {
    let mut command = Command::new("sleep");
    command
        .arg("30")
        .env_clear()
        .env("API_SERVER_KEY", "never-public-launch-secret");
    command
}

#[test]
fn runtime_launch_dispatch_binding_rejects_untrusted_shape_and_noncanonical_ids() {
    let id = Uuid::new_v4();
    let good = json!({"fleet_launch":{"version":1,"launch_id":id}});
    assert_eq!(
        crate::runtime_launches::dispatch_launch_id(&good).unwrap(),
        Some(id)
    );
    for bad in [
        Value::Null,
        json!({"version":1}),
        json!({"version":1.0,"launch_id":id}),
        json!({"version":2,"launch_id":id}),
        json!({"version":1,"launch_id":Uuid::nil()}),
        json!({"version":1,"launch_id":id.simple().to_string()}),
        json!({"version":1,"launch_id":id,"secret":"untrusted"}),
    ] {
        assert!(crate::runtime_launches::dispatch_launch_id(&json!({"fleet_launch":bad})).is_err());
    }
    for legacy in [
        json!({}),
        json!({"fleet_launch":{"version":1,"launch_id":null}}),
    ] {
        assert!(
            crate::runtime_launches::dispatch_launch_id(&legacy)
                .unwrap()
                .is_none()
        );
    }
}

#[tokio::test]
async fn runtime_launch_prepared_dispatch_cannot_follow_gateway_replacement() {
    let Some((repo, agent, _, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let runtime = supervisor(config, repo.clone());
    let original = bind(&runtime, &agent).await;
    let mut child = command().spawn().unwrap();
    let pid = i32::try_from(child.id().unwrap()).unwrap();
    runtime
        .record_native_spawn(agent.id, Some(pid))
        .await
        .unwrap();
    runtime.children.lock().await.insert(agent.id, child);
    let capabilities = json!({"fleet_launch":{"version":1,"launch_id":original.id}});
    runtime
        .verify_dispatch_launch(agent.id, &capabilities)
        .await
        .unwrap();
    assert!(
        runtime
            .verify_dispatch_launch(agent.id, &json!({}))
            .await
            .is_err()
    );
    runtime.stop_locked(&agent).await.unwrap();
    assert!(
        runtime
            .verify_dispatch_launch(agent.id, &capabilities)
            .await
            .is_err()
    );
    let current = repo.get_agent(agent.id).await.unwrap();
    let replacement = bind(&runtime, &current).await;
    child = command().spawn().unwrap();
    let pid = i32::try_from(child.id().unwrap()).unwrap();
    runtime
        .record_native_spawn(agent.id, Some(pid))
        .await
        .unwrap();
    runtime.children.lock().await.insert(agent.id, child);
    assert_ne!(replacement.id, original.id);
    assert!(
        runtime
            .verify_dispatch_launch(agent.id, &capabilities)
            .await
            .is_err()
    );
    runtime
        .verify_dispatch_launch(
            agent.id,
            &json!({"fleet_launch":{"version":1,"launch_id":replacement.id}}),
        )
        .await
        .unwrap();
    runtime.stop_locked(&current).await.unwrap();
    tokio::fs::remove_dir_all(root).await.unwrap();
}

async fn bind(runtime: &LocalRuntimeSupervisor, agent: &Agent) -> RuntimeLaunchBinding {
    runtime
        .prepare_native_launch(agent, &command(), LaunchPhase::Regular)
        .await
        .unwrap();
    runtime
        .repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap()
        .binding
}

#[tokio::test]
async fn runtime_launch_concurrent_controllers_receive_one_spawn_permit() {
    let Some((repo, agent, _, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let first = supervisor(config.clone(), repo.clone());
    let second = supervisor(config, repo.clone());
    let first_command = command();
    let second_command = command();
    let (a, b) = tokio::join!(
        first.prepare_native_launch(&agent, &first_command, LaunchPhase::Regular),
        second.prepare_native_launch(&agent, &second_command, LaunchPhase::Regular),
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let original = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(original.state, "claimed");
    assert!(original.pid.is_none());
    assert!(first.children.lock().await.is_empty());
    assert!(second.children.lock().await.is_empty());
    assert!(
        !serde_json::to_string(&original.binding)
            .unwrap()
            .contains("never-public-launch-secret")
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn runtime_launch_unknown_spawn_blocks_restart_stop_and_config_files() {
    let Some((repo, agent, owner, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let first = supervisor(config.clone(), repo.clone());
    let original = bind(&first, &agent).await;
    drop(first);
    // Ready metadata cannot hide a crash between claim commit and status write.
    assert_eq!(
        repo.get_agent(agent.id).await.unwrap().status,
        AgentStatus::Ready
    );
    let recovered = supervisor(config, repo.clone());
    assert!(matches!(
        recovered.start(&agent).await,
        Err(AppError::Unavailable(_))
    ));
    assert!(matches!(
        recovered.stop(&agent).await,
        Err(AppError::Unavailable(_))
    ));
    assert_eq!(
        recovered.health_locked(&agent).await.unwrap().status,
        AgentStatus::Degraded
    );
    let revision = repo
        .create_config_revision(
            agent.id,
            UpdateAgentConfigRequest {
                config_json: json!({}),
                soul_md: "Must not be written".into(),
                env_json: json!({}),
            },
            owner,
        )
        .await
        .unwrap();
    let mut journal = None;
    assert!(matches!(
        recovered
            .apply_config_revision(&revision, &mut journal)
            .await,
        Err(AppError::Unavailable(_))
    ));
    assert!(journal.is_none());
    assert!(
        !std::path::Path::new(&agent.paths.config)
            .join("SOUL.md")
            .exists()
    );
    let replay = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(replay.binding.id, original.id);
    assert_eq!(replay.state, "claimed");
    assert!(repo.claim_runtime_launch(&original).await.is_err());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn runtime_launch_pid_owner_and_history_are_immutable() {
    let Some((repo, agent, _, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let runtime = supervisor(config, repo.clone());
    let original = bind(&runtime, &agent).await;
    let mut foreign = original.clone();
    foreign.controller_id = Uuid::new_v4();
    assert!(
        repo.observe_runtime_launch(&foreign, "spawn_failed", None)
            .await
            .is_err()
    );
    repo.observe_runtime_launch(&original, "gateway_started", Some(101))
        .await
        .unwrap();
    repo.observe_runtime_launch(&original, "gateway_started", Some(101))
        .await
        .unwrap();
    assert!(
        repo.observe_runtime_launch(&original, "gateway_exited", Some(102))
            .await
            .is_err()
    );
    assert!(
        repo.observe_runtime_launch(&original, "spawn_failed", None)
            .await
            .is_err()
    );
    assert!(repo.db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_launches SET binding=binding||'{\"phase\":\"rollback\"}'::jsonb WHERE id=$1", [original.id.into()]
    )).await.is_err());
    assert!(
        repo.db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "DELETE FROM runtime_launches WHERE id=$1",
                [original.id.into()]
            ))
            .await
            .is_err()
    );
    repo.observe_runtime_launch(&original, "gateway_exited", Some(101))
        .await
        .unwrap();
    repo.observe_runtime_launch(&original, "gateway_exited", Some(101))
        .await
        .unwrap();
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.observe_runtime_launch(&original, "gateway_started", Some(101))
            .await
            .is_err()
    );
    let mut next = original.clone();
    next.id = Uuid::new_v4();
    repo.claim_runtime_launch(&next).await.unwrap();
    let stale = |status: AgentStatus, pid: Option<i32>| RuntimeStatePatch {
        status,
        desired_state: if pid.is_some() {
            DesiredState::Running
        } else {
            DesiredState::Stopped
        },
        pid,
        health_status: Some("stale".into()),
        health_detail: None,
        last_capabilities_json: None,
        startup_command_redacted: None,
        started_at: None,
        stopped_at: None,
    };
    assert!(
        repo.update_runtime_state(agent.id, stale(AgentStatus::Stopped, None))
            .await
            .is_err()
    );
    repo.observe_runtime_launch(&next, "gateway_started", Some(202))
        .await
        .unwrap();
    let snapshot = serde_json::to_value(repo.get_agent(agent.id).await.unwrap()).unwrap();
    for patch in [
        stale(AgentStatus::Stopped, None),
        stale(AgentStatus::Running, Some(101)),
    ] {
        assert!(repo.update_runtime_state(agent.id, patch).await.is_err());
    }
    assert_eq!(
        serde_json::to_value(repo.get_agent(agent.id).await.unwrap()).unwrap(),
        snapshot
    );
    repo.observe_runtime_launch(&original, "gateway_exited", Some(101))
        .await
        .unwrap();
    let current = repo.get_agent(agent.id).await.unwrap();
    assert_eq!(current.runtime.pid, Some(202));
    assert_eq!(current.status, AgentStatus::Starting);
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn runtime_launch_missing_ack_cannot_become_dispatchable() {
    let Some((repo, agent, _, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let runtime = supervisor(config, repo.clone());
    let original = bind(&runtime, &agent).await;
    assert!(runtime.verify_gateway_launch(agent.id).await.is_err());
    repo.observe_runtime_launch(&original, "gateway_started", Some(101))
        .await
        .unwrap();
    // PostgreSQL PID alone cannot create local controller ownership or acknowledgement.
    assert!(runtime.verify_gateway_launch(agent.id).await.is_err());
    let recovered = supervisor(runtime.config.clone(), repo);
    assert!(recovered.verify_gateway_launch(agent.id).await.is_err());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn runtime_launch_definite_spawn_failure_closes_only_original_attempt() {
    let Some((repo, agent, _, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let mut config = (*config).clone();
    config.fleet.hermes_command = root.join("does-not-exist").to_string_lossy().into_owned();
    let runtime = supervisor(Arc::new(config), repo.clone());
    let result = runtime.start(&agent).await.unwrap();
    assert_eq!(result.status, AgentStatus::Failed);
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    let row = repo
        .db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT state,pid FROM runtime_launches WHERE agent_id=$1",
            [agent.id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<String>("", "state").unwrap(), "spawn_failed");
    assert!(row.try_get::<Option<i32>>("", "pid").unwrap().is_none());
    assert!(runtime.children.lock().await.is_empty());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn runtime_launch_binding_checks_current_paths_and_port() {
    let Some((repo, agent, _, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let runtime = supervisor(config, repo.clone());
    let original = bind(&runtime, &agent).await;
    repo.observe_runtime_launch(&original, "spawn_failed", None)
        .await
        .unwrap();
    let mut changed = original.clone();
    changed.id = Uuid::new_v4();
    changed.api_port = Some(1);
    assert!(matches!(
        repo.claim_runtime_launch(&changed).await,
        Err(AppError::Conflict(_))
    ));
    changed.api_port = original.api_port;
    changed.paths.config.push_str("/foreign");
    assert!(matches!(
        repo.claim_runtime_launch(&changed).await,
        Err(AppError::Conflict(_))
    ));
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn runtime_launch_rechecks_runtime_ownership_under_database_lock() {
    let Some((repo, agent, _, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let runtime = supervisor(config, repo.clone());
    repo.db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agent_runtime SET pid=123,desired_state='running' WHERE agent_id=$1",
            [agent.id.into()],
        ))
        .await
        .unwrap();
    assert!(matches!(
        runtime
            .prepare_native_launch(&agent, &command(), LaunchPhase::Regular)
            .await,
        Err(AppError::Unavailable(_))
    ));
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(runtime.children.lock().await.is_empty());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

async fn active_revision(
    repo: &crate::PostgresFleetRepository,
    agent: &Agent,
    owner: Uuid,
) -> domain::AgentConfigRevision {
    let revision = repo
        .create_config_revision(
            agent.id,
            UpdateAgentConfigRequest {
                config_json: json!({"model":"fixture"}),
                soul_md: "Snapshot".into(),
                env_json: json!({}),
            },
            owner,
        )
        .await
        .unwrap();
    repo.validate_config_revision(agent.id, revision.revision, vec![])
        .await
        .unwrap();
    repo.request_config_activation(agent.id, revision.revision, owner)
        .await
        .unwrap();
    repo.claim_config_activation().await.unwrap().unwrap();
    repo.finish_config_activation(agent.id, revision.revision, None, true)
        .await
        .unwrap();
    repo.get_effective_config_revision(agent.id)
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn runtime_launch_stale_effective_revision_and_snapshot_are_denied() {
    let Some((repo, agent, owner, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    active_revision(&repo, &agent, owner).await;
    let runtime = supervisor(config, repo.clone());
    let original = bind(&runtime, &agent).await;
    repo.observe_runtime_launch(&original, "spawn_failed", None)
        .await
        .unwrap();
    let mut changed = original.clone();
    changed.id = Uuid::new_v4();
    changed.configuration_sha256 = Some("a".repeat(64));
    assert!(repo.claim_runtime_launch(&changed).await.is_err());
    active_revision(&repo, &agent, owner).await;
    changed = original;
    changed.id = Uuid::new_v4();
    assert!(repo.claim_runtime_launch(&changed).await.is_err());
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn runtime_launch_activation_and_rollback_bind_distinct_revisions() {
    let Some((repo, agent, owner, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let effective = active_revision(&repo, &agent, owner).await;
    let next = repo
        .create_config_revision(
            agent.id,
            UpdateAgentConfigRequest {
                config_json: json!({"model":"next"}),
                soul_md: "Next".into(),
                env_json: json!({}),
            },
            owner,
        )
        .await
        .unwrap();
    repo.validate_config_revision(agent.id, next.revision, vec![])
        .await
        .unwrap();
    repo.request_config_activation(agent.id, next.revision, owner)
        .await
        .unwrap();
    repo.claim_config_activation().await.unwrap().unwrap();
    let runtime = supervisor(config, repo.clone());
    assert!(
        runtime
            .prepare_native_launch(&agent, &command(), LaunchPhase::Regular)
            .await
            .is_err()
    );
    runtime
        .prepare_native_launch(&agent, &command(), LaunchPhase::Activation(next.revision))
        .await
        .unwrap();
    let launch = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(launch.binding.configuration_revision, Some(next.revision));
    runtime.record_native_end(agent.id, false).await.unwrap();
    runtime
        .prepare_native_launch(&agent, &command(), LaunchPhase::Rollback)
        .await
        .unwrap();
    let launch = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        launch.binding.configuration_revision,
        Some(effective.revision)
    );
    assert_eq!(launch.binding.phase, "rollback");
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn runtime_launch_committed_ack_readback_requires_retained_original_child() {
    let Some((repo, agent, _, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let runtime = supervisor(config, repo.clone());
    let mut command = command();
    runtime
        .prepare_native_launch(&agent, &command, LaunchPhase::Regular)
        .await
        .unwrap();
    let child = command.kill_on_drop(true).spawn().unwrap();
    let pid = child.id().unwrap() as i32;
    runtime.children.lock().await.insert(agent.id, child);
    let original = {
        let mut launches = runtime.launches.lock().await;
        let launch = launches.get_mut(&agent.id).unwrap();
        launch.pid = Some(pid);
        launch.binding.clone()
    };
    // The DB committed but the controller did not receive its acknowledgement.
    repo.observe_runtime_launch(&original, "gateway_started", Some(pid))
        .await
        .unwrap();
    let recovered = supervisor(runtime.config.clone(), repo.clone());
    assert!(recovered.verify_gateway_launch(agent.id).await.is_err());
    runtime.verify_gateway_launch(agent.id).await.unwrap();
    assert_eq!(
        runtime.launches.lock().await.get(&agent.id).unwrap().state,
        "gateway_started"
    );
    runtime.stop_locked(&agent).await.unwrap();
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn runtime_launch_exit_metadata_failure_keeps_original_for_readonly_wait_retry() {
    let Some((repo, agent, _, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let runtime = supervisor(config, repo.clone());
    let mut command = command();
    runtime
        .prepare_native_launch(&agent, &command, LaunchPhase::Regular)
        .await
        .unwrap();
    let child = command.kill_on_drop(true).spawn().unwrap();
    let pid = child.id().unwrap() as i32;
    runtime.children.lock().await.insert(agent.id, child);
    runtime
        .record_native_spawn(agent.id, Some(pid))
        .await
        .unwrap();
    let original = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    let fault = format!("launch_exit_fault_{}", Uuid::new_v4().simple());
    repo.db.execute_unprepared(&format!("CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.agent_id='{}' AND NEW.pid IS NULL THEN RAISE EXCEPTION 'test metadata fault'; END IF;
        RETURN NEW; END $$;
        CREATE TRIGGER {fault} BEFORE UPDATE ON agent_runtime FOR EACH ROW EXECUTE FUNCTION {fault}();", agent.id))
        .await.unwrap();
    assert!(runtime.stop_locked(&agent).await.is_err());
    let held = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(held.binding.id, original.binding.id);
    assert_eq!(held.state, "gateway_started");
    assert_eq!(
        repo.get_agent(agent.id).await.unwrap().runtime.pid,
        Some(pid)
    );
    assert!(
        runtime
            .children
            .lock()
            .await
            .get_mut(&agent.id)
            .unwrap()
            .try_wait()
            .unwrap()
            .is_some()
    );
    repo.db
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON agent_runtime; DROP FUNCTION {fault}();"
        ))
        .await
        .unwrap();
    // The same already-waited child is replayed, not a second signal or a replacement process.
    runtime.stop_locked(&agent).await.unwrap();
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.get_agent(agent.id)
            .await
            .unwrap()
            .runtime
            .pid
            .is_none()
    );
    assert!(runtime.children.lock().await.is_empty());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn runtime_launch_lost_database_spawn_ack_keeps_child_and_blocks_health() {
    let Some((repo, agent, _, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let runtime = supervisor(config, repo.clone());
    let mut command = command();
    runtime
        .prepare_native_launch(&agent, &command, LaunchPhase::Regular)
        .await
        .unwrap();
    let original = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    let fault = format!("launch_ack_fault_{}", Uuid::new_v4().simple());
    repo.db.execute_unprepared(&format!("CREATE FUNCTION {fault}() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN IF NEW.id='{}' AND NEW.state='gateway_started' THEN RAISE EXCEPTION 'test ACK fault'; END IF;
        RETURN NEW; END $$;
        CREATE TRIGGER {fault} BEFORE UPDATE ON runtime_launches FOR EACH ROW EXECUTE FUNCTION {fault}();", original.binding.id))
        .await.unwrap();
    let child = command.kill_on_drop(true).spawn().unwrap();
    let pid = child.id().unwrap() as i32;
    runtime.children.lock().await.insert(agent.id, child);
    assert!(
        runtime
            .record_native_spawn(agent.id, Some(pid))
            .await
            .is_err()
    );
    let held = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(held.binding.id, original.binding.id);
    assert_eq!(held.state, "claimed");
    assert!(runtime.health_locked(&agent).await.is_err());
    assert_eq!(
        runtime.children.lock().await.get(&agent.id).unwrap().id(),
        Some(pid as u32)
    );
    repo.db
        .execute_unprepared(&format!(
            "DROP TRIGGER {fault} ON runtime_launches; DROP FUNCTION {fault}();"
        ))
        .await
        .unwrap();
    // Only the controller retaining the actual child may record its positively observed gateway exit.
    runtime.stop_locked(&agent).await.unwrap();
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn runtime_launch_real_child_exit_records_gateway_not_namespace_quiescence() {
    let Some((repo, agent, _, config, root)) = fixture(AgentKind::Hermes).await else {
        return;
    };
    let runtime = supervisor(config, repo.clone());
    let mut command = command();
    runtime
        .prepare_native_launch(&agent, &command, LaunchPhase::Regular)
        .await
        .unwrap();
    let child = command.kill_on_drop(true).spawn().unwrap();
    let pid = child.id().unwrap() as i32;
    runtime.children.lock().await.insert(agent.id, child);
    runtime
        .record_native_spawn(agent.id, Some(pid))
        .await
        .unwrap();
    runtime.verify_gateway_launch(agent.id).await.unwrap();
    runtime.stop_locked(&agent).await.unwrap();
    assert!(runtime.children.lock().await.is_empty());
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    let row = repo
        .db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT state,pid FROM runtime_launches WHERE agent_id=$1",
            [agent.id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.try_get::<String>("", "state").unwrap(),
        "gateway_exited"
    );
    assert_eq!(row.try_get::<i32>("", "pid").unwrap(), pid);
    tokio::fs::remove_dir_all(root).await.unwrap();
}
