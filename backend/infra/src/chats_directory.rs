use crate::{PostgresFleetRepository, redact_text};
use domain::{AgentSession, ChatsDirectoryAgent, ChatsDirectoryFilter, ChatsDirectoryPage};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use serde::Deserialize;
use shared::AppError;
use uuid::Uuid;

// Counts, selection, cursor validation and page use one snapshot and one scoped relation.
const DIRECTORY_SQL: &str = r#"
WITH scoped AS MATERIALIZED (
    SELECT s.id, s.agent_id, s.created_at
    FROM agent_sessions s
    JOIN agents a ON a.id=s.agent_id AND a.status <> 'archived'
    JOIN users u ON u.id=s.user_id
    WHERE ($1::boolean OR s.user_id IN (SELECT value::uuid FROM jsonb_array_elements_text($2::jsonb)))
      AND (s.title ILIKE $3 ESCAPE E'\\' OR COALESCE(s.task_key,'') ILIKE $3 ESCAPE E'\\'
           OR u.display_name ILIKE $3 ESCAPE E'\\')
), counts AS (
    SELECT agent_id, count(*) AS matches FROM scoped GROUP BY agent_id
), directory AS MATERIALIZED (
    SELECT a.id, a.ordinal, COALESCE(c.matches,0) AS matches,
        jsonb_build_object('id',a.id,'ordinal',a.ordinal,'name',a.name,'kind',a.kind,
            'product_role',a.product_role,'role',a.role,'sdlc_role',a.sdlc_role,
            'status',a.status,'display_name',a.display_name,'description',a.description,
            'namespace_id',a.namespace_id,'workflow_id',a.workflow_id,
            'runtime_version',a.runtime_version,'dashboard_port',a.dashboard_port,'api_port',a.api_port) AS agent
    FROM agents a LEFT JOIN counts c ON c.agent_id=a.id WHERE a.status <> 'archived'
), selected AS (
    SELECT id FROM directory WHERE $4::uuid IS NULL OR id=$4
    ORDER BY CASE WHEN matches > 0 THEN 0 ELSE 1 END, ordinal, id LIMIT 1
), cursor AS (
    SELECT id,created_at FROM scoped WHERE id=$5::uuid AND agent_id=(SELECT id FROM selected)
), page AS (
    SELECT s.id,s.created_at,
        jsonb_build_object('id',s.id,'agent_id',s.agent_id,'primary_agent_id',s.agent_id,
            'agent_name',a.name,'primary_agent_name',a.name,'user_id',s.user_id,
            'user_email',u.email,'user_username',u.username,'user_display_name',u.display_name,
            'leader_agent_id',s.leader_agent_id,'leader_agent_name',l.name,
            'parent_session_id',s.parent_session_id,'created_by_leader_agent_id',s.created_by_leader_agent_id,
            'visibility',s.visibility,'title',s.title,'task_key',s.task_key,'state',s.state,
            'namespace_id',s.namespace_id,'external_session_id',s.external_session_id,
            'last_message_preview',s.last_message_preview,'created_at',s.created_at,'updated_at',s.updated_at) AS item
    FROM scoped f JOIN agent_sessions s ON s.id=f.id JOIN agents a ON a.id=s.agent_id
    JOIN users u ON u.id=s.user_id LEFT JOIN agents l ON l.id=s.leader_agent_id
    WHERE f.agent_id=(SELECT id FROM selected)
      AND ($5::uuid IS NULL OR (f.created_at,f.id)<(SELECT created_at,id FROM cursor))
    ORDER BY f.created_at DESC,f.id DESC LIMIT $6
)
SELECT jsonb_build_object(
    'agent_valid', $4::uuid IS NULL OR EXISTS(SELECT 1 FROM directory WHERE id=$4),
    'before_valid', $5::uuid IS NULL OR EXISTS(SELECT 1 FROM cursor),
    'agents', COALESCE((SELECT jsonb_agg(jsonb_build_object('agent',agent,'matching_session_count',matches) ORDER BY ordinal,id) FROM directory),'[]'::jsonb),
    'selected_agent_id', (SELECT id FROM selected),
    'items', COALESCE((SELECT jsonb_agg(item ORDER BY created_at DESC,id DESC) FROM page),'[]'::jsonb)
) AS payload
"#;

#[derive(Deserialize)]
struct DirectoryResult {
    agent_valid: bool,
    before_valid: bool,
    agents: Vec<ChatsDirectoryAgent>,
    selected_agent_id: Option<Uuid>,
    items: Vec<AgentSession>,
}

fn literal_search(value: &str) -> String {
    format!(
        "%{}%",
        value
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_")
    )
}

