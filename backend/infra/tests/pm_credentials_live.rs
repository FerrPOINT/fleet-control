//! Opt-in real Base/Tracker TCP interoperability. See scripts/pm_credentials_live/README.md.
use domain::{
    PmExecutionIdentity, TrackerCreatedDraft, TrackerDraftReservationReadback,
    TrackerPmDraftReservation, TrackerStage, TrackerTaskContext,
};
use infra::pm_credentials::{PmCredentialCommand, PmCredentialIssuer};
use reqwest::{Client, StatusCode, header};
use sea_orm::{ConnectionTrait, Database, DatabaseBackend, Statement};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{path::PathBuf, time::Duration};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    base_url: String,
    tracker_url: String,
    auth_database_url: String,
    tracker_database_url: String,
    owner_bearer: String,
    pm_browser_bearer: String,
    parent_pat: String,
    parent_id: Uuid,
    owner_subject: String,
    machine_subject: String,
    owner_local_id: Uuid,
    pm_local_id: Uuid,
    project_id: Uuid,
    agent_id: Uuid,
    tracker_instance_id: String,
}

fn canonical<T: DeserializeOwned + Serialize>(value: Value) -> T {
    let typed: T = serde_json::from_value(value.clone()).expect("producer DTO must decode");
    assert_eq!(serde_json::to_value(&typed).unwrap(), value);
    typed
}

async fn json_response(response: reqwest::Response, expected: StatusCode) -> Value {
    assert_eq!(
        response.status(),
        expected,
        "unexpected producer HTTP status"
    );
    let mut response = response;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.expect("producer body transport") {
        assert!(bytes.len().saturating_add(chunk.len()) <= 262_144);
        bytes.extend_from_slice(&chunk);
    }
    // Tracker's legacy auth middleware returns a bare status, not a JSON error DTO.
    if bytes.is_empty() && expected.is_client_error() {
        return Value::Null;
    }
    serde_json::from_slice(&bytes).expect("producer JSON body")
}

async fn draft(client: &Client, fixture: &Fixture, title: &str) -> TrackerCreatedDraft {
    canonical(
        json_response(
            client
                .post(format!(
                    "{}/api/v1/projects/{}/sdlc/drafts",
                    fixture.tracker_url, fixture.project_id
                ))
                .bearer_auth(&fixture.owner_bearer)
                .json(
                    &json!({"title":title,"description":"private interop fixture input",
                    "idempotency_key":Uuid::new_v4().to_string()}),
                )
                .send()
                .await
                .expect("real Tracker Draft request"),
            StatusCode::CREATED,
        )
        .await,
    )
}

