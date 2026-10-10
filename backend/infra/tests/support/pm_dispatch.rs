use super::*;
use domain::{PmDispatchIntent, PmGuidancePermit};
use serde_json::json;

struct CurrentTracker;

#[async_trait::async_trait]
impl app::pm_draft::PmDraftTracker for CurrentTracker {
    async fn context(
        &self,
        op: &domain::PmDraftOperation,
    ) -> Result<domain::TrackerTaskContext, shared::AppError> {
        let binding = op.identity()?;
        Ok(serde_json::from_value(json!({"contract_version":1,
            "tracker_instance_id":binding.tracker_instance_id,"project_id":binding.project_id,
            "task_id":binding.task_id,"root_task_id":binding.root_task_id,"owner_subject":binding.owner_subject,
            "stage":"Draft","requirement_revision":null,"waiting_reason":null,
            "permissions":{"can_answer":false,"can_confirm":false},"assignment":op.reservation.as_ref().unwrap().assignment})).unwrap())
    }
    async fn verify_namespace(
        &self,
        op: &domain::PmDraftOperation,
        agent: &domain::Agent,
    ) -> Result<(), shared::AppError> {
        assert_eq!(op.request.agent_id, agent.id);
        Ok(())
    }
    async fn find_draft(
        &self,
        _: &domain::PmDraftOperation,
    ) -> Result<Option<domain::TrackerCreatedDraft>, shared::AppError> {
        unreachable!("dispatch must not recreate the task")
    }
    async fn create_draft(
        &self,
        _: &domain::PmDraftOperation,
    ) -> Result<domain::TrackerCreatedDraft, shared::AppError> {
        unreachable!("dispatch must not recreate the task")
    }
    async fn original_input(
        &self,
        op: &domain::PmDraftOperation,
    ) -> Result<domain::TrackerDraftInputReceipt, shared::AppError> {
        Ok(op.input.clone().unwrap())
    }
    async fn reservation(
        &self,
        op: &domain::PmDraftOperation,
    ) -> Result<domain::TrackerDraftReservationReadback, shared::AppError> {
        let result = op.reservation.clone().unwrap();
        Ok(domain::TrackerDraftReservationReadback {
            contract_version: 1,
            binding: op.identity()?,
            owner_version: result.owner_cas.version,
            current: Some(result.clone()),
            operation: Some(domain::TrackerDraftReservationOperation {
                idempotency_key: op.reservation_key(),
                request_sha256: domain::pm_canonical_hash(
                    &json!({"operation":"reserve_pm_draft","payload":op.reservation_body()}),
                ),
                result,
            }),
        })
    }
    async fn reserve_pm(
        &self,
        _: &domain::PmDraftOperation,
    ) -> Result<domain::TrackerPmDraftReservation, shared::AppError> {
        unreachable!("dispatch must retain the current assignment")
    }
}

fn workflow_capabilities(kind: &str) -> serde_json::Value {
    let assignment = kind == "assignment";
    json!({"ok":true,"role_key":"project_manager","credential_kind":kind,
        "capabilities":if assignment {json!(["assign","bind","rebind"])} else {json!(["step","history"])},
        "readiness":{"service":"ready","schema":"ready","catalog":"ready"},
        "source_provenance":{"schema_version":1,"source_revision":"a".repeat(40),"source_archive_sha256":"b".repeat(64),"runtime_bundle_sha256":"c".repeat(64)},
        "runtimeCompatibility":{"catalogVersion":2,"catalogRevision":"a".repeat(40),"catalogSha256":"d".repeat(64),
            "skillsRevision":"e".repeat(40),"skillsManifestSha256":"f".repeat(64),"capabilityRevision":"hermes-sdlc-runtime/v2","capabilitySha256":"0".repeat(64)},
        "pm_continuation":{"contract_version":1,"base_path":"/internal/runtime/v1/pm",
            "commands":if assignment {json!(["bind","resume","rebind","readback"])} else {json!(["checkpoint","readback"])},
            "terminal_proof":"configured-runtime-readback","dispatch_owner":"fleet","execution_token_header":"X-Workflow-Execution-Token"}})
}