impl PostgresFleetRepository {
    pub async fn chats_directory(
        &self,
        filter: ChatsDirectoryFilter,
    ) -> Result<ChatsDirectoryPage, AppError> {
        filter.validate()?;
        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                DIRECTORY_SQL,
                vec![
                    filter.include_all_users.into(),
                    serde_json::json!(filter.user_ids).into(),
                    literal_search(filter.search.trim()).into(),
                    filter.agent_id.into(),
                    filter.before.into(),
                    ((filter.limit + 1) as i64).into(),
                ],
            ))
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::internal("missing chat directory result"))?;
        let payload: serde_json::Value = row.try_get("", "payload").map_err(AppError::database)?;
        let mut result: DirectoryResult =
            serde_json::from_value(payload).map_err(AppError::internal)?;
        if !result.agent_valid {
            return Err(AppError::validation("invalid or archived agent_id"));
        }
        if !result.before_valid {
            return Err(AppError::validation(
                "invalid cursor for the selected scope",
            ));
        }
        if result
            .agents
            .iter()
            .any(|item| item.matching_session_count > 9_007_199_254_740_991)
        {
            return Err(AppError::internal(
                "chat count exceeds safe JSON integer range",
            ));
        }
        let more = result.items.len() > filter.limit as usize;
        result.items.truncate(filter.limit as usize);
        let next_before = more
            .then(|| result.items.last().map(|item| item.id))
            .flatten();
        for item in &mut result.items {
            item.last_message_preview = item
                .last_message_preview
                .take()
                .map(|value| redact_text(&value));
        }
        Ok(ChatsDirectoryPage {
            agents: result.agents,
            selected_agent_id: result.selected_agent_id,
            items: result.items,
            next_before,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{MockDatabase, Value};
    use std::collections::BTreeMap;

    #[test]
    fn search_treats_sql_wildcards_and_backslash_literally() {
        assert_eq!(literal_search("a%b_c\\d"), "%a\\%b\\_c\\\\d%");
        assert_eq!(literal_search("' OR true --"), "%' OR true --%");
    }

    #[tokio::test]
    async fn one_query_serves_all_agents_counts_and_page_without_n_plus_one() {
        let owner = Uuid::new_v4();
        let agents: Vec<_> = (1..=40)
            .map(|ordinal| {
                serde_json::json!({
                    "agent": {
                        "id": Uuid::new_v4(), "ordinal": ordinal,
                        "name": format!("agent{ordinal}"), "display_name": "Directory agent",
                        "kind": "hermes", "product_role": "executor", "role": "developer",
                        "sdlc_role": "developer", "status": "running", "description": null,
                        "namespace_id": null, "workflow_id": null, "runtime_version": null,
                        "dashboard_port": 29002 + ordinal * 10, "api_port": 29001 + ordinal * 10
                    },
                    "matching_session_count": 205
                })
            })
            .collect();
        let agent_id: Uuid = serde_json::from_value(agents[0]["agent"]["id"].clone()).unwrap();
        let ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        let items: Vec<_> = ids.iter().map(|id| serde_json::json!({
            "id": id, "agent_id": agent_id, "primary_agent_id": agent_id,
            "agent_name": "agent1", "primary_agent_name": "agent1", "user_id": owner,
            "user_email": "owner@example.test", "user_username": "owner", "user_display_name": "Owner",
            "leader_agent_id": null, "leader_agent_name": null, "parent_session_id": null,
            "created_by_leader_agent_id": null, "visibility": "private", "title": "Directory chat",
            "task_key": null, "state": "draft", "namespace_id": null, "external_session_id": null,
            "last_message_preview": "message token=private", "created_at": "2026-10-01T12:00:00Z",
            "updated_at": "2026-10-01T12:00:00Z"
        })).collect();
        let payload = serde_json::json!({
            "agent_valid": true, "before_valid": true, "agents": agents,
            "selected_agent_id": agent_id, "items": items
        });
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results([[BTreeMap::from([("payload", Value::from(payload))])]])
            .into_connection();
        let repo = PostgresFleetRepository::new(db);
        let page = repo
            .chats_directory(ChatsDirectoryFilter {
                agent_id: Some(agent_id),
                user_ids: vec![owner],
                include_all_users: false,
                search: String::new(),
                before: None,
                limit: 2,
            })
            .await
            .unwrap();
        assert_eq!(page.agents.len(), 40);
        assert_eq!(page.agents[0].matching_session_count, 205);
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.next_before, Some(ids[1]));
        assert_eq!(
            page.items[0].last_message_preview.as_deref(),
            Some("message token=redacted")
        );
        assert_eq!(repo.db.into_transaction_log().len(), 1);
    }
}
