use super::*;
use domain::{MessageHistoryPage, TaskChatBinding};
use sea_orm::FromQueryResult;

impl PostgresFleetRepository {
    pub(crate) async fn persist_pm_draft_chat(
        &self,
        command: domain::CreatePmDraftChat,
        owner: Uuid,
    ) -> Result<AgentSession, AppError> {
        let binding = &command.binding;
        let valid_subject = Uuid::parse_str(&binding.owner_subject)
            .is_ok_and(|id| !id.is_nil() && id.to_string() == binding.owner_subject);
        if !valid_subject
            || owner.is_nil()
            || [
                binding.project_id,
                binding.task_id,
                binding.root_task_id,
                binding.agent_id,
            ]
            .iter()
            .any(Uuid::is_nil)
            || binding.task_id != binding.root_task_id
            || binding.tracker_instance_id.is_empty()
            || binding.tracker_instance_id.len() > 128
            || binding
                .tracker_instance_id
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
            || command.title.trim().is_empty()
            || command.title.chars().count() > 500
            || command.title.chars().any(char::is_control)
            || command.task_key.is_empty()
            || command.task_key.len() > 128
            || command
                .task_key
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
            || command.idempotency_key.is_empty()
            || command.idempotency_key.len() > 128
            || command
                .idempotency_key
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
        {
            return Err(AppError::validation("invalid server-derived PM Draft chat"));
        }
        let hash = payload_hash(&json!({"operation":"pm_draft_chat_v1","command":command}))?;
        let txn = self.db.begin().await.map_err(AppError::database)?;
        // Share the actor/key lock with ordinary session creation; no free chat is committed.
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT pg_advisory_xact_lock(hashtextextended($1,0))",
            [format!("session:{owner}:{}", command.idempotency_key).into()],
        ))
        .await
        .map_err(AppError::database)?;
        let user = user::Entity::find_by_id(owner)
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or(AppError::Unauthorized)?;
        if !user.is_active || user.central_sub.as_deref() != Some(binding.owner_subject.as_str()) {
            return Err(AppError::Forbidden);
        }
        if let Some(existing) = agent_session::Entity::find()
            .filter(agent_session::Column::UserId.eq(owner))
            .filter(agent_session::Column::IdempotencyKey.eq(&command.idempotency_key))
            .one(&txn)
            .await
            .map_err(AppError::database)?
        {
            if existing.idempotency_payload_hash.as_deref() != Some(hash.as_str()) {
                return Err(AppError::conflict(
                    "session command key has a different payload",
                ));
            }
            let saved = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
                "SELECT session_id FROM task_chat_bindings WHERE session_id=$1 AND tracker_instance_id=$2
                 AND project_id=$3 AND task_id=$4 AND root_task_id=$5 AND agent_id=$6 AND owner_subject=$7",
                [existing.id.into(),binding.tracker_instance_id.clone().into(),binding.project_id.into(),
                 binding.task_id.into(),binding.root_task_id.into(),binding.agent_id.into(),binding.owner_subject.clone().into()]))
                .await.map_err(AppError::database)?;
            if saved.is_none() {
                return Err(AppError::conflict("PM Draft binding cannot be recovered"));
            }
            txn.commit().await.map_err(AppError::database)?;
            return self.get_session(existing.id).await;
        }
        let agent = agent::Entity::find_by_id(binding.agent_id)
            .one(&txn)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::not_found("agent", binding.agent_id))?;
        if agent.status == "archived"
            || agent.kind != "hermes"
            || agent.sdlc_role.as_deref() != Some("project_manager")
        {
            return Err(AppError::conflict(
                "PM Draft requires an available concrete Hermes PM agent",
            ));
        }
        let id = Uuid::new_v4();
        txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO agent_sessions(id,agent_id,user_id,title,task_key,state,visibility,namespace_id,idempotency_key,idempotency_payload_hash)
             VALUES($1,$2,$3,$4,$5,'draft','private',$6,$7,$8)",
            [id.into(),binding.agent_id.into(),owner.into(),command.title.clone().into(),command.task_key.clone().into(),
             agent.namespace_id.into(),command.idempotency_key.clone().into(),hash.into()])).await.map_err(AppError::database)?;
        let inserted = txn.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO task_chat_bindings(session_id,tracker_instance_id,project_id,task_id,root_task_id,agent_id,owner_subject,idempotency_key)
             VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(tracker_instance_id,task_id,agent_id) DO NOTHING RETURNING session_id",
            [id.into(),binding.tracker_instance_id.clone().into(),binding.project_id.into(),binding.task_id.into(),
             binding.root_task_id.into(),binding.agent_id.into(),binding.owner_subject.clone().into(),command.idempotency_key.into()]))
            .await.map_err(AppError::database)?;
        if inserted.is_none() {
            return Err(AppError::conflict(
                "task already has a chat for this concrete agent",
            ));
        }
        txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "INSERT INTO session_participants(id,session_id,participant_type,user_id,agent_id,session_role)
             VALUES($1,$2,'user',$3,NULL,'owner'),($4,$2,'agent',NULL,$5,'primary')",
            [Uuid::new_v4().into(),id.into(),owner.into(),Uuid::new_v4().into(),binding.agent_id.into()]))
            .await.map_err(AppError::database)?;
        let event = json!({"type":"task.bound","session_id":id,"tracker_instance_id":binding.tracker_instance_id,
            "task_id":binding.task_id,"root_task_id":binding.root_task_id,"project_id":binding.project_id,"agent_id":binding.agent_id});
        txn.execute(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "INSERT INTO audit_log(id,actor_user_id,action,entity_type,entity_id,payload)
             VALUES($1,$2,'session.pm_draft.create','session',$3,$4)",
            [
                Uuid::new_v4().into(),
                owner.into(),
                id.to_string().into(),
                event.clone().into(),
            ],
        ))
        .await
        .map_err(AppError::database)?;
        txn.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "WITH cursor AS (INSERT INTO session_event_cursors(session_id,sequence) VALUES($1,1)
                ON CONFLICT(session_id) DO UPDATE SET sequence=session_event_cursors.sequence+1 RETURNING sequence)
             INSERT INTO session_events(session_id,sequence,event_type,payload) SELECT $1,sequence,'task.bound',$2 FROM cursor",
            [id.into(),event.into()])).await.map_err(AppError::database)?;
        txn.commit().await.map_err(AppError::database)?;
        self.get_session(id).await
    }

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
            request_payload_hash: None,
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
                m.append_sequence<(SELECT append_sequence FROM session_messages WHERE id=$2 AND session_id=$1))
                ORDER BY m.append_sequence DESC LIMIT $3",
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
                request_payload_hash: None,
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