fn workflow_assignment(request: &serde_json::Value) -> serde_json::Value {
    let mut result = request.clone();
    let map = result.as_object_mut().unwrap();
    let task = map.remove("task").unwrap();
    let key = map.remove("operation_key").unwrap();
    map.remove("runtime_compatibility").unwrap();
    map.remove("expected_revision").unwrap();
    map.remove("expected_status").unwrap();
    map.extend(json!({"task_key":task,"assignment_operation_key":key,"assignment_revision":1,
        "workflow_id":1,"mode_id":2,"current_phase_id":3,"current_phase_code":"PM-DRAFT-01",
        "current_phase_name":"Draft","status":"active","binding_state":"unbound",
        "task_workspace_ref":null,"workspace_revision":null,"tech_execution_workspace_ref":null,
        "tech_execution_attempt_ref":null,"decomposition_revision_ref":null,"workspace_generation":null,
        "lease_generation":null,"binding_ref":null,"hermes_run_ref":null,"bind_operation_key":null,
        "concrete_agent_ref":null}).as_object().unwrap().clone());
    result
}

struct AbortServer(tokio::task::JoinHandle<()>);
impl Drop for AbortServer {
    fn drop(&mut self) {
        self.0.abort();
    }
}

// Exercises the production service, not a parallel orchestration model. External services are
// controlled HTTP peers; actual Workflow/Tracker acceptance remains a hosted integration check.
#[tokio::test]
async fn pm_production_dispatch_binds_and_steers_once_and_holds_unknown_post() {
    use app::RuntimeSupervisor;
    use axum::{
        Json,
        http::{HeaderMap, Method, StatusCode, Uri},
        response::IntoResponse,
    };
    use tokio::sync::Mutex;
    for unknown in [false, true] {
        let (repo, op, _) = super::pm_draft_creation::credential_fixture()
            .await
            .expect("isolated PostgreSQL is required for PM dispatch tests");
        let reservation = domain::initial_pm_reservation(&op).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut config = AppConfig::default();
        config.pm.dispatch.enabled = true;
        config.pm.dispatch.assignment_token = "a".repeat(40);
        config.pm.dispatch.runtime_token = "r".repeat(40);
        config.pm.readback_token = "p".repeat(40);
        config.fleet.project_workflow_url = Some(format!("http://127.0.0.1:{port}"));
        config.fleet.runtime_token_secret = "test-only-native-runtime-secret".into();
        let native = format!(
            "Bearer {}",
            infra::agent_runtime_token(&config, op.request.agent_id).unwrap()
        );
        let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agents SET api_port=$2,workflow_id='1' WHERE id=$1",
            [op.request.agent_id.into(), i32::from(port).into()],
        ))
        .await
        .unwrap();
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let assigned = Arc::new(Mutex::new(None::<serde_json::Value>));
        let state = (
            calls.clone(),
            assigned,
            op.clone(),
            reservation.clone(),
            native,
        );
        let router = axum::Router::new().fallback(move |method: Method, uri: Uri, headers: HeaderMap, bytes: axum::body::Bytes| {
            let (calls, assigned, op, reservation, native) = state.clone();
            async move {
                let path = uri.path();
                calls.lock().await.push(format!("{method} {path}"));
                let body: serde_json::Value = if bytes.is_empty() { serde_json::Value::Null }
                    else { serde_json::from_slice(&bytes).unwrap() };
                let auth = headers["authorization"].to_str().unwrap();
                if path.starts_with("/internal/") {
                    let runtime = matches!(path, "/internal/runtime/step");
                    let kind = if auth == format!("Bearer {}", "a".repeat(40)) { "assignment" }
                        else { assert_eq!(auth, format!("Bearer {}", "r".repeat(40))); "runtime" };
                    assert_eq!(headers["cache-control"], "no-cache, no-store");
                    if path == "/internal/runtime/capabilities" {
                        assert_eq!(method, Method::GET);
                        return Json(workflow_capabilities(kind)).into_response();
                    }
                    assert_eq!(kind, if runtime {"runtime"} else {"assignment"});
                    assert_eq!(method, Method::POST);
                    if path == "/internal/runtime/assign" {
                        assert_eq!(body["operation_key"], reservation.identity.assignment_operation_key);
                        let mut value = assigned.lock().await;
                        let result = value.get_or_insert_with(|| workflow_assignment(&body)).clone();
                        return Json(json!({"ok":true,"exit_code":0,"result":result})).into_response();
                    }
                    if path == "/internal/runtime/bind" {
                        assert_eq!(body["hermes_run_ref"], "run_pm");
                        let mut value = assigned.lock().await;
                        let result = value.as_mut().unwrap();
                        for field in ["binding_ref","hermes_run_ref","bind_operation_key","concrete_agent_ref"] {
                            result[field] = body[field].clone();
                        }
                        result["binding_state"] = json!("bound");
                        return Json(json!({"ok":true,"exit_code":0,"result":result})).into_response();
                    }
                    if path == "/internal/runtime/v1/pm/bind" {
                        let mut expected = serde_json::to_value(&reservation.identity).unwrap();
                        expected.as_object_mut().unwrap().extend(json!({"operation_key":reservation.dispatch_operation_key,
                            "expected_version":0,"binding_ref":reservation.binding_ref,"hermes_run_ref":"run_pm",
                            "session_run_id":reservation.session_run_id}).as_object().unwrap().clone());
                        assert_eq!(body, expected);
                        return Json(json!({"ok":true,"execution_token":"b".repeat(64),"result":{
                            "contract_version":1,"identity":reservation.identity,"state":"active","version":1,"fence":1,
                            "session_run_id":reservation.session_run_id,"binding_ref":reservation.binding_ref,"hermes_run_ref":"run_pm",
                            "checkpoint":null,"resume_operation_key":null,"resume_session_run_id":null,"terminal_readback":null,
                            "workflow_step_allowed":true,"resume_delivered":false}})).into_response();
                    }
                    assert_eq!(path, "/internal/runtime/step");
                    assert_eq!(headers["x-workflow-execution-token"], "b".repeat(64));
                    assert!(body.get("report").is_none());
                    assert_eq!(body["step_operation_key"], format!("fleet-pm-first-step:{}", op.id));
                    return Json(json!({"ok":true,"exit_code":0,"output":"Verified PM instructions","result":{
                        "ok":true,"task_key":reservation.identity.task,"phase_code":"PM-DRAFT-01","status":"active",
                        "instructions":"Verified PM instructions","phase_contract":{},"workflow_id":1,
                        "mode_id":2,"mode_key":"draft","cycle_number":0}})).into_response();
                }
                assert_eq!(auth, native);
                match path {
                    "/health" => Json(json!({"status":"ok"})).into_response(),
                    "/v1/capabilities" => {
                        let mut caps = super::hermes_protocol_fixture::capabilities();
                        caps["features"]["run_steer"] = json!(true);
                        caps["endpoints"]["run_steer"] = json!({"method":"POST","path":"/v1/runs/{run_id}/steer"});
                        Json(caps).into_response()
                    }
                    "/v1/runs" => {
                        assert_eq!(method, Method::POST);
                        assert_eq!(headers["idempotency-key"], op.id.to_string());
                        assert_eq!(body.as_object().unwrap().len(), 2);
                        assert_eq!(body["session_id"], reservation.runtime_session_id());
                        assert!(body["input"].as_str().unwrap().contains(&op.request.description));
                        if unknown {
                            return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({"error":"controlled unknown ACK"}))).into_response();
                        }
                        (StatusCode::ACCEPTED, Json(json!({"run_id":"run_pm","status":"started","replayed":false}))).into_response()
                    }
                    "/v1/runs/run_pm" => {
                        assert_eq!(method, Method::GET);
                        Json(json!({"object":"hermes.run","run_id":"run_pm","session_id":"native-session","status":"running"})).into_response()
                    }
                    "/v1/runs/run_pm/steer" => {
                        assert_eq!(method, Method::POST);
                        assert_eq!(body, json!({"input":"Verified PM instructions"}));
                        Json(json!({"object":"hermes.run.steer","run_id":"run_pm","accepted":true})).into_response()
                    }
                    _ => panic!("unexpected PM HTTP route"),
                }
            }
        });
        let _server = AbortServer(tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap()
        }));
        let repo = Arc::new(repo);
        let (events, _) = tokio::sync::broadcast::channel(32);
        let supervisor =
            infra::runtime::LocalRuntimeSupervisor::new(Arc::new(config), repo.clone(), events);
        for _ in 0..2 {
            let result = tokio::time::timeout(
                Duration::from_secs(10),
                supervisor.dispatch_pm_draft(&op, &CurrentTracker),
            )
            .await
            .unwrap();
            assert_eq!(result.is_err(), unknown);
        }
        let calls = calls.lock().await;
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.as_str() == "POST /v1/runs")
                .count(),
            1
        );
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.as_str() == "POST /v1/runs/run_pm/steer")
                .count(),
            usize::from(!unknown)
        );
        if !unknown {
            let at = |call: &str| calls.iter().position(|actual| actual == call).unwrap();
            assert!(at("POST /internal/runtime/assign") < at("POST /v1/runs"));
            assert!(at("GET /v1/runs/run_pm") < at("POST /internal/runtime/bind"));
            assert!(at("POST /internal/runtime/v1/pm/bind") < at("POST /internal/runtime/step"));
            assert!(at("POST /internal/runtime/step") < at("POST /v1/runs/run_pm/steer"));
            assert!(
                repo.get_pm_run(op.id)
                    .await
                    .unwrap()
                    .terminal_status
                    .is_none()
            );
        } else {
            let intent = repo.get_pm_dispatch(op.id).await.unwrap().unwrap();
            assert!(intent.submitted && intent.hermes_run_ref.is_none());
            assert!(
                !calls
                    .iter()
                    .any(|call| call == "POST /internal/runtime/bind")
            );
        }
    }
}

