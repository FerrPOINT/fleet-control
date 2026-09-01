use uuid::Uuid;

pub type UserId = Uuid;
pub type AgentId = Uuid;
pub type SessionId = Uuid;
pub type SkillId = Uuid;
pub type WorkflowBindingId = Uuid;
pub type AgentEventId = Uuid;
pub type AgentLogId = Uuid;

pub fn new_id() -> Uuid {
    Uuid::new_v4()
}
