use super::*;
use domain::{MessageHistoryPage, TaskChatBinding};
use sea_orm::FromQueryResult;

impl PostgresFleetRepository {
    pub(crate) async fn message_by_id(&self, id: Uuid) -> Result<SessionMessage, AppError> {
        let row = session_message::Entity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("session_message", id))?;
        let author_type = parse_message_author_type(&row.author_type);
        let author_user = match row.author_user_id {
            Some(id) => user::Entity::find_by_id(id)
                .one(&self.db)
                .await
                .map_err(AppError::database)?,
            None => None,
        };
        let author_agent = match row.author_agent_id {
            Some(id) => agent::Entity::find_by_id(id)
                .one(&self.db)
                .await
                .map_err(AppError::database)?,
            None => None,
        };
        Ok(SessionMessage {
            id: row.id,
            session_id: row.session_id,
            author_type,
            author_user_id: row.author_user_id,
            author_agent_id: row.author_agent_id,
            author_display_name: message_author_display_name(
                author_type,
                author_user.as_ref(),
                author_agent.as_ref(),
            ),
            body: redact_text(&row.body),
            message_kind: parse_message_kind(&row.message_kind),
            runtime_message_id: row.runtime_message_id,
            delivery_state: parse_message_delivery_state(&row.delivery_state),
            delivery_error: row.delivery_error.map(|error| redact_text(&error)),
            replayed: false,
            created_at: api_ts(row.created_at),
        })
    }
    pub(crate) async fn task_binding(
        &self,
        session_id: Uuid,
    ) -> Result<Option<TaskChatBinding>, AppError> {
        let row = self.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT to_jsonb(b) - 'session_id' - 'idempotency_key' - 'created_at' AS binding FROM task_chat_bindings b WHERE session_id=$1",
            [session_id.into()])).await.map_err(AppError::database)?;
        row.map(|row| {
            let value: Value = row.try_get("", "binding").map_err(AppError::database)?;
            serde_json::from_value(value).map_err(AppError::internal)
        })
        .transpose()
    }

    pub(crate) async fn persist_task_binding(
        &self,
        session_id: Uuid,
        binding: TaskChatBinding,
        key: String,
    ) -> Result<TaskChatBinding, AppError> {
        if key.is_empty()
            || key.len() > 128
            || key
                .chars()
                .any(|value| value.is_whitespace() || value.is_control())
        {
            return Err(AppError::validation(
                "idempotency_key must contain 1..128 bytes",
            ));
        }
        let txn = self.db.begin().await.map_err(AppError::database)?;
        let row = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT s.agent_id,s.user_id,s.visibility,s.leader_agent_id,u.central_sub FROM agent_sessions s
                JOIN users u ON u.id=s.user_id WHERE s.id=$1 FOR UPDATE OF s", [session_id.into()]))
            .await.map_err(AppError::database)?.ok_or_else(|| AppError::not_found("session", session_id))?;
        let agent: Uuid = row.try_get("", "agent_id").map_err(AppError::database)?;
        let owner: Uuid = row.try_get("", "user_id").map_err(AppError::database)?;
        let subject: Option<String> = row.try_get("", "central_sub").map_err(AppError::database)?;
        let leader: Option<Uuid> = row
            .try_get("", "leader_agent_id")
            .map_err(AppError::database)?;
        let visibility: String = row.try_get("", "visibility").map_err(AppError::database)?;
        if agent != binding.agent_id
            || subject.as_deref() != Some(binding.owner_subject.as_str())
            || leader.is_some()
            || visibility != "private"
        {
            return Err(AppError::Forbidden);
        }
        let existing = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT to_jsonb(b) - 'session_id' - 'idempotency_key' - 'created_at' AS binding FROM task_chat_bindings b WHERE session_id=$1", [session_id.into()]))
            .await.map_err(AppError::database)?;
        if let Some(existing) = existing {
            let value: Value = existing
                .try_get("", "binding")
                .map_err(AppError::database)?;
            let previous: TaskChatBinding =
                serde_json::from_value(value).map_err(AppError::internal)?;
            if previous != binding {
                return Err(AppError::conflict("task chat binding is immutable"));
            }
            return Ok(previous);
        }
        let messages = txn
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "SELECT (EXISTS(SELECT 1 FROM session_messages WHERE session_id=$1 AND author_type <> 'system')
                    OR EXISTS(SELECT 1 FROM session_agent_runs WHERE session_id=$1 AND
                        (state <> 'pending' OR runtime_session_id IS NOT NULL OR runtime_run_id IS NOT NULL OR last_event_at IS NOT NULL))) AS present",
                [session_id.into()],
            ))
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::internal("missing message count"))?;
        if messages
            .try_get::<bool>("", "present")
            .map_err(AppError::database)?
        {
            return Err(AppError::conflict(
                "only an empty private chat can be explicitly bound",
            ));
        }
        let inserted = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
                VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(tracker_instance_id,task_id,agent_id) DO NOTHING RETURNING session_id",
            [session_id.into(), binding.tracker_instance_id.clone().into(), binding.project_id.into(), binding.task_id.into(),
                binding.root_task_id.into(), binding.agent_id.into(), binding.owner_subject.clone().into(), key.into()]))
            .await.map_err(AppError::database)?;
        if inserted.is_none() {
            return Err(AppError::conflict(
                "task already has a chat for this concrete agent",
            ));
        }
        let payload = json!({"type":"task.bound", "session_id":session_id,
            "tracker_instance_id":binding.tracker_instance_id,"task_id":binding.task_id,
            "root_task_id":binding.root_task_id,"project_id":binding.project_id,"agent_id":binding.agent_id});
        audit_log::Entity::insert(audit_log::ActiveModel {
            id: Set(Uuid::new_v4()),
            actor_user_id: Set(Some(owner)),
            action: Set("session.task.bind".into()),
            entity_type: Set("session".into()),
            entity_id: Set(Some(session_id.to_string())),
            payload: Set(payload.clone()),
            created_at: Set(now()),
        })
        .exec(&txn)
        .await
        .map_err(AppError::database)?;
        txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "WITH cursor AS (
                INSERT INTO session_event_cursors(session_id,sequence) VALUES($1,1)
                ON CONFLICT(session_id) DO UPDATE SET sequence=session_event_cursors.sequence+1 RETURNING sequence)
             INSERT INTO session_events(session_id,sequence,event_type,payload)
                SELECT $1,sequence,'task.bound',$2 FROM cursor", [session_id.into(),payload.into()]))
            .await.map_err(AppError::database)?;
        txn.commit().await.map_err(AppError::database)?;
        Ok(binding)
    }

    pub(crate) async fn paged_message_history(
        &self,
        session_id: Uuid,
        before: Option<Uuid>,
        limit: u64,
    ) -> Result<MessageHistoryPage, AppError> {
        if !(1..=100).contains(&limit) {
            return Err(AppError::validation(
                "history limit must be between 1 and 100",
            ));
        }
        if let Some(id) = before {
            let cursor = session_message::Entity::find_by_id(id)
                .filter(session_message::Column::SessionId.eq(session_id))
                .one(&self.db)
                .await
                .map_err(AppError::database)?;
            if cursor.is_none() {
                return Err(AppError::validation("invalid history cursor"));
            }
        }
        let rows = self.db.query_all(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT m.id,m.session_id,m.author_type,m.author_user_id,m.author_agent_id,m.body,m.message_kind,
                m.runtime_message_id,m.idempotency_key,m.idempotency_payload_hash,m.created_by_user_id,
                m.delivery_state,m.delivery_error,m.created_at,
                CASE m.author_type WHEN 'user' THEN COALESCE(u.display_name,'Unknown user')
                    WHEN 'agent' THEN COALESCE(a.display_name,'Unknown agent') ELSE 'Fleet Control' END AS author_display_name
                FROM session_messages m LEFT JOIN users u ON u.id=m.author_user_id
                    LEFT JOIN agents a ON a.id=m.author_agent_id WHERE m.session_id=$1 AND ($2::uuid IS NULL OR
                (m.created_at,m.id)<(SELECT created_at,id FROM session_messages WHERE id=$2 AND session_id=$1))
                ORDER BY m.created_at DESC,m.id DESC LIMIT $3",
            [session_id.into(), before.into(), ((limit + 1) as i64).into()]))
            .await.map_err(AppError::database)?;
        let has_more = rows.len() > limit as usize;
        let mut items = Vec::new();
        for row in rows.into_iter().take(limit as usize) {
            let author_display_name: String = row
                .try_get("", "author_display_name")
                .map_err(AppError::database)?;
            let row =
                session_message::Model::from_query_result(&row, "").map_err(AppError::database)?;
            let author_type = parse_message_author_type(&row.author_type);
            items.push(SessionMessage {
                id: row.id,
                session_id: row.session_id,
                author_type,
                author_user_id: row.author_user_id,
                author_agent_id: row.author_agent_id,
                author_display_name,
                body: redact_text(&row.body),
                message_kind: parse_message_kind(&row.message_kind),
                runtime_message_id: row.runtime_message_id,
                delivery_state: parse_message_delivery_state(&row.delivery_state),
                delivery_error: row.delivery_error.map(|error| redact_text(&error)),
                replayed: false,
                created_at: api_ts(row.created_at),
            });
        }
        let next_before = if has_more {
            items.last().map(|item| item.id)
        } else {
            None
        };
        items.reverse();
        Ok(MessageHistoryPage { items, next_before })
    }
}
