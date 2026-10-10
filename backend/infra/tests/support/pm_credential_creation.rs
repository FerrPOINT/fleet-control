use super::*;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use domain::{PmCredentialCommand, PmDraftOperation};
use infra::pm_credentials::PmCredentialCoordinator;
use migration::MigratorTrait;
use serde_json::{Value, json};
use tokio::sync::Mutex;

const PARENT: &str = "sdlc_pat_parent-test-secret-1234567890";
const CHILD: &str = "sdlc_pat_child-test-secret-1234567890";

#[tokio::test]
async fn pm_mcp_publishes_tracker_receipts_then_resumes_only_after_saved_answer_and_terminal_proof()
{
    use app::RuntimeSupervisor;
    use axum::{http::Method, response::IntoResponse};
    use domain::*;
    use sha2::{Digest, Sha256};
    for fault in ["none", "stop-not-terminal", "unknown-native-post"] {
        let mut fixture = fixture()
            .await
            .expect("isolated PostgreSQL required for PM tool integration");
        fixture.config.fleet.runtime_token_secret = "native-test-only-secret".into();
        let expected_native = format!(
            "Bearer {}",
            infra::agent_runtime_token(&fixture.config, fixture.remote.operation.request.agent_id)
                .unwrap()
        );
        fixture.prepare().await.unwrap();
        let op = fixture.operation().await;
        let reservation = initial_pm_reservation(&op).unwrap();
        let mut assigned = super::pm_dispatch::workflow_assignment(
            &initial_pm_assignment(&op, Value::Null).unwrap(),
        );
        assigned["binding_state"] = json!("bound");
        assigned["binding_ref"] = json!(reservation.binding_ref);
        assigned["hermes_run_ref"] = json!("run_old");
        assigned["concrete_agent_ref"] = json!(reservation.identity.agent_ref);
        assigned["bind_operation_key"] = json!(format!("fleet-pm-runtime-bind:{}", op.id));
        let phase_state = Arc::new(Mutex::new(
            json!({"phase":"PM-DRAFT-01","status":"active","blocked_body":null}),
        ));
        let ledger = Arc::new(Mutex::new(
            json!({"contract_version":1,"identity":reservation.identity,"state":"active","version":1,"fence":1,
            "session_run_id":op.id,"binding_ref":reservation.binding_ref,"hermes_run_ref":"run_old","checkpoint":null,
            "resume_operation_key":null,"resume_session_run_id":null,"terminal_readback":null,"workflow_step_allowed":true,"resume_delivered":false}),
        ));
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let stopped = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let state = (
            ledger.clone(),
            calls.clone(),
            stopped.clone(),
            reservation.clone(),
            expected_native,
            assigned,
            phase_state.clone(),
        );
        let router=Router::new().fallback(move |method:Method,uri:axum::http::Uri,headers:HeaderMap,bytes:axum::body::Bytes|{
            let (ledger,calls,stopped,reservation,expected_native,assigned,phase_state)=state.clone();
            async move {
                let path=uri.path(); calls.lock().await.push(format!("{method} {path}"));
                let body:Value=if bytes.is_empty(){Value::Null}else{serde_json::from_slice(&bytes).unwrap()};
                if path.starts_with("/internal/") {
                    assert_eq!(method,Method::POST);
                    let checkpoint=path.ends_with("/checkpoint");
                    let step=path=="/internal/runtime/step";
                    assert_eq!(headers["authorization"],format!("Bearer {}",if checkpoint||step{"r"}else{"a"}.repeat(40)));
                    if path=="/internal/runtime/assign" || path=="/internal/runtime/bind" {
                        let mut result=assigned.clone();
                        let phase=phase_state.lock().await;
                        // Match the producer: only bind replay reads the actual advanced task phase.
                        if path.ends_with("/bind") {
                            assert_eq!(body["bind_operation_key"],assigned["bind_operation_key"]);
                            assert_eq!(body["hermes_run_ref"],"run_old");
                            result["current_phase_code"]=phase["phase"].clone();
                        }
                        result["status"]=phase["status"].clone();
                        return Json(json!({"ok":true,"exit_code":0,"result":result})).into_response();
                    }
                    if step {
                        assert_eq!(headers["x-workflow-execution-token"],"b".repeat(64));
                        let current=ledger.lock().await.clone();
                        assert_eq!(body["session_run_id"],current["session_run_id"]);
                        assert_eq!(body["binding_ref"],current["binding_ref"]);
                        assert_eq!(body["hermes_run_ref"],current["hermes_run_ref"]);
                        let mut phase=phase_state.lock().await;
                        if body["report"]=="blocked" && !phase["blocked_body"].is_null() {
                            assert_eq!(body,phase["blocked_body"],"BLOCKED lost-ACK replay must retain the original active-status body");
                            return Json(json!({"ok":false,"exit_code":1,"output":"blocked","result":{"verdict":"BLOCKED"}})).into_response();
                        }
                        assert_eq!(body["expected_phase_code"],phase["phase"]);
                        assert_eq!(body["expected_status"],phase["status"]);
                        if body["report"]=="advance" {
                            assert_eq!(phase["phase"],"PM-DRAFT-01");phase["phase"]=json!("PM-DRAFT-02");
                            return Json(json!({"ok":true,"exit_code":0,"output":"advance","result":{"verdict":"PASS"}})).into_response();
                        }
                        if body["report"]=="blocked" {
                            phase["status"]=json!("blocked");phase["blocked_body"]=body;
                            return (StatusCode::SERVICE_UNAVAILABLE,Json(json!({"error":"committed response lost"}))).into_response();
                        }
                        assert!(body.get("report").is_none());
                        return Json(json!({"ok":true,"exit_code":0,"output":"Current phase instructions","result":{"ok":true,
                            "task_key":reservation.identity.task,"phase_code":phase["phase"],"status":phase["status"],
                            "instructions":"Current phase instructions","phase_contract":{},"workflow_id":1,"mode_id":2,"mode_key":"draft","cycle_number":0}})).into_response();
                    }
                    let mut snapshot=ledger.lock().await;
                    if path.ends_with("/readback") {
                        let mut result=snapshot.clone();result["operation"]=Value::Null;
                        return Json(if result["state"]=="active" {json!({"ok":true,"result":result,"execution_token":"b".repeat(64)})}else{json!({"ok":true,"result":result})}).into_response();
                    }
                    assert_eq!(body["expected_version"],snapshot["version"]);assert_eq!(body["expected_fence"],snapshot["fence"]);
                    assert_eq!(body["session_run_id"],snapshot["session_run_id"]);
                    assert_eq!(body["hermes_run_ref"],snapshot["hermes_run_ref"]);assert_eq!(body["binding_ref"],snapshot["binding_ref"]);
                    if checkpoint {
                        assert_eq!(headers["x-workflow-execution-token"],"b".repeat(64));
                        assert_eq!(snapshot["state"],"active");
                        snapshot["checkpoint"]=json!({"checkpoint_ref":body["checkpoint_ref"],"clarification_request_ref":body["clarification_request_ref"],
                            "clarification_version":body["clarification_version"],"requirements_revision":body["requirements_revision"]});
                        snapshot["state"]=json!("waiting");snapshot["workflow_step_allowed"]=json!(false);
                    } else if path.ends_with("/resume") {
                        assert!(stopped.load(Ordering::SeqCst));assert_eq!(snapshot["state"],"waiting");
                        assert!(Uuid::parse_str(body["answer_event_ref"].as_str().unwrap()).is_ok());
                        snapshot["state"]=json!("resume_pending");snapshot["fence"]=json!(2);
                        snapshot["resume_operation_key"]=body["operation_key"].clone();snapshot["resume_session_run_id"]=body["new_session_run_id"].clone();
                        snapshot["terminal_readback"]=serde_json::to_value(PmRuntimeObservation{identity:reservation.identity.clone(),
                            observation_ref:Uuid::new_v4(),binding_ref:reservation.binding_ref.clone(),hermes_run_ref:"run_old".into(),
                            session_run_id:reservation.session_run_id,status:PmRuntimeStatus::Stopped,dispatch_operation_key:reservation.dispatch_operation_key.clone(),checkpoint_ref:None,fence:1}).unwrap();
                    } else {
                        assert!(path.ends_with("/rebind"));assert_eq!(snapshot["state"],"resume_pending");
                        assert_eq!(body["resume_operation_key"],snapshot["resume_operation_key"]);
                        snapshot["session_run_id"]=body["new_session_run_id"].clone();snapshot["binding_ref"]=body["new_binding_ref"].clone();
                        snapshot["hermes_run_ref"]=body["new_hermes_run_ref"].clone();snapshot["state"]=json!("active");
                        snapshot["terminal_readback"]=Value::Null;snapshot["resume_delivered"]=json!(true);snapshot["workflow_step_allowed"]=json!(true);
                    }
                    snapshot["version"]=json!(snapshot["version"].as_i64().unwrap()+1);
                    return Json(if snapshot["state"]=="active" {json!({"ok":true,"result":snapshot.clone(),"execution_token":"b".repeat(64)})}else{json!({"ok":true,"result":snapshot.clone()})}).into_response();
                }
                assert_eq!(headers["authorization"],expected_native);
                match path {
                    "/health"=>Json(json!({"status":"ok"})).into_response(),
                    "/v1/capabilities"=>Json(super::hermes_protocol_fixture::capabilities()).into_response(),
                    "/v1/runs/run_old/stop"=>{
                        assert_eq!(method,Method::POST);
                        if fault!="stop-not-terminal" {stopped.store(true,Ordering::SeqCst);}
                        Json(json!({"run_id":"run_old","status":"stopping"})).into_response()
                    }
                    "/v1/runs/run_old"=>Json(if stopped.load(Ordering::SeqCst){json!({"object":"hermes.run","run_id":"run_old","session_id":"native-session",
                        "status":"stopped","completed":false,"partial":true,"interrupted":true})}else{json!({"object":"hermes.run","run_id":"run_old","session_id":"native-session","status":"running"})}).into_response(),
                    "/v1/runs"=>{
                        assert_eq!(method,Method::POST);assert_eq!(headers["idempotency-key"],ledger.lock().await["resume_session_run_id"].as_str().unwrap());
                        assert!(stopped.load(Ordering::SeqCst));assert!(!body.to_string().contains(PARENT));assert!(!body.to_string().contains(CHILD));
                        if fault=="unknown-native-post" {(StatusCode::SERVICE_UNAVAILABLE,Json(json!({"error":"unknown acceptance"}))).into_response()}
                        else {(StatusCode::ACCEPTED,Json(json!({"run_id":"run_new","status":"started","replayed":false}))).into_response()}
                    }
                    "/v1/runs/run_new"=>Json(json!({"object":"hermes.run","run_id":"run_new","session_id":"native-session","status":"running"})).into_response(),
                    "/v1/runs/run_old/events" | "/v1/runs/run_new/events" => {
                        assert_eq!(method, Method::GET);
                        ([("content-type", "text/event-stream")], "").into_response()
                    }
                    _=>panic!("unexpected production PM endpoint"),
                }
            }
        });
        let _peer = super::pm_dispatch::AbortServer(tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap()
        }));
        fixture.config.pm.dispatch.enabled = true;
        fixture.config.pm.dispatch.assignment_token = "a".repeat(40);
        fixture.config.pm.dispatch.runtime_token = "r".repeat(40);
        fixture.config.pm.readback_token = "p".repeat(40);
        fixture.config.fleet.runtime_token_secret = "native-test-only-secret".into();
        fixture.config.fleet.project_workflow_url = Some(format!("http://127.0.0.1:{port}"));
        let api_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let api_origin = format!("http://{}", api_listener.local_addr().unwrap());
        fixture.config.pm.dispatch.tool_origin = Some(api_origin.clone());
        let _home = super::pm_dispatch::install_tool_home(
            fixture.remote.repo.as_ref(),
            &op,
            &mut fixture.config,
        )
        .await;
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
        fixture
            .remote
            .repo
            .reserve_pm_run(reservation.clone())
            .await
            .unwrap();
        let mut hash = Sha256::new();
        hash.update(b"fleet-pm-workflow-v1\0");
        hash.update("a".repeat(40));
        hash.update(b"\0");
        hash.update("r".repeat(40));
        let native = infra::agent_runtime_token(&fixture.config, op.request.agent_id).unwrap();
        let mut native_hash = Sha256::new();
        native_hash.update(b"fleet-hermes-default-profile-v1\0");
        native_hash.update(native.as_bytes());
        fixture
            .remote
            .repo
            .prepare_pm_dispatch(PmDispatchIntent {
                session_run_id: op.id,
                origin: format!("http://127.0.0.1:{port}"),
                credential_fingerprint: hex::encode(native_hash.finalize()),
                request_body:
                    json!({"input":"original","session_id":reservation.runtime_session_id()})
                        .to_string(),
                workflow_assignment: json!({}),
                workflow_origin: format!("http://127.0.0.1:{port}"),
                workflow_credential_fingerprint: hex::encode(hash.finalize()),
                runtime_context: json!({}),
                submitted: false,
                hermes_run_ref: None,
            })
            .await
            .unwrap();
        assert!(
            fixture
                .remote
                .repo
                .claim_pm_submission(op.id)
                .await
                .unwrap()
        );
        fixture
            .remote
            .repo
            .record_pm_submission(op.id, "run_old".into())
            .await
            .unwrap();
        fixture
            .remote
            .repo
            .accept_pm_run(op.id, "run_old".into(), "native-session".into())
            .await
            .unwrap();
        let config = Arc::new(fixture.config.clone());
        let (events, _) = tokio::sync::broadcast::channel(32);
        let runtime = Arc::new(infra::runtime::LocalRuntimeSupervisor::new(
            config.clone(),
            fixture.remote.repo.clone(),
            events.clone(),
        ));
        let (restart, _) = tokio::sync::mpsc::channel(1);
        let ctx = Arc::new(app::AppContext::new(
            config,
            fixture.remote.repo.clone(),
            Arc::new(infra::FilesystemProvisioner),
            runtime.clone(),
            events,
            restart,
        ));
        let api_router = Router::new()
            .route(
                "/mcp/{agent}",
                post(api::routes::pm_tools::handle)
                    .layer(axum::extract::DefaultBodyLimit::max(262144)),
            )
            .with_state(ctx);
        let _api = super::pm_dispatch::AbortServer(tokio::spawn(async move {
            axum::serve(api_listener, api_router).await.unwrap()
        }));
        let client = reqwest::Client::new();
        let endpoint = format!("{api_origin}/mcp/{}", op.request.agent_id);
        let initialize = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"test","version":"1"}}});
        assert_eq!(
            client
                .post(&endpoint)
                .json(&initialize)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            client
                .post(&endpoint)
                .bearer_auth(PARENT)
                .json(&initialize)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let initialized: Value = client
            .post(&endpoint)
            .bearer_auth(&native)
            .json(&initialize)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(initialized["result"]["protocolVersion"], "2025-03-26");
        // Exercise the real HTTP extractor/handler boundary, not just DTO deserialization.
        for (request, expected) in [
            (
                client
                    .post(&endpoint)
                    .bearer_auth(&native)
                    .header("Origin", "https://untrusted.invalid")
                    .json(&initialize),
                StatusCode::FORBIDDEN,
            ),
            (
                client
                    .post(&endpoint)
                    .bearer_auth(&native)
                    .header("Authorization", format!("Bearer {native}"))
                    .json(&initialize),
                StatusCode::UNAUTHORIZED,
            ),
            (
                client
                    .post(&endpoint)
                    .bearer_auth(&native)
                    .header("MCP-Protocol-Version", "2025-03-26")
                    .header("MCP-Protocol-Version", "2025-03-26")
                    .json(&initialize),
                StatusCode::BAD_REQUEST,
            ),
            (
                client
                    .post(&endpoint)
                    .bearer_auth(&native)
                    .header("MCP-Protocol-Version", "unsupported")
                    .json(&initialize),
                StatusCode::BAD_REQUEST,
            ),
            (
                client
                    .post(&endpoint)
                    .bearer_auth(&native)
                    .header("Content-Type", "text/plain")
                    .body("{}"),
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
            ),
            (
                client
                    .post(&endpoint)
                    .bearer_auth(&native)
                    .header("Content-Type", "application/json")
                    .body("{"),
                StatusCode::BAD_REQUEST,
            ),
            (
                client
                    .post(&endpoint)
                    .bearer_auth(&native)
                    .json(&json!({"padding":"x".repeat(262144)})),
                StatusCode::PAYLOAD_TOO_LARGE,
            ),
            (
                client.get(&endpoint).bearer_auth(&native),
                StatusCode::METHOD_NOT_ALLOWED,
            ),
            (
                client
                    .post(format!("{api_origin}/mcp/{}", Uuid::new_v4()))
                    .bearer_auth(&native)
                    .json(&initialize),
                StatusCode::UNAUTHORIZED,
            ),
        ] {
            assert_eq!(request.send().await.unwrap().status(), expected);
        }
        let negotiated = client
            .post(&endpoint)
            .bearer_auth(&native)
            .header("MCP-Protocol-Version", "2025-03-26")
            .json(&initialize)
            .send()
            .await
            .unwrap();
        assert_eq!(negotiated.status(), StatusCode::OK);
        assert_eq!(negotiated.headers()["cache-control"], "no-store");
        let fence = json!({"assignment_id":op.reservation.as_ref().unwrap().assignment.assignment_id,"execution_id":op.reservation.as_ref().unwrap().assignment.execution_id,
            "agent_id":op.request.agent_id,"assignment_version":1});
        let document = json!({"goal":"Owner goal","scope":["bounded"],"exclusions":[],"scenarios":[],"acceptance_criteria":["criterion"],"constraints":[],"dependencies":[],"assumptions":[],"checklist":[],"prerequisites":[]});
        let revision = json!({"fence":fence,"expected_requirement_revision":null,"document":document,"idempotency_key":"original-revision"});
        for _ in 0..2 {
            let response:Value=client.post(&endpoint).bearer_auth(&native).json(&json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"tracker_publish_revision",
            "arguments":{"operation_id":op.id,"session_run_id":op.id,"command":revision}}})).send().await.unwrap().json().await.unwrap();
            assert_eq!(response["result"]["isError"], false);
        }
        assert_eq!(fixture.remote.revisions.lock().await.len(), 1);
        if fault == "none" {
            let step = |key: &str, report: Option<&str>| PmToolCall {
                operation_id: op.id,
                session_run_id: op.id,
                command: json!({"step_operation_key":key,"report":report}),
            };
            assert_eq!(
                runtime
                    .call_pm_tool(
                        op.request.agent_id,
                        "workflow_step",
                        step("advance", Some("advance"))
                    )
                    .await
                    .unwrap()["supervisor_accepted"],
                true
            );
            let instructions = runtime
                .call_pm_tool(
                    op.request.agent_id,
                    "workflow_step",
                    step("instructions-after-advance", None),
                )
                .await
                .unwrap();
            assert_eq!(instructions["result"]["phase_code"], "PM-DRAFT-02");
            assert!(
                runtime
                    .call_pm_tool(
                        op.request.agent_id,
                        "workflow_step",
                        step("blocked", Some("blocked"))
                    )
                    .await
                    .is_err()
            );
            assert_eq!(phase_state.lock().await["status"], "blocked");
            for _ in 0..2 {
                assert_eq!(
                    runtime
                        .call_pm_tool(
                            op.request.agent_id,
                            "workflow_step",
                            step("blocked", Some("blocked"))
                        )
                        .await
                        .unwrap(),
                    json!({"supervisor_accepted":false,"exit_code":1})
                );
            }
            let before = calls
                .lock()
                .await
                .iter()
                .filter(|c| c.as_str() == "POST /internal/runtime/step")
                .count();
            assert!(
                runtime
                    .call_pm_tool(
                        op.request.agent_id,
                        "workflow_step",
                        step("blocked", Some("changed report"))
                    )
                    .await
                    .is_err()
            );
            assert_eq!(
                calls
                    .lock()
                    .await
                    .iter()
                    .filter(|c| c.as_str() == "POST /internal/runtime/step")
                    .count(),
                before
            );
            assert_eq!(
                before, 4,
                "advance + instructions + lost ACK + exact replay; cached replay is local"
            );
            let saved = fixture
                .remote
                .repo
                .get_pm_tool(op.id, "blocked")
                .await
                .unwrap()
                .unwrap();
            assert_eq!(
                saved.request["body"],
                phase_state.lock().await["blocked_body"]
            );
            assert_eq!(saved.request["body"]["expected_phase_code"], "PM-DRAFT-02");
            assert_eq!(saved.request["body"]["expected_status"], "active");
        }
        let question_id = Uuid::new_v4();
        let checkpoint = Uuid::new_v4();
        let request_id = Uuid::new_v4();
        let question = json!({"fence":fence,"request_id":request_id,"question_id":question_id,"expected_question_version":null,"requirement_revision":1,"checkpoint_id":checkpoint,
            "requirement_reference":null,"text":"What is the expected outcome?","rationale":"Clarify acceptance","required":true,"mode":"text","options":[],"recommended_option_id":null,"idempotency_key":"original-question"});
        for _ in 0..2 {
            let response:Value=client.post(&endpoint).bearer_auth(&native).json(&json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"tracker_publish_question",
            "arguments":{"operation_id":op.id,"session_run_id":op.id,"command":question}}})).send().await.unwrap().json().await.unwrap();
            assert_eq!(response["result"]["isError"], false);
        }
        assert_eq!(fixture.remote.questions.lock().await.len(), 1);
        assert_eq!(ledger.lock().await["state"], "waiting");
        let actor = ClarificationCommandActor {
            session_id: op.session_id.unwrap(),
            user_id: op.owner_user_id,
            subject: op.owner_subject.clone(),
            binding: fixture
                .remote
                .repo
                .get_task_chat_binding(op.session_id.unwrap())
                .await
                .unwrap()
                .expect("persisted task-chat binding"),
        };
        let saved = fixture
            .remote
            .repo
            .store_clarification_command(
                &actor,
                question_id,
                ClarificationAnswerRequest {
                    expected_question_version: 1,
                    requirement_revision: 1,
                    selected_option_ids: vec![],
                    text: Some("Verified expected outcome".into()),
                    comment: None,
                    idempotency_key: "owner-answer".into(),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            runtime.resume_pm_answer(&actor, &saved).await.unwrap(),
            PmContinuationOutcome::Pending
        );
        assert!(!stopped.load(Ordering::SeqCst));
        let permit = fixture
            .remote
            .repo
            .claim_clarification_delivery(&actor, saved.id)
            .await
            .unwrap();
        let answer:TrackerAnswer=serde_json::from_value(json!({"id":Uuid::new_v4(),"question_id":question_id,"question_version":1,"requirement_revision":1,
            "selected_option_ids":[],"text":"Verified expected outcome","comment":null,"author_subject":op.owner_subject,"created_at":"2026-10-10T00:00:00Z"})).unwrap();
        let delivered = fixture
            .remote
            .repo
            .finish_clarification_delivery(
                &actor,
                saved.id,
                permit.attempt_id.unwrap(),
                ClarificationDeliveryOutcome::Delivered(answer.clone()),
            )
            .await
            .unwrap();
        {
            let mut questions = fixture.remote.questions.lock().await;
            questions[0]["answer"] = serde_json::to_value(answer).unwrap();
            questions[0]["state"] = json!("answered");
        }
        for _ in 0..2 {
            assert_eq!(
                runtime.resume_pm_answer(&actor, &delivered).await.unwrap(),
                if fault == "none" {
                    PmContinuationOutcome::Confirmed
                } else {
                    PmContinuationOutcome::Pending
                }
            );
        }
        if fault == "none" {
            let resumed = fixture
                .remote
                .repo
                .read_pm_operation_for_session(actor.session_id, actor.user_id)
                .await
                .unwrap();
            let current = ledger.lock().await["session_run_id"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap();
            let instructions = runtime
                .call_pm_tool(
                    op.request.agent_id,
                    "workflow_step",
                    PmToolCall {
                        operation_id: resumed.id,
                        session_run_id: current,
                        command: json!({"step_operation_key":"instructions-after-rebind","report":null}),
                    },
                )
                .await
                .unwrap();
            assert_eq!(instructions["result"]["phase_code"], "PM-DRAFT-02");
            assert_ne!(current, op.id, "PM rebind must use the new native run");
        }
        let requests = calls.lock().await;
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.as_str() == "POST /v1/runs/run_old/stop")
                .count(),
            1
        );
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.as_str() == "POST /v1/runs")
                .count(),
            usize::from(fault != "stop-not-terminal")
        );
        assert_eq!(
            requests
                .iter()
                .filter(|r| r.as_str() == "POST /internal/runtime/v1/pm/rebind")
                .count(),
            usize::from(fault == "none")
        );
        assert_eq!(
            ledger.lock().await["state"],
            if fault == "none" {
                "active"
            } else if fault == "stop-not-terminal" {
                "waiting"
            } else {
                "resume_pending"
            }
        );
        if fault == "unknown-native-post" {
            let journal = fixture
                .remote
                .repo
                .get_pm_dispatch(saved.id)
                .await
                .unwrap()
                .unwrap();
            assert!(journal.submitted && journal.hermes_run_ref.is_none());
        }
        let custody = serde_json::to_string(&fixture.operation().await).unwrap();
        assert!(!custody.contains(PARENT) && !custody.contains(CHILD));
    }
}

