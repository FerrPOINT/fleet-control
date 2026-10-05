use super::*;
use domain::{ApprovalChoice, ApprovalDecision, ApprovalDecisionState};
use std::os::unix::fs::PermissionsExt;

#[path = "native_approval_recovery.rs"]
mod native_approval_recovery;

#[path = "native_approval_restart.rs"]
mod native_approval_restart;

#[derive(Default)]
struct ApprovalModel {
    files: Mutex<HashMap<String, String>>,
    calls: Mutex<HashMap<String, usize>>,
    first_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
    tool_gate: Mutex<Option<Arc<tokio::sync::Notify>>>,
}

async fn approval_inference(
    State(model): State<Arc<ApprovalModel>>,
    Json(body): Json<Value>,
) -> Response {
    assert_eq!(body["model"], "fleet-managed-local-model");
    let messages = body["messages"].as_array().unwrap();
    let prompt = messages
        .iter()
        .rev()
        .find(|m| m["role"] == "user")
        .and_then(|m| m["content"].as_str())
        .unwrap();
    let filename = model.files.lock().await.get(prompt).cloned().unwrap();
    *model.calls.lock().await.entry(prompt.into()).or_default() += 1;
    let tool_returned = messages.iter().any(|m| m["role"] == "tool");
    if !tool_returned {
        let gate = model.first_gate.lock().await.clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
    } else {
        let gate = model.tool_gate.lock().await.clone();
        if let Some(gate) = gate {
            gate.notified().await;
        }
    }
    let (message, finish) = if tool_returned {
        (
            json!({"role":"assistant","content":"Native tool decision observed; no SDLC success claimed."}),
            "stop",
        )
    } else {
        assert!(
            body["tools"]
                .as_array()
                .unwrap()
                .iter()
                .any(|tool| tool["function"]["name"] == "terminal")
        );
        let arguments =
            json!({"command":format!("chmod 666 ./{filename}"),"timeout":5}).to_string();
        (
            json!({"role":"assistant","content":null,"tool_calls":[{"id":"call_fleet_native_approval",
            "type":"function","function":{"name":"terminal","arguments":arguments}}]}),
            "tool_calls",
        )
    };
    if body["stream"] == true {
        let mut delta = message.clone();
        if let Some(calls) = delta["tool_calls"].as_array_mut() {
            calls[0]["index"] = json!(0);
        }
        let chunk = json!({"id":"chatcmpl-native-approval","object":"chat.completion.chunk",
            "created":1,"model":"fleet-managed-local-model","choices":[{"index":0,"delta":delta,"finish_reason":null}]});
        let end = json!({"id":"chatcmpl-native-approval","object":"chat.completion.chunk",
            "created":1,"model":"fleet-managed-local-model","choices":[{"index":0,"delta":{},"finish_reason":finish}]});
        return (
            [("content-type", "text/event-stream")],
            format!("data: {chunk}\n\ndata: {end}\n\ndata: [DONE]\n\n"),
        )
            .into_response();
    }
    Json(json!({"id":"chatcmpl-native-approval","object":"chat.completion","created":1,
        "model":"fleet-managed-local-model","choices":[{"index":0,"message":message,"finish_reason":finish}],
        "usage":{"prompt_tokens":12,"completion_tokens":5,"total_tokens":17}})).into_response()
}

#[tokio::test]
#[ignore = "requires exact native Hermes image and disposable owned PostgreSQL/agent roots"]
async fn managed_native_approval_decisions_are_exact_once_and_unknown_ack_is_held() {
    native_approval_scenario(false).await;
}

#[tokio::test]
#[ignore = "requires exact native Hermes/control plugin and disposable owned PostgreSQL/agent roots"]
async fn managed_native_original_approval_outcomes_recover_lost_http_ack() {
    native_approval_scenario(true).await;
}

