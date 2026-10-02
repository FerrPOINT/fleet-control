use super::*;
use domain::{PmDraftOperation, PmDraftProof};

async fn active_owner(
    db: &impl ConnectionTrait,
    owner: Uuid,
    subject: &str,
) -> Result<(), AppError> {
    let user = user::Entity::find_by_id(owner)
        .one(db)
        .await
        .map_err(AppError::database)?
        .ok_or(AppError::Unauthorized)?;
    if !user.is_active || user.central_sub.as_deref() != Some(subject) {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

fn decode(row: sea_orm::QueryResult) -> Result<PmDraftOperation, AppError> {
    serde_json::from_value(
        row.try_get::<Value>("", "operation")
            .map_err(AppError::database)?,
    )
    .map_err(AppError::internal)
}

impl PostgresFleetRepository {
    pub(crate) async fn reserve_pm_creation(
        &self,
        operation: PmDraftOperation,
    ) -> Result<PmDraftOperation, AppError> {
        operation.request.validate()?;
        let subject = Uuid::parse_str(&operation.owner_subject).ok();
        if operation.id.is_nil()
            || operation.owner_user_id.is_nil()
            || operation.project_id.is_nil()
            || subject.is_none_or(|id| id.is_nil() || id.to_string() != operation.owner_subject)
            || !domain::pm_draft::valid_key(&operation.tracker_instance_id)
            || operation.draft.is_some()
            || operation.input.is_some()
            || operation.reservation.is_some()
            || operation.session_id.is_some()
        {
            return Err(AppError::validation("invalid PM creation operation"));
        }
        let txn = self.db.begin().await.map_err(AppError::database)?;
        active_owner(&txn, operation.owner_user_id, &operation.owner_subject).await?;
        let value = serde_json::to_value(&operation).map_err(AppError::internal)?;
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO pm_draft_creation_operations(id,owner_user_id,idempotency_key,operation)
             VALUES($1,$2,$3,$4) ON CONFLICT(owner_user_id,idempotency_key) DO NOTHING",
            [
                operation.id.into(),
                operation.owner_user_id.into(),
                operation.request.idempotency_key.clone().into(),
                value.into(),
            ],
        ))
        .await
        .map_err(AppError::database)?;
        let row = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT operation FROM pm_draft_creation_operations WHERE owner_user_id=$1 AND idempotency_key=$2 FOR UPDATE",
            [operation.owner_user_id.into(),operation.request.idempotency_key.clone().into()]))
            .await.map_err(AppError::database)?.ok_or_else(|| AppError::conflict("PM operation cannot be recovered"))?;
        let current = decode(row)?;
        if current.owner_subject != operation.owner_subject
            || current.tracker_instance_id != operation.tracker_instance_id
            || current.project_id != operation.project_id
            || current.request != operation.request
        {
            return Err(AppError::conflict(
                "PM creation key has a different payload",
            ));
        }
        txn.commit().await.map_err(AppError::database)?;
        Ok(current)
    }

    pub(crate) async fn read_pm_creation(
        &self,
        id: Uuid,
        owner: Uuid,
    ) -> Result<PmDraftOperation, AppError> {
        let row = self.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT operation FROM pm_draft_creation_operations WHERE id=$1 AND owner_user_id=$2",
            [id.into(),owner.into()])).await.map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("PM creation operation", id))?;
        let operation = decode(row)?;
        active_owner(&self.db, owner, &operation.owner_subject).await?;
        Ok(operation)
    }

    pub(crate) async fn persist_pm_creation_proof(
        &self,
        id: Uuid,
        owner: Uuid,
        proof: PmDraftProof,
    ) -> Result<PmDraftOperation, AppError> {
        let txn = self.db.begin().await.map_err(AppError::database)?;
        let row = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT operation FROM pm_draft_creation_operations WHERE id=$1 AND owner_user_id=$2 FOR UPDATE",
            [id.into(),owner.into()])).await.map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("PM creation operation", id))?;
        let mut operation = decode(row)?;
        active_owner(&txn, owner, &operation.owner_subject).await?;
        if let PmDraftProof::Chat(session_id) = &proof {
            let task = operation
                .draft
                .as_ref()
                .ok_or_else(|| AppError::conflict("Draft proof is missing"))?;
            let bound = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "SELECT b.session_id FROM task_chat_bindings b JOIN agent_sessions s ON s.id=b.session_id
                 WHERE b.session_id=$1 AND b.tracker_instance_id=$2 AND b.project_id=$3 AND b.task_id=$4
                   AND b.root_task_id=$4 AND b.agent_id=$5 AND b.owner_subject=$6 AND s.user_id=$7",
                [(*session_id).into(),operation.tracker_instance_id.clone().into(),operation.project_id.into(),
                 task.task_id.into(),operation.request.agent_id.into(),operation.owner_subject.clone().into(),owner.into()]))
                .await.map_err(AppError::database)?;
            if bound.is_none() {
                return Err(AppError::conflict(
                    "PM chat receipt does not match the operation",
                ));
            }
        }
        operation.apply(proof)?;
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE pm_draft_creation_operations SET operation=$2,updated_at=now() WHERE id=$1",
            [
                id.into(),
                serde_json::to_value(&operation)
                    .map_err(AppError::internal)?
                    .into(),
            ],
        ))
        .await
        .map_err(AppError::database)?;
        txn.commit().await.map_err(AppError::database)?;
        Ok(operation)
    }
}