async fn setup() -> (
    PostgresFleetRepository,
    domain::PmRunReservation,
    PmDispatchIntent,
) {
    let (repo, reservation) = pm_fixture()
        .await
        .expect("isolated PostgreSQL is required for PM custody tests");
    repo.reserve_pm_run(reservation.clone()).await.unwrap();
    let intent = PmDispatchIntent {
        session_run_id: reservation.session_run_id,
        origin: "http://127.0.0.1:23810".into(),
        credential_fingerprint: "a".repeat(64),
        request_body: serde_json::to_string(
            &json!({"input":"Original owner task","session_id":reservation.runtime_session_id()}),
        )
        .unwrap(),
        workflow_assignment: json!({"operation_key":reservation.identity.assignment_operation_key}),
        workflow_origin: "http://workflow.test".into(),
        workflow_credential_fingerprint: "b".repeat(64),
        runtime_context: json!({}),
        submitted: false,
        hermes_run_ref: None,
    };
    repo.prepare_pm_dispatch(intent.clone()).await.unwrap();
    (repo, reservation, intent)
}

#[tokio::test]
async fn pm_submission_is_one_shot_across_concurrent_claims_and_repository_restart() {
    let (repo, reservation, intent) = setup().await;
    let id = reservation.session_run_id;
    let (left, right) = tokio::join!(repo.claim_pm_submission(id), repo.claim_pm_submission(id));
    assert_ne!(left.unwrap(), right.unwrap());
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let restarted = PostgresFleetRepository::new(db.clone());
    assert!(!restarted.claim_pm_submission(id).await.unwrap());
    let unknown = restarted.prepare_pm_dispatch(intent.clone()).await.unwrap();
    assert!(unknown.submitted);
    assert!(unknown.hermes_run_ref.is_none());
    assert!(
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE pm_dispatch_journal SET submitted=false WHERE session_run_id=$1",
            [id.into()]
        ))
        .await
        .is_err()
    );
    let mut changed = intent;
    changed.request_body = changed.request_body.replace("Original", "Changed");
    assert!(restarted.prepare_pm_dispatch(changed).await.is_err());
    restarted
        .record_pm_submission(id, "run_pm_original".into())
        .await
        .unwrap();
    assert!(
        restarted
            .record_pm_submission(id, "run_foreign".into())
            .await
            .is_err()
    );
    let accepted_ack = restarted.get_pm_dispatch(id).await.unwrap().unwrap();
    assert_eq!(
        accepted_ack.hermes_run_ref.as_deref(),
        Some("run_pm_original")
    );
    assert!(!restarted.claim_pm_submission(id).await.unwrap());
    assert!(
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "DELETE FROM pm_dispatch_journal WHERE session_run_id=$1",
            [id.into()]
        ))
        .await
        .is_err()
    );
}

