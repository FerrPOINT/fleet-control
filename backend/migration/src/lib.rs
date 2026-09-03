pub use sea_orm_migration::prelude::*;

mod m20260901_000001_fleet_control;
mod m20260901_000002_session_users;
mod m20260901_000003_leaders_sessions;
mod m20260901_000004_pre_development_hardening;
mod m20260901_000005_runtime_protocol;

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
        ]
    }
}
