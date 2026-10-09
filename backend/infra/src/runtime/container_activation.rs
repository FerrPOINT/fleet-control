//! Original-custody replacement and read-only restart recovery, never a host controller.
use super::*;
use app::container_activation::{
    Activation, Claim, Generation, Phase, Readiness, RecoveryAction, RecoveryHold, RecoveryReason,
    held,
};
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
const MANAGED_FILE_LIMIT: usize = 262_144;
const PLAN_LIMIT: usize = 1_048_576;
const PREPLAN_RETRY_DELAY: Duration = Duration::from_secs(5);

fn validate_managed_files(files: &Files) -> Result<(), AppError> {
    if files
        .values()
        .flatten()
        .any(|body| body.len() > MANAGED_FILE_LIMIT)
    {
        return Err(AppError::validation(
            "Docker activation managed bytes exceed the readback limit",
        ));
    }
    Ok(())
}

#[derive(Debug)]
enum ActivationFailure {
    RetryReadOnly(AppError),
    Held(AppError),
    Recovery(Box<RecoveryHold>),
}

impl From<AppError> for ActivationFailure {
    fn from(error: AppError) -> Self {
        Self::Held(error)
    }
}

fn preplan_failure(error: AppError) -> ActivationFailure {
    match error {
        AppError::Validation(_) => ActivationFailure::Held(error),
        _ => ActivationFailure::RetryReadOnly(error),
    }
}

async fn retry_preplan<F, Fut>(
    agent: Uuid,
    delay: Duration,
    mut attempt: F,
) -> Result<(), ActivationFailure>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<(), ActivationFailure>>,
{
    loop {
        match attempt().await {
            Ok(()) => return Ok(()),
            Err(error @ (ActivationFailure::Held(_) | ActivationFailure::Recovery(_))) => {
                return Err(error);
            }
            Err(ActivationFailure::RetryReadOnly(error)) => {
                tracing::warn!(agent_id=%agent, "Docker activation read-only preflight will retry: {}",
                    crate::redact_text(&error.to_string()));
                sleep(delay).await;
            }
        }
    }
}

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
    fn validate_bytes(&self) -> Result<(), AppError> {
        validate_managed_files(&self.files)?;
        validate_managed_files(&self.previous_files)?;
        if serde_json::to_vec(self).map_err(|_| held())?.len() > PLAN_LIMIT {
            return Err(AppError::validation(
                "Docker activation serialized plan exceeds the private readback limit",
            ));
        }
        Ok(())
    }

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
    if path.parent() != Some(root) || bytes.len() > PLAN_LIMIT {
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

async fn seal_plan(root: &Path, path: &Path, plan: &Plan) -> Result<(), AppError> {
    plan.validate_bytes()?;
    write_once(root, path, plan).await?;
    write_once(root, &recipe_path(root, &plan.candidate), &plan.candidate).await?;
    write_once(root, &recipe_path(root, &plan.rollback), &plan.rollback).await
}

async fn read_plan(
    root: &Path,
    path: &Path,
    activation: Option<&Activation>,
) -> Result<Plan, AppError> {
    let plan: Plan = serde_json::from_slice(&private_file(root, path, PLAN_LIMIT as u64).await?)
        .map_err(|_| held())?;
    plan.validate_bytes()?;
    if let Some(activation) = activation {
        if canonical_hash(&plan.claim()?)? != canonical_hash(&activation.claim)? {
            return Err(held());
        }
    }
    // A crash halfway through sealing is not permission to fill in missing evidence.
    for intent in [&plan.candidate, &plan.rollback] {
        let saved: Intent = serde_json::from_slice(
            &private_file(root, &recipe_path(root, intent), PLAN_LIMIT as u64).await?,
        )
        .map_err(|_| held())?;
        if canonical_hash(&saved)? != canonical_hash(intent)? {
            return Err(held());
        }
    }
    Ok(plan)
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
            private_file(
                path.parent().ok_or_else(held)?,
                path,
                MANAGED_FILE_LIMIT as u64,
            )
            .await?,
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(held()),
    }
}

