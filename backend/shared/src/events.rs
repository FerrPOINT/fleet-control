use serde::Serialize;

/// Real-time event delivered over SSE at `/api/v1/events`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FleetEvent {
    AgentCreated {
        agent_id: String,
        name: String,
    },
    AgentUpdated {
        agent_id: String,
        name: String,
    },
    RuntimeChanged {
        agent_id: String,
        status: String,
    },
    SkillChanged {
        agent_id: String,
        skill: String,
    },
    SessionChanged {
        session_id: String,
        agent_id: String,
    },
    SessionMessageChanged {
        session_id: String,
        message_id: String,
        event: String,
    },
    SessionRunChanged {
        session_id: String,
        run_id: String,
        runtime_run_id: Option<String>,
        state: String,
    },
    SessionRunDelta {
        session_id: String,
        run_id: String,
        runtime_run_id: Option<String>,
        delta: String,
    },
    RuntimeApprovalRequested {
        session_id: String,
        run_id: String,
        approval_id: String,
    },
    WorkflowBindingChanged {
        agent_id: String,
    },
}