struct Remote {
    repo: Arc<PostgresFleetRepository>,
    operation: PmDraftOperation,
    subject: String,
    scopes: Vec<String>,
    receipt: Mutex<Option<Value>>,
    parent_principal: Mutex<Option<Value>>,
    child_principal: Mutex<Option<Value>>,
    parent_checks: AtomicUsize,
    child_checks: AtomicUsize,
    posts: AtomicUsize,
    children: AtomicUsize,
    contexts: AtomicUsize,
    mode: AtomicUsize,
    leases: Mutex<std::collections::HashMap<String, Value>>,
    revisions: Mutex<Vec<Value>>,
    questions: Mutex<Vec<Value>>,
}

impl Remote {
    fn valid_principal(&self, parent: bool) -> Value {
        json!({"sub":self.subject,"email":"machine@example.test","display_name":"PM machine",
            "scopes":if parent {vec!["task-tracker:read".to_string(),"task-tracker:write".to_string()]} else {self.scopes.clone()}})
    }
}

fn invalid_principals(valid: &Value) -> Vec<(String, Value)> {
    let mut wrong_subject = valid.clone();
    wrong_subject["sub"] = json!(Uuid::new_v4());
    let mut extra = valid.clone();
    extra["scopes"]
        .as_array_mut()
        .unwrap()
        .push(json!("task-tracker:admin"));
    let mut cases = vec![
        ("wrong subject".into(), wrong_subject),
        ("extra scope".into(), extra),
    ];
    for (index, scope) in valid["scopes"].as_array().unwrap().iter().enumerate() {
        let mut missing = valid.clone();
        missing["scopes"].as_array_mut().unwrap().remove(index);
        cases.push((format!("missing {scope}"), missing));
        let mut duplicate = valid.clone();
        duplicate["scopes"]
            .as_array_mut()
            .unwrap()
            .push(scope.clone());
        cases.push((format!("duplicate {scope}"), duplicate));
    }
    cases
}

