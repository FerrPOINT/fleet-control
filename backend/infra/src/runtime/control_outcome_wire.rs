//! Original-context GET witness. This module grants no dispatch permit or runtime authority.
use super::hermes_wire;
use domain::ApprovalChoice;
use reqwest::{Client, Response, StatusCode, header};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::AppError;
use std::time::Duration;
use uuid::Uuid;

const SOURCE: &str = "bbaf7af5c83546d19f8060f4097d3bb25cd1a3c3";
const CAPABILITIES: &str = "/fleet/v1/controls/capabilities";
const LOOKUP: &str = "/fleet/v1/controls/lookup";
const MAX_BODY: usize = 65_536;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Operation {
    Steer,
    Stop,
    Approval,
}

impl Operation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Steer => "steer",
            Self::Stop => "stop",
            Self::Approval => "approval",
        }
    }
}

/// No Debug: guidance and original action context must not enter logs or public receipts.
pub enum Request<'a> {
    Steer(&'a str),
    Stop,
    Approval {
        request_id: &'a str,
        choice: ApprovalChoice,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SteerBody {
    input: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalBody {
    request_id: String,
    choice: ApprovalChoice,
    resolve_all: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Endpoint {
    method: String,
    path: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capabilities {
    object: String,
    contract_version: u32,
    store_id: String,
    scope_fingerprint: String,
    profile: String,
    native_source_revision: String,
    single_send: bool,
    non_dispatch_lookup: bool,
    lookup: Endpoint,
    operations: Vec<String>,
}

impl Capabilities {
    fn validate(&self, token: &str) -> Result<(), AppError> {
        if token.is_empty()
            || self.object != "fleet.hermes.controls.capabilities"
            || self.contract_version != 1
            || !canonical_uuid(&self.store_id)
            || self.scope_fingerprint != scope(token)
            || self.profile != "default"
            || self.native_source_revision != SOURCE
            || !self.single_send
            || !self.non_dispatch_lookup
            || self.lookup.method != "GET"
            || self.lookup.path != LOOKUP
            || self.operations.len() != 3
            || ["steer", "stop", "approval"]
                .iter()
                .any(|op| self.operations.iter().filter(|v| v.as_str() == *op).count() != 1)
        {
            return Err(unavailable());
        }
        Ok(())
    }
}

/// Prepared before dispatch; persistence and a separate single-use permit are still required.
/// Deserialization is not validation: lookup rechecks every original fact before HTTP.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    origin: String,
    credential_fingerprint: String,
    capabilities: Capabilities,
    command_id: Uuid,
    run_id: String,
    operation: Operation,
    request_body: String,
    request_sha256: String,
}

impl Context {
    fn validate(&self, origin: &str, token: &str) -> Result<(), AppError> {
        validate_origin(origin)?;
        self.capabilities.validate(token)?;
        if self.origin != origin
            || self.credential_fingerprint != hermes_wire::credential_fingerprint(token)
            || self.command_id.is_nil()
            || !native_run(&self.run_id)
            || self.request_body.len() > MAX_BODY
            || self.request_sha256 != digest(self.request_body.as_bytes())
        {
            return Err(unavailable());
        }
        match self.operation {
            Operation::Steer => {
                let body: SteerBody =
                    serde_json::from_str(&self.request_body).map_err(|_| unavailable())?;
                if !has_input(&body.input) {
                    return Err(unavailable());
                }
            }
            Operation::Stop if !self.request_body.is_empty() => return Err(unavailable()),
            Operation::Stop => {}
            Operation::Approval => {
                let body: ApprovalBody =
                    serde_json::from_str(&self.request_body).map_err(|_| unavailable())?;
                if !approval_id(&body.request_id) || body.resolve_all {
                    return Err(unavailable());
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Acknowledgement {
    Steered,
    Stopping,
    ApprovalResolved,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Uncertain,
    Acknowledged(Acknowledgement),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SteerAck {
    object: String,
    run_id: String,
    accepted: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StopAck {
    run_id: String,
    status: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalAck {
    object: String,
    run_id: String,
    request_id: String,
    choice: ApprovalChoice,
    resolved: u64,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Ack {
    Steer(SteerAck),
    Stop(StopAck),
    Approval(ApprovalAck),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Found {
    object: String,
    contract_version: u32,
    profile: String,
    scope_fingerprint: String,
    store_id: String,
    command_id: String,
    run_id: String,
    operation: Operation,
    request_sha256: String,
    state: String,
    ack: Option<Ack>,
}

fn outcome(bytes: &[u8], context: &Context) -> Result<Outcome, AppError> {
    let found: Found = serde_json::from_slice(bytes).map_err(|_| unavailable())?;
    if found.object != "fleet.hermes.controls.lookup"
        || found.contract_version != 1
        || found.profile != "default"
        || found.scope_fingerprint != context.capabilities.scope_fingerprint
        || found.store_id != context.capabilities.store_id
        || found.command_id != context.command_id.to_string()
        || found.run_id != context.run_id
        || found.operation != context.operation
        || found.request_sha256 != context.request_sha256
    {
        return Err(unavailable());
    }
    if found.state == "uncertain" && found.ack.is_none() {
        return Ok(Outcome::Uncertain);
    }
    if found.state != "acknowledged" {
        return Err(unavailable());
    }
    let ack = match (context.operation, found.ack) {
        (Operation::Steer, Some(Ack::Steer(ack)))
            if ack.object == "hermes.run.steer" && ack.run_id == context.run_id && ack.accepted =>
        {
            Acknowledgement::Steered
        }
        (Operation::Stop, Some(Ack::Stop(ack)))
            if ack.run_id == context.run_id && ack.status == "stopping" =>
        {
            Acknowledgement::Stopping
        }
        (Operation::Approval, Some(Ack::Approval(ack))) => {
            let body: ApprovalBody =
                serde_json::from_str(&context.request_body).map_err(|_| unavailable())?;
            if ack.object != "hermes.run.approval_response"
                || ack.run_id != context.run_id
                || ack.request_id != body.request_id
                || ack.choice != body.choice
                || ack.resolved != 1
            {
                return Err(unavailable());
            }
            Acknowledgement::ApprovalResolved
        }
        _ => return Err(unavailable()),
    };
    Ok(Outcome::Acknowledged(ack))
}

fn unavailable() -> AppError {
    AppError::Unavailable("Hermes original control outcome is not verified".into())
}

fn canonical_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|id| !id.is_nil() && id.to_string() == value)
}

fn native_run(value: &str) -> bool {
    value.strip_prefix("run_").is_some_and(|id| {
        id.len() == 32
            && id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn approval_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(|c| (c as u32) < 32 || c == '\u{7f}')
}

fn has_input(value: &str) -> bool {
    // Python str.strip also includes the four ASCII record separators.
    value
        .chars()
        .any(|c| !c.is_whitespace() && !('\u{1c}'..='\u{1f}').contains(&c))
}

fn validate_origin(origin: &str) -> Result<(), AppError> {
    let url = reqwest::Url::parse(origin).map_err(|_| unavailable())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
        || url.as_str() != format!("{origin}/")
    {
        return Err(unavailable());
    }
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn scope(token: &str) -> String {
    digest(format!("default\0{token}").as_bytes())
}

async fn json_bytes(response: Response) -> Result<Vec<u8>, AppError> {
    if !response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(';')
                .next()
                .is_some_and(|mime| mime.trim().eq_ignore_ascii_case("application/json"))
        })
    {
        return Err(unavailable());
    }
    hermes_wire::read_body(response, StatusCode::OK, MAX_BODY)
        .await
        .map_err(|_| unavailable())
}

/// GET only. The caller must use the supervisor's no-proxy/no-redirect/no-retry client.
/// A returned context is not authorization: persist it with a single-use permit before POST.
pub async fn prepare(
    client: &Client,
    origin: &str,
    token: &str,
    command_id: Uuid,
    run_id: &str,
    request: Request<'_>,
) -> Result<Context, AppError> {
    validate_origin(origin)?;
    if token.is_empty() || command_id.is_nil() || !native_run(run_id) {
        return Err(unavailable());
    }
    let (operation, body) = match request {
        Request::Stop => (Operation::Stop, String::new()),
        Request::Steer(input) if has_input(input) && input.len() <= MAX_BODY => (
            Operation::Steer,
            serde_json::to_string(&SteerBody {
                input: input.to_owned(),
            })
            .map_err(|_| unavailable())?,
        ),
        Request::Approval { request_id, choice } if approval_id(request_id) => (
            Operation::Approval,
            serde_json::to_string(&ApprovalBody {
                request_id: request_id.to_owned(),
                choice,
                resolve_all: false,
            })
            .map_err(|_| unavailable())?,
        ),
        _ => return Err(unavailable()),
    };
    if body.len() > MAX_BODY {
        return Err(unavailable());
    }
    let response = client
        .get(format!("{origin}{CAPABILITIES}"))
        .bearer_auth(token)
        .header(header::ACCEPT_ENCODING, "identity")
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|_| unavailable())?;
    let capabilities: Capabilities =
        serde_json::from_slice(&json_bytes(response).await?).map_err(|_| unavailable())?;
    let context = Context {
        origin: origin.to_owned(),
        credential_fingerprint: hermes_wire::credential_fingerprint(token),
        capabilities,
        command_id,
        run_id: run_id.to_owned(),
        operation,
        request_sha256: digest(body.as_bytes()),
        request_body: body,
    };
    context.validate(origin, token)?;
    Ok(context)
}

/// Original-context GET only; missing/uncertain/invalid replies never authorize another POST.
/// No fresh epoch is adopted here, and no run status can substitute for a decision witness.
pub async fn lookup(
    client: &Client,
    context: &Context,
    origin: &str,
    token: &str,
) -> Result<Outcome, AppError> {
    context.validate(origin, token)?;
    let response = client
        .get(format!("{origin}{LOOKUP}"))
        .bearer_auth(token)
        .header(header::ACCEPT_ENCODING, "identity")
        .query(&[
            ("store_id", context.capabilities.store_id.as_str()),
            ("command_id", context.command_id.to_string().as_str()),
            ("run_id", context.run_id.as_str()),
            ("operation", context.operation.as_str()),
            ("request_sha256", context.request_sha256.as_str()),
        ])
        .timeout(Duration::from_secs(10))
        .send()
        .await
        .map_err(|_| unavailable())?;
    outcome(&json_bytes(response).await?, context)
}

#[cfg(test)]
#[path = "control_outcome_wire_tests.rs"]
mod tests;
