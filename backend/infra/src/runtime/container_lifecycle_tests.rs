use super::*;
use app::runtime_launch::{
    ContainerEngineIdentity, ContainerRegistration, RuntimeContainerBinding, RuntimeLaunchBinding,
    RuntimeLaunchRecord,
};
use sha2::{Digest, Sha256};
use std::path::Path;

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
        },
    );
    let container = launch.container.as_ref().unwrap();
    let files = container_control::ContainerLaunchFiles {
        policy: container.policy.clone(),
        compose: container.compose.clone().into(),
        journal: container.journal.clone().into(),
        stop_journal: container.stop_journal.clone().into(),
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
