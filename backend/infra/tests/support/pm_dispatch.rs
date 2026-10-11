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
            "commands":if assignment {json!(["assign","bind","resume","rebind","readback"])} else {json!(["checkpoint","readback"])},
            "terminal_proof":"configured-runtime-readback","dispatch_owner":"fleet","execution_token_header":"X-Workflow-Execution-Token"}})
}

pub(super) fn workflow_assignment(request: &serde_json::Value) -> serde_json::Value {
    json!({"task_key":request["task"],"assignment_operation_key":request["assignment_operation_key"],
        "assignment_revision":request["assignment_revision"],
        "workflow_id":1,"mode_id":2,"current_phase_id":3,"current_phase_code":"PM-DRAFT-01",
        "current_phase_name":"Draft","status":"active","binding_state":"unbound",
        "workflow_key":"hermes-sdlc:project_manager","mode_key":"draft","cycle_number":0,"attempt_number":1,
        "role_key":"project_manager","execution_scope":"business","stage_key":"draft",
        "business_task_ref":request["task_ref"],"root_task_ref":request["root_ref"],
        "assignment_ref":request["assignment_ref"],"stage_revision":request["owner_version"].as_u64().unwrap().to_string(),
        "work_item_ref":null,"work_item_revision":null,"queue_item_ref":null,
        "task_workspace_ref":null,"workspace_revision":null,"tech_execution_workspace_ref":null,
        "tech_execution_attempt_ref":null,"decomposition_revision_ref":null,"workspace_generation":null,
        "lease_generation":1,"binding_ref":null,"hermes_run_ref":null,"bind_operation_key":null,
        "concrete_agent_ref":null,"exact_input_refs":[{"kind":"pm_draft_input",
            "ref":request["input_snapshot_ref"],"hash":request["input_sha256"]}]})
}

pub(super) struct AbortServer(pub(super) tokio::task::JoinHandle<()>);
impl Drop for AbortServer {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(super) struct ToolHome(std::path::PathBuf);
impl Drop for ToolHome {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove owned PM fixture home");
    }
}

