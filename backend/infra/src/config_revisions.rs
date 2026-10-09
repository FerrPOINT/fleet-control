use super::{PostgresFleetRepository, api_ts, now, redact_text};
use app::FleetRepository;
use domain::{AgentConfigRevision, AgentConfigurationSnapshot, UpdateAgentConfigRequest};
use sea_orm::{ConnectionTrait, DatabaseBackend, QueryResult, Statement, TransactionTrait};
use shared::AppError;
use uuid::Uuid;

const SELECT_REVISIONS: &str = "SELECT r.*, h.desired_revision = r.revision AS is_desired,
    COALESCE(h.effective_revision = r.revision, false) AS is_effective, h.draining
    FROM agent_config_revisions r JOIN agent_config_heads h USING(agent_id)";

fn from_row(row: QueryResult) -> Result<AgentConfigRevision, AppError> {
    let errors: serde_json::Value = row
        .try_get("", "validation_errors")
        .map_err(AppError::database)?;
    let snapshot: serde_json::Value = row.try_get("", "snapshot").map_err(AppError::database)?;
    Ok(AgentConfigRevision {
        agent_id: row.try_get("", "agent_id").map_err(AppError::database)?,
        revision: row.try_get("", "revision").map_err(AppError::database)?,
        state: row.try_get("", "state").map_err(AppError::database)?,
        snapshot: serde_json::from_value(snapshot).map_err(AppError::internal)?,
        validation_errors: serde_json::from_value(errors).map_err(AppError::internal)?,
        last_error: row.try_get("", "last_error").map_err(AppError::database)?,
        is_desired: row.try_get("", "is_desired").map_err(AppError::database)?,
        is_effective: row
            .try_get("", "is_effective")
            .map_err(AppError::database)?,
        draining: row.try_get("", "draining").map_err(AppError::database)?,
        created_at: api_ts(row.try_get("", "created_at").map_err(AppError::database)?),
    })
}

pub(super) async fn effective(
    repo: &PostgresFleetRepository,
    id: Uuid,
) -> Result<Option<AgentConfigRevision>, AppError> {
    repo.db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("{SELECT_REVISIONS} WHERE r.agent_id=$1 AND r.revision=h.effective_revision"),
            [id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .map(from_row)
        .transpose()
}

pub(super) async fn list(
    repo: &PostgresFleetRepository,
    id: Uuid,
) -> Result<Vec<AgentConfigRevision>, AppError> {
    repo.get_agent(id).await?;
    repo.db
        .query_all(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("{SELECT_REVISIONS} WHERE r.agent_id = $1 ORDER BY r.revision DESC LIMIT 100"),
            [id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .into_iter()
        .map(from_row)
        .collect()
}

pub(super) async fn get(
    repo: &PostgresFleetRepository,
    id: Uuid,
    revision: i64,
) -> Result<AgentConfigRevision, AppError> {
    repo.db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("{SELECT_REVISIONS} WHERE r.agent_id = $1 AND r.revision = $2"),
            [id.into(), revision.into()],
        ))
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::not_found("config_revision", revision))
        .and_then(from_row)
}

