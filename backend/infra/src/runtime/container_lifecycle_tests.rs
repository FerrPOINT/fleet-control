use super::*;
use app::runtime_launch::{
    ContainerEngineIdentity, ContainerRegistration, RuntimeContainerBinding, RuntimeLaunchBinding,
    RuntimeLaunchRecord,
};
use sea_orm::ConnectionTrait;
use sha2::{Digest, Sha256};
use std::path::Path;

#[test]
fn agent_container_projects_exclude_shared_infrastructure_and_unowned_names() {
    for name in ["sdlc1", "sdlc2", "sdlc-qa-container-test-123abc"] {
        assert!(crate::runtime_launches::valid_agent_container_project(name));
    }
    for name in [
        "sdlc-common",
        "sdlc-demo",
        "sdlc-build-container-test-123abc",
        "sdlc-qa-",
        "sdlc-qa--invalid",
        "sdlc-qa-Uppercase",
        "sdlc-qa-test/foreign",
        "",
    ] {
        assert!(
            !crate::runtime_launches::valid_agent_container_project(name),
            "{name}"
        );
    }
    assert!(!crate::runtime_launches::valid_agent_container_project(
        &format!("sdlc-qa-{}", "a".repeat(128))
    ));
}

fn binding(agent: &Agent, controller_id: Uuid) -> RuntimeLaunchBinding {
    let generation = Uuid::new_v4();
    let registration = ContainerRegistration {
        contract_version: 2,
        operation_id: Uuid::new_v4(),
        container_id: "a".repeat(64),
        resource_id: agent.id,
        generation,
        engine: ContainerEngineIdentity {
            id: "original-engine".into(),
            kernel_version: "original-kernel".into(),
            server_version: "29".into(),
        },
        policy_sha256: "b".repeat(64),
        inventory_sha256: "c".repeat(64),
        running_inventory_sha256: "d".repeat(64),
        compose_sha256: "e".repeat(64),
        network_sha256: Some("f".repeat(64)),
        mount_mapping_sha256: None,
    };
    let controller = Path::new(&agent.paths.config)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("controller");
    let policy = json!({"contract_version": 2, "project": "sdlc-qa-container-test-original", "service": "agent1-runtime",
        "resource_id":agent.id, "generation":generation, "mounts": [
            {"type":"bind", "source":agent.paths.runtime, "destination":"/runtime", "read_only":true},
            {"type":"bind", "source":agent.paths.config, "destination":"/config", "read_only":false},
            {"type":"bind", "source":agent.paths.workspace, "destination":"/workspace", "read_only":false},
            {"type":"bind", "source":agent.paths.logs, "destination":"/logs", "read_only":false}]});
    let container = RuntimeContainerBinding {
        registration,
        policy,
        compose: controller
            .join("compose.json")
            .to_string_lossy()
            .into_owned(),
        journal: controller
            .join("launches.sqlite")
            .to_string_lossy()
            .into_owned(),
        stop_journal: controller
            .join("stops.sqlite")
            .to_string_lossy()
            .into_owned(),
        source_sha256: ["1".repeat(64), "2".repeat(64), "3".repeat(64)],
        context: "desktop-linux".into(),
        mount_mapping: None,
        mapping_file: None,
    };
    RuntimeLaunchBinding {
        id: generation,
        agent_id: agent.id,
        controller_id,
        kind: AgentKind::Hermes,
        paths: agent.paths.clone(),
        api_port: agent.api_port,
        phase: "regular".into(),
        configuration_revision: None,
        configuration_sha256: None,
        command_sha256: crate::runtime_launches::snapshot_hash(
            &serde_json::to_value(&container).unwrap(),
        )
        .unwrap(),
        container: Some(container),
    }
}

