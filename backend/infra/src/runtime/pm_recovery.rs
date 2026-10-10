//! Bounded original-key replay, only for newly attested container PM intents.
use super::*;
use domain::{PmDispatchIntent, SdlcRole};
use sha2::{Digest, Sha256};

const PROOF: &str = "fleet_pm_replay";
const HORIZON_SECONDS: i64 = 86_340;
const HTTP_SECONDS: u64 = 30;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Proof {
    version: u8,
    captured_at: chrono::DateTime<chrono::Utc>,
    deadline: chrono::DateTime<chrono::Utc>,
    key: Uuid,
    request_sha256: String,
    capabilities: Value,
    generation: Uuid,
    launch_sha256: String,
    snapshot: Value,
}

fn held() -> AppError {
    AppError::Unavailable("PM original-key replay proof is unavailable or expired".into())
}

pub(super) fn context(value: &Value) -> Result<Value, AppError> {
    let mut value = value.clone();
    value.as_object_mut().ok_or_else(held)?.remove(PROOF);
    Ok(value)
}

fn digest(body: &str) -> String {
    hex::encode(Sha256::digest(body.as_bytes()))
}

fn proof(intent: &PmDispatchIntent) -> Result<Proof, AppError> {
    serde_json::from_value(
        intent
            .runtime_context
            .get(PROOF)
            .cloned()
            .ok_or_else(held)?,
    )
    .map_err(|_| held())
}