// Materialize the effective profile in an owned temporary home, without starting a runtime.
pub(super) async fn install_tool_home(
    repo: &PostgresFleetRepository,
    op: &domain::PmDraftOperation,
    config: &mut AppConfig,
) -> ToolHome {
    use sha2::{Digest, Sha256};
    let root = std::env::temp_dir().join(format!("fleet-pm-tools-test-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    config.fleet.agents_root = root.to_string_lossy().into_owned();
    let agent = repo.get_agent(op.request.agent_id).await.unwrap();
    let home = root.join(&agent.name);
    let config_path = home.join("config");
    let workspace = home.join("workspace");
    std::fs::create_dir_all(&config_path).unwrap();
    std::fs::create_dir_all(&workspace).unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE agents SET runtime_path=$2,config_path=$3,workspace_path=$4,logs_path=$5 WHERE id=$1",
        [agent.id.into(),home.join("runtime").to_string_lossy().into_owned().into(),config_path.to_string_lossy().into_owned().into(),
         workspace.to_string_lossy().into_owned().into(),home.join("logs").to_string_lossy().into_owned().into()])).await.unwrap();
    std::fs::write(
        home.join(".fleet-agent.json"),
        json!({"id":agent.id,"name":agent.name,"kind":"hermes"}).to_string(),
    )
    .unwrap();
    let origin = config
        .pm
        .dispatch
        .tool_origin
        .as_ref()
        .unwrap()
        .trim_end_matches('/');
    let yaml = json!({"model":"test-model","terminal":{"cwd":workspace.to_string_lossy()},
        "mcp_servers":{"fleet_pm":{"url":format!("{origin}/internal/runtime/v1/pm/agents/{}/mcp",agent.id),
        "transport":"http","headers":{"Authorization":"Bearer ${API_SERVER_KEY}","MCP-Protocol-Version":"2025-03-26"},"strict_redirect_headers":true,"trust":"full"}},
        "platform_toolsets":{"api_server":["fleet_pm"]}});
    let env = format!(
        "HERMES_HOME={}\nHERMES_SERVE_HEADLESS=1\nAPI_SERVER_ENABLED=true\nAPI_SERVER_KEY={}\n",
        serde_json::to_string(&config_path.to_string_lossy()).unwrap(),
        infra::agent_runtime_token(config, agent.id).unwrap()
    );
    let files = [
        ("config.yaml", serde_json::to_string_pretty(&yaml).unwrap()),
        ("SOUL.md", "# Fixture PM\n".into()),
        (".env", env),
    ];
    let mut hashes = serde_json::Map::new();
    for (name, body) in files {
        hashes.insert(
            name.into(),
            json!(hex::encode(Sha256::digest(body.as_bytes()))),
        );
        std::fs::write(config_path.join(name), body).unwrap();
    }
    std::fs::write(
        config_path.join(".fleet-config-revision.json"),
        json!({"agent_id":agent.id,"revision":1,"hashes":hashes}).to_string(),
    )
    .unwrap();
    let snapshot = json!({"config":{"config_json":{"model":"test-model"},"soul_md":"# Fixture PM\n","env_json":{}},"skills":[]});
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agent_config_revisions(agent_id,revision,state,snapshot,created_by_user_id) VALUES($1,1,'active',$2,$3)",
        [agent.id.into(),snapshot.into(),op.owner_user_id.into()])).await.unwrap();
    db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agent_config_heads(agent_id,desired_revision,effective_revision,draining) VALUES($1,1,1,false)",[agent.id.into()])).await.unwrap();
    ToolHome(root)
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
        config.pm.dispatch.tool_origin = Some(format!("http://127.0.0.1:{port}"));
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
        let _home = install_tool_home(&repo, &op, &mut config).await;
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
                    if path == "/internal/runtime/v1/pm/assign" {
                        let mut expected = serde_json::to_value(&reservation.identity).unwrap();
                        let input = &op.input.as_ref().unwrap().input;
                        expected.as_object_mut().unwrap().extend(json!({"owner_version":1,
                            "input_snapshot_ref":input.snapshot_ref,"input_sha256":input.sha256,
                            "runtime_compatibility":workflow_capabilities("assignment")["runtimeCompatibility"]}).as_object().unwrap().clone());
                        assert_eq!(body, expected);
                        assert_eq!(body["assignment_operation_key"], reservation.identity.assignment_operation_key);
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
                    "/v1/runs/run_pm/events" => {
                        assert_eq!(method, Method::GET);
                        ([("content-type", "text/event-stream")], "").into_response()
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
            let proof = repo
                .get_pm_tool(op.id, &format!("fleet-pm-first-step:{}", op.id))
                .await
                .unwrap()
                .unwrap();
            assert_eq!(proof.kind, "workflow_step");
            assert!(proof.attempted);
            assert_eq!(proof.request["body"]["hermes_run_ref"], "run_pm");
            assert_eq!(
                proof.result.as_ref().unwrap()["result"]["instructions"],
                "Verified PM instructions"
            );
            let at = |call: &str| calls.iter().position(|actual| actual == call).unwrap();
            assert!(at("POST /internal/runtime/v1/pm/assign") < at("POST /v1/runs"));
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
            assert!(
                repo.get_pm_tool(op.id, &format!("fleet-pm-first-step:{}", op.id))
                    .await
                    .unwrap()
                    .is_none()
            );
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

fn instruction_receipt_fixture(
    reservation: &domain::PmRunReservation,
    run_ref: &str,
    key: &str,
) -> (domain::PmToolCommand, serde_json::Value) {
    let body = json!({"task":reservation.identity.task,"step_operation_key":key,
        "assignment_revision":reservation.identity.assignment_revision,"assignment_ref":reservation.identity.assignment_ref,
        "binding_ref":reservation.binding_ref,"hermes_run_ref":run_ref,"mode_key":"draft","cycle_number":0,"attempt_number":1,
        "expected_phase_code":"PM-DRAFT-01","expected_status":"active","session_run_id":reservation.session_run_id});
    (
        domain::PmToolCommand {
            session_run_id: reservation.session_run_id,
            key: key.into(),
            kind: "workflow_step".into(),
            request: json!({"caller":{"step_operation_key":key,"report":null},"body":body,"workflow_id":1,"mode_id":2}),
            attempted: false,
            result: None,
        },
        json!({"ok":true,"exit_code":0,"output":"Fixture instructions","result":{"ok":true,
        "task_key":reservation.identity.task,"phase_code":"PM-DRAFT-01","status":"active",
        "instructions":"Fixture instructions","phase_contract":{},"workflow_id":1,"mode_id":2,"mode_key":"draft","cycle_number":0}}),
    )
}

#[tokio::test]
async fn pm_publication_claim_requires_exact_run_instruction_receipt() {
    let (repo, reservation, _) = setup().await;
    let id = reservation.session_run_id;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET workflow_id='1' WHERE id=$1",
        [reservation.identity.agent_id().unwrap().into()],
    ))
    .await
    .unwrap();
    assert!(repo.claim_pm_submission(id).await.unwrap());
    repo.record_pm_submission(id, "run_instruction".into())
        .await
        .unwrap();
    repo.accept_pm_run(id, "run_instruction".into(), "native-session".into())
        .await
        .unwrap();
    for kind in ["revision", "question"] {
        repo.prepare_pm_tool(domain::PmToolCommand {
            session_run_id: id,
            key: kind.into(),
            kind: kind.into(),
            request: json!({"original":true}),
            attempted: false,
            result: None,
        })
        .await
        .unwrap();
        assert!(!repo.claim_pm_tool(id, kind).await.unwrap());
    }
    for (n, path, value) in [
        (0, "/body/session_run_id", json!(Uuid::new_v4())),
        (1, "/body/hermes_run_ref", json!("run_foreign")),
        (2, "/body/binding_ref", json!("foreign-binding")),
        (3, "/body/assignment_ref", json!(Uuid::new_v4())),
        (4, "/body/assignment_revision", json!(999)),
        (5, "/body/task", json!("FOREIGN-1")),
        (6, "/workflow_id", json!(99)),
    ] {
        let (mut proof, result) =
            instruction_receipt_fixture(&reservation, "run_instruction", &format!("wrong-{n}"));
        *proof.request.pointer_mut(path).unwrap() = value;
        repo.prepare_pm_tool(proof.clone()).await.unwrap();
        assert!(repo.claim_pm_tool(id, &proof.key).await.unwrap());
        repo.finish_pm_tool(id, &proof.key, result).await.unwrap();
        for kind in ["revision", "question"] {
            assert!(!repo.claim_pm_tool(id, kind).await.unwrap());
        }
    }
    for (n, path, value) in [
        (0, "/ok", json!(false)),
        (1, "/exit_code", json!(1)),
        (2, "/result/task_key", json!("FOREIGN-1")),
        (3, "/result/instructions", json!("")),
        (4, "/output", json!("foreign output")),
        (5, "/result/phase_contract", serde_json::Value::Null),
    ] {
        let (proof, mut result) = instruction_receipt_fixture(
            &reservation,
            "run_instruction",
            &format!("wrong-result-{n}"),
        );
        *result.pointer_mut(path).unwrap() = value;
        repo.prepare_pm_tool(proof.clone()).await.unwrap();
        assert!(repo.claim_pm_tool(id, &proof.key).await.unwrap());
        repo.finish_pm_tool(id, &proof.key, result).await.unwrap();
        for kind in ["revision", "question"] {
            assert!(!repo.claim_pm_tool(id, kind).await.unwrap());
        }
    }
    let (proof, result) =
        instruction_receipt_fixture(&reservation, "run_instruction", "read-instructions");
    repo.prepare_pm_tool(proof.clone()).await.unwrap();
    assert!(repo.claim_pm_tool(id, &proof.key).await.unwrap());
    for kind in ["revision", "question"] {
        assert!(!repo.claim_pm_tool(id, kind).await.unwrap());
    }
    let restarted = PostgresFleetRepository::new(
        sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap(),
    );
    restarted
        .finish_pm_tool(id, &proof.key, result.clone())
        .await
        .unwrap();
    for kind in ["revision", "question"] {
        let (left, right) = tokio::join!(
            repo.claim_pm_tool(id, kind),
            restarted.claim_pm_tool(id, kind)
        );
        assert_ne!(
            left.unwrap(),
            right.unwrap(),
            "one original publication claim even across repository restart"
        );
        assert!(!repo.claim_pm_tool(id, kind).await.unwrap());
    }
    let mut changed = proof.clone();
    changed.request["body"]["expected_status"] = json!("blocked");
    assert!(repo.prepare_pm_tool(changed).await.is_err());
    assert!(
        repo.finish_pm_tool(
            id,
            &proof.key,
            json!({"supervisor_accepted":true,"exit_code":0})
        )
        .await
        .is_err()
    );
    assert_eq!(
        repo.get_pm_tool(id, &proof.key)
            .await
            .unwrap()
            .unwrap()
            .result,
        Some(result)
    );
}

