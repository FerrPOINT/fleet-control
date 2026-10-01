use super::*;
use domain::{TaskChatBinding, TrackerOutboxPage, TrackerProjectionReceipt};

impl PostgresFleetRepository {
    pub(crate) async fn source_event_cursor(&self, session: Uuid) -> Result<i64, AppError> {
        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT COALESCE(c.sequence,0) AS sequence FROM task_chat_bindings b
             LEFT JOIN tracker_event_cursors c ON c.session_id=b.session_id WHERE b.session_id=$1",
                [session.into()],
            ))
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("task binding", session))?;
        row.try_get("", "sequence").map_err(AppError::database)
    }

    // This consumes a trusted gateway response, never caller-supplied event JSON.
    // Authorization and dependency identity must be checked before the gateway fetch.
    pub(crate) async fn persist_tracker_page(
        &self,
        session: Uuid,
        binding: TaskChatBinding,
        after: i64,
        page: TrackerOutboxPage,
    ) -> Result<TrackerProjectionReceipt, AppError> {
        page.validate(&binding, after)?;
        let txn = self.db.begin().await.map_err(AppError::database)?;
        // Match the existing mirror lock order: session -> source cursor -> stream cursor.
        let row = txn
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT to_jsonb(b)-'session_id'-'idempotency_key'-'created_at' AS binding
             FROM agent_sessions s JOIN task_chat_bindings b ON b.session_id=s.id
             WHERE s.id=$1 FOR NO KEY UPDATE OF s",
                [session.into()],
            ))
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("task binding", session))?;
        let actual: TaskChatBinding = serde_json::from_value(
            row.try_get::<Value>("", "binding")
                .map_err(AppError::database)?,
        )
        .map_err(AppError::internal)?;
        if actual != binding {
            return Err(AppError::conflict("Tracker projection binding changed"));
        }
        txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO tracker_event_cursors(session_id,sequence) VALUES($1,0) ON CONFLICT DO NOTHING",
            [session.into()])).await.map_err(AppError::database)?;
        let cursor = txn
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT sequence FROM tracker_event_cursors WHERE session_id=$1 FOR UPDATE",
                [session.into()],
            ))
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::internal("missing Tracker event cursor"))?
            .try_get::<i64>("", "sequence")
            .map_err(AppError::database)?;

        let mut pending = Vec::new();
        for event in page.events {
            let hash = hex::encode(Sha256::digest(
                serde_json::to_vec(&event).map_err(AppError::internal)?,
            ));
            let previous = txn
                .query_all(Statement::from_sql_and_values(
                    DatabaseBackend::Postgres,
                    "SELECT event_id,source_sequence,payload_hash FROM tracker_event_inbox
                 WHERE session_id=$1 AND (event_id=$2 OR source_sequence=$3)",
                    [session.into(), event.event_id.into(), event.sequence.into()],
                ))
                .await
                .map_err(AppError::database)?;
            if !previous.is_empty() {
                if previous.len() != 1
                    || previous[0]
                        .try_get::<Uuid>("", "event_id")
                        .map_err(AppError::database)?
                        != event.event_id
                    || previous[0]
                        .try_get::<i64>("", "source_sequence")
                        .map_err(AppError::database)?
                        != event.sequence
                    || previous[0]
                        .try_get::<String>("", "payload_hash")
                        .map_err(AppError::database)?
                        != hash
                {
                    return Err(AppError::conflict("Tracker event replay payload changed"));
                }
            } else {
                pending.push((event, hash));
            }
        }
        if !pending.is_empty() && after != cursor {
            return Err(AppError::conflict(
                "Tracker projection cursor changed; refetch page",
            ));
        }
        if pending.is_empty() {
            if after > cursor {
                return Err(AppError::conflict(
                    "Tracker page starts beyond persisted cursor",
                ));
            }
            txn.commit().await.map_err(AppError::database)?;
            return Ok(TrackerProjectionReceipt {
                cursor,
                projected: 0,
            });
        }
        let projected = pending.len();
        let mut next_cursor = cursor;
        for (event, hash) in pending {
            let message = Uuid::new_v4();
            let body = event.summary()?;
            txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "INSERT INTO session_messages(id,session_id,author_type,body,message_kind,runtime_message_id,delivery_state,created_at)
                 VALUES($1,$2,'system',$3,'system_event',$4,'mirrored',$5)",
                [message.into(),session.into(),body.into(),format!("tracker:{}",event.event_id).into(),event.created_at.into()]
            )).await.map_err(AppError::database)?;
            txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "INSERT INTO tracker_event_inbox(session_id,event_id,source_sequence,event_type,payload_hash,message_id,source_created_at)
                 VALUES($1,$2,$3,$4,$5,$6,$7)",
                [session.into(),event.event_id.into(),event.sequence.into(),event.event_type.clone().into(),hash.into(),message.into(),event.created_at.into()]
            )).await.map_err(AppError::database)?;
            let payload = json!({"type":"tracker_event", "session_id":session,
                "source_event_id":event.event_id,"source_sequence":event.sequence,
                "event_type":event.event_type,"requirement_revision":event.payload.requirement_revision,
                "stage":event.payload.stage});
            txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "WITH cursor AS (
                    INSERT INTO session_event_cursors(session_id,sequence) VALUES($1,1)
                    ON CONFLICT(session_id) DO UPDATE SET sequence=session_event_cursors.sequence+1 RETURNING sequence)
                 INSERT INTO session_events(session_id,sequence,event_type,payload)
                    SELECT $1,sequence,'tracker_event',$2 FROM cursor",
                [session.into(),payload.into()]
            )).await.map_err(AppError::database)?;
            next_cursor = event.sequence;
        }
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "UPDATE tracker_event_cursors SET sequence=$2 WHERE session_id=$1",
            [session.into(), next_cursor.into()],
        ))
        .await
        .map_err(AppError::database)?;
        txn.commit().await.map_err(AppError::database)?;
        Ok(TrackerProjectionReceipt {
            cursor: next_cursor,
            projected,
        })
    }
}