async fn native_approval_scenario(outcomes: bool) {
    let db = native_database().await;
    let owner = Uuid::new_v4();
    let stranger = Uuid::new_v4();
    for id in [owner, stranger] {
        db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO users(id,email,username,display_name,password_hash,is_system_admin,system_role)
             VALUES($1,$2,$3,'Native approval owner','disabled',false,'user')",
            [id.into(),format!("{id}@example.test").into(),id.to_string().into()])).await.unwrap();
    }
    let audit_db = sea_orm::Database::connect(std::env::var("FLEET_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let repo = Arc::new(PostgresFleetRepository::new(db));
    repo.ensure_runtime_templates().await.unwrap();
    let model = Arc::new(ApprovalModel::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let router = Router::new().route("/v1/chat/completions",post(approval_inference))
        .route("/v1/models",get(|| async { Json(json!({"object":"list","data":[{"id":"fleet-managed-local-model","object":"model"}]})) }))
        .with_state(model.clone());
    let model_server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut config = AppConfig::default();
    config.fleet.agents_root = "/tmp/fleet-native-supervisor/approval-agents".into();
    config.fleet.hermes_source = "/opt/hermes".into();
    config.fleet.hermes_command = "/opt/fleet-hermes/bin/hermes".into();
    config.fleet.runtime_token_secret = format!("owned-native-approvals-{}", Uuid::new_v4());
    config.fleet.agent_port_base = 29400;
    config.fleet.agent_port_stride = 5;
    config.fleet.project_workflow_url = None;
    config.fleet.hermes_control_outcome_enabled = outcomes;
    let config = Arc::new(config);
    let root = Path::new("/tmp/fleet-native-supervisor/approval-fault");
    tokio::fs::create_dir(root).await.unwrap();
    let (events, _) = tokio::sync::broadcast::channel(32);
    let runtime = Arc::new(LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    let agent = create_agent(&repo, &config, "Native approval agent").await;
    let plugin = Path::new(&agent.paths.config).join("plugins/fleet-native-approval-observer");
    tokio::fs::create_dir_all(&plugin).await.unwrap();
    tokio::fs::write(
        plugin.join("__init__.py"),
        include_str!("../../../../scripts/native_supervisor_live/approval_fault_plugin.py"),
    )
    .await
    .unwrap();
    tokio::fs::write(plugin.join("plugin.yaml"),b"name: fleet-native-approval-observer\nversion: 1.0.0\nkind: platform\nplatforms:\n  - api_server\n").await.unwrap();
    let mut desired = configuration(port, "FLEET_NATIVE_APPROVAL_SOUL");
    desired.soul_md =
        "# Native approvals\nUse only the requested owned QA terminal command.\n".into();
    desired.config_json["platform_toolsets"]["api_server"] = json!(["terminal"]);
    desired.config_json["approvals"] = json!({"mode":"manual","timeout":120});
    desired.config_json["plugins"] = json!({"enabled":["fleet-native-approval-observer"]});
    if outcomes {
        let controls = Path::new(&agent.paths.config).join("plugins/fleet-hermes-controls");
        tokio::fs::create_dir_all(&controls).await.unwrap();
        for name in ["__init__.py", "plugin.py", "store.py", "plugin.yaml"] {
            tokio::fs::copy(
                Path::new("/qa/control-plugin").join(name),
                controls.join(name),
            )
            .await
            .unwrap();
        }
        desired.config_json["plugins"] =
            json!({"enabled":["fleet-hermes-controls","fleet-native-approval-observer"]});
    }
    desired.env_json["TERMINAL_ENV"] = json!("local");
    desired.env_json["TERMINAL_CWD"] = json!(agent.paths.workspace);
    desired.env_json["FLEET_NATIVE_SUPERVISOR_TEST"] = json!("1");
    desired.env_json["FLEET_NATIVE_APPROVAL_FAULT_ROOT"] = json!(root.to_str().unwrap());
    activate(&repo, agent.id, owner, desired).await;
    assert_eq!(
        runtime.start(&agent).await.unwrap().status,
        AgentStatus::Running
    );
    let (restart_tx, _) = tokio::sync::mpsc::channel(1);
    let ctx = Arc::new(app::AppContext::new(
        config.clone(),
        repo.clone(),
        Arc::new(FilesystemProvisioner),
        runtime.clone(),
        events,
        restart_tx,
    ));
    let owner_token = ctx
        .auth
        .issue_tokens(&repo.find_user_by_id(owner).await.unwrap().unwrap())
        .unwrap()
        .response
        .access_token;
    let stranger_token = ctx
        .auth
        .issue_tokens(&repo.find_user_by_id(stranger).await.unwrap().unwrap())
        .unwrap()
        .response
        .access_token;
    let fleet = Router::new()
        .route(
            "/api/v1/sessions/{session_id}/approvals/{approval_id}/decision",
            get(api::routes::approvals::read).post(api::routes::approvals::decide),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            ctx.clone(),
            api::middleware::require_auth,
        ))
        .with_state(ctx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let fleet_server = tokio::spawn(async move { axum::serve(listener, fleet).await.unwrap() });
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(20))
        .build()
        .unwrap();
    for (label, choice, lost) in [
        ("once", ApprovalChoice::Once, false),
        ("deny", ApprovalChoice::Deny, false),
        ("lost", ApprovalChoice::Once, true),
    ] {
        let filename = format!("approval-{label}.txt");
        let file = Path::new(&agent.paths.workspace).join(&filename);
        tokio::fs::write(&file, b"owned disposable native approval target")
            .await
            .unwrap();
        tokio::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600))
            .await
            .unwrap();
        let prompt = format!("managed-native-approval-{label}-{}", Uuid::new_v4());
        model.files.lock().await.insert(prompt.clone(), filename);
        let session = repo
            .create_session(
                CreateSessionRequest {
                    primary_agent_id: Some(agent.id),
                    agent_id: None,
                    title: format!("Native approval {label}"),
                    task_key: None,
                    leader_agent_id: None,
                    parent_session_id: None,
                    namespace_id: None,
                    idempotency_key: Some(Uuid::new_v4().to_string()),
                },
                owner,
            )
            .await
            .unwrap();
        let message = repo
            .create_session_message(
                session.id,
                CreateSessionMessageRequest {
                    body: prompt.clone(),
                    author_agent_id: None,
                    message_kind: Some(MessageKind::UserPrompt),
                    runtime_message_id: None,
                    idempotency_key: Some(Uuid::new_v4().to_string()),
                },
                owner,
            )
            .await
            .unwrap();
        let approval = timeout(Duration::from_secs(90), async {
            loop {
                let approvals = repo.list_session_approvals(session.id).await.unwrap();
                if let Some(approval) = approvals.into_iter().next() {
                    break approval;
                }
                for run in repo.list_session_agent_runs(session.id).await.unwrap() {
                    assert!(
                        !matches!(
                            run.state,
                            SessionRunState::Completed
                                | SessionRunState::Failed
                                | SessionRunState::Cancelled
                        ),
                        "native run ended without approval: {:?}",
                        run.last_error
                    );
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("native approval request was not mirrored");
        assert_eq!(approval.agent_id, agent.id);
        assert!(approval.runtime_approval_id.is_some());
        assert_eq!(
            tokio::fs::metadata(&file)
                .await
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        let before = dispatch_snapshot(
            &repo
                .get_hermes_dispatch_intent(message.id)
                .await
                .unwrap()
                .unwrap(),
        );
        let url = format!(
            "{base}/api/v1/sessions/{}/approvals/{}/decision",
            session.id, approval.id
        );
        let command = domain::ApprovalDecisionRequest {
            choice,
            idempotency_key: Uuid::new_v4().to_string(),
        };
        assert_eq!(
            client
                .post(&url)
                .bearer_auth(&stranger_token)
                .json(&command)
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::FORBIDDEN
        );
        assert_eq!(
            client
                .post(&url)
                .bearer_auth(&owner_token)
                .json(&json!({"choice":"always","idempotency_key":"forbidden-widening"}))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            native_observations(root)
                .await
                .iter()
                .filter(|row| row["kind"] == "approval_post")
                .count(),
            match label {
                "once" => 0,
                "deny" => 1,
                _ => 2,
            }
        );
        if lost {
            tokio::fs::write(root.join("drop-next-approval"), b"one owned ACK loss")
                .await
                .unwrap();
            if outcomes {
                tokio::fs::write(
                    root.join("hold-outcome-lookup"),
                    b"owned original witness hold",
                )
                .await
                .unwrap();
            }
        }
        let response = client
            .post(&url)
            .bearer_auth(&owner_token)
            .json(&command)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let decision = response.json::<ApprovalDecision>().await.unwrap();
        let expected = if lost {
            ApprovalDecisionState::Uncertain
        } else {
            ApprovalDecisionState::Delivered
        };
        assert_eq!(decision.state, expected);
        assert_eq!(decision.actor_user_id, owner);
        assert_eq!(decision.approval_id, approval.id);
        let completed = terminal(&repo, session.id).await;
        assert_eq!(completed.id, approval.session_run_id);
        assert_eq!(
            completed.runtime_run_id.as_deref(),
            Some(approval.runtime_run_id.as_str())
        );
        assert_eq!(
            tokio::fs::metadata(&file)
                .await
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            if choice == ApprovalChoice::Deny {
                0o600
            } else {
                0o666
            }
        );
        assert_eq!(
            dispatch_snapshot(
                &repo
                    .get_hermes_dispatch_intent(message.id)
                    .await
                    .unwrap()
                    .unwrap()
            ),
            before
        );
        if outcomes {
            let saved = repo
                .get_approval_outcome(decision.id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(saved.context["command_id"], decision.id.to_string());
            assert_eq!(saved.context["operation"], "approval");
            let body: Value =
                serde_json::from_str(saved.context["request_body"].as_str().unwrap()).unwrap();
            assert_eq!(
                body["request_id"].as_str(),
                approval.runtime_approval_id.as_deref()
            );
            assert_eq!(body["choice"], choice.as_str());
            assert_eq!(body["resolve_all"], false);
            if lost {
                timeout(Duration::from_secs(20), async {
                    loop {
                        if native_observations(root).await.iter().any(|row| {
                            row["kind"] == "approval_outcome_lookup"
                                && row["key"] == decision.id.to_string()
                                && row["held"] == true
                        }) {
                            break;
                        }
                        sleep(Duration::from_millis(100)).await;
                    }
                })
                .await
                .expect("original approval GET hold was not observed");
                assert_eq!(
                    repo.approval_decision(session.id, approval.id)
                        .await
                        .unwrap()
                        .state,
                    ApprovalDecisionState::Uncertain
                );
                tokio::fs::remove_file(root.join("hold-outcome-lookup"))
                    .await
                    .unwrap();
                timeout(Duration::from_secs(20), async {
                    loop {
                        if repo
                            .approval_decision(session.id, approval.id)
                            .await
                            .unwrap()
                            .state
                            == ApprovalDecisionState::Delivered
                        {
                            break;
                        }
                        sleep(Duration::from_millis(100)).await;
                    }
                })
                .await
                .expect("original native approval ACK was not recovered");
                let after = repo
                    .get_approval_outcome(decision.id)
                    .await
                    .unwrap()
                    .unwrap();
                assert_eq!(after.context, saved.context);
                let historical = repo.get_session_agent_run(completed.id).await.unwrap();
                assert_eq!(historical.state, completed.state);
                assert_eq!(historical.updated_at, completed.updated_at);
                assert!(
                    native_observations(root)
                        .await
                        .iter()
                        .any(|row| row["kind"] == "approval_outcome_lookup"
                            && row["key"] == decision.id.to_string()
                            && row["held"] == false)
                );
            }
            let audit = audit_db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "SELECT count(*) AS count FROM audit_log WHERE entity_id=$1 AND action='approval.decision_delivered'",
                [decision.id.to_string().into()])).await.unwrap().unwrap();
            assert_eq!(audit.try_get::<i64>("", "count").unwrap(), 1);
        }
        let expected = if outcomes {
            ApprovalDecisionState::Delivered
        } else {
            expected
        };
        let transcript_before_replay =
            serde_json::to_value(repo.list_session_messages(session.id).await.unwrap()).unwrap();
        for _ in 0..2 {
            let replay = client
                .post(&url)
                .bearer_auth(&owner_token)
                .json(&command)
                .send()
                .await
                .unwrap();
            assert_eq!(replay.status(), reqwest::StatusCode::OK);
            let replay = replay.json::<ApprovalDecision>().await.unwrap();
            assert_eq!(replay.id, decision.id);
            assert_eq!(replay.state, expected);
        }
        let read = client
            .get(&url)
            .bearer_auth(&owner_token)
            .send()
            .await
            .unwrap()
            .json::<ApprovalDecision>()
            .await
            .unwrap();
        assert_eq!(read.id, decision.id);
        assert_eq!(read.state, expected);
        let changed = domain::ApprovalDecisionRequest {
            choice: if choice == ApprovalChoice::Once {
                ApprovalChoice::Deny
            } else {
                ApprovalChoice::Once
            },
            idempotency_key: command.idempotency_key,
        };
        assert_eq!(
            client
                .post(&url)
                .bearer_auth(&owner_token)
                .json(&changed)
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::CONFLICT
        );
        let rows = native_observations(root).await;
        let posts: Vec<_> = rows
            .iter()
            .filter(|row| {
                row["kind"] == "approval_post" && row["run_id"] == approval.runtime_run_id
            })
            .collect();
        assert_eq!(posts.len(), 1);
        assert_eq!(
            posts[0]["request_id"].as_str(),
            approval.runtime_approval_id.as_deref()
        );
        assert_eq!(posts[0]["choice"], choice.as_str());
        assert_eq!(posts[0]["resolve_all"], false);
        if outcomes {
            let saved = repo
                .get_approval_outcome(decision.id)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(posts[0]["key"], decision.id.to_string());
            assert_eq!(
                posts[0]["store_id"],
                saved.context["capabilities"]["store_id"]
            );
            assert_eq!(posts[0]["sha256"], saved.context["request_sha256"]);
            assert_eq!(
                rows.iter()
                    .filter(|row| row["kind"] == "approval_ack"
                        && row["run_id"] == approval.runtime_run_id)
                    .count(),
                1
            );
        }
        assert_eq!(
            rows.iter()
                .filter(|row| row["kind"] == "approval_ack_dropped"
                    && row["run_id"] == approval.runtime_run_id)
                .count(),
            usize::from(lost)
        );
        assert_eq!(model.calls.lock().await.get(&prompt), Some(&2));
        assert_eq!(
            repo.list_session_approvals(session.id).await.unwrap().len(),
            1
        );
        let messages = repo.list_session_messages(session.id).await.unwrap();
        assert_eq!(
            serde_json::to_value(&messages).unwrap(),
            transcript_before_replay
        );
        assert_eq!(
            messages
                .iter()
                .filter(|m| {
                    m.author_type == domain::MessageAuthorType::Agent
                        && m.message_kind == MessageKind::AssistantMessage
                })
                .count(),
            1
        );
        assert!(
            messages
                .iter()
                .any(|m| m.message_kind == MessageKind::ToolEvent)
        );
    }
    assert_eq!(
        native_observations(root)
            .await
            .iter()
            .filter(|row| row["kind"] == "approval_post")
            .count(),
        3
    );
    assert_eq!(
        runtime.stop(&agent).await.unwrap().status,
        AgentStatus::Stopped
    );
    fleet_server.abort();
    model_server.abort();
    if outcomes {
        println!(
            "Original native approvals passed: real terminal guard, owner HTTP once/deny, one exact POST/native ACK/audit per UUID, lost HTTP ACK held through terminal then original GET recovery, immutable context/dispatch/terminal history. No Fleet OS restart, combined extensions, task/PM or safe descendants claimed."
        );
    } else {
        println!(
            "Managed native approvals passed: real terminal guard/request, owner HTTP once/deny, exact action, one POST per decision, lost real ACK remains uncertain after terminal/replay. No scoped task/PM admission or OS-descendant proof."
        );
    }
}