#[tokio::test]
async fn pm_submission_is_one_shot_across_concurrent_claims_and_repository_restart() {
    let (repo, reservation, intent) = setup().await;
    let id = reservation.session_run_id;
    let (left, right) = tokio::join!(repo.claim_pm_submission(id), repo.claim_pm_submission(id));
    assert_ne!(left.unwrap(), right.unwrap());
    let url = std::env::var("FLEET_TEST_DATABASE_URL").unwrap();
    let db = sea_orm::Database::connect(&url).await.unwrap();
    let restarted = PostgresFleetRepository::new(sea_orm::Database::connect(&url).await.unwrap());
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
    // Exercise the database CHECK itself, not the Rust pre-validation, without consuming the ACK.
    for (run_ref, valid) in [
        ("r".to_string(), true),
        ("r".repeat(255), true),
        ("r".repeat(256), true),
        ("r".repeat(512), true),
        (String::new(), false),
        ("r".repeat(513), false),
        ("run/slash".to_string(), false),
        ("run.dot".to_string(), false),
        ("run space".to_string(), false),
        ("run\n".to_string(), false),
        ("run\u{e9}".to_string(), false),
    ] {
        let txn = db.begin().await.unwrap();
        let result = txn
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE pm_dispatch_journal SET hermes_run_ref=$2 WHERE session_run_id=$1 AND submitted",
                [id.into(), run_ref.into()],
            ))
            .await;
        assert_eq!(result.is_ok(), valid, "PM ACK character and length guard");
        if let Ok(result) = result {
            assert_eq!(result.rows_affected(), 1);
        }
        txn.rollback().await.unwrap();
        let pending = restarted.get_pm_dispatch(id).await.unwrap().unwrap();
        assert!(pending.submitted && pending.hermes_run_ref.is_none());
    }
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
    for stage in ["reserved", "prepared", "submitted"] {
        let (repo, reservation) = pm_fixture()
            .await
            .expect("isolated PostgreSQL is required for PM activation tests");
        repo.reserve_pm_run(reservation.clone()).await.unwrap();
        let id = reservation.session_run_id;
        let agent = reservation.identity.agent_id().unwrap();
        let owner = repo
            .get_session(reservation.session_id)
            .await
            .unwrap()
            .user_id;
        let intent = PmDispatchIntent {
            session_run_id: id,
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
        if stage != "reserved" {
            repo.prepare_pm_dispatch(intent.clone()).await.unwrap();
        }
        if stage == "submitted" {
            assert!(repo.claim_pm_submission(id).await.unwrap());
        }
        let draft = repo
            .create_config_revision(agent, configuration(), owner)
            .await
            .unwrap();
        let validated = repo
            .validate_config_revision(agent, draft.revision, vec![])
            .await
            .unwrap();
        assert_eq!(validated.state, "validated");
        assert!(!validated.draining);
        let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        let snapshot_sql = "SELECT jsonb_build_object(
            'head',(SELECT to_jsonb(h) FROM agent_config_heads h WHERE h.agent_id=$1),
            'revision',(SELECT to_jsonb(c) FROM agent_config_revisions c WHERE c.agent_id=$1 AND c.revision=$3),
            'run',(SELECT to_jsonb(r) FROM session_agent_runs r WHERE r.id=$2),
            'binding',(SELECT to_jsonb(b) FROM pm_run_bindings b WHERE b.session_run_id=$2),
            'journal',(SELECT to_jsonb(j) FROM pm_dispatch_journal j WHERE j.session_run_id=$2),
            'audit',(SELECT COALESCE(jsonb_agg(to_jsonb(a) ORDER BY a.id),'[]'::jsonb)
                FROM audit_log a WHERE a.entity_type='agent_config' AND a.entity_id=$1::text)
            ) AS snapshot";
        let before: serde_json::Value = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                snapshot_sql,
                [agent.into(), id.into(), draft.revision.into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "snapshot")
            .unwrap();
        assert_eq!(before["run"]["state"], "pending");
        assert!(before["binding"]["hermes_run_ref"].is_null());
        assert_eq!(before["journal"].is_null(), stage == "reserved");
        assert!(matches!(
            repo.request_config_activation(agent, draft.revision, owner)
                .await,
            Err(shared::AppError::Conflict(_))
        ));
        let after: serde_json::Value = db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                snapshot_sql,
                [agent.into(), id.into(), draft.revision.into()],
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get("", "snapshot")
            .unwrap();
        assert_eq!(before, after, "activation mutated {stage} PM custody");
        assert!(!repo.agent_is_draining(agent).await.unwrap());
        assert_eq!(
            repo.get_pm_dispatch(id).await.unwrap().is_none(),
            stage == "reserved"
        );
        let original = repo.prepare_pm_dispatch(intent.clone()).await.unwrap();
        assert_eq!(original.request_body, intent.request_body);
        assert_eq!(original.submitted, stage == "submitted");
        assert!(original.hermes_run_ref.is_none());
        assert_eq!(
            repo.claim_pm_submission(id).await.unwrap(),
            stage != "submitted"
        );
        assert!(!repo.claim_pm_submission(id).await.unwrap());
        repo.record_pm_submission(id, "run_config_drain".into())
            .await
            .unwrap();
        repo.accept_pm_run(id, "run_config_drain".into(), "native-session".into())
            .await
            .unwrap();
        assert_eq!(
            repo.get_session_agent_run(id).await.unwrap().state,
            SessionRunState::Running
        );
        repo.request_config_activation(agent, draft.revision, owner)
            .await
            .unwrap();
        assert!(repo.agent_is_draining(agent).await.unwrap());
        assert!(repo.claim_config_activation().await.unwrap().is_none());
        repo.observe_pm_run(id, domain::PmRuntimeStatus::Completed)
            .await
            .unwrap();
        let claimed = repo.claim_config_activation().await.unwrap().unwrap();
        assert_eq!(claimed.agent_id, agent);
        assert_eq!(claimed.revision, draft.revision);
        repo.finish_config_activation(agent, draft.revision, None, true)
            .await
            .unwrap();
        assert!(!repo.agent_is_draining(agent).await.unwrap());
        db.close().await.unwrap();
    }
    let (repo, reservation, _) = setup().await;
    let id = reservation.session_run_id;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let agent = reservation.identity.agent_id().unwrap();
    let owner = repo
        .get_session(reservation.session_id)
        .await
        .unwrap()
        .user_id;
    repo.create_config_revision(agent, configuration(), owner)
        .await
        .unwrap();
    let drained = db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agent_config_heads SET draining=true WHERE agent_id=$1",
            [agent.into()],
        ))
        .await
        .unwrap();
    assert_eq!(drained.rows_affected(), 1);
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