#[tokio::test]
async fn pm_drain_and_owner_revocation_block_the_unconsumed_submission() {
    let (repo, reservation, _) = setup().await;
    let id = reservation.session_run_id;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let agent = reservation.identity.agent_id().unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO agent_config_heads(agent_id,desired_revision,draining) VALUES($1,1,true)",
        [agent.into()],
    ))
    .await
    .unwrap();
    assert!(!repo.claim_pm_submission(id).await.unwrap());
    assert!(!repo.get_pm_dispatch(id).await.unwrap().unwrap().submitted);
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agent_config_heads SET draining=false WHERE agent_id=$1",
        [agent.into()],
    ))
    .await
    .unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE users SET is_active=false WHERE id=(SELECT user_id FROM agent_sessions WHERE id=$1)", [reservation.session_id.into()])).await.unwrap();
    assert!(!repo.claim_pm_submission(id).await.unwrap());
    assert!(!repo.get_pm_dispatch(id).await.unwrap().unwrap().submitted);
}

#[tokio::test]
async fn pm_guidance_unknown_is_not_retried_and_delivered_ack_does_not_complete_the_run() {
    let (repo, reservation, _) = setup().await;
    let id = reservation.session_run_id;
    assert!(repo.claim_pm_submission(id).await.unwrap());
    repo.record_pm_submission(id, "run_pm".into())
        .await
        .unwrap();
    repo.accept_pm_run(id, "run_pm".into(), "native-session".into())
        .await
        .unwrap();
    let guidance = r#"{"input":"Verified Workflow instructions"}"#.to_string();
    assert!(matches!(
        repo.claim_pm_guidance(id, guidance.clone()).await.unwrap(),
        PmGuidancePermit::Claimed
    ));
    assert!(matches!(
        repo.claim_pm_guidance(id, guidance.clone()).await.unwrap(),
        PmGuidancePermit::Unknown
    ));
    assert!(
        repo.claim_pm_guidance(id, r#"{"input":"changed"}"#.into())
            .await
            .is_err()
    );
    repo.finish_pm_guidance(id).await.unwrap();
    assert!(matches!(
        repo.claim_pm_guidance(id, guidance).await.unwrap(),
        PmGuidancePermit::Delivered
    ));
    let record = repo.get_pm_run(id).await.unwrap();
    assert!(record.terminal_status.is_none());
    assert_eq!(
        repo.get_session_agent_run(id).await.unwrap().state,
        SessionRunState::Running
    );
}