#[tokio::test]
#[ignore = "requires scripts/pm_credentials_live/run.py and disposable real producers"]
async fn actual_base_child_authorizes_current_tracker_assignment() {
    let path = PathBuf::from(
        std::env::var("FLEET_PM_LIVE_FIXTURE").expect("explicit isolated fixture is required"),
    );
    let fixture: Fixture =
        serde_json::from_slice(&std::fs::read(path).expect("private fixture file"))
            .expect("fixture shape");
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .no_proxy()
        .build()
        .unwrap();
    let task = draft(&client, &fixture, "PM interop private root").await;
    let foreign = draft(&client, &fixture, "PM interop private foreign").await;
    assert_eq!(task.tracker_instance_id, fixture.tracker_instance_id);
    assert_eq!(task.owner_subject, fixture.owner_subject);
    assert_eq!(task.root_task_id, task.task_id);
    let assignment_url = format!(
        "{}/api/v1/issues/{}/sdlc/pm-draft-assignment",
        fixture.tracker_url, task.task_id
    );
    let reservation_key = Uuid::new_v4().to_string();
    let reservation: TrackerPmDraftReservation = canonical(
        json_response(
            client
                .post(&assignment_url)
                .bearer_auth(&fixture.owner_bearer)
                .json(
                    &json!({"expected_owner_version":0,"expected_assignment_version":null,
                    "requested_agent_id":fixture.agent_id,"idempotency_key":reservation_key}),
                )
                .send()
                .await
                .expect("real Tracker reservation"),
            StatusCode::CREATED,
        )
        .await,
    );
    assert_eq!(reservation.variant, "pm_draft_reserved");
    assert_eq!(reservation.admission_state, "reserved");
    assert!(!reservation.dispatch_allowed);
    assert_eq!(
        reservation.assignment.machine_subject,
        fixture.machine_subject
    );
    assert_eq!(reservation.assignment.agent_id, fixture.agent_id);
    assert_eq!(reservation.binding.task_id, task.task_id);
    assert_eq!(reservation.binding.project_id, fixture.project_id);
    assert_eq!(reservation.binding.owner_subject, fixture.owner_subject);
    let readback: TrackerDraftReservationReadback = canonical(
        json_response(
            client
                .get(&assignment_url)
                .query(&[("idempotency_key", &reservation_key)])
                .bearer_auth(&fixture.owner_bearer)
                .send()
                .await
                .unwrap(),
            StatusCode::OK,
        )
        .await,
    );
    assert_eq!(readback.current.as_ref(), Some(&reservation));
    assert_eq!(readback.operation.unwrap().result, reservation);

    // Every execution field comes from the real immutable owner receipt, not a fixture claim.
    let identity = PmExecutionIdentity {
        task: reservation.execution.key.clone(),
        execution_ref: reservation.assignment.execution_id.to_string(),
        tracker_instance_ref: reservation.binding.tracker_instance_id.clone(),
        tracker_project_ref: reservation.binding.project_id.to_string(),
        task_ref: reservation.binding.task_id.to_string(),
        root_ref: reservation.binding.root_task_id.to_string(),
        agent_ref: reservation.assignment.agent_id.to_string(),
        assignment_operation_key: reservation.assignment_operation_key.clone(),
        assignment_ref: reservation.assignment.assignment_id.to_string(),
        assignment_revision: i64::try_from(reservation.assignment.version).unwrap(),
    };
    identity.validate().unwrap();
    let operation_key = format!("fleet-pm-credential:{}", Uuid::new_v4());
    let command = PmCredentialCommand::tracker(&identity, operation_key.clone(), 300).unwrap();
    let issuer =
        PmCredentialIssuer::new(&fixture.base_url, &fixture.tracker_url, &fixture.parent_pat)
            .unwrap();
    let child = issuer
        .issue(&command)
        .await
        .expect("actual Base delegation");
    let context_url = format!(
        "{}/api/v1/issues/{}/sdlc/context",
        fixture.tracker_url, task.task_id,
    );
    let request = child.authorize(client.get(&context_url)).unwrap();
    let authorization = request.headers()[header::AUTHORIZATION].clone();
    assert!(authorization.is_sensitive());
    let context: TrackerTaskContext =
        canonical(json_response(client.execute(request).await.unwrap(), StatusCode::OK).await);
    assert_eq!(context.contract_version, 1);
    assert_eq!(
        context.tracker_instance_id,
        reservation.binding.tracker_instance_id
    );
    assert_eq!(context.project_id, reservation.binding.project_id);
    assert_eq!(context.task_id, reservation.binding.task_id);
    assert_eq!(context.root_task_id, reservation.binding.root_task_id);
    assert_eq!(context.owner_subject, reservation.binding.owner_subject);
    assert!(matches!(context.stage, TrackerStage::Draft));
    assert!(!context.permissions.can_answer && !context.permissions.can_confirm);
    let current = context.assignment.unwrap();
    assert_eq!(current.assignment_id, reservation.assignment.assignment_id);
    assert_eq!(current.execution_id, reservation.assignment.execution_id);
    assert_eq!(current.agent_id, reservation.assignment.agent_id);
    assert_eq!(current.version, identity.assignment_revision);
    assert_eq!(current.machine_subject, fixture.machine_subject);

    let principal = json_response(
        client
            .get(format!("{}/auth/tokens/introspect", fixture.base_url))
            .header(header::AUTHORIZATION, authorization.clone())
            .send()
            .await
            .unwrap(),
        StatusCode::OK,
    )
    .await;
    assert_eq!(principal["sub"], fixture.machine_subject);
    let mut actual_scopes: Vec<String> =
        serde_json::from_value(principal["scopes"].clone()).unwrap();
    actual_scopes.sort_unstable();
    assert_eq!(actual_scopes, command.scopes());

    let replay = issuer
        .issue(&command)
        .await
        .expect("actual Base exact replay");
    assert_eq!(replay.token_id(), child.token_id());
    assert_eq!(replay.expires_at(), child.expires_at());
    let replay_request = replay.authorize(client.get(&context_url)).unwrap();
    // Never print either bearer on an assertion failure.
    assert!(replay_request.headers()[header::AUTHORIZATION] == authorization);
    json_response(
        client.execute(replay_request).await.unwrap(),
        StatusCode::OK,
    )
    .await;

    let auth_db = Database::connect(&fixture.auth_database_url).await.unwrap();
    let row = auth_db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT parent_token_id,user_id,scopes FROM personal_tokens WHERE id=$1",
            [child.token_id().into()],
        ))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        row.try_get::<Uuid>("", "parent_token_id").unwrap(),
        fixture.parent_id
    );
    assert_eq!(
        row.try_get::<Uuid>("", "user_id").unwrap().to_string(),
        fixture.machine_subject
    );
    assert_eq!(
        row.try_get::<Vec<String>>("", "scopes").unwrap(),
        command.scopes()
    );
    let row = auth_db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT count(*) AS n FROM personal_tokens WHERE parent_token_id=$1 AND idempotency_key=$2",
        [fixture.parent_id.into(), operation_key.into()])).await.unwrap().unwrap();
    assert_eq!(row.try_get::<i64>("", "n").unwrap(), 1);
    let tracker_db = Database::connect(&fixture.tracker_database_url)
        .await
        .unwrap();
    for (local_id, subject) in [
        (fixture.owner_local_id, &fixture.owner_subject),
        (fixture.pm_local_id, &fixture.machine_subject),
    ] {
        assert_ne!(local_id.to_string(), *subject);
        let row = tracker_db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT central_sub,is_active FROM users WHERE id=$1",
                [local_id.into()],
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(row.try_get::<String>("", "central_sub").unwrap(), *subject);
        assert!(row.try_get::<bool>("", "is_active").unwrap());
    }
    println!("LIVE_PM_INTEROP actual issuance/current-context/exact-replay/lineage verified");

    // Bypass the client allowlist only to exercise the actual server's independent boundary.
    for path in [
        format!("issues/{}/sdlc/context", foreign.task_id),
        format!("issues/{}", task.task_id),
        format!("issues/{}/sdlc/pm-draft-assignment", task.task_id),
    ] {
        let denial = json_response(
            client
                .get(format!("{}/api/v1/{path}", fixture.tracker_url))
                .header(header::AUTHORIZATION, authorization.clone())
                .send()
                .await
                .unwrap(),
            StatusCode::FORBIDDEN,
        )
        .await;
        assert!(!denial.to_string().contains("private"));
    }
    json_response(
        client
            .post(format!(
                "{}/api/v1/issues/{}/sdlc/requirements/1/confirm",
                fixture.tracker_url, task.task_id
            ))
            .header(header::AUTHORIZATION, authorization.clone())
            .json(&json!({}))
            .send()
            .await
            .unwrap(),
        StatusCode::FORBIDDEN,
    )
    .await;
    let response = client
        .delete(format!(
            "{}/auth/tokens/{}",
            fixture.base_url, fixture.parent_id
        ))
        .bearer_auth(&fixture.pm_browser_bearer)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    json_response(
        client
            .get(&context_url)
            .header(header::AUTHORIZATION, authorization.clone())
            .send()
            .await
            .unwrap(),
        StatusCode::UNAUTHORIZED,
    )
    .await;
    json_response(
        client
            .get(format!("{}/auth/tokens/introspect", fixture.base_url))
            .header(header::AUTHORIZATION, authorization)
            .send()
            .await
            .unwrap(),
        StatusCode::UNAUTHORIZED,
    )
    .await;
    assert!(matches!(
        issuer.issue(&command).await,
        Err(shared::AppError::Unauthorized)
    ));
    println!(
        "LIVE_PM_INTEROP issuance/current-context/exact-replay/foreign/legacy/owner-only/parent-revocation passed; no admission or dispatch"
    );
}
