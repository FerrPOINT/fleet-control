use super::*;
use domain::{
    ClarificationAnswerCommand, ClarificationAnswerRequest, ClarificationCommandActor,
    ClarificationDeliveryOutcome, ClarificationDeliveryPermit, answer_matches_command,
    answer_payload_hash, canonical_answer_request,
};
use sea_orm::{DatabaseTransaction, QueryResult};

const RECORD: &str = "jsonb_build_object('id',c.id,'session_id',c.session_id,'question_id',c.question_id,
    'request',c.request_body::jsonb,'payload_sha256',c.payload_sha256,'state',c.state,
    'answer',c.answer,'continuation_state',c.continuation_state,'rejection_status',c.rejection_status,'created_at',c.created_at,'updated_at',c.updated_at)";

fn db_error(error: sea_orm::DbErr) -> AppError {
    if matches!(
        error.sql_err(),
        Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
    ) {
        AppError::conflict("an original clarification command already holds this question or key")
    } else {
        AppError::Database("clarification journal operation failed".into())
    }
}

// API proves current Tracker project/owner access first; recheck local custody
// under shared locks before ANY journal lookup, including historical replay.
async fn authorize(
    tx: &DatabaseTransaction,
    actor: &ClarificationCommandActor,
) -> Result<(), AppError> {
    if actor.subject != actor.binding.owner_subject {
        return Err(AppError::Forbidden);
    }
    let row = tx
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT s.id FROM agent_sessions s JOIN users u ON u.id=s.user_id
         JOIN task_chat_bindings b ON b.session_id=s.id
         WHERE s.id=$1 AND s.user_id=$2 AND u.is_active AND u.central_sub=$3
           AND to_jsonb(b)-'session_id'-'idempotency_key'-'created_at'=$4
         FOR SHARE OF s,u,b",
            [
                actor.session_id.into(),
                actor.user_id.into(),
                actor.subject.clone().into(),
                serde_json::to_value(&actor.binding)
                    .map_err(AppError::internal)?
                    .into(),
            ],
        ))
        .await
        .map_err(db_error)?;
    if row.is_none() {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

fn receipt(row: &QueryResult) -> Result<ClarificationAnswerCommand, AppError> {
    let result: ClarificationAnswerCommand =
        serde_json::from_value(row.try_get::<Value>("", "record").map_err(db_error)?)
            .map_err(|_| AppError::Database("invalid clarification journal record".into()))?;
    let body: String = row.try_get("", "request_body").map_err(db_error)?;
    if answer_payload_hash(&body) != result.payload_sha256
        || canonical_answer_request(result.request.clone())? != body
    {
        return Err(AppError::Database(
            "clarification command integrity mismatch".into(),
        ));
    }
    Ok(result)
}

async fn row(
    tx: &DatabaseTransaction,
    actor: &ClarificationCommandActor,
    id: Uuid,
) -> Result<QueryResult, AppError> {
    tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        format!("SELECT {RECORD} AS record,c.request_body,c.ever_uncertain FROM clarification_answer_commands c
            WHERE c.id=$1 AND c.session_id=$2 AND c.actor_user_id=$3 AND c.owner_subject=$4 AND c.binding=$5 FOR UPDATE"),
        [id.into(),actor.session_id.into(),actor.user_id.into(),actor.subject.clone().into(),
         serde_json::to_value(&actor.binding).map_err(AppError::internal)?.into()]))
        .await.map_err(db_error)?.ok_or_else(|| AppError::not_found("clarification command", id))
}

