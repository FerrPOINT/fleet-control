use super::*;
use app::container_runtime::{ContainerPreparation, ContainerPreparationClaim, PreparedContainer};
use sea_orm::{DatabaseTransaction, QueryResult};

fn held() -> AppError {
    AppError::Unavailable("Original container preparation remains held".into())
}

fn value(claim: &ContainerPreparationClaim) -> Result<Value, AppError> {
    serde_json::to_value(claim).map_err(|_| held())
}

fn decode(row: QueryResult) -> Result<ContainerPreparation, AppError> {
    Ok(ContainerPreparation {
        claim: serde_json::from_value(row.try_get("", "claim").map_err(|_| held())?)
            .map_err(|_| held())?,
        attempted: row.try_get("", "attempted").map_err(|_| held())?,
        receipt: row
            .try_get::<Option<Value>>("", "receipt")
            .map_err(|_| held())?
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| held())?,
    })
}

pub(super) async fn get(
    repo: &PostgresFleetRepository,
    agent: Uuid,
) -> Result<Option<ContainerPreparation>, AppError> {
    repo.db
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT claim,attempted,receipt FROM runtime_container_preparations WHERE agent_id=$1",
            [agent.into()],
        ))
        .await
        .map_err(|_| held())?
        .map(decode)
        .transpose()
}

async fn checked(tx: &DatabaseTransaction, c: &ContainerPreparationClaim) -> Result<(), AppError> {
    let a = crate::container_runtime::lock(tx, c.agent_id).await?;
    crate::container_runtime::idle(tx, c.agent_id).await?;
    if a.try_get::<String>("", "kind").map_err(|_| held())? != "hermes"
        || !matches!(
            a.try_get::<String>("", "status")
                .map_err(|_| held())?
                .as_str(),
            "ready" | "stopped" | "failed"
        )
        || a.try_get::<Option<shared::Timestamp>>("", "archived_at")
            .map_err(|_| held())?
            .is_some()
        || a.try_get::<Option<i32>>("", "api_port")
            .map_err(|_| held())?
            != c.api_port
        || c.agent_id.is_nil()
        || c.generation.is_nil()
        || c.operation_id.is_nil()
        || c.intent_sha256.len() != 64
        || !c
            .intent_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(held());
    }
    for (column, path) in [
        ("runtime_path", &c.paths.runtime),
        ("config_path", &c.paths.config),
        ("workspace_path", &c.paths.workspace),
        ("logs_path", &c.paths.logs),
    ] {
        if a.try_get::<String>("", column).map_err(|_| held())? != *path {
            return Err(held());
        }
    }
    // Lock configuration authority in the same transaction as the one-shot permit.
    let head = tx
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT effective_revision FROM agent_config_heads WHERE agent_id=$1 FOR UPDATE",
            [c.agent_id.into()],
        ))
        .await
        .map_err(|_| held())?;
    let revision = head
        .map(|r| r.try_get::<Option<i64>>("", "effective_revision"))
        .transpose()
        .map_err(|_| held())?
        .flatten();
    let hash = if let Some(n) = revision {
        let r = tx
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT snapshot FROM agent_config_revisions WHERE agent_id=$1 AND revision=$2",
                [c.agent_id.into(), n.into()],
            ))
            .await
            .map_err(|_| held())?
            .ok_or_else(held)?;
        Some(crate::runtime::container_control::canonical_hash(
            &r.try_get::<Value>("", "snapshot").map_err(|_| held())?,
        )?)
    } else {
        None
    };
    if c.configuration_revision != revision || c.configuration_sha256 != hash {
        return Err(held());
    }
    let launch = tx
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT 1 FROM runtime_container_launches WHERE agent_id=$1",
            [c.agent_id.into()],
        ))
        .await
        .map_err(|_| held())?;
    if launch.is_some() {
        return Err(held());
    }
    Ok(())
}

pub(super) async fn claim(
    repo: &PostgresFleetRepository,
    c: &ContainerPreparationClaim,
) -> Result<ContainerPreparation, AppError> {
    let tx = repo.db.begin().await.map_err(|_| held())?;
    checked(&tx, c).await?;
    tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO runtime_container_preparations(agent_id,generation,operation_id,claim) VALUES($1,$2,$3,$4)
         ON CONFLICT(agent_id) DO NOTHING", [c.agent_id.into(),c.generation.into(),c.operation_id.into(),value(c)?.into()]))
        .await.map_err(|_| held())?;
    let row = tx
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT claim,attempted,receipt FROM runtime_container_preparations WHERE agent_id=$1",
            [c.agent_id.into()],
        ))
        .await
        .map_err(|_| held())?
        .ok_or_else(held)?;
    let record = decode(row)?;
    if value(&record.claim)? != value(c)? {
        return Err(held());
    }
    tx.commit().await.map_err(|_| held())?;
    Ok(record)
}

pub(super) async fn delivery(
    repo: &PostgresFleetRepository,
    c: &ContainerPreparationClaim,
) -> Result<bool, AppError> {
    let tx = repo.db.begin().await.map_err(|_| held())?;
    checked(&tx, c).await?;
    let n = tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_container_preparations SET attempted=true WHERE agent_id=$1 AND claim=$2 AND NOT attempted AND receipt IS NULL",
        [c.agent_id.into(),value(c)?.into()])).await.map_err(|_| held())?.rows_affected();
    tx.commit().await.map_err(|_| held())?;
    Ok(n == 1)
}

pub(super) async fn acknowledge(
    repo: &PostgresFleetRepository,
    c: &ContainerPreparationClaim,
    receipt: &PreparedContainer,
) -> Result<(), AppError> {
    let tx = repo.db.begin().await.map_err(|_| held())?;
    checked(&tx, c).await?;
    let r = &receipt.container.registration;
    crate::runtime::container_control::validate_registration(r)?;
    if receipt.agent_id != c.agent_id
        || r.resource_id != c.agent_id
        || r.generation != c.generation
        || r.operation_id != c.operation_id
        || receipt.api_port != c.api_port
        || serde_json::to_value(&receipt.paths).map_err(|_| held())?
            != serde_json::to_value(&c.paths).map_err(|_| held())?
        || receipt.configuration_revision != c.configuration_revision
        || receipt.configuration_sha256 != c.configuration_sha256
    {
        return Err(held());
    }
    let v = serde_json::to_value(receipt).map_err(|_| held())?;
    let n = tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE runtime_container_preparations SET receipt=$3 WHERE agent_id=$1 AND claim=$2 AND attempted
         AND (receipt IS NULL OR receipt=$3)", [c.agent_id.into(),value(c)?.into(),v.into()]))
        .await.map_err(|_| held())?.rows_affected();
    if n != 1 {
        return Err(held());
    }
    tx.commit().await.map_err(|_| held())
}
