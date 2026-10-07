use super::{
    LaunchPhase, LocalRuntimeSupervisor, activation_journal, activation_lock, container_control,
    unconfirmed_runtime,
};
use domain::{Agent, AgentKind, AgentStatus};
use shared::AppError;
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn held() -> AppError {
    AppError::Unavailable(
        "interrupted configuration requires original runtime reconciliation".into(),
    )
}

fn snapshot_hash(revision: &domain::AgentConfigRevision) -> Result<String, AppError> {
    crate::runtime_launches::snapshot_hash(
        &serde_json::to_value(&revision.snapshot).map_err(|_| held())?,
    )
}

impl LocalRuntimeSupervisor {
    pub(super) async fn recover_pending_configurations(&self) {
        if !self.config.fleet.configuration_recovery_enabled {
            return;
        }
        let Ok(agents) = self.repo.list_agents().await else {
            return;
        };
        for agent in agents {
            if agent.kind != AgentKind::Hermes || agent.status == AgentStatus::Archived {
                continue;
            }
            let path = activation_journal::controller_journal_path(
                Path::new(&self.config.fleet.controller_root),
                agent.id,
            );
            if matches!(tokio::fs::symlink_metadata(&path).await, Err(error) if error.kind() == std::io::ErrorKind::NotFound)
            {
                continue;
            }
            if let Err(error) = self.recover_config_activation(agent.id).await {
                tracing::debug!(agent_id=%agent.id, "configuration recovery remains held: {error}");
            }
        }
    }

    pub(super) async fn recover_config_activation(&self, agent_id: Uuid) -> Result<bool, AppError> {
        if !self.config.fleet.configuration_recovery_enabled {
            return Err(held());
        }
        let lifecycle = self.lifecycle_lock(agent_id).await;
        let _guard = lifecycle.lock().await;
        let lock = activation_lock::ActivationLock::acquire(
            Path::new(&self.config.fleet.controller_root),
            agent_id,
        )
        .await?;
        let agent = self.repo.get_agent(agent_id).await?;
        if agent.kind != AgentKind::Hermes || agent.status == AgentStatus::Archived {
            return Err(held());
        }
        let Some(journal) = activation_journal::ActivationJournal::reopen(
            activation_journal::JournalLocation {
                agents_root: Path::new(&self.config.fleet.agents_root),
                config_directory: Path::new(&agent.paths.config),
                controller_root: Path::new(&self.config.fleet.controller_root),
            },
            agent_id,
            &self.config.fleet.runtime_token_secret,
            lock,
        )
        .await?
        else {
            return Ok(false);
        };
        let identity = journal.identity();
        let revision = self
            .repo
            .get_config_revision(agent_id, identity.revision)
            .await?;
        if !revision.is_desired || snapshot_hash(&revision)? != identity.candidate_sha256 {
            return Err(held());
        }
        let plan = crate::configuration_files(&agent, &self.config, &revision).await?;
        journal.verify_plan(&plan)?;
        if revision.state == "active" && revision.is_effective && !revision.draining {
            journal.verify_candidate_files().await?;
            self.verify_recovered_configuration_runtime(&agent, &journal, true)
                .await?;
            journal.acknowledge().await?;
            return Ok(true);
        }
        let effective = self.repo.get_effective_config_revision(agent_id).await?;
        if effective.as_ref().map(|revision| revision.revision) != identity.effective_revision
            || effective.as_ref().map(snapshot_hash).transpose()? != identity.effective_sha256
            || !matches!(revision.state.as_str(), "activating" | "failed")
        {
            return Err(held());
        }
        let backups = journal.verified_backups().await?;
        if !revision.draining {
            if revision.state != "failed" {
                return Err(held());
            }
            verify_restored(&self.config.fleet.agents_root, &backups).await?;
            self.verify_recovered_configuration_runtime(&agent, &journal, false)
                .await?;
            journal.acknowledge().await?;
            return Ok(true);
        }
        self.repo
            .verify_configuration_rollback(&journal.rollback_claim())
            .await?;
        if self
            .repo
            .has_pending_container_preparation(agent_id)
            .await?
        {
            return Err(held());
        }
        if let Some(open) = self.repo.get_open_runtime_launch(agent_id).await? {
            let original = identity.original_launch.as_ref().is_some_and(|original| {
                serde_json::to_value(original).ok() == serde_json::to_value(&open.binding).ok()
            });
            let replacement = open.binding.agent_id == agent_id
                && serde_json::to_value(&open.binding.paths).map_err(|_| held())?
                    == serde_json::to_value(&agent.paths).map_err(|_| held())?
                && open.binding.api_port == agent.api_port
                && ((open.binding.phase == "activation"
                    && open.binding.configuration_revision == Some(identity.revision)
                    && open.binding.configuration_sha256.as_ref()
                        == Some(&identity.candidate_sha256))
                    || (open.binding.phase == "rollback"
                        && open.binding.configuration_revision == identity.effective_revision
                        && open.binding.configuration_sha256 == identity.effective_sha256));
            if !journal.was_running()
                || open.binding.container.is_none()
                || !(original || replacement)
            {
                return Err(held());
            }
            journal.verify().await?;
            self.stop_locked(&agent).await?;
        }
        let stopped = self.repo.get_agent(agent_id).await?;
        if unconfirmed_runtime(&stopped)
            || self.repo.get_open_runtime_launch(agent_id).await?.is_some()
        {
            return Err(held());
        }
        if self.config.fleet.container_control.is_some() {
            self.verify_container_configuration_quiescent(&stopped)
                .await?;
        }
        if journal.was_running() {
            self.verify_original_configuration_exit(&journal).await?;
        }
        // The original namespace is confirmed empty before restoring any backup bytes.
        for (path, previous) in &backups {
            journal.verify_lock().await?;
            crate::reject_symlink_components(Path::new(&self.config.fleet.agents_root), path)
                .await?;
            match previous {
                Some(bytes) => {
                    crate::configuration_disk::create_directory(
                        Path::new(&self.config.fleet.agents_root),
                        path.parent().ok_or_else(held)?,
                    )
                    .await?;
                    crate::write_configuration_file(path, bytes).await?;
                }
                None => {
                    crate::configuration_disk::remove(
                        Path::new(&self.config.fleet.agents_root),
                        path,
                    )
                    .await?
                }
            }
        }
        verify_restored(&self.config.fleet.agents_root, &backups).await?;
        if journal.was_running() {
            journal.verify().await?;
            if self
                .start_locked(&stopped, LaunchPhase::Rollback)
                .await?
                .status
                != AgentStatus::Running
            {
                return Err(held());
            }
        }
        journal.verify().await?;
        self.repo
            .settle_configuration_rollback(&journal.rollback_claim())
            .await?;
        journal.acknowledge().await?;
        Ok(true)
    }

