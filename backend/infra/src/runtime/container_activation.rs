//! Live-custodian replacement using original Base commands, never a host controller.
use super::*;
use app::container_activation::{Activation, Claim, Generation, Phase, Readiness, held};
use app::container_runtime::{ContainerBinding, ContainerLaunch, PreparedContainer};
use container_control::{
    ContainerControl, ContainerLaunchFiles, ContainerObservation, ContainerReceiptState,
    canonical_hash,
};
use container_lifecycle::{private_file, private_root, validate_recipe};
use container_preparation::{Intent, mapped_policy};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tokio::io::AsyncWriteExt;

type Files = BTreeMap<String, Option<Vec<u8>>>;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    id: Uuid,
    controller_id: Uuid,
    revision: i64,
    previous: ContainerLaunch,
    candidate: Intent,
    candidate_stop_id: Uuid,
    rollback: Intent,
    rollback_stop_id: Uuid,
    files: Files,
    previous_files: Files,
    marker_sha256: String,
}

impl Plan {
    fn claim(&self) -> Result<Claim, AppError> {
        Ok(Claim {
            id: self.id,
            agent_id: self.previous.prepared.agent_id,
            controller_id: self.controller_id,
            revision: self.revision,
            previous_revision: self.previous.prepared.configuration_revision,
            configuration_sha256: self
                .candidate
                .configuration_sha256
                .clone()
                .ok_or_else(held)?,
            previous_configuration_sha256: self.previous.prepared.configuration_sha256.clone(),
            files_sha256: canonical_hash(&self.files)?,
            previous_files_sha256: canonical_hash(&self.previous_files)?,
            intent_sha256: canonical_hash(self)?,
            candidate_intent_sha256: canonical_hash(&self.candidate)?,
            rollback_intent_sha256: canonical_hash(&self.rollback)?,
            previous: self.previous.clone(),
            candidate: Generation {
                generation: self.candidate.generation,
                operation_id: self.candidate.operation_id,
                stop_id: self.candidate_stop_id,
            },
            rollback: Generation {
                generation: self.rollback.generation,
                operation_id: self.rollback.operation_id,
                stop_id: self.rollback_stop_id,
            },
        })
    }
}

// A permanent inode prevents lock-file replacement from splitting native custody.
struct AgentLock {
    _file: std::fs::File,
}

impl AgentLock {
    #[cfg(target_os = "linux")]
    async fn acquire(root: &Path, agent: Uuid) -> Result<Self, AppError> {
        use std::os::{
            fd::AsRawFd,
            unix::fs::{MetadataExt, OpenOptionsExt},
        };
        let path = root.join(format!("{agent}.activation.lock"));
        crate::reject_symlink_components(root, &path)
            .await
            .map_err(|_| held())?;
        let file = match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(f) => {
                f.sync_all().map_err(|_| held())?;
                std::fs::File::open(root)
                    .and_then(|f| f.sync_all())
                    .map_err(|_| held())?;
                f
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .map_err(|_| held())?,
            Err(_) => return Err(held()),
        };
        let before = std::fs::symlink_metadata(&path).map_err(|_| held())?;
        let opened = file.metadata().map_err(|_| held())?;
        if !before.is_file()
            || !opened.is_file()
            || opened.nlink() != 1
            || opened.mode() & 0o777 != 0o600
            || opened.uid() != std::fs::metadata(root).map_err(|_| held())?.uid()
            || (before.dev(), before.ino()) != (opened.dev(), opened.ino())
        {
            return Err(held());
        }
        unsafe extern "C" {
            fn flock(fd: std::ffi::c_int, operation: std::ffi::c_int) -> std::ffi::c_int;
        }
        // LOCK_EX | LOCK_NB. The owned descriptor releases the lock on all returns/cancellation.
        if unsafe { flock(file.as_raw_fd(), 2 | 4) } != 0 {
            return Err(held());
        }
        let after = std::fs::symlink_metadata(path).map_err(|_| held())?;
        if (after.dev(), after.ino()) != (opened.dev(), opened.ino()) {
            return Err(held());
        }
        Ok(Self { _file: file })
    }

    #[cfg(not(target_os = "linux"))]
    async fn acquire(_root: &Path, _agent: Uuid) -> Result<Self, AppError> {
        Err(held())
    }
}

