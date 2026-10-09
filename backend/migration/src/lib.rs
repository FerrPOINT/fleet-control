pub use sea_orm_migration::prelude::*;

#[cfg(test)]
mod lineage_tests;
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
mod m20261001_000009_sdlc_roles;
mod m20261001_000010_session_events;
mod m20261001_000010_task_chats;
mod m20261001_000011_message_dispatch;
mod m20261001_000012_config_revisions;
mod m20261004_000011_pm_credentials;
mod m20261004_000012_hermes_dispatch_journal;
mod m20261005_000013_runtime_controls;
mod m20261005_000014_hermes_journal_time_order;
mod m20261009_000015_container_controller;
mod m20261009_000016_mapped_controller_recovery;
mod m20261009_000017_container_preparation;
mod m20261009_000018_container_activation;
mod m20261009_000019_recovered_activation;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        CanonicalMigrator::migrations()
    }

    async fn get_migration_with_status<C>(
        db: &C,
    ) -> Result<Vec<sea_orm_migration::Migration>, DbErr>
    where
        C: ConnectionTrait,
    {
        let applied = Self::get_migration_models(db).await?;
        let split = applied
            .iter()
            .any(|row| SPLIT_VERSIONS.contains(&row.version.as_str()));
        let combined = applied.iter().any(|row| row.version == COMBINED_VERSION);
        if split && combined {
            return Err(DbErr::Custom(
                "Fleet migration history mixes split SDLC and combined foundation versions; operator review is required".into(),
            ));
        }
        // Preserve the original ledger and delegate unknown-version validation to SeaORM.
        if split {
            LegacyMigrator::get_migration_with_status(db).await
        } else {
            CanonicalMigrator::get_migration_with_status(db).await
        }
    }
}

const COMBINED_VERSION: &str = "m20261001_000009_sdlc_foundation";
const SPLIT_VERSIONS: [&str; 4] = [
    "m20261001_000009_sdlc_roles",
    "m20261001_000010_session_events",
    "m20261001_000011_message_dispatch",
    "m20261001_000012_config_revisions",
];

fn common_migrations() -> Vec<Box<dyn MigrationTrait>> {
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
    ]
}

struct CanonicalMigrator;

#[async_trait::async_trait]
impl MigratorTrait for CanonicalMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let mut migrations = common_migrations();
        migrations.push(Box::new(m20261001_000009_sdlc_foundation::Migration));
        migrations.push(Box::new(m20261001_000010_task_chats::Migration));
        migrations.push(Box::new(m20261004_000011_pm_credentials::Migration));
        migrations.push(Box::new(
            m20261004_000012_hermes_dispatch_journal::Migration,
        ));
        migrations.push(Box::new(m20261005_000013_runtime_controls::Migration));
        migrations.push(Box::new(
            m20261005_000014_hermes_journal_time_order::Migration,
        ));
        migrations.push(Box::new(m20261009_000015_container_controller::Migration));
        migrations.push(Box::new(
            m20261009_000016_mapped_controller_recovery::Migration,
        ));
        migrations.push(Box::new(m20261009_000017_container_preparation::Migration));
        migrations.push(Box::new(m20261009_000018_container_activation::Migration));
        migrations.push(Box::new(m20261009_000019_recovered_activation::Migration));
        migrations
    }
}

struct LegacyMigrator;

#[async_trait::async_trait]
impl MigratorTrait for LegacyMigrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let mut migrations = common_migrations();
        migrations.extend([
            Box::new(m20261001_000009_sdlc_roles::Migration) as Box<dyn MigrationTrait>,
            Box::new(m20261001_000010_session_events::Migration),
            Box::new(m20261001_000011_message_dispatch::Migration),
            Box::new(m20261001_000012_config_revisions::Migration),
            Box::new(m20261001_000010_task_chats::Migration),
            Box::new(m20261004_000011_pm_credentials::Migration),
            Box::new(m20261004_000012_hermes_dispatch_journal::Migration),
            Box::new(m20261005_000013_runtime_controls::Migration),
            Box::new(m20261005_000014_hermes_journal_time_order::Migration),
            Box::new(m20261009_000015_container_controller::Migration),
            Box::new(m20261009_000016_mapped_controller_recovery::Migration),
            Box::new(m20261009_000017_container_preparation::Migration),
            Box::new(m20261009_000018_container_activation::Migration),
            Box::new(m20261009_000019_recovered_activation::Migration),
        ]);
        migrations
    }
}
