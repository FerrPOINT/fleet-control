use super::container_control::*;
use super::*;
use std::path::PathBuf;

fn fixture() -> (ContainerLaunchFiles, Value, Uuid) {
    let operation = Uuid::new_v4();
    let policy = json!({"contract_version":2,"resource_id":Uuid::new_v4(),"generation":Uuid::new_v4(),
        "network":{"id":"0".repeat(64),"name":"owned","internal":true}});
    let files = ContainerLaunchFiles {
        policy: policy.clone(),
        compose: "/private/start.json".into(),
        journal: "/private/launch.sqlite".into(),
        stop_journal: "/private/stop.sqlite".into(),
        mapped: None,
        recovery: None,
    };
    let mut allocated = policy;
    allocated["network"]["id"] = json!("c".repeat(64));
    let receipt = json!({"state":"prepared","policy":allocated,"registration":{
        "contract_version":2,"operation_id":operation,"container_id":"a".repeat(64),
        "resource_id":allocated["resource_id"],"generation":allocated["generation"],
        "engine":{"ID":"engine","KernelVersion":"kernel","ServerVersion":"29"},
        "policy_sha256":canonical_hash(&allocated).unwrap(),"inventory_sha256":"b".repeat(64),
        "running_inventory_sha256":"d".repeat(64),"compose_sha256":"e".repeat(64),"network_sha256":"f".repeat(64)}});
    (files, receipt, operation)
}

#[tokio::test]
async fn adapter_fake_preserves_original_command_on_unknown_readback() {
    use sha2::{Digest, Sha256};
    let root = std::env::temp_dir().join(format!("fleet-preparation-adapter-{}", Uuid::new_v4()));
    tokio::fs::create_dir_all(root.join("scripts"))
        .await
        .unwrap();
    let (mut files, receipt, operation) = fixture();
    files.compose = root.join("start.json");
    files.journal = root.join("launch.sqlite");
    files.stop_journal = root.join("stop.sqlite");
    let process = json!({"environment":{"API_SERVER_KEY":"private-original"}});
    let source = format!(
        r#"import json,sys
from pathlib import Path
r=json.load(sys.stdin)
assert r['protocol_version']==1
assert r['operation_id']=={operation:?}
assert r['process']['environment']['API_SERVER_KEY']=='private-original'
assert set(r)=={{'protocol_version','action','context','policy','compose','journal','process','operation_id','creation_compose','creation_journal'}}
with Path(r['creation_journal']).open('a') as f: f.write(r['action']+'\n')
result=json.loads({encoded:?})
if r['action']=='prepare':
    print(json.dumps({{'protocol_version':1,'action':r['action'],'result':{{'state':'held'}}}}))
    sys.exit(2)
assert r['action']=='reconcile_preparation'
print(json.dumps({{'protocol_version':1,'action':r['action'],'result':result}}))
"#,
        operation = operation.to_string(),
        encoded = receipt.to_string()
    );
    let sources = [
        "# fake boundary\n".to_owned(),
        "# fake bootstrap\n".to_owned(),
        source,
    ];
    for (name, source) in [
        "runtime_boundary.py",
        "runtime_bootstrap.py",
        "runtime_control.py",
    ]
    .iter()
    .zip(&sources)
    {
        tokio::fs::write(root.join("scripts").join(name), source)
            .await
            .unwrap();
    }
    let control = ContainerControl::new(
        PathBuf::from(std::env::var("FLEET_TEST_PYTHON").unwrap_or_else(|_| "python3".into())),
        ControlSource {
            root: root.clone(),
            sha256: sources.map(|s| hex::encode(Sha256::digest(s.as_bytes()))),
        },
        "desktop-linux".into(),
    )
    .unwrap();
    let create = root.join("create.json");
    let journal = root.join("create.sqlite");
    assert!(
        control
            .preparation(&files, &process, operation, &create, &journal, true)
            .await
            .is_err()
    );
    let read = control
        .preparation(&files, &process, operation, &create, &journal, false)
        .await
        .unwrap();
    assert_eq!(read.registration.operation_id, operation);
    let replay = control
        .preparation(&files, &process, operation, &create, &journal, false)
        .await
        .unwrap();
    assert_eq!(read.registration, replay.registration);
    assert_eq!(
        tokio::fs::read_to_string(journal).await.unwrap(),
        "prepare\nreconcile_preparation\nreconcile_preparation\n"
    );
    tokio::fs::remove_dir_all(root).await.unwrap();
}

#[test]
fn preparation_receipt_allows_only_original_bridge_allocation() {
    let (files, receipt, operation) = fixture();
    container_control::validate_preparation_receipt(receipt.clone(), 0, &files, operation).unwrap();
    for path in ["generation", "resource_id", "operation_id"] {
        let mut changed = receipt.clone();
        changed["registration"][path] = json!(Uuid::new_v4());
        assert!(
            container_control::validate_preparation_receipt(changed, 0, &files, operation).is_err()
        );
    }
    for (key, value) in [
        ("image_id", json!("sha256:foreign")),
        ("mounts", json!([])),
        (
            "network",
            json!({"id":"0".repeat(64),"name":"owned","internal":true}),
        ),
    ] {
        let mut changed = receipt.clone();
        changed["policy"][key] = value;
        changed["registration"]["policy_sha256"] =
            json!(canonical_hash(&changed["policy"]).unwrap());
        assert!(
            container_control::validate_preparation_receipt(changed, 0, &files, operation).is_err()
        );
    }
}

#[test]
fn unknown_or_foreign_preparation_ack_does_not_become_ready() {
    let (files, mut receipt, operation) = fixture();
    let mut malformed = receipt.clone();
    malformed["policy"] = json!("not-a-policy");
    assert!(
        container_control::validate_preparation_receipt(malformed, 0, &files, operation).is_err()
    );
    assert!(
        container_control::validate_preparation_receipt(receipt.clone(), 2, &files, operation)
            .is_err()
    );
    receipt["origin"] = json!("http://foreign:1");
    assert!(
        container_control::validate_preparation_receipt(receipt, 0, &files, operation).is_err()
    );
    assert!(
        container_control::validate_preparation_receipt(
            json!({"state":"held"}),
            2,
            &files,
            operation
        )
        .is_err()
    );
}