#[tokio::test]
async fn pm_claims_complete_with_one_physical_connection_without_second_pool_acquisition() {
    let (_, reservation, _) = setup().await;
    let id = reservation.session_run_id;
    let mut options =
        sea_orm::ConnectOptions::new(std::env::var("FLEET_TEST_DATABASE_URL").unwrap());
    options
        .max_connections(1)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(1));
    let db = sea_orm::Database::connect(options).await.unwrap();
    db.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "UPDATE agents SET workflow_id='1' WHERE id=$1",
        [reservation.identity.agent_id().unwrap().into()],
    ))
    .await
    .unwrap();
    let repo = PostgresFleetRepository::new(db);
    assert!(
        tokio::time::timeout(Duration::from_secs(2), repo.claim_pm_submission(id))
            .await
            .unwrap()
            .unwrap()
    );
    assert!(
        !tokio::time::timeout(Duration::from_secs(2), repo.claim_pm_submission(id))
            .await
            .unwrap()
            .unwrap()
    );
    repo.record_pm_submission(id, "run_single_pool".into())
        .await
        .unwrap();
    repo.accept_pm_run(id, "run_single_pool".into(), "native-session".into())
        .await
        .unwrap();
    let body = r#"{"input":"Verified instructions"}"#.to_owned();
    assert!(matches!(
        tokio::time::timeout(
            Duration::from_secs(2),
            repo.claim_pm_guidance(id, body.clone())
        )
        .await
        .unwrap()
        .unwrap(),
        PmGuidancePermit::Claimed
    ));
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(2), repo.claim_pm_guidance(id, body))
            .await
            .unwrap()
            .unwrap(),
        PmGuidancePermit::Unknown
    ));
    let tool = domain::PmToolCommand {
        session_run_id: id,
        key: "publish-original".into(),
        kind: "revision".into(),
        request: json!({"goal":"original"}),
        result: None,
        attempted: false,
    };
    let (instructions, result) =
        instruction_receipt_fixture(&reservation, "run_single_pool", "pool-instructions");
    repo.prepare_pm_tool(instructions.clone()).await.unwrap();
    assert!(repo.claim_pm_tool(id, &instructions.key).await.unwrap());
    repo.finish_pm_tool(id, &instructions.key, result)
        .await
        .unwrap();
    repo.prepare_pm_tool(tool.clone()).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), repo.claim_pm_tool(id, &tool.key))
            .await
            .unwrap()
            .unwrap()
    );
    assert!(!repo.claim_pm_tool(id, &tool.key).await.unwrap());
    let mut changed = tool;
    changed.request = json!({"goal":"changed"});
    assert!(repo.prepare_pm_tool(changed).await.is_err());
}

