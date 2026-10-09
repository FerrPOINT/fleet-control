use domain::PmExecutionIdentity;
use infra::pm_credentials::{PmCredentialCommand, PmCredentialIssuer};
use reqwest::{Client, StatusCode, Url, header};
use serde_json::Value;
use shared::AppError;
use std::time::Duration;
use uuid::Uuid;

fn required(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("required disposable Auth test input is missing"))
}

#[tokio::test]
#[ignore = "requires source-qualified disposable real Base Auth, not an HTTP stub"]
async fn real_base_delegation_wire_replay_conflict_and_revoke_match_fleet() {
    assert_eq!(required("FLEET_REAL_AUTH_TEST_OWNED"), "disposable-compose");
    assert_eq!(
        required("FLEET_REAL_AUTH_TEST_SOURCE_SHA"),
        "8a345988d8e0815c2d6d7dfd597822df2b8ae992"
    );
    let url = Url::parse(&required("FLEET_REAL_AUTH_TEST_URL"))
        .unwrap_or_else(|_| panic!("invalid disposable Auth test origin"));
    assert_eq!(url.scheme(), "http");
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    assert!(url.port().is_some_and(|port| port >= 40000));
    assert!(url.username().is_empty() && url.password().is_none());
    assert!(url.query().is_none() && url.fragment().is_none());
    assert_eq!(url.path(), "/");
    let parent = required("FLEET_REAL_AUTH_TEST_PARENT_PAT");
    let subject = required("FLEET_REAL_AUTH_TEST_SUBJECT");
    let subject_id = Uuid::parse_str(&subject).expect("invalid synthetic Auth subject");
    assert!(!subject_id.is_nil());
    assert_eq!(subject_id.to_string(), subject);
    let client = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let task = Uuid::new_v4().to_string();
    let identity = PmExecutionIdentity {
        task: "SDLC-1".into(),
        execution_ref: Uuid::new_v4().to_string(),
        tracker_instance_ref: "real-auth-contract-test".into(),
        tracker_project_ref: Uuid::new_v4().to_string(),
        task_ref: task.clone(),
        root_ref: task,
        agent_ref: Uuid::new_v4().to_string(),
        assignment_operation_key: "real-auth-assignment".into(),
        assignment_ref: Uuid::new_v4().to_string(),
        assignment_revision: 1,
    };
    let key = format!("fleet-real-auth:{}", Uuid::new_v4());
    let command = PmCredentialCommand::tracker(&identity, key.clone(), 60).unwrap();
    let issuer =
        PmCredentialIssuer::new(url.as_str(), "https://tracker.example.test/", &parent).unwrap();
    assert!(!format!("{issuer:?}").contains(&parent));
    let issued = issuer.issue(&command).await.unwrap();
    let replay = issuer.issue(&command).await.unwrap();
    assert_eq!(issued.token_id(), replay.token_id());
    assert_eq!(issued.expires_at(), replay.expires_at());

    // Build only: no request is sent to Tracker and no assignment authority is claimed.
    let bound_request = issued
        .authorize(client.get(format!(
            "https://tracker.example.test/api/v1/issues/{}/sdlc/context",
            identity.task_ref
        )))
        .unwrap();
    let child = bound_request.headers()[header::AUTHORIZATION].clone();
    assert!(child.is_sensitive());
    let replay_request = replay
        .authorize(client.get(format!(
            "https://tracker.example.test/api/v1/issues/{}/sdlc/context",
            identity.task_ref
        )))
        .unwrap();
    assert!(replay_request.headers()[header::AUTHORIZATION] == child);
    let introspection = client
        .get(url.join("auth/tokens/introspect").unwrap())
        .header(header::AUTHORIZATION, child.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(introspection.status(), StatusCode::OK);
    let principal: Value = introspection.json().await.unwrap();
    assert_eq!(principal.as_object().unwrap().len(), 4);
    assert_eq!(principal["sub"], subject);
    assert!(principal["email"].is_string() && principal["display_name"].is_string());
    assert_eq!(principal["scopes"], serde_json::json!(command.scopes()));
    let changed = PmCredentialCommand::tracker(&identity, key, 59).unwrap();
    assert!(matches!(
        issuer.issue(&changed).await,
        Err(AppError::Conflict(_))
    ));
    issuer.revoke(&issued).await.unwrap();
    let revoked = client
        .get(url.join("auth/tokens/introspect").unwrap())
        .header(header::AUTHORIZATION, child)
        .send()
        .await
        .unwrap();
    assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);
}
