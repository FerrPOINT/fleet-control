use super::*;

impl PostgresFleetRepository {
    pub(super) async fn claim_owned_message_dispatch(
        &self,
        controller_id: Option<Uuid>,
    ) -> Result<Option<SessionMessage>, AppError> {
        let row = self.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "WITH candidate AS (
                SELECT o.message_id FROM message_dispatch_outbox o
                JOIN agents a ON a.id = o.agent_id
                JOIN session_messages m ON m.id = o.message_id
                JOIN agent_sessions s ON s.id = m.session_id
                WHERE o.state = 'pending' AND a.status = 'running' AND a.kind = 'hermes'
                  AND ($1::uuid IS NULL
                    OR NOT EXISTS (SELECT 1 FROM runtime_launches l WHERE l.agent_id=a.id AND l.state IN ('claimed','gateway_started'))
                    OR EXISTS (SELECT 1 FROM runtime_launches l WHERE l.agent_id=a.id
                      AND l.state='gateway_started' AND l.controller_id=$1))
                  AND NOT EXISTS (SELECT 1 FROM task_chat_bindings b WHERE b.session_id = s.id)
                  AND NOT EXISTS (SELECT 1 FROM agent_config_heads h WHERE h.agent_id = a.id AND h.draining)
                  AND NOT EXISTS (SELECT 1 FROM message_dispatch_outbox busy WHERE busy.agent_id = a.id AND busy.state IN ('dispatching','uncertain'))
                  AND NOT EXISTS (SELECT 1 FROM session_agent_runs r WHERE r.agent_id = a.id
                      AND r.state IN ('pending','running','waiting','stopping') AND r.runtime_session_id IS NOT NULL)
                ORDER BY o.created_at, o.message_id FOR UPDATE OF a, o, s SKIP LOCKED LIMIT 1)
             UPDATE message_dispatch_outbox o SET state = 'dispatching', updated_at = now()
                FROM candidate WHERE o.message_id = candidate.message_id RETURNING o.message_id",
            [controller_id.into()]))
            .await.map_err(AppError::database)?;
        let Some(row) = row else {
            return Ok(None);
        };
        let id: Uuid = row.try_get("", "message_id").map_err(AppError::database)?;
        let message = session_message::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("session_message", id))?;
        let mut result = self.message_by_id(id).await?;
        // Runtime dispatch receives original bytes; public transcript reads are redacted.
        result.body = message.body;
        Ok(Some(result))
    }
}