fn validate(
    intent: &PmDispatchIntent,
    launch: &app::container_runtime::ContainerLaunch,
    capabilities: &Value,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<(), AppError> {
    let proof = proof(intent)?;
    let snapshot = launch.snapshot.as_ref().ok_or_else(held)?;
    if proof.version != 1
        || proof.key != intent.session_run_id
        || proof.request_sha256 != digest(&intent.request_body)
        || proof.captured_at > now
        || proof.deadline != proof.captured_at + chrono::Duration::seconds(HORIZON_SECONDS)
        || proof.deadline - now <= chrono::Duration::seconds(HTTP_SECONDS as i64)
        || proof.capabilities != hermes_wire::dispatch_capabilities(capabilities)?
        || proof.generation != launch.prepared.container.registration.generation
        || intent.runtime_context["fleet_container_generation"] != json!(proof.generation)
        || proof.launch_sha256 != container_control::launch_hash(launch)?
        || proof.snapshot != *snapshot
        || snapshot["init_pid"].as_u64().is_none_or(|pid| pid == 0)
        || snapshot["started_at"].as_str().is_none_or(|value| {
            value.starts_with("0001-") || chrono::DateTime::parse_from_rfc3339(value).is_err()
        })
        || launch.state != "running"
        || launch.origin.as_deref() != Some(intent.origin.as_str())
    {
        return Err(held());
    }
    Ok(())
}

pub(super) async fn prepare(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    mut intent: PmDispatchIntent,
) -> Result<PmDispatchIntent, AppError> {
    // A continuation must not inherit the predecessor's key/body/deadline proof.
    intent.runtime_context = context(&intent.runtime_context)?;
    if let Some(original) = supervisor
        .repo
        .get_pm_dispatch(intent.session_run_id)
        .await?
    {
        // Repository equality still checks every original body/identity field. Only
        // the original proof is reused; retry never refreshes its deadline.
        if let Some(saved) = original.runtime_context.get(PROOF) {
            intent.runtime_context[PROOF] = saved.clone();
        }
    } else if intent
        .runtime_context
        .get("fleet_container_generation")
        .is_some()
    {
        pm_dispatch::verify_context(supervisor, agent, &intent).await?;
        let capabilities = supervisor.probe_hermes(agent).await?;
        let launch = supervisor
            .repo
            .get_container_launch(agent.id)
            .await?
            .ok_or_else(held)?;
        // This timestamp precedes journal insertion and therefore the first POST;
        // its horizon is more conservative than the journal's created_at + 24h.
        let captured_at = chrono::Utc::now();
        let captured = Proof {
            version: 1,
            captured_at,
            deadline: captured_at + chrono::Duration::seconds(HORIZON_SECONDS),
            key: intent.session_run_id,
            request_sha256: digest(&intent.request_body),
            capabilities: hermes_wire::dispatch_capabilities(&capabilities)?,
            generation: launch.prepared.container.registration.generation,
            launch_sha256: container_control::launch_hash(&launch)?,
            snapshot: launch.snapshot.clone().ok_or_else(held)?,
        };
        intent.runtime_context[PROOF] = serde_json::to_value(captured).map_err(|_| held())?;
        validate(&intent, &launch, &capabilities, chrono::Utc::now())?;
        pm_dispatch::verify_context(supervisor, agent, &intent).await?;
    }
    supervisor.repo.prepare_pm_dispatch(intent).await
}

async fn verify(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    intent: &PmDispatchIntent,
) -> Result<(), AppError> {
    // Cheap proof denial precedes native IO for legacy/non-container intents.
    proof(intent)?;
    pm_dispatch::verify_context(supervisor, agent, intent).await?;
    let capabilities = supervisor.probe_hermes(agent).await?;
    let launch = supervisor
        .repo
        .get_container_launch(agent.id)
        .await?
        .ok_or_else(held)?;
    validate(intent, &launch, &capabilities, chrono::Utc::now())
}

async fn post(
    client: &reqwest::Client,
    base: &str,
    token: &str,
    intent: &PmDispatchIntent,
) -> Result<String, AppError> {
    tokio::time::timeout(
        Duration::from_secs(HTTP_SECONDS),
        hermes_wire::submit(
            client,
            base,
            token,
            intent.session_run_id,
            &intent.request_body,
            None,
        ),
    )
    .await
    .map_err(|_| held())?
}

pub(super) async fn submit(
    supervisor: &LocalRuntimeSupervisor,
    agent: &Agent,
    intent: &PmDispatchIntent,
    authorize: impl std::future::Future<Output = Result<(), AppError>>,
) -> Result<String, AppError> {
    if let Some(run) = &intent.hermes_run_ref {
        return Ok(run.clone());
    }
    let first = !intent.submitted
        && supervisor
            .repo
            .claim_pm_submission(intent.session_run_id)
            .await?;
    let current = supervisor
        .repo
        .get_pm_dispatch(intent.session_run_id)
        .await?
        .ok_or_else(held)?;
    if let Some(run) = &current.hermes_run_ref {
        return Ok(run.clone());
    }
    if !current.submitted {
        return Err(held());
    }
    // Concurrent claim losers may replay, but only through native atomic key
    // reservation. Old/no-proof intents retain their one original attempt.
    if !first || current.runtime_context.get(PROOF).is_some() {
        verify(supervisor, agent, &current).await?;
    }
    // Replay proof is not task authority. Re-read the caller's current owner,
    // assignment and effective configuration after the potentially slow probes.
    authorize.await?;
    let current_agent = supervisor.repo.get_agent(agent.id).await?;
    if current_agent.kind != AgentKind::Hermes
        || current_agent.status != AgentStatus::Running
        || current_agent.sdlc_role != Some(SdlcRole::ProjectManager)
    {
        return Err(held());
    }
    let base = pm_dispatch::verify_context(supervisor, agent, &current).await?;
    if current.runtime_context.get(PROOF).is_some() {
        let saved = proof(&current)?;
        if saved.deadline - chrono::Utc::now() <= chrono::Duration::seconds(HTTP_SECONDS as i64) {
            return Err(held());
        }
    }
    let token = crate::agent_runtime_token(&supervisor.config, agent.id)?;
    let run = post(&supervisor.client, &base, &token, &current).await?;
    pm_dispatch::verify_context(supervisor, agent, &current).await?;
    // An ACK remains safe to save after the replay horizon: it cannot dispatch.
    supervisor
        .repo
        .record_pm_submission(current.session_run_id, run.clone())
        .await?;
    Ok(run)
}

#[cfg(test)]
#[path = "pm_recovery_tests.rs"]
mod tests;
