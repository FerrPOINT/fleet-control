//! PM admission for the existing human control receipt, never a second ledger.
use super::*;
use domain::{PmHumanControlScope, RuntimeControlOperation};
use sea_orm::{DatabaseTransaction, QueryResult};

pub(super) async fn authorize(
    tx: &DatabaseTransaction,
    session: Uuid,
    actor: Uuid,
    scope: &PmHumanControlScope,
) -> Result<(), AppError> {
    let row = tx
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT s.id FROM agent_sessions s JOIN users u ON u.id=s.user_id
         JOIN task_chat_bindings t ON t.session_id=s.id
         WHERE s.id=$1 AND s.user_id=$2 AND u.is_active AND u.central_sub=$3 AND t.owner_subject=$3
         FOR SHARE OF u",
            [
                session.into(),
                actor.into(),
                scope.owner_subject.clone().into(),
            ],
        ))
        .await
        .map_err(AppError::database)?;
    if row.is_none()
        || scope.record.reservation.session_id != session
        || scope.owner_user_id != actor
    {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

pub(super) async fn current(
    tx: &DatabaseTransaction,
    expected: &SessionAgentRun,
    op: RuntimeControlOperation,
    scope: &PmHumanControlScope,
) -> Result<QueryResult, AppError> {
    let row = tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
        "SELECT b.reservation,b.hermes_run_ref,b.hermes_session_ref,b.terminal_status,
            j.intent || jsonb_build_object('submitted',j.submitted,'hermes_run_ref',j.hermes_run_ref) AS intent,
            encode(sha256(convert_to(j.intent->>'request_body','UTF8')),'hex') AS request_hash,
            j.intent->>'origin' AS origin,j.intent->>'credential_fingerprint' AS credential_fingerprint
         FROM pm_run_bindings b JOIN pm_dispatch_journal j USING(session_run_id)
         JOIN agents a ON a.id=b.agent_id JOIN agent_sessions s ON s.id=b.session_id
         JOIN task_chat_bindings t ON t.session_id=s.id JOIN users u ON u.id=s.user_id
         WHERE b.session_run_id=$1 AND b.session_id=$2 AND b.agent_id=$3
           AND s.agent_id=b.agent_id AND s.state='active' AND a.kind='hermes'
           AND a.status='running' AND a.archived_at IS NULL AND a.sdlc_role='project_manager'
           AND s.user_id=$4 AND u.is_active AND u.central_sub=$5 AND t.owner_subject=$5
           AND b.terminal_status IS NULL AND j.submitted AND j.hermes_run_ref=b.hermes_run_ref
           AND ($6='stop' OR b.reservation->>'checkpoint_ref' IS NOT NULL OR j.guidance_delivered)
           AND NOT EXISTS(SELECT 1 FROM pm_tool_commands WHERE session_run_id=b.session_run_id AND kind='stop' AND attempted)
           AND NOT EXISTS(SELECT 1 FROM runtime_control_commands WHERE session_run_id=b.session_run_id AND operation='stop' AND state='acknowledged')
           AND NOT EXISTS(SELECT 1 FROM agent_config_heads WHERE agent_id=b.agent_id AND draining)
           AND b.reservation->'identity'->>'task_ref'=t.task_id::text
           AND b.reservation->'identity'->>'root_ref'=t.root_task_id::text
           AND b.reservation->'identity'->>'tracker_project_ref'=t.project_id::text
           AND b.reservation->'identity'->>'tracker_instance_ref'=t.tracker_instance_id
         FOR SHARE OF b,j,t",
        [expected.id.into(),expected.session_id.into(),expected.agent_id.into(),scope.owner_user_id.into(),scope.owner_subject.clone().into(),op.as_str().into()])).await.map_err(AppError::database)?
        .ok_or_else(||AppError::conflict("PM control custody changed"))?;
    let reservation: Value = row.try_get("", "reservation").map_err(AppError::database)?;
    let intent: domain::PmDispatchIntent =
        serde_json::from_value(row.try_get("", "intent").map_err(AppError::database)?)
            .map_err(|_| AppError::conflict("PM control dispatch custody is invalid"))?;
    let run_ref: Option<String> = row
        .try_get("", "hermes_run_ref")
        .map_err(AppError::database)?;
    let session_ref: Option<String> = row
        .try_get("", "hermes_session_ref")
        .map_err(AppError::database)?;
    if reservation != serde_json::to_value(&scope.record.reservation).map_err(AppError::internal)?
        || intent != scope.intent
        || run_ref != scope.record.hermes_run_ref
        || session_ref != scope.record.hermes_session_ref
        || run_ref != expected.runtime_run_id
        || expected.runtime_session_id.as_deref()
            != Some(scope.record.reservation.runtime_session_id().as_str())
        || run_ref.is_none()
        || session_ref.is_none()
        || scope.record.terminal_status.is_some()
        || (op == RuntimeControlOperation::Steer && expected.state != SessionRunState::Running)
    {
        return Err(AppError::conflict("PM control original identity changed"));
    }
    Ok(row)
}