async fn introspection(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
) -> (StatusCode, Json<Value>) {
    let root = headers["authorization"] == format!("Bearer {PARENT}");
    if root {
        remote.parent_checks.fetch_add(1, Ordering::SeqCst);
    } else {
        remote.child_checks.fetch_add(1, Ordering::SeqCst);
    }
    if root && remote.mode.load(Ordering::SeqCst) == 8 {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":"revoked parent detail"})),
        );
    }
    if !root {
        assert_eq!(headers["authorization"], format!("Bearer {CHILD}"));
        if remote.mode.load(Ordering::SeqCst) == 4 {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error":"revoked secret detail"})),
            );
        }
    }
    let principal = if root {
        remote.parent_principal.lock().await.clone()
    } else {
        remote.child_principal.lock().await.clone()
    };
    (
        StatusCode::OK,
        Json(principal.unwrap_or_else(|| remote.valid_principal(root))),
    )
}

async fn delegate(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
    Json(command): Json<Value>,
) -> (StatusCode, HeaderMap, Json<Value>) {
    assert_eq!(headers["authorization"], format!("Bearer {PARENT}"));
    let saved = remote
        .repo
        .read_pm_draft_operation(remote.operation.id, remote.operation.owner_user_id)
        .await
        .unwrap();
    let journal = saved.credentials.unwrap();
    if command["idempotency_key"]
        .as_str()
        .unwrap()
        .starts_with("fleet-pm-tools:")
    {
        assert_eq!(command["scopes"], json!(remote.scopes));
        assert!(
            command["idempotency_key"]
                .as_str()
                .unwrap()
                .starts_with(&format!("fleet-pm-tools:{}:", remote.operation.id))
        );
        let mut leases = remote.leases.lock().await;
        let value = leases
            .entry(command["idempotency_key"].as_str().unwrap().into())
            .or_insert_with(|| {
                json!({
            "secret":CHILD,"token_id":Uuid::new_v4(),"scopes":remote.scopes,
            "expires_at":chrono::Utc::now()+chrono::Duration::seconds(300)})
            })
            .clone();
        let mut headers = HeaderMap::new();
        headers.insert("cache-control", "no-store".parse().unwrap());
        return (StatusCode::OK, headers, Json(value));
    }
    assert_eq!(journal.intent.command, command);
    assert_eq!(
        journal.intent.request_sha256,
        domain::pm_canonical_hash(&command)
    );
    assert_eq!(journal.intent.parent_fingerprint.len(), 64);
    assert_eq!(
        command["idempotency_key"],
        remote.operation.credential_key()
    );
    remote.posts.fetch_add(1, Ordering::SeqCst);
    let mut receipt = remote.receipt.lock().await;
    let replay = receipt.is_some();
    let value = receipt
        .get_or_insert_with(|| {
            remote.children.fetch_add(1, Ordering::SeqCst);
            json!({"secret":CHILD,"token_id":Uuid::new_v4(),"scopes":remote.scopes,
            "expires_at":chrono::Utc::now()+chrono::Duration::seconds(command["expires_in_seconds"].as_i64().unwrap())})
        })
        .clone();
    let mut headers = HeaderMap::new();
    headers.insert("cache-control", "no-store".parse().unwrap());
    if remote.mode.load(Ordering::SeqCst) == 1 {
        return (
            StatusCode::BAD_GATEWAY,
            headers,
            Json(json!({"error":"lost acknowledgement"})),
        );
    }
    (
        if replay {
            StatusCode::OK
        } else {
            StatusCode::CREATED
        },
        headers,
        Json(value),
    )
}

