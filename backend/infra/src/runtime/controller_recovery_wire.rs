//! Closed native ownership input and original-key receipt, never a public readiness DTO.
use super::container_control::{
    ContainerControl, ContainerLaunchFiles, ContainerObservation, ContainerRegistration,
    canonical_hash, decode_controller_restart,
};
use app::runtime_launch::{ControllerRecoveryCommand, RuntimeLaunchRecord};
use serde::Deserialize;
use serde_json::Value;
use shared::AppError;
use std::path::Path;

fn held() -> AppError {
    AppError::Unavailable("original native controller ownership remains fenced".into())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryReceipt {
    contract_version: u8,
    state: String,
    request_sha256: String,
    recovery: Value,
    witness: Value,
}

pub(crate) fn validate_command(
    command: &ControllerRecoveryCommand,
    launch: &RuntimeLaunchRecord,
) -> Result<(), AppError> {
    let request = &command.request;
    let binding = &launch.binding;
    let container = binding.container.as_ref().ok_or_else(held)?;
    let mapping = container.mount_mapping.as_ref().ok_or_else(held)?;
    if command.epoch <= 0
        || command.lease_version <= 0
        || (command.epoch == 1) != request.predecessor_id.is_none()
        || [
            request.id,
            request.controller_id,
            request.original_controller_id,
            request.launch_id,
            request.agent_id,
        ]
        .into_iter()
        .any(|id| id.is_nil())
        || request
            .predecessor_id
            .is_some_and(|id| id.is_nil() || id == request.id)
        || request.controller_id == request.original_controller_id
        || request.launch_id != binding.id
        || request.agent_id != binding.agent_id
        || request.original_controller_id != binding.controller_id
        || request.agent_pid <= 0
        || launch.pid != Some(request.agent_pid)
        || launch.state != "gateway_started"
        || request.launch_sha256 != canonical_hash(binding)?
        || request.mapping_sha256 != canonical_hash(mapping)?
        || request.registration_sha256 != canonical_hash(&container.registration)?
        || request.controller_snapshot.container_id != mapping.snapshot.container_id
        || request.controller_snapshot.inventory_sha256 != mapping.snapshot.inventory_sha256
        || request.controller_snapshot.started_at == mapping.snapshot.started_at
        || request.controller_snapshot.started_at.is_empty()
        || request.controller_snapshot.started_at.starts_with("0001-")
        || request.controller_snapshot.started_at.len() > 128
        || !request
            .controller_snapshot
            .started_at
            .bytes()
            .all(|byte| byte.is_ascii_graphic())
        || request.controller_snapshot.init_pid == 0
    {
        return Err(held());
    }
    let deadline =
        chrono::DateTime::parse_from_rfc3339(&command.lease_expires_at).map_err(|_| held())?;
    if deadline.offset().local_minus_utc() != 0 || command.lease_expires_at.len() > 96 {
        return Err(held());
    }
    crate::runtime_launches::validate_container_binding(binding)
}

/// Historical ACK validation never checks or renews current owner authority.
pub(crate) fn validate_receipt(
    value: &Value,
    command: &ControllerRecoveryCommand,
    launch: &RuntimeLaunchRecord,
) -> Result<(), AppError> {
    validate_command(command, launch)?;
    let receipt: RecoveryReceipt = serde_json::from_value(value.clone()).map_err(|_| held())?;
    if receipt.contract_version != 1
        || receipt.state != "controller_recovered"
        || receipt.recovery != serde_json::to_value(command).map_err(|_| held())?
        || receipt.request_sha256 != canonical_hash(command)?
    {
        return Err(held());
    }
    let container = launch.binding.container.as_ref().ok_or_else(held)?;
    let mapping = container.mount_mapping.as_ref().ok_or_else(held)?;
    let witness = decode_controller_restart(0, receipt.witness, mapping, &container.registration)?;
    if witness.current_controller_snapshot != command.request.controller_snapshot
        || witness
            .receipt
            .snapshot
            .as_ref()
            .and_then(|snapshot| i32::try_from(snapshot.init_pid).ok())
            != Some(command.request.agent_pid)
        || !matches!(
            witness.receipt.observation,
            ContainerObservation::Running | ContainerObservation::NamespaceExited
        )
    {
        return Err(held());
    }
    Ok(())
}

impl ContainerControl {
    pub async fn recover_controller(
        &self,
        files: &ContainerLaunchFiles,
        launch: &RuntimeLaunchRecord,
        command: &ControllerRecoveryCommand,
        journal: &Path,
    ) -> Result<Value, AppError> {
        self.controller_recovery_call(files, launch, command, journal, "recover_controller")
            .await
    }

    pub async fn read_controller_recovery(
        &self,
        files: &ContainerLaunchFiles,
        launch: &RuntimeLaunchRecord,
        command: &ControllerRecoveryCommand,
        journal: &Path,
    ) -> Result<Value, AppError> {
        self.controller_recovery_call(files, launch, command, journal, "read_controller_recovery")
            .await
    }

    async fn controller_recovery_call(
        &self,
        files: &ContainerLaunchFiles,
        launch: &RuntimeLaunchRecord,
        command: &ControllerRecoveryCommand,
        journal: &Path,
        action: &str,
    ) -> Result<Value, AppError> {
        validate_command(command, launch)?;
        let original: &ContainerRegistration = &launch
            .binding
            .container
            .as_ref()
            .ok_or_else(held)?
            .registration;
        let (status, value) = self
            .call_with_owner(
                files,
                action,
                serde_json::json!({"registration": original}),
                Some((command, journal)),
            )
            .await?;
        if status != 0 {
            return Err(held());
        }
        validate_receipt(&value, command, launch)?;
        Ok(value)
    }
}