pub(super) async fn store(
    repo: &PostgresFleetRepository,
    actor: &ClarificationCommandActor,
    question: Uuid,
    request: ClarificationAnswerRequest,
) -> Result<ClarificationAnswerCommand, AppError> {
    if question.is_nil() {
        return Err(AppError::validation("invalid question"));
    }
    let body = canonical_answer_request(request.clone())?;
    let hash = answer_payload_hash(&body);
    let tx = repo.db.begin().await.map_err(db_error)?;
    authorize(&tx, actor).await?;
    tx.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
        [format!(
            "clarification:{}:{}",
            actor.user_id, request.idempotency_key
        )
        .into()],
    ))
    .await
    .map_err(db_error)?;
    let existing = tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT id,session_id FROM clarification_answer_commands WHERE actor_user_id=$1 AND idempotency_key=$2",
        [actor.user_id.into(),request.idempotency_key.clone().into()])).await.map_err(db_error)?;
    if let Some(existing) = existing {
        if existing
            .try_get::<Uuid>("", "session_id")
            .map_err(db_error)?
            != actor.session_id
        {
            return Err(AppError::conflict(
                "original clarification key belongs to another chat",
            ));
        }
        let saved =
            receipt(&row(&tx, actor, existing.try_get("", "id").map_err(db_error)?).await?)?;
        if saved.question_id != question || saved.payload_sha256 != hash {
            return Err(AppError::conflict(
                "original clarification command has a different target or payload",
            ));
        }
        tx.commit().await.map_err(db_error)?;
        return Ok(saved);
    }
    if tx
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT id FROM clarification_answer_commands WHERE session_id=$1
             AND state='delivered' AND continuation_state='pending' LIMIT 1",
            [actor.session_id.into()],
        ))
        .await
        .map_err(db_error)?
        .is_some()
    {
        return Err(AppError::conflict(
            "continue the original delivered PM answer before creating another command",
        ));
    }
    let id = Uuid::new_v4();
    tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "INSERT INTO clarification_answer_commands(id,session_id,question_id,actor_user_id,owner_subject,
            binding,idempotency_key,request_body,payload_sha256) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        [id.into(),actor.session_id.into(),question.into(),actor.user_id.into(),actor.subject.clone().into(),
         serde_json::to_value(&actor.binding).map_err(AppError::internal)?.into(),request.idempotency_key.into(),body.into(),hash.into()]))
        .await.map_err(db_error)?;
    let result = receipt(&row(&tx, actor, id).await?)?;
    tx.commit().await.map_err(db_error)?;
    Ok(result)
}

pub(super) async fn get(
    repo: &PostgresFleetRepository,
    actor: &ClarificationCommandActor,
    id: Uuid,
) -> Result<ClarificationAnswerCommand, AppError> {
    let tx = repo.db.begin().await.map_err(db_error)?;
    authorize(&tx, actor).await?;
    let result = receipt(&row(&tx, actor, id).await?)?;
    tx.commit().await.map_err(db_error)?;
    Ok(result)
}

pub(super) async fn list(
    repo: &PostgresFleetRepository,
    actor: &ClarificationCommandActor,
) -> Result<Vec<ClarificationAnswerCommand>, AppError> {
    let tx = repo.db.begin().await.map_err(db_error)?;
    authorize(&tx, actor).await?;
    let rows = tx.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        format!("SELECT {RECORD} AS record,c.request_body FROM clarification_answer_commands c
            WHERE c.session_id=$1 AND c.actor_user_id=$2 AND c.owner_subject=$3 AND c.binding=$4
                AND (c.state IN ('stored','delivering','uncertain') OR (c.state='delivered' AND c.continuation_state='pending'))
                ORDER BY c.created_at,c.id LIMIT 101"),
        [actor.session_id.into(),actor.user_id.into(),actor.subject.clone().into(),
         serde_json::to_value(&actor.binding).map_err(AppError::internal)?.into()])).await.map_err(db_error)?;
    if rows.len() > 100 {
        return Err(AppError::Unavailable(
            "clarification journal requires paged recovery".into(),
        ));
    }
    let result = rows.iter().map(receipt).collect::<Result<Vec<_>, _>>()?;
    tx.commit().await.map_err(db_error)?;
    Ok(result)
}