async fn context(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
) -> (StatusCode, Json<Value>) {
    assert_eq!(headers["authorization"], format!("Bearer {CHILD}"));
    remote.contexts.fetch_add(1, Ordering::SeqCst);
    let mode = remote.mode.load(Ordering::SeqCst);
    if mode == 2 {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error":"private Tracker detail"})),
        );
    }
    let op = &remote.operation;
    let identity = op.identity().unwrap();
    let mut value = json!({"contract_version":1,"tracker_instance_id":identity.tracker_instance_id,
        "project_id":identity.project_id,"task_id":identity.task_id,"root_task_id":identity.root_task_id,
        "owner_subject":identity.owner_subject,"stage":"Draft","requirement_revision":null,"waiting_reason":null,
        "permissions":{"can_answer":false,"can_confirm":false},"assignment":op.reservation.as_ref().unwrap().assignment});
    if let Some(revision) = remote.revisions.lock().await.last() {
        value["requirement_revision"] = revision["revision"].clone();
    }
    match mode {
        3 => value["assignment"]["assignment_id"] = json!(Uuid::new_v4()),
        5 => value["owner_subject"] = json!(remote.subject),
        6 => {
            value["assignment"]["agent_id"] = json!(op.request.agent_id.to_string().to_uppercase())
        }
        7 => value["permissions"]["can_confirm"] = json!(true),
        _ => (),
    }
    (StatusCode::OK, Json(value))
}