    async fn verify_original_configuration_exit(
        &self,
        journal: &activation_journal::ActivationJournal,
    ) -> Result<(), AppError> {
        let original = journal
            .identity()
            .original_launch
            .as_ref()
            .ok_or_else(held)?;
        let saved = self
            .repo
            .get_runtime_launch(original.id)
            .await?
            .ok_or_else(held)?;
        if saved.state != "gateway_exited"
            || saved.pid.is_none()
            || serde_json::to_value(&saved.binding).map_err(|_| held())?
                != serde_json::to_value(original).map_err(|_| held())?
        {
            return Err(held());
        }
        let binding = original.container.as_ref().ok_or_else(held)?;
        let observation = if original.controller_id == self.controller_id {
            self.observe_container(&saved).await?
        } else {
            self.container_control(binding)?
                .observe_controller_restart(
                    &self.container_files(binding).await?,
                    &binding.registration,
                )
                .await?
                .receipt
        };
        if observation.observation != container_control::ContainerObservation::NamespaceExited {
            return Err(held());
        }
        Ok(())
    }

    async fn verify_recovered_configuration_runtime(
        &self,
        agent: &Agent,
        journal: &activation_journal::ActivationJournal,
        candidate: bool,
    ) -> Result<(), AppError> {
        if !journal.was_running() {
            if unconfirmed_runtime(agent)
                || self.repo.get_open_runtime_launch(agent.id).await?.is_some()
                || self
                    .repo
                    .has_pending_container_preparation(agent.id)
                    .await?
            {
                return Err(held());
            }
            return Ok(());
        }
        let identity = journal.identity();
        let current = self
            .repo
            .get_open_runtime_launch(agent.id)
            .await?
            .ok_or_else(held)?;
        let expected_revision = if candidate {
            Some(identity.revision)
        } else {
            identity.effective_revision
        };
        let expected_hash = if candidate {
            Some(identity.candidate_sha256.clone())
        } else {
            identity.effective_sha256.clone()
        };
        if current.binding.configuration_revision != expected_revision
            || current.binding.configuration_sha256 != expected_hash
            || current.binding.phase != if candidate { "activation" } else { "rollback" }
            || self.health_locked(agent).await?.status != AgentStatus::Running
        {
            return Err(held());
        }
        Ok(())
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use app::FleetRepository;
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    use std::sync::Arc;

    async fn interrupted() -> Option<(
        Arc<crate::PostgresFleetRepository>,
        Agent,
        Arc<shared::AppConfig>,
        PathBuf,
    )> {
        interrupted_with_effective(false).await
    }

    async fn interrupted_with_effective(
        with_effective: bool,
    ) -> Option<(
        Arc<crate::PostgresFleetRepository>,
        Agent,
        Arc<shared::AppConfig>,
        PathBuf,
    )> {
        let (repo, agent, owner, config, root) =
            super::super::lifecycle_tests::fixture(AgentKind::Hermes).await?;
        let mut config = (*config).clone();
        config.fleet.configuration_recovery_enabled = true;
        let config = Arc::new(config);
        let soul = Path::new(&agent.paths.config).join("SOUL.md");
        tokio::fs::write(&soul, b"last-working-configuration")
            .await
            .unwrap();
        if with_effective {
            let previous = repo
                .create_config_revision(
                    agent.id,
                    domain::UpdateAgentConfigRequest {
                        config_json: serde_json::json!({"model":"previous-model"}),
                        soul_md: "last-working-configuration".into(),
                        env_json: serde_json::json!({}),
                    },
                    owner,
                )
                .await
                .unwrap();
            repo.validate_config_revision(agent.id, previous.revision, vec![])
                .await
                .unwrap();
            repo.request_config_activation(agent.id, previous.revision, owner)
                .await
                .unwrap();
            let previous = repo.claim_config_activation().await.unwrap().unwrap();
            let runtime = super::super::lifecycle_tests::supervisor(config.clone(), repo.clone());
            let mut journal = None;
            runtime
                .apply_config_revision(&previous, &mut journal)
                .await
                .unwrap();
            repo.finish_config_activation(agent.id, previous.revision, None, true)
                .await
                .unwrap();
            journal.unwrap().acknowledge().await.unwrap();
        }
        let revision = super::super::lifecycle_tests::revision(&repo, &agent, owner).await;
        repo.request_config_activation(agent.id, revision.revision, owner)
            .await
            .unwrap();
        let claimed = repo.claim_config_activation().await.unwrap().unwrap();
        let original = super::super::lifecycle_tests::supervisor(config.clone(), repo.clone());
        let mut journal = None;
        original
            .apply_config_revision(&claimed, &mut journal)
            .await
            .unwrap();
        assert!(journal.is_some());
        drop(journal);
        Some((repo, agent, config, root))
    }

    #[tokio::test]
    async fn interrupted_stopped_activation_restores_files_and_settles_once() {
        let Some((repo, agent, config, root)) = interrupted().await else {
            return;
        };
        let restarted = super::super::lifecycle_tests::supervisor(config.clone(), repo.clone());
        assert!(restarted.recover_config_activation(agent.id).await.unwrap());
        assert!(!repo.agent_is_draining(agent.id).await.unwrap());
        assert!(
            repo.get_effective_config_revision(agent.id)
                .await
                .unwrap()
                .is_none()
        );
        assert_eq!(
            repo.list_config_revisions(agent.id).await.unwrap()[0].state,
            "failed"
        );
        assert_eq!(
            tokio::fs::read(Path::new(&agent.paths.config).join("SOUL.md"))
                .await
                .unwrap(),
            b"last-working-configuration"
        );
        assert!(!Path::new(&agent.paths.config).join(".env").exists());
        assert!(
            !activation_journal::controller_journal_path(
                Path::new(&config.fleet.controller_root),
                agent.id
            )
            .exists()
        );
        assert!(!restarted.recover_config_activation(agent.id).await.unwrap());
        let audit = repo.db.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "SELECT count(*) AS count FROM audit_log WHERE entity_id=$1 AND action='agent_config.recovery_rollback'",
            [agent.id.to_string().into()])).await.unwrap().unwrap();
        assert_eq!(audit.try_get::<i64>("", "count").unwrap(), 1);
        assert!(restarted.children.lock().await.is_empty());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn committed_candidate_retires_journal_without_reapplying_or_rollback() {
        let Some((repo, agent, config, root)) = interrupted().await else {
            return;
        };
        let revision = repo.list_config_revisions(agent.id).await.unwrap()[0].revision;
        repo.finish_config_activation(agent.id, revision, None, true)
            .await
            .unwrap();
        let restarted = super::super::lifecycle_tests::supervisor(config.clone(), repo.clone());
        assert!(restarted.recover_config_activation(agent.id).await.unwrap());
        assert_eq!(
            repo.get_effective_config_revision(agent.id)
                .await
                .unwrap()
                .unwrap()
                .revision,
            revision
        );
        assert_eq!(
            tokio::fs::read(Path::new(&agent.paths.config).join("SOUL.md"))
                .await
                .unwrap(),
            b"Updated lifecycle SOUL"
        );
        assert!(!repo.agent_is_draining(agent.id).await.unwrap());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn rollback_preserves_existing_effective_snapshot_and_files() {
        let Some((repo, agent, config, root)) = interrupted_with_effective(true).await else {
            return;
        };
        let effective = repo
            .get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .unwrap();
        let original_hash = snapshot_hash(&effective).unwrap();
        let restarted = super::super::lifecycle_tests::supervisor(config, repo.clone());
        assert!(restarted.recover_config_activation(agent.id).await.unwrap());
        let current = repo
            .get_effective_config_revision(agent.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(current.revision, effective.revision);
        assert_eq!(snapshot_hash(&current).unwrap(), original_hash);
        assert_eq!(current.state, "active");
        let desired = repo.list_config_revisions(agent.id).await.unwrap();
        assert!(desired.iter().any(|revision| revision.is_desired
            && revision.state == "failed"
            && !revision.is_effective
            && !revision.draining));
        assert_eq!(
            tokio::fs::read(Path::new(&agent.paths.config).join("SOUL.md"))
                .await
                .unwrap(),
            b"last-working-configuration"
        );
        let yaml = tokio::fs::read_to_string(Path::new(&agent.paths.config).join("config.yaml"))
            .await
            .unwrap();
        assert!(yaml.contains("previous-model"));
        assert!(restarted.children.lock().await.is_empty());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn released_drain_prevents_backup_restoration_before_database_preflight() {
        let Some((repo, agent, config, root)) = interrupted().await else {
            return;
        };
        let path = activation_journal::controller_journal_path(
            Path::new(&config.fleet.controller_root),
            agent.id,
        );
        let original = tokio::fs::read(&path).await.unwrap();
        repo.db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE agent_config_heads SET draining=false WHERE agent_id=$1",
                [agent.id.into()],
            ))
            .await
            .unwrap();
        let restarted = super::super::lifecycle_tests::supervisor(config, repo.clone());
        assert!(restarted.recover_config_activation(agent.id).await.is_err());
        assert_eq!(tokio::fs::read(path).await.unwrap(), original);
        assert_eq!(
            tokio::fs::read(Path::new(&agent.paths.config).join("SOUL.md"))
                .await
                .unwrap(),
            b"Updated lifecycle SOUL"
        );
        assert_eq!(
            repo.list_config_revisions(agent.id).await.unwrap()[0].state,
            "activating"
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn failed_settlement_retains_journal_and_replays_original_rollback_once() {
        let Some((repo, agent, config, root)) = interrupted().await else {
            return;
        };
        let path = activation_journal::controller_journal_path(
            Path::new(&config.fleet.controller_root),
            agent.id,
        );
        let original = tokio::fs::read(&path).await.unwrap();
        let suffix = Uuid::new_v4().simple().to_string();
        let function = format!("fleet_recovery_failure_{suffix}");
        repo.db.execute(Statement::from_string(DatabaseBackend::Postgres, format!(
            "CREATE FUNCTION {function}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
             IF NEW.action='agent_config.recovery_rollback' THEN RAISE EXCEPTION 'fixture recovery settlement rejected'; END IF;
             RETURN NEW; END $$"))).await.unwrap();
        repo.db.execute(Statement::from_string(DatabaseBackend::Postgres, format!(
            "CREATE TRIGGER {function} BEFORE INSERT ON audit_log FOR EACH ROW EXECUTE FUNCTION {function}()"))).await.unwrap();
        let restarted = super::super::lifecycle_tests::supervisor(config, repo.clone());
        assert!(restarted.recover_config_activation(agent.id).await.is_err());
        assert!(repo.agent_is_draining(agent.id).await.unwrap());
        assert_eq!(
            repo.list_config_revisions(agent.id).await.unwrap()[0].state,
            "activating"
        );
        assert_eq!(tokio::fs::read(&path).await.unwrap(), original);
        assert_eq!(
            tokio::fs::read(Path::new(&agent.paths.config).join("SOUL.md"))
                .await
                .unwrap(),
            b"last-working-configuration"
        );
        repo.db
            .execute(Statement::from_string(
                DatabaseBackend::Postgres,
                format!("DROP TRIGGER {function} ON audit_log"),
            ))
            .await
            .unwrap();
        repo.db
            .execute(Statement::from_string(
                DatabaseBackend::Postgres,
                format!("DROP FUNCTION {function}()"),
            ))
            .await
            .unwrap();
        assert!(restarted.recover_config_activation(agent.id).await.unwrap());
        assert!(!repo.agent_is_draining(agent.id).await.unwrap());
        assert!(!path.exists());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn changed_candidate_snapshot_prevents_recovery_effects() {
        let Some((repo, agent, config, root)) = interrupted().await else {
            return;
        };
        let path = activation_journal::controller_journal_path(
            Path::new(&config.fleet.controller_root),
            agent.id,
        );
        let original = tokio::fs::read(&path).await.unwrap();
        repo.db.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres,
            "UPDATE agent_config_revisions SET snapshot=jsonb_set(snapshot,'{config,soul_md}','\"concurrent-snapshot\"'::jsonb) WHERE agent_id=$1",
            [agent.id.into()])).await.unwrap();
        let restarted = super::super::lifecycle_tests::supervisor(config, repo.clone());
        assert!(restarted.recover_config_activation(agent.id).await.is_err());
        assert!(repo.agent_is_draining(agent.id).await.unwrap());
        assert_eq!(tokio::fs::read(&path).await.unwrap(), original);
        assert_eq!(
            tokio::fs::read(Path::new(&agent.paths.config).join("SOUL.md"))
                .await
                .unwrap(),
            b"Updated lifecycle SOUL"
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn uncertain_runtime_rotated_key_and_disabled_recovery_preserve_candidate_and_drain() {
        let Some((repo, agent, config, root)) = interrupted().await else {
            return;
        };
        let path = activation_journal::controller_journal_path(
            Path::new(&config.fleet.controller_root),
            agent.id,
        );
        let journal = tokio::fs::read(&path).await.unwrap();
        for disabled in [true, false] {
            let mut invalid = (*config).clone();
            if disabled {
                invalid.fleet.configuration_recovery_enabled = false;
            } else {
                invalid.fleet.runtime_token_secret = "rotated-unverified-key".into();
            }
            let restarted =
                super::super::lifecycle_tests::supervisor(Arc::new(invalid), repo.clone());
            assert!(restarted.recover_config_activation(agent.id).await.is_err());
        }
        repo.db
            .execute(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                "UPDATE agent_runtime SET pid=4242,desired_state='running' WHERE agent_id=$1",
                [agent.id.into()],
            ))
            .await
            .unwrap();
        let restarted = super::super::lifecycle_tests::supervisor(config, repo.clone());
        assert!(restarted.recover_config_activation(agent.id).await.is_err());
        assert_eq!(tokio::fs::read(&path).await.unwrap(), journal);
        assert_eq!(
            tokio::fs::read(Path::new(&agent.paths.config).join("SOUL.md"))
                .await
                .unwrap(),
            b"Updated lifecycle SOUL"
        );
        assert!(repo.agent_is_draining(agent.id).await.unwrap());
        assert!(restarted.children.lock().await.is_empty());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}

async fn verify_restored(
    root: &str,
    backups: &[(PathBuf, Option<Vec<u8>>)],
) -> Result<(), AppError> {
    for (path, expected) in backups {
        if activation_journal::read_backup(Path::new(root), path)
            .await?
            .as_ref()
            != expected.as_ref()
        {
            return Err(held());
        }
    }
    Ok(())
}
