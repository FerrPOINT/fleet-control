pub mod user {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub email: String,
        pub username: String,
        pub display_name: String,
        pub password_hash: String,
        pub refresh_token_hash: Option<String>,
        pub system_role: String,
        pub is_system_admin: bool,
        pub is_active: bool,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod agent {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "agents")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub ordinal: i32,
        pub name: String,
        pub kind: String,
        pub product_role: String,
        pub role: String,
        pub status: String,
        pub display_name: String,
        pub description: Option<String>,
        pub namespace_id: Option<String>,
        pub workflow_id: Option<String>,
        pub runtime_version: Option<String>,
        pub dashboard_port: Option<i32>,
        pub api_port: Option<i32>,
        pub runtime_path: String,
        pub config_path: String,
        pub workspace_path: String,
        pub logs_path: String,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: DateTimeWithTimeZone,
        pub archived_at: Option<DateTimeWithTimeZone>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod agent_runtime {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "agent_runtime")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub agent_id: Uuid,
        pub desired_state: String,
        pub pid: Option<i32>,
        pub health_status: Option<String>,
        pub health_detail: Option<String>,
        pub command_preview: String,
        pub env_preview: Json,
        pub last_capabilities_json: Json,
        pub startup_command_redacted: Option<String>,
        pub started_at: Option<DateTimeWithTimeZone>,
        pub stopped_at: Option<DateTimeWithTimeZone>,
        pub last_health_at: Option<DateTimeWithTimeZone>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod agent_config {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "agent_configs")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub agent_id: Uuid,
        pub config_json: Json,
        pub soul_md: String,
        pub env_json: Json,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod agent_skill {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "agent_skills")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub agent_id: Uuid,
        pub name: String,
        pub title: String,
        pub state: String,
        pub source: String,
        pub content: Option<String>,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod agent_session {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "agent_sessions")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub agent_id: Uuid,
        pub user_id: Uuid,
        pub leader_agent_id: Option<Uuid>,
        pub parent_session_id: Option<Uuid>,
        pub created_by_leader_agent_id: Option<Uuid>,
        pub visibility: String,
        pub title: String,
        pub task_key: Option<String>,
        pub state: String,
        pub namespace_id: Option<String>,
        pub external_session_id: Option<String>,
        pub last_message_preview: Option<String>,
        pub idempotency_key: Option<String>,
        pub idempotency_payload_hash: Option<String>,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod leader_executor {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "leader_executors")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub leader_agent_id: Uuid,
        #[sea_orm(primary_key, auto_increment = false)]
        pub executor_agent_id: Uuid,
        pub created_by_user_id: Option<Uuid>,
        pub created_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod session_participant {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "session_participants")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub session_id: Uuid,
        pub participant_type: String,
        pub user_id: Option<Uuid>,
        pub agent_id: Option<Uuid>,
        pub session_role: String,
        pub created_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod session_message {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "session_messages")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub session_id: Uuid,
        pub author_type: String,
        pub author_user_id: Option<Uuid>,
        pub author_agent_id: Option<Uuid>,
        pub body: String,
        pub message_kind: String,
        pub runtime_message_id: Option<String>,
        pub idempotency_key: Option<String>,
        pub idempotency_payload_hash: Option<String>,
        pub created_by_user_id: Option<Uuid>,
        pub delivery_state: String,
        pub delivery_error: Option<String>,
        pub created_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod session_agent_run {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "session_agent_runs")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub session_id: Uuid,
        pub agent_id: Uuid,
        pub runtime_session_id: Option<String>,
        pub runtime_run_id: Option<String>,
        pub run_role: String,
        pub state: String,
        pub last_error: Option<String>,
        pub last_event_at: Option<DateTimeWithTimeZone>,
        pub model: Option<String>,
        pub provider: Option<String>,
        pub model_options: Json,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod runtime_approval_request {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "runtime_approval_requests")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub session_id: Uuid,
        pub session_run_id: Uuid,
        pub agent_id: Uuid,
        pub runtime_run_id: String,
        pub runtime_approval_id: Option<String>,
        pub prompt: String,
        pub detail: Json,
        pub state: String,
        pub resolved_by_user_id: Option<Uuid>,
        pub resolved_at: Option<DateTimeWithTimeZone>,
        pub created_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod workflow_binding {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "workflow_bindings")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub agent_id: Uuid,
        pub namespace_id: Option<String>,
        pub namespace_name: Option<String>,
        pub workflow_id: Option<String>,
        pub workflow_name: Option<String>,
        pub binding_status: String,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod runtime_template {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "runtime_templates")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub kind: String,
        pub display_name: String,
        pub implemented: bool,
        pub enabled: bool,
        pub description: String,
        pub capabilities: Json,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod agent_event {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "agent_events")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub agent_id: Option<Uuid>,
        pub event_type: String,
        pub message: String,
        pub payload: Json,
        pub created_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod agent_log {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "agent_logs")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub agent_id: Uuid,
        pub stream: String,
        pub message: String,
        pub created_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod audit_log {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "audit_log")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub actor_user_id: Option<Uuid>,
        pub action: String,
        pub entity_type: String,
        pub entity_id: Option<String>,
        pub payload: Json,
        pub created_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod deployment_job {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "deployment_jobs")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub job_kind: String,
        pub state: String,
        pub agent_id: Option<Uuid>,
        pub runtime_kind: Option<String>,
        pub requested_by_user_id: Option<Uuid>,
        pub title: String,
        pub detail: Json,
        pub last_error: Option<String>,
        pub created_at: DateTimeWithTimeZone,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod control_setting {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "control_settings")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub key: String,
        pub value_json: Json,
        pub updated_by_user_id: Option<Uuid>,
        pub updated_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