struct Fixture {
    remote: Arc<Remote>,
    config: AppConfig,
    server: tokio::task::JoinHandle<()>,
}

async fn revisions(State(remote): State<Arc<Remote>>, headers: HeaderMap) -> Json<Value> {
    assert_eq!(headers["authorization"], format!("Bearer {CHILD}"));
    Json(json!({"revisions":remote.revisions.lock().await.clone()}))
}
async fn questions(State(remote): State<Arc<Remote>>, headers: HeaderMap) -> Json<Value> {
    assert_eq!(headers["authorization"], format!("Bearer {CHILD}"));
    Json(json!({"questions":remote.questions.lock().await.clone()}))
}
async fn publish_revision(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    assert_eq!(headers["authorization"], format!("Bearer {CHILD}"));
    let assignment = &remote.operation.reservation.as_ref().unwrap().assignment;
    assert_eq!(
        body["fence"],
        json!({"assignment_id":assignment.assignment_id,"execution_id":assignment.execution_id,
        "agent_id":assignment.agent_id,"assignment_version":assignment.version})
    );
    let mut revisions = remote.revisions.lock().await;
    assert_eq!(
        body["expected_requirement_revision"],
        if revisions.is_empty() {
            Value::Null
        } else {
            json!(revisions.len())
        }
    );
    let mut revision = body["document"].clone();
    revision.as_object_mut().unwrap().extend(
        json!({"revision":revisions.len()+1,"content_hash":"a".repeat(64),
        "author_subject":remote.subject,"created_at":"2026-10-10T00:00:00Z"})
        .as_object()
        .unwrap()
        .clone(),
    );
    revisions.push(revision.clone());
    Json(revision)
}
async fn publish_question(
    State(remote): State<Arc<Remote>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    assert_eq!(headers["authorization"], format!("Bearer {CHILD}"));
    let mut question = body.clone();
    let map = question.as_object_mut().unwrap();
    let fence = map.remove("fence").unwrap();
    map.extend(fence.as_object().unwrap().clone());
    let id = map.remove("question_id").unwrap();
    map.remove("idempotency_key");
    map.remove("expected_question_version");
    let binding = remote.operation.identity().unwrap();
    map.extend(json!({"id":id,"version":1,"task_id":binding.task_id,"root_task_id":binding.root_task_id,
        "state":"open","answer":null,"author_subject":remote.subject,"created_at":"2026-10-10T00:00:00Z"}).as_object().unwrap().clone());
    remote.questions.lock().await.push(question.clone());
    Json(question)
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

async fn fixture() -> Option<Fixture> {
    let (repo, operation, subject) = super::pm_draft_creation::credential_fixture().await?;
    let scopes = PmCredentialCommand::tracker(
        &operation.execution_identity().unwrap(),
        operation.credential_key(),
        300,
    )
    .unwrap()
    .scopes()
    .to_vec();
    let remote = Arc::new(Remote {
        repo: Arc::new(repo),
        operation,
        subject: subject.clone(),
        scopes,
        receipt: Mutex::new(None),
        parent_principal: Mutex::new(None),
        child_principal: Mutex::new(None),
        parent_checks: AtomicUsize::new(0),
        child_checks: AtomicUsize::new(0),
        posts: AtomicUsize::new(0),
        children: AtomicUsize::new(0),
        contexts: AtomicUsize::new(0),
        mode: AtomicUsize::new(0),
        leases: Mutex::new(std::collections::HashMap::new()),
        revisions: Mutex::new(vec![]),
        questions: Mutex::new(vec![]),
    });
    let router = Router::new()
        .route("/auth/tokens/introspect", get(introspection))
        .route("/auth/tokens/delegate", post(delegate))
        .route(
            &format!(
                "/api/v1/issues/{}/sdlc/context",
                remote.operation.identity().unwrap().task_id
            ),
            get(context),
        )
        .route(
            &format!(
                "/api/v1/issues/{}/sdlc/requirements",
                remote.operation.identity().unwrap().task_id
            ),
            post(publish_revision),
        )
        .route(
            &format!(
                "/api/v1/issues/{}/sdlc/requirements/revisions",
                remote.operation.identity().unwrap().task_id
            ),
            get(revisions),
        )
        .route(
            &format!(
                "/api/v1/issues/{}/sdlc/clarifications",
                remote.operation.identity().unwrap().task_id
            ),
            get(questions).post(publish_question),
        )
        .with_state(remote.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = AppConfig::default();
    config.tracker.url = origin.clone();
    config.pm.credentials = shared::PmCredentialsConfig {
        enabled: true,
        auth_url: origin,
        machine_subject: subject,
        parent_pat: PARENT.into(),
        ttl_seconds: 300,
    };
    Some(Fixture {
        remote,
        config,
        server,
    })
}

impl Fixture {
    fn coordinator(&self) -> PmCredentialCoordinator {
        PmCredentialCoordinator::configured(&self.config)
            .unwrap()
            .unwrap()
    }
    async fn operation(&self) -> PmDraftOperation {
        self.remote
            .repo
            .read_pm_draft_operation(
                self.remote.operation.id,
                self.remote.operation.owner_user_id,
            )
            .await
            .unwrap()
    }
    async fn prepare(&self) -> Result<(), shared::AppError> {
        self.coordinator()
            .prepare_credential(self.remote.repo.as_ref(), &self.operation().await)
            .await
            .map(|_| ())
    }

    async fn audits(&self) -> Vec<Value> {
        let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        db.query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT to_jsonb(audit_log) AS row FROM audit_log WHERE entity_type='pm_draft_operation' AND entity_id=$1 ORDER BY id",
            [self.remote.operation.id.to_string().into()],
        ))
        .await
        .unwrap()
        .into_iter()
        .map(|row| row.try_get("", "row").unwrap())
        .collect()
    }
}