impl LocalRuntimeSupervisor {
    pub(super) async fn reconcile_container_revision(
        &self,
        revision: &domain::AgentConfigRevision,
    ) {
        if let Err(error) = retry_preplan(revision.agent_id, PREPLAN_RETRY_DELAY, || {
            self.apply_container_revision(revision)
        })
        .await
        {
            let record = self
                .repo
                .get_container_activation(revision.agent_id, revision.revision)
                .await;
            let launch = self.repo.get_container_launch(revision.agent_id).await;
            if let (Ok(record), Ok(Some(launch))) = (record, launch) {
                let recovery = match &error {
                    ActivationFailure::Recovery(recovery) => recovery.as_ref().clone(),
                    _ => {
                        let mut recovery =
                            RecoveryHold::new(self.controller_id, &launch, record.as_ref());
                        if recovery.reason != RecoveryReason::UnknownOriginalEffect {
                            recovery.reason = RecoveryReason::EvidenceUnavailable;
                        }
                        recovery.action = RecoveryAction::ReconcileOriginalCommand;
                        recovery
                    }
                };
                if self
                    .repo
                    .hold_container_activation(revision, record.as_ref(), &launch, &recovery)
                    .await
                    .is_err()
                {
                    tracing::warn!(agent_id=%revision.agent_id, "Activation recovery audit not committed; original state remains held");
                }
            }
            let detail = match error {
                ActivationFailure::Recovery(recovery) => {
                    format!("{:?}: {:?}", recovery.reason, recovery.action)
                }
                ActivationFailure::Held(error) | ActivationFailure::RetryReadOnly(error) => {
                    crate::redact_text(&error.to_string())
                }
            };
            tracing::warn!(
                agent_id = %revision.agent_id,
                "Docker configuration remains drained: {}",
                detail
            );
        }
    }

    async fn activation_restart_hold(
        &self,
        launch: &ContainerLaunch,
        activation: Option<&Activation>,
    ) -> Result<ActivationFailure, AppError> {
        if activation.is_some_and(|a| !a.tracks_launch(launch)) {
            return Err(held());
        }
        let mut recovery = RecoveryHold::new(self.controller_id, launch, activation);
        let (control, mut files) = self.container_files(&launch.prepared.container).await?;
        let original = &launch.prepared.container.registration;
        if let Some(saved) = self
            .repo
            .get_container_recovery(original.generation)
            .await?
        {
            recovery.recovery_id = Some(saved.command.request.id);
            recovery.recovery_command_sha256 = Some(canonical_hash(&saved.command)?);
            // Historical readback uses the immutable delivery, never the mutable heartbeat.
            files.recovery = Some(saved.command.clone());
            if let Ok(ack) = control
                .recovery_action(
                    &files,
                    original,
                    "read_controller_recovery",
                    self.controller_id,
                )
                .await
            {
                if ack["witness"]["receipt"]["snapshot"]
                    == launch.snapshot.clone().ok_or_else(held)?
                {
                    recovery.readback_sha256 = Some(canonical_hash(&ack)?);
                    if saved.receipt.as_ref() == Some(&ack)
                        && saved.command.request.controller_id == self.controller_id
                        && saved.lease_valid
                        && saved.lease_receipt.is_some()
                        && self.container_owner_files(launch).await.is_ok()
                        && recovery.reason != RecoveryReason::UnknownOriginalEffect
                    {
                        recovery.reason = RecoveryReason::RecoveredGenerationChangeUnsupported;
                        recovery.action =
                            RecoveryAction::ResumeOrRollbackOriginalPlanWithCompatibleBase;
                    }
                }
            }
        } else if let Ok(receipt) = control.observe(&files, original).await {
            // Original protocol2 may still read the same physical start after a process-only restart.
            // This evidence is diagnostic, never logical ownership or a new native permit.
            if receipt
                .snapshot
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(|_| held())?
                == launch.snapshot
            {
                recovery.readback_sha256 = Some(canonical_hash(&receipt)?);
            }
        }
        Ok(ActivationFailure::Recovery(Box::new(recovery)))
    }

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
        // Validate rendered bytes, including secrets, pretty JSON, skills and the revision marker.
        validate_managed_files(&next_files)?;
        let mut old_files = Files::new();
        for name in next_files.keys() {
            crate::reject_symlink_components(Path::new(&agent.paths.config), Path::new(name))
                .await
                .map_err(|_| held())?;
            old_files.insert(name.clone(), read_managed(Path::new(name)).await?);
        }
        self.activation_running(agent, &previous).await?;
        self.probe_hermes_at(
            previous.origin.as_deref().ok_or_else(held)?,
            &crate::agent_runtime_token(&self.config, agent.id)?,
        )
        .await
        .map_err(|_| held())?;
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
        let plan = Plan {
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
        };
        plan.validate_bytes()?;
        Ok(plan)
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

