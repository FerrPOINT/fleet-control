//! Private Base subprocess protocol. No Docker command or credential is agent-supplied.
pub use app::runtime_launch::{
    ContainerEngineIdentity as EngineIdentity, ContainerMountMapping, ContainerRegistration,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use shared::AppError;
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use uuid::Uuid;

const LIMIT: usize = 64 * 1024;
const SOURCE_LIMIT: usize = 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(60);
// Compile captured, hash-checked bytes; never import checkout files or cached bytecode.
const BOOTSTRAP: &str = r#"import base64,io,json,sys,types
from pathlib import Path
payload=json.load(sys.stdin)
request=json.dumps(payload['request'],ensure_ascii=False,separators=(',',':')).encode('utf-8')
sys.stdin=io.TextIOWrapper(io.BytesIO(request),encoding='utf-8')
package=types.ModuleType('scripts')
package.__path__=[]
sys.modules['scripts']=package
for name,source in zip(('runtime_boundary','runtime_bootstrap','runtime_control'),payload['sources'],strict=True):
    qualified='scripts.'+name
    module=types.ModuleType('__main__' if name=='runtime_control' else qualified)
    module.__package__='scripts'
    module.__file__=str(Path(sys.argv[1])/'scripts'/(name+'.py'))
    sys.modules[qualified]=module
    setattr(package,name,module)
    if name=='runtime_control':
        sys.modules['__main__']=module
    exec(compile(base64.b64decode(source,validate=True),module.__file__,'exec'),module.__dict__)
"#;

#[derive(Debug, Clone)]
pub struct ControlSource {
    pub root: PathBuf,
    /// Boundary, bootstrap and control bytes from an operator-pinned Base checkout.
    pub sha256: [String; 3],
}

#[derive(Debug, Clone)]
pub struct ContainerControl {
    python: PathBuf,
    source: ControlSource,
    context: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerSnapshot {
    pub contract_version: u8,
    pub container_id: String,
    pub engine: EngineIdentity,
    pub policy_sha256: String,
    pub inventory_sha256: String,
    pub started_at: String,
    pub init_pid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_sha256: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerObservation {
    NeverStarted,
    Running,
    NamespaceExited,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerReceiptState {
    Registered,
    Observed,
    Held,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerReceipt {
    pub contract_version: u8,
    pub operation_id: Uuid,
    pub container_id: String,
    pub resource_id: Uuid,
    pub generation: Uuid,
    pub registration_sha256: String,
    pub state: ContainerReceiptState,
    pub observation: ContainerObservation,
    #[serde(deserialize_with = "required_snapshot")]
    pub snapshot: Option<ContainerSnapshot>,
}

fn required_snapshot<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<ContainerSnapshot>, D::Error> {
    Option::deserialize(deserializer)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerStopReceipt {
    pub contract_version: u8,
    pub operation_id: Uuid,
    pub container_id: String,
    pub resource_id: Uuid,
    pub generation: Uuid,
    pub snapshot_sha256: String,
    pub state: ContainerReceiptState,
    pub observation: ContainerObservation,
}

/// Paths remain controller-private; neither these nor registrations are public agent DTOs.
#[derive(Debug, Clone)]
pub struct ContainerLaunchFiles {
    pub policy: Value,
    pub compose: PathBuf,
    pub journal: PathBuf,
    pub stop_journal: PathBuf,
    pub mount_mapping: Option<ContainerMountMapping>,
    pub mapping_file: Option<PathBuf>,
}

// Resolved credentials remain private. Never derive Debug or expose this as an API DTO.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerProcess {
    pub user: String,
    pub entrypoint: Vec<String>,
    pub command: Vec<String>,
    pub environment: std::collections::BTreeMap<String, String>,
    pub working_dir: String,
    pub pids_limit: u32,
    pub memory_bytes: u64,
    pub nano_cpus: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerPreparation {
    pub(super) state: String,
    pub policy: Value,
    pub registration: ContainerRegistration,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ControllerAttachment {
    state: String,
    registration_sha256: String,
    controller_id: String,
    controller_sha256: String,
    network_id: String,
}

pub(super) fn validate_preparation(
    prepared: &ContainerPreparation,
    policy: &Value,
    operation_id: Uuid,
    mapping: Option<&ContainerMountMapping>,
) -> Result<(), AppError> {
    validate_registration(&prepared.registration)?;
    let mut expected = policy.clone();
    let network_id = prepared.policy["network"]["id"].as_str().ok_or_else(held)?;
    if policy["network"]["id"] != "0".repeat(64)
        || !hash(network_id)
        || network_id == "0".repeat(64)
    {
        return Err(held());
    }
    expected["network"]["id"] = json!(network_id);
    if prepared.state != "prepared"
        || prepared.policy != expected
        || !matches!(prepared.registration.contract_version, 2 | 3)
        || policy["contract_version"] != json!(prepared.registration.contract_version)
        || prepared.registration.operation_id != operation_id
        || prepared.registration.policy_sha256 != canonical_hash(&prepared.policy)?
        || prepared.policy["resource_id"] != json!(prepared.registration.resource_id)
        || prepared.policy["generation"] != json!(prepared.registration.generation)
    {
        return Err(held());
    }
    validate_mapping_registration(&prepared.policy, mapping, &prepared.registration)?;
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    protocol_version: u8,
    action: String,
    result: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Endpoint {
    receipt: ContainerReceipt,
    host: std::net::Ipv4Addr,
}

// Raw stdout/stderr may contain credentials; never derive Debug or expose an API DTO.
pub struct ContainerLogTail {
    pub receipt: ContainerReceipt,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LogEnvelope {
    receipt: ContainerReceipt,
    stdout_base64: String,
    stderr_base64: String,
}

fn decode_logs(
    status: i32,
    value: Value,
    original: &ContainerRegistration,
) -> Result<ContainerLogTail, AppError> {
    let envelope: LogEnvelope = serde_json::from_value(value).map_err(|_| held())?;
    validate_receipt(&envelope.receipt, original, status, "logs")?;
    if envelope.receipt.state != ContainerReceiptState::Observed {
        return Err(held());
    }
    let stdout = STANDARD
        .decode(envelope.stdout_base64)
        .map_err(|_| held())?;
    let stderr = STANDARD
        .decode(envelope.stderr_base64)
        .map_err(|_| held())?;
    if stdout.len() + stderr.len() > 32 * 1024 {
        return Err(held());
    }
    Ok(ContainerLogTail {
        receipt: envelope.receipt,
        stdout,
        stderr,
    })
}

fn held() -> AppError {
    AppError::Unavailable(
        "original Docker operation requires reconciliation; no automatic resend".into(),
    )
}

fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(crate) fn canonical_hash(value: &impl Serialize) -> Result<String, AppError> {
    let mut sorted = serde_json::to_value(value).map_err(|_| held())?;
    sorted.sort_all_objects();
    Ok(hex::encode(Sha256::digest(
        serde_json::to_vec(&sorted).map_err(|_| held())?,
    )))
}

pub(crate) fn validate_registration(value: &ContainerRegistration) -> Result<(), AppError> {
    if !matches!(value.contract_version, 1..=3)
        || value.operation_id.is_nil()
        || value.resource_id.is_nil()
        || value.generation.is_nil()
        || (value.contract_version >= 2) != value.network_sha256.is_some()
        || (value.contract_version == 3) != value.mount_mapping_sha256.is_some()
        || [
            &value.container_id,
            &value.policy_sha256,
            &value.inventory_sha256,
            &value.running_inventory_sha256,
            &value.compose_sha256,
        ]
        .iter()
        .any(|value| !hash(value))
        || value
            .network_sha256
            .as_ref()
            .is_some_and(|value| !hash(value))
        || value
            .mount_mapping_sha256
            .as_ref()
            .is_some_and(|value| !hash(value))
        || [
            &value.engine.id,
            &value.engine.kernel_version,
            &value.engine.server_version,
        ]
        .iter()
        .any(|value| value.is_empty() || value.len() > 256 || !value.is_ascii())
    {
        return Err(held());
    }
    Ok(())
}

fn linux_directory(value: &str) -> bool {
    value.starts_with('/')
        && value != "/"
        && value.len() <= 4096
        && !value.contains(['\\', '\0'])
        && value[1..]
            .split('/')
            .all(|part| !matches!(part, "" | "." | ".."))
}

fn agent_name(value: &str) -> bool {
    value.strip_prefix("agent").is_some_and(|ordinal| {
        !ordinal.is_empty()
            && ordinal.as_bytes()[0].is_ascii_digit()
            && ordinal.as_bytes()[0] != b'0'
            && ordinal.bytes().all(|byte| byte.is_ascii_digit())
    })
}

pub(super) fn validate_mount_mapping(
    mapping: &ContainerMountMapping,
    local: &Value,
    controller: &shared::config::BridgeControllerConfig,
    local_root: &str,
) -> Result<(), AppError> {
    if mapping.state != "resolved"
        || mapping.local_root != local_root
        || !linux_directory(local_root)
        || mapping.controller.container_id != controller.container_id
        || mapping.controller.image_id != controller.image_id
        || mapping.controller.service != controller.service
        || !hash(&mapping.controller.container_id)
        || !mapping
            .controller
            .image_id
            .strip_prefix("sha256:")
            .is_some_and(hash)
        || !matches!(
            mapping.controller.service.as_str(),
            "fleet-backend" | "fleet-control-backend"
        )
        || mapping.snapshot.container_id != mapping.controller.container_id
        || mapping.snapshot.init_pid == 0
        || mapping.snapshot.started_at.is_empty()
        || mapping.snapshot.started_at.starts_with("0001-")
        || [
            &mapping.snapshot.inventory_sha256,
            &mapping.volume_sha256,
            &mapping.input_policy_sha256,
        ]
        .iter()
        .any(|value| !hash(value))
        || [
            &mapping.engine.id,
            &mapping.engine.kernel_version,
            &mapping.engine.server_version,
        ]
        .iter()
        .any(|value| value.is_empty() || value.len() > 256 || !value.is_ascii())
        || mapping.volume_name.is_empty()
        || mapping.volume_name.len() > 128
        || !mapping.volume_name.as_bytes()[0].is_ascii_alphanumeric()
        || !mapping
            .volume_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
        || local["contract_version"] != 2
        || local["network"]["id"] != "0".repeat(64)
        || mapping.input_policy_sha256 != canonical_hash(local)?
        || mapping.mounts.len() != 4
    {
        return Err(held());
    }
    let mounts = local["mounts"]
        .as_array()
        .filter(|mounts| mounts.len() == 4)
        .ok_or_else(held)?;
    let mut original_agent = None;
    let mut daemon_root = None;
    for ((local_mount, projected), area) in
        mounts
            .iter()
            .zip(&mapping.mounts)
            .zip(["runtime", "config", "workspace", "logs"])
    {
        let source = local_mount["source"].as_str().ok_or_else(held)?;
        let relative = source
            .strip_prefix(&format!("{local_root}/"))
            .ok_or_else(held)?;
        let (agent, leaf) = relative.split_once('/').ok_or_else(held)?;
        let (daemon_parent, daemon_leaf) = projected.source.rsplit_once('/').ok_or_else(held)?;
        let (base, daemon_agent) = daemon_parent.rsplit_once('/').ok_or_else(held)?;
        if !agent_name(agent)
            || leaf != area
            || !linux_directory(source)
            || !linux_directory(&projected.source)
            || !linux_directory(base)
            || daemon_agent != agent
            || daemon_leaf != area
            || original_agent.is_some_and(|original| original != agent)
            || daemon_root.is_some_and(|original| original != base)
            || local_mount
                != &json!({"type":"bind","source":source,"destination":format!("/{area}"),"read_only":area=="runtime"})
            || projected.mount_type != "bind"
            || projected.destination != format!("/{area}")
            || projected.read_only != (area == "runtime")
        {
            return Err(held());
        }
        original_agent = Some(agent);
        daemon_root = Some(base);
    }
    Ok(())
}

pub(super) fn mapped_policy(
    local: &Value,
    mapping: &ContainerMountMapping,
) -> Result<Value, AppError> {
    let controller = shared::config::BridgeControllerConfig {
        container_id: mapping.controller.container_id.clone(),
        image_id: mapping.controller.image_id.clone(),
        service: mapping.controller.service.clone(),
    };
    validate_mount_mapping(mapping, local, &controller, &mapping.local_root)?;
    let mut policy = local.clone();
    policy["contract_version"] = json!(3);
    policy["mounts"] = json!(mapping.mounts.iter().map(|mount| {
        let agent = mount.source.rsplit('/').nth(1).ok_or_else(held)?;
        Ok(json!({"type":"volume","source":mapping.volume_name,
            "subpath":format!("{agent}{}",mount.destination),"destination":mount.destination,"read_only":mount.read_only}))
    }).collect::<Result<Vec<_>, AppError>>()?);
    Ok(policy)
}

pub(crate) fn local_mapping_policy(
    policy: &Value,
    mapping: &ContainerMountMapping,
) -> Result<Value, AppError> {
    if policy["contract_version"] != 3
        || !policy["network"].is_object()
        || mapping.mounts.len() != 4
    {
        return Err(held());
    }
    let mut local = policy.clone();
    local["contract_version"] = json!(2);
    local["network"]["id"] = json!("0".repeat(64));
    local["mounts"] = json!(mapping.mounts.iter().map(|mount| {
        let agent = mount.source.rsplit('/').nth(1).ok_or_else(held)?;
        Ok(json!({"type":"bind","source":format!("{}/{agent}{}",mapping.local_root,mount.destination),
            "destination":mount.destination,"read_only":mount.read_only}))
    }).collect::<Result<Vec<_>, AppError>>()?);
    let mut expected = mapped_policy(&local, mapping)?;
    expected["network"]["id"] = policy["network"]["id"].clone();
    if expected != *policy {
        return Err(held());
    }
    Ok(local)
}

pub(crate) fn validate_mapping_registration(
    policy: &Value,
    mapping: Option<&ContainerMountMapping>,
    registration: &ContainerRegistration,
) -> Result<(), AppError> {
    match mapping {
        Some(mapping) if registration.contract_version == 3 => {
            local_mapping_policy(policy, mapping)?;
            if registration.engine != mapping.engine
                || registration.mount_mapping_sha256.as_deref()
                    != Some(canonical_hash(mapping)?.as_str())
            {
                return Err(held());
            }
        }
        None if registration.contract_version != 3 && policy["contract_version"] != 3 => {}
        _ => return Err(held()),
    }
    Ok(())
}

impl ContainerControl {
    pub fn new(python: PathBuf, source: ControlSource, context: String) -> Result<Self, AppError> {
        if python.as_os_str().is_empty()
            || !source.root.is_absolute()
            || context.is_empty()
            || context.len() > 128
            || !context
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
            || !context.as_bytes()[0].is_ascii_alphanumeric()
            || source.sha256.iter().any(|value| !hash(value))
        {
            return Err(AppError::validation(
                "invalid protected Docker control configuration",
            ));
        }
        Ok(Self {
            python,
            source,
            context,
        })
    }

    async fn call(
        &self,
        files: &ContainerLaunchFiles,
        action: &str,
        extra: Value,
    ) -> Result<(i32, Value), AppError> {
        let mut sources = Vec::with_capacity(3);
        let filesystem_root = self.source.root.ancestors().last().ok_or_else(held)?;
        for (name, expected) in [
            "runtime_boundary.py",
            "runtime_bootstrap.py",
            "runtime_control.py",
        ]
        .iter()
        .zip(&self.source.sha256)
        {
            let path = self.source.root.join("scripts").join(name);
            crate::reject_symlink_components(filesystem_root, &path)
                .await
                .map_err(|_| held())?;
            let meta = tokio::fs::symlink_metadata(&path)
                .await
                .map_err(|_| held())?;
            if !meta.is_file() || meta.len() > SOURCE_LIMIT as u64 {
                return Err(held());
            }
            let mut bytes = Vec::new();
            tokio::fs::File::open(&path)
                .await
                .map_err(|_| held())?
                .take((SOURCE_LIMIT + 1) as u64)
                .read_to_end(&mut bytes)
                .await
                .map_err(|_| held())?;
            if bytes.len() > SOURCE_LIMIT || hex::encode(Sha256::digest(&bytes)) != *expected {
                return Err(held());
            }
            sources.push(STANDARD.encode(bytes));
        }
        let protocol_version = match (&files.mount_mapping, &files.mapping_file) {
            (None, None) if files.policy["contract_version"] != 3 => 1,
            (Some(mapping), Some(path)) if action != "resolve_mounts" => {
                local_mapping_policy(&files.policy, mapping)?;
                if !path.is_absolute()
                    || path.parent() != files.journal.parent()
                    || [&files.compose, &files.journal, &files.stop_journal].contains(&path)
                {
                    return Err(held());
                }
                2
            }
            _ => return Err(held()),
        };
        let mut request = json!({"protocol_version":protocol_version, "action":action, "context":self.context,
            "policy":files.policy, "compose":files.compose, "journal":files.journal});
        if protocol_version == 2 {
            request["mount_mapping"] = json!(files.mount_mapping);
            request["mapping_file"] = json!(files.mapping_file);
        }
        if !files.compose.is_absolute()
            || !files.journal.is_absolute()
            || !files.stop_journal.is_absolute()
        {
            return Err(AppError::validation(
                "Docker launch storage must be absolute",
            ));
        }
        if extra
            .as_object()
            .ok_or_else(held)?
            .keys()
            .any(|key| request.get(key).is_some())
        {
            return Err(held());
        }
        request
            .as_object_mut()
            .ok_or_else(held)?
            .extend(extra.as_object().ok_or_else(held)?.clone());
        if serde_json::to_vec(&request).map_err(|_| held())?.len() > LIMIT {
            return Err(held());
        }
        let payload = serde_json::to_vec(&json!({"sources":sources,"request":request}))
            .map_err(|_| held())?;
        let mut command = Command::new(&self.python);
        command
            .args(["-I", "-B", "-c", BOOTSTRAP])
            .arg(&self.source.root)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        for key in [
            "PATH",
            "HOME",
            "USERPROFILE",
            "SYSTEMROOT",
            "WINDIR",
            "TEMP",
            "TMP",
            "DOCKER_CONFIG",
            "DOCKER_HOST",
            "DOCKER_TLS",
            "DOCKER_TLS_VERIFY",
            "DOCKER_CERT_PATH",
        ] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        let mut child = command.spawn().map_err(|_| held())?;
        let mut stdin = child.stdin.take().ok_or_else(held)?;
        let stdout = child.stdout.take().ok_or_else(held)?;
        let stderr = child.stderr.take().ok_or_else(held)?;
        let result = tokio::time::timeout(DEADLINE, async {
            stdin.write_all(&payload).await.map_err(|_| held())?;
            stdin.shutdown().await.map_err(|_| held())?;
            drop(stdin);
            let (status, output, _) = tokio::try_join!(
                async { child.wait().await.map_err(|_| held()) },
                bounded(stdout),
                bounded(stderr)
            )?;
            Ok::<_, AppError>((status.code().ok_or_else(held)?, output))
        })
        .await
        .map_err(|_| held())??;
        if !matches!(result.0, 0 | 2) {
            return Err(held());
        }
        let envelope: Envelope = serde_json::from_slice(&result.1).map_err(|_| held())?;
        if envelope.protocol_version != protocol_version || envelope.action != action {
            return Err(held());
        }
        Ok((result.0, envelope.result))
    }

    /// Read-only proof; never grants permission to create or start a process.
    pub async fn resolve_mounts(
        &self,
        files: &ContainerLaunchFiles,
        controller: &shared::config::BridgeControllerConfig,
        local_root: &str,
    ) -> Result<ContainerMountMapping, AppError> {
        let (status, value) = self
            .call(
                files,
                "resolve_mounts",
                json!({
                    "controller":controller,"local_root":local_root,
                }),
            )
            .await?;
        if status != 0 {
            return Err(held());
        }
        let mapping: ContainerMountMapping = serde_json::from_value(value).map_err(|_| held())?;
        validate_mount_mapping(&mapping, &files.policy, controller, local_root)?;
        Ok(mapping)
    }

    pub async fn register(
        &self,
        files: &ContainerLaunchFiles,
        container_id: &str,
        operation_id: Uuid,
    ) -> Result<ContainerRegistration, AppError> {
        let (_, value) = self
            .call(
                files,
                "register",
                json!({"container_id":container_id, "operation_id":operation_id}),
            )
            .await?;
        let registration: ContainerRegistration =
            serde_json::from_value(value).map_err(|_| held())?;
        validate_registration(&registration)?;
        validate_mapping_registration(&files.policy, files.mount_mapping.as_ref(), &registration)?;
        if registration.container_id != container_id
            || registration.operation_id != operation_id
            || files.policy.get("resource_id").and_then(Value::as_str)
                != Some(registration.resource_id.to_string().as_str())
            || files.policy.get("generation").and_then(Value::as_str)
                != Some(registration.generation.to_string().as_str())
            || files.policy.get("contract_version").and_then(Value::as_u64)
                != Some(registration.contract_version.into())
            || !hash(&registration.container_id)
        {
            return Err(held());
        }
        Ok(registration)
    }

    /// Creates but never starts the process. Fleet must persist its DB binding before start.
    pub async fn prepare(
        &self,
        files: &ContainerLaunchFiles,
        process: &ContainerProcess,
        operation_id: Uuid,
        creation_compose: &std::path::Path,
        creation_journal: &std::path::Path,
    ) -> Result<ContainerPreparation, AppError> {
        if operation_id.is_nil()
            || !creation_compose.is_absolute()
            || !creation_journal.is_absolute()
            || creation_compose.parent() != files.compose.parent()
            || creation_journal.parent() != files.compose.parent()
        {
            return Err(held());
        }
        let (status, value) = self
            .call(
                files,
                "prepare",
                json!({
                    "process":process, "operation_id":operation_id,
                    "creation_compose":creation_compose, "creation_journal":creation_journal,
                }),
            )
            .await?;
        if status != 0 {
            return Err(held());
        }
        let prepared: ContainerPreparation = serde_json::from_value(value).map_err(|_| held())?;
        validate_preparation(
            &prepared,
            &files.policy,
            operation_id,
            files.mount_mapping.as_ref(),
        )?;
        Ok(prepared)
    }

    pub async fn observe(
        &self,
        files: &ContainerLaunchFiles,
        original: &ContainerRegistration,
    ) -> Result<ContainerReceipt, AppError> {
        self.receipt(files, original, "observe").await
    }

    pub async fn endpoint(
        &self,
        files: &ContainerLaunchFiles,
        original: &ContainerRegistration,
    ) -> Result<std::net::Ipv4Addr, AppError> {
        validate_registration(original)?;
        validate_mapping_registration(&files.policy, files.mount_mapping.as_ref(), original)?;
        let (status, value) = self
            .call(files, "endpoint", json!({"registration": original}))
            .await?;
        let endpoint: Endpoint = serde_json::from_value(value).map_err(|_| held())?;
        validate_receipt(&endpoint.receipt, original, status, "endpoint")?;
        if !matches!(original.contract_version, 2 | 3)
            || endpoint.receipt.state != ContainerReceiptState::Observed
            || endpoint.receipt.observation != ContainerObservation::Running
            || !endpoint.host.is_private()
            || endpoint.host.is_loopback()
            || endpoint.host.is_link_local()
            || endpoint.host.is_unspecified()
            || endpoint.host.is_multicast()
        {
            return Err(held());
        }
        Ok(endpoint.host)
    }

    pub async fn attach_controller(
        &self,
        files: &ContainerLaunchFiles,
        original: &ContainerRegistration,
        controller: &shared::config::BridgeControllerConfig,
        journal: &std::path::Path,
    ) -> Result<(), AppError> {
        validate_registration(original)?;
        if !hash(&controller.container_id)
            || controller.container_id == original.container_id
            || !controller
                .image_id
                .strip_prefix("sha256:")
                .is_some_and(hash)
            || !matches!(
                controller.service.as_str(),
                "fleet-backend" | "fleet-control-backend"
            )
            || !matches!(original.contract_version, 2 | 3)
            || !journal.is_absolute()
            || journal.parent() != files.journal.parent()
            || journal == files.journal
            || journal == files.compose
            || journal == files.stop_journal
        {
            return Err(held());
        }
        validate_mapping_registration(&files.policy, files.mount_mapping.as_ref(), original)?;
        if files.mount_mapping.as_ref().is_some_and(|mapping| {
            mapping.controller.container_id != controller.container_id
                || mapping.controller.image_id != controller.image_id
                || mapping.controller.service != controller.service
        }) {
            return Err(held());
        }
        let (status, value) = self
            .call(
                files,
                "attach_controller",
                json!({
                    "registration":original, "controller":controller, "attachment_journal":journal,
                }),
            )
            .await?;
        let attachment: ControllerAttachment = serde_json::from_value(value).map_err(|_| held())?;
        if status != 0
            || attachment.state != "attached"
            || attachment.registration_sha256 != canonical_hash(original)?
            || attachment.controller_id != controller.container_id
            || !hash(&attachment.controller_sha256)
            || files.policy["network"]["id"].as_str() != Some(attachment.network_id.as_str())
        {
            return Err(held());
        }
        Ok(())
    }

    /// Private bounded tail, not durable ingestion. Caller must redact before any storage or API.
    pub async fn log_tail(
        &self,
        files: &ContainerLaunchFiles,
        original: &ContainerRegistration,
        tail: u16,
    ) -> Result<ContainerLogTail, AppError> {
        if !(1..=200).contains(&tail) {
            return Err(held());
        }
        validate_registration(original)?;
        validate_mapping_registration(&files.policy, files.mount_mapping.as_ref(), original)?;
        let (status, value) = self
            .call(files, "logs", json!({"registration":original,"tail":tail}))
            .await?;
        decode_logs(status, value, original)
    }

    /// Caller must commit the registration in Fleet's authoritative launch journal first.
    pub async fn start(
        &self,
        files: &ContainerLaunchFiles,
        original: &ContainerRegistration,
    ) -> Result<ContainerReceipt, AppError> {
        self.receipt(files, original, "start").await
    }

    async fn receipt(
        &self,
        files: &ContainerLaunchFiles,
        original: &ContainerRegistration,
        action: &str,
    ) -> Result<ContainerReceipt, AppError> {
        validate_registration(original)?;
        validate_mapping_registration(&files.policy, files.mount_mapping.as_ref(), original)?;
        let (status, value) = self
            .call(files, action, json!({"registration":original}))
            .await?;
        let receipt: ContainerReceipt = serde_json::from_value(value).map_err(|_| held())?;
        validate_receipt(&receipt, original, status, action)?;
        Ok(receipt)
    }

    pub async fn stop(
        &self,
        files: &ContainerLaunchFiles,
        original: &ContainerRegistration,
        operation_id: Uuid,
    ) -> Result<ContainerStopReceipt, AppError> {
        let observed = self.observe(files, original).await?;
        let snapshot_sha256 = canonical_hash(observed.snapshot.as_ref().ok_or_else(held)?)?;
        let (status, value) = self
            .call(
                files,
                "stop",
                json!({"registration":original,
            "operation_id":operation_id,"stop_journal":files.stop_journal}),
            )
            .await?;
        let receipt: ContainerStopReceipt = serde_json::from_value(value).map_err(|_| held())?;
        if receipt.contract_version != original.contract_version
            || receipt.operation_id != operation_id
            || receipt.container_id != original.container_id
            || receipt.resource_id != original.resource_id
            || receipt.generation != original.generation
            || receipt.snapshot_sha256 != snapshot_sha256
            || status != 0
            || receipt.state != ContainerReceiptState::Observed
            || receipt.observation != ContainerObservation::NamespaceExited
        {
            return Err(held());
        }
        Ok(receipt)
    }
}

async fn bounded(reader: impl AsyncRead + Unpin) -> Result<Vec<u8>, AppError> {
    let mut output = Vec::new();
    reader
        .take((LIMIT + 1) as u64)
        .read_to_end(&mut output)
        .await
        .map_err(|_| held())?;
    if output.len() > LIMIT {
        return Err(held());
    }
    Ok(output)
}

fn validate_receipt(
    receipt: &ContainerReceipt,
    original: &ContainerRegistration,
    status: i32,
    action: &str,
) -> Result<(), AppError> {
    if receipt.contract_version != original.contract_version
        || receipt.operation_id != original.operation_id
        || receipt.container_id != original.container_id
        || receipt.resource_id != original.resource_id
        || receipt.generation != original.generation
        || receipt.registration_sha256 != canonical_hash(original)?
    {
        return Err(held());
    }
    let valid = match (&receipt.state, receipt.observation, &receipt.snapshot) {
        (ContainerReceiptState::Held, ContainerObservation::Unavailable, None) => status == 2,
        (ContainerReceiptState::Registered, ContainerObservation::NeverStarted, None) => {
            status == 0 && action == "observe"
        }
        (
            ContainerReceiptState::Observed,
            ContainerObservation::Running | ContainerObservation::NamespaceExited,
            Some(snapshot),
        ) => {
            status == 0
                && snapshot.contract_version == original.contract_version
                && snapshot.container_id == original.container_id
                && snapshot.engine == original.engine
                && snapshot.policy_sha256 == original.policy_sha256
                && snapshot.inventory_sha256 == original.running_inventory_sha256
                && snapshot.network_sha256 == original.network_sha256
                && snapshot.init_pid > 0
                && !snapshot.started_at.is_empty()
                && !snapshot.started_at.starts_with("0001-")
        }
        _ => false,
    };
    if !valid {
        return Err(held());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_receipt_hash_matches_python_sorted_nested_json() {
        assert_eq!(
            canonical_hash(&json!({"z":{"y":2,"a":1},"a":0})).unwrap(),
            "83881ff50a9a61481c4ed5e19aa8d7610ccfa1705bf25a25c1a00aa713949461"
        );
    }

    #[test]
    fn preparation_accepts_only_original_policy_with_allocated_bridge_id() {
        let mut registration = original();
        let input = json!({"contract_version":2,"resource_id":registration.resource_id,
            "generation":registration.generation,"mounts":[{"source":"/owned/config"}],
            "network":{"id":"0".repeat(64),"name":"original-bridge","internal":true}});
        let mut policy = input.clone();
        policy["network"]["id"] = json!("1".repeat(64));
        registration.policy_sha256 = canonical_hash(&policy).unwrap();
        let original = ContainerPreparation {
            state: "prepared".into(),
            policy,
            registration,
        };
        validate_preparation(&original, &input, original.registration.operation_id, None).unwrap();
        for field in [
            "mount",
            "bridge",
            "generation",
            "state",
            "hash",
            "operation",
        ] {
            let mut changed: ContainerPreparation = serde_json::from_value(json!({
                "state":original.state,"policy":original.policy,"registration":original.registration})).unwrap();
            match field {
                "mount" => changed.policy["mounts"][0]["source"] = json!("/other-agent/config"),
                "bridge" => changed.policy["network"]["internal"] = json!(false),
                "generation" => changed.registration.generation = Uuid::new_v4(),
                "state" => changed.state = "held".into(),
                "hash" => changed.registration.policy_sha256 = "0".repeat(64),
                _ => changed.registration.operation_id = Uuid::new_v4(),
            }
            assert!(
                validate_preparation(&changed, &input, original.registration.operation_id, None)
                    .is_err(),
                "{field}"
            );
        }
    }

    fn original() -> ContainerRegistration {
        ContainerRegistration {
            contract_version: 2,
            operation_id: Uuid::new_v4(),
            container_id: "a".repeat(64),
            resource_id: Uuid::new_v4(),
            generation: Uuid::new_v4(),
            engine: EngineIdentity {
                id: "host".into(),
                kernel_version: "kernel".into(),
                server_version: "29".into(),
            },
            policy_sha256: "b".repeat(64),
            inventory_sha256: "c".repeat(64),
            running_inventory_sha256: "d".repeat(64),
            compose_sha256: "e".repeat(64),
            network_sha256: Some("f".repeat(64)),
            mount_mapping_sha256: None,
        }
    }

    fn mapping_fixture() -> (Value, ContainerMountMapping) {
        let local = json!({"contract_version":2,"project":"sdlc1","service":"agent1-runtime",
            "resource_id":Uuid::new_v4(),"generation":Uuid::new_v4(),
            "image_id":format!("sha256:{}","b".repeat(64)),"task":"test","purpose":"gate",
            "network":{"id":"0".repeat(64),"name":"sdlc1-agent1","internal":true},
            "mounts":(["runtime","config","workspace","logs"].map(|area| json!({
                "type":"bind","source":format!("/local/agents/agent1/{area}"),
                "destination":format!("/{area}"),"read_only":area=="runtime"})))});
        let mapping = serde_json::from_value(json!({"state":"resolved",
            "controller":{"container_id":"a".repeat(64),"image_id":format!("sha256:{}","c".repeat(64)),"service":"fleet-backend"},
            "snapshot":{"container_id":"a".repeat(64),"started_at":"2026-10-06T12:00:00Z","init_pid":999,"inventory_sha256":"d".repeat(64)},
            "engine":{"ID":"host","KernelVersion":"kernel","ServerVersion":"29"},
            "local_root":"/local/agents","volume_name":"sdlc1_fleet_agents","volume_sha256":"e".repeat(64),
            "mounts":(["runtime","config","workspace","logs"].map(|area| json!({
                "type":"bind","source":format!("/daemon/volumes/own/_data/agent1/{area}"),
                "destination":format!("/{area}"),"read_only":area=="runtime"}))),
            "input_policy_sha256":canonical_hash(&local).unwrap()})).unwrap();
        (local, mapping)
    }

    #[test]
    fn mapping_builds_volume_subpaths_and_only_bridge_allocation_may_change() {
        let (local, mapping) = mapping_fixture();
        let mut policy = mapped_policy(&local, &mapping).unwrap();
        assert_eq!(
            policy["mounts"][1],
            json!({"type":"volume","source":"sdlc1_fleet_agents",
            "subpath":"agent1/config","destination":"/config","read_only":false})
        );
        policy["network"]["id"] = json!("f".repeat(64));
        assert_eq!(local_mapping_policy(&policy, &mapping).unwrap(), local);
        for field in [
            "sibling",
            "volume",
            "propagation",
            "read_only",
            "root",
            "recipe",
            "version",
        ] {
            let mut changed = policy.clone();
            match field {
                "sibling" => changed["mounts"][1]["subpath"] = json!("agent2/config"),
                "volume" => changed["mounts"][1]["source"] = json!("sdlc2_fleet_agents"),
                "propagation" => changed["mounts"][0]["propagation"] = json!("rslave"),
                "read_only" => changed["mounts"][0]["read_only"] = json!(false),
                "root" => changed["mounts"][1]["subpath"] = json!("."),
                "recipe" => changed["image_id"] = json!(format!("sha256:{}", "1".repeat(64))),
                _ => changed["contract_version"] = json!(2),
            }
            assert!(local_mapping_policy(&changed, &mapping).is_err(), "{field}");
        }
    }

    #[test]
    fn mapping_rejects_malformed_or_unrelated_original_proof() {
        let (local, original) = mapping_fixture();
        let controller = shared::config::BridgeControllerConfig {
            container_id: original.controller.container_id.clone(),
            image_id: original.controller.image_id.clone(),
            service: "fleet-backend".into(),
        };
        validate_mount_mapping(&original, &local, &controller, "/local/agents").unwrap();
        for field in [
            "controller",
            "image",
            "snapshot",
            "pid",
            "started",
            "engine",
            "hash",
            "root",
            "sibling",
            "alias",
            "order",
            "extra",
        ] {
            let mut changed = original.clone();
            match field {
                "controller" => changed.controller.container_id = "f".repeat(64),
                "image" => changed.controller.image_id = format!("sha256:{}", "f".repeat(64)),
                "snapshot" => changed.snapshot.container_id = "f".repeat(64),
                "pid" => changed.snapshot.init_pid = 0,
                "started" => changed.snapshot.started_at = "0001-01-01".into(),
                "engine" => changed.engine.id.clear(),
                "hash" => changed.input_policy_sha256 = "0".repeat(64),
                "root" => changed.local_root = "/local/agents/..".into(),
                "sibling" => {
                    changed.mounts[1].source = "/daemon/volumes/own/_data/agent2/config".into()
                }
                "alias" => changed.mounts[1].source = "/foreign/_data/agent1/config".into(),
                "order" => changed.mounts.swap(0, 1),
                _ => changed.mounts.push(changed.mounts[0].clone()),
            }
            assert!(
                validate_mount_mapping(&changed, &local, &controller, "/local/agents").is_err(),
                "{field}"
            );
        }
        let mut dto = serde_json::to_value(original).unwrap();
        dto["snapshot"]["adopt_pid"] = json!(999);
        assert!(serde_json::from_value::<ContainerMountMapping>(dto).is_err());
        for path in [
            "/",
            "//local/agents",
            "/local//agents",
            "/local/./agents",
            "/local/agents/",
            "/local/../agents",
            "relative",
            "/local\\agents",
        ] {
            assert!(!linux_directory(path), "{path}");
        }
    }

    #[test]
    fn mapped_preparation_requires_original_mapping_digest_and_engine() {
        let (local, mapping) = mapping_fixture();
        let input = mapped_policy(&local, &mapping).unwrap();
        let mut policy = input.clone();
        policy["network"]["id"] = json!("1".repeat(64));
        let mut registration = original();
        registration.contract_version = 3;
        registration.resource_id = serde_json::from_value(policy["resource_id"].clone()).unwrap();
        registration.generation = serde_json::from_value(policy["generation"].clone()).unwrap();
        registration.policy_sha256 = canonical_hash(&policy).unwrap();
        registration.mount_mapping_sha256 = Some(canonical_hash(&mapping).unwrap());
        let mut prepared = ContainerPreparation {
            state: "prepared".into(),
            policy,
            registration,
        };
        validate_preparation(
            &prepared,
            &input,
            prepared.registration.operation_id,
            Some(&mapping),
        )
        .unwrap();
        assert!(
            validate_preparation(&prepared, &input, prepared.registration.operation_id, None)
                .is_err()
        );
        prepared.registration.engine.id = "foreign".into();
        assert!(
            validate_preparation(
                &prepared,
                &input,
                prepared.registration.operation_id,
                Some(&mapping)
            )
            .is_err()
        );
        prepared.registration.engine = mapping.engine.clone();
        prepared.registration.mount_mapping_sha256 = Some("0".repeat(64));
        assert!(
            validate_preparation(
                &prepared,
                &input,
                prepared.registration.operation_id,
                Some(&mapping)
            )
            .is_err()
        );
        prepared.registration.mount_mapping_sha256 = None;
        assert!(validate_registration(&prepared.registration).is_err());
        prepared.registration.mount_mapping_sha256 = Some(canonical_hash(&mapping).unwrap());
        prepared.registration.contract_version = 2;
        assert!(validate_registration(&prepared.registration).is_err());
    }

    fn receipt(original: &ContainerRegistration) -> ContainerReceipt {
        ContainerReceipt {
            contract_version: 2,
            operation_id: original.operation_id,
            container_id: original.container_id.clone(),
            resource_id: original.resource_id,
            generation: original.generation,
            registration_sha256: canonical_hash(original).unwrap(),
            state: ContainerReceiptState::Observed,
            observation: ContainerObservation::Running,
            snapshot: Some(ContainerSnapshot {
                contract_version: 2,
                container_id: original.container_id.clone(),
                engine: original.engine.clone(),
                policy_sha256: original.policy_sha256.clone(),
                inventory_sha256: original.running_inventory_sha256.clone(),
                started_at: "2026-10-06T12:00:00.123456789Z".into(),
                init_pid: 123,
                network_sha256: original.network_sha256.clone(),
            }),
        }
    }

    #[test]
    fn original_ack_and_exit_are_not_interchangeable_with_held_or_foreign_receipts() {
        let original = original();
        let mut value = receipt(&original);
        validate_receipt(&value, &original, 0, "start").unwrap();
        value.observation = ContainerObservation::NamespaceExited;
        validate_receipt(&value, &original, 0, "observe").unwrap();
        value.generation = Uuid::new_v4();
        assert!(validate_receipt(&value, &original, 0, "observe").is_err());
        value.generation = original.generation;
        value.snapshot.as_mut().unwrap().inventory_sha256 = original.inventory_sha256.clone();
        assert!(validate_receipt(&value, &original, 0, "start").is_err());
        value.snapshot = None;
        assert!(validate_receipt(&value, &original, 0, "start").is_err());
        value.state = ContainerReceiptState::Held;
        value.observation = ContainerObservation::Unavailable;
        validate_receipt(&value, &original, 2, "start").unwrap();
        assert!(validate_receipt(&value, &original, 0, "start").is_err());
    }

    fn log_envelope(original: &ContainerRegistration) -> Value {
        json!({"receipt":{"contract_version":original.contract_version,
            "operation_id":original.operation_id,"container_id":original.container_id,
            "resource_id":original.resource_id,"generation":original.generation,
            "registration_sha256":canonical_hash(original).unwrap(),
            "state":"observed","observation":"running","snapshot":receipt(original).snapshot},
            "stdout_base64":STANDARD.encode(b"output\xff"),"stderr_base64":STANDARD.encode(b"error\n")})
    }

    #[test]
    fn log_tail_preserves_binary_streams_and_exact_original_receipt() {
        let original = original();
        for state in ["running", "namespace_exited"] {
            let mut value = log_envelope(&original);
            value["receipt"]["observation"] = json!(state);
            let logs = decode_logs(0, value, &original).unwrap();
            assert_eq!(logs.stdout, b"output\xff");
            assert_eq!(logs.stderr, b"error\n");
            assert_eq!(logs.receipt.generation, original.generation);
        }
    }

    #[test]
    fn log_tail_rejects_unknown_foreign_malformed_and_oversized_output() {
        let original = original();
        for field in [
            "unknown",
            "foreign",
            "missing",
            "extra",
            "base64",
            "oversized",
        ] {
            let mut value = log_envelope(&original);
            match field {
                "unknown" => {
                    value["receipt"]["state"] = json!("held");
                    value["receipt"]["observation"] = json!("unavailable");
                    value["receipt"]["snapshot"] = Value::Null;
                }
                "foreign" => value["receipt"]["generation"] = json!(Uuid::new_v4()),
                "missing" => {
                    value.as_object_mut().unwrap().remove("stderr_base64");
                }
                "extra" => value["credentials"] = json!("do-not-adopt"),
                "base64" => value["stderr_base64"] = json!("%%%"),
                _ => value["stdout_base64"] = json!(STANDARD.encode(vec![b'x'; 32 * 1024 + 1])),
            }
            assert!(decode_logs(0, value, &original).is_err(), "{field}");
        }
        assert!(decode_logs(2, log_envelope(&original), &original).is_err());
    }

    #[test]
    fn preexec_observation_is_not_a_start_ack() {
        let original = original();
        let mut value = receipt(&original);
        value.state = ContainerReceiptState::Registered;
        value.observation = ContainerObservation::NeverStarted;
        value.snapshot = None;
        validate_receipt(&value, &original, 0, "observe").unwrap();
        assert!(validate_receipt(&value, &original, 0, "start").is_err());
    }

    #[test]
    fn receipt_requires_explicit_nullable_snapshot_and_closed_fields() {
        let original = original();
        let mut value = json!({"contract_version":2,"operation_id":original.operation_id,
            "container_id":original.container_id,"resource_id":original.resource_id,
            "generation":original.generation,"registration_sha256":canonical_hash(&original).unwrap(),
            "state":"held","observation":"unavailable","snapshot":null});
        let receipt: ContainerReceipt = serde_json::from_value(value.clone()).unwrap();
        validate_receipt(&receipt, &original, 2, "start").unwrap();
        value.as_object_mut().unwrap().remove("snapshot");
        assert!(serde_json::from_value::<ContainerReceipt>(value.clone()).is_err());
        value["snapshot"] = Value::Null;
        value["adopt_pid"] = json!(123);
        assert!(serde_json::from_value::<ContainerReceipt>(value).is_err());
    }

    #[test]
    fn registration_requires_all_original_hashes_and_versioned_network() {
        let mut value = original();
        validate_registration(&value).unwrap();
        value.network_sha256 = None;
        assert!(validate_registration(&value).is_err());
        value.contract_version = 1;
        validate_registration(&value).unwrap();
        value.compose_sha256 = "mutable".into();
        assert!(validate_registration(&value).is_err());
    }

    #[test]
    fn source_configuration_rejects_implicit_context_and_unpinned_sources() {
        let source = ControlSource {
            root: PathBuf::from("relative"),
            sha256: ["a".repeat(64), "b".repeat(64), "c".repeat(64)],
        };
        assert!(ContainerControl::new("python".into(), source, "desktop-linux".into()).is_err());
        let source = ControlSource {
            root: std::env::current_dir().unwrap(),
            sha256: ["a".repeat(64), "b".repeat(64), "c".repeat(64)],
        };
        for context in ["", "--host=foreign", "a b", "/remote"] {
            assert!(
                ContainerControl::new("python".into(), source.clone(), context.into()).is_err()
            );
        }
        ContainerControl::new("python".into(), source, "desktop-linux".into()).unwrap();
    }

    #[tokio::test]
    async fn native_output_is_bounded_and_cannot_become_a_receipt() {
        assert!(bounded(&vec![b'x'; LIMIT + 1][..]).await.is_err());
        assert_eq!(bounded(&b"receipt"[..]).await.unwrap(), b"receipt");
    }

    #[cfg(unix)]
    async fn source_fixture() -> (ContainerControl, ContainerLaunchFiles) {
        let root = std::env::temp_dir().join(format!("fleet-control-source-{}", Uuid::new_v4()));
        let scripts = root.join("scripts");
        tokio::fs::create_dir_all(&scripts).await.unwrap();
        let contents = [
            "VALUE='verified'\n",
            "from scripts.runtime_boundary import VALUE\n",
            "import json,sys\nfrom scripts.runtime_bootstrap import VALUE\nr=json.load(sys.stdin.buffer)\nprint(json.dumps({'protocol_version':1,'action':r['action'],'result':VALUE}))\n",
        ];
        let names = ["runtime_boundary", "runtime_bootstrap", "runtime_control"];
        let mut hashes = Vec::new();
        for (name, content) in names.iter().zip(contents) {
            tokio::fs::write(scripts.join(format!("{name}.py")), content)
                .await
                .unwrap();
            hashes.push(hex::encode(Sha256::digest(content.as_bytes())));
        }
        let control = ContainerControl::new(
            "python3".into(),
            ControlSource {
                root: root.clone(),
                sha256: hashes.try_into().unwrap(),
            },
            "desktop-linux".into(),
        )
        .unwrap();
        let files = ContainerLaunchFiles {
            policy: json!({}),
            compose: root.join("compose.json"),
            journal: root.join("journal.json"),
            stop_journal: root.join("stop.json"),
            mount_mapping: None,
            mapping_file: None,
        };
        (control, files)
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn pinned_sources_ignore_valid_poisoned_bytecode_and_package_initializers() {
        let (control, files) = source_fixture().await;
        let scripts = control.source.root.join("scripts");
        tokio::fs::write(
            scripts.join("__init__.py"),
            "raise RuntimeError('untrusted package')",
        )
        .await
        .unwrap();
        let poison = Command::new("python3")
            .args([
                "-I",
                "-c",
                r#"import os,py_compile,sys
from pathlib import Path
p=Path(sys.argv[1])
original=p.read_bytes()
p.write_bytes(original.replace(b'verified',b'poisoned'))
py_compile.compile(str(p),doraise=True)
stat=p.stat()
p.write_bytes(original)
os.utime(p,ns=(stat.st_atime_ns,stat.st_mtime_ns))
"#,
            ])
            .arg(scripts.join("runtime_boundary.py"))
            .status()
            .await
            .unwrap();
        let result = control.call(&files, "observe", json!({})).await;
        tokio::fs::remove_dir_all(&control.source.root)
            .await
            .unwrap();
        assert!(poison.success());
        assert_eq!(result.unwrap(), (0, json!("verified")));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn checkout_modules_outside_pinned_sources_cannot_be_imported() {
        let (mut control, files) = source_fixture().await;
        let scripts = control.source.root.join("scripts");
        let changed = "from scripts.unpinned import VALUE\n";
        tokio::fs::write(scripts.join("runtime_bootstrap.py"), changed)
            .await
            .unwrap();
        tokio::fs::write(scripts.join("unpinned.py"), "VALUE='untrusted'\n")
            .await
            .unwrap();
        control.source.sha256[1] = hex::encode(Sha256::digest(changed.as_bytes()));
        let result = control.call(&files, "observe", json!({})).await;
        tokio::fs::remove_dir_all(&control.source.root)
            .await
            .unwrap();
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn source_hash_drift_and_parent_symlinks_are_rejected_before_execution() {
        let (control, files) = source_fixture().await;
        let scripts = control.source.root.join("scripts");
        let source = scripts.join("runtime_boundary.py");
        let original = tokio::fs::read(&source).await.unwrap();
        tokio::fs::write(&source, "VALUE='changed'\n")
            .await
            .unwrap();
        let drift = control.call(&files, "observe", json!({})).await;
        tokio::fs::write(source, original).await.unwrap();
        let alias = control.source.root.join("linked-root");
        std::os::unix::fs::symlink(&control.source.root, &alias).unwrap();
        let mut aliased = control.clone();
        aliased.source.root = alias.clone();
        let linked_root = aliased.call(&files, "observe", json!({})).await;
        tokio::fs::remove_file(alias).await.unwrap();
        let moved = control.source.root.join("moved-scripts");
        tokio::fs::rename(&scripts, &moved).await.unwrap();
        std::os::unix::fs::symlink(&moved, &scripts).unwrap();
        let linked = control.call(&files, "observe", json!({})).await;
        tokio::fs::remove_file(scripts).await.unwrap();
        tokio::fs::remove_dir_all(&control.source.root)
            .await
            .unwrap();
        assert!(drift.is_err());
        assert!(linked_root.is_err());
        assert!(linked.is_err());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn even_hash_pinned_sources_cannot_exceed_the_capture_limit() {
        let (mut control, files) = source_fixture().await;
        let mut bytes = b"VALUE='verified'\n".to_vec();
        bytes.resize(SOURCE_LIMIT + 1, b' ');
        control.source.sha256[0] = hex::encode(Sha256::digest(&bytes));
        tokio::fs::write(
            control.source.root.join("scripts/runtime_boundary.py"),
            bytes,
        )
        .await
        .unwrap();
        let result = control.call(&files, "observe", json!({})).await;
        tokio::fs::remove_dir_all(&control.source.root)
            .await
            .unwrap();
        assert!(result.is_err());
    }
}
