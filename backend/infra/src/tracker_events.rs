use super::*;
use domain::{TaskChatBinding, TrackerMetadataPage, TrackerOutboxPage, TrackerProjectionReceipt};

struct ProjectionEvent {
    sequence: i64,
    event_id: Uuid,
    event_type: String,
    hash: String,
    created_at: chrono::DateTime<chrono::Utc>,
    stage: domain::TrackerStage,
    revision: Option<i64>,
    summary: &'static str,
}

impl PostgresFleetRepository {
    pub(crate) async fn metadata_targets(
        &self,
        instance: &str,
        projects: &[Uuid],
        after: Option<Uuid>,
    ) -> Result<Vec<domain::TrackerProjectionTarget>, AppError> {
        if instance.trim().is_empty() || projects.iter().any(Uuid::is_nil) {
            return Err(AppError::validation("invalid Tracker projection scope"));
        }
        let rows = self.db.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT b.session_id,to_jsonb(b)-'session_id'-'idempotency_key'-'created_at' AS binding
             FROM task_chat_bindings b JOIN agent_sessions s ON s.id=b.session_id
             JOIN users u ON u.id=s.user_id
             WHERE b.tracker_instance_id=$1 AND u.is_active
               AND b.project_id IN (SELECT value::uuid FROM jsonb_array_elements_text($2::jsonb))
               AND ($3::uuid IS NULL OR b.session_id>$3)
             ORDER BY b.session_id LIMIT 100",
             [instance.into(),json!(projects).into(),after.into()]
        )).await.map_err(AppError::database)?;
        rows.into_iter()
            .map(|row| {
                Ok(domain::TrackerProjectionTarget {
                    session_id: row.try_get("", "session_id").map_err(AppError::database)?,
                    binding: serde_json::from_value(
                        row.try_get::<Value>("", "binding")
                            .map_err(AppError::database)?,
                    )
                    .map_err(AppError::internal)?,
                })
            })
            .collect()
    }

    pub(crate) async fn source_event_cursor(
        &self,
        session: Uuid,
        projection: &str,
    ) -> Result<i64, AppError> {
        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT COALESCE(c.sequence,0) AS sequence,c.projection,c.contract_version FROM task_chat_bindings b
             LEFT JOIN tracker_event_cursors c ON c.session_id=b.session_id WHERE b.session_id=$1",
                [session.into()],
            ))
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("task binding", session))?;
        let actual = row
            .try_get::<Option<String>>("", "projection")
            .map_err(AppError::database)?;
        if actual.is_some()
            && (actual.as_deref() != Some(projection)
                || row
                    .try_get::<i16>("", "contract_version")
                    .map_err(AppError::database)?
                    != 1)
        {
            return Err(AppError::conflict(
                "Tracker projection requires explicit migration",
            ));
        }
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
        let events = page
            .events
            .into_iter()
            .map(|event| {
                Ok(ProjectionEvent {
                    hash: hex::encode(Sha256::digest(
                        serde_json::to_vec(&event).map_err(AppError::internal)?,
                    )),
                    summary: event.summary()?,
                    sequence: event.sequence,
                    event_id: event.event_id,
                    event_type: event.event_type,
                    created_at: event.created_at,
                    stage: event.payload.stage,
                    revision: event.payload.requirement_revision,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        self.persist_projection(session, binding, after, "legacy_full_v1", events)
            .await
    }

    pub(crate) async fn persist_tracker_metadata(
        &self,
        session: Uuid,
        binding: TaskChatBinding,
        after: i64,
        page: TrackerMetadataPage,
    ) -> Result<TrackerProjectionReceipt, AppError> {
        page.validate(&binding, after)?;
        let events = page
            .events
            .into_iter()
            .map(|event| {
                Ok(ProjectionEvent {
                    summary: event.summary()?,
                    sequence: domain::tracker_metadata_cursor(&event.sequence)?,
                    created_at: event.source_time()?,
                    event_id: event.event_id,
                    event_type: event.event_type,
                    hash: event.metadata_sha256,
                    stage: event.payload.stage,
                    revision: event.payload.current_requirement_revision,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        self.persist_projection(session, binding, after, "metadata_v1", events)
            .await
    }

    async fn persist_projection(
        &self,
        session: Uuid,
        binding: TaskChatBinding,
        after: i64,
        projection: &str,
        events: Vec<ProjectionEvent>,
    ) -> Result<TrackerProjectionReceipt, AppError> {
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
            "INSERT INTO tracker_event_cursors(session_id,sequence,projection,contract_version) VALUES($1,0,$2,1) ON CONFLICT DO NOTHING",
            [session.into(),projection.into()])).await.map_err(AppError::database)?;
        let cursor_row = txn
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT sequence,projection,contract_version FROM tracker_event_cursors WHERE session_id=$1 FOR UPDATE",
                [session.into()],
            ))
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::internal("missing Tracker event cursor"))?;
        if cursor_row
            .try_get::<String>("", "projection")
            .map_err(AppError::database)?
            != projection
            || cursor_row
                .try_get::<i16>("", "contract_version")
                .map_err(AppError::database)?
                != 1
        {
            return Err(AppError::conflict(
                "Tracker projection requires explicit migration",
            ));
        }
        let cursor = cursor_row
            .try_get::<i64>("", "sequence")
            .map_err(AppError::database)?;

        let mut pending = Vec::new();
        for event in events {
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
                        != event.hash
                {
                    return Err(AppError::conflict("Tracker event replay payload changed"));
                }
            } else {
                pending.push(event);
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
        for event in pending {
            let message = Uuid::new_v4();
            let body = event.summary;
            txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "INSERT INTO session_messages(id,session_id,author_type,body,message_kind,runtime_message_id,delivery_state,created_at)
                 VALUES($1,$2,'system',$3,'system_event',$4,'mirrored',$5)",
                [message.into(),session.into(),body.into(),format!("tracker:{}",event.event_id).into(),event.created_at.into()]
            )).await.map_err(AppError::database)?;
            txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "INSERT INTO tracker_event_inbox(session_id,event_id,source_sequence,event_type,payload_hash,message_id,source_created_at)
                 VALUES($1,$2,$3,$4,$5,$6,$7)",
                [session.into(),event.event_id.into(),event.sequence.into(),event.event_type.clone().into(),event.hash.into(),message.into(),event.created_at.into()]
            )).await.map_err(AppError::database)?;
            let sequence = if projection == "metadata_v1" {
                json!(event.sequence.to_string())
            } else {
                json!(event.sequence)
            };
            let payload = json!({"type":"tracker_event", "session_id":session,
                "source_event_id":event.event_id,"source_sequence":sequence,
                "event_type":event.event_type,"requirement_revision":event.revision,
                "stage":event.stage});
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
