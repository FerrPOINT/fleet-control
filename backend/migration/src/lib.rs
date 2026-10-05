pub use sea_orm_migration::prelude::*;

mod m20260901_000001_fleet_control;
mod m20260901_000002_session_users;
mod m20260901_000003_leaders_sessions;
mod m20260901_000004_pre_development_hardening;
mod m20260901_000005_runtime_protocol;
mod m20260914_000006_fleet_alerts;
mod m20260918_000007_central_subject;
mod m20260923_000008_managed_settings;
mod m20260930_000009_product_deployments;
mod m20261001_000009_sdlc_foundation;
mod m20261001_000010_task_chats;
mod m20261004_000011_pm_credentials;
mod m20261004_000012_hermes_dispatch_journal;
mod m20261005_000013_runtime_controls;
mod m20261005_000014_hermes_journal_time_order;
mod m20261005_000015_runtime_control_outcomes;
mod m20261005_000016_runtime_approval_outcomes;
mod m20261006_000017_runtime_launches;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260901_000001_fleet_control::Migration),
            Box::new(m20260901_000002_session_users::Migration),
            Box::new(m20260901_000003_leaders_sessions::Migration),
            Box::new(m20260901_000004_pre_development_hardening::Migration),
            Box::new(m20260901_000005_runtime_protocol::Migration),
            Box::new(m20260914_000006_fleet_alerts::Migration),
            Box::new(m20260918_000007_central_subject::Migration),
            Box::new(m20260923_000008_managed_settings::Migration),
            Box::new(m20260930_000009_product_deployments::Migration),
            Box::new(m20261001_000009_sdlc_foundation::Migration),
            Box::new(m20261001_000010_task_chats::Migration),
            Box::new(m20261004_000011_pm_credentials::Migration),
            Box::new(m20261004_000012_hermes_dispatch_journal::Migration),
            Box::new(m20261005_000013_runtime_controls::Migration),
            Box::new(m20261005_000014_hermes_journal_time_order::Migration),
            Box::new(m20261005_000015_runtime_control_outcomes::Migration),
            Box::new(m20261005_000016_runtime_approval_outcomes::Migration),
            Box::new(m20261006_000017_runtime_launches::Migration),
        ]
    }
}
