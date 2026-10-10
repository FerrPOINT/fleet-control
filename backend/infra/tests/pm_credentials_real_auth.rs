use domain::PmExecutionIdentity;
use infra::pm_credentials::{PmCredentialCommand, PmCredentialIssuer};
use reqwest::{Client, StatusCode, Url, header};
use serde_json::Value;
use sha2::{Digest, Sha256};
use shared::AppError;
use std::{
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use uuid::Uuid;

fn required(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("required disposable Auth test input is missing"))
}

struct OwnedAuth {
    process: Child,
}

impl Drop for OwnedAuth {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

impl OwnedAuth {
    async fn start(client: &Client, url: &Url, policy: Value, registration_open: bool) -> Self {
        let binary = PathBuf::from(required("FLEET_REAL_AUTH_TEST_BINARY"));
        assert!(binary.is_absolute() && binary.is_file());
        let digest = hex::encode(Sha256::digest(
            std::fs::read(&binary).expect("cannot read disposable Auth binary"),
        ));
        assert_eq!(digest, required("FLEET_REAL_AUTH_TEST_BINARY_SHA256"));
        let database = required("FLEET_REAL_AUTH_TEST_DATABASE_URL");
        assert_eq!(
            database,
            "postgres://fleet_test@postgres:5432/fleet_real_auth_test"
        );
        let mut server = Self {
            process: Command::new(binary)
                .env_clear()
                .env("AUTH_DATABASE_URL", database)
                .env("AUTH_BIND", format!("127.0.0.1:{}", url.port().unwrap()))
                .env("AUTH_ISSUER", url.as_str())
                .env("AUTH_PUBLIC_BASE_URL", url.as_str())
                .env("AUTH_REGISTRATION_OPEN", registration_open.to_string())
                .env("AUTH_TOKEN_DELEGATION_POLICIES_JSON", policy.to_string())
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .expect("cannot start disposable Auth binary"),
        };
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            assert!(
                server.process.try_wait().unwrap().is_none(),
                "disposable Auth exited before readiness"
            );
            if client
                .get(url.join("health").unwrap())
                .send()
                .await
                .is_ok_and(|response| response.status() == StatusCode::OK)
            {
                return server;
            }
            assert!(
                Instant::now() < deadline,
                "disposable Auth readiness expired"
            );
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

struct RealAuthFixture {
    client: Client,
    url: Url,
    auth: OwnedAuth,
    parent: String,
    parent_id: Uuid,
    subject: String,
    email: String,
    password: String,
}

async fn real_auth_fixture() -> RealAuthFixture {
    assert_eq!(
        required("FLEET_REAL_AUTH_TEST_OWNED"),
        "source-qualified-disposable"
    );
    assert_eq!(
        required("FLEET_REAL_AUTH_TEST_SOURCE_SHA"),
        "01388dfb43332cbe5837fd5e1fadccf09cb8886d"
    );
    let client = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let listener = (41000..42000)
        .find_map(|port| TcpListener::bind(("127.0.0.1", port)).ok())
        .expect("no disposable Auth port is available");
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let url = Url::parse(&format!("http://127.0.0.1:{port}/")).unwrap();
    let bootstrap = OwnedAuth::start(&client, &url, serde_json::json!([]), true).await;
    let email = format!("fleet-contract-{}@example.test", Uuid::new_v4());
    let password = Uuid::new_v4().to_string();
    let registered = client
        .post(url.join("auth/register").unwrap())
        .json(&serde_json::json!({"email":email,"username":"fleet-contract","display_name":"Fleet Contract","password":password}))
        .send()
        .await
        .unwrap();
    assert_eq!(registered.status(), StatusCode::CREATED);
    let user: Value = registered.json().await.unwrap();
    let subject = user["id"].as_str().unwrap().to_string();
    assert!(!Uuid::parse_str(&subject).unwrap().is_nil());
    let login = client
        .post(url.join("auth/login").unwrap())
        .json(&serde_json::json!({"email":email,"password":password}))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
    let login: Value = login.json().await.unwrap();
    let issued_parent = client
        .post(url.join("auth/tokens").unwrap())
        .bearer_auth(login["access_token"].as_str().unwrap())
        .json(&serde_json::json!({"label":"disposable Fleet contract","scopes":["task-tracker:read","task-tracker:write"],"expires_in_days":1}))
        .send()
        .await
        .unwrap();
    assert_eq!(issued_parent.status(), StatusCode::CREATED);
    let issued_parent: Value = issued_parent.json().await.unwrap();
    let parent = issued_parent["secret"].as_str().unwrap().to_string();
    let parent_id = Uuid::parse_str(issued_parent["id"].as_str().unwrap()).unwrap();
    drop(bootstrap);
    // PATs survive a real server restart; browser JWTs and signing keys are not reused.
    let auth = OwnedAuth::start(
        &client,
        &url,
        serde_json::json!([{"subject":subject,"service":"task-tracker","allowed_extra_scope_prefixes":["task-tracker:sdlc:assign","task-tracker:sdlc:pm:","task-tracker:sdlc:evidence:"]}]),
        false,
    )
    .await;
    RealAuthFixture {
        client,
        url,
        auth,
        parent,
        parent_id,
        subject,
        email,
        password,
    }
}

#[tokio::test]
#[ignore = "requires source-qualified disposable real Base Auth, not an HTTP stub"]
async fn real_base_delegation_wire_replay_conflict_and_revoke_match_fleet() {
    let RealAuthFixture {
        client,
        url,
        auth: _auth,
        parent,
        subject,
        ..
    } = real_auth_fixture().await;
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
    assert_eq!(principal["display_name"], "Fleet Contract");
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

async fn token_inventory(client: &Client, url: &Url, email: &str, password: &str) -> Vec<Value> {
    // Each server restart generates a fresh signing key, so never reuse a browser JWT.
    let login = client
        .post(url.join("auth/login").unwrap())
        .json(&serde_json::json!({"email":email,"password":password}))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
    let login: Value = login.json().await.unwrap();
    let listed = client
        .get(url.join("auth/tokens").unwrap())
        .bearer_auth(login["access_token"].as_str().unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);
    let tokens: Vec<Value> = listed.json().await.unwrap();
    for token in &tokens {
        for forbidden in ["secret", "token_hash", "operation_hash", "idempotency_key"] {
            assert!(token.get(forbidden).is_none());
        }
    }
    tokens
}

#[tokio::test]
#[ignore = "requires source-qualified disposable real Base Auth, not an HTTP stub"]
async fn real_base_expired_children_replay_without_minting_and_are_rejected_by_fleet() {
    let RealAuthFixture {
        client,
        url,
        auth,
        parent,
        parent_id,
        subject,
        email,
        password,
    } = real_auth_fixture().await;
    let task = Uuid::new_v4().to_string();
    let identity = PmExecutionIdentity {
        task: "SDLC-1".into(),
        execution_ref: Uuid::new_v4().to_string(),
        tracker_instance_ref: "real-auth-expiry-test".into(),
        tracker_project_ref: Uuid::new_v4().to_string(),
        task_ref: task.clone(),
        root_ref: task,
        agent_ref: Uuid::new_v4().to_string(),
        assignment_operation_key: "real-auth-expiry-assignment".into(),
        assignment_ref: Uuid::new_v4().to_string(),
        assignment_revision: 1,
    };
    let commands = ["retained", "unretained"].map(|kind| {
        PmCredentialCommand::tracker(
            &identity,
            format!("fleet-expiry-{kind}:{}", Uuid::new_v4()),
            10,
        )
        .unwrap()
    });
    let issuer =
        PmCredentialIssuer::new(url.as_str(), "https://tracker.example.test/", &parent).unwrap();
    let issued = issuer.issue(&commands[0]).await.unwrap();
    let tracker_url = format!(
        "https://tracker.example.test/api/v1/issues/{}/sdlc/context",
        identity.task_ref
    );
    let child = issued
        .authorize(client.get(&tracker_url))
        .unwrap()
        .headers()[header::AUTHORIZATION]
        .clone();
    assert!(child.is_sensitive());
    let introspection = client
        .get(url.join("auth/tokens/introspect").unwrap())
        .header(header::AUTHORIZATION, child.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(introspection.status(), StatusCode::OK);

    // Discard the second receipt body after Auth commits. This models unretained ACK,
    // not a claimed network failure or a persisted Fleet operation journal.
    let unretained = client
        .post(url.join("auth/tokens/delegate").unwrap())
        .bearer_auth(&parent)
        .json(&commands[1])
        .send()
        .await
        .unwrap();
    assert_eq!(unretained.status(), StatusCode::CREATED);
    assert_eq!(unretained.headers()[header::CACHE_CONTROL], "no-store");
    drop(unretained);
    let before = token_inventory(&client, &url, &email, &password).await;
    assert_eq!(before.len(), 3);
    let children: Vec<_> = before
        .iter()
        .filter(|row| row["parent_token_id"] == serde_json::json!(parent_id))
        .collect();
    assert_eq!(children.len(), 2);
    for child in &children {
        assert_eq!(child["active"], true);
    }
    let retained = children
        .iter()
        .find(|row| row["id"] == serde_json::json!(issued.token_id()))
        .unwrap();
    assert_eq!(
        retained["expires_at"],
        serde_json::json!(issued.expires_at())
    );
    let unretained = children
        .iter()
        .find(|row| row["id"] != serde_json::json!(issued.token_id()))
        .unwrap();
    assert_ne!(retained["id"], unretained["id"]);
    let deadline = Instant::now() + Duration::from_secs(20);
    let expires_at = children
        .iter()
        .map(|row| {
            row["expires_at"]
                .as_str()
                .unwrap()
                .parse::<chrono::DateTime<chrono::Utc>>()
                .unwrap()
        })
        .max()
        .unwrap();
    while chrono::Utc::now() <= expires_at + chrono::Duration::milliseconds(100) {
        assert!(
            Instant::now() < deadline,
            "real child expiry wait exceeded its bound"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    // Re-open the same real Auth database; expired child rows must survive recovery.
    drop(auth);
    let _auth = OwnedAuth::start(&client, &url,
        serde_json::json!([{"subject":subject,"service":"task-tracker","allowed_extra_scope_prefixes":["task-tracker:sdlc:assign","task-tracker:sdlc:pm:","task-tracker:sdlc:evidence:"]}]), false
    ).await;
    let expired = token_inventory(&client, &url, &email, &password).await;
    assert_eq!(expired.len(), before.len());
    for row in &expired {
        let original = before
            .iter()
            .find(|original| original["id"] == row["id"])
            .unwrap();
        let mut original = original.clone();
        original["active"] = serde_json::json!(row["parent_token_id"].is_null());
        assert_eq!(*row, original);
    }
    assert!(matches!(
        issued.authorize(client.get(&tracker_url)),
        Err(AppError::Unauthorized)
    ));
    for (command, original) in commands.iter().zip([*retained, *unretained]) {
        let mut previous: Option<Value> = None;
        for _ in 0..2 {
            let replay = client
                .post(url.join("auth/tokens/delegate").unwrap())
                .bearer_auth(&parent)
                .json(command)
                .send()
                .await
                .unwrap();
            assert_eq!(replay.status(), StatusCode::OK);
            assert_eq!(replay.headers()[header::CACHE_CONTROL], "no-store");
            let replay: Value = replay.json().await.unwrap();
            assert_eq!(replay.as_object().unwrap().len(), 4);
            assert_eq!(replay["token_id"], original["id"]);
            assert_eq!(replay["expires_at"], original["expires_at"]);
            assert_eq!(replay["scopes"], serde_json::json!(command.scopes()));
            if original["id"] == serde_json::json!(issued.token_id()) {
                assert!(
                    child.to_str().unwrap()
                        == format!("Bearer {}", replay["secret"].as_str().unwrap())
                );
            }
            if let Some(previous) = &previous {
                assert!(
                    previous == &replay,
                    "expired replay changed its original receipt"
                );
            }
            let introspection = client
                .get(url.join("auth/tokens/introspect").unwrap())
                .bearer_auth(replay["secret"].as_str().unwrap())
                .send()
                .await
                .unwrap();
            assert_eq!(introspection.status(), StatusCode::UNAUTHORIZED);
            let error = issuer.issue(command).await.unwrap_err();
            assert!(matches!(error, AppError::Unavailable(ref message)
                if message == "credential acknowledgement scope or expiry mismatch"));
            assert_eq!(
                token_inventory(&client, &url, &email, &password).await,
                expired
            );
            previous = Some(replay);
        }
    }
    let introspection = client
        .get(url.join("auth/tokens/introspect").unwrap())
        .header(header::AUTHORIZATION, child)
        .send()
        .await
        .unwrap();
    assert_eq!(introspection.status(), StatusCode::UNAUTHORIZED);
    // This harness proves producer persistence and Fleet issuer/request rejection,
    // not Fleet journal recovery, runtime admission, or live Tracker acceptance.
}
