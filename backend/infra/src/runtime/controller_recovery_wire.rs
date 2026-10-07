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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_accepts_only_exact_closed_owner_version_and_deadline() {
        let id = uuid::Uuid::new_v4();
        let command: ControllerRecoveryCommand = serde_json::from_value(serde_json::json!({
            "request": {"id":id,"launch_id":uuid::Uuid::new_v4(),"agent_id":uuid::Uuid::new_v4(),
                "original_controller_id":uuid::Uuid::new_v4(),"controller_id":uuid::Uuid::new_v4(),
                "predecessor_id":null,"launch_sha256":"a".repeat(64),"mapping_sha256":"b".repeat(64),
                "registration_sha256":"c".repeat(64),"agent_pid":12345,
                "controller_snapshot":{"container_id":"d".repeat(64),"started_at":"2026-10-07T12:00:00Z",
                    "init_pid":987,"inventory_sha256":"e".repeat(64)}},
            "epoch":1,"lease_version":2,"lease_expires_at":"2026-10-07T12:00:30+00:00"
        })).unwrap();
        let original = serde_json::json!({"state":"controller_heartbeat","recovery_id":id,
            "lease_version":2,"lease_expires_at":command.lease_expires_at});
        validate_heartbeat(original.clone(), &command).unwrap();
        let anchor = serde_json::json!({"witness":{"receipt":{"observation":"running"}}});
        let mut live = original.clone();
        live["state"] = serde_json::json!("controller_heartbeat_live");
        live["receipt"] = anchor["witness"]["receipt"].clone();
        validate_live_heartbeat(live.clone(), &command, &anchor).unwrap();
        assert!(validate_live_heartbeat(original.clone(), &command, &anchor).is_err());
        for (key, value) in [
            ("state", serde_json::json!("controller_heartbeat")),
            ("recovery_id", serde_json::json!(uuid::Uuid::new_v4())),
            ("lease_version", serde_json::json!(true)),
            (
                "lease_expires_at",
                serde_json::json!("2026-10-07T12:00:31+00:00"),
            ),
            (
                "receipt",
                serde_json::json!({"observation":"namespace_exited"}),
            ),
            ("extra", serde_json::json!(null)),
        ] {
            let mut changed = live.clone();
            changed[key] = value;
            assert!(
                validate_live_heartbeat(changed, &command, &anchor).is_err(),
                "{key}"
            );
        }
        live.as_object_mut().unwrap().remove("receipt");
        assert!(validate_live_heartbeat(live, &command, &anchor).is_err());
        for (key, value) in [
            ("state", serde_json::json!("controller_recovered")),
            ("recovery_id", serde_json::json!(uuid::Uuid::new_v4())),
            ("lease_version", serde_json::json!(1)),
            ("lease_version", serde_json::json!(true)),
            (
                "lease_expires_at",
                serde_json::json!("2026-10-07T12:00:31+00:00"),
            ),
            ("extra", serde_json::json!(null)),
        ] {
            let mut changed = original.clone();
            changed[key] = value;
            assert!(validate_heartbeat(changed, &command).is_err(), "{key}");
        }
        for key in ["state", "recovery_id", "lease_version", "lease_expires_at"] {
            let mut changed = original.clone();
            changed.as_object_mut().unwrap().remove(key);
            assert!(validate_heartbeat(changed, &command).is_err(), "{key}");
        }
    }
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

#[cfg(test)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeartbeatReceipt {
    state: String,
    recovery_id: uuid::Uuid,
    lease_version: i64,
    lease_expires_at: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LiveHeartbeatReceipt {
    state: String,
    recovery_id: uuid::Uuid,
    lease_version: i64,
    lease_expires_at: String,
    receipt: Value,
}

fn validate_live_heartbeat(
    value: Value,
    command: &ControllerRecoveryCommand,
    original_receipt: &Value,
) -> Result<(), AppError> {
    let receipt: LiveHeartbeatReceipt = serde_json::from_value(value).map_err(|_| held())?;
    if receipt.state != "controller_heartbeat_live"
        || receipt.recovery_id != command.request.id
        || receipt.lease_version != command.lease_version
        || receipt.lease_expires_at != command.lease_expires_at
        || receipt.receipt != original_receipt["witness"]["receipt"]
    {
        return Err(held());
    }
    Ok(())
}

#[cfg(test)]
fn validate_heartbeat(value: Value, command: &ControllerRecoveryCommand) -> Result<(), AppError> {
    let receipt: HeartbeatReceipt = serde_json::from_value(value).map_err(|_| held())?;
    if receipt.state != "controller_heartbeat"
        || receipt.recovery_id != command.request.id
        || receipt.lease_version != command.lease_version
        || receipt.lease_expires_at != command.lease_expires_at
    {
        return Err(held());
    }
    Ok(())
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
    pub(super) async fn heartbeat_controller_live(
        &self,
        files: &ContainerLaunchFiles,
        launch: &RuntimeLaunchRecord,
        command: &ControllerRecoveryCommand,
        journal: &Path,
        original_receipt: &Value,
    ) -> Result<(), AppError> {
        validate_command(command, launch)?;
        let original = &launch
            .binding
            .container
            .as_ref()
            .ok_or_else(held)?
            .registration;
        let (status, value) = self
            .call_with_owner(
                files,
                "heartbeat_controller_live",
                serde_json::json!({"registration": original}),
                Some((command, journal)),
            )
            .await?;
        if status != 0 {
            return Err(held());
        }
        validate_live_heartbeat(value, command, original_receipt)
    }

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