async fn assert_changed_valid_receipt_conflicts(field: &str) {
    let Some(fixture) = fixture().await else {
        return;
    };
    // Retain an ACK without ever authorizing a Tracker context request.
    fixture.remote.mode.store(4, Ordering::SeqCst);
    assert!(matches!(
        fixture.prepare().await,
        Err(shared::AppError::Forbidden)
    ));
    let saved = fixture.operation().await;
    let original = saved
        .credentials
        .as_ref()
        .unwrap()
        .receipt
        .as_ref()
        .unwrap();
    let audits = fixture.audits().await;
    assert_eq!(audits.len(), 2);
    fixture.remote.mode.store(0, Ordering::SeqCst);
    {
        let mut response = fixture.remote.receipt.lock().await;
        let response = response.as_mut().unwrap();
        match field {
            "token_id" => response[field] = json!(Uuid::new_v4()),
            "expires_at" => {
                response[field] = json!(original.expires_at - chrono::Duration::seconds(1))
            }
            _ => unreachable!(),
        }
        let changed: domain::PmCredentialReceipt = serde_json::from_value(json!({
            "token_id": response["token_id"], "expires_at": response["expires_at"], "scopes": response["scopes"]
        })).unwrap();
        assert!(!changed.token_id.is_nil());
        assert!(changed.expires_at > chrono::Utc::now());
        assert!(changed.expires_at <= chrono::Utc::now() + chrono::Duration::seconds(300));
        assert_eq!(changed.scopes, fixture.remote.scopes);
        assert_ne!(&changed, original);
        // The domain guard must reject a valid but different receipt too, not just SQL tampering.
        let mut operation = saved.clone();
        assert!(matches!(
            operation.apply(domain::PmDraftProof::CredentialAcknowledged(changed)),
            Err(shared::AppError::Conflict(_))
        ));
        assert_eq!(operation.credentials, saved.credentials);
    }
    for attempt in 1..=2 {
        assert!(matches!(
            fixture.prepare().await,
            Err(shared::AppError::Conflict(_))
        ));
        assert_eq!(fixture.operation().await.credentials, saved.credentials);
        assert_eq!(fixture.audits().await, audits);
        assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 1 + attempt);
        assert_eq!(
            fixture.remote.parent_checks.load(Ordering::SeqCst),
            1 + attempt
        );
        assert_eq!(fixture.remote.child_checks.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 0);
    }
    *fixture.remote.receipt.lock().await =
        Some(json!({"secret":CHILD,"token_id":original.token_id,
        "expires_at":original.expires_at,"scopes":original.scopes}));
    fixture.prepare().await.unwrap();
    assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.operation().await.credentials, saved.credentials);
    assert_eq!(fixture.audits().await, audits);
}

#[tokio::test]
async fn pm_credentials_pg_changed_valid_child_uuid_replay_conflicts_without_context_or_mutation() {
    assert_changed_valid_receipt_conflicts("token_id").await;
}

#[tokio::test]
async fn pm_credentials_pg_changed_valid_child_expiry_replay_conflicts_without_context_or_mutation()
{
    assert_changed_valid_receipt_conflicts("expires_at").await;
}

#[tokio::test]
async fn pm_credentials_pg_parent_principal_mismatch_prevents_first_post() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let cases = invalid_principals(&fixture.remote.valid_principal(true));
    *fixture.remote.parent_principal.lock().await = Some(cases[0].1.clone());
    assert!(matches!(
        fixture.prepare().await,
        Err(shared::AppError::Forbidden)
    ));
    let saved = fixture.operation().await;
    assert!(saved.credentials.as_ref().unwrap().receipt.is_none());
    let audits = fixture.audits().await;
    assert_eq!(audits.len(), 1);
    for (index, (label, principal)) in cases.into_iter().enumerate() {
        *fixture.remote.parent_principal.lock().await = Some(principal);
        assert!(
            matches!(fixture.prepare().await, Err(shared::AppError::Forbidden)),
            "{label}"
        );
        assert_eq!(
            fixture.operation().await.credentials,
            saved.credentials,
            "{label}"
        );
        assert_eq!(fixture.audits().await, audits, "{label}");
        assert_eq!(
            fixture.remote.parent_checks.load(Ordering::SeqCst),
            index + 2,
            "{label}"
        );
        assert_eq!(
            fixture.remote.child_checks.load(Ordering::SeqCst),
            0,
            "{label}"
        );
        assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 0, "{label}");
        assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 0, "{label}");
        assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 0, "{label}");
    }
    *fixture.remote.parent_principal.lock().await = None;
    fixture.prepare().await.unwrap();
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    assert_eq!(
        fixture.operation().await.credentials.unwrap().intent,
        saved.credentials.unwrap().intent
    );
    assert_eq!(fixture.audits().await.len(), 2);
}

#[tokio::test]
async fn pm_credentials_pg_parent_principal_mismatch_retains_ack_without_another_post() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.remote.mode.store(4, Ordering::SeqCst);
    assert!(matches!(
        fixture.prepare().await,
        Err(shared::AppError::Forbidden)
    ));
    let saved = fixture.operation().await;
    assert!(saved.credentials.as_ref().unwrap().receipt.is_some());
    let audits = fixture.audits().await;
    assert_eq!(audits.len(), 2);
    fixture.remote.mode.store(0, Ordering::SeqCst);
    for (index, (label, principal)) in invalid_principals(&fixture.remote.valid_principal(true))
        .into_iter()
        .enumerate()
    {
        *fixture.remote.parent_principal.lock().await = Some(principal);
        assert!(
            matches!(fixture.prepare().await, Err(shared::AppError::Forbidden)),
            "{label}"
        );
        assert_eq!(
            fixture.operation().await.credentials,
            saved.credentials,
            "{label}"
        );
        assert_eq!(fixture.audits().await, audits, "{label}");
        assert_eq!(
            fixture.remote.parent_checks.load(Ordering::SeqCst),
            index + 2,
            "{label}"
        );
        assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 1, "{label}");
        assert_eq!(
            fixture.remote.child_checks.load(Ordering::SeqCst),
            1,
            "{label}"
        );
        assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1, "{label}");
        assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 0, "{label}");
    }
    *fixture.remote.parent_principal.lock().await = None;
    fixture.prepare().await.unwrap();
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.operation().await.credentials, saved.credentials);
    assert_eq!(fixture.audits().await, audits);
}