pub(super) async fn claim(
    repo: &PostgresFleetRepository,
    actor: &ClarificationCommandActor,
    id: Uuid,
) -> Result<ClarificationDeliveryPermit, AppError> {
    let tx = repo.db.begin().await.map_err(db_error)?;
    authorize(&tx, actor).await?;
    row(&tx, actor, id).await?;
    let attempt = Uuid::new_v4();
    let claimed = tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE clarification_answer_commands SET ever_uncertain=ever_uncertain OR state='delivering',state='delivering',attempt_id=$2,
            lease_until=clock_timestamp()+interval '30 seconds',updated_at=clock_timestamp()
         WHERE id=$1 AND (state IN ('stored','uncertain') OR (state='delivering' AND lease_until <= clock_timestamp()))",
        [id.into(),attempt.into()])).await.map_err(db_error)?.rows_affected() == 1;
    let command = receipt(&row(&tx, actor, id).await?)?;
    tx.commit().await.map_err(db_error)?;
    Ok(ClarificationDeliveryPermit {
        command,
        attempt_id: claimed.then_some(attempt),
    })
}

pub(super) async fn finish_continuation(
    repo: &PostgresFleetRepository,
    actor: &ClarificationCommandActor,
    id: Uuid,
    outcome: domain::PmContinuationOutcome,
) -> Result<ClarificationAnswerCommand, AppError> {
    let tx = repo.db.begin().await.map_err(db_error)?;
    authorize(&tx, actor).await?;
    let saved = receipt(&row(&tx, actor, id).await?)?;
    // Disabled dispatch / NotRequired cannot settle an existing PM receipt.
    if matches!(outcome, domain::PmContinuationOutcome::Confirmed)
        && saved.state == domain::ClarificationDeliveryState::Delivered
        && saved.continuation_state == domain::ClarificationContinuationState::Pending
    {
        tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE clarification_answer_commands SET continuation_state='confirmed',updated_at=clock_timestamp() WHERE id=$1",
            [id.into()])).await.map_err(db_error)?;
    }
    let result = receipt(&row(&tx, actor, id).await?)?;
    tx.commit().await.map_err(db_error)?;
    Ok(result)
}

pub(super) async fn finish(
    repo: &PostgresFleetRepository,
    actor: &ClarificationCommandActor,
    id: Uuid,
    attempt: Uuid,
    outcome: ClarificationDeliveryOutcome,
) -> Result<ClarificationAnswerCommand, AppError> {
    let tx = repo.db.begin().await.map_err(db_error)?;
    authorize(&tx, actor).await?;
    let original = row(&tx, actor, id).await?;
    let command = receipt(&original)?;
    let ever_uncertain: bool = original.try_get("", "ever_uncertain").map_err(db_error)?;
    let (state, answer, rejection): (&str, Option<Value>, Option<i32>) = match outcome {
        ClarificationDeliveryOutcome::Delivered(answer)
            if answer_matches_command(&command, &answer, &actor.subject) =>
        {
            (
                "delivered",
                Some(serde_json::to_value(answer).map_err(AppError::internal)?),
                None,
            )
        }
        ClarificationDeliveryOutcome::Rejected(status)
            if !ever_uncertain && (400..=499).contains(&status) =>
        {
            ("rejected", None, Some(i32::from(status)))
        }
        _ => ("uncertain", None, None),
    };
    tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "UPDATE clarification_answer_commands SET state=$3,answer=$4,rejection_status=$5,ever_uncertain=ever_uncertain OR $3='uncertain',
            lease_until=NULL,updated_at=clock_timestamp() WHERE id=$1 AND state='delivering' AND attempt_id=$2",
        [id.into(),attempt.into(),state.into(),answer.into(),rejection.into()])).await.map_err(db_error)?;
    // A late older completion cannot overwrite a newer attempt's state.
    let result = receipt(&row(&tx, actor, id).await?)?;
    tx.commit().await.map_err(db_error)?;
    Ok(result)
}