async fn write_once<T: Serialize>(root: &Path, path: &Path, value: &T) -> Result<(), AppError> {
    crate::reject_symlink_components(root, path)
        .await
        .map_err(|_| held())?;
    let bytes = serde_json::to_vec(value).map_err(|_| held())?;
    if path.parent() != Some(root) || bytes.len() > 1_048_576 {
        return Err(held());
    }
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(path).await.map_err(|_| held())?;
    file.write_all(&bytes).await.map_err(|_| held())?;
    file.sync_all().await.map_err(|_| held())?;
    tokio::fs::File::open(root)
        .await
        .map_err(|_| held())?
        .sync_all()
        .await
        .map_err(|_| held())
}

fn recipe_path(root: &Path, intent: &Intent) -> PathBuf {
    root.join(format!(
        "{}.{}.activation-recipe.json",
        intent.agent_id, intent.generation
    ))
}

async fn read_managed(path: &Path) -> Result<Option<Vec<u8>>, AppError> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(_) => Ok(Some(
            private_file(path.parent().ok_or_else(held)?, path, 262_144).await?,
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(held()),
    }
}

impl LocalRuntimeSupervisor {
    async fn activation_marker(&self, agent: &Agent) -> Result<String, AppError> {
        let agents = Path::new(&self.config.fleet.agents_root);
        crate::reject_symlink_components(Path::new("/"), agents)
            .await
            .map_err(|_| held())?;
        let root = crate::safe_agent_root(agents, &agent.name).map_err(|_| held())?;
        if agent.name != format!("agent{}", agent.ordinal)
            || agent.ordinal < 1
            || crate::inspect_agent_marker(&root, agent)
                .await
                .map_err(|_| held())?
                != (true, true)
        {
            return Err(held());
        }
        for (area, path) in [
            ("runtime", &agent.paths.runtime),
            ("config", &agent.paths.config),
            ("workspace", &agent.paths.workspace),
            ("logs", &agent.paths.logs),
        ] {
            if Path::new(path) != root.join(area) {
                return Err(held());
            }
            crate::reject_symlink_components(agents, Path::new(path))
                .await
                .map_err(|_| held())?;
            if !tokio::fs::symlink_metadata(path)
                .await
                .map_err(|_| held())?
                .is_dir()
            {
                return Err(held());
            }
        }
        canonical_hash(&private_file(&root, &root.join(".fleet-agent.json"), 16_384).await?)
    }

    async fn activation_files(
        &self,
        agent: &Agent,
        plan: &Plan,
        rollback: bool,
        apply: bool,
    ) -> Result<(), AppError> {
        if self.activation_marker(agent).await? != plan.marker_sha256 {
            return Err(held());
        }
        let files = if rollback {
            &plan.previous_files
        } else {
            &plan.files
        };
        let config = Path::new(&agent.paths.config);
        for (name, expected) in files {
            let path = Path::new(name);
            if !path.starts_with(config)
                || path == config
                || crate::normalize_path(path).map_err(|_| held())? != path
            {
                return Err(held());
            }
            crate::reject_symlink_components(config, path)
                .await
                .map_err(|_| held())?;
            let actual = read_managed(path).await?;
            if apply && actual != *expected {
                match expected {
                    Some(body) => {
                        let parent = path.parent().ok_or_else(held)?;
                        let mut dirs = tokio::fs::DirBuilder::new();
                        dirs.recursive(true);
                        #[cfg(unix)]
                        dirs.mode(0o700);
                        dirs.create(parent).await.map_err(|_| held())?;
                        crate::reject_symlink_components(config, path)
                            .await
                            .map_err(|_| held())?;
                        crate::write_configuration_file(path, body)
                            .await
                            .map_err(|_| held())?;
                    }
                    None if actual.is_some() => {
                        tokio::fs::remove_file(path).await.map_err(|_| held())?;
                    }
                    None => {}
                }
                let mut parent = path.parent();
                while let Some(directory) = parent {
                    if !directory.starts_with(config) {
                        break;
                    }
                    tokio::fs::File::open(directory)
                        .await
                        .map_err(|_| held())?
                        .sync_all()
                        .await
                        .map_err(|_| held())?;
                    if directory == config {
                        break;
                    }
                    parent = directory.parent();
                }
            }
            if read_managed(path).await? != *expected {
                return Err(held());
            }
        }
        if self.activation_marker(agent).await? != plan.marker_sha256 {
            return Err(held());
        }
        Ok(())
    }