#[tokio::test]
async fn pm_credentials_pg_child_principal_mismatch_retains_ack_without_usable_credential() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let cases = invalid_principals(&fixture.remote.valid_principal(false));
    *fixture.remote.child_principal.lock().await = Some(cases[0].1.clone());
    assert!(matches!(
        fixture.prepare().await,
        Err(shared::AppError::Forbidden)
    ));
    let saved = fixture.operation().await;
    assert!(saved.credentials.as_ref().unwrap().receipt.is_some());
    let audits = fixture.audits().await;
    assert_eq!(audits.len(), 2);
    for (index, (label, principal)) in cases.into_iter().enumerate() {
        *fixture.remote.child_principal.lock().await = Some(principal);
        assert!(
            matches!(fixture.prepare().await, Err(shared::AppError::Forbidden)),
            "{label}"
        );
        assert_eq!(
            fixture.operation().await.credentials,
            saved.credentials,
            "{label}"
        );
        assert_eq!(fixture.audits().await, audits, "{label}");
        assert_eq!(
            fixture.remote.parent_checks.load(Ordering::SeqCst),
            index + 2,
            "{label}"
        );
        assert_eq!(
            fixture.remote.child_checks.load(Ordering::SeqCst),
            index + 2,
            "{label}"
        );
        assert_eq!(
            fixture.remote.posts.load(Ordering::SeqCst),
            index + 2,
            "{label}"
        );
        assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1, "{label}");
        assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 0, "{label}");
    }
    *fixture.remote.child_principal.lock().await = None;
    fixture.prepare().await.unwrap();
    assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.operation().await.credentials, saved.credentials);
    assert_eq!(fixture.audits().await, audits);
}

#[tokio::test]
async fn pm_credentials_pg_lost_ack_restart_replays_exact_child_without_run_or_secret() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.remote.mode.store(1, Ordering::SeqCst);
    assert!(matches!(
        super::pm_draft_creation::continue_with_credentials(
            fixture.remote.repo.as_ref(),
            fixture.operation().await,
            &fixture.coordinator(),
        )
        .await,
        Err(shared::AppError::Unavailable(_))
    ));
    let pending = fixture.operation().await;
    assert!(pending.credentials.as_ref().unwrap().receipt.is_none());
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 0);
    fixture.remote.mode.store(0, Ordering::SeqCst);
    let response = super::pm_draft_creation::continue_with_credentials(
        fixture.remote.repo.as_ref(),
        fixture.operation().await,
        &fixture.coordinator(),
    )
    .await
    .unwrap();
    assert_eq!(response.session_id, fixture.remote.operation.session_id);
    let saved = fixture.operation().await;
    fixture.prepare().await.unwrap();
    assert_eq!(saved.credentials, fixture.operation().await.credentials);
    let encoded = serde_json::to_string(&saved).unwrap();
    assert!(!encoded.contains(PARENT) && !encoded.contains(CHILD));
    assert!(
        !serde_json::to_string(&saved.response())
            .unwrap()
            .contains("credentials")
    );
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 3);
    assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 2);
    assert!(
        fixture
            .remote
            .repo
            .list_session_agent_runs(saved.session_id.unwrap())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(!saved.response().dispatch_allowed);
}

#[tokio::test]
async fn pm_credentials_pg_partial_success_keeps_receipt_and_checks_current_context_on_replay() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.remote.mode.store(2, Ordering::SeqCst);
    let error = fixture.prepare().await.unwrap_err();
    assert!(!error.to_string().contains("private Tracker detail"));
    let saved = fixture.operation().await;
    assert!(saved.credentials.as_ref().unwrap().receipt.is_some());
    for mode in [3, 5, 6, 7] {
        fixture.remote.mode.store(mode, Ordering::SeqCst);
        assert!(fixture.prepare().await.is_err(), "mode={mode}");
        assert_eq!(saved.credentials, fixture.operation().await.credentials);
    }
    fixture.remote.mode.store(0, Ordering::SeqCst);
    fixture.prepare().await.unwrap();
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    assert_eq!(saved.credentials, fixture.operation().await.credentials);
}

#[tokio::test]
async fn pm_credentials_pg_rotated_parent_origins_or_ttl_conflict_before_external_request() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.prepare().await.unwrap();
    let saved = fixture.operation().await;
    for kind in 0..4 {
        let mut config = fixture.config.clone();
        match kind {
            0 => {
                config.pm.credentials.parent_pat = "sdlc_pat_rotated-test-secret-1234567890".into()
            }
            1 => config.pm.credentials.auth_url = "http://127.0.0.1:1/".into(),
            2 => config.tracker.url = "http://127.0.0.1:1/".into(),
            _ => config.pm.credentials.ttl_seconds = 301,
        }
        let coordinator = PmCredentialCoordinator::configured(&config)
            .unwrap()
            .unwrap();
        assert!(matches!(
            coordinator
                .prepare_credential(fixture.remote.repo.as_ref(), &saved)
                .await,
            Err(shared::AppError::Conflict(_))
        ));
    }
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    assert_eq!(saved.credentials, fixture.operation().await.credentials);
}

#[tokio::test]
async fn pm_credentials_pg_concurrent_preparation_freezes_one_receipt_and_rejects_payload_conflict()
{
    let Some(fixture) = fixture().await else {
        return;
    };
    let (first, second) = tokio::join!(fixture.prepare(), fixture.prepare());
    first.unwrap();
    second.unwrap();
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    let saved = fixture.operation().await;
    let mut intent = saved.credentials.as_ref().unwrap().intent.clone();
    intent.command["scopes"] = json!(["task-tracker:read"]);
    intent.request_sha256 = domain::pm_canonical_hash(&intent.command);
    assert!(matches!(
        fixture
            .remote
            .repo
            .record_pm_draft_proof(
                saved.id,
                saved.owner_user_id,
                domain::PmDraftProof::CredentialIntent(intent)
            )
            .await,
        Err(shared::AppError::Conflict(_))
    ));
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 2);
    assert_eq!(saved.credentials, fixture.operation().await.credentials);
}

#[tokio::test]
async fn pm_credentials_pg_revoked_child_cannot_be_used() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.prepare().await.unwrap();
    fixture.remote.mode.store(4, Ordering::SeqCst);
    assert!(matches!(
        fixture.prepare().await,
        Err(shared::AppError::Forbidden)
    ));
    assert_eq!(fixture.remote.contexts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn pm_credentials_pg_expired_receipt_stops_before_external_renewal() {
    let Some(mut fixture) = fixture().await else {
        return;
    };
    fixture.config.pm.credentials.ttl_seconds = 1;
    fixture.prepare().await.unwrap();
    tokio::time::sleep(Duration::from_millis(1100)).await;
    assert!(matches!(
        fixture.prepare().await,
        Err(shared::AppError::Unauthorized)
    ));
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn pm_credentials_pg_database_guards_monotonic_journal_and_single_redacted_audit() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.prepare().await.unwrap();
    fixture.prepare().await.unwrap();
    let saved = fixture.operation().await;
    let original = serde_json::to_value(&saved).unwrap();
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let mut erase = original.clone();
    erase.as_object_mut().unwrap().remove("credentials");
    let mut pending = original.clone();
    pending["credentials"]["receipt"] = Value::Null;
    let mut parent = original.clone();
    parent["credentials"]["intent"]["parent_fingerprint"] = json!("a".repeat(64));
    let mut receipt = original.clone();
    receipt["credentials"]["receipt"]["token_id"] = json!(Uuid::new_v4());
    let mut secret = original.clone();
    secret["credentials"]["receipt"]["secret"] = json!(CHILD);
    let mut payload = original.clone();
    payload["request"]["title"] = json!("Different task");
    for value in [erase, pending, parent, receipt, secret, payload] {
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE pm_draft_creation_operations SET operation=$2 WHERE id=$1",
                [saved.id.into(), value.into()]
            ))
            .await
            .is_err()
        );
    }
    assert!(
        db.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "DELETE FROM pm_draft_creation_operations WHERE id=$1",
            [saved.id.into()]
        ))
        .await
        .is_err()
    );
    assert_eq!(fixture.operation().await.credentials, saved.credentials);
    let rows = db.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT action,payload FROM audit_log WHERE entity_type='pm_draft_operation' AND entity_id=$1 ORDER BY action",
        [saved.id.to_string().into()])).await.unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        let action: String = row.try_get("", "action").unwrap();
        assert!(matches!(
            action.as_str(),
            "pm_credentials.intent" | "pm_credentials.acknowledged"
        ));
        let json: Value = row.try_get("", "payload").unwrap();
        let bytes = json.to_string();
        assert!(
            !bytes.contains(PARENT)
                && !bytes.contains(CHILD)
                && !bytes.contains("parent_fingerprint")
        );
    }
    let ledger_before = migration::Migrator::get_migration_models(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.version, row.applied_at))
        .collect::<std::collections::BTreeMap<_, _>>();
    // Later migrations must not change which recovery guard this case exercises.
    let credential_migration = migration::Migrator::migrations()
        .into_iter()
        .find(|item| item.name() == "m20261004_000011_pm_credentials")
        .expect("credential migration must remain registered");
    let error = credential_migration
        .down(&migration::SchemaManager::new(&db))
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("PM credential journals require explicit reconciliation before downgrade")
    );
    let ledger_after = migration::Migrator::get_migration_models(&db)
        .await
        .unwrap()
        .into_iter()
        .map(|row| (row.version, row.applied_at))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(ledger_before, ledger_after);
    assert_eq!(fixture.operation().await.credentials, saved.credentials);
}