pub(super) async fn create(
    repo: &PostgresFleetRepository,
    id: Uuid,
    config: UpdateAgentConfigRequest,
    actor: Uuid,
) -> Result<AgentConfigRevision, AppError> {
    let errors = config.input_errors();
    if !errors.is_empty() {
        return Err(AppError::validation(errors.join("; ")));
    }
    let skills = repo.list_agent_skills(id).await?;
    let snapshot = serde_json::to_value(AgentConfigurationSnapshot {
        config: config.clone(),
        skills,
    })
    .map_err(AppError::internal)?;
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    let row = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM agents WHERE id = $1 AND archived_at IS NULL FOR UPDATE",
            [id.into()],
        ))
        .await
        .map_err(AppError::database)?;
    if row.is_none() {
        return Err(AppError::not_found("agent", id));
    }
    let draining = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT draining FROM agent_config_heads WHERE agent_id = $1",
            [id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .map(|row| {
            row.try_get::<bool>("", "draining")
                .map_err(AppError::database)
        })
        .transpose()?
        .unwrap_or(false);
    if draining {
        return Err(AppError::conflict(
            "configuration activation is already in progress",
        ));
    }
    let row = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO agent_config_revisions(agent_id, revision, state, snapshot, created_by_user_id)
         SELECT $1, COALESCE(MAX(revision), 0) + 1, 'draft', $2, $3 FROM agent_config_revisions WHERE agent_id = $1
         RETURNING revision", [id.into(), snapshot.into(), actor.into()]))
        .await.map_err(AppError::database)?.ok_or_else(|| AppError::internal("missing config revision"))?;
    let revision: i64 = row.try_get("", "revision").map_err(AppError::database)?;
    txn.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "INSERT INTO agent_config_heads(agent_id, desired_revision) VALUES ($1, $2)
         ON CONFLICT(agent_id) DO UPDATE SET desired_revision = $2",
        [id.into(), revision.into()],
    ))
    .await
    .map_err(AppError::database)?;
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE agent_configs SET config_json = $2, soul_md = $3, env_json = $4, updated_at = $5 WHERE agent_id = $1",
        [id.into(), config.config_json.into(), config.soul_md.into(), config.env_json.into(), now().into()]))
        .await.map_err(AppError::database)?;
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO audit_log(id, actor_user_id, action, entity_type, entity_id, payload, created_at)
         VALUES ($1,$2,'agent_config.draft','agent_config',$3,$4,now())",
        [Uuid::new_v4().into(), actor.into(), id.to_string().into(), serde_json::json!({"revision": revision}).into()]))
        .await.map_err(AppError::database)?;
    txn.commit().await.map_err(AppError::database)?;
    get(repo, id, revision).await
}

pub(super) async fn validate(
    repo: &PostgresFleetRepository,
    id: Uuid,
    revision: i64,
    errors: Vec<String>,
) -> Result<AgentConfigRevision, AppError> {
    let result = repo
        .db
        .execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE agent_config_revisions SET state = $3, validation_errors = $4
         WHERE agent_id = $1 AND revision = $2 AND state IN ('draft','validated','failed')",
            [
                id.into(),
                revision.into(),
                if errors.is_empty() {
                    "validated"
                } else {
                    "draft"
                }
                .into(),
                serde_json::json!(errors).into(),
            ],
        ))
        .await
        .map_err(AppError::database)?;
    if result.rows_affected() == 0 {
        return Err(AppError::conflict(
            "revision cannot be validated in its current state",
        ));
    }
    get(repo, id, revision).await
}

pub(super) async fn activate(
    repo: &PostgresFleetRepository,
    id: Uuid,
    revision: i64,
    actor: Uuid,
) -> Result<AgentConfigRevision, AppError> {
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    txn.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM agents WHERE id = $1 FOR UPDATE",
        [id.into()],
    ))
    .await
    .map_err(AppError::database)?;
    let changed = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE agent_config_heads SET draining = true
         WHERE agent_id = $1 AND desired_revision = $2 AND NOT draining
           AND EXISTS (SELECT 1 FROM agent_config_revisions r WHERE r.agent_id = $1 AND r.revision = $2 AND r.state = 'validated')",
        [id.into(), revision.into()])).await.map_err(AppError::database)?;
    if changed.rows_affected() == 0 {
        return Err(AppError::conflict(
            "only the desired validated revision can be activated",
        ));
    }
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE agent_config_revisions SET state = 'activating', claimed_at = NULL, last_error = NULL WHERE agent_id = $1 AND revision = $2",
        [id.into(), revision.into()])).await.map_err(AppError::database)?;
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO audit_log(id, actor_user_id, action, entity_type, entity_id, payload, created_at)
         VALUES ($1,$2,'agent_config.activate','agent_config',$3,$4,now())",
        [Uuid::new_v4().into(), actor.into(), id.to_string().into(), serde_json::json!({"revision": revision}).into()]))
        .await.map_err(AppError::database)?;
    txn.commit().await.map_err(AppError::database)?;
    get(repo, id, revision).await
}

