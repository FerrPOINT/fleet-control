//! Initial PM dispatch through the existing Hermes/Workflow HTTP contracts.
use super::*;
use domain::{PmDispatchIntent, PmDraftOperation, PmRunReservation};
use serde::{Deserialize, de::DeserializeOwned};

pub(super) fn unavailable() -> AppError {
    AppError::Unavailable("PM Workflow dispatch contract is unavailable or invalid".into())
}

pub(super) fn decode<T: DeserializeOwned + Serialize>(value: Value) -> Result<T, AppError> {
    let typed: T = serde_json::from_value(value.clone()).map_err(|_| unavailable())?;
    if serde_json::to_value(&typed).map_err(|_| unavailable())? != value {
        return Err(unavailable());
    }
    Ok(typed)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Readiness {
    service: String,
    schema: String,
    catalog: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    schema_version: u8,
    source_revision: String,
    source_archive_sha256: String,
    runtime_bundle_sha256: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct Compatibility {
    catalog_version: u8,
    catalog_revision: String,
    catalog_sha256: String,
    skills_revision: String,
    skills_manifest_sha256: String,
    capability_revision: String,
    capability_sha256: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Continuation {
    contract_version: u8,
    base_path: String,
    commands: Vec<String>,
    terminal_proof: String,
    dispatch_owner: String,
    execution_token_header: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Capabilities {
    ok: bool,
    role_key: String,
    credential_kind: String,
    capabilities: Vec<String>,
    readiness: Readiness,
    source_provenance: Provenance,
    #[serde(rename = "runtimeCompatibility")]
    compatibility: Compatibility,
    pm_continuation: Continuation,
}

fn hex_ref(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl Capabilities {
    fn validate(&self, kind: &str) -> Result<(), AppError> {
        let (capabilities, commands): (&[&str], &[&str]) = if kind == "assignment" {
            (
                &["assign", "bind", "rebind"],
                &["assign", "bind", "resume", "rebind", "readback"],
            )
        } else {
            (&["step", "history"], &["checkpoint", "readback"])
        };
        if !self.ok
            || self.role_key != "project_manager"
            || self.credential_kind != kind
            || self
                .capabilities
                .iter()
                .map(String::as_str)
                .ne(capabilities.iter().copied())
            || self
                .pm_continuation
                .commands
                .iter()
                .map(String::as_str)
                .ne(commands.iter().copied())
            || self.readiness.service != "ready"
            || self.readiness.schema != "ready"
            || self.readiness.catalog != "ready"
            || self.source_provenance.schema_version != 1
            || !hex_ref(&self.source_provenance.source_revision, 40)
            || !hex_ref(&self.source_provenance.source_archive_sha256, 64)
            || !hex_ref(&self.source_provenance.runtime_bundle_sha256, 64)
            || self.compatibility.catalog_version != 2
            || self.compatibility.catalog_revision != self.source_provenance.source_revision
            || !hex_ref(&self.compatibility.skills_revision, 40)
            || !hex_ref(&self.compatibility.catalog_sha256, 64)
            || !hex_ref(&self.compatibility.skills_manifest_sha256, 64)
            || !hex_ref(&self.compatibility.capability_sha256, 64)
            || self.compatibility.capability_revision != "hermes-sdlc-runtime/v2"
            || self.pm_continuation.contract_version != 1
            || self.pm_continuation.base_path != "/internal/runtime/v1/pm"
            || self.pm_continuation.terminal_proof != "configured-runtime-readback"
            || self.pm_continuation.dispatch_owner != "fleet"
            || self.pm_continuation.execution_token_header != "X-Workflow-Execution-Token"
        {
            return Err(unavailable());
        }
        Ok(())
    }
}

// Exact result emitted by Workflow _assignment_response, including required nullable fields.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Assignment {
    task_key: String,
    workflow_id: i64,
    workflow_key: String,
    mode_id: i64,
    mode_key: String,
    cycle_number: i64,
    attempt_number: i64,
    assignment_operation_key: String,
    assignment_revision: i64,
    role_key: String,
    execution_scope: String,
    stage_key: String,
    business_task_ref: String,
    root_task_ref: String,
    work_item_ref: Option<String>,
    work_item_revision: Option<i64>,
    queue_item_ref: Option<String>,
    task_workspace_ref: Option<String>,
    workspace_revision: Option<i64>,
    tech_execution_workspace_ref: Option<String>,
    tech_execution_attempt_ref: Option<String>,
    decomposition_revision_ref: Option<String>,
    stage_revision: String,
    assignment_ref: String,
    binding_ref: Option<String>,
    hermes_run_ref: Option<String>,
    bind_operation_key: Option<String>,
    concrete_agent_ref: Option<String>,
    workspace_generation: Option<i64>,
    lease_generation: Option<i64>,
    exact_input_refs: Vec<ExactInput>,
    binding_state: String,
    status: String,
    current_phase_id: i64,
    current_phase_code: String,
    current_phase_name: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExactInput {
    kind: String,
    #[serde(rename = "ref")]
    reference: String,
    hash: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AssignmentResponse {
    ok: bool,
    exit_code: i64,
    result: Assignment,
}

impl AssignmentResponse {
    fn validate_identity(
        &self,
        operation: &PmDraftOperation,
        reservation: &PmRunReservation,
    ) -> Result<(), AppError> {
        let a = &self.result;
        let expected = domain::initial_pm_assignment(operation, Value::Null)?;
        let input = operation.input.as_ref().ok_or_else(unavailable)?;
        if !self.ok
            || self.exit_code != 0
            || a.task_key != reservation.identity.task
            || a.assignment_operation_key != reservation.identity.assignment_operation_key
            || a.assignment_ref != reservation.identity.assignment_ref
            || a.assignment_revision != reservation.identity.assignment_revision
            || a.business_task_ref != reservation.identity.task_ref
            || a.root_task_ref != reservation.identity.root_ref
            || a.work_item_ref.is_some()
            || a.queue_item_ref.is_some()
            || a.work_item_revision.is_some()
            || a.stage_revision
                != expected["owner_version"]
                    .as_u64()
                    .ok_or_else(unavailable)?
                    .to_string()
            || a.role_key != "project_manager"
            || a.workflow_key != "hermes-sdlc:project_manager"
            || a.mode_key != "draft"
            || a.execution_scope != "business"
            || a.stage_key != "draft"
            || a.cycle_number != 0
            || a.attempt_number != 1
            || a.workflow_id <= 0
            || a.mode_id <= 0
            || a.current_phase_id <= 0
            || a.task_workspace_ref.is_some()
            || a.workspace_revision.is_some()
            || a.tech_execution_workspace_ref.is_some()
            || a.tech_execution_attempt_ref.is_some()
            || a.decomposition_revision_ref.is_some()
            || a.workspace_generation.is_some()
            || a.lease_generation != Some(1)
            || a.exact_input_refs.len() != 1
            || a.exact_input_refs[0].kind != "pm_draft_input"
            || a.exact_input_refs[0].reference != input.input.snapshot_ref.to_string()
            || a.exact_input_refs[0].hash != input.input.sha256
        {
            return Err(unavailable());
        }
        Ok(())
    }

    fn validate(
        &self,
        operation: &PmDraftOperation,
        reservation: &PmRunReservation,
        run: Option<&str>,
    ) -> Result<(), AppError> {
        self.validate_identity(operation, reservation)?;
        let a = &self.result;
        if a.status != "active" || a.current_phase_code != "PM-DRAFT-01" {
            return Err(unavailable());
        }
        match run {
            None if a.binding_state == "unbound"
                && a.binding_ref.is_none()
                && a.hermes_run_ref.is_none()
                && a.bind_operation_key.is_none()
                && a.concrete_agent_ref.is_none() =>
            {
                Ok(())
            }
            Some(run)
                if a.binding_state == "bound"
                    && a.binding_ref.as_deref() == Some(reservation.binding_ref.as_str())
                    && a.hermes_run_ref.as_deref() == Some(run)
                    && a.bind_operation_key.as_deref()
                        == Some(format!("fleet-pm-runtime-bind:{}", operation.id).as_str())
                    && a.concrete_agent_ref.as_deref()
                        == Some(reservation.identity.agent_ref.as_str()) =>
            {
                Ok(())
            }
            _ => Err(unavailable()),
        }
    }
}

pub(super) struct Workflow<'a> {
    supervisor: &'a LocalRuntimeSupervisor,
    pub(super) origin: String,
    pub(super) assignment: &'a str,
    pub(super) runtime: &'a str,
}

impl<'a> Workflow<'a> {
    pub(super) fn verify_intent(&self, intent: &PmDispatchIntent) -> Result<(), AppError> {
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        hash.update(b"fleet-pm-workflow-v1\0");
        hash.update(self.assignment.as_bytes());
        hash.update(b"\0");
        hash.update(self.runtime.as_bytes());
        if intent.workflow_origin != self.origin
            || intent.workflow_credential_fingerprint != hex::encode(hash.finalize())
        {
            return Err(AppError::conflict(
                "PM original Workflow credentials or origin changed",
            ));
        }
        Ok(())
    }

    pub(super) async fn cursor(
        &self,
        intent: &PmDispatchIntent,
        operation: &PmDraftOperation,
    ) -> Result<(i64, i64, String, String), AppError> {
        self.verify_intent(intent)?;
        let initial = domain::initial_pm_reservation(operation)?;
        let identity = &initial.identity;
        let original = self
            .supervisor
            .repo
            .get_pm_dispatch(operation.id)
            .await?
            .ok_or_else(unavailable)?;
        self.verify_intent(&original)?;
        let native = original
            .hermes_run_ref
            .as_deref()
            .filter(|_| original.submitted)
            .ok_or_else(unavailable)?;
        // Bind replay returns the actual task cursor; assign replay deliberately returns phase zero.
        let binding = json!({"task":identity.task,"bind_operation_key":format!("fleet-pm-runtime-bind:{}",operation.id),
            "assignment_operation_key":identity.assignment_operation_key,"assignment_revision":identity.assignment_revision,
            "assignment_ref":identity.assignment_ref,"binding_ref":initial.binding_ref,"hermes_run_ref":native,
            "mode_key":"draft","cycle_number":0,"attempt_number":1,"expected_binding_state":"unbound","concrete_agent_ref":identity.agent_ref});
        let assigned: AssignmentResponse = decode(
            self.call(
                "/internal/runtime/bind",
                self.assignment,
                Some(binding),
                None,
            )
            .await?,
        )?;
        assigned.validate_identity(operation, &initial)?;
        let a = assigned.result;
        if !assigned.ok
            || assigned.exit_code != 0
            || a.task_key != identity.task
            || a.assignment_ref != identity.assignment_ref
            || a.assignment_revision != identity.assignment_revision
            || a.assignment_operation_key != identity.assignment_operation_key
            || a.concrete_agent_ref.as_deref() != Some(identity.agent_ref.as_str())
            || a.business_task_ref != identity.task_ref
            || a.root_task_ref != identity.root_ref
            || a.role_key != "project_manager"
            || a.mode_key != "draft"
            || a.cycle_number != 0
            || a.attempt_number != 1
            || a.workflow_id <= 0
            || a.mode_id <= 0
            || a.binding_state != "bound"
            || a.binding_ref.as_deref() != Some(initial.binding_ref.as_str())
            || a.hermes_run_ref.as_deref() != Some(native)
            || a.bind_operation_key.as_deref()
                != Some(format!("fleet-pm-runtime-bind:{}", operation.id).as_str())
            || !a.current_phase_code.starts_with("PM-DRAFT-")
            || !domain::valid_ref(&a.current_phase_code, 128)
            || !matches!(a.status.as_str(), "active" | "blocked")
        {
            return Err(unavailable());
        }
        Ok((a.workflow_id, a.mode_id, a.current_phase_code, a.status))
    }
    pub(super) fn configured(supervisor: &'a LocalRuntimeSupervisor) -> Result<Self, AppError> {
        let config = &supervisor.config;
        let origin = config
            .fleet
            .project_workflow_url
            .as_deref()
            .ok_or_else(unavailable)?;
        let url = reqwest::Url::parse(origin).map_err(|_| unavailable())?;
        if !config.pm.dispatch.enabled
            || origin.trim() != origin
            || origin.contains('\\')
            || !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(unavailable());
        }
        let assignment = &config.pm.dispatch.assignment_token;
        let runtime = &config.pm.dispatch.runtime_token;
        let others = [
            &config.pm.readback_token,
            &config.pm.namespace_read_pat,
            &config.pm.credentials.parent_pat,
            &config.auth.jwt_secret,
            &config.fleet.runtime_token_secret,
        ];
        for token in [assignment, runtime] {
            if !(32..=512).contains(&token.len())
                || !token.bytes().all(|b| b.is_ascii_graphic())
                || others.contains(&token)
                || config.fleet.project_workflow_catalog_token.as_ref() == Some(token)
            {
                return Err(unavailable());
            }
        }
        if assignment == runtime {
            return Err(unavailable());
        }
        if !(32..=512).contains(&config.pm.readback_token.len())
            || !config
                .pm
                .readback_token
                .bytes()
                .all(|b| b.is_ascii_graphic())
        {
            return Err(unavailable());
        }
        Ok(Self {
            supervisor,
            origin: origin.trim_end_matches('/').into(),
            assignment,
            runtime,
        })
    }

    pub(super) async fn call(
        &self,
        path: &str,
        token: &str,
        body: Option<Value>,
        scope: Option<&str>,
    ) -> Result<Value, AppError> {
        let mut request = match body {
            Some(body) => self
                .supervisor
                .client
                .post(format!("{}{path}", self.origin))
                .json(&body),
            None => self.supervisor.client.get(format!("{}{path}", self.origin)),
        }
        .bearer_auth(token)
        .timeout(Duration::from_secs(10))
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .header(reqwest::header::CACHE_CONTROL, "no-cache, no-store");
        if let Some(scope) = scope {
            let mut header =
                reqwest::header::HeaderValue::from_str(scope).map_err(|_| unavailable())?;
            header.set_sensitive(true);
            request = request.header("X-Workflow-Execution-Token", header);
        }
        let response = request.send().await.map_err(|_| unavailable())?;
        if !response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|header| header.to_str().ok())
            .is_some_and(|mime| {
                mime.split(';')
                    .next()
                    .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
            })
        {
            return Err(unavailable());
        }
        hermes_wire::read_json(response, reqwest::StatusCode::OK, 262_144).await
    }
}

pub(super) async fn dispatch(
    supervisor: &LocalRuntimeSupervisor,
    operation: &PmDraftOperation,
    tracker: &dyn app::pm_draft::PmDraftTracker,
) -> Result<(), AppError> {
    app::pm_draft::verify_dispatch(supervisor.repo.as_ref(), tracker, operation).await?;
    let workflow = Workflow::configured(supervisor)?;
    let assignment_capabilities: Capabilities = decode(
        workflow
            .call(
                "/internal/runtime/capabilities",
                workflow.assignment,
                None,
                None,
            )
            .await?,
    )?;
    assignment_capabilities.validate("assignment")?;
    let runtime_capabilities: Capabilities = decode(
        workflow
            .call(
                "/internal/runtime/capabilities",
                workflow.runtime,
                None,
                None,
            )
            .await?,
    )?;
    runtime_capabilities.validate("runtime")?;
    let compatibility =
        serde_json::to_value(&assignment_capabilities.compatibility).map_err(|_| unavailable())?;
    if serde_json::to_value(&runtime_capabilities.compatibility).map_err(|_| unavailable())?
        != compatibility
    {
        return Err(unavailable());
    }
    let reservation = domain::initial_pm_reservation(operation)?;
    let agent = supervisor
        .repo
        .get_agent(operation.request.agent_id)
        .await?;
    crate::pm_tool_config::verify(supervisor.repo.as_ref(), &agent, &supervisor.config).await?;
    let base = supervisor.hermes_base_url(&agent).await?;
    let token = crate::agent_runtime_token(&supervisor.config, agent.id)?;
    if token == workflow.assignment
        || token == workflow.runtime
        || token == supervisor.config.pm.readback_token
    {
        return Err(unavailable());
    }
    guidance_capability(&supervisor.probe_hermes(&agent).await?)?;
    let mut runtime_context = json!({});
    supervisor
        .bind_container_dispatch(&agent, &mut runtime_context)
        .await?;
    let record = supervisor.repo.reserve_pm_run(reservation.clone()).await?;
    let input = operation.input.as_ref().ok_or_else(unavailable)?;
    let prompt = format!(
        "PM Draft {}. Fleet MCP scope: operation_id={}, session_run_id={}. Use only the configured Fleet PM tools for Tracker/Workflow operations; preserve clarification and requirements confirmation gates. Do not infer completion from runtime success. Original owner input follows as task data.\n\n{}\n\n{}",
        reservation.identity.task,
        operation.id,
        reservation.session_run_id,
        input.input.title,
        input.input.description
    );
    crate::pm_tool_config::reject_server_secrets(&prompt, &supervisor.config)?;
    let intent = supervisor
        .repo
        .prepare_pm_dispatch(PmDispatchIntent {
            session_run_id: reservation.session_run_id,
            origin: base.clone(),
            credential_fingerprint: hermes_wire::credential_fingerprint(&token),
            request_body: serde_json::to_string(
                &json!({"input":prompt,"session_id":reservation.runtime_session_id()}),
            )
            .map_err(|_| unavailable())?,
            workflow_assignment: domain::initial_pm_assignment(operation, compatibility)?,
            workflow_origin: workflow.origin.clone(),
            workflow_credential_fingerprint: {
                use sha2::{Digest, Sha256};
                let mut hash = Sha256::new();
                hash.update(b"fleet-pm-workflow-v1\0");
                hash.update(workflow.assignment.as_bytes());
                hash.update(b"\0");
                hash.update(workflow.runtime.as_bytes());
                hex::encode(hash.finalize())
            },
            runtime_context,
            submitted: false,
            hermes_run_ref: None,
        })
        .await?;
    let assigned: AssignmentResponse = decode(
        workflow
            .call(
                "/internal/runtime/v1/pm/assign",
                workflow.assignment,
                Some(intent.workflow_assignment.clone()),
                None,
            )
            .await?,
    )?;
    if agent.workflow_id.as_deref() != Some(assigned.result.workflow_id.to_string().as_str()) {
        return Err(AppError::conflict(
            "PM Workflow differs from the concrete agent binding",
        ));
    }
    // An exact assign replay returns the current binding, not necessarily the original unbound result.
    assigned
        .validate(operation, &reservation, None)
        .or_else(|error| match intent.hermes_run_ref.as_deref() {
            Some(run) => assigned.validate(operation, &reservation, Some(run)),
            None => Err(error),
        })?;
    verify_context(supervisor, &agent, &intent).await?;
    let run_ref = if let Some(run_ref) = intent.hermes_run_ref {
        run_ref
    } else {
        app::pm_draft::verify_dispatch(supervisor.repo.as_ref(), tracker, operation).await?;
        if intent.submitted
            || !supervisor
                .repo
                .claim_pm_submission(reservation.session_run_id)
                .await?
        {
            return Err(AppError::Unavailable(
                "PM Hermes acceptance is unknown; no automatic resubmission".into(),
            ));
        }
        let run_ref = hermes_wire::submit(
            &supervisor.client,
            &base,
            &token,
            reservation.session_run_id,
            &intent.request_body,
            None,
        )
        .await?;
        // Save the ACK before GET: a lost readback must never cause a second POST.
        supervisor
            .repo
            .record_pm_submission(reservation.session_run_id, run_ref.clone())
            .await?;
        run_ref
    };
    let native =
        hermes_wire::read_accepted_run(&supervisor.client, &base, &token, &run_ref).await?;
    let effective_session = hermes_wire::effective_session(&native, &run_ref)?;
    if record
        .hermes_run_ref
        .as_ref()
        .is_some_and(|old| old != &run_ref)
    {
        return Err(unavailable());
    }
    let accepted = supervisor
        .repo
        .accept_pm_run(
            reservation.session_run_id,
            run_ref.clone(),
            effective_session,
        )
        .await?;
    let status = supervisor.probe_pm_run(&agent, &accepted).await?;
    supervisor
        .repo
        .observe_pm_run(reservation.session_run_id, status)
        .await?;
    if status != domain::PmRuntimeStatus::Running {
        return Err(AppError::conflict(
            "PM run terminated before Workflow binding; reconcile without redispatch",
        ));
    }
    let binding = json!({"task":reservation.identity.task,"bind_operation_key":format!("fleet-pm-runtime-bind:{}",operation.id),
        "assignment_operation_key":reservation.identity.assignment_operation_key,"assignment_revision":reservation.identity.assignment_revision,
        "assignment_ref":reservation.identity.assignment_ref,"binding_ref":reservation.binding_ref,"hermes_run_ref":run_ref,
        "mode_key":"draft","cycle_number":0,"attempt_number":1,"expected_binding_state":"unbound","concrete_agent_ref":reservation.identity.agent_ref});
    let bound: AssignmentResponse = decode(
        workflow
            .call(
                "/internal/runtime/bind",
                workflow.assignment,
                Some(binding),
                None,
            )
            .await?,
    )?;
    bound.validate(operation, &reservation, Some(&run_ref))?;
    if bound.result.workflow_id != assigned.result.workflow_id
        || bound.result.mode_id != assigned.result.mode_id
    {
        return Err(unavailable());
    }
    let mut bind = serde_json::to_value(&reservation.identity).map_err(|_| unavailable())?;
    bind.as_object_mut().ok_or_else(unavailable)?.extend(json!({"operation_key":reservation.dispatch_operation_key,"expected_version":0,
        "binding_ref":reservation.binding_ref,"hermes_run_ref":run_ref,"session_run_id":reservation.session_run_id}).as_object().ok_or_else(unavailable)?.clone());
    let pm = workflow
        .call(
            "/internal/runtime/v1/pm/bind",
            workflow.assignment,
            Some(bind),
            None,
        )
        .await?;
    let scope = validate_pm_binding(pm, &reservation, &run_ref)?;
    let step = json!({"task":reservation.identity.task,"step_operation_key":format!("fleet-pm-first-step:{}",operation.id),
        "assignment_revision":reservation.identity.assignment_revision,"assignment_ref":reservation.identity.assignment_ref,
        "binding_ref":reservation.binding_ref,"hermes_run_ref":run_ref,"mode_key":"draft","cycle_number":0,"attempt_number":1,
        "expected_phase_code":"PM-DRAFT-01","expected_status":"active","session_run_id":reservation.session_run_id});
    let first_step = workflow
        .call(
            "/internal/runtime/step",
            workflow.runtime,
            Some(step),
            Some(&scope),
        )
        .await?;
    let instructions = validate_first_step(
        first_step,
        &reservation,
        (bound.result.workflow_id, bound.result.mode_id),
    )?;
    app::pm_draft::verify_dispatch(supervisor.repo.as_ref(), tracker, operation).await?;
    let intent = supervisor
        .repo
        .get_pm_dispatch(reservation.session_run_id)
        .await?
        .ok_or_else(unavailable)?;
    verify_context(supervisor, &agent, &intent).await?;
    guidance_capability(&supervisor.probe_hermes(&agent).await?)?;
    let native =
        hermes_wire::read_accepted_run(&supervisor.client, &base, &token, &run_ref).await?;
    if native["status"] != "running"
        || accepted.hermes_session_ref.as_deref()
            != Some(hermes_wire::effective_session(&native, &run_ref)?.as_str())
    {
        return Err(AppError::Unavailable(
            "PM run is not ready for Workflow guidance".into(),
        ));
    }
    let guidance =
        serde_json::to_string(&json!({"input":instructions})).map_err(|_| unavailable())?;
    match supervisor
        .repo
        .claim_pm_guidance(reservation.session_run_id, guidance.clone())
        .await?
    {
        domain::PmGuidancePermit::Delivered => return Ok(()),
        domain::PmGuidancePermit::Unknown => {
            return Err(AppError::Unavailable(
                "PM guidance acceptance is unknown; no automatic retry".into(),
            ));
        }
        domain::PmGuidancePermit::Claimed => (),
    }
    // Native steering is ordinary post-acceptance guidance, not a pre-model barrier.
    let response = supervisor
        .client
        .post(format!("{base}/v1/runs/{run_ref}/steer"))
        .bearer_auth(&token)
        .timeout(Duration::from_secs(10))
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(guidance)
        .send()
        .await
        .map_err(|_| {
            AppError::Unavailable("PM guidance acceptance is unknown; no automatic retry".into())
        })?;
    let ack: NativeGuidanceAck =
        decode(hermes_wire::read_json(response, reqwest::StatusCode::OK, 65536).await?)?;
    if ack.object != "hermes.run.steer" || ack.run_id != run_ref || !ack.accepted {
        return Err(unavailable());
    }
    supervisor
        .repo
        .finish_pm_guidance(reservation.session_run_id)
        .await?;
    Ok(())
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct NativeGuidanceAck {
    object: String,
    run_id: String,
    accepted: bool,
}

fn guidance_capability(value: &Value) -> Result<(), AppError> {
    hermes_wire::task_protocol(value)?;
    if value["features"]["run_steer"] != true
        || value["endpoints"]["run_steer"]["method"] != "POST"
        || value["endpoints"]["run_steer"]["path"] != "/v1/runs/{run_id}/steer"
    {
        return Err(unavailable());
    }
    Ok(())
}

pub(super) async fn verify_context(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    intent: &PmDispatchIntent,
) -> Result<String, AppError> {
    let current_agent = supervisor.repo.get_agent(agent.id).await?;
    let origin = supervisor.hermes_base_url(&current_agent).await?;
    let mut context = json!({});
    supervisor
        .bind_container_dispatch(&current_agent, &mut context)
        .await?;
    let token = crate::agent_runtime_token(&supervisor.config, agent.id)?;
    if origin != intent.origin
        || context != intent.runtime_context
        || hermes_wire::credential_fingerprint(&token) != intent.credential_fingerprint
    {
        return Err(AppError::conflict("PM original runtime context changed"));
    }
    Ok(origin)
}

fn validate_pm_binding(
    value: Value,
    reservation: &PmRunReservation,
    run_ref: &str,
) -> Result<String, AppError> {
    // PMResponse's result is compared to the complete initial snapshot, not a loose ok flag.
    let expected = json!({"contract_version":1,"identity":reservation.identity,"state":"active","version":1,"fence":1,
        "session_run_id":reservation.session_run_id,"binding_ref":reservation.binding_ref,"hermes_run_ref":run_ref,
        "checkpoint":null,"resume_operation_key":null,"resume_session_run_id":null,"terminal_readback":null,
        "workflow_step_allowed":true,"resume_delivered":false});
    let object = value.as_object().ok_or_else(unavailable)?;
    if object.len() != 3 || value["ok"] != true || value["result"] != expected {
        return Err(unavailable());
    }
    let token = value["execution_token"]
        .as_str()
        .filter(|token| hex_ref(token, 64))
        .ok_or_else(unavailable)?;
    Ok(token.into())
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FirstStep {
    ok: bool,
    exit_code: i64,
    output: String,
    result: FirstStepResult,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FirstStepResult {
    ok: bool,
    task_key: String,
    phase_code: String,
    status: String,
    instructions: String,
    phase_contract: Value,
    workflow_id: i64,
    mode_id: i64,
    mode_key: String,
    cycle_number: i64,
}

fn validate_first_step(
    value: Value,
    reservation: &PmRunReservation,
    workflow_mode: (i64, i64),
) -> Result<String, AppError> {
    let value: FirstStep = decode(value)?;
    let result = value.result;
    if !value.ok
        || value.exit_code != 0
        || !result.ok
        || result.task_key != reservation.identity.task
        || result.phase_code != "PM-DRAFT-01"
        || result.mode_key != "draft"
        || result.status != "active"
        || result.cycle_number != 0
        || result.instructions.is_empty()
        || value.output != result.instructions
        || (result.workflow_id, result.mode_id) != workflow_mode
        || !result.phase_contract.is_object()
    {
        return Err(unavailable());
    }
    Ok(result.instructions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assignment_fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../tests/fixtures/pm-dispatch/workflow-assign-response.json"
        ))
        .unwrap()
    }

    fn assignment_operation() -> PmDraftOperation {
        let wire: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/pm-dispatch/workflow-assign-request.json"
        ))
        .unwrap();
        let mut reservation: Value = serde_json::from_str(include_str!(
            "../../../api/tests/fixtures/pm-draft/reservation.json"
        ))
        .unwrap();
        reservation["input"]["sha256"] = wire["input_sha256"].clone();
        let binding = reservation["binding"].clone();
        serde_json::from_value(json!({"id":Uuid::from_u128(1),"owner_user_id":Uuid::from_u128(2),
            "owner_subject":binding["owner_subject"],"tracker_instance_id":binding["tracker_instance_id"],
            "project_id":binding["project_id"],"session_id":Uuid::from_u128(3),"credentials":null,
            "request":{"agent_id":wire["agent_ref"],"title":"Owner goal","description":"Exact original input",
                "idempotency_key":"owner-request"},
            "draft":{"tracker_instance_id":binding["tracker_instance_id"],"project_id":binding["project_id"],
                "task_id":binding["task_id"],"root_task_id":binding["root_task_id"],"owner_subject":binding["owner_subject"],
                "task_key":"PM-1","stage":"Draft"},
            "input":{"contract_version":1,"tracker_instance_id":binding["tracker_instance_id"],"project_id":binding["project_id"],
                "task_id":binding["task_id"],"root_task_id":binding["root_task_id"],"owner_subject":binding["owner_subject"],
                "input":{"snapshot_ref":wire["input_snapshot_ref"],"sha256":wire["input_sha256"],
                    "title":"Owner goal","description":"Exact original input"}},"reservation":reservation}))
        .unwrap()
    }

    #[test]
    fn draft_assignment_decoder_requires_exact_producer_fields_including_nulls() {
        let operation = assignment_operation();
        let reservation = domain::initial_pm_reservation(&operation).unwrap();
        let valid = assignment_fixture();
        decode::<AssignmentResponse>(valid.clone())
            .unwrap()
            .validate(&operation, &reservation, None)
            .unwrap();
        for field in [
            "work_item_ref",
            "work_item_revision",
            "queue_item_ref",
            "task_workspace_ref",
            "workspace_revision",
            "tech_execution_workspace_ref",
            "tech_execution_attempt_ref",
            "decomposition_revision_ref",
            "workspace_generation",
            "binding_ref",
            "hermes_run_ref",
            "bind_operation_key",
            "concrete_agent_ref",
        ] {
            let mut missing = valid.clone();
            missing["result"].as_object_mut().unwrap().remove(field);
            assert!(decode::<AssignmentResponse>(missing).is_err(), "{field}");
        }
        let mut extra = valid;
        extra["result"]["assignment_shape"] = json!("business-pre-decomposition");
        assert!(decode::<AssignmentResponse>(extra).is_err());
    }

    #[test]
    fn draft_assignment_readback_rejects_identity_input_and_future_resource_drift() {
        let operation = assignment_operation();
        let reservation = domain::initial_pm_reservation(&operation).unwrap();
        let valid = assignment_fixture();
        for (pointer, wrong) in [
            ("/ok", json!(false)),
            ("/exit_code", json!(1)),
            ("/result/task_key", json!("SDLC-2")),
            ("/result/assignment_operation_key", json!("foreign")),
            ("/result/assignment_ref", json!(Uuid::from_u128(9))),
            ("/result/assignment_revision", json!(2)),
            ("/result/business_task_ref", json!(Uuid::from_u128(9))),
            ("/result/root_task_ref", json!(Uuid::from_u128(9))),
            ("/result/stage_revision", json!("2")),
            ("/result/role_key", json!("developer")),
            ("/result/workflow_key", json!("hermes-sdlc:developer")),
            ("/result/mode_key", json!("analysis")),
            ("/result/execution_scope", json!("technical")),
            ("/result/stage_key", json!("Draft")),
            ("/result/cycle_number", json!(1)),
            ("/result/attempt_number", json!(2)),
            ("/result/work_item_ref", json!("invented")),
            ("/result/work_item_revision", json!(0)),
            ("/result/queue_item_ref", json!("invented")),
            ("/result/task_workspace_ref", json!("invented")),
            ("/result/workspace_revision", json!(1)),
            ("/result/tech_execution_workspace_ref", json!("invented")),
            ("/result/tech_execution_attempt_ref", json!("invented")),
            ("/result/decomposition_revision_ref", json!("invented")),
            ("/result/workspace_generation", json!(1)),
            ("/result/lease_generation", Value::Null),
            ("/result/lease_generation", json!(2)),
            ("/result/exact_input_refs/0/kind", json!("original_input")),
            ("/result/exact_input_refs/0/ref", json!(Uuid::from_u128(9))),
            ("/result/exact_input_refs/0/hash", json!("f".repeat(64))),
            ("/result/exact_input_refs", json!([])),
            ("/result/status", json!("done")),
            ("/result/current_phase_code", json!("PM-DRAFT-02")),
        ] {
            let mut wrong_value = valid.clone();
            *wrong_value.pointer_mut(pointer).unwrap() = wrong;
            assert!(
                decode::<AssignmentResponse>(wrong_value)
                    .unwrap()
                    .validate(&operation, &reservation, None)
                    .is_err(),
                "{pointer}"
            );
        }
    }

    #[test]
    fn draft_bind_preserves_native_tuple_and_advanced_cursor_identity() {
        let operation = assignment_operation();
        let reservation = domain::initial_pm_reservation(&operation).unwrap();
        let mut valid = assignment_fixture();
        valid["result"]["binding_state"] = json!("bound");
        valid["result"]["binding_ref"] = json!(reservation.binding_ref);
        valid["result"]["hermes_run_ref"] = json!("run_pm");
        valid["result"]["bind_operation_key"] =
            json!(format!("fleet-pm-runtime-bind:{}", operation.id));
        valid["result"]["concrete_agent_ref"] = json!(reservation.identity.agent_ref);
        decode::<AssignmentResponse>(valid.clone())
            .unwrap()
            .validate(&operation, &reservation, Some("run_pm"))
            .unwrap();
        for (field, wrong) in [
            ("binding_state", json!("unbound")),
            ("binding_ref", json!("foreign")),
            ("hermes_run_ref", json!("run_foreign")),
            ("bind_operation_key", json!("foreign")),
            ("concrete_agent_ref", json!(Uuid::from_u128(9))),
        ] {
            let mut value = valid.clone();
            value["result"][field] = wrong;
            assert!(
                decode::<AssignmentResponse>(value)
                    .unwrap()
                    .validate(&operation, &reservation, Some("run_pm"))
                    .is_err(),
                "{field}"
            );
        }
        valid["result"]["current_phase_code"] = json!("PM-DRAFT-02");
        valid["result"]["status"] = json!("blocked");
        let cursor = decode::<AssignmentResponse>(valid).unwrap();
        cursor.validate_identity(&operation, &reservation).unwrap();
        assert!(
            cursor
                .validate(&operation, &reservation, Some("run_pm"))
                .is_err()
        );
    }

    fn reservation() -> PmRunReservation {
        serde_json::from_value(json!({"session_id":Uuid::from_u128(1),"session_run_id":Uuid::from_u128(2),
            "identity":{"task":"SDLC-1","execution_ref":Uuid::from_u128(3),"tracker_instance_ref":"tracker",
                "tracker_project_ref":Uuid::from_u128(4),"task_ref":Uuid::from_u128(5),"root_ref":Uuid::from_u128(5),
                "agent_ref":Uuid::from_u128(6),"assignment_operation_key":"assign:1","assignment_ref":Uuid::from_u128(7),"assignment_revision":1},
            "binding_ref":"fleet:pm:fixture","dispatch_operation_key":"pm-bind:1","checkpoint_ref":null,"fence":1})).unwrap()
    }

    fn binding(reservation: &PmRunReservation) -> Value {
        json!({"ok":true,"execution_token":"a".repeat(64),"result":{
            "contract_version":1,"identity":reservation.identity,"state":"active","version":1,"fence":1,
            "session_run_id":reservation.session_run_id,"binding_ref":reservation.binding_ref,"hermes_run_ref":"run_pm",
            "checkpoint":null,"resume_operation_key":null,"resume_session_run_id":null,"terminal_readback":null,
            "workflow_step_allowed":true,"resume_delivered":false}})
    }

    #[test]
    fn pm_bind_requires_the_complete_exact_initial_snapshot() {
        let reservation = reservation();
        let valid = binding(&reservation);
        assert_eq!(
            validate_pm_binding(valid.clone(), &reservation, "run_pm").unwrap(),
            "a".repeat(64)
        );
        for (pointer, wrong) in [
            ("/ok", json!(false)),
            ("/execution_token", json!("secret-not-a-scoped-token")),
            ("/result/identity/task", json!("SDLC-2")),
            ("/result/identity/execution_ref", json!(Uuid::from_u128(9))),
            ("/result/identity/tracker_instance_ref", json!("foreign")),
            (
                "/result/identity/tracker_project_ref",
                json!(Uuid::from_u128(9)),
            ),
            ("/result/identity/task_ref", json!(Uuid::from_u128(9))),
            ("/result/identity/root_ref", json!(Uuid::from_u128(9))),
            ("/result/identity/agent_ref", json!(Uuid::from_u128(9))),
            (
                "/result/identity/assignment_operation_key",
                json!("foreign"),
            ),
            ("/result/identity/assignment_ref", json!(Uuid::from_u128(9))),
            ("/result/identity/assignment_revision", json!(2)),
            ("/result/session_run_id", json!(Uuid::from_u128(9))),
            ("/result/hermes_run_ref", json!("run_foreign")),
            ("/result/state", json!("waiting")),
            ("/result/workflow_step_allowed", json!(false)),
            ("/result/fence", json!(2)),
            ("/result/checkpoint", json!({"checkpoint_ref":"stale"})),
            ("/result/resume_delivered", json!(true)),
        ] {
            let mut value = valid.clone();
            *value.pointer_mut(pointer).unwrap() = wrong;
            assert!(
                validate_pm_binding(value, &reservation, "run_pm").is_err(),
                "{pointer}"
            );
        }
        let mut extra = valid.clone();
        extra["native_authorization"] = json!(true);
        assert!(validate_pm_binding(extra, &reservation, "run_pm").is_err());
        let mut incomplete = valid;
        incomplete["result"]
            .as_object_mut()
            .unwrap()
            .remove("checkpoint");
        assert!(validate_pm_binding(incomplete, &reservation, "run_pm").is_err());
    }

    #[test]
    fn first_step_is_instructions_not_a_completion_or_resume_receipt() {
        let reservation = reservation();
        let valid = json!({"ok":true,"exit_code":0,"output":"Owner-reviewed phase instructions","result":{
            "ok":true,"task_key":"SDLC-1","phase_code":"PM-DRAFT-01","status":"active",
            "instructions":"Owner-reviewed phase instructions","phase_contract":{},"workflow_id":1,
            "mode_id":2,"mode_key":"draft","cycle_number":0}});
        assert!(validate_first_step(valid.clone(), &reservation, (1, 2)).is_ok());
        for (pointer, wrong) in [
            ("/result/task_key", json!("SDLC-2")),
            ("/result/phase_code", json!("PM-DRAFT-02")),
            ("/result/status", json!("done")),
            ("/result/mode_key", json!("analysis")),
            ("/result/instructions", Value::Null),
            ("/result/cycle_number", json!(1)),
            ("/output", json!("different")),
        ] {
            let mut value = valid.clone();
            *value.pointer_mut(pointer).unwrap() = wrong;
            assert!(
                validate_first_step(value, &reservation, (1, 2)).is_err(),
                "{pointer}"
            );
        }
        let mut extra = valid;
        extra["result"]["complete"] = json!(true);
        assert!(validate_first_step(extra, &reservation, (1, 2)).is_err());
    }

    #[test]
    fn workflow_capabilities_do_not_accept_catalog_credentials_or_base_pending() {
        let value = json!({"ok":true,"role_key":"project_manager","credential_kind":"assignment",
            "capabilities":["assign","bind","rebind"],"readiness":{"service":"ready","schema":"ready","catalog":"ready"},
            "source_provenance":{"schema_version":1,"source_revision":"a".repeat(40),"source_archive_sha256":"b".repeat(64),"runtime_bundle_sha256":"c".repeat(64)},
            "runtimeCompatibility":{"catalogVersion":2,"catalogRevision":"a".repeat(40),"catalogSha256":"d".repeat(64),
                "skillsRevision":"e".repeat(40),"skillsManifestSha256":"f".repeat(64),"capabilityRevision":"hermes-sdlc-runtime/v2","capabilitySha256":"0".repeat(64)},
            "pm_continuation":{"contract_version":1,"base_path":"/internal/runtime/v1/pm","commands":["assign","bind","resume","rebind","readback"],
                "terminal_proof":"configured-runtime-readback","dispatch_owner":"fleet","execution_token_header":"X-Workflow-Execution-Token"}});
        decode::<Capabilities>(value.clone())
            .unwrap()
            .validate("assignment")
            .unwrap();
        for (pointer, wrong) in [
            ("/credential_kind", json!("catalog")),
            ("/role_key", json!("developer")),
            ("/readiness/catalog", json!("not_ready")),
            ("/runtimeCompatibility/catalogVersion", json!(3)),
            (
                "/runtimeCompatibility/catalogRevision",
                json!("b".repeat(40)),
            ),
            ("/pm_continuation/dispatch_owner", json!("hermes")),
            ("/pm_continuation/commands", json!(["bind"])),
            (
                "/pm_continuation/commands",
                json!(["bind", "resume", "rebind", "readback"]),
            ),
            (
                "/pm_continuation/commands",
                json!([
                    "assign",
                    "bind",
                    "resume",
                    "rebind",
                    "readback",
                    "idle_prompt"
                ]),
            ),
        ] {
            let mut value = value.clone();
            *value.pointer_mut(pointer).unwrap() = wrong;
            assert!(
                decode::<Capabilities>(value)
                    .unwrap()
                    .validate("assignment")
                    .is_err(),
                "{pointer}"
            );
        }
        let mut runtime = value.clone();
        runtime["credential_kind"] = json!("runtime");
        runtime["capabilities"] = json!(["step", "history"]);
        runtime["pm_continuation"]["commands"] = json!(["checkpoint", "readback"]);
        decode::<Capabilities>(runtime)
            .unwrap()
            .validate("runtime")
            .unwrap();
        let mut extra = value;
        extra["custom_pre_model_barrier"] = json!(true);
        assert!(decode::<Capabilities>(extra).is_err());
    }

    #[test]
    fn native_guidance_ack_uses_the_existing_closed_three_field_contract() {
        let valid = json!({"object":"hermes.run.steer","run_id":"run_pm","accepted":true});
        let ack: NativeGuidanceAck = decode(valid.clone()).unwrap();
        assert_eq!(ack.run_id, "run_pm");
        assert!(ack.accepted);
        let mut extra = valid.clone();
        extra["complete"] = json!(true);
        assert!(decode::<NativeGuidanceAck>(extra).is_err());
        let mut missing = valid;
        missing.as_object_mut().unwrap().remove("accepted");
        assert!(decode::<NativeGuidanceAck>(missing).is_err());
    }
}