#[tokio::test]
async fn pm_credentials_pg_first_intent_and_ack_require_readable_strict_shapes() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let op = fixture.operation().await;
    let command = serde_json::to_value(
        PmCredentialCommand::tracker(&op.execution_identity().unwrap(), op.credential_key(), 300)
            .unwrap(),
    )
    .unwrap();
    let intent = domain::PmCredentialIntent {
        request_sha256: domain::pm_canonical_hash(&command),
        command,
        parent_fingerprint: "a".repeat(64),
        base_origin: fixture.config.pm.credentials.auth_url.clone(),
        tracker_origin: fixture.config.tracker.url.clone(),
        machine_subject: fixture.remote.subject.clone(),
    };
    let mut valid = serde_json::to_value(&op).unwrap();
    valid["credentials"] = json!({"intent":intent,"receipt":null});
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    for field in [
        "base_origin",
        "tracker_origin",
        "machine_subject",
        "parent_fingerprint",
        "request_sha256",
    ] {
        let mut value = valid.clone();
        value["credentials"]["intent"][field] = json!({});
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE pm_draft_creation_operations SET operation=$2 WHERE id=$1",
                [op.id.into(), value.into()]
            ))
            .await
            .is_err()
        );
        assert!(fixture.operation().await.credentials.is_none());
    }
    for field in ["parent_fingerprint", "request_sha256"] {
        let expression = format!(
            "UPDATE pm_draft_creation_operations SET operation=jsonb_set($2,'{{credentials,intent,{field}}}',to_jsonb(1111111111111111111111111111111111111111111111111111111111111111::numeric)) WHERE id=$1"
        );
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                expression,
                [op.id.into(), valid.clone().into()]
            ))
            .await
            .is_err()
        );
        assert!(fixture.operation().await.credentials.is_none());
    }
    fixture
        .remote
        .repo
        .record_pm_draft_proof(
            op.id,
            op.owner_user_id,
            domain::PmDraftProof::CredentialIntent(intent),
        )
        .await
        .unwrap();
    let pending = fixture.operation().await;
    for expiry in [
        "not-a-date",
        "2026-02-30T12:00:00Z",
        "infinity",
        "2026-10-04T12:00:00.1234567890Z",
        "2026-10-04T24:00:00Z",
        "2026-10-04T12:00:00+24:00",
        "2026-10-04T12:00:60Z",
    ] {
        let mut value = serde_json::to_value(&pending).unwrap();
        value["credentials"]["receipt"] =
            json!({"token_id":Uuid::new_v4(),"expires_at":expiry,"scopes":fixture.remote.scopes});
        assert!(
            db.execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE pm_draft_creation_operations SET operation=$2 WHERE id=$1",
                [op.id.into(), value.into()]
            ))
            .await
            .is_err()
        );
        assert!(
            fixture
                .operation()
                .await
                .credentials
                .unwrap()
                .receipt
                .is_none()
        );
    }
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn pm_credentials_pg_revoked_parent_prevents_post_and_does_not_erase_ack() {
    let Some(fixture) = fixture().await else {
        return;
    };
    fixture.prepare().await.unwrap();
    let saved = fixture.operation().await;
    fixture.remote.mode.store(8, Ordering::SeqCst);
    assert!(matches!(
        fixture.prepare().await,
        Err(shared::AppError::Unauthorized)
    ));
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 1);
    assert_eq!(saved.credentials, fixture.operation().await.credentials);
}

#[tokio::test]
async fn pm_credentials_pg_audit_failure_rolls_back_journal_and_recovers_original_child() {
    let Some(fixture) = fixture().await else {
        return;
    };
    let db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let id = fixture.remote.operation.id;
    let guard = format!("fleet_test_credential_audit_{}", id.simple());
    for action in ["pm_credentials.intent", "pm_credentials.acknowledged"] {
        // Inject at the audit boundary after the journal UPDATE in the same transaction.
        db.execute_unprepared(&format!(
            "CREATE FUNCTION {guard}() RETURNS trigger AS $$ BEGIN
             IF NEW.entity_type='pm_draft_operation' AND NEW.entity_id=TG_ARGV[0]
                AND NEW.action=TG_ARGV[1] THEN
                RAISE EXCEPTION 'injected credential audit failure';
             END IF; RETURN NEW; END; $$ LANGUAGE plpgsql;
             CREATE TRIGGER {guard} BEFORE INSERT ON audit_log
                FOR EACH ROW EXECUTE FUNCTION {guard}('{id}','{action}');"
        ))
        .await
        .unwrap();
        assert!(fixture.prepare().await.is_err());
        db.execute_unprepared(&format!(
            "DROP TRIGGER {guard} ON audit_log; DROP FUNCTION {guard}();"
        ))
        .await
        .unwrap();
        let saved = fixture.operation().await;
        if action == "pm_credentials.intent" {
            assert!(saved.credentials.is_none());
            assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 0);
        } else {
            assert!(saved.credentials.unwrap().receipt.is_none());
            assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 1);
            assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
        }
    }
    let issued_id = fixture.remote.receipt.lock().await.as_ref().unwrap()["token_id"]
        .as_str()
        .unwrap()
        .to_string();
    fixture.prepare().await.unwrap();
    let receipt = fixture
        .operation()
        .await
        .credentials
        .unwrap()
        .receipt
        .unwrap();
    assert_eq!(receipt.token_id.to_string(), issued_id);
    assert_eq!(fixture.remote.posts.load(Ordering::SeqCst), 2);
    assert_eq!(fixture.remote.children.load(Ordering::SeqCst), 1);
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT action FROM audit_log WHERE entity_type='pm_draft_operation'
             AND entity_id=$1 ORDER BY action",
            [id.to_string().into()],
        ))
        .await
        .unwrap();
    let actions: Vec<String> = rows
        .into_iter()
        .map(|row| row.try_get("", "action").unwrap())
        .collect();
    assert_eq!(
        actions,
        ["pm_credentials.acknowledged", "pm_credentials.intent"]
    );
}