    async fn activation_fork(
        &self,
        original: &Intent,
        revision: Option<i64>,
        hash: Option<String>,
        root: &Path,
        control: &ContainerControl,
    ) -> Result<Intent, AppError> {
        let mut intent = original.clone();
        intent.generation = Uuid::new_v4();
        intent.operation_id = Uuid::new_v4();
        intent.configuration_revision = revision;
        intent.configuration_sha256 = hash;
        let project = intent.local_policy["project"]
            .as_str()
            .ok_or_else(held)?
            .to_owned();
        let service = format!(
            "agent{}-runtime-{}",
            intent.local_policy["mounts"][0]["source"]
                .as_str()
                .and_then(|s| Path::new(s).parent())
                .and_then(Path::file_name)
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix("agent"))
                .ok_or_else(held)?,
            intent.generation.simple()
        );
        intent.local_policy["generation"] = json!(intent.generation);
        intent.local_policy["service"] = json!(service);
        intent.local_policy["network"]["id"] = json!("0".repeat(64));
        intent.local_policy["network"]["name"] = json!(format!("{project}-{service}"));
        intent.policy = intent.local_policy.clone();
        intent.mapped = None;
        if let Some(old) = &original.mapped {
            let trusted = self
                .config
                .fleet
                .container_control
                .as_ref()
                .and_then(|c| c.mapping_controller.as_ref())
                .ok_or_else(held)?;
            let mapping = control
                .resolve_mounts(&intent.files(root), trusted, &self.config.fleet.agents_root)
                .await?;
            if mapping.controller != old.mapping.controller
                || mapping.snapshot != old.mapping.snapshot
                || mapping.engine != old.mapping.engine
                || mapping.volume_name != old.mapping.volume_name
                || mapping.volume_sha256 != old.mapping.volume_sha256
                || mapping.mounts != old.mapping.mounts
            {
                return Err(held());
            }
            let name = format!("{}.{}", intent.agent_id, intent.generation);
            let mapped = app::container_runtime::MappedContainer {
                mapping,
                mapping_file: root
                    .join(format!("{name}.mapping.json"))
                    .to_string_lossy()
                    .into_owned(),
                attachment_journal: root
                    .join(format!("{name}.attach.sqlite"))
                    .to_string_lossy()
                    .into_owned(),
                recovery_journal: root
                    .join(format!("{name}.recovery.sqlite"))
                    .to_string_lossy()
                    .into_owned(),
            };
            intent.policy = mapped_policy(
                &intent.local_policy,
                &mapped,
                trusted,
                &self.config.fleet.agents_root,
            )?;
            intent.mapped = Some(mapped);
        }
        Ok(intent)
    }

    async fn activation_plan(
        &self,
        agent: &Agent,
        revision: &domain::AgentConfigRevision,
        root: &Path,
    ) -> Result<Plan, AppError> {
        let previous = self
            .repo
            .get_container_launch(agent.id)
            .await?
            .ok_or_else(held)?;
        if previous.controller_id != self.controller_id
            || previous.state != "running"
            || self
                .repo
                .get_container_recovery(previous.prepared.container.registration.generation)
                .await?
                .is_some()
            || revision.state != "activating"
            || !revision.is_desired
            || !revision.draining
            || !revision.validation_errors.is_empty()
        {
            return Err(held());
        }
        self.checked_prepared(agent, &previous.prepared).await?;
        let (control, _) = self.container_files(&previous.prepared.container).await?;
        let mut path = root.join(format!("{}.container-intent.json", agent.id));
        let initial = self
            .repo
            .get_container_preparation(agent.id)
            .await?
            .ok_or_else(held)?;
        if initial.claim.generation != previous.prepared.container.registration.generation {
            path = root.join(format!(
                "{}.{}.activation-recipe.json",
                agent.id, previous.prepared.container.registration.generation
            ));
        }
        let original: Intent = serde_json::from_slice(&private_file(root, &path, 65_536).await?)
            .map_err(|_| held())?;
        if self
            .repo
            .container_generation_intent_hash(agent.id, original.generation)
            .await?
            .as_ref()
            != Some(&canonical_hash(&original)?)
            || original.generation != previous.prepared.container.registration.generation
            || original.operation_id != previous.prepared.container.registration.operation_id
            || original.agent_id != agent.id
            || original.configuration_revision != previous.prepared.configuration_revision
            || original.configuration_sha256 != previous.prepared.configuration_sha256
            || original.local_policy["image_id"] != previous.prepared.container.policy["image_id"]
        {
            return Err(held());
        }
        let effective = self.repo.get_container_configuration(agent.id).await?;
        if let Some(mut effective) = effective.clone() {
            effective.draining = false;
            crate::effective_configuration::verify(agent, &self.config, &effective)
                .await
                .map_err(|_| held())?;
        } else {
            for name in [".env", "config.yaml", "SOUL.md"] {
                use sha2::Digest;
                let body = private_file(
                    Path::new(&agent.paths.config),
                    &Path::new(&agent.paths.config).join(name),
                    65_536,
                )
                .await?;
                if original.files_sha256.get(name)
                    != Some(&hex::encode(sha2::Sha256::digest(&body)))
                {
                    return Err(held());
                }
            }
        }
        self.activation_running(agent, &previous).await?;
        self.probe_hermes_at(
            previous.origin.as_deref().ok_or_else(held)?,
            &crate::agent_runtime_token(&self.config, agent.id)?,
        )
        .await
        .map_err(|_| held())?;
        let target = crate::configuration_files(agent, &self.config, revision)
            .await
            .map_err(|_| held())?;
        let mut next_files = Files::new();
        if let Some(effective) = &effective {
            for (path, _) in crate::configuration_files(agent, &self.config, effective)
                .await
                .map_err(|_| held())?
            {
                next_files.insert(path.to_string_lossy().into_owned(), None);
            }
        }
        for (path, body) in target {
            let absent = body.is_empty() && path.file_name().is_some_and(|s| s == "SKILL.md");
            next_files.insert(
                path.to_string_lossy().into_owned(),
                if absent {
                    None
                } else {
                    Some(body.into_bytes())
                },
            );
        }
        let mut old_files = Files::new();
        for name in next_files.keys() {
            crate::reject_symlink_components(Path::new(&agent.paths.config), Path::new(name))
                .await
                .map_err(|_| held())?;
            old_files.insert(name.clone(), read_managed(Path::new(name)).await?);
        }
        let candidate = self
            .activation_fork(
                &original,
                Some(revision.revision),
                Some(canonical_hash(&revision.snapshot)?),
                root,
                &control,
            )
            .await?;
        let rollback = self
            .activation_fork(
                &original,
                previous.prepared.configuration_revision,
                previous.prepared.configuration_sha256.clone(),
                root,
                &control,
            )
            .await?;
        // Both immutable recipes, including exact credentials/image/mapping, precede old stop.
        write_once(root, &recipe_path(root, &candidate), &candidate).await?;
        write_once(root, &recipe_path(root, &rollback), &rollback).await?;
        Ok(Plan {
            id: Uuid::new_v4(),
            controller_id: self.controller_id,
            revision: revision.revision,
            previous,
            candidate,
            candidate_stop_id: Uuid::new_v4(),
            rollback,
            rollback_stop_id: Uuid::new_v4(),
            files: next_files,
            previous_files: old_files,
            marker_sha256: self.activation_marker(agent).await?,
        })
    }

    async fn activation_running(
        &self,
        agent: &Agent,
        launch: &ContainerLaunch,
    ) -> Result<(), AppError> {
        let (control, files) = self.container_files(&launch.prepared.container).await?;
        let r = control
            .observe(&files, &launch.prepared.container.registration)
            .await?;
        if r.state != ContainerReceiptState::Observed
            || r.observation != ContainerObservation::Running
            || r.snapshot
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(|_| held())?
                != launch.snapshot
            || launch.origin.as_ref()
                != Some(&format!(
                    "http://{}:{}",
                    control
                        .endpoint(&files, &launch.prepared.container.registration)
                        .await?,
                    agent.api_port.ok_or_else(held)?
                ))
        {
            return Err(held());
        }
        Ok(())
    }

    async fn activation_exited(&self, launch: &ContainerLaunch) -> Result<(), AppError> {
        let (control, files) = self.container_files(&launch.prepared.container).await?;
        let r = control
            .observe(&files, &launch.prepared.container.registration)
            .await?;
        if r.state != ContainerReceiptState::Observed
            || r.observation != ContainerObservation::NamespaceExited
            || r.snapshot
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(|_| held())?
                != launch.snapshot
        {
            return Err(held());
        }
        Ok(())
    }

    async fn activation_prepare(
        &self,
        agent: &Agent,
        root: &Path,
        intent: &Intent,
        stop_id: Uuid,
        first: bool,
        control: &ContainerControl,
    ) -> Result<ContainerLaunch, AppError> {
        let files = intent.files(root);
        let name = format!("{}.{}", agent.id, intent.generation);
        let r = control
            .preparation(
                &files,
                &intent.process,
                intent.operation_id,
                &root.join(format!("{name}.create.json")),
                &root.join(format!("{name}.create.sqlite")),
                first,
            )
            .await?;
        let prepared = PreparedContainer {
            agent_id: agent.id,
            paths: agent.paths.clone(),
            api_port: agent.api_port,
            configuration_revision: intent.configuration_revision,
            configuration_sha256: intent.configuration_sha256.clone(),
            container: ContainerBinding {
                registration: r.registration,
                policy: r.policy,
                compose: files.compose.to_string_lossy().into_owned(),
                journal: files.journal.to_string_lossy().into_owned(),
                stop_journal: files.stop_journal.to_string_lossy().into_owned(),
                source_sha256: intent.source_sha256.clone(),
                context: intent.context.clone(),
                mapped: intent.mapped.clone(),
            },
        };
        let compose: Value = serde_json::from_slice(
            &private_file(root, Path::new(&prepared.container.compose), 1_048_576).await?,
        )
        .map_err(|_| held())?;
        validate_recipe(
            agent,
            &prepared,
            &compose,
            &crate::agent_runtime_token(&self.config, agent.id)?,
        )?;
        self.container_files(&prepared.container).await?;
        Ok(ContainerLaunch {
            prepared,
            controller_id: self.controller_id,
            state: "claimed".into(),
            snapshot: None,
            origin: None,
            stop_id,
        })
    }

    async fn activation_start(
        &self,
        agent: &Agent,
        launch: &ContainerLaunch,
        first: bool,
    ) -> Result<ContainerLaunch, AppError> {
        let (control, files) = self.container_files(&launch.prepared.container).await?;
        let r = if first {
            control
                .start(&files, &launch.prepared.container.registration)
                .await?
        } else {
            control
                .observe(&files, &launch.prepared.container.registration)
                .await?
        };
        if r.state != ContainerReceiptState::Observed
            || r.observation != ContainerObservation::Running
        {
            return Err(held());
        }
        let mut running = launch.clone();
        running.state = "running".into();
        running.snapshot =
            Some(serde_json::to_value(r.snapshot.ok_or_else(held)?).map_err(|_| held())?);
        running.origin = Some(format!(
            "http://{}:{}",
            control
                .endpoint(&files, &launch.prepared.container.registration)
                .await?,
            agent.api_port.ok_or_else(held)?
        ));
        Ok(running)
    }

    async fn activation_ready(
        &self,
        agent: &Agent,
        plan: &Plan,
        launch: &ContainerLaunch,
        rollback: bool,
    ) -> Result<Readiness, AppError> {
        let deadline = tokio::time::Instant::now() + HERMES_READY_TIMEOUT;
        let token = crate::agent_runtime_token(&self.config, agent.id)?;
        loop {
            self.activation_running(agent, launch).await?;
            self.activation_files(agent, plan, rollback, false).await?;
            let result = self
                .probe_hermes_at(launch.origin.as_deref().ok_or_else(held)?, &token)
                .await;
            if let Ok(capabilities) = result {
                self.activation_running(agent, launch).await?;
                self.activation_files(agent, plan, rollback, false).await?;
                return Ok(Readiness {
                    generation: launch.prepared.container.registration.generation,
                    files_sha256: canonical_hash(if rollback {
                        &plan.previous_files
                    } else {
                        &plan.files
                    })?,
                    capabilities_sha256: canonical_hash(&capabilities)?,
                });
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(AppError::validation(
                    "Candidate failed bounded API readiness with original namespace still known",
                ));
            }
            sleep(HERMES_READY_POLL).await;
        }
    }

    pub(super) async fn apply_container_revision(
        &self,
        revision: &domain::AgentConfigRevision,
    ) -> Result<(), AppError> {
        let _operations = self.container_operations.lock().await;
        let agent = self.repo.get_agent(revision.agent_id).await?;
        let config = self
            .config
            .fleet
            .container_control
            .as_ref()
            .ok_or_else(held)?;
        let root = private_root(&config.controller_root).await?;
        let _lock = AgentLock::acquire(&root, agent.id).await?;
        let path = root.join(format!(
            "{}.{}.activation.json",
            agent.id, revision.revision
        ));
        let saved = self
            .repo
            .get_container_activation(agent.id, revision.revision)
            .await?;
        let plan: Plan = if let Some(record) = &saved {
            if record.claim.controller_id != self.controller_id {
                return Err(held());
            }
            serde_json::from_slice(&private_file(&root, &path, 1_048_576).await?)
                .map_err(|_| held())?
        } else {
            if tokio::fs::symlink_metadata(&path).await.is_ok() {
                return Err(held());
            }
            let plan = self.activation_plan(&agent, revision, &root).await?;
            write_once(&root, &path, &plan).await?;
            plan
        };
        let claim = plan.claim()?;
        let mut record = match saved {
            Some(record) => record,
            None => self.repo.claim_container_activation(&claim).await?,
        };
        if canonical_hash(&record.claim)? != canonical_hash(&claim)? {
            return Err(held());
        }
        let (control, _) = self
            .container_files(&plan.previous.prepared.container)
            .await?;
        let mut fresh = false;
        loop {
            // Never trust a cached plan across a native command or filesystem write.
            let reread: Plan =
                serde_json::from_slice(&private_file(&root, &path, 1_048_576).await?)
                    .map_err(|_| held())?;
            if canonical_hash(&reread)? != claim.intent_sha256
                || self.activation_marker(&agent).await? != plan.marker_sha256
            {
                return Err(held());
            }
            let mut next = record.clone();
            use Phase::*;
            match record.phase {
                Planned => {
                    self.activation_running(&agent, &plan.previous).await?;
                    self.activation_files(&agent, &plan, true, false).await?;
                    next.phase = StoppingPrevious;
                }
                StoppingPrevious => {
                    let files = self
                        .container_files(&plan.previous.prepared.container)
                        .await?
                        .1;
                    next.previous_stop = Some(
                        serde_json::to_value(
                            control
                                .stop(
                                    &files,
                                    &plan.previous.prepared.container.registration,
                                    plan.previous.stop_id,
                                )
                                .await?,
                        )
                        .map_err(|_| held())?,
                    );
                    next.phase = PreviousStopped;
                }
                PreviousStopped => {
                    self.activation_exited(&plan.previous).await?;
                    next.phase = ApplyingCandidate;
                }
                ApplyingCandidate | ApplyingRollback => {
                    let rollback = record.phase == ApplyingRollback;
                    self.activation_exited(&plan.previous).await?;
                    if let Some(candidate) = &record.candidate {
                        self.activation_exited(candidate).await?;
                    }
                    match self.activation_files(&agent, &plan, rollback, true).await {
                        Ok(()) => {
                            next.phase = if rollback {
                                PreparingRollback
                            } else {
                                PreparingCandidate
                            }
                        }
                        Err(_) if !rollback => next.phase = ApplyingRollback,
                        Err(_) => return Err(held()),
                    }
                }
                PreparingCandidate | PreparingRollback => {
                    let rollback = record.phase == PreparingRollback;
                    self.activation_files(&agent, &plan, rollback, false)
                        .await?;
                    let intent = if rollback {
                        &plan.rollback
                    } else {
                        &plan.candidate
                    };
                    let l = self
                        .activation_prepare(
                            &agent,
                            &root,
                            intent,
                            if rollback {
                                plan.rollback_stop_id
                            } else {
                                plan.candidate_stop_id
                            },
                            fresh,
                            &control,
                        )
                        .await?;
                    if rollback {
                        next.rollback = Some(l);
                        next.phase = RollbackPrepared;
                    } else {
                        next.candidate = Some(l);
                        next.phase = CandidatePrepared;
                    }
                }
                CandidatePrepared | RollbackPrepared => {
                    let rollback = record.phase == RollbackPrepared;
                    self.activation_files(&agent, &plan, rollback, false)
                        .await?;
                    let l = if rollback {
                        &record.rollback
                    } else {
                        &record.candidate
                    }
                    .as_ref()
                    .ok_or_else(held)?;
                    let files = self.container_files(&l.prepared.container).await?.1;
                    if files.mapped.is_some() {
                        control
                            .attach(&files, &l.prepared.container.registration)
                            .await?;
                    }
                    next.phase = if rollback {
                        StartingRollback
                    } else {
                        StartingCandidate
                    };
                }
                StartingCandidate | StartingRollback => {
                    let rollback = record.phase == StartingRollback;
                    self.activation_files(&agent, &plan, rollback, false)
                        .await?;
                    let l = if rollback {
                        &record.rollback
                    } else {
                        &record.candidate
                    }
                    .as_ref()
                    .ok_or_else(held)?;
                    let running = self.activation_start(&agent, l, fresh).await?;
                    if rollback {
                        next.rollback = Some(running);
                        next.phase = RollbackRunning;
                    } else {
                        next.candidate = Some(running);
                        next.phase = CandidateRunning;
                    }
                }
                CandidateRunning | RollbackRunning => {
                    let rollback = record.phase == RollbackRunning;
                    let l = if rollback {
                        &record.rollback
                    } else {
                        &record.candidate
                    }
                    .as_ref()
                    .ok_or_else(held)?;
                    match self.activation_ready(&agent, &plan, l, rollback).await {
                        Ok(proof) => {
                            next.readiness = Some(proof);
                            next.phase = if rollback {
                                RollbackReady
                            } else {
                                CandidateReady
                            };
                        }
                        Err(AppError::Validation(_)) if !rollback => {
                            let mut l = l.clone();
                            l.state = "stopping".into();
                            next.candidate = Some(l);
                            next.phase = StoppingCandidate;
                        }
                        Err(_) => return Err(held()),
                    }
                }
                StoppingCandidate => {
                    let l = record.candidate.as_ref().ok_or_else(held)?;
                    let files = self.container_files(&l.prepared.container).await?.1;
                    next.candidate_stop = Some(
                        serde_json::to_value(
                            control
                                .stop(&files, &l.prepared.container.registration, l.stop_id)
                                .await?,
                        )
                        .map_err(|_| held())?,
                    );
                    let mut l = l.clone();
                    l.state = "exited".into();
                    next.candidate = Some(l);
                    next.phase = CandidateStopped;
                }
                CandidateStopped => {
                    self.activation_exited(record.candidate.as_ref().ok_or_else(held)?)
                        .await?;
                    next.phase = ApplyingRollback;
                }
                CandidateReady | RollbackReady => {
                    let rollback = record.phase == RollbackReady;
                    let l = if rollback {
                        &record.rollback
                    } else {
                        &record.candidate
                    }
                    .as_ref()
                    .ok_or_else(held)?;
                    // Publication is bracketed by fresh API, file and physical readback.
                    self.activation_ready(&agent, &plan, l, rollback)
                        .await
                        .map_err(|_| held())?;
                    next.phase = if rollback { RolledBack } else { Committed };
                }
                Committed | RolledBack => return Ok(()),
            }
            self.repo
                .advance_container_activation(&record, &next)
                .await?;
            fresh = true;
            record = next;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> Plan {
        let agent = Uuid::new_v4();
        let generation = Uuid::new_v4();
        let operation = Uuid::new_v4();
        let paths = json!({"runtime":"/agents/agent1/runtime","config":"/agents/agent1/config","workspace":"/agents/agent1/workspace","logs":"/agents/agent1/logs"});
        let policy = json!({"contract_version":2,"generation":generation,"resource_id":agent,"image_id":"sha256:original"});
        let intent:Intent=serde_json::from_value(json!({"agent_id":agent,"generation":generation,"operation_id":operation,
            "paths":paths,"api_port":24003,"configuration_revision":null,"configuration_sha256":null,"policy":policy,"local_policy":policy,
            "process":{"environment":{"API_SERVER_KEY":"private-original-credential"}},"source_sha256":container_lifecycle::UTILITY_SHA256,
            "context":"desktop-linux","mapped":null,"files_sha256":{}})).unwrap();
        let previous:ContainerLaunch=serde_json::from_value(json!({"controller_id":Uuid::new_v4(),"state":"running","stop_id":Uuid::new_v4(),
            "snapshot":{"init_pid":42},"origin":"http://172.18.0.2:24003","prepared":{"agent_id":agent,"paths":paths,"api_port":24003,
            "configuration_revision":null,"configuration_sha256":null,"container":{"policy":policy,"compose":"/private/compose.json",
            "journal":"/private/launch.sqlite","stop_journal":"/private/stop.sqlite","source_sha256":intent.source_sha256,"context":intent.context,
            "registration":{"contract_version":2,"operation_id":operation,"container_id":"a".repeat(64),"resource_id":agent,"generation":generation,
            "engine":{"ID":"engine","KernelVersion":"kernel","ServerVersion":"29"},"policy_sha256":"b".repeat(64),"inventory_sha256":"c".repeat(64),
            "running_inventory_sha256":"d".repeat(64),"compose_sha256":"e".repeat(64),"network_sha256":"f".repeat(64)}}}})).unwrap();
        let mut candidate = intent.clone();
        candidate.generation = Uuid::new_v4();
        candidate.operation_id = Uuid::new_v4();
        candidate.configuration_revision = Some(1);
        candidate.configuration_sha256 = Some("0".repeat(64));
        let mut rollback = intent;
        rollback.generation = Uuid::new_v4();
        rollback.operation_id = Uuid::new_v4();
        Plan {
            id: Uuid::new_v4(),
            controller_id: previous.controller_id,
            revision: 1,
            previous,
            candidate,
            candidate_stop_id: Uuid::new_v4(),
            rollback,
            rollback_stop_id: Uuid::new_v4(),
            files: BTreeMap::from([(
                "/agents/agent1/config/.env".into(),
                Some(b"NEW=private-target".to_vec()),
            )]),
            previous_files: BTreeMap::from([(
                "/agents/agent1/config/.env".into(),
                Some(b"OLD=private-previous".to_vec()),
            )]),
            marker_sha256: "1".repeat(64),
        }
    }

    #[test]
    fn saved_plan_seals_exact_rollback_credentials_recipe_and_both_generations() {
        let original = plan();
        let claim = original.claim().unwrap();
        let public = serde_json::to_string(&claim).unwrap();
        for secret in [
            "private-target",
            "private-previous",
            "private-original-credential",
        ] {
            assert!(!public.contains(secret));
        }
        for field in [
            "target",
            "rollback",
            "credential",
            "image",
            "generation",
            "stop",
            "marker",
        ] {
            let mut changed: Plan =
                serde_json::from_value(serde_json::to_value(&original).unwrap()).unwrap();
            match field {
                "target" => {
                    changed
                        .files
                        .values_mut()
                        .next()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .push(b'x');
                }
                "rollback" => {
                    changed
                        .previous_files
                        .values_mut()
                        .next()
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .push(b'x');
                }
                "credential" => {
                    changed.rollback.process["environment"]["API_SERVER_KEY"] = json!("rotated")
                }
                "image" => changed.candidate.policy["image_id"] = json!("sha256:foreign"),
                "generation" => changed.rollback.generation = Uuid::new_v4(),
                "stop" => changed.candidate_stop_id = Uuid::new_v4(),
                _ => changed.marker_sha256 = "2".repeat(64),
            }
            assert_ne!(changed.claim().unwrap().intent_sha256, claim.intent_sha256);
        }
    }

    #[cfg(target_os = "linux")]
    async fn root() -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("fleet-activation-{}", Uuid::new_v4()));
        tokio::fs::create_dir(&root).await.unwrap();
        tokio::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
            .await
            .unwrap();
        private_root(root.to_str().unwrap()).await.unwrap()
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn immutable_plan_is_durable_and_partial_or_foreign_files_stay_held() {
        let root = root().await;
        let path = root.join("activation.json");
        let plan = plan();
        write_once(&root, &path, &plan).await.unwrap();
        let before = private_file(&root, &path, 1_048_576).await.unwrap();
        assert!(write_once(&root, &path, &plan).await.is_err());
        assert_eq!(private_file(&root, &path, 1_048_576).await.unwrap(), before);
        let partial = root.join("partial.json");
        tokio::fs::write(&partial, b"{").await.unwrap();
        assert!(write_once(&root, &partial, &plan).await.is_err());
        assert_eq!(tokio::fs::read(&partial).await.unwrap(), b"{");
        let foreign = root.join("foreign.json");
        std::os::unix::fs::symlink(&path, &foreign).unwrap();
        assert!(write_once(&root, &foreign, &plan).await.is_err());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn native_custody_lock_has_one_winner_and_never_replaces_inode() {
        use std::os::unix::fs::MetadataExt;
        let root = root().await;
        let agent = Uuid::new_v4();
        let path = root.join(format!("{agent}.activation.lock"));
        let first = AgentLock::acquire(&root, agent).await.unwrap();
        let inode = std::fs::metadata(&path).unwrap().ino();
        assert!(AgentLock::acquire(&root, agent).await.is_err());
        drop(first);
        let second = AgentLock::acquire(&root, agent).await.unwrap();
        assert_eq!(std::fs::metadata(&path).unwrap().ino(), inode);
        drop(second);
        std::fs::hard_link(&path, root.join("foreign-link")).unwrap();
        assert!(AgentLock::acquire(&root, agent).await.is_err());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
