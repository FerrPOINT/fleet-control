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
mod m20261005_000015_runtime_control_outcomes;
mod m20261005_000016_runtime_approval_outcomes;
mod m20261006_000017_runtime_launches;
mod m20261006_000018_container_preparations;
mod m20261006_000019_runtime_endpoints;
mod m20261007_000020_controller_recovery;
mod m20261007_000021_controller_recovery_delivery;
mod m20261007_000022_controller_stop_delivery;
mod m20261008_000090_execution_context;
mod m20261008_000091_context_drafts;
mod m20261009_000023_pm_execution_lease;
mod m20261009_000024_pm_workflow_assignment;
mod m20261009_000025_pm_checkpoints;
mod m20261009_000026_pm_resumes;
mod m20261010_000027_pm_native_dispatch;

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
        migrations.extend(runtime_followups());
        migrations.push(Box::new(m20261008_000090_execution_context::Migration));
        migrations.push(Box::new(m20261008_000091_context_drafts::Migration));
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
        ]);
        migrations.extend(runtime_followups());
        migrations.push(Box::new(m20261008_000090_execution_context::Migration));
        migrations.push(Box::new(m20261008_000091_context_drafts::Migration));
        migrations
    }
}

// Append the same releases after either accepted historical foundation.
fn runtime_followups() -> Vec<Box<dyn MigrationTrait>> {
    vec![
        Box::new(m20261001_000010_task_chats::Migration),
        Box::new(m20261004_000011_pm_credentials::Migration),
        Box::new(m20261004_000012_hermes_dispatch_journal::Migration),
        Box::new(m20261005_000013_runtime_controls::Migration),
        Box::new(m20261005_000014_hermes_journal_time_order::Migration),
        Box::new(m20261005_000015_runtime_control_outcomes::Migration),
        Box::new(m20261005_000016_runtime_approval_outcomes::Migration),
        Box::new(m20261006_000017_runtime_launches::Migration),
        Box::new(m20261006_000018_container_preparations::Migration),
        Box::new(m20261006_000019_runtime_endpoints::Migration),
        Box::new(m20261007_000020_controller_recovery::Migration),
        Box::new(m20261007_000021_controller_recovery_delivery::Migration),
        Box::new(m20261007_000022_controller_stop_delivery::Migration),
        Box::new(m20261009_000023_pm_execution_lease::Migration),
        Box::new(m20261009_000024_pm_workflow_assignment::Migration),
        Box::new(m20261009_000025_pm_checkpoints::Migration),
        Box::new(m20261009_000026_pm_resumes::Migration),
        Box::new(m20261010_000027_pm_native_dispatch::Migration),
    ]
}
