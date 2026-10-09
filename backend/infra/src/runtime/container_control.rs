//! Private Base subprocess protocol. No Docker command or credential is agent-supplied.
pub use app::container_runtime::{
    ContainerEngineIdentity as EngineIdentity, ContainerRecoveryCommand, ContainerRegistration,
    ControllerSnapshot, MappedContainer,
};
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
const DEADLINE: Duration = Duration::from_secs(60);
// Execute only the exact captured utility bytes, never a mutable checkout import or pyc.
const BOOTSTRAP: &str = r#"import io,json,sys,types
from pathlib import Path
payload=json.load(sys.stdin)
sys.stdin=io.TextIOWrapper(io.BytesIO(json.dumps(payload['request'],ensure_ascii=False,separators=(',',':')).encode('utf-8')),encoding='utf-8')
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
    exec(compile(source,module.__file__,'exec'),module.__dict__)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerObservation {
    NeverStarted,
    Running,
    NamespaceExited,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerReceiptState {
    Registered,
    Observed,
    Held,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Clone)]
pub struct ContainerLaunchFiles {
    pub policy: Value,
    pub compose: PathBuf,
    pub journal: PathBuf,
    pub stop_journal: PathBuf,
    pub mapped: Option<MappedContainer>,
    pub recovery: Option<ContainerRecoveryCommand>,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RestartWitness {
    contract_version: u8,
    state: String,
    original_mapping_sha256: String,
    registration_sha256: String,
    original_controller_snapshot: ControllerSnapshot,
    pub current_controller_snapshot: ControllerSnapshot,
    pub receipt: ContainerReceipt,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryAck {
    contract_version: u8,
    state: String,
    request_sha256: String,
    recovery: ContainerRecoveryCommand,
    witness: RestartWitness,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Heartbeat {
    state: String,
    recovery_id: Uuid,
    lease_version: i64,
    lease_expires_at: String,
}

fn validate_restart(
    w: &RestartWitness,
    files: &ContainerLaunchFiles,
    r: &ContainerRegistration,
    status: i32,
) -> Result<(), AppError> {
    validate_registration(r)?;
    let m = &files.mapped.as_ref().ok_or_else(held)?.mapping;
    let current = &w.current_controller_snapshot;
    if status != 0
        || r.contract_version != 3
        || m.engine != r.engine
        || w.contract_version != 1
        || w.state != "controller_restart_observed"
        || w.original_mapping_sha256 != canonical_hash(m)?
        || r.mount_mapping_sha256.as_ref() != Some(&w.original_mapping_sha256)
        || w.registration_sha256 != canonical_hash(r)?
        || w.original_controller_snapshot != m.snapshot
        || current.container_id != m.controller.container_id
        || current.inventory_sha256 != m.snapshot.inventory_sha256
        || current.started_at == m.snapshot.started_at
        || current.init_pid == 0
        || current.started_at.starts_with("0001-")
        || chrono::DateTime::parse_from_rfc3339(&current.started_at).is_err()
        || w.receipt.state != ContainerReceiptState::Observed
        || !matches!(
            w.receipt.observation,
            ContainerObservation::Running | ContainerObservation::NamespaceExited
        )
    {
        return Err(held());
    }
    validate_receipt(&w.receipt, r, 0, "observe")
}

fn validate_attachment(
    value: &Value,
    files: &ContainerLaunchFiles,
    original: &ContainerRegistration,
) -> Result<(), AppError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Attachment {
        state: String,
        registration_sha256: String,
        controller_id: String,
        controller_sha256: String,
        network_id: String,
    }
    let a: Attachment = serde_json::from_value(value.clone()).map_err(|_| held())?;
    let m = &files.mapped.as_ref().ok_or_else(held)?.mapping;
    if a.state != "attached"
        || a.registration_sha256 != canonical_hash(original)?
        || a.controller_id != m.controller.container_id
        || a.controller_sha256 != canonical_hash(&m.snapshot)?
        || files.policy["network"]["id"] != a.network_id
    {
        return Err(held());
    }
    Ok(())
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

fn verified_source(bytes: Vec<u8>, expected: &str) -> Result<String, AppError> {
    if bytes.len() > 1024 * 1024 || hex::encode(Sha256::digest(&bytes)) != expected {
        return Err(held());
    }
    String::from_utf8(bytes).map_err(|_| held())
}

pub(crate) fn canonical_hash(value: &impl Serialize) -> Result<String, AppError> {
    use std::fmt::Write;

    let mut sorted = serde_json::to_value(value).map_err(|_| held())?;
    sorted.sort_all_objects();
    let json = serde_json::to_string(&sorted).map_err(|_| held())?;
    // Match Base json.dumps(..., ensure_ascii=True), including DEL and surrogate pairs.
    let mut ascii = String::with_capacity(json.len());
    for c in json.chars() {
        if c < '\u{7f}' {
            ascii.push(c);
        } else {
            for unit in c.encode_utf16(&mut [0; 2]) {
                write!(ascii, "\\u{unit:04x}").map_err(|_| held())?;
            }
        }
    }
    Ok(hex::encode(Sha256::digest(ascii.as_bytes())))
}

pub(crate) fn launch_hash(
    launch: &app::container_runtime::ContainerLaunch,
) -> Result<String, AppError> {
    let mut value = serde_json::to_value(launch).map_err(|_| held())?;
    value.as_object_mut().ok_or_else(held)?.remove("state");
    canonical_hash(&value)
}

pub(crate) fn validate_registration(value: &ContainerRegistration) -> Result<(), AppError> {
    if !matches!(value.contract_version, 1 | 2 | 3)
        || value.operation_id.is_nil()
        || value.resource_id.is_nil()
        || value.generation.is_nil()
        || (value.contract_version >= 2) != value.network_sha256.is_some()
        || (value.contract_version == 3) != value.mount_mapping_sha256.is_some()
        || value
            .mount_mapping_sha256
            .as_ref()
            .is_some_and(|v| !hash(v))
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

impl ContainerControl {
    pub(super) async fn resolve_mounts(
        &self,
        files: &ContainerLaunchFiles,
        controller: &shared::config::MappingControllerConfig,
        local_root: &str,
    ) -> Result<app::container_runtime::ContainerMapping, AppError> {
        let (status, value) = self
            .call(
                files,
                "resolve_mounts",
                json!({
                    "controller":controller,"local_root":local_root
                }),
            )
            .await?;
        if status != 0 || files.mapped.is_some() || files.recovery.is_some() {
            return Err(held());
        }
        serde_json::from_value(value).map_err(|_| held())
    }

    pub(super) async fn preparation(
        &self,
        files: &ContainerLaunchFiles,
        process: &Value,
        operation_id: Uuid,
        creation_compose: &std::path::Path,
        creation_journal: &std::path::Path,
        first_delivery: bool,
    ) -> Result<PreparationReceipt, AppError> {
        let action = if first_delivery {
            "prepare"
        } else {
            "reconcile_preparation"
        };
        let (status, value) = self
            .call(
                files,
                action,
                json!({
                    "process":process,"operation_id":operation_id,
                    "creation_compose":creation_compose,"creation_journal":creation_journal
                }),
            )
            .await?;
        validate_preparation_receipt(value, status, files, operation_id)
    }

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
        let mut sources = Vec::new();
        for (name, expected) in [
            "runtime_boundary.py",
            "runtime_bootstrap.py",
            "runtime_control.py",
        ]
        .iter()
        .zip(&self.source.sha256)
        {
            let path = self.source.root.join("scripts").join(name);
            let meta = tokio::fs::symlink_metadata(&path)
                .await
                .map_err(|_| held())?;
            if !meta.is_file() || meta.len() > 1024 * 1024 {
                return Err(held());
            }
            let mut bytes = Vec::new();
            tokio::fs::File::open(&path)
                .await
                .map_err(|_| held())?
                .take(1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .await
                .map_err(|_| held())?;
            sources.push(verified_source(bytes, expected)?);
        }
        let version = if files.recovery.is_some() {
            3
        } else if files.mapped.is_some() {
            2
        } else {
            1
        };
        let mut request = json!({"protocol_version":version, "action":action, "context":self.context,
            "policy":files.policy, "compose":files.compose, "journal":files.journal});
        if let Some(mapped) = &files.mapped {
            request["mount_mapping"] = json!(mapped.mapping);
            request["mapping_file"] = json!(mapped.mapping_file);
        }
        if let Some(recovery) = &files.recovery {
            request["recovery"] = json!(recovery);
            request["recovery_journal"] =
                json!(files.mapped.as_ref().ok_or_else(held)?.recovery_journal);
        }
        if !files.compose.is_absolute()
            || !files.journal.is_absolute()
            || !files.stop_journal.is_absolute()
        {
            return Err(AppError::validation(
                "Docker launch storage must be absolute",
            ));
        }
        request
            .as_object_mut()
            .ok_or_else(held)?
            .extend(extra.as_object().ok_or_else(held)?.clone());
        if serde_json::to_vec(&request).map_err(|_| held())?.len() > LIMIT {
            return Err(held());
        }
        let payload = serde_json::to_vec(&json!({"request":request, "sources":sources}))
            .map_err(|_| held())?;
        if payload.len() > 3 * 1024 * 1024 + LIMIT {
            return Err(held());
        }
        let mut command = Command::new(&self.python);
        command
            .args(["-I", "-c", BOOTSTRAP])
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
        if envelope.protocol_version != version || envelope.action != action {
            return Err(held());
        }
        Ok((result.0, envelope.result))
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
        let mut extra = json!({"registration":original});
        let action = if files.recovery.is_none() && files.mapped.is_some() {
            let mapped = files.mapped.as_ref().ok_or_else(held)?;
            extra["controller"] = json!(mapped.mapping.controller);
            extra["attachment_journal"] = json!(mapped.attachment_journal);
            "endpoint_attached"
        } else {
            "endpoint"
        };
        let (status, mut value) = self.call(files, action, extra).await?;
        if action == "endpoint_attached" {
            let attachment = value
                .as_object_mut()
                .ok_or_else(held)?
                .remove("attachment")
                .ok_or_else(held)?;
            validate_attachment(&attachment, files, original)?;
        }
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

    /// Caller must commit the registration in Fleet's authoritative launch journal first.
    pub async fn start(
        &self,
        files: &ContainerLaunchFiles,
        original: &ContainerRegistration,
    ) -> Result<ContainerReceipt, AppError> {
        self.receipt(files, original, "start").await
    }

    pub async fn attach(
        &self,
        files: &ContainerLaunchFiles,
        original: &ContainerRegistration,
    ) -> Result<(), AppError> {
        let mapped = files.mapped.as_ref().ok_or_else(held)?;
        if files.recovery.is_some() {
            return Err(held());
        }
        let (status, value) = self
            .call(
                files,
                "attach_controller",
                json!({"registration":original,
            "controller":mapped.mapping.controller,"attachment_journal":mapped.attachment_journal}),
            )
            .await?;
        if status != 0 {
            return Err(held());
        }
        validate_attachment(&value, files, original)
    }

    pub(super) async fn controller_restart(
        &self,
        files: &ContainerLaunchFiles,
        original: &ContainerRegistration,
    ) -> Result<RestartWitness, AppError> {
        let mut original_files = files.clone();
        original_files.recovery = None;
        let (status, value) = self
            .call(
                &original_files,
                "observe_controller_restart",
                json!({"registration":original}),
            )
            .await?;
        let witness: RestartWitness = serde_json::from_value(value).map_err(|_| held())?;
        validate_restart(&witness, files, original, status)?;
        Ok(witness)
    }

    pub async fn recovery_action(
        &self,
        files: &ContainerLaunchFiles,
        original: &ContainerRegistration,
        action: &str,
        controller_id: Uuid,
    ) -> Result<Value, AppError> {
        if !matches!(
            action,
            "recover_controller" | "read_controller_recovery" | "heartbeat_controller"
        ) {
            return Err(held());
        }
        if action == "heartbeat_controller"
            && files.recovery.as_ref().map(|c| c.request.controller_id) != Some(controller_id)
        {
            return Err(held());
        }
        let (status, value) = self
            .call(files, action, json!({"registration":original}))
            .await?;
        validate_recovery_value(files, original, action, status, &value)?;
        Ok(value)
    }

    async fn receipt(
        &self,
        files: &ContainerLaunchFiles,
        original: &ContainerRegistration,
        action: &str,
    ) -> Result<ContainerReceipt, AppError> {
        validate_registration(original)?;
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PreparationReceipt {
    state: String,
    pub policy: Value,
    pub registration: ContainerRegistration,
}

pub(super) fn validate_preparation_receipt(
    value: Value,
    status: i32,
    files: &ContainerLaunchFiles,
    operation_id: Uuid,
) -> Result<PreparationReceipt, AppError> {
    let receipt: PreparationReceipt = serde_json::from_value(value).map_err(|_| held())?;
    let r = &receipt.registration;
    validate_registration(r)?;
    if !receipt.policy.is_object() || !receipt.policy["network"].is_object() {
        return Err(held());
    }
    let mut unallocated = receipt.policy.clone();
    unallocated["network"]["id"] = json!("0".repeat(64));
    if status != 0
        || receipt.state != "prepared"
        || unallocated != files.policy
        || r.operation_id != operation_id
        || receipt.policy["contract_version"] != r.contract_version
        || receipt.policy["resource_id"] != r.resource_id.to_string()
        || receipt.policy["generation"] != r.generation.to_string()
        || canonical_hash(&receipt.policy)? != r.policy_sha256
        || receipt.policy["network"]["id"]
            .as_str()
            .is_none_or(|s| !hash(s) || s == "0".repeat(64))
        || files.recovery.is_some()
    {
        return Err(held());
    }
    match (&files.mapped, &r.mount_mapping_sha256) {
        (Some(m), Some(h))
            if r.contract_version == 3
                && *h == canonical_hash(&m.mapping)?
                && r.engine == m.mapping.engine => {}
        (None, None) if r.contract_version == 2 => {}
        _ => return Err(held()),
    }
    Ok(receipt)
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

pub(crate) fn validate_recovery_value(
    files: &ContainerLaunchFiles,
    original: &ContainerRegistration,
    action: &str,
    status: i32,
    value: &Value,
) -> Result<(), AppError> {
    validate_registration(original)?;
    let command = files.recovery.as_ref().ok_or_else(held)?;
    let request = &command.request;
    let mapping = &files.mapped.as_ref().ok_or_else(held)?.mapping;
    let deadline =
        chrono::DateTime::parse_from_rfc3339(&command.lease_expires_at).map_err(|_| held())?;
    if original.contract_version != 3
        || request.agent_id != original.resource_id
        || request.launch_id != original.generation
        || [
            request.id,
            request.agent_id,
            request.launch_id,
            request.original_controller_id,
            request.controller_id,
        ]
        .iter()
        .any(Uuid::is_nil)
        || request.original_controller_id == request.controller_id
        || request.agent_pid <= 0
        || !hash(&request.launch_sha256)
        || request.mapping_sha256 != canonical_hash(mapping)?
        || original.mount_mapping_sha256.as_ref() != Some(&request.mapping_sha256)
        || request.registration_sha256 != canonical_hash(original)?
        || command.epoch <= 0
        || command.lease_version <= 0
        || (command.epoch == 1) != request.predecessor_id.is_none()
        || request
            .predecessor_id
            .is_some_and(|id| id.is_nil() || id == request.id)
        || request.controller_snapshot.container_id != mapping.controller.container_id
        || request.controller_snapshot.inventory_sha256 != mapping.snapshot.inventory_sha256
        || request.controller_snapshot.started_at == mapping.snapshot.started_at
        || request.controller_snapshot.init_pid == 0
        || command.lease_expires_at.len() > 96
        || deadline.offset().local_minus_utc() != 0
    {
        return Err(held());
    }
    if status != 0 {
        return Err(held());
    }
    if action == "heartbeat_controller" {
        let ack: Heartbeat = serde_json::from_value(value.clone()).map_err(|_| held())?;
        if ack.state != "controller_heartbeat"
            || ack.recovery_id != command.request.id
            || ack.lease_version != command.lease_version
            || ack.lease_expires_at != command.lease_expires_at
        {
            return Err(held());
        }
    } else if matches!(action, "recover_controller" | "read_controller_recovery") {
        let ack: RecoveryAck = serde_json::from_value(value.clone()).map_err(|_| held())?;
        if ack.contract_version != 1
            || ack.state != "controller_recovered"
            || ack.recovery != *command
            || ack.request_sha256 != canonical_hash(command)?
        {
            return Err(held());
        }
        validate_restart(&ack.witness, files, original, 0)?;
        if ack.witness.current_controller_snapshot != command.request.controller_snapshot
            || ack
                .witness
                .receipt
                .snapshot
                .as_ref()
                .map(|s| s.init_pid as i64)
                != Some(command.request.agent_pid as i64)
        {
            return Err(held());
        }
    } else {
        return Err(held());
    }
    Ok(())
}

pub(crate) fn validate_receipt(
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
                && snapshot.started_at.len() <= 128
                && chrono::DateTime::parse_from_rfc3339(&snapshot.started_at).is_ok()
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
    fn canonical_hash_matches_base_ascii_escaping_for_unicode_and_del() {
        // Independently generated by Base's sort_keys/separators/ensure_ascii recipe.
        let value = json!({"z":{"\\":"\"\\\n\t\u{7f}","\u{1f600}":"/srv/\u{430}\u{433}\u{435}\u{43d}\u{442}\u{44b}"},"\u{e9}":"\u{2028}\u{2029}"});
        assert_eq!(
            canonical_hash(&value).unwrap(),
            "9f8e9d6aa4038a3f5ee775ae03966955cbb132701da1266358b48d5af159b8a6"
        );
    }

    #[test]
    fn canonical_source_bytes_are_verified_without_crlf_normalization() {
        let source = b"# canonical Git blob\nVALUE = 1\n";
        let expected = "fbc22cbff63796d0d0273005eb5b563dd76162feb5ebc0ef57c782fece6111ef";
        assert_eq!(
            verified_source(source.to_vec(), expected)
                .unwrap()
                .as_bytes(),
            source
        );
        assert!(
            verified_source(b"# canonical Git blob\r\nVALUE = 1\r\n".to_vec(), expected).is_err()
        );
        assert!(verified_source(vec![0xff], &hex::encode(Sha256::digest([0xff]))).is_err());
        assert!(verified_source(vec![b'a'; 1024 * 1024 + 1], expected).is_err());
    }

    pub(super) fn original() -> ContainerRegistration {
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

    pub(super) fn receipt(original: &ContainerRegistration) -> ContainerReceipt {
        ContainerReceipt {
            contract_version: original.contract_version,
            operation_id: original.operation_id,
            container_id: original.container_id.clone(),
            resource_id: original.resource_id,
            generation: original.generation,
            registration_sha256: canonical_hash(original).unwrap(),
            state: ContainerReceiptState::Observed,
            observation: ContainerObservation::Running,
            snapshot: Some(ContainerSnapshot {
                contract_version: original.contract_version,
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
}

#[cfg(test)]
mod mapped_tests {
    use super::*;
    use app::container_runtime::*;

    fn fixture() -> (ContainerLaunchFiles, ContainerRegistration, Value) {
        let (_, mapping, _) = super::super::container_mapping::tests::fixture();
        let mut r = tests::original();
        r.contract_version = 3;
        r.container_id = "9".repeat(64);
        r.engine = mapping.engine.clone();
        r.mount_mapping_sha256 = Some(canonical_hash(&mapping).unwrap());
        let mut current = mapping.snapshot.clone();
        current.started_at = "2026-10-09T13:00:00Z".into();
        current.init_pid = 51;
        let command = ContainerRecoveryCommand {
            request: ContainerRecoveryRequest {
                id: Uuid::new_v4(),
                launch_id: r.generation,
                agent_id: r.resource_id,
                original_controller_id: Uuid::new_v4(),
                controller_id: Uuid::new_v4(),
                predecessor_id: None,
                launch_sha256: "1".repeat(64),
                mapping_sha256: canonical_hash(&mapping).unwrap(),
                registration_sha256: canonical_hash(&r).unwrap(),
                controller_snapshot: current.clone(),
                agent_pid: 123,
            },
            epoch: 1,
            lease_version: 1,
            lease_expires_at: "2026-10-09T13:00:30Z".into(),
        };
        let witness = json!({"contract_version":1,"state":"controller_restart_observed","original_mapping_sha256":canonical_hash(&mapping).unwrap(),"registration_sha256":canonical_hash(&r).unwrap(),
            "original_controller_snapshot":mapping.snapshot,"current_controller_snapshot":current,"receipt":tests::receipt(&r)});
        let ack = json!({"contract_version":1,"state":"controller_recovered","request_sha256":canonical_hash(&command).unwrap(),"recovery":command,"witness":witness});
        let files = ContainerLaunchFiles {
            policy: json!({"network":{"id":"e".repeat(64)}}),
            compose: "/private/compose.json".into(),
            journal: "/private/start.sqlite".into(),
            stop_journal: "/private/stop.sqlite".into(),
            mapped: Some(MappedContainer {
                mapping,
                mapping_file: "/private/mapping.json".into(),
                attachment_journal: "/private/attach.sqlite".into(),
                recovery_journal: "/private/recovery.sqlite".into(),
            }),
            recovery: Some(command),
        };
        (files, r, ack)
    }

    #[test]
    fn registration_versions_require_original_mapping_hash_only_for_v3() {
        let (_, mut r, _) = fixture();
        validate_registration(&r).unwrap();
        r.contract_version = 2;
        assert!(validate_registration(&r).is_err());
        r.mount_mapping_sha256 = None;
        validate_registration(&r).unwrap();
        assert!(
            !json!(r)
                .as_object()
                .unwrap()
                .contains_key("mount_mapping_sha256")
        );
        r.contract_version = 3;
        assert!(validate_registration(&r).is_err());
    }

    #[test]
    fn restart_witness_requires_distinct_physical_start_and_unchanged_inventory() {
        let (files, r, ack) = fixture();
        let witness: RestartWitness = serde_json::from_value(ack["witness"].clone()).unwrap();
        validate_restart(&witness, &files, &r, 0).unwrap();
        for (key, value) in [
            (
                "started_at",
                json!(files.mapped.as_ref().unwrap().mapping.snapshot.started_at),
            ),
            ("init_pid", json!(0)),
            ("inventory_sha256", json!("f".repeat(64))),
            ("container_id", json!("f".repeat(64))),
        ] {
            let mut changed = ack["witness"].clone();
            changed["current_controller_snapshot"][key] = value;
            let witness = serde_json::from_value(changed).unwrap();
            assert!(validate_restart(&witness, &files, &r, 0).is_err());
        }
    }

    #[test]
    fn recovery_ack_is_exact_original_command_and_agent_snapshot() {
        let (files, r, ack) = fixture();
        validate_recovery_value(&files, &r, "read_controller_recovery", 0, &ack).unwrap();
        for path in ["request_sha256", "recovery", "witness"] {
            let mut changed = ack.clone();
            match path {
                "recovery" => changed[path]["request"]["controller_id"] = json!(Uuid::new_v4()),
                "witness" => {
                    changed[path]["receipt"]["snapshot"]["started_at"] =
                        json!("0001-01-01T00:00:00Z")
                }
                _ => changed[path] = json!("f".repeat(64)),
            };
            assert!(
                validate_recovery_value(&files, &r, "recover_controller", 0, &changed).is_err()
            );
        }
        assert!(validate_recovery_value(&files, &r, "recover_controller", 2, &ack).is_err());
    }

    #[test]
    fn heartbeat_ack_cannot_change_owner_version_deadline_or_add_authority() {
        let (files, r, _) = fixture();
        let c = files.recovery.as_ref().unwrap();
        let ack = json!({"state":"controller_heartbeat","recovery_id":c.request.id,"lease_version":c.lease_version,"lease_expires_at":c.lease_expires_at});
        validate_recovery_value(&files, &r, "heartbeat_controller", 0, &ack).unwrap();
        for (key, value) in [
            ("recovery_id", json!(Uuid::new_v4())),
            ("lease_version", json!(2)),
            ("lease_expires_at", json!("2099-01-01T00:00:00Z")),
            ("endpoint", json!("http://foreign")),
        ] {
            let mut changed = ack.clone();
            changed[key] = value;
            assert!(
                validate_recovery_value(&files, &r, "heartbeat_controller", 0, &changed).is_err()
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn foreign_owner_cannot_deliver_or_extend_any_heartbeat() {
        use std::os::unix::fs::PermissionsExt;

        struct Fixture(PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let root =
            Fixture(std::env::temp_dir().join(format!("fleet-owner-heartbeat-{}", Uuid::new_v4())));
        std::fs::create_dir_all(root.0.join("scripts")).unwrap();
        let source = b"# sealed fixture\n";
        for name in [
            "runtime_boundary.py",
            "runtime_bootstrap.py",
            "runtime_control.py",
        ] {
            std::fs::write(root.0.join("scripts").join(name), source).unwrap();
        }
        let (mut files, r, _) = fixture();
        let c = files.recovery.as_ref().unwrap();
        let owner = c.request.controller_id;
        let envelope = json!({"protocol_version":3,"action":"heartbeat_controller","result":{
            "state":"controller_heartbeat","recovery_id":c.request.id,
            "lease_version":c.lease_version,"lease_expires_at":c.lease_expires_at}});
        let python = root.0.join("python");
        std::fs::write(&python, format!("#!/bin/sh\n/bin/cat >/dev/null\nprintf x >> \"$4/heartbeat-calls\"\nprintf '%s' '{envelope}'\n")).unwrap();
        std::fs::set_permissions(&python, std::fs::Permissions::from_mode(0o700)).unwrap();
        let control = ContainerControl::new(
            python,
            ControlSource {
                root: root.0.clone(),
                sha256: std::array::from_fn(|_| hex::encode(Sha256::digest(source))),
            },
            "desktop-linux".into(),
        )
        .unwrap();
        control
            .recovery_action(&files, &r, "heartbeat_controller", owner)
            .await
            .unwrap();
        assert_eq!(std::fs::read(root.0.join("heartbeat-calls")).unwrap(), b"x");
        let foreign = Uuid::new_v4();
        for (version, deadline) in [
            (1, "2026-10-09T13:00:30Z"),
            (2, "2099-01-01T00:00:00Z"),
            (2, "2000-01-01T00:00:00Z"),
        ] {
            let c = files.recovery.as_mut().unwrap();
            c.lease_version = version;
            c.lease_expires_at = deadline.into();
            assert!(
                control
                    .recovery_action(&files, &r, "heartbeat_controller", foreign)
                    .await
                    .is_err()
            );
            assert_eq!(std::fs::read(root.0.join("heartbeat-calls")).unwrap(), b"x");
        }
        files.recovery = None;
        assert!(
            control
                .recovery_action(&files, &r, "heartbeat_controller", owner)
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(root.0.join("heartbeat-calls")).unwrap(), b"x");
    }

    #[test]
    fn original_attachment_hash_cannot_follow_restarted_or_foreign_controller() {
        let (files, r, _) = fixture();
        let m = &files.mapped.as_ref().unwrap().mapping;
        let ack = json!({"state":"attached","registration_sha256":canonical_hash(&r).unwrap(),"controller_id":m.controller.container_id,
            "controller_sha256":canonical_hash(&m.snapshot).unwrap(),"network_id":files.policy["network"]["id"]});
        validate_attachment(&ack, &files, &r).unwrap();
        for key in [
            "controller_id",
            "controller_sha256",
            "registration_sha256",
            "network_id",
        ] {
            let mut changed = ack.clone();
            changed[key] = json!("f".repeat(64));
            assert!(validate_attachment(&changed, &files, &r).is_err());
        }
    }
}
