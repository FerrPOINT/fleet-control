use super::*;
use sha2::Digest;

pub(super) async fn verify(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    record: &domain::PmRunRecord,
    requested: &domain::PmNativeConfiguration,
) -> Result<bool, AppError> {
    requested.validate()?;
    Ok(verified(supervisor, agent, record, Some(requested))
        .await?
        .is_some())
}

pub(super) async fn observe(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    record: &domain::PmRunRecord,
) -> Result<domain::PmNativeAdmissionObservation, AppError> {
    verified(supervisor, agent, record, None)
        .await?
        .ok_or_else(|| AppError::Unavailable("PM Workflow enrollment is pending".into()))
}

async fn verified(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    record: &domain::PmRunRecord,
    requested: Option<&domain::PmNativeConfiguration>,
) -> Result<Option<domain::PmNativeAdmissionObservation>, AppError> {
    let operation = supervisor
        .repo
        .find_pm_creation_for_session(record.reservation.session_id)
        .await?;
    if operation.execution_identity()? != record.reservation.identity {
        return Err(AppError::conflict(
            "PM native run differs from original creation",
        ));
    }
    let coordinator =
        crate::pm_credentials::PmCredentialCoordinator::configured(&supervisor.config)?
            .ok_or_else(|| AppError::Unavailable("PM scoped credentials are unavailable".into()))?;
    let credential = coordinator
        .prepare_credential(supervisor.repo.as_ref(), &operation)
        .await?;
    let workflow = crate::pm_workflow::PmWorkflowClient::configured(&supervisor.config)?
        .ok_or_else(|| AppError::Unavailable("PM Workflow is unavailable".into()))?;
    if !workflow.verify_active_run(record).await? {
        return Ok(None);
    }
    let original =
        record.reservation.runtime_binding.as_ref().ok_or_else(|| {
            AppError::Unavailable("PM original native binding is required".into())
        })?;
    if pm_readback::capture(supervisor, agent).await? != *original {
        return Err(AppError::conflict("PM native launch binding changed"));
    }
    let launch = supervisor
        .repo
        .get_runtime_launch(original.launch_id)
        .await?
        .ok_or_else(|| AppError::Unavailable("PM original launch is unavailable".into()))?;
    let effective = supervisor
        .repo
        .get_effective_config_revision(agent.id)
        .await?
        .ok_or_else(|| AppError::Unavailable("PM effective configuration is unavailable".into()))?;
    let snapshot = serde_json::to_value(&effective.snapshot).map_err(AppError::internal)?;
    if launch.binding.configuration_revision != Some(effective.revision)
        || launch.binding.configuration_sha256.as_deref()
            != Some(crate::runtime_launches::snapshot_hash(&snapshot)?.as_str())
    {
        return Err(AppError::conflict(
            "PM effective configuration differs from its original launch",
        ));
    }
    crate::effective_configuration::verify(agent, &supervisor.config, &effective).await?;
    let native = record
        .hermes_run_ref
        .as_deref()
        .filter(|v| crate::pm_execution::valid_hermes_ref(v))
        .ok_or_else(|| AppError::Unavailable("PM native run acceptance is unknown".into()))?;
    let response = supervisor
        .client
        .get(format!(
            "{}/fleet/v1/pm/inventory/{native}",
            original.origin
        ))
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .timeout(Duration::from_secs(5))
        .bearer_auth(crate::agent_runtime_token(&supervisor.config, agent.id)?)
        .send()
        .await
        .map_err(|_| AppError::Unavailable("PM native inventory is unavailable".into()))?;
    let value = hermes_wire::read_json(response, reqwest::StatusCode::OK, 16 * 1024).await?;
    let inventory: domain::PmNativeInventory = serde_json::from_value(value.clone())
        .map_err(|_| AppError::conflict("PM native inventory is malformed"))?;
    if serde_json::to_value(&inventory).map_err(AppError::internal)? != value {
        return Err(AppError::conflict("PM native inventory is not canonical"));
    }
    let actual_request = domain::PmNativeConfiguration {
        model: inventory.model.clone(),
        provider: inventory.provider.clone(),
        api_mode: inventory.api_mode.clone(),
        route_sha256: inventory.route_sha256.clone(),
        tools: inventory.tools.clone(),
        output_limit: inventory.output_limit,
    };
    let requested = requested.unwrap_or(&actual_request);
    inventory.verify(record, requested, &effective.snapshot.config.config_json)?;
    let expected_route = effective.snapshot.config.config_json["model"]["base_url"]
        .as_str()
        .ok_or_else(|| AppError::conflict("PM model route must be explicit"))?;
    if requested.route_sha256 != hex::encode(sha2::Sha256::digest(expected_route.as_bytes())) {
        return Err(AppError::conflict(
            "PM model route differs from its effective snapshot",
        ));
    }
    if pm_readback::probe(supervisor, agent, record).await? != domain::PmRuntimeStatus::Running {
        return Err(AppError::conflict("PM native run is not active"));
    }
    if pm_readback::capture(supervisor, agent).await? != *original {
        return Err(AppError::conflict(
            "PM native launch changed during admission",
        ));
    }
    let original_claim = operation
        .execution_lease
        .as_ref()
        .ok_or_else(|| AppError::conflict("PM original lease is missing"))?;
    let current = coordinator
        .read_execution_lease(
            &operation,
            &credential,
            Some(&domain::PmExecutionLeaseCommand::Claim(
                original_claim.claim.clone(),
            )),
        )
        .await?;
    if current.state != domain::PmExecutionLeaseState::Active || current.operation.is_none() {
        return Err(AppError::conflict(
            "PM lease is not active at native admission",
        ));
    }
    Ok(Some(domain::PmNativeAdmissionObservation {
        contract_version: 1,
        observation_ref: Uuid::new_v4(),
        observed_at: chrono::Utc::now(),
        identity: record.reservation.identity.clone(),
        session_run_id: record.reservation.session_run_id,
        native_run_ref: native.into(),
        native_session_ref: inventory.session_id,
        binding_ref: record.reservation.binding_ref.clone(),
        fence: record.reservation.fence,
        effective_config_revision: effective.revision,
        configuration_sha256: crate::runtime_launches::snapshot_hash(&snapshot)?,
        native_configuration: requested.clone(),
    }))
}
