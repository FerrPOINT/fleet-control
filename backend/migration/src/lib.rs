pub use sea_orm_migration::prelude::*;

mod m20260901_000001_fleet_control;
mod m20260901_000002_session_users;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260901_000001_fleet_control::Migration),
            Box::new(m20260901_000002_session_users::Migration),
        ]
    }
}