pub(super) async fn claim(
    repo: &PostgresFleetRepository,
) -> Result<Option<AgentConfigRevision>, AppError> {
    let row = repo.db.query_one(Statement::from_string(DatabaseBackend::Postgres,
        "WITH candidate AS (
            SELECT r.agent_id, r.revision FROM agent_config_revisions r
            JOIN agent_config_heads h USING(agent_id)
            WHERE r.state = 'activating' AND r.claimed_at IS NULL AND h.draining
              AND NOT EXISTS (SELECT 1 FROM session_agent_runs run WHERE run.agent_id = r.agent_id
                AND (run.runtime_session_id IS NOT NULL OR EXISTS(SELECT 1 FROM runtime_container_launches c WHERE c.agent_id=r.agent_id))
                AND run.state IN ('pending','running','waiting','stopping'))
              AND NOT EXISTS (SELECT 1 FROM message_dispatch_outbox o WHERE o.agent_id = r.agent_id AND o.state IN ('dispatching','uncertain'))
              AND NOT EXISTS (SELECT 1 FROM hermes_dispatch_journal j WHERE j.agent_id = r.agent_id AND j.state IN ('prepared','submitted'))
            ORDER BY r.created_at FOR UPDATE OF r, h SKIP LOCKED LIMIT 1)
         UPDATE agent_config_revisions r SET claimed_at = now() FROM candidate c
            WHERE r.agent_id = c.agent_id AND r.revision = c.revision RETURNING r.agent_id, r.revision".to_string()))
        .await.map_err(AppError::database)?;
    match row {
        Some(row) => Ok(Some(
            get(
                repo,
                row.try_get("", "agent_id").map_err(AppError::database)?,
                row.try_get("", "revision").map_err(AppError::database)?,
            )
            .await?,
        )),
        None => Ok(None),
    }
}

pub(super) async fn finish(
    repo: &PostgresFleetRepository,
    id: Uuid,
    revision: i64,
    error: Option<String>,
    reconciled: bool,
) -> Result<(), AppError> {
    let txn = repo.db.begin().await.map_err(AppError::database)?;
    super::container_runtime::lock(&txn, id).await?;
    let container = txn
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT generation FROM runtime_container_launches WHERE agent_id=$1 LIMIT 1",
            [id.into()],
        ))
        .await
        .map_err(AppError::database)?;
    if container.is_some() {
        return Err(AppError::Unavailable(
            "Docker activation can only finish with original physical and readiness proof".into(),
        ));
    }
    let changed = txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE agent_config_revisions SET state = $3, last_error = $4 WHERE agent_id = $1 AND revision = $2 AND state = 'activating'",
        [id.into(), revision.into(), if error.is_some() { "failed" } else { "active" }.into(), error.clone().map(|error| redact_text(&error)).into()]))
        .await.map_err(AppError::database)?;
    if changed.rows_affected() == 0 {
        return Err(AppError::conflict("activation state changed concurrently"));
    }
    txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE agent_config_heads SET draining = $4, effective_revision = CASE WHEN $3 THEN effective_revision ELSE $2 END
         WHERE agent_id = $1 AND desired_revision = $2", [id.into(), revision.into(), error.is_some().into(), (!reconciled).into()]))
        .await.map_err(AppError::database)?;
    txn.commit().await.map_err(AppError::database)?;
    Ok(())
}

pub(super) async fn draining(repo: &PostgresFleetRepository, id: Uuid) -> Result<bool, AppError> {
    repo.db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT draining FROM agent_config_heads WHERE agent_id = $1",
            [id.into()],
        ))
        .await
        .map_err(AppError::database)?
        .map(|row| row.try_get("", "draining").map_err(AppError::database))
        .transpose()
        .map(|value| value.unwrap_or(false))
}