#[tokio::test]
async fn container_endpoint_requires_original_started_custody_and_is_immutable() {
    let Some((repo, agent, _, _, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await else {
        return;
    };
    let launch = binding(&agent, Uuid::new_v4());
    repo.claim_runtime_launch(&launch).await.unwrap();
    let origin = format!("http://172.18.0.2:{}", agent.api_port.unwrap());
    assert!(
        repo.record_container_endpoint(&launch, 12345, &origin)
            .await
            .is_err()
    );
    repo.observe_runtime_launch(&launch, "gateway_started", Some(12345))
        .await
        .unwrap();
    let (first, replay) = tokio::join!(
        repo.record_container_endpoint(&launch, 12345, &origin),
        repo.record_container_endpoint(&launch, 12345, &origin)
    );
    first.unwrap();
    replay.unwrap();
    assert!(
        repo.record_container_endpoint(&launch, 12346, &origin)
            .await
            .is_err()
    );
    let other = format!("http://172.18.0.3:{}", agent.api_port.unwrap());
    assert!(
        repo.record_container_endpoint(&launch, 12345, &other)
            .await
            .is_err()
    );
    let mut foreign = launch.clone();
    foreign.controller_id = Uuid::new_v4();
    assert!(
        repo.record_container_endpoint(&foreign, 12345, &origin)
            .await
            .is_err()
    );
    for sql in [
        "UPDATE runtime_launch_endpoints SET origin=origin WHERE launch_id=$1",
        "DELETE FROM runtime_launch_endpoints WHERE launch_id=$1",
    ] {
        assert!(
            repo.db
                .execute(sea_orm::Statement::from_sql_and_values(
                    sea_orm::DatabaseBackend::Postgres,
                    sql,
                    [launch.id.into()]
                ))
                .await
                .is_err()
        );
    }
    assert!(
        repo.db
            .execute_unprepared("TRUNCATE runtime_launch_endpoints")
            .await
            .is_err()
    );
    repo.observe_runtime_launch(&launch, "gateway_exited", Some(12345))
        .await
        .unwrap();
    assert!(
        repo.record_container_endpoint(&launch, 12345, &origin)
            .await
            .is_err()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn container_dispatch_origin_requires_sealed_generation_not_an_arbitrary_private_ip() {
    let Some((repo, agent, _, _, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await else {
        return;
    };
    let launch = binding(&agent, Uuid::new_v4());
    repo.claim_runtime_launch(&launch).await.unwrap();
    repo.observe_runtime_launch(&launch, "gateway_started", Some(12345))
        .await
        .unwrap();
    let origin = format!("http://172.18.0.2:{}", agent.api_port.unwrap());
    let caps = json!({"fleet_launch":{"version":1,"launch_id":launch.id}});
    let repo_ref = &repo;
    let check = |address: String, capabilities: Value| async move {
        let row = repo_ref
            .db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT fleet_hermes_origin_matches($1,$2,$3) AS matched",
                [agent.id.into(), address.into(), capabilities.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        row.try_get::<bool>("", "matched").unwrap()
    };
    assert!(!check(origin.clone(), caps.clone()).await);
    repo.record_container_endpoint(&launch, 12345, &origin)
        .await
        .unwrap();
    assert!(check(origin.clone(), caps.clone()).await);
    for address in [
        format!("http://127.0.0.1:{}", agent.api_port.unwrap()),
        format!("http://172.18.0.3:{}", agent.api_port.unwrap()),
        "http://8.8.8.8:29100".into(),
    ] {
        assert!(!check(address, caps.clone()).await);
    }
    assert!(!check(origin.clone(), json!({})).await);
    assert!(
        !check(
            origin.clone(),
            json!({"fleet_launch":{"version":1,"launch_id":Uuid::new_v4()}})
        )
        .await
    );
    repo.observe_runtime_launch(&launch, "gateway_exited", Some(12345))
        .await
        .unwrap();
    assert!(!check(origin, caps).await);
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn container_approval_recovery_requires_the_accepted_sealed_gateway_origin() {
    let Some((repo, agent, owner, _, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let launch = binding(&agent, Uuid::new_v4());
    repo.claim_runtime_launch(&launch).await.unwrap();
    repo.observe_runtime_launch(&launch, "gateway_started", Some(12345))
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
                title: "Container approval recovery".into(),
                task_key: None,
                leader_agent_id: None,
                parent_session_id: None,
                namespace_id: None,
                idempotency_key: Some(Uuid::new_v4().to_string()),
            },
            owner,
        )
        .await
        .unwrap();
    let message = repo
        .create_session_message(
            session.id,
            domain::CreateSessionMessageRequest {
                body: "Perform the requested owned action".into(),
                author_agent_id: None,
                message_kind: Some(domain::MessageKind::UserPrompt),
                runtime_message_id: None,
                idempotency_key: Some(Uuid::new_v4().to_string()),
            },
            owner,
        )
        .await
        .unwrap();
    repo.db
        .execute(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "UPDATE message_dispatch_outbox SET state='dispatching' WHERE message_id=$1",
            [message.id.into()],
        ))
        .await
        .unwrap();
    repo.update_agent_status(agent.id, AgentStatus::Running)
        .await
        .unwrap();
    let requested = format!("fleet:{}:{}", session.id, agent.id);
    let capabilities = json!({
        "object":"hermes.api_server.capabilities", "platform":"hermes-agent",
        "auth":{"type":"bearer","required":true},
        "runtime":{"mode":"server_agent","tool_execution":"server","split_runtime":false},
        "features":{"run_submission":true,"run_status":true,"run_events_sse":true,"run_stop":true,
            "runs_idempotency":{"supported":true,"durable":true,"retention_seconds":86400}},
        "endpoints":{"runs":{"method":"POST","path":"/v1/runs"},
            "run_status":{"method":"GET","path":"/v1/runs/{run_id}"},
            "run_events":{"method":"GET","path":"/v1/runs/{run_id}/events"},
            "run_stop":{"method":"POST","path":"/v1/runs/{run_id}/stop"}},
        "fleet_launch":{"version":1,"launch_id":launch.id}
    });
    let fingerprint = "a".repeat(64);
    repo.prepare_hermes_dispatch(app::HermesDispatchDraft {
        message_id: message.id,
        session_id: session.id,
        agent_id: agent.id,
        run_role: SessionRunRole::Primary,
        requested_session_id: requested.clone(),
        input: message.body,
        origin: origin.clone(),
        credential_fingerprint: fingerprint.clone(),
        capabilities,
    })
    .await
    .unwrap();
    let run = repo
        .claim_hermes_submission(message.id, origin.clone(), fingerprint.clone())
        .await
        .unwrap()
        .unwrap()
        .run;
    let native_id = format!("run_{}", Uuid::new_v4().simple());
    let effective = format!("native:{}", Uuid::new_v4());
    repo.accept_hermes_run(message.id, run.id, native_id.clone())
        .await
        .unwrap();
    repo.pin_hermes_run_session(run.id, native_id.clone(), requested, effective.clone())
        .await
        .unwrap();
    let request = app::RuntimeApprovalCreate {
        session_id: session.id,
        session_run_id: run.id,
        agent_id: agent.id,
        runtime_run_id: native_id.clone(),
        runtime_approval_id: Some("container-recovered-approval".into()),
        prompt: "Review this owned action".into(),
        detail: json!({"event":"approval.request","run_id":native_id,
            "request_id":"container-recovered-approval","choices":["once","deny"]}),
    };
    for rejected_origin in [
        format!("http://127.0.0.1:{}", agent.api_port.unwrap()),
        format!("http://172.18.0.3:{}", agent.api_port.unwrap()),
    ] {
        assert!(
            repo.recover_hermes_approval(
                request.clone(),
                effective.clone(),
                rejected_origin,
                fingerprint.clone()
            )
            .await
            .is_err()
        );
    }
    let (approval, created) = repo
        .recover_hermes_approval(
            request.clone(),
            effective.clone(),
            origin.clone(),
            fingerprint.clone(),
        )
        .await
        .unwrap();
    assert!(created);
    assert!(
        !repo
            .recover_hermes_approval(
                request.clone(),
                effective.clone(),
                origin.clone(),
                fingerprint.clone()
            )
            .await
            .unwrap()
            .1
    );
    assert_eq!(
        repo.list_session_approvals(session.id).await.unwrap().len(),
        1
    );
    assert_eq!(
        repo.get_session_agent_run(run.id).await.unwrap().state,
        SessionRunState::Waiting
    );
    repo.observe_runtime_launch(&launch, "gateway_exited", Some(12345))
        .await
        .unwrap();
    assert!(
        repo.recover_hermes_approval(request, effective, origin, fingerprint)
            .await
            .is_err()
    );
    assert_eq!(
        repo.list_session_approvals(session.id).await.unwrap()[0].id,
        approval.id
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn container_lifecycle_binding_rejects_cross_agent_mounts_identity_and_sources() {
    let Some((_, agent, _, _, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await else {
        return;
    };
    let original = binding(&agent, Uuid::new_v4());
    crate::runtime_launches::validate_container_binding(&original).unwrap();
    for field in [
        "resource",
        "generation",
        "project",
        "mount",
        "source",
        "journal",
        "hash",
        "kind",
    ] {
        let mut changed = original.clone();
        let container = changed.container.as_mut().unwrap();
        match field {
            "resource" => container.registration.resource_id = Uuid::new_v4(),
            "generation" => container.registration.generation = Uuid::new_v4(),
            "project" => container.policy["project"] = json!("unowned-project"),
            "mount" => container.policy["mounts"][1]["source"] = json!("/other-agent/config"),
            "source" => container.source_sha256[0] = "unpinned".into(),
            "journal" => container.stop_journal = container.journal.clone(),
            "kind" => changed.kind = AgentKind::JavaAgent,
            _ => changed.command_sha256 = "0".repeat(64),
        }
        if field != "hash" {
            changed.command_sha256 = crate::runtime_launches::snapshot_hash(
                &serde_json::to_value(changed.container.as_ref().unwrap()).unwrap(),
            )
            .unwrap();
        }
        assert!(
            crate::runtime_launches::validate_container_binding(&changed).is_err(),
            "{field}"
        );
    }
    let mut legacy = serde_json::to_value(&original).unwrap();
    legacy.as_object_mut().unwrap().remove("container");
    let decoded: RuntimeLaunchBinding = serde_json::from_value(legacy.clone()).unwrap();
    assert!(decoded.container.is_none());
    assert_eq!(serde_json::to_value(decoded).unwrap(), legacy);
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
async fn fake_control(
    config: &AppConfig,
    mut launch: RuntimeLaunchBinding,
    unknown: bool,
) -> (AppConfig, RuntimeLaunchBinding) {
    use std::os::unix::fs::PermissionsExt;
    let mut config = config.clone();
    for path in [
        &launch.paths.runtime,
        &launch.paths.config,
        &launch.paths.workspace,
        &launch.paths.logs,
    ] {
        tokio::fs::create_dir_all(path).await.unwrap();
    }
    let base = Path::new(&config.fleet.controller_root).join("base-source");
    tokio::fs::create_dir_all(base.join("scripts"))
        .await
        .unwrap();
    let program = r#"import sys,json,hashlib
from pathlib import Path
r=json.load(sys.stdin); reg=r['registration']; root=Path(r['journal']).parent; started=root/'start-effect'
digest=lambda value:hashlib.sha256(json.dumps(value,sort_keys=True,separators=(',',':')).encode()).hexdigest()
snap={'contract_version':2,'container_id':reg['container_id'],'engine':reg['engine'],'policy_sha256':reg['policy_sha256'],'inventory_sha256':reg['running_inventory_sha256'],'started_at':'2026-10-06T12:00:00.123456789Z','init_pid':12345,'network_sha256':reg['network_sha256']}
result={k:reg[k] for k in ('contract_version','operation_id','container_id','resource_id','generation')}
result.update(registration_sha256=digest(reg),state='registered',observation='never_started',snapshot=None)
if r['action']=='start':
 started.write_text(str(int(started.read_text())+1) if started.exists() else '1')
if started.exists():
 result.update(state='observed',observation='running',snapshot=snap)
 if (root/'unknown-mode').exists():result.update(state='held',observation='unavailable',snapshot=None)
if (root/'stop-effect').exists():result.update(state='observed',observation='namespace_exited',snapshot=snap)
if r['action']=='stop':
 (root/'stop-effect').write_text('1')
 result={k:reg[k] for k in ('contract_version','container_id','resource_id','generation')}
 result.update(operation_id=r['operation_id'],snapshot_sha256=digest(snap),state='observed',observation='namespace_exited')
print(json.dumps({'protocol_version':1,'action':r['action'],'result':result}));sys.exit(2 if result['state']=='held' else 0)
"#;
    let mut hashes = Vec::new();
    for (name, bytes) in [
        ("runtime_boundary.py", ""),
        ("runtime_bootstrap.py", ""),
        ("runtime_control.py", program),
    ] {
        tokio::fs::write(base.join("scripts").join(name), bytes)
            .await
            .unwrap();
        hashes.push(hex::encode(Sha256::digest(bytes.as_bytes())));
    }
    let container = launch.container.as_mut().unwrap();
    container.source_sha256 = hashes.try_into().unwrap();
    config.fleet.container_control = Some(shared::config::ContainerControlConfig {
        python: "python3".into(),
        base_root: base.to_string_lossy().into_owned(),
        source_sha256: container.source_sha256.clone(),
        context: container.context.clone(),
        provisioning: None,
        bridge_controller: None,
    });
    launch.command_sha256 =
        crate::runtime_launches::snapshot_hash(&serde_json::to_value(&container).unwrap()).unwrap();
    let mut probe = Command::new("python3").args(["-I", "-c",
        "import runpy,sys,types; from pathlib import Path; p=types.ModuleType('scripts'); p.__path__=[str(Path(sys.argv[1])/'scripts')]; sys.modules['scripts']=p; runpy.run_module('scripts.runtime_control',run_name='__main__')"])
        .arg(&base).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let mut input = probe.stdin.take().unwrap();
    use tokio::io::AsyncWriteExt;
    input.write_all(&serde_json::to_vec(&json!({"action":"observe","registration":container.registration,"journal":container.journal})).unwrap()).await.unwrap();
    drop(input);
    let output = probe.wait_with_output().await.unwrap();
    assert!(
        output.status.success(),
        "Fixture-only Python error: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let prepared = Path::new(&config.fleet.controller_root)
        .join(format!("{}.container-prepared.json", launch.agent_id));
    tokio::fs::write(
        &prepared,
        serde_json::to_vec(
            &json!({"agent_id":launch.agent_id,"paths":launch.paths,"api_port":launch.api_port,
        "configuration_revision":null,"configuration_sha256":null,"container":container}),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    tokio::fs::set_permissions(prepared, std::fs::Permissions::from_mode(0o600))
        .await
        .unwrap();
    if unknown {
        tokio::fs::write(
            Path::new(&config.fleet.controller_root).join("unknown-mode"),
            "1",
        )
        .await
        .unwrap();
    }
    (config, launch)
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn container_lifecycle_preparation_checks_marker_and_rejects_workspace_links() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let (config, _) = fake_control(&config, binding(&agent, Uuid::new_v4()), false).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let marker = Path::new(&agent.paths.config)
        .parent()
        .unwrap()
        .join(".fleet-agent.json");
    let original_marker = tokio::fs::read(&marker).await.unwrap();
    tokio::fs::write(
        &marker,
        serde_json::to_vec(&json!({"id":Uuid::new_v4(),"name":agent.name})).unwrap(),
    )
    .await
    .unwrap();
    assert!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    tokio::fs::write(marker, original_marker).await.unwrap();
    tokio::fs::remove_dir(&agent.paths.workspace).await.unwrap();
    std::os::unix::fs::symlink(&agent.paths.config, &agent.paths.workspace).unwrap();
    assert!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(!root.join("controller/start-effect").exists());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn container_lifecycle_unknown_start_is_durable_and_never_redispatched_or_native_spawned() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let (config, _) = fake_control(&config, binding(&agent, Uuid::new_v4()), true).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    assert!(matches!(
        runtime.start_locked(&agent, LaunchPhase::Regular).await,
        Err(AppError::Unavailable(_))
    ));
    let original = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert!(original.binding.container.is_some());
    assert_eq!(original.state, "claimed");
    let fresh = repo.get_agent(agent.id).await.unwrap();
    assert_eq!(fresh.status, AgentStatus::Starting);
    assert_eq!(fresh.runtime.desired_state, DesiredState::Running);
    assert!(
        runtime
            .start_locked(&fresh, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert!(runtime.stop_locked(&fresh).await.is_err());
    assert!(runtime.gateway_launch_generation(agent.id).await.is_err());
    assert!(runtime.children.lock().await.is_empty());
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/start-effect"))
            .await
            .unwrap(),
        "1"
    );
    assert!(!root.join("controller/stop-effect").exists());
    let after = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(after.binding).unwrap(),
        serde_json::to_value(original.binding).unwrap()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn container_lifecycle_original_ack_authorizes_generation_and_namespace_stop_only() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let (config, mut launch) = fake_control(&config, binding(&agent, Uuid::new_v4()), false).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config.clone()), repo.clone());
    launch.controller_id = runtime.controller_id;
    repo.claim_runtime_launch(&launch).await.unwrap();
    runtime.launches.lock().await.insert(
        agent.id,
        RuntimeLaunchRecord {
            binding: launch.clone(),
            state: "claimed".into(),
            pid: None,
            controller_recovery: false,
        },
    );
    let container = launch.container.as_ref().unwrap();
    let files = container_control::ContainerLaunchFiles {
        policy: container.policy.clone(),
        compose: container.compose.clone().into(),
        journal: container.journal.clone().into(),
        stop_journal: container.stop_journal.clone().into(),
        mount_mapping: None,
        mapping_file: None,
    };
    let receipt = runtime
        .container_control(container)
        .unwrap()
        .start(&files, &container.registration)
        .await
        .unwrap();
    let pid = receipt.snapshot.unwrap().init_pid as i32;
    repo.observe_runtime_launch(&launch, "gateway_started", Some(pid))
        .await
        .unwrap();
    assert_eq!(
        runtime.gateway_launch_generation(agent.id).await.unwrap(),
        Some(launch.id)
    );
    let foreign = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    assert!(foreign.gateway_launch_generation(agent.id).await.is_err());
    assert!(foreign.stop_locked(&agent).await.is_err());
    assert!(!root.join("controller/stop-effect").exists());
    assert_eq!(
        runtime.stop_locked(&agent).await.unwrap().status,
        AgentStatus::Stopped
    );
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(runtime.children.lock().await.is_empty());
    assert!(runtime.gateway_launch_generation(agent.id).await.is_err());
    assert!(
        runtime
            .start_locked(
                &repo.get_agent(agent.id).await.unwrap(),
                LaunchPhase::Regular
            )
            .await
            .is_err()
    );
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/start-effect"))
            .await
            .unwrap(),
        "1"
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
pub(super) async fn fake_creation(config: &AppConfig, agent: &Agent, unknown: bool) -> AppConfig {
    let (mut config, _) = fake_control(config, binding(agent, Uuid::new_v4()), false).await;
    let root = Path::new(&config.fleet.controller_root);
    tokio::fs::remove_file(root.join(format!("{}.container-prepared.json", agent.id)))
        .await
        .unwrap();
    let program = r#"import sys,json,hashlib,time
from pathlib import Path
r=json.load(sys.stdin);root=Path(r['journal']).parent
from datetime import datetime,timezone
digest=lambda v:hashlib.sha256(json.dumps(v,sort_keys=True,separators=(',',':')).encode()).hexdigest()
if r['protocol_version']==2:
 assert r['policy']['contract_version']==3
 mapping=r['mount_mapping'];file=Path(r['mapping_file'])
 assert not (root/'mapping-drift').exists()
 raw=json.dumps(mapping,sort_keys=True,separators=(',',':'))
 if r['action']=='prepare' and not file.exists():file.write_text(raw);file.chmod(0o600)
 assert file.read_text()==raw
 if (root/'controller-restart').exists() and r['action']!='observe_controller_restart':
  raise RuntimeError('Original mapped controller epoch changed')
if r['action']=='resolve_mounts':
 mounts=[dict(m,source='/daemon/volumes/own/_data/'+m['source'][len(r['local_root'])+1:]) for m in r['policy']['mounts']]
 result={'state':'resolved','controller':r['controller'],'snapshot':{'container_id':r['controller']['container_id'],'started_at':'2026-10-06T12:00:00Z','init_pid':999,'inventory_sha256':'a'*64},'engine':{'ID':'original-engine','KernelVersion':'original-kernel','ServerVersion':'29'},'local_root':r['local_root'],'volume_name':'qa_owned_agents','volume_sha256':'b'*64,'mounts':mounts,'input_policy_sha256':digest(r['policy'])}
 if (root/'mapping-drift').exists():result['snapshot']['init_pid']+=1
elif r['action']=='prepare':
 with (root/'prepare-calls').open('a') as calls:calls.write(r['operation_id']+'\n')
 intents=[json.loads(p.read_bytes()) for p in root.glob(r['policy']['resource_id']+'*.container-creation.json')]
 intent=next(v for v in intents if v['generation']==r['policy']['generation'])
 assert intent['generation']==r['policy']['generation'] and intent['operation_id']==r['operation_id']
 if 'environment_snapshot' in intent:
  snapshot=intent['environment_snapshot'];assert snapshot['version']==1
  envfile=Path(intent['paths']['config'])/'.env'
  assert snapshot['dotenv']==(envfile.read_text() if envfile.exists() else None)
  assert snapshot['dotenv_sha256']==(hashlib.sha256(envfile.read_bytes()).hexdigest() if envfile.exists() else None)
 assert r['process']['environment']['HERMES_HOME']=='/config' and r['process']['working_dir']=='/workspace'
 if not (root/'prepare-effect').exists():(root/'prepare-effect').write_text('1')
 if (root/'prepare-unknown').exists():
  result={'state':'held','operation_id':r['operation_id'],'resource_id':r['policy']['resource_id'],'generation':r['policy']['generation']}
 else:
  policy=r['policy'];policy['network']['id']='1'*64
  reg={'contract_version':policy['contract_version'],'operation_id':r['operation_id'],'container_id':digest(policy['generation']),'resource_id':policy['resource_id'],'generation':policy['generation'],'engine':{'ID':'original-engine','KernelVersion':'original-kernel','ServerVersion':'29'},'policy_sha256':digest(policy),'inventory_sha256':'c'*64,'running_inventory_sha256':'d'*64,'compose_sha256':'e'*64,'network_sha256':'f'*64}
  if r['protocol_version']==2:reg['mount_mapping_sha256']=digest(r['mount_mapping'])
  result={'state':'prepared','policy':policy,'registration':reg}
elif r['action'] in ('observe_controller_restart','recover_controller','read_controller_recovery','heartbeat_controller','heartbeat_controller_live') or (r['action']=='observe' and 'recovery' in r):
 if (root/'hold-recovery-handshake').exists():assert r['action'] not in ('observe_controller_restart','recover_controller','read_controller_recovery')
 if (root/'hold-reservation-witness').exists():assert r['action']!='observe_controller_restart'
 reg=r['registration'];mapping=r['mount_mapping']
 assert (root/'controller-restart').exists() and (root/(reg['generation']+'.started')).exists()
 assert (root/'known-start').exists() and not (root/(reg['generation']+'.unknown')).exists()
 receipt={k:reg[k] for k in ('contract_version','operation_id','container_id','resource_id','generation')}
 snap={'contract_version':reg['contract_version'],'container_id':reg['container_id'],'engine':reg['engine'],'policy_sha256':reg['policy_sha256'],'inventory_sha256':reg['running_inventory_sha256'],'started_at':'2026-10-06T12:00:00.123456789Z','init_pid':12345,'network_sha256':reg['network_sha256']}
 if (root/'restart-agent-drift').exists():snap['init_pid']+=1
 receipt.update(registration_sha256=digest(reg),state='observed',observation='running',snapshot=snap)
 current=dict(mapping['snapshot'],started_at='2026-10-07T12:00:00Z',init_pid=987)
 result={'contract_version':1,'state':'controller_restart_observed','original_mapping_sha256':digest(mapping),'registration_sha256':digest(reg),'original_controller_snapshot':mapping['snapshot'],'current_controller_snapshot':current,'receipt':receipt}
 if r['action']!='observe_controller_restart':
  assert r['protocol_version']==3 and Path(r['recovery_journal']).parent==root
  command=r['recovery'];assert command['request']['controller_snapshot']==current
  ack=root/(command['request']['id']+'.recovery-ack.json')
  lease=root/(command['request']['id']+'.recovery-lease.json')
  if r['action']=='recover_controller':
   with (root/'recovery-calls').open('a') as calls:calls.write(command['request']['id']+'\n')
   result={'contract_version':1,'state':'controller_recovered','request_sha256':digest(command),'recovery':command,'witness':result}
   assert not ack.exists();ack.write_text(json.dumps(result));ack.chmod(0o600)
   lease.write_text(json.dumps(command))
   if (root/'recovery-unknown').exists():raise RuntimeError('Original ACK reply lost')
  elif r['action']=='read_controller_recovery':
   result=json.loads(ack.read_bytes());assert result['recovery']==command
  elif r['action'] in ('heartbeat_controller','heartbeat_controller_live'):
   if (root/'slow-heartbeat').exists():time.sleep(5)
   original=json.loads(ack.read_bytes())['recovery'];previous=json.loads(lease.read_bytes())
   assert command['request']==original['request'] and command['epoch']==original['epoch']
   with (root/'heartbeat-calls').open('a') as calls:calls.write(str(command['lease_version'])+'\n')
   if command!=previous:
    assert command['lease_version']==previous['lease_version']+1
    assert not (root/'native-lease-expired').exists()
    assert datetime.fromisoformat(previous['lease_expires_at'])>datetime.now(timezone.utc)
    assert datetime.fromisoformat(command['lease_expires_at'])>datetime.fromisoformat(previous['lease_expires_at'])
    lease.write_text(json.dumps(command))
    if (root/'heartbeat-unknown').exists():
     (root/'heartbeat-unknown').unlink();raise RuntimeError('Heartbeat ACK reply lost')
   result={'state':'controller_heartbeat','recovery_id':command['request']['id'],'lease_version':command['lease_version'],'lease_expires_at':command['lease_expires_at']}
   if r['action']=='heartbeat_controller_live':
    assert not (root/'native-lease-expired').exists()
    assert datetime.fromisoformat(command['lease_expires_at'])>datetime.now(timezone.utc)
    result.update(state='controller_heartbeat_live',receipt=receipt)
   if (root/'heartbeat-malformed').exists():result['lease_version']+=1
  else:
   assert json.loads(lease.read_bytes())==command
   assert not (root/'native-lease-expired').exists()
   assert datetime.fromisoformat(command['lease_expires_at'])>datetime.now(timezone.utc)
   result=receipt
elif r['action']=='attach_controller':
 reg=r['registration']
 if (root/'attach-unknown').exists():
  result={'state':'held'}
 else:
  (root/'attach-effect').write_text('1')
  result={'state':'attached','registration_sha256':digest(reg),'controller_id':r['controller']['container_id'],'controller_sha256':'e'*64,'network_id':r['policy']['network']['id']}
else:
 reg=r['registration'];result={k:reg[k] for k in ('contract_version','operation_id','container_id','resource_id','generation')}
 result.update(registration_sha256=digest(reg),state='registered',observation='never_started',snapshot=None)
 started=root/(reg['generation']+'.started');stopped=root/(reg['generation']+'.stopped')
 if r['action']=='start':
  (root/'start-effect').write_text('1');started.write_text('1')
  with (root/'start-calls').open('a') as calls:calls.write(reg['generation']+'\n')
  if (root/'hold-next-start').exists():(root/(reg['generation']+'.unknown')).write_text('1')
 if started.exists():
  result.update(state='held',observation='unavailable')
  if ((r['action']!='start' and (root/'recover-start').exists()) or (root/'known-start').exists()) and not (root/(reg['generation']+'.unknown')).exists():
   snap={'contract_version':reg['contract_version'],'container_id':reg['container_id'],'engine':reg['engine'],'policy_sha256':reg['policy_sha256'],'inventory_sha256':reg['running_inventory_sha256'],'started_at':'2026-10-06T12:00:00.123456789Z','init_pid':12345,'network_sha256':reg['network_sha256']}
   result.update(state='observed',observation='namespace_exited' if stopped.exists() else 'running',snapshot=snap)
   if r['action']=='stop':
    stopped.write_text('1');result={k:reg[k] for k in ('contract_version','container_id','resource_id','generation')}
    result.update(operation_id=r['operation_id'],snapshot_sha256=digest(snap),state='observed',observation='namespace_exited')
 if r['action']=='endpoint' and result['state']=='observed' and result['observation']=='running':
  result={'host':(root/'endpoint-host').read_text(),'receipt':result}
print(json.dumps({'protocol_version':r['protocol_version'],'action':r['action'],'result':result}));sys.exit(2 if result.get('state')=='held' else 0)
"#;
    let control = config.fleet.container_control.as_mut().unwrap();
    let source = Path::new(&control.base_root).join("scripts/runtime_control.py");
    tokio::fs::write(source, program).await.unwrap();
    control.source_sha256[2] = hex::encode(Sha256::digest(program.as_bytes()));
    control.provisioning = Some(shared::config::ContainerProvisioningConfig {
        project: "sdlc-qa-container-create-aaaaaaaaaaaa".into(),
        image_id: format!("sha256:{}", "b".repeat(64)),
        user: "10001:10001".into(),
        entrypoint: vec!["/usr/bin/hermes".into()],
        pids_limit: 128,
        memory_bytes: 512 * 1024 * 1024,
        nano_cpus: 1_000_000_000,
        network_internal: true,
        task: "container-creation".into(),
        purpose: "consumer-test".into(),
    });
    if unknown {
        tokio::fs::write(root.join("prepare-unknown"), "1")
            .await
            .unwrap();
    }
    config
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn pending_container_freezes_dotenv_before_create_and_rejects_rotation_or_snapshot_removal() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = fake_creation(&config, &agent, true).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let envfile = Path::new(&agent.paths.config).join(".env");
    let original_env = "PROVIDER_TOKEN=\"generation-old-credential\"\n";
    tokio::fs::write(&envfile, original_env).await.unwrap();
    assert!(
        runtime
            .prepared_container(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    let private = root.join("controller");
    let intent_path = private.join(format!("{}.container-creation.json", agent.id));
    let original = tokio::fs::read(&intent_path).await.unwrap();
    let mut intent: Value = serde_json::from_slice(&original).unwrap();
    assert_eq!(intent["environment_snapshot"]["dotenv"], original_env);
    assert_eq!(
        intent["environment_snapshot"]["dotenv_sha256"],
        hex::encode(Sha256::digest(original_env.as_bytes()))
    );
    let calls = tokio::fs::read(private.join("prepare-calls"))
        .await
        .unwrap();
    tokio::fs::write(&envfile, "PROVIDER_TOKEN=\"rotated-credential\"\n")
        .await
        .unwrap();
    assert!(
        runtime
            .prepared_container(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert_eq!(tokio::fs::read(&intent_path).await.unwrap(), original);
    assert_eq!(
        tokio::fs::read(private.join("prepare-calls"))
            .await
            .unwrap(),
        calls
    );
    // Removing the optional field cannot turn a claimed new intent into a legacy one.
    intent
        .as_object_mut()
        .unwrap()
        .remove("environment_snapshot");
    tokio::fs::write(&intent_path, serde_json::to_vec(&intent).unwrap())
        .await
        .unwrap();
    assert!(
        runtime
            .prepared_container(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert_eq!(
        tokio::fs::read(private.join("prepare-calls"))
            .await
            .unwrap(),
        calls
    );
    tokio::fs::write(&intent_path, original).await.unwrap();
    tokio::fs::write(&envfile, original_env).await.unwrap();
    tokio::fs::remove_file(private.join("prepare-unknown"))
        .await
        .unwrap();
    let prepared = runtime
        .prepared_container(&agent, LaunchPhase::Regular)
        .await
        .unwrap();
    assert_eq!(json!(prepared.id), intent["generation"]);
    assert!(!private.join("start-effect").exists());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn prepared_container_rechecks_dotenv_before_start_and_preserves_private_original() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = fake_creation(&config, &agent, false).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo);
    let envfile = Path::new(&agent.paths.config).join(".env");
    let body = "TOKEN=\"generation-original\"\n";
    tokio::fs::write(&envfile, body).await.unwrap();
    let prepared = runtime
        .prepared_container(&agent, LaunchPhase::Regular)
        .await
        .unwrap();
    let private = root.join("controller");
    let path = private.join(format!("{}.container-creation.json", agent.id));
    let bytes = tokio::fs::read(&path).await.unwrap();
    let metadata = tokio::fs::metadata(&path).await.unwrap();
    use std::os::unix::fs::MetadataExt;
    assert_eq!(metadata.mode() & 0o777, 0o600);
    for current in [Some("TOKEN=rotated"), None] {
        if let Some(current) = current {
            tokio::fs::write(&envfile, current).await.unwrap();
        } else {
            tokio::fs::remove_file(&envfile).await.unwrap();
        }
        assert!(
            runtime
                .start_locked(&agent, LaunchPhase::Regular)
                .await
                .is_err()
        );
        assert!(!private.join("start-effect").exists());
        assert_eq!(tokio::fs::read(&path).await.unwrap(), bytes);
    }
    tokio::fs::write(&envfile, body).await.unwrap();
    assert_eq!(
        runtime
            .prepared_container(&agent, LaunchPhase::Regular)
            .await
            .unwrap()
            .id,
        prepared.id
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn oversized_private_environment_intent_is_rejected_before_database_claim_or_create() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = fake_creation(&config, &agent, false).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    // The input itself fits; JSON escaping would overflow the existing private document cap.
    tokio::fs::write(
        Path::new(&agent.paths.config).join(".env"),
        "\n".repeat(40 * 1024),
    )
    .await
    .unwrap();
    assert!(
        runtime
            .prepared_container(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert!(
        !repo
            .has_pending_container_preparation(agent.id)
            .await
            .unwrap()
    );
    let private = root.join("controller");
    assert!(!private.join("prepare-calls").exists());
    assert!(
        !private
            .join(format!("{}.container-creation.json", agent.id))
            .exists()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn invalid_agent_project_is_rejected_before_intent_prepare_or_launch() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let mut config = fake_creation(&config, &agent, false).await;
    config
        .fleet
        .container_control
        .as_mut()
        .unwrap()
        .provisioning
        .as_mut()
        .unwrap()
        .project = "sdlc-common".into();
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let result = runtime.start_locked(&agent, LaunchPhase::Regular).await;
    let private = root.join("controller");
    assert!(matches!(result, Err(AppError::Validation(_))));
    assert!(!private.join("prepare-effect").exists());
    assert!(!private.join("start-effect").exists());
    assert!(
        !private
            .join(format!("{}.container-creation.json", agent.id))
            .exists()
    );
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(repo.get_agent(agent.id).await.unwrap().status, agent.status);
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn java_cannot_fall_back_to_native_when_docker_mode_is_selected() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::JavaAgent).await
    else {
        return;
    };
    let mut config = (*config).clone();
    config.fleet.container_control = Some(shared::config::ContainerControlConfig {
        python: "must-not-execute".into(),
        base_root: root.join("unavailable-base").to_string_lossy().into_owned(),
        source_sha256: ["a".repeat(64), "b".repeat(64), "c".repeat(64)],
        context: "desktop-linux".into(),
        provisioning: None,
        bridge_controller: None,
    });
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let result = runtime.start_locked(&agent, LaunchPhase::Regular).await;
    assert!(matches!(result, Err(AppError::Unavailable(detail))
        if detail == "Java Agent Docker runtime is not implemented; native fallback is forbidden"));
    assert!(runtime.children.lock().await.is_empty());
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(repo.get_agent(agent.id).await.unwrap().runtime.pid, None);
    assert_eq!(repo.get_agent(agent.id).await.unwrap().status, agent.status);
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn container_lifecycle_automatic_creation_binds_before_start_and_holds_unknown_start() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = fake_creation(&config, &agent, false).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    assert!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    let launch = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(launch.state, "claimed");
    let private = root.join("controller");
    let intent: Value = serde_json::from_slice(
        &tokio::fs::read(private.join(format!("{}.container-creation.json", agent.id)))
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(intent["generation"], json!(launch.binding.id));
    assert_eq!(intent["process"]["environment"]["HOME"], "/config");
    assert!(
        private
            .join(format!("{}.container-prepared.json", agent.id))
            .exists()
    );
    assert!(
        runtime
            .start_locked(
                &repo.get_agent(agent.id).await.unwrap(),
                LaunchPhase::Regular
            )
            .await
            .is_err()
    );
    assert_eq!(
        tokio::fs::read_to_string(private.join("prepare-effect"))
            .await
            .unwrap(),
        "1"
    );
    assert_eq!(
        tokio::fs::read_to_string(private.join("start-effect"))
            .await
            .unwrap(),
        "1"
    );
    assert!(runtime.children.lock().await.is_empty());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn container_lifecycle_unknown_creation_reuses_exact_intent_and_rejects_changed_credentials()
{
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let mut config = fake_creation(&config, &agent, true).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config.clone()), repo.clone());
    assert!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    let path = root.join(format!("controller/{}.container-creation.json", agent.id));
    let original = tokio::fs::read(&path).await.unwrap();
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert_eq!(tokio::fs::read(&path).await.unwrap(), original);
    config.fleet.runtime_token_secret = "different-secret-must-not-replace-pending-intent".into();
    let changed = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    assert!(
        changed
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert_eq!(tokio::fs::read(&path).await.unwrap(), original);
    tokio::fs::remove_file(root.join("controller/prepare-unknown"))
        .await
        .unwrap();
    assert!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    let launch = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    let intent: Value = serde_json::from_slice(&original).unwrap();
    assert_eq!(intent["generation"], json!(launch.binding.id));
    assert!(runtime.children.lock().await.is_empty());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn prepared_container_rejects_changed_credentials_and_recipe_before_launch_claim() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = fake_creation(&config, &agent, false).await;
    let original = lifecycle_tests::supervisor(Arc::new(config.clone()), repo.clone());
    let binding = original
        .prepared_container(&agent, LaunchPhase::Regular)
        .await
        .unwrap();
    let private = root.join("controller");
    let intent_path = private.join(format!("{}.container-creation.json", agent.id));
    let prepared_path = private.join(format!("{}.container-prepared.json", agent.id));
    let intent = tokio::fs::read(&intent_path).await.unwrap();
    let prepared = tokio::fs::read(&prepared_path).await.unwrap();
    for field in [
        "token",
        "image",
        "entrypoint",
        "user",
        "memory",
        "network",
        "cors",
        "controller",
    ] {
        let mut changed = config.clone();
        let provision = changed
            .fleet
            .container_control
            .as_mut()
            .unwrap()
            .provisioning
            .as_mut()
            .unwrap();
        match field {
            "token" => changed.fleet.runtime_token_secret = "changed-after-prepare".into(),
            "image" => provision.image_id = format!("sha256:{}", "c".repeat(64)),
            "entrypoint" => provision.entrypoint = vec!["/different/hermes".into()],
            "user" => provision.user = "10002:10002".into(),
            "memory" => provision.memory_bytes += 1,
            "network" => provision.network_internal = false,
            "cors" => changed
                .server
                .cors_allowed_origins
                .push("https://changed.test".into()),
            "controller" => {
                changed
                    .fleet
                    .container_control
                    .as_mut()
                    .unwrap()
                    .bridge_controller = Some(shared::config::BridgeControllerConfig {
                    container_id: "a".repeat(64),
                    image_id: format!("sha256:{}", "b".repeat(64)),
                    service: "fleet-backend".into(),
                })
            }
            _ => unreachable!(),
        }
        let runtime = lifecycle_tests::supervisor(Arc::new(changed), repo.clone());
        assert!(
            matches!(
                runtime.start_locked(&agent, LaunchPhase::Regular).await,
                Err(AppError::Unavailable(_))
            ),
            "{field}"
        );
        assert!(!private.join("start-effect").exists(), "{field}");
        assert_eq!(tokio::fs::read(&intent_path).await.unwrap(), intent);
        assert_eq!(tokio::fs::read(&prepared_path).await.unwrap(), prepared);
        assert!(
            repo.get_open_runtime_launch(agent.id)
                .await
                .unwrap()
                .is_none(),
            "{field}"
        );
    }
    let same = original
        .prepared_container(&agent, LaunchPhase::Regular)
        .await
        .unwrap();
    assert_eq!(same.id, binding.id);
    assert_eq!(
        tokio::fs::read_to_string(private.join("prepare-effect"))
            .await
            .unwrap(),
        "1"
    );
    assert_eq!(tokio::fs::read(&intent_path).await.unwrap(), intent);
    assert_eq!(tokio::fs::read(&prepared_path).await.unwrap(), prepared);
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn trusted_bridge_attachment_precedes_launch_and_unknown_blocks_start() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let mut config = fake_creation(&config, &agent, false).await;
    config
        .fleet
        .container_control
        .as_mut()
        .unwrap()
        .bridge_controller = Some(shared::config::BridgeControllerConfig {
        container_id: "a".repeat(64),
        image_id: format!("sha256:{}", "b".repeat(64)),
        service: "fleet-backend".into(),
    });
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let private = root.join("controller");
    tokio::fs::write(private.join("attach-unknown"), "1")
        .await
        .unwrap();
    assert!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert!(!private.join("start-effect").exists());
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    tokio::fs::remove_file(private.join("attach-unknown"))
        .await
        .unwrap();
    assert!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert!(private.join("attach-effect").exists());
    assert!(private.join("start-effect").exists());
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_some()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
fn with_mapping_controller(mut config: AppConfig) -> AppConfig {
    config
        .fleet
        .container_control
        .as_mut()
        .unwrap()
        .bridge_controller = Some(shared::config::BridgeControllerConfig {
        container_id: "a".repeat(64),
        image_id: format!("sha256:{}", "b".repeat(64)),
        service: "fleet-backend".into(),
    });
    config
}

#[cfg(target_os = "linux")]
type RecoveryFixture = (
    Arc<crate::PostgresFleetRepository>,
    Agent,
    Uuid,
    RuntimeLaunchBinding,
    app::runtime_launch::ControllerRecoveryRequest,
    std::path::PathBuf,
);

#[cfg(target_os = "linux")]
fn recovery_command(
    record: &app::runtime_launch::ControllerRecoveryRecord,
) -> app::runtime_launch::ControllerRecoveryCommand {
    app::runtime_launch::ControllerRecoveryCommand {
        request: record.request.clone(),
        epoch: record.epoch,
        lease_version: record.lease_version,
        lease_expires_at: record.lease_expires_at.clone(),
    }
}

#[cfg(target_os = "linux")]
fn recovery_receipt(
    command: &app::runtime_launch::ControllerRecoveryCommand,
    launch: &RuntimeLaunchBinding,
) -> Value {
    let container = launch.container.as_ref().unwrap();
    let reg = &container.registration;
    let mapping = container.mount_mapping.as_ref().unwrap();
    json!({"contract_version":1,"state":"controller_recovered","request_sha256":container_control::canonical_hash(command).unwrap(),
        "recovery":command,"witness":{"contract_version":1,"state":"controller_restart_observed",
        "original_mapping_sha256":container_control::canonical_hash(mapping).unwrap(),
        "registration_sha256":container_control::canonical_hash(reg).unwrap(),
        "original_controller_snapshot":mapping.snapshot,"current_controller_snapshot":command.request.controller_snapshot,
        "receipt":{"contract_version":reg.contract_version,"operation_id":reg.operation_id,"container_id":reg.container_id,
        "resource_id":reg.resource_id,"generation":reg.generation,"registration_sha256":container_control::canonical_hash(reg).unwrap(),
        "state":"observed","observation":"running","snapshot":{"contract_version":reg.contract_version,"container_id":reg.container_id,
        "engine":reg.engine,"policy_sha256":reg.policy_sha256,"inventory_sha256":reg.running_inventory_sha256,
        "started_at":"2026-10-06T12:00:00.123456789Z","init_pid":command.request.agent_pid,"network_sha256":reg.network_sha256}}}})
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_delivery_retains_one_original_body_and_one_concurrent_dispatch_claim() {
    let Some((repo, _, _, launch, request, root)) = recovery_fixture().await else {
        return;
    };
    let record = repo.reserve_controller_recovery(&request).await.unwrap();
    let command = recovery_command(&record);
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..16 {
        let repo = repo.clone();
        let command = command.clone();
        tasks.spawn(async move {
            repo.retain_controller_recovery_command(&command)
                .await
                .unwrap()
        });
    }
    while let Some(result) = tasks.join_next().await {
        assert!(result.unwrap().command == command);
    }
    let mut conflict = command.clone();
    conflict.lease_expires_at = "2030-01-01T00:00:00+00:00".into();
    assert!(matches!(
        repo.retain_controller_recovery_command(&conflict).await,
        Err(AppError::Conflict(_))
    ));
    let mut claims = tokio::task::JoinSet::new();
    for _ in 0..16 {
        let repo = repo.clone();
        claims.spawn(async move {
            repo.claim_controller_recovery_dispatch(request.id, request.controller_id)
                .await
                .unwrap()
        });
    }
    let mut winners = 0;
    while let Some(result) = claims.join_next().await {
        winners += u32::from(result.unwrap());
    }
    assert_eq!(winners, 1);
    assert!(
        repo.claim_controller_recovery_dispatch(request.id, Uuid::new_v4())
            .await
            .is_err()
    );
    let retained = repo
        .read_controller_recovery_delivery(request.id)
        .await
        .unwrap()
        .unwrap();
    assert!(retained.dispatch_claimed && retained.native_receipt.is_none());
    assert!(retained.command == command);
    assert_eq!(
        serde_json::to_value(
            repo.get_open_runtime_launch(request.agent_id)
                .await
                .unwrap()
                .unwrap()
                .binding
        )
        .unwrap(),
        serde_json::to_value(launch).unwrap()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_delivery_rejects_unclaimed_or_drifted_receipt_and_commits_one_redacted_audit() {
    let Some((repo, _, _, launch, request, root)) = recovery_fixture().await else {
        return;
    };
    let record = repo.reserve_controller_recovery(&request).await.unwrap();
    let command = recovery_command(&record);
    repo.retain_controller_recovery_command(&command)
        .await
        .unwrap();
    let receipt = recovery_receipt(&command, &launch);
    assert!(
        repo.settle_controller_recovery_outcome(request.id, &receipt)
            .await
            .is_err()
    );
    assert!(
        repo.claim_controller_recovery_dispatch(request.id, request.controller_id)
            .await
            .unwrap()
    );
    for drift in [
        "command",
        "pid",
        "epoch",
        "extra",
        "missing",
        "original",
        "registration",
        "timestamp",
    ] {
        let mut changed = receipt.clone();
        match drift {
            "command" => changed["recovery"]["lease_version"] = json!(2),
            "pid" => changed["witness"]["receipt"]["snapshot"]["init_pid"] = json!(9),
            "epoch" => {
                changed["witness"]["current_controller_snapshot"]["started_at"] = json!("other")
            }
            "extra" => changed["secret"] = json!("never-store-native-secret"),
            "missing" => {
                changed.as_object_mut().unwrap().remove("witness");
            }
            "original" => changed["witness"]["original_mapping_sha256"] = json!("a".repeat(64)),
            "registration" => changed["witness"]["receipt"]["generation"] = json!(Uuid::new_v4()),
            _ => changed["recovery"]["lease_expires_at"] = json!("2000-01-01T00:00:00+00:00"),
        }
        assert!(
            repo.settle_controller_recovery_outcome(request.id, &changed)
                .await
                .is_err(),
            "{drift}"
        );
    }
    let result = repo
        .settle_controller_recovery_outcome(request.id, &receipt)
        .await
        .unwrap();
    let replay = repo
        .settle_controller_recovery_outcome(request.id, &receipt)
        .await
        .unwrap();
    assert_eq!(result.native_receipt_sha256, replay.native_receipt_sha256);
    assert_eq!(result.lease_version, record.lease_version);
    assert_eq!(result.lease_expires_at, record.lease_expires_at);
    let audit = repo
        .db
        .query_one(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT count(*) AS count, min(payload::text) AS payload FROM audit_log
         WHERE action='runtime.controller_recovery.outcome' AND entity_id=$1",
            [request.id.to_string().into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(audit.try_get::<i64>("", "count").unwrap(), 1);
    assert!(
        !audit
            .try_get::<String>("", "payload")
            .unwrap()
            .contains("never-store")
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_delivery_historical_ack_after_real_expiry_does_not_revive_owner() {
    let Some((repo, _, _, launch, request, root)) = recovery_fixture().await else {
        return;
    };
    let record = repo.reserve_controller_recovery(&request).await.unwrap();
    let command = recovery_command(&record);
    repo.retain_controller_recovery_command(&command)
        .await
        .unwrap();
    assert!(
        repo.claim_controller_recovery_dispatch(request.id, request.controller_id)
            .await
            .unwrap()
    );
    let receipt = recovery_receipt(&command, &launch);
    tokio::time::sleep(std::time::Duration::from_secs(31)).await;
    let result = repo
        .settle_controller_recovery_outcome(request.id, &receipt)
        .await
        .unwrap();
    assert_eq!(result.state, "acknowledged");
    assert!(!result.lease_valid);
    assert_eq!(result.lease_version, 1);
    assert_eq!(result.lease_expires_at, record.lease_expires_at);
    assert!(
        !repo
            .claim_controller_recovery_dispatch(request.id, request.controller_id)
            .await
            .unwrap()
    );
    assert!(
        repo.heartbeat_controller_recovery(request.id, request.controller_id, 1)
            .await
            .is_err()
    );
    let mut successor = request.clone();
    successor.id = Uuid::new_v4();
    successor.controller_id = Uuid::new_v4();
    successor.predecessor_id = Some(request.id);
    assert!(repo.reserve_controller_recovery(&successor).await.is_err());
    successor.controller_snapshot.started_at = "2026-10-07T13:00:00Z".into();
    assert_eq!(
        repo.reserve_controller_recovery(&successor)
            .await
            .unwrap()
            .epoch,
        2
    );
    let historical = repo
        .read_controller_recovery_delivery(request.id)
        .await
        .unwrap()
        .unwrap();
    assert!(historical.command == command);
    assert_eq!(historical.native_receipt, Some(receipt));
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_worker_recovers_original_native_ack_without_repeating_unknown_dispatch() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = with_mapping_controller(fake_creation(&config, &agent, false).await);
    let first = lifecycle_tests::supervisor(Arc::new(config.clone()), repo.clone());
    let launch = first
        .prepared_container(&agent, LaunchPhase::Regular)
        .await
        .unwrap();
    repo.claim_runtime_launch(&launch).await.unwrap();
    repo.observe_runtime_launch(&launch, "gateway_started", Some(12345))
        .await
        .unwrap();
    for name in [
        format!("{}.started", launch.id),
        "known-start".into(),
        "controller-restart".into(),
        "recovery-unknown".into(),
    ] {
        tokio::fs::write(root.join("controller").join(name), "1")
            .await
            .unwrap();
    }
    let second = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let result = second.recover_container_controller(agent.id).await.unwrap();
    assert_eq!(result.state, "acknowledged");
    assert_eq!(result.request.controller_id, second.controller_id);
    assert_ne!(result.request.controller_id, launch.controller_id);
    assert_eq!(result.epoch, 1);
    tokio::fs::write(root.join("controller/hold-reservation-witness"), "1")
        .await
        .unwrap();
    let replay = second.recover_container_controller(agent.id).await.unwrap();
    assert_eq!(result.native_receipt_sha256, replay.native_receipt_sha256);
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/recovery-calls"))
            .await
            .unwrap()
            .lines()
            .count(),
        1
    );
    let delivery = repo
        .read_controller_recovery_delivery(result.request.id)
        .await
        .unwrap()
        .unwrap();
    assert!(delivery.dispatch_claimed && delivery.native_receipt.is_some());
    assert_eq!(
        serde_json::to_value(
            repo.get_open_runtime_launch(agent.id)
                .await
                .unwrap()
                .unwrap()
                .binding
        )
        .unwrap(),
        serde_json::to_value(launch).unwrap()
    );
    assert!(second.gateway_launch_generation(agent.id).await.is_err());
    assert!(second.children.lock().await.is_empty());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_worker_holds_claimed_command_without_native_ack_and_never_redispatches() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = with_mapping_controller(fake_creation(&config, &agent, false).await);
    let first = lifecycle_tests::supervisor(Arc::new(config.clone()), repo.clone());
    let launch = first
        .prepared_container(&agent, LaunchPhase::Regular)
        .await
        .unwrap();
    repo.claim_runtime_launch(&launch).await.unwrap();
    repo.observe_runtime_launch(&launch, "gateway_started", Some(12345))
        .await
        .unwrap();
    for name in [
        format!("{}.started", launch.id),
        "known-start".into(),
        "controller-restart".into(),
    ] {
        tokio::fs::write(root.join("controller").join(name), "1")
            .await
            .unwrap();
    }
    let second = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let container = launch.container.as_ref().unwrap();
    let control = second.container_control(container).unwrap();
    let files = second.container_files(container).await.unwrap();
    let witness = control
        .observe_controller_restart(&files, &container.registration)
        .await
        .unwrap();
    let record = repo
        .reserve_controller_recovery(&app::runtime_launch::ControllerRecoveryRequest {
            id: Uuid::new_v4(),
            launch_id: launch.id,
            agent_id: agent.id,
            original_controller_id: launch.controller_id,
            controller_id: second.controller_id,
            predecessor_id: None,
            launch_sha256: container_control::canonical_hash(&launch).unwrap(),
            mapping_sha256: container_control::canonical_hash(
                container.mount_mapping.as_ref().unwrap(),
            )
            .unwrap(),
            registration_sha256: container_control::canonical_hash(&container.registration)
                .unwrap(),
            controller_snapshot: witness.current_controller_snapshot,
            agent_pid: 12345,
        })
        .await
        .unwrap();
    let command = recovery_command(&record);
    repo.retain_controller_recovery_command(&command)
        .await
        .unwrap();
    assert!(
        repo.claim_controller_recovery_dispatch(record.request.id, second.controller_id)
            .await
            .unwrap()
    );
    // Crash after the durable claim but before the native call is indistinguishable from lost acceptance.
    for _ in 0..2 {
        assert!(second.recover_container_controller(agent.id).await.is_err());
    }
    assert!(!root.join("controller/recovery-calls").exists());
    let delivery = repo
        .read_controller_recovery_delivery(record.request.id)
        .await
        .unwrap()
        .unwrap();
    assert!(delivery.dispatch_claimed && delivery.native_receipt.is_none());
    assert_eq!(
        repo.read_controller_recovery(record.request.id)
            .await
            .unwrap()
            .unwrap()
            .state,
        "reserved"
    );
    assert!(second.children.lock().await.is_empty());
    assert!(second.gateway_launch_generation(agent.id).await.is_err());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
async fn recovery_fixture() -> Option<RecoveryFixture> {
    let (repo, agent, owner, config, root) = lifecycle_tests::fixture(AgentKind::Hermes).await?;
    let config = with_mapping_controller(fake_creation(&config, &agent, false).await);
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let launch = runtime
        .prepared_container(&agent, LaunchPhase::Regular)
        .await
        .unwrap();
    repo.claim_runtime_launch(&launch).await.unwrap();
    repo.observe_runtime_launch(&launch, "gateway_started", Some(12345))
        .await
        .unwrap();
    let container = launch.container.as_ref().unwrap();
    let mapping = container.mount_mapping.as_ref().unwrap();
    let mut snapshot = mapping.snapshot.clone();
    snapshot.started_at = "2026-10-07T12:00:00Z".into();
    let request = app::runtime_launch::ControllerRecoveryRequest {
        id: Uuid::new_v4(),
        launch_id: launch.id,
        agent_id: agent.id,
        original_controller_id: launch.controller_id,
        controller_id: Uuid::new_v4(),
        predecessor_id: None,
        launch_sha256: crate::runtime_launches::snapshot_hash(
            &serde_json::to_value(&launch).unwrap(),
        )
        .unwrap(),
        mapping_sha256: container_control::canonical_hash(mapping).unwrap(),
        registration_sha256: container_control::canonical_hash(&container.registration).unwrap(),
        controller_snapshot: snapshot,
        agent_pid: 12345,
    };
    Some((repo, agent, owner, launch, request, root))
}

#[cfg(target_os = "linux")]
type HeartbeatFixture = (
    Arc<crate::PostgresFleetRepository>,
    Agent,
    LocalRuntimeSupervisor,
    app::runtime_launch::ControllerRecoveryRecord,
    std::path::PathBuf,
);

#[cfg(target_os = "linux")]
async fn controller_heartbeat_fixture() -> Option<HeartbeatFixture> {
    let (repo, agent, _, config, root) = lifecycle_tests::fixture(AgentKind::Hermes).await?;
    let mut config = with_mapping_controller(fake_creation(&config, &agent, false).await);
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
    for name in [
        format!("{}.started", launch.id),
        "known-start".into(),
        "controller-restart".into(),
    ] {
        tokio::fs::write(root.join("controller").join(name), "1")
            .await
            .unwrap();
    }
    let second = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    let record = second.recover_container_controller(agent.id).await.unwrap();
    Some((repo, agent, second, record, root))
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_heartbeat_catches_up_lost_native_ack_without_skipping_a_version() {
    let Some((repo, agent, runtime, original, root)) = controller_heartbeat_fixture().await else {
        return;
    };
    let before = repo
        .read_controller_recovery_delivery(original.request.id)
        .await
        .unwrap()
        .unwrap();
    tokio::fs::write(root.join("controller/heartbeat-unknown"), "1")
        .await
        .unwrap();
    assert!(
        runtime
            .heartbeat_container_controller(agent.id)
            .await
            .is_err()
    );
    let pending = repo
        .read_controller_recovery(original.request.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending.lease_version, 2);
    let recovered = runtime
        .heartbeat_container_controller(agent.id)
        .await
        .unwrap();
    assert_eq!(recovered.lease_version, 3);
    assert!(recovered.request == original.request);
    let after = repo
        .read_controller_recovery_delivery(original.request.id)
        .await
        .unwrap()
        .unwrap();
    assert!(after.command == before.command && after.native_receipt == before.native_receipt);
    assert_eq!(after.native_receipt_sha256, before.native_receipt_sha256);
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/heartbeat-calls"))
            .await
            .unwrap(),
        "1\n2\n2\n3\n"
    );
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/recovery-calls"))
            .await
            .unwrap()
            .lines()
            .count(),
        1
    );
    assert!(runtime.gateway_launch_generation(agent.id).await.is_err());
    assert!(runtime.children.lock().await.is_empty());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_reconciliation_renews_acknowledged_owner_without_replaying_handshake() {
    let Some((repo, agent, runtime, original, root)) = controller_heartbeat_fixture().await else {
        return;
    };
    let delivery = repo
        .read_controller_recovery_delivery(original.request.id)
        .await
        .unwrap()
        .unwrap();
    tokio::fs::write(root.join("controller/hold-recovery-handshake"), "1")
        .await
        .unwrap();
    runtime
        .reconcile_controller_recovery(agent.id)
        .await
        .unwrap();
    let renewed = repo
        .read_controller_recovery(original.request.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(renewed.lease_version, 2);
    assert!(renewed.lease_valid && renewed.request == original.request);
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/heartbeat-calls"))
            .await
            .unwrap(),
        "1\n2\n"
    );
    let retained = repo
        .read_controller_recovery_delivery(original.request.id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        retained.command == delivery.command && retained.native_receipt == delivery.native_receipt
    );
    tokio::fs::write(root.join("controller/restart-agent-drift"), "1")
        .await
        .unwrap();
    assert!(
        runtime
            .reconcile_controller_recovery(agent.id)
            .await
            .is_err()
    );
    assert_eq!(
        repo.read_controller_recovery(original.request.id)
            .await
            .unwrap()
            .unwrap()
            .lease_version,
        2
    );
    assert!(runtime.gateway_launch_generation(agent.id).await.is_err());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_startup_heartbeat_has_its_full_budget_after_lifecycle_lock_wait() {
    let Some((repo, agent, runtime, original, root)) = controller_heartbeat_fixture().await else {
        return;
    };
    tokio::fs::write(root.join("controller/slow-heartbeat"), "1")
        .await
        .unwrap();
    let lock = runtime.lifecycle_lock(agent.id).await;
    let guard = lock.lock().await;
    runtime.spawn_controller_recovery();
    tokio::time::sleep(Duration::from_secs(11)).await;
    drop(guard);
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let record = repo
                .read_controller_recovery(original.request.id)
                .await
                .unwrap()
                .unwrap();
            let lease: serde_json::Value = serde_json::from_slice(
                &tokio::fs::read(root.join(format!(
                    "controller/{}.recovery-lease.json",
                    original.request.id
                )))
                .await
                .unwrap(),
            )
            .unwrap();
            if record.lease_version == 2
                && record.lease_valid
                && lease["lease_version"] == 2
                && lease["lease_expires_at"] == record.lease_expires_at
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("lock contention consumed the native heartbeat budget");
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/heartbeat-calls"))
            .await
            .unwrap(),
        "1\n2\n"
    );
    assert!(runtime.gateway_launch_generation(agent.id).await.is_err());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_worker_quiescence_finishes_native_write_and_stays_stopped() {
    let Some((repo, agent, runtime, original, root)) = controller_heartbeat_fixture().await else {
        return;
    };
    tokio::fs::write(root.join("controller/slow-heartbeat"), "1")
        .await
        .unwrap();
    runtime.spawn_controller_recovery();
    runtime.spawn_controller_recovery();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            if repo
                .read_controller_recovery(original.request.id)
                .await
                .unwrap()
                .unwrap()
                .lease_version
                == 2
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .unwrap();
    let other = runtime.clone();
    let (first, second) = tokio::join!(
        runtime.quiesce_controller_recovery(),
        other.quiesce_controller_recovery(),
    );
    first.unwrap();
    second.unwrap();
    let lease: serde_json::Value = serde_json::from_slice(
        &tokio::fs::read(root.join(format!(
            "controller/{}.recovery-lease.json",
            original.request.id
        )))
        .await
        .unwrap(),
    )
    .unwrap();
    let record = repo
        .read_controller_recovery(original.request.id)
        .await
        .unwrap()
        .unwrap();
    assert!(record.lease_valid && record.lease_version == 2);
    assert_eq!(lease["lease_version"], 2);
    assert_eq!(lease["lease_expires_at"], record.lease_expires_at);
    runtime.spawn_controller_recovery();
    tokio::time::sleep(Duration::from_secs(11)).await;
    assert_eq!(
        repo.read_controller_recovery(original.request.id)
            .await
            .unwrap()
            .unwrap()
            .lease_version,
        2
    );
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/heartbeat-calls"))
            .await
            .unwrap(),
        "1\n2\n"
    );
    assert!(runtime.gateway_launch_generation(agent.id).await.is_err());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_heartbeat_recovers_db_commit_before_native_delivery_and_holds_bad_ack() {
    let Some((repo, agent, runtime, original, root)) = controller_heartbeat_fixture().await else {
        return;
    };
    repo.heartbeat_controller_recovery(original.request.id, runtime.controller_id, 1)
        .await
        .unwrap();
    // The process died after the DB extension, before native version2 was delivered.
    let recovered = runtime
        .heartbeat_container_controller(agent.id)
        .await
        .unwrap();
    assert_eq!(recovered.lease_version, 3);
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/heartbeat-calls"))
            .await
            .unwrap(),
        "2\n3\n"
    );
    tokio::fs::write(root.join("controller/heartbeat-malformed"), "1")
        .await
        .unwrap();
    assert!(
        runtime
            .heartbeat_container_controller(agent.id)
            .await
            .is_err()
    );
    assert_eq!(
        repo.read_controller_recovery(original.request.id)
            .await
            .unwrap()
            .unwrap()
            .lease_version,
        3
    );
    let foreign = lifecycle_tests::supervisor(runtime.config.clone(), repo.clone());
    assert!(
        foreign
            .heartbeat_container_controller(agent.id)
            .await
            .is_err()
    );
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/heartbeat-calls"))
            .await
            .unwrap(),
        "2\n3\n3\n"
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_heartbeat_never_renews_from_historical_native_ack_or_expired_db_lease() {
    let Some((repo, agent, runtime, original, root)) = controller_heartbeat_fixture().await else {
        return;
    };
    let mut disabled = runtime.clone();
    let mut config = (*runtime.config).clone();
    config.fleet.controller_recovery_enabled = false;
    disabled.config = Arc::new(config);
    disabled
        .reconcile_controller_recovery(agent.id)
        .await
        .unwrap();
    assert!(!root.join("controller/heartbeat-calls").exists());
    tokio::fs::write(root.join("controller/native-lease-expired"), "1")
        .await
        .unwrap();
    assert!(
        runtime
            .heartbeat_container_controller(agent.id)
            .await
            .is_err()
    );
    assert_eq!(
        repo.read_controller_recovery(original.request.id)
            .await
            .unwrap()
            .unwrap()
            .lease_version,
        1
    );
    let calls = tokio::fs::read_to_string(root.join("controller/heartbeat-calls"))
        .await
        .unwrap();
    assert_eq!(calls, "1\n");
    // Production DB triggers and the real clock remain active.
    tokio::time::sleep(Duration::from_secs(31)).await;
    assert!(
        runtime
            .heartbeat_container_controller(agent.id)
            .await
            .is_err()
    );
    assert_eq!(
        tokio::fs::read_to_string(root.join("controller/heartbeat-calls"))
            .await
            .unwrap(),
        calls
    );
    assert!(runtime.gateway_launch_generation(agent.id).await.is_err());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_recovery_concurrent_replays_preserve_one_epoch_and_original_launch() {
    let Some((repo, agent, _, launch, request, root)) = recovery_fixture().await else {
        return;
    };
    let before = serde_json::to_value(repo.get_agent(agent.id).await.unwrap()).unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..16 {
        let repo = repo.clone();
        let request = request.clone();
        tasks.spawn(async move { repo.reserve_controller_recovery(&request).await.unwrap() });
    }
    let mut expiry = None;
    while let Some(result) = tasks.join_next().await {
        let result = result.unwrap();
        assert_eq!(result.request.id, request.id);
        assert_eq!(result.epoch, 1);
        assert_eq!(result.state, "reserved");
        assert_eq!(result.lease_version, 1);
        assert!(result.native_receipt_sha256.is_none());
        assert_eq!(
            expiry.get_or_insert(result.lease_expires_at.clone()),
            &result.lease_expires_at
        );
    }
    let persisted = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert!(persisted.controller_recovery);
    assert_eq!(
        serde_json::to_value(persisted.binding).unwrap(),
        serde_json::to_value(launch).unwrap()
    );
    assert_eq!(persisted.pid, Some(12345));
    assert_eq!(
        serde_json::to_value(repo.get_agent(agent.id).await.unwrap()).unwrap(),
        before
    );
    let mut changed = request.clone();
    changed.controller_id = Uuid::new_v4();
    assert!(matches!(
        repo.reserve_controller_recovery(&changed).await,
        Err(AppError::Conflict(_))
    ));
    changed.id = Uuid::new_v4();
    assert!(repo.reserve_controller_recovery(&changed).await.is_err());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_recovery_rejects_changed_original_identity_before_reservation() {
    let Some((repo, _, _, _, request, root)) = recovery_fixture().await else {
        return;
    };
    for field in [
        "pid",
        "launch",
        "mapping",
        "registration",
        "inventory",
        "container",
        "start",
        "owner",
        "predecessor",
    ] {
        let mut changed = request.clone();
        match field {
            "pid" => changed.agent_pid += 1,
            "launch" => changed.launch_sha256 = "f".repeat(64),
            "mapping" => changed.mapping_sha256 = "f".repeat(64),
            "registration" => changed.registration_sha256 = "f".repeat(64),
            "inventory" => changed.controller_snapshot.inventory_sha256 = "f".repeat(64),
            "container" => changed.controller_snapshot.container_id = "f".repeat(64),
            "start" => changed.controller_snapshot.started_at = "2026-10-06T12:00:00Z".into(),
            "owner" => changed.original_controller_id = Uuid::new_v4(),
            "predecessor" => changed.predecessor_id = Some(Uuid::new_v4()),
            _ => unreachable!(),
        }
        assert!(
            repo.reserve_controller_recovery(&changed).await.is_err(),
            "{field}"
        );
        assert!(
            repo.read_controller_recovery(request.id)
                .await
                .unwrap()
                .is_none()
        );
    }
    repo.reserve_controller_recovery(&request).await.unwrap();
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_recovery_heartbeat_and_ack_require_exact_owner_and_lease_version() {
    let Some((repo, _, _, _, request, root)) = recovery_fixture().await else {
        return;
    };
    let original = repo.reserve_controller_recovery(&request).await.unwrap();
    assert!(
        repo.heartbeat_controller_recovery(request.id, Uuid::new_v4(), 1)
            .await
            .is_err()
    );
    let heartbeat = repo
        .heartbeat_controller_recovery(request.id, request.controller_id, 1)
        .await
        .unwrap();
    assert_eq!(heartbeat.lease_version, 2);
    assert!(heartbeat.lease_expires_at > original.lease_expires_at);
    assert!(
        repo.heartbeat_controller_recovery(request.id, request.controller_id, 1)
            .await
            .is_err()
    );
    assert!(
        repo.acknowledge_controller_recovery(request.id, request.controller_id, 1, &"a".repeat(64))
            .await
            .is_err()
    );
    assert!(
        repo.acknowledge_controller_recovery(request.id, Uuid::new_v4(), 2, &"a".repeat(64))
            .await
            .is_err()
    );
    let acknowledged = repo
        .acknowledge_controller_recovery(request.id, request.controller_id, 2, &"a".repeat(64))
        .await
        .unwrap();
    assert_eq!(acknowledged.state, "acknowledged");
    let replay = repo
        .acknowledge_controller_recovery(request.id, request.controller_id, 2, &"a".repeat(64))
        .await
        .unwrap();
    assert_eq!(replay.lease_expires_at, acknowledged.lease_expires_at);
    assert_eq!(replay.lease_version, acknowledged.lease_version);
    assert!(
        repo.acknowledge_controller_recovery(request.id, request.controller_id, 2, &"b".repeat(64))
            .await
            .is_err()
    );
    let other = crate::PostgresFleetRepository::new(
        crate::connect_database(shared::DatabaseConfig {
            url: std::env::var("FLEET_TEST_DATABASE_URL").unwrap(),
            max_connections: 2,
            min_connections: 1,
            connect_timeout_seconds: 10,
            idle_timeout_seconds: 60,
        })
        .await
        .unwrap(),
    );
    let readback = other
        .read_controller_recovery(request.id)
        .await
        .unwrap()
        .unwrap();
    assert!(readback.request == request);
    assert_eq!(
        readback.native_receipt_sha256,
        acknowledged.native_receipt_sha256
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_recovery_reservation_fences_original_outbox_endpoint_and_observations() {
    let Some((repo, agent, owner, launch, request, root)) = recovery_fixture().await else {
        return;
    };
    repo.update_agent_status(agent.id, AgentStatus::Running)
        .await
        .unwrap();
    let session = repo
        .create_session(
            domain::CreateSessionRequest {
                primary_agent_id: Some(agent.id),
                agent_id: None,
                title: "Recovery fence".into(),
                task_key: None,
                leader_agent_id: None,
                parent_session_id: None,
                namespace_id: None,
                idempotency_key: Some(Uuid::new_v4().to_string()),
            },
            owner,
        )
        .await
        .unwrap();
    let message = repo
        .create_session_message(
            session.id,
            domain::CreateSessionMessageRequest {
                body: "do not dispatch through an unresolved owner".into(),
                author_agent_id: None,
                message_kind: Some(MessageKind::UserPrompt),
                runtime_message_id: None,
                idempotency_key: Some(Uuid::new_v4().to_string()),
            },
            owner,
        )
        .await
        .unwrap();
    repo.reserve_controller_recovery(&request).await.unwrap();
    assert!(
        repo.claim_controller_message_dispatch(launch.controller_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.claim_controller_message_dispatch(request.controller_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(repo.claim_message_dispatch().await.unwrap().is_none());
    assert_eq!(
        repo.db
            .query_one(sea_orm::Statement::from_sql_and_values(
                sea_orm::DatabaseBackend::Postgres,
                "SELECT state FROM message_dispatch_outbox WHERE message_id=$1",
                [message.id.into()]
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<String>("", "state")
            .unwrap(),
        "pending"
    );
    assert!(
        repo.record_container_endpoint(
            &launch,
            12345,
            &format!("http://172.18.0.2:{}", agent.api_port.unwrap())
        )
        .await
        .is_err()
    );
    assert!(
        repo.observe_runtime_launch(&launch, "gateway_exited", Some(12345))
            .await
            .is_err()
    );
    repo.acknowledge_controller_recovery(request.id, request.controller_id, 1, &"a".repeat(64))
        .await
        .unwrap();
    // A DB acknowledgement alone is not a Base permit or a new generation.
    assert!(
        repo.claim_controller_message_dispatch(request.controller_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        repo.observe_runtime_launch(&launch, "gateway_exited", Some(12345))
            .await
            .is_err()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_recovery_expiry_never_retries_unknown_acceptance_and_fences_successor() {
    let Some((repo, _, _, _, request, root)) = recovery_fixture().await else {
        return;
    };
    let Some((other, _, _, _, acknowledged_request, other_root)) = recovery_fixture().await else {
        return;
    };
    repo.reserve_controller_recovery(&request).await.unwrap();
    other
        .reserve_controller_recovery(&acknowledged_request)
        .await
        .unwrap();
    other
        .acknowledge_controller_recovery(
            acknowledged_request.id,
            acknowledged_request.controller_id,
            1,
            &"a".repeat(64),
        )
        .await
        .unwrap();
    // Use the real PostgreSQL clock; production triggers are not disabled for testing.
    tokio::time::sleep(Duration::from_secs(31)).await;
    let expired = repo.reserve_controller_recovery(&request).await.unwrap();
    assert!(!expired.lease_valid);
    assert_eq!(expired.lease_version, 1);
    assert!(
        repo.heartbeat_controller_recovery(request.id, request.controller_id, 1)
            .await
            .is_err()
    );
    assert!(
        repo.acknowledge_controller_recovery(request.id, request.controller_id, 1, &"a".repeat(64))
            .await
            .is_err()
    );
    let mut successor = request.clone();
    successor.id = Uuid::new_v4();
    successor.controller_id = Uuid::new_v4();
    successor.predecessor_id = Some(request.id);
    successor.controller_snapshot.started_at = "2026-10-08T12:00:00Z".into();
    assert!(repo.reserve_controller_recovery(&successor).await.is_err());
    let mut next = acknowledged_request.clone();
    next.id = Uuid::new_v4();
    next.controller_id = Uuid::new_v4();
    next.predecessor_id = Some(acknowledged_request.id);
    // Expired DB lease without a distinct physical controller start is not cessation.
    assert!(other.reserve_controller_recovery(&next).await.is_err());
    next.controller_snapshot.started_at = "2026-10-08T12:00:00Z".into();
    let result = other.reserve_controller_recovery(&next).await.unwrap();
    assert_eq!(result.epoch, 2);
    assert_eq!(
        other
            .read_controller_recovery(acknowledged_request.id)
            .await
            .unwrap()
            .unwrap()
            .state,
        "superseded"
    );
    assert!(
        other
            .heartbeat_controller_recovery(
                acknowledged_request.id,
                acknowledged_request.controller_id,
                1
            )
            .await
            .is_err()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
    tokio::fs::remove_dir_all(other_root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_recovery_sql_history_cannot_be_erased_relabelled_or_unbounded() {
    let Some((repo, _, _, _, request, root)) = recovery_fixture().await else {
        return;
    };
    repo.reserve_controller_recovery(&request).await.unwrap();
    for change in [
        "controller_id='00000000-0000-4000-8000-000000000099'",
        "epoch=2",
        "request='{}'::jsonb",
        "request_sha256=repeat('b',64)",
        "state='superseded'",
        "lease_expires_at=clock_timestamp()+interval '1 hour',lease_version=lease_version+1",
        "state='acknowledged',acknowledged_at=clock_timestamp()",
    ] {
        assert!(
            repo.db
                .execute_unprepared(&format!(
                    "UPDATE runtime_controller_recoveries SET {change} WHERE id='{}'",
                    request.id
                ))
                .await
                .is_err(),
            "{change}"
        );
    }
    for operation in [
        "DELETE FROM runtime_controller_recoveries",
        "TRUNCATE runtime_controller_recoveries",
    ] {
        assert!(repo.db.execute_unprepared(operation).await.is_err());
    }
    let original = repo
        .read_controller_recovery(request.id)
        .await
        .unwrap()
        .unwrap();
    assert!(original.request == request);
    assert_eq!(original.state, "reserved");
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn controller_restart_health_observes_original_ack_without_granting_new_custody() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = with_mapping_controller(fake_creation(&config, &agent, false).await);
    let runtime = lifecycle_tests::supervisor(Arc::new(config.clone()), repo.clone());
    let launch = runtime
        .prepared_container(&agent, LaunchPhase::Regular)
        .await
        .unwrap();
    repo.claim_runtime_launch(&launch).await.unwrap();
    let container = launch.container.as_ref().unwrap();
    let private = root.join("controller");
    tokio::fs::write(private.join("known-start"), "1")
        .await
        .unwrap();
    let launch_files = container_control::ContainerLaunchFiles {
        policy: container.policy.clone(),
        compose: container.compose.clone().into(),
        journal: container.journal.clone().into(),
        stop_journal: container.stop_journal.clone().into(),
        mount_mapping: container.mount_mapping.clone(),
        mapping_file: container
            .mapping_file
            .as_ref()
            .map(std::path::PathBuf::from),
    };
    let started = runtime
        .container_control(container)
        .unwrap()
        .start(&launch_files, &container.registration)
        .await
        .unwrap();
    repo.observe_runtime_launch(
        &launch,
        "gateway_started",
        Some(started.snapshot.unwrap().init_pid as i32),
    )
    .await
    .unwrap();
    tokio::fs::write(private.join("controller-restart"), "1")
        .await
        .unwrap();
    let retained = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    let agent_before = serde_json::to_value(repo.get_agent(agent.id).await.unwrap()).unwrap();
    let files = [
        private
            .join(format!("{}.container-prepared.json", agent.id))
            .to_string_lossy()
            .into_owned(),
        container.mapping_file.clone().unwrap(),
        private.join("start-calls").to_string_lossy().into_owned(),
    ];
    let mut original_bytes = Vec::new();
    for path in &files {
        original_bytes.push(tokio::fs::read(path).await.unwrap());
    }
    let restarted = lifecycle_tests::supervisor(Arc::new(config.clone()), repo.clone());
    assert_ne!(restarted.controller_id, launch.controller_id);
    for _ in 0..2 {
        let result = restarted.health_locked(&agent).await.unwrap();
        assert_eq!(result.status, AgentStatus::Degraded);
        assert!(
            result
                .message
                .contains("ownership transfer remains required")
        );
    }
    assert!(restarted.gateway_launch_generation(agent.id).await.is_err());
    assert!(restarted.stop_locked(&agent).await.is_err());
    assert!(
        restarted
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert!(restarted.launches.lock().await.is_empty());
    assert!(restarted.children.lock().await.is_empty());
    let after = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(after.binding).unwrap(),
        serde_json::to_value(&retained.binding).unwrap()
    );
    assert_eq!(after.state, retained.state);
    assert_eq!(after.pid, retained.pid);
    assert_eq!(
        serde_json::to_value(repo.get_agent(agent.id).await.unwrap()).unwrap(),
        agent_before
    );
    for (path, bytes) in files.iter().zip(&original_bytes) {
        assert_eq!(&tokio::fs::read(path).await.unwrap(), bytes);
    }
    tokio::fs::write(private.join("restart-agent-drift"), "1")
        .await
        .unwrap();
    assert!(restarted.health_locked(&agent).await.is_err());
    tokio::fs::remove_file(private.join("restart-agent-drift"))
        .await
        .unwrap();
    let mut changed_config = config;
    changed_config
        .fleet
        .container_control
        .as_mut()
        .unwrap()
        .source_sha256[2] = "0".repeat(64);
    let changed = lifecycle_tests::supervisor(Arc::new(changed_config), repo.clone());
    assert!(changed.health_locked(&agent).await.is_err());
    assert_eq!(
        serde_json::to_value(repo.get_agent(agent.id).await.unwrap()).unwrap(),
        agent_before
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn mapped_container_persists_original_proof_and_rejects_drift_before_launch() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = with_mapping_controller(fake_creation(&config, &agent, false).await);
    let runtime = lifecycle_tests::supervisor(Arc::new(config.clone()), repo.clone());
    let original = runtime
        .prepared_container(&agent, LaunchPhase::Regular)
        .await
        .unwrap();
    let container = original.container.as_ref().unwrap();
    let mapping = container.mount_mapping.as_ref().unwrap();
    assert_eq!(container.registration.contract_version, 3);
    assert_eq!(
        container.registration.mount_mapping_sha256,
        Some(container_control::canonical_hash(mapping).unwrap())
    );
    assert_eq!(container.policy["mounts"][1]["source"], "qa_owned_agents");
    assert_eq!(
        container.policy["mounts"][1]["subpath"],
        format!("{}/config", agent.name)
    );
    let private = root.join("controller");
    let intent_path = private.join(format!("{}.container-creation.json", agent.id));
    let intent_bytes = tokio::fs::read(&intent_path).await.unwrap();
    let intent: Value = serde_json::from_slice(&intent_bytes).unwrap();
    assert_eq!(
        intent["mount_mapping"],
        serde_json::to_value(mapping).unwrap()
    );
    assert_eq!(intent["mapping_file"], json!(container.mapping_file));
    let file = Path::new(container.mapping_file.as_ref().unwrap());
    let proof_bytes = tokio::fs::read(file).await.unwrap();
    let repeated = runtime
        .prepared_container(&agent, LaunchPhase::Regular)
        .await
        .unwrap();
    assert_eq!(repeated.command_sha256, original.command_sha256);
    assert_eq!(repeated.id, original.id);
    for field in ["missing", "digest", "downgrade", "file", "local", "sibling"] {
        let mut changed = original.clone();
        let container = changed.container.as_mut().unwrap();
        match field {
            "missing" => container.mount_mapping = None,
            "digest" => container.registration.mount_mapping_sha256 = Some("0".repeat(64)),
            "downgrade" => {
                container.registration.contract_version = 2;
                container.registration.mount_mapping_sha256 = None;
                container.policy["contract_version"] = json!(2);
            }
            "file" => container.mapping_file = Some(container.journal.clone()),
            "local" => changed.paths.config = "/foreign/agent1/config".into(),
            _ => container.policy["mounts"][1]["subpath"] = json!("agent999/config"),
        }
        changed.command_sha256 = crate::runtime_launches::snapshot_hash(
            &serde_json::to_value(changed.container.as_ref().unwrap()).unwrap(),
        )
        .unwrap();
        assert!(
            crate::runtime_launches::validate_container_binding(&changed).is_err(),
            "{field}"
        );
    }
    tokio::fs::write(private.join("mapping-drift"), "1")
        .await
        .unwrap();
    assert!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(!private.join("start-effect").exists());
    tokio::fs::remove_file(private.join("mapping-drift"))
        .await
        .unwrap();
    tokio::fs::write(file, b"{}").await.unwrap();
    assert!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    tokio::fs::write(file, &proof_bytes).await.unwrap();
    let mut changed = config.clone();
    changed
        .fleet
        .container_control
        .as_mut()
        .unwrap()
        .bridge_controller
        .as_mut()
        .unwrap()
        .container_id = "f".repeat(64);
    let replacement = lifecycle_tests::supervisor(Arc::new(changed), repo.clone());
    assert!(
        replacement
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert_eq!(tokio::fs::read(&intent_path).await.unwrap(), intent_bytes);
    assert_eq!(tokio::fs::read(file).await.unwrap(), proof_bytes);
    assert!(!private.join("start-effect").exists());
    assert!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    let launch = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(launch.state, "claimed");
    assert_eq!(
        launch.binding.container.unwrap().mount_mapping.as_ref(),
        Some(mapping)
    );
    assert!(
        runtime
            .start_locked(
                &repo.get_agent(agent.id).await.unwrap(),
                LaunchPhase::Regular
            )
            .await
            .is_err()
    );
    assert_eq!(
        tokio::fs::read_to_string(private.join("prepare-effect"))
            .await
            .unwrap(),
        "1"
    );
    assert_eq!(
        tokio::fs::read_to_string(private.join("start-effect"))
            .await
            .unwrap(),
        "1"
    );
    tokio::fs::write(private.join("recover-start"), "1")
        .await
        .unwrap();
    runtime
        .health_locked(&repo.get_agent(agent.id).await.unwrap())
        .await
        .unwrap();
    assert_eq!(
        runtime.gateway_launch_generation(agent.id).await.unwrap(),
        Some(original.id)
    );
    assert_eq!(
        runtime.stop_locked(&agent).await.unwrap().status,
        AgentStatus::Stopped
    );
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn mapped_unknown_preparation_never_adopts_changed_controller_snapshot() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = with_mapping_controller(fake_creation(&config, &agent, true).await);
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    assert!(
        runtime
            .prepared_container(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    let private = root.join("controller");
    let path = private.join(format!("{}.container-creation.json", agent.id));
    let bytes = tokio::fs::read(&path).await.unwrap();
    assert!(serde_json::from_slice::<Value>(&bytes).unwrap()["mount_mapping"].is_object());
    tokio::fs::write(private.join("mapping-drift"), "1")
        .await
        .unwrap();
    tokio::fs::remove_file(private.join("prepare-unknown"))
        .await
        .unwrap();
    assert!(
        runtime
            .prepared_container(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert_eq!(tokio::fs::read(&path).await.unwrap(), bytes);
    assert!(
        !private
            .join(format!("{}.container-prepared.json", agent.id))
            .exists()
    );
    assert!(!private.join("start-effect").exists());
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    tokio::fs::remove_file(private.join("mapping-drift"))
        .await
        .unwrap();
    let prepared = runtime
        .prepared_container(&agent, LaunchPhase::Regular)
        .await
        .unwrap();
    assert_eq!(prepared.container.unwrap().registration.contract_version, 3);
    assert_eq!(tokio::fs::read(&path).await.unwrap(), bytes);
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn container_restart_requires_original_exit_and_preserves_previous_generation() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = fake_creation(&config, &agent, false).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    assert!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    let original = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    let private = root.join("controller");
    let intent_path = private.join(format!("{}.container-creation.json", agent.id));
    let prepared_path = private.join(format!("{}.container-prepared.json", agent.id));
    let original_intent = tokio::fs::read(&intent_path).await.unwrap();
    let original_prepared = tokio::fs::read(&prepared_path).await.unwrap();
    let next_path = private.join(format!("{}.1.container-creation.json", agent.id));
    let starting = repo.get_agent(agent.id).await.unwrap();
    assert!(repo.next_container_launch_ordinal(agent.id).await.is_err());
    assert!(runtime.restart(&starting).await.is_err());
    assert!(!next_path.exists());
    assert_eq!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .unwrap()
            .binding
            .id,
        original.binding.id
    );

    // The fake Base resolves only its original ACK and attests namespace exit.
    tokio::fs::write(private.join("recover-start"), "1")
        .await
        .unwrap();
    assert!(runtime.restart(&starting).await.is_err());
    let next = repo
        .get_open_runtime_launch(agent.id)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(next.binding.id, original.binding.id);
    assert_ne!(
        next.binding
            .container
            .as_ref()
            .unwrap()
            .registration
            .container_id,
        original
            .binding
            .container
            .as_ref()
            .unwrap()
            .registration
            .container_id
    );
    assert_eq!(next.state, "claimed");
    assert!(next_path.exists());
    assert_eq!(
        tokio::fs::read(&intent_path).await.unwrap(),
        original_intent
    );
    assert_eq!(
        tokio::fs::read(&prepared_path).await.unwrap(),
        original_prepared
    );
    let next_intent = tokio::fs::read(&next_path).await.unwrap();
    tokio::fs::remove_file(private.join("recover-start"))
        .await
        .unwrap();
    let fresh = repo.get_agent(agent.id).await.unwrap();
    assert!(
        runtime
            .start_locked(&fresh, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert_eq!(tokio::fs::read(&next_path).await.unwrap(), next_intent);
    assert!(runtime.children.lock().await.is_empty());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn lost_creation_intent_cannot_create_another_container_or_fall_back_to_native() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let config = fake_creation(&config, &agent, true).await;
    let runtime = lifecycle_tests::supervisor(Arc::new(config), repo.clone());
    assert!(
        runtime
            .prepared_container(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    let private = root.join("controller");
    let path = private.join(format!("{}.container-creation.json", agent.id));
    let bytes = tokio::fs::read(&path).await.unwrap();
    let intent: Value = serde_json::from_slice(&bytes).unwrap();
    let row = repo.db.query_one(sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT generation,intent_sha256 FROM runtime_container_preparations WHERE agent_id=$1 AND ordinal=0",
        [agent.id.into()],
    )).await.unwrap().unwrap();
    assert_eq!(
        json!(row.try_get::<Uuid>("", "generation").unwrap()),
        intent["generation"]
    );
    assert_eq!(
        row.try_get::<String>("", "intent_sha256").unwrap(),
        crate::runtime_launches::snapshot_hash(&intent).unwrap()
    );
    let calls = tokio::fs::read(private.join("prepare-calls"))
        .await
        .unwrap();
    tokio::fs::remove_file(&path).await.unwrap();
    tokio::fs::remove_file(private.join("prepare-unknown"))
        .await
        .unwrap();
    assert!(
        runtime
            .start_locked(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    assert!(
        !path.exists(),
        "rejected replacement must not overwrite a lost original"
    );
    assert_eq!(
        tokio::fs::read(private.join("prepare-calls"))
            .await
            .unwrap(),
        calls
    );
    assert!(!private.join("start-effect").exists());
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );

    let native = lifecycle_tests::supervisor(runtime.config.clone(), repo.clone());
    let native_binding = app::runtime_launch::RuntimeLaunchBinding {
        id: Uuid::new_v4(),
        agent_id: agent.id,
        controller_id: native.controller_id,
        kind: agent.kind,
        paths: agent.paths.clone(),
        api_port: agent.api_port,
        phase: "regular".into(),
        configuration_revision: None,
        configuration_sha256: None,
        command_sha256: "a".repeat(64),
        container: None,
    };
    assert!(repo.claim_runtime_launch(&native_binding).await.is_err());
    // Restoring the exact original private document permits readback on its original key.
    tokio::fs::write(&path, &bytes).await.unwrap();
    use std::os::unix::fs::PermissionsExt;
    tokio::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .await
        .unwrap();
    let recovered = runtime
        .prepared_container(&agent, LaunchPhase::Regular)
        .await
        .unwrap();
    assert_eq!(json!(recovered.id), intent["generation"]);
    assert!(!private.join("start-effect").exists());
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn whole_private_directory_loss_retains_database_precreate_fence() {
    let Some((repo, agent, _, config, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await
    else {
        return;
    };
    let mut config = fake_creation(&config, &agent, true).await;
    // Keep the pinned utility available: the hold must come from DB custody,
    // not a missing executable prerequisite after deleting private documents.
    let source = root.join("base-source");
    tokio::fs::create_dir_all(source.join("scripts"))
        .await
        .unwrap();
    let control = config.fleet.container_control.as_mut().unwrap();
    for name in [
        "runtime_boundary.py",
        "runtime_bootstrap.py",
        "runtime_control.py",
    ] {
        let bytes = tokio::fs::read(Path::new(&control.base_root).join("scripts").join(name))
            .await
            .unwrap();
        tokio::fs::write(source.join("scripts").join(name), bytes)
            .await
            .unwrap();
    }
    control.base_root = source.to_string_lossy().into_owned();
    let runtime = lifecycle_tests::supervisor(Arc::new(config.clone()), repo.clone());
    assert!(
        runtime
            .prepared_container(&agent, LaunchPhase::Regular)
            .await
            .is_err()
    );
    let private = root.join("controller");
    tokio::fs::remove_dir_all(&private).await.unwrap();
    tokio::fs::create_dir(&private).await.unwrap();
    use std::os::unix::fs::PermissionsExt;
    tokio::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o700))
        .await
        .unwrap();
    let controller = activation_journal::private_controller_directory(&private)
        .await
        .unwrap();
    for supervisor in [
        &runtime,
        &lifecycle_tests::supervisor(Arc::new(config), repo.clone()),
    ] {
        assert!(
            matches!(supervisor.start_locked(&agent, LaunchPhase::Regular).await,
            Err(AppError::Unavailable(detail))
                if detail == "original container preparation requires reconciliation")
        );
        assert!(!controller.join("prepare-effect").exists());
        assert!(!controller.join("start-effect").exists());
        assert!(
            !controller
                .join(format!("{}.container-creation.json", agent.id))
                .exists()
        );
    }
    assert!(
        repo.get_open_runtime_launch(agent.id)
            .await
            .unwrap()
            .is_none()
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[tokio::test]
async fn precreate_claim_serializes_controllers_and_replays_only_exact_identity() {
    let Some((repo, agent, _, _, root)) = lifecycle_tests::fixture(AgentKind::Hermes).await else {
        return;
    };
    let first = app::runtime_launch::RuntimeContainerPreparation {
        agent_id: agent.id,
        ordinal: 0,
        controller_id: Uuid::new_v4(),
        generation: Uuid::new_v4(),
        operation_id: Uuid::new_v4(),
        intent_sha256: "a".repeat(64),
    };
    let mut competitor = first.clone();
    competitor.controller_id = Uuid::new_v4();
    competitor.generation = Uuid::new_v4();
    competitor.operation_id = Uuid::new_v4();
    let configuration = app::runtime_launch::RuntimeConfigurationClaim {
        phase: "regular".into(),
        revision: None,
        sha256: None,
    };
    let (a, b) = tokio::join!(
        repo.claim_container_preparation(&first, &configuration),
        repo.claim_container_preparation(&competitor, &configuration)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let original = if a.is_ok() { first } else { competitor };
    repo.claim_container_preparation(&original, &configuration)
        .await
        .unwrap();
    for field in ["controller", "generation", "operation", "hash", "ordinal"] {
        let mut changed = original.clone();
        match field {
            "controller" => changed.controller_id = Uuid::new_v4(),
            "generation" => changed.generation = Uuid::new_v4(),
            "operation" => changed.operation_id = Uuid::new_v4(),
            "hash" => changed.intent_sha256 = "b".repeat(64),
            "ordinal" => changed.ordinal += 1,
            _ => unreachable!(),
        }
        assert!(
            repo.claim_container_preparation(&changed, &configuration)
                .await
                .is_err(),
            "{field}"
        );
    }
    let row = repo
        .db
        .query_one(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT count(*) AS count FROM runtime_container_preparations WHERE agent_id=$1",
            [agent.id.into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.try_get::<i64>("", "count").unwrap(), 1);
    tokio::fs::remove_dir_all(root).await.unwrap();
}