    async fn apply_container_revision(
        &self,
        revision: &domain::AgentConfigRevision,
    ) -> Result<(), ActivationFailure> {
        let _operations = self.container_operations.lock(revision.agent_id).await;
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
        let launch = self
            .repo
            .get_container_launch(agent.id)
            .await?
            .ok_or_else(held)?;
        if saved.as_ref().is_some_and(|a| a.phase.terminal()) {
            return Ok(());
        }
        self.activation_marker(&agent).await?;
        let has_plan = match tokio::fs::symlink_metadata(&path).await {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(_) => return Err(held().into()),
        };
        let plan: Plan = if saved.is_some() || has_plan {
            let plan = read_plan(&root, &path, saved.as_ref()).await?;
            if plan.revision != revision.revision
                || plan.previous.prepared.agent_id != agent.id
                || self.activation_marker(&agent).await? != plan.marker_sha256
                || saved.is_none() && !Activation::planned(plan.claim()?).tracks_launch(&launch)
            {
                return Err(held().into());
            }
            if plan.controller_id != self.controller_id
                || self
                    .repo
                    .get_container_recovery(launch.prepared.container.registration.generation)
                    .await?
                    .is_some()
            {
                return Err(self
                    .activation_restart_hold(&launch, saved.as_ref())
                    .await?);
            }
            plan
        } else {
            if launch.controller_id != self.controller_id
                || self
                    .repo
                    .get_container_recovery(launch.prepared.container.registration.generation)
                    .await?
                    .is_some()
            {
                return Err(self.activation_restart_hold(&launch, None).await?);
            }
            // The only retryable stage: no PG command or private plan, and read-only observations.
            let plan = self
                .activation_plan(&agent, revision, &root)
                .await
                .map_err(preplan_failure)?;
            // Sealing/unknown writes never return a read-only retry permit.
            seal_plan(&root, &path, &plan).await?;
            plan
        };
        plan.validate_bytes()?;
        let claim = plan.claim()?;
        let mut record = match saved {
            Some(record) => record,
            None => self.repo.claim_container_activation(&claim).await?,
        };
        if canonical_hash(&record.claim)? != canonical_hash(&claim)? {
            return Err(held().into());
        }
        let (control, _) = self
            .container_files(&plan.previous.prepared.container)
            .await?;
        let mut fresh = false;
        loop {
            // Never trust a cached plan across a native command or filesystem write.
            let reread: Plan =
                serde_json::from_slice(&private_file(&root, &path, PLAN_LIMIT as u64).await?)
                    .map_err(|_| held())?;
            if canonical_hash(&reread)? != claim.intent_sha256
                || self.activation_marker(&agent).await? != plan.marker_sha256
            {
                return Err(held().into());
            }
            let mut next = record.clone();
            use Phase::*;
            match record.phase {
                Planned => {
                    plan.validate_bytes()?;
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
                        Err(_) => return Err(held().into()),
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
                        Err(_) => return Err(held().into()),
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

    struct Scratch(PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

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
        let mut original = plan();
        original.files.insert(
            "/agents/agent1/config/skills/\u{e9}\u{1f600}/SKILL.md".into(),
            Some(
                "\u{43f}\u{440}\u{438}\u{432}\u{435}\u{442}"
                    .as_bytes()
                    .to_vec(),
            ),
        );
        let claim = original.claim().unwrap();
        let reloaded: Plan =
            serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();
        assert_eq!(reloaded.claim().unwrap().intent_sha256, claim.intent_sha256);
        assert_eq!(
            canonical_hash(&json!({"config":{"config_json":{},"soul_md":"\u{43f}\u{440}\u{438}\u{432}\u{435}\u{442} \u{1f600}","env_json":{}},"skills":[]})).unwrap(),
            "a5de7dfacd6c2771ef639bb9cbbfe24b3f38b4eeb5170ad7f6c7a6c6b2404e69"
        );
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

    #[test]
    fn managed_limits_cover_all_rendered_bytes_and_the_serialized_plan() {
        let request = domain::UpdateAgentConfigRequest {
            config_json: json!({}),
            soul_md: "A".repeat(MANAGED_FILE_LIMIT + 1),
            env_json: json!({}),
        };
        // This valid API input was previously stopped/applied before readback failed.
        assert!(request.input_errors().is_empty());
        for name in [
            "SOUL.md",
            "config.yaml",
            ".env",
            "skills/enabled/SKILL.md",
            ".fleet-config-revision.json",
        ] {
            let mut p = plan();
            p.files
                .insert(name.into(), Some(vec![b'A'; MANAGED_FILE_LIMIT]));
            assert!(p.validate_bytes().is_ok());
            p.files.get_mut(name).unwrap().as_mut().unwrap().push(b'A');
            assert!(matches!(p.validate_bytes(), Err(AppError::Validation(_))));
        }
        let utf8 = "\u{e9}".repeat(MANAGED_FILE_LIMIT / 2);
        let mut files = Files::from([("SOUL.md".into(), Some(utf8.into_bytes()))]);
        assert!(validate_managed_files(&files).is_ok());
        files
            .get_mut("SOUL.md")
            .unwrap()
            .as_mut()
            .unwrap()
            .push(b'A');
        assert!(validate_managed_files(&files).is_err());
        let mut aggregate = plan();
        aggregate.files = (0..2)
            .map(|n| (format!("skill{n}"), Some(vec![b'A'; 180_000])))
            .collect();
        assert!(validate_managed_files(&aggregate.files).is_ok());
        assert!(serde_json::to_vec(&aggregate).unwrap().len() > PLAN_LIMIT);
        assert!(matches!(
            aggregate.validate_bytes(),
            Err(AppError::Validation(_))
        ));
    }

    #[tokio::test]
    async fn validation_and_unknown_effect_failures_never_retry() {
        for stage in [
            "seal",
            "stop",
            "prepare",
            "attach",
            "start",
            "publish",
            "validation",
        ] {
            let mut attempts = 0;
            let result = retry_preplan(Uuid::new_v4(), Duration::ZERO, || {
                attempts += 1;
                let error = if stage == "validation" {
                    preplan_failure(AppError::validation("oversized rendered bytes"))
                } else {
                    // Every error outside the explicitly read-only plan builder defaults to Held.
                    ActivationFailure::from(held())
                };
                std::future::ready(Err(error))
            })
            .await;
            assert!(result.is_err(), "{stage}");
            assert_eq!(attempts, 1, "{stage}");
        }
    }

    #[tokio::test]
    async fn read_only_observe_failure_retries_original_command_until_recovery() {
        use container_control::ControlSource;
        use sha2::{Digest, Sha256};
        let scratch =
            Scratch(std::env::temp_dir().join(format!("fleet-preplan-observe-{}", Uuid::new_v4())));
        let root = &scratch.0;
        tokio::fs::create_dir_all(root.join("scripts"))
            .await
            .unwrap();
        let p = plan();
        let r = &p.previous.prepared.container.registration;
        let snapshot = json!({"contract_version":2,"container_id":r.container_id,"engine":r.engine,
            "policy_sha256":r.policy_sha256,"inventory_sha256":r.running_inventory_sha256,
            "network_sha256":r.network_sha256,"init_pid":42,"started_at":"2026-10-09T10:00:00Z"});
        let receipt = json!({"contract_version":2,"operation_id":r.operation_id,
            "container_id":r.container_id,"resource_id":r.resource_id,"registration_sha256":canonical_hash(r).unwrap(),
            "generation":r.generation,"state":"observed","observation":"running","snapshot":snapshot});
        let source = format!(
            r#"import json,sys
from pathlib import Path
r=json.load(sys.stdin)
assert r['action']=='observe'
assert r['registration']==json.loads({original:?})
audit=Path(sys.argv[1])/'read-attempts'
failed=not audit.exists()
with audit.open('a') as f: f.write(r['action']+'\n')
result=json.loads({receipt:?})
if failed: result.update(state='held',observation='unavailable',snapshot=None)
print(json.dumps({{'protocol_version':1,'action':r['action'],'result':result}}))
sys.exit(2 if failed else 0)
"#,
            original = serde_json::to_string(r).unwrap(),
            receipt = receipt.to_string()
        );
        let sources = [
            "# fake boundary\n".to_owned(),
            "# fake bootstrap\n".to_owned(),
            source,
        ];
        for (name, source) in [
            "runtime_boundary.py",
            "runtime_bootstrap.py",
            "runtime_control.py",
        ]
        .iter()
        .zip(&sources)
        {
            tokio::fs::write(root.join("scripts").join(name), source)
                .await
                .unwrap();
        }
        let control = ContainerControl::new(
            PathBuf::from(std::env::var("FLEET_TEST_PYTHON").unwrap_or_else(|_| "python3".into())),
            ControlSource {
                root: root.clone(),
                sha256: sources.map(|s| hex::encode(Sha256::digest(s.as_bytes()))),
            },
            "desktop-linux".into(),
        )
        .unwrap();
        let files = ContainerLaunchFiles {
            policy: p.previous.prepared.container.policy.clone(),
            compose: root.join("compose.json"),
            journal: root.join("launch.sqlite"),
            stop_journal: root.join("stop.sqlite"),
            mapped: None,
            recovery: None,
        };
        // The same live worker retains its claimed revision; neither read grants a native permit.
        tokio::time::timeout(
            Duration::from_secs(20),
            retry_preplan(p.previous.prepared.agent_id, Duration::ZERO, || async {
                let read = control.observe(&files, r).await.map_err(preplan_failure)?;
                if read.state != ContainerReceiptState::Observed
                    || read.observation != ContainerObservation::Running
                {
                    return Err(preplan_failure(held()));
                }
                assert_eq!(serde_json::to_value(read.snapshot).unwrap(), snapshot);
                Ok(())
            }),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(
            tokio::fs::read_to_string(root.join("read-attempts"))
                .await
                .unwrap(),
            "observe\nobserve\n"
        );
        for path in ["compose.json", "launch.sqlite", "stop.sqlite"] {
            assert!(!root.join(path).exists());
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
    async fn restart_reads_exact_sealed_plan_without_resealing_or_replacing_commands() {
        let scratch = Scratch(root().await);
        let root = &scratch.0;
        let path = root.join("activation.json");
        let plan = plan();
        seal_plan(root, &path, &plan).await.unwrap();
        let before = private_file(root, &path, PLAN_LIMIT as u64).await.unwrap();
        // No PG ACK yet: only the complete original plan can be considered, never a fresh fork.
        let restored = read_plan(root, &path, None).await.unwrap();
        assert_eq!(
            canonical_hash(&restored).unwrap(),
            canonical_hash(&plan).unwrap()
        );
        let record = Activation::planned(plan.claim().unwrap());
        read_plan(root, &path, Some(&record)).await.unwrap();
        let mut foreign = record.clone();
        foreign.claim.controller_id = Uuid::new_v4();
        assert!(read_plan(root, &path, Some(&foreign)).await.is_err());
        assert_eq!(
            private_file(root, &path, PLAN_LIMIT as u64).await.unwrap(),
            before
        );
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn interrupted_sealing_holds_without_rebuilding_missing_or_foreign_recipe() {
        let scratch = Scratch(root().await);
        let root = &scratch.0;
        let path = root.join("activation.json");
        let plan = plan();
        write_once(root, &path, &plan).await.unwrap();
        assert!(read_plan(root, &path, None).await.is_err());
        assert!(!recipe_path(root, &plan.candidate).exists());
        write_once(root, &recipe_path(root, &plan.candidate), &plan.candidate)
            .await
            .unwrap();
        assert!(read_plan(root, &path, None).await.is_err());
        assert!(!recipe_path(root, &plan.rollback).exists());
        let mut foreign = plan.rollback.clone();
        foreign.process["environment"]["API_SERVER_KEY"] = json!("foreign-credential");
        write_once(root, &recipe_path(root, &plan.rollback), &foreign)
            .await
            .unwrap();
        assert!(read_plan(root, &path, None).await.is_err());
        assert_eq!(
            canonical_hash(
                &serde_json::from_slice::<Intent>(
                    &private_file(root, &recipe_path(root, &plan.rollback), PLAN_LIMIT as u64)
                        .await
                        .unwrap()
                )
                .unwrap()
            )
            .unwrap(),
            canonical_hash(&foreign).unwrap()
        );
    }

    #[test]
    fn restart_holds_name_original_unknown_generation_not_the_last_observable_launch() {
        let plan = plan();
        let mut activation = Activation::planned(plan.claim().unwrap());
        for (phase, identity) in [
            (Phase::PreparingCandidate, &plan.candidate),
            (Phase::StartingCandidate, &plan.candidate),
            (Phase::StoppingCandidate, &plan.candidate),
            (Phase::PreparingRollback, &plan.rollback),
            (Phase::StartingRollback, &plan.rollback),
        ] {
            activation.phase = phase;
            let hold = RecoveryHold::new(Uuid::new_v4(), &plan.previous, Some(&activation));
            assert!(activation.tracks_launch(&plan.previous));
            let mut foreign = plan.previous.clone();
            foreign.controller_id = Uuid::new_v4();
            assert!(!activation.tracks_launch(&foreign));
            foreign = plan.previous.clone();
            foreign.prepared.container.registration.generation = Uuid::new_v4();
            assert!(!activation.tracks_launch(&foreign));
            assert_eq!(hold.reason, RecoveryReason::UnknownOriginalEffect);
            assert_eq!(hold.action, RecoveryAction::ReconcileOriginalCommand);
            assert_eq!(hold.generation, identity.generation);
            assert_eq!(hold.operation_id, identity.operation_id);
            assert_eq!(
                hold.custody_generation,
                plan.previous.prepared.container.registration.generation
            );
            assert_ne!(hold.generation, hold.custody_generation);
            let public = serde_json::to_string(&hold).unwrap();
            for secret in [
                "private-target",
                "private-previous",
                "private-original-credential",
            ] {
                assert!(!public.contains(secret));
            }
        }
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn oversized_target_never_seals_or_changes_previous_working_bytes() {
        use std::os::unix::fs::PermissionsExt;
        let scratch = Scratch(root().await);
        let root = &scratch.0;
        let soul = root.join("SOUL.md");
        let original = vec![b'A'; MANAGED_FILE_LIMIT];
        tokio::fs::write(&soul, &original).await.unwrap();
        tokio::fs::set_permissions(&soul, std::fs::Permissions::from_mode(0o600))
            .await
            .unwrap();
        assert_eq!(read_managed(&soul).await.unwrap(), Some(original.clone()));
        let mut p = plan();
        p.files.insert(
            soul.to_string_lossy().into_owned(),
            Some(vec![b'A'; MANAGED_FILE_LIMIT + 1]),
        );
        let path = root.join("activation.json");
        assert!(matches!(
            seal_plan(&root, &path, &p).await,
            Err(AppError::Validation(_))
        ));
        assert!(!path.exists());
        assert!(!recipe_path(&root, &p.candidate).exists());
        assert!(!recipe_path(&root, &p.rollback).exists());
        assert_eq!(read_managed(&soul).await.unwrap(), Some(original));
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