#[tokio::test]
async fn pm_downgrade_refuses_unknown_known_and_guidance_custody_without_changing_ledger() {
    use migration::MigratorTrait;
    let (repo, reservation, _) = setup().await;
    let id = reservation.session_run_id;
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    // Invoke 022's own refusal; 024 must retain the repaired ACK constraint for known custody.
    let dispatch_migration = migration::Migrator::migrations()
        .into_iter()
        .find(|item| item.name() == "m20261010_000022_pm_dispatch")
        .expect("PM dispatch migration must remain registered");
    let before = migration::Migrator::get_migration_models(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|r| (r.version, r.applied_at))
        .collect::<Vec<_>>();
    assert!(
        before
            .iter()
            .any(|(version, _)| version == "m20261010_000022_pm_dispatch")
    );
    assert!(
        before
            .iter()
            .any(|(version, _)| version == "m20261010_000024_pm_ack_bounds")
    );
    assert!(repo.claim_pm_submission(id).await.unwrap());
    for phase in ["unknown", "known", "guidance"] {
        if phase == "known" {
            repo.record_pm_submission(id, "run_custody".into())
                .await
                .unwrap();
            repo.accept_pm_run(id, "run_custody".into(), "native-session".into())
                .await
                .unwrap();
        }
        if phase == "guidance" {
            assert!(matches!(
                repo.claim_pm_guidance(id, r#"{"input":"original"}"#.into())
                    .await
                    .unwrap(),
                PmGuidancePermit::Claimed
            ));
        }
        let original = repo.get_pm_dispatch(id).await.unwrap().unwrap();
        let error = dispatch_migration
            .down(&migration::SchemaManager::new(&db))
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("PM dispatch custody prevents downgrade"));
        assert!(repo.get_pm_dispatch(id).await.unwrap().unwrap() == original);
        let after = migration::Migrator::get_migration_models(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|r| (r.version, r.applied_at))
            .collect::<Vec<_>>();
        assert_eq!(before, after);
        assert!(!repo.claim_pm_submission(id).await.unwrap());
    }
}
