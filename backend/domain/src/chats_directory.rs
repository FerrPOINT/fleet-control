use crate::{AgentDirectoryItem, AgentSession};
use serde::{Deserialize, Serialize};
use shared::AppError;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TaskProjectAccess {
    pub tracker_instance_id: String,
    pub project_ids: Vec<Uuid>,
}

#[derive(Debug, Clone)]
pub struct ChatsDirectoryFilter {
    pub agent_id: Option<Uuid>,
    pub user_ids: Vec<Uuid>,
    pub include_all_users: bool,
    pub search: String,
    pub before: Option<Uuid>,
    pub limit: u64,
    // Server-derived scope only; absent proof never authorizes a task-bound chat.
    pub task_project_access: Option<TaskProjectAccess>,
    pub private_user_id: Option<Uuid>,
}

impl ChatsDirectoryFilter {
    pub fn validate(&self) -> Result<(), AppError> {
        if !(1..=100).contains(&self.limit) {
            return Err(AppError::validation("limit must be between 1 and 100"));
        }
        if self.search.chars().count() > 200 {
            return Err(AppError::validation(
                "q must contain at most 200 characters",
            ));
        }
        if self.before.is_some() && self.agent_id.is_none() {
            return Err(AppError::validation("before requires agent_id"));
        }
        if !self.include_all_users && self.user_ids.is_empty() {
            return Err(AppError::Forbidden);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatsDirectoryAgent {
    pub agent: AgentDirectoryItem,
    pub matching_session_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChatsDirectoryPage {
    pub agents: Vec<ChatsDirectoryAgent>,
    pub selected_agent_id: Option<Uuid>,
    pub items: Vec<AgentSession>,
    pub next_before: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter() -> ChatsDirectoryFilter {
        ChatsDirectoryFilter {
            agent_id: None,
            user_ids: vec![Uuid::new_v4()],
            include_all_users: false,
            search: String::new(),
            before: None,
            limit: 50,
            task_project_access: None,
            private_user_id: None,
        }
    }

    #[test]
    fn validates_limits_search_and_cursor_agent() {
        let mut query = filter();
        for limit in [0, 101, u64::MAX] {
            query.limit = limit;
            assert!(query.validate().is_err());
        }
        query.limit = 100;
        query.search = "x".repeat(201);
        assert!(query.validate().is_err());
        query.search = "x".repeat(200);
        assert!(query.validate().is_ok());
        query.before = Some(Uuid::new_v4());
        assert!(query.validate().is_err());
        query.agent_id = Some(Uuid::new_v4());
        assert!(query.validate().is_ok());
    }

    #[test]
    fn empty_user_scope_never_becomes_all_users() {
        let mut query = filter();
        query.user_ids.clear();
        assert!(matches!(query.validate(), Err(AppError::Forbidden)));
        query.include_all_users = true;
        assert!(query.validate().is_ok());
    }
}
