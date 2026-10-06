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
    let program = r#"import sys,json,hashlib
from pathlib import Path
r=json.load(sys.stdin);root=Path(r['journal']).parent
digest=lambda v:hashlib.sha256(json.dumps(v,sort_keys=True,separators=(',',':')).encode()).hexdigest()
if r['protocol_version']==2:
 assert r['policy']['contract_version']==3
 mapping=r['mount_mapping'];file=Path(r['mapping_file'])
 assert not (root/'mapping-drift').exists()
 raw=json.dumps(mapping,sort_keys=True,separators=(',',':'))
 if r['action']=='prepare' and not file.exists():file.write_text(raw);file.chmod(0o600)
 assert file.read_text()==raw
if r['action']=='resolve_mounts':
 mounts=[dict(m,source='/daemon/volumes/own/_data/'+m['source'][len(r['local_root'])+1:]) for m in r['policy']['mounts']]
 result={'state':'resolved','controller':r['controller'],'snapshot':{'container_id':r['controller']['container_id'],'started_at':'2026-10-06T12:00:00Z','init_pid':999,'inventory_sha256':'a'*64},'engine':{'ID':'original-engine','KernelVersion':'original-kernel','ServerVersion':'29'},'local_root':r['local_root'],'volume_name':'qa_owned_agents','volume_sha256':'b'*64,'mounts':mounts,'input_policy_sha256':digest(r['policy'])}
 if (root/'mapping-drift').exists():result['snapshot']['init_pid']+=1
elif r['action']=='prepare':
 with (root/'prepare-calls').open('a') as calls:calls.write(r['operation_id']+'\n')
 intents=[json.loads(p.read_bytes()) for p in root.glob(r['policy']['resource_id']+'*.container-creation.json')]
 intent=next(v for v in intents if v['generation']==r['policy']['generation'])
 assert intent['generation']==r['policy']['generation'] and intent['operation_id']==r['operation_id']
 assert r['process']['environment']['HERMES_HOME']=='/config' and r['process']['working_dir']=='/workspace'
 if not (root/'prepare-effect').exists():(root/'prepare-effect').write_text('1')
 if (root/'prepare-unknown').exists():
  result={'state':'held','operation_id':r['operation_id'],'resource_id':r['policy']['resource_id'],'generation':r['policy']['generation']}
 else:
  policy=r['policy'];policy['network']['id']='1'*64
  reg={'contract_version':policy['contract_version'],'operation_id':r['operation_id'],'container_id':digest(policy['generation']),'resource_id':policy['resource_id'],'generation':policy['generation'],'engine':{'ID':'original-engine','KernelVersion':'original-kernel','ServerVersion':'29'},'policy_sha256':digest(policy),'inventory_sha256':'c'*64,'running_inventory_sha256':'d'*64,'compose_sha256':'e'*64,'network_sha256':'f'*64}
  if r['protocol_version']==2:reg['mount_mapping_sha256']=digest(r['mount_mapping'])
  result={'state':'prepared','policy':policy,'registration':reg}
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
