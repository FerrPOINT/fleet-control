use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use shared::AppError;
use std::{
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

const MAX_BACKUP_BYTES: usize = 8 * 1024 * 1024;
const MAX_JOURNAL_BYTES: usize = 24 * 1024 * 1024;
const MAX_FILES: usize = 128;
const JOURNAL_NAME: &str = ".fleet-activation-journal.json";

pub(super) struct JournalLocation<'a> {
    pub agents_root: &'a Path,
    pub config_directory: &'a Path,
    pub controller_root: &'a Path,
}

fn held() -> AppError {
    AppError::Unavailable(
        "private controller journal requires reconciliation; agent remains drained".into(),
    )
}

pub(super) async fn private_controller_directory(path: &Path) -> Result<PathBuf, AppError> {
    if path.as_os_str().is_empty() {
        return Err(held());
    }
    let absolute = crate::normalize_path(path).map_err(|_| held())?;
    let filesystem_root = absolute.ancestors().last().ok_or_else(held)?;
    crate::reject_symlink_components(filesystem_root, &absolute)
        .await
        .map_err(|_| held())?;
    let metadata = tokio::fs::symlink_metadata(&absolute)
        .await
        .map_err(|_| held())?;
    if !metadata.is_dir() {
        return Err(held());
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        let process_owner = tokio::fs::metadata("/proc/self")
            .await
            .map_err(|_| held())?
            .uid();
        if metadata.uid() != process_owner || metadata.mode() & 0o777 != 0o700 {
            return Err(held());
        }
        tokio::fs::canonicalize(&absolute).await.map_err(|_| held())
    }
    #[cfg(not(target_os = "linux"))]
    Err(held())
}

pub(super) fn controller_journal_path(controller_root: &Path, agent_id: Uuid) -> PathBuf {
    controller_root.join(format!("{agent_id}.activation.json"))
}

// Contains resolved env secrets: never implement Debug or expose the document through API.
pub(super) struct ActivationJournal {
    root: PathBuf,
    path: PathBuf,
    document: Vec<u8>,
    lock: super::activation_lock::ActivationLock,
    payload: JournalDocument,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ActivationIdentity {
    pub agent_id: Uuid,
    pub revision: i64,
    pub candidate_sha256: String,
    pub effective_revision: Option<i64>,
    pub effective_sha256: Option<String>,
    pub controller_id: Uuid,
    pub original_launch: Option<app::runtime_launch::RuntimeLaunchBinding>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalBackup {
    path: PathBuf,
    previous_hex: Option<String>,
    expected_sha256: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct JournalDocument {
    contract_version: u8,
    identity: ActivationIdentity,
    agents_root: PathBuf,
    config_directory: PathBuf,
    was_running: bool,
    files: Vec<JournalBackup>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SignedJournal {
    payload: JournalDocument,
    authentication: String,
}

fn authenticate(secret: &str, payload: &JournalDocument) -> Result<Hmac<Sha256>, AppError> {
    if secret.trim().is_empty() {
        return Err(held());
    }
    let mut authentication =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| held())?;
    authentication.update(b"fleet:configuration-activation-journal:v3\0");
    authentication.update(&serde_json::to_vec(payload).map_err(|_| held())?);
    Ok(authentication)
}

fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_document(document: &JournalDocument) -> Result<(), AppError> {
    let identity = &document.identity;
    if document.contract_version != 3
        || identity.agent_id.is_nil()
        || identity.controller_id.is_nil()
        || identity.revision <= 0
        || !valid_hash(&identity.candidate_sha256)
        || identity
            .effective_revision
            .is_some_and(|revision| revision <= 0 || revision >= identity.revision)
        || identity.effective_revision.is_some() != identity.effective_sha256.is_some()
        || identity
            .effective_sha256
            .as_ref()
            .is_some_and(|hash| !valid_hash(hash))
        || document.files.is_empty()
        || document.files.len() > MAX_FILES
    {
        return Err(held());
    }
    if let Some(launch) = &identity.original_launch
        && (launch.id.is_nil()
            || launch.agent_id != identity.agent_id
            || launch.controller_id != identity.controller_id
            || !document.was_running)
    {
        return Err(held());
    }
    let mut paths = std::collections::HashSet::new();
    let mut total = 0usize;
    for entry in &document.files {
        if entry.path.as_os_str().is_empty()
            || entry
                .path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
            || entry.path == Path::new(JOURNAL_NAME)
            || !paths.insert(entry.path.to_string_lossy().to_ascii_lowercase())
            || entry
                .expected_sha256
                .as_ref()
                .is_some_and(|hash| !valid_hash(hash))
            || (entry.expected_sha256.is_none()
                && entry.path.file_name().is_none_or(|name| name != "SKILL.md"))
        {
            return Err(held());
        }
        if let Some(previous) = &entry.previous_hex {
            if previous.len() > MAX_BACKUP_BYTES * 2 || hex::decode(previous).is_err() {
                return Err(held());
            }
            total = total.saturating_add(previous.len() / 2);
        }
    }
    if total > MAX_BACKUP_BYTES {
        return Err(held());
    }
    Ok(())
}

async fn read_private_journal(root: &Path, path: &Path) -> Result<Option<Vec<u8>>, AppError> {
    crate::reject_symlink_components(root, path)
        .await
        .map_err(|_| held())?;
    #[cfg(target_os = "linux")]
    {
        use rustix::fs::{Mode, OFlags, open};
        use std::os::unix::fs::MetadataExt;
        let descriptor = match open(
            path,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK,
            Mode::empty(),
        ) {
            Ok(descriptor) => descriptor,
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(_) => return Err(held()),
        };
        let file = tokio::fs::File::from_std(descriptor.into());
        let metadata = file.metadata().await.map_err(|_| held())?;
        let named = tokio::fs::symlink_metadata(path)
            .await
            .map_err(|_| held())?;
        if !metadata.is_file()
            || metadata.nlink() != 1
            || metadata.mode() & 0o777 != 0o600
            || metadata.uid() != tokio::fs::metadata(root).await.map_err(|_| held())?.uid()
            || metadata.len() > MAX_JOURNAL_BYTES as u64
            || metadata.dev() != named.dev()
            || metadata.ino() != named.ino()
        {
            return Err(held());
        }
        let mut document = Vec::new();
        file.take((MAX_JOURNAL_BYTES + 1) as u64)
            .read_to_end(&mut document)
            .await
            .map_err(|_| held())?;
        let named = tokio::fs::symlink_metadata(path)
            .await
            .map_err(|_| held())?;
        if document.len() > MAX_JOURNAL_BYTES
            || !named.is_file()
            || named.dev() != metadata.dev()
            || named.ino() != metadata.ino()
            || named.nlink() != 1
            || named.mode() & 0o777 != 0o600
            || named.uid() != metadata.uid()
        {
            return Err(held());
        }
        Ok(Some(document))
    }
    #[cfg(not(target_os = "linux"))]
    Err(held())
}

pub(super) async fn read_backup(root: &Path, path: &Path) -> Result<Option<Vec<u8>>, AppError> {
    crate::reject_symlink_components(root, path).await?;
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.is_file() => {}
        Ok(_) => {
            return Err(AppError::validation(
                "configuration backup must be a regular file",
            ));
        }
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            return Err(AppError::Unavailable(
                "configuration backup cannot be read".into(),
            ));
        }
    }
    let file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            return Err(AppError::Unavailable(
                "configuration backup cannot be read".into(),
            ));
        }
    };
    if !file.metadata().await.map_err(AppError::internal)?.is_file() {
        return Err(AppError::validation(
            "configuration backup must be a regular file",
        ));
    }
    let mut bytes = Vec::new();
    file.take((MAX_BACKUP_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| AppError::Unavailable("configuration backup cannot be read".into()))?;
    if bytes.len() > MAX_BACKUP_BYTES {
        return Err(AppError::validation(
            "configuration backup exceeds its byte limit",
        ));
    }
    Ok(Some(bytes))
}

impl ActivationJournal {
    pub(super) async fn prepare(
        location: JournalLocation<'_>,
        identity: ActivationIdentity,
        was_running: bool,
        files: &[(PathBuf, String)],
        backups: &[(PathBuf, Option<Vec<u8>>)],
        secret: &str,
        lock: super::activation_lock::ActivationLock,
    ) -> Result<Self, AppError> {
        let root = location.agents_root;
        let config_directory = location.config_directory;
        let agent_id = identity.agent_id;
        if identity.revision <= 0
            || agent_id.is_nil()
            || files.is_empty()
            || files.len() > MAX_FILES
            || files.len() != backups.len()
        {
            return Err(AppError::validation(
                "invalid configuration journal identity or file count",
            ));
        }
        crate::reject_symlink_components(root, config_directory).await?;
        // Never auto-adopt or relocate an interrupted legacy recovery document.
        match tokio::fs::symlink_metadata(config_directory.join(JOURNAL_NAME)).await {
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            _ => return Err(held()),
        }
        let controller_root = private_controller_directory(location.controller_root).await?;
        lock.verify_for(&controller_root, agent_id).await?;
        let agents_root = tokio::fs::canonicalize(root).await.map_err(|_| held())?;
        let canonical_config_directory = tokio::fs::canonicalize(config_directory)
            .await
            .map_err(|_| held())?;
        if !canonical_config_directory.starts_with(&agents_root) {
            return Err(held());
        }
        if controller_root.starts_with(&agents_root) || agents_root.starts_with(&controller_root) {
            return Err(held());
        }
        let mut entries = Vec::new();
        let mut total = 0usize;
        let mut paths = std::collections::HashSet::new();
        for ((path, body), (backup_path, previous)) in files.iter().zip(backups) {
            crate::reject_symlink_components(root, path).await?;
            let relative = path
                .strip_prefix(config_directory)
                .map_err(|_| AppError::validation("journal file escapes agent configuration"))?;
            if path != backup_path
                || relative.as_os_str().is_empty()
                || relative
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
                || relative == Path::new(JOURNAL_NAME)
                || !paths.insert(relative.to_string_lossy().to_ascii_lowercase())
            {
                return Err(AppError::validation(
                    "invalid or duplicate configuration journal path",
                ));
            }
            total = total.saturating_add(previous.as_ref().map_or(0, Vec::len));
            if total > MAX_BACKUP_BYTES {
                return Err(AppError::validation(
                    "configuration backup exceeds its total byte limit",
                ));
            }
            let absent = body.is_empty() && path.file_name().is_some_and(|name| name == "SKILL.md");
            entries.push(JournalBackup {
                path: relative.to_path_buf(),
                previous_hex: previous.as_ref().map(hex::encode),
                expected_sha256: if absent {
                    None
                } else {
                    Some(hex::encode(Sha256::digest(body.as_bytes())))
                },
            });
        }
        let payload = JournalDocument {
            contract_version: 3,
            identity,
            agents_root,
            config_directory: canonical_config_directory,
            was_running,
            files: entries,
        };
        validate_document(&payload)?;
        let authentication = hex::encode(authenticate(secret, &payload)?.finalize().into_bytes());
        let signed = SignedJournal {
            payload,
            authentication,
        };
        let document = serde_json::to_vec(&signed).map_err(|_| held())?;
        let payload = signed.payload;
        if document.len() > MAX_JOURNAL_BYTES {
            return Err(AppError::validation(
                "configuration journal exceeds its byte limit",
            ));
        }
        let path = controller_journal_path(&controller_root, agent_id);
        crate::reject_symlink_components(&controller_root, &path)
            .await
            .map_err(|_| held())?;
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options.open(&path).await.map_err(|_| {
            AppError::Unavailable(
                "configuration journal cannot be reserved; reconciliation is required".into(),
            )
        })?;
        // A partial write intentionally leaves a hold; it must never become another activation.
        file.write_all(&document).await.map_err(|_| {
            AppError::Unavailable(
                "configuration journal persistence failed; agent remains drained".into(),
            )
        })?;
        file.sync_all().await.map_err(|_| {
            AppError::Unavailable(
                "configuration journal persistence failed; agent remains drained".into(),
            )
        })?;
        drop(file);
        sync_directory(&controller_root).await?;
        Ok(Self {
            root: controller_root,
            path,
            document,
            lock,
            payload,
        })
    }

    pub(super) fn identity(&self) -> &ActivationIdentity {
        &self.payload.identity
    }

    pub(super) fn was_running(&self) -> bool {
        self.payload.was_running
    }

    pub(super) fn rollback_claim(&self) -> app::runtime_launch::ConfigurationRollbackClaim {
        let identity = self.identity();
        app::runtime_launch::ConfigurationRollbackClaim {
            agent_id: identity.agent_id,
            revision: identity.revision,
            candidate_sha256: identity.candidate_sha256.clone(),
            effective_revision: identity.effective_revision,
            effective_sha256: identity.effective_sha256.clone(),
            journal_sha256: hex::encode(Sha256::digest(&self.document)),
        }
    }

    pub(super) async fn reopen(
        location: JournalLocation<'_>,
        agent_id: Uuid,
        secret: &str,
        lock: super::activation_lock::ActivationLock,
    ) -> Result<Option<Self>, AppError> {
        let root = private_controller_directory(location.controller_root).await?;
        lock.verify_for(&root, agent_id).await?;
        let path = controller_journal_path(&root, agent_id);
        crate::reject_symlink_components(&root, &path)
            .await
            .map_err(|_| held())?;
        let document = match read_private_journal(&root, &path).await {
            Ok(Some(document)) => document,
            Ok(None) => return Ok(None),
            Err(_) => return Err(held()),
        };
        let signed: SignedJournal = serde_json::from_slice(&document).map_err(|_| held())?;
        let signature = hex::decode(&signed.authentication).map_err(|_| held())?;
        authenticate(secret, &signed.payload)?
            .verify_slice(&signature)
            .map_err(|_| held())?;
        validate_document(&signed.payload)?;
        let agents_root = tokio::fs::canonicalize(location.agents_root)
            .await
            .map_err(|_| held())?;
        crate::reject_symlink_components(location.agents_root, location.config_directory)
            .await
            .map_err(|_| held())?;
        let config_directory = tokio::fs::canonicalize(location.config_directory)
            .await
            .map_err(|_| held())?;
        if signed.payload.identity.agent_id != agent_id
            || signed.payload.agents_root != agents_root
            || signed.payload.config_directory != config_directory
            || !config_directory.starts_with(&agents_root)
            || root.starts_with(&agents_root)
            || agents_root.starts_with(&root)
        {
            return Err(held());
        }
        match tokio::fs::symlink_metadata(config_directory.join(JOURNAL_NAME)).await {
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            _ => return Err(held()),
        }
        let journal = Self {
            root,
            path,
            document,
            lock,
            payload: signed.payload,
        };
        journal.verify().await?;
        Ok(Some(journal))
    }

    pub(super) fn verify_plan(&self, files: &[(PathBuf, String)]) -> Result<(), AppError> {
        if files.len() != self.payload.files.len() {
            return Err(held());
        }
        for ((path, body), entry) in files.iter().zip(&self.payload.files) {
            let absent = body.is_empty() && path.file_name().is_some_and(|name| name == "SKILL.md");
            let expected = if absent {
                None
            } else {
                Some(hex::encode(Sha256::digest(body.as_bytes())))
            };
            if *path != self.payload.config_directory.join(&entry.path)
                || expected != entry.expected_sha256
            {
                return Err(held());
            }
        }
        Ok(())
    }

    pub(super) async fn verify_candidate_files(&self) -> Result<(), AppError> {
        self.verify().await?;
        for entry in &self.payload.files {
            let path = self.payload.config_directory.join(&entry.path);
            let bytes = read_backup(&self.payload.agents_root, &path).await?;
            if bytes
                .as_ref()
                .map(|bytes| hex::encode(Sha256::digest(bytes)))
                != entry.expected_sha256
            {
                return Err(held());
            }
        }
        Ok(())
    }

    pub(super) async fn verify(&self) -> Result<(), AppError> {
        self.lock.verify().await?;
        private_controller_directory(&self.root).await?;
        crate::reject_symlink_components(&self.root, &self.path)
            .await
            .map_err(|_| held())?;
        let observed = read_private_journal(&self.root, &self.path)
            .await?
            .ok_or_else(held)?;
        if observed != self.document {
            return Err(AppError::Unavailable(
                "configuration journal identity changed; preserved for reconciliation".into(),
            ));
        }
        Ok(())
    }

    pub(super) async fn verify_lock(&self) -> Result<(), AppError> {
        self.lock.verify().await
    }

    pub(super) async fn verified_backups(
        &self,
    ) -> Result<Vec<(PathBuf, Option<Vec<u8>>)>, AppError> {
        self.verify().await?;
        let mut backups = Vec::with_capacity(self.payload.files.len());
        for entry in &self.payload.files {
            if entry.path.as_os_str().is_empty()
                || entry
                    .path
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
                || entry.expected_sha256.as_ref().is_some_and(|hash| {
                    hash.len() != 64 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
                })
            {
                return Err(held());
            }
            let previous = entry
                .previous_hex
                .as_ref()
                .map(hex::decode)
                .transpose()
                .map_err(|_| held())?;
            backups.push((self.payload.config_directory.join(&entry.path), previous));
        }
        Ok(backups)
    }

    pub(super) async fn acknowledge(self) -> Result<(), AppError> {
        self.verify().await?;
        tokio::fs::remove_file(&self.path).await.map_err(|_| {
            AppError::Unavailable(
                "configuration journal acknowledgement requires reconciliation".into(),
            )
        })?;
        sync_directory(
            self.path
                .parent()
                .expect("journal has configuration parent"),
        )
        .await
    }
}

async fn sync_directory(path: &Path) -> Result<(), AppError> {
    #[cfg(unix)]
    {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || std::fs::File::open(path)?.sync_all())
            .await
            .map_err(|_| {
                AppError::Unavailable("configuration journal directory sync failed".into())
            })?
            .map_err(|_| {
                AppError::Unavailable("configuration journal directory sync failed".into())
            })?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    type Fixture = (
        PathBuf,
        PathBuf,
        Vec<(PathBuf, String)>,
        Vec<(PathBuf, Option<Vec<u8>>)>,
    );

    async fn fixture() -> Fixture {
        let root = std::env::temp_dir().join(format!("fleet-journal-test-{}", Uuid::new_v4()));
        let config = root.join("agents/agent1/config");
        tokio::fs::create_dir_all(&config).await.unwrap();
        let controller = root.join("controller");
        tokio::fs::create_dir(&controller).await.unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(&controller, std::fs::Permissions::from_mode(0o700))
                .await
                .unwrap();
        }
        let path = config.join(".env");
        tokio::fs::write(&path, b"old-secret").await.unwrap();
        (
            root,
            config,
            vec![(path.clone(), "new-secret".into())],
            vec![(path, Some(b"old-secret".to_vec()))],
        )
    }

    async fn prepare(
        root: &Path,
        config: &Path,
        id: Uuid,
        revision: i64,
        running: bool,
        files: &[(PathBuf, String)],
        backups: &[(PathBuf, Option<Vec<u8>>)],
    ) -> Result<ActivationJournal, AppError> {
        ActivationJournal::prepare(
            JournalLocation {
                agents_root: &root.join("agents"),
                config_directory: config,
                controller_root: &root.join("controller"),
            },
            identity(id, revision),
            running,
            files,
            backups,
            "journal-test-secret",
            super::super::activation_lock::ActivationLock::acquire(&root.join("controller"), id)
                .await?,
        )
        .await
    }

    fn identity(agent_id: Uuid, revision: i64) -> ActivationIdentity {
        ActivationIdentity {
            agent_id,
            revision,
            candidate_sha256: hex::encode(Sha256::digest(b"candidate")),
            effective_revision: None,
            effective_sha256: None,
            controller_id: Uuid::new_v4(),
            original_launch: None,
        }
    }

    async fn reopen(
        root: &Path,
        config: &Path,
        id: Uuid,
        secret: &str,
    ) -> Result<Option<ActivationJournal>, AppError> {
        ActivationJournal::reopen(
            JournalLocation {
                agents_root: &root.join("agents"),
                config_directory: config,
                controller_root: &root.join("controller"),
            },
            id,
            secret,
            super::super::activation_lock::ActivationLock::acquire(&root.join("controller"), id)
                .await?,
        )
        .await
    }

    #[tokio::test]
    async fn signed_reopen_recovers_original_identity_and_rejects_tamper_rotation_and_legacy() {
        let (root, config, files, backups) = fixture().await;
        let id = Uuid::new_v4();
        let journal = prepare(&root, &config, id, 1, false, &files, &backups)
            .await
            .unwrap();
        let controller_id = journal.identity().controller_id;
        let path = journal.path.clone();
        let original = journal.document.clone();
        drop(journal);
        assert!(reopen(&root, &config, id, "rotated-key").await.is_err());
        let recovered = reopen(&root, &config, id, "journal-test-secret")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(recovered.identity().controller_id, controller_id);
        assert_eq!(recovered.verified_backups().await.unwrap(), backups);
        recovered.verify_plan(&files).unwrap();
        assert!(
            recovered
                .verify_plan(&[(files[0].0.clone(), "different-candidate".into())])
                .is_err()
        );
        drop(recovered);
        let mut tampered: serde_json::Value = serde_json::from_slice(&original).unwrap();
        tampered["payload"]["files"][0]["previous_hex"] = serde_json::json!(hex::encode(b"forged"));
        tokio::fs::write(&path, serde_json::to_vec(&tampered).unwrap())
            .await
            .unwrap();
        assert!(
            reopen(&root, &config, id, "journal-test-secret")
                .await
                .is_err()
        );
        tokio::fs::write(&path, b"{\"contract_version\":2}")
            .await
            .unwrap();
        assert!(
            reopen(&root, &config, id, "journal-test-secret")
                .await
                .is_err()
        );
        assert_eq!(tokio::fs::read(&files[0].0).await.unwrap(), b"old-secret");
        tokio::fs::write(&path, original).await.unwrap();
        reopen(&root, &config, id, "journal-test-secret")
            .await
            .unwrap()
            .unwrap()
            .acknowledge()
            .await
            .unwrap();
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn reopen_accepts_bounded_journal_larger_than_individual_backup_read_limit() {
        let (root, config, files, _) = fixture().await;
        let id = Uuid::new_v4();
        let backups = vec![(files[0].0.clone(), Some(vec![b'a'; 5 * 1024 * 1024]))];
        let journal = prepare(&root, &config, id, 1, false, &files, &backups)
            .await
            .unwrap();
        assert!(journal.document.len() > MAX_BACKUP_BYTES);
        drop(journal);
        let recovered = reopen(&root, &config, id, "journal-test-secret")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(recovered.verified_backups().await.unwrap(), backups);
        recovered.acknowledge().await.unwrap();
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn journal_preserves_backup_and_exclusive_hold_until_acknowledgement() {
        let (root, config, files, backups) = fixture().await;
        let id = Uuid::new_v4();
        let journal = prepare(&root, &config, id, 1, false, &files, &backups)
            .await
            .unwrap();
        let payload: serde_json::Value =
            serde_json::from_slice(&tokio::fs::read(&journal.path).await.unwrap()).unwrap();
        let payload = &payload["payload"];
        assert_eq!(payload["contract_version"], 3);
        assert_eq!(
            payload["config_directory"],
            serde_json::json!(tokio::fs::canonicalize(&config).await.unwrap())
        );
        assert_eq!(
            payload["agents_root"],
            serde_json::json!(tokio::fs::canonicalize(root.join("agents")).await.unwrap())
        );
        assert_eq!(
            payload["files"][0]["previous_hex"],
            hex::encode(b"old-secret")
        );
        assert_eq!(
            payload["files"][0]["expected_sha256"],
            hex::encode(Sha256::digest(b"new-secret"))
        );
        assert!(
            prepare(&root, &config, id, 2, false, &files, &backups)
                .await
                .is_err()
        );
        assert_eq!(tokio::fs::read(&files[0].0).await.unwrap(), b"old-secret");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                tokio::fs::metadata(&journal.path)
                    .await
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        let journal_path = journal.path.clone();
        assert!(!config.join(JOURNAL_NAME).exists());
        assert!(journal_path.starts_with(root.join("controller")));
        journal.acknowledge().await.unwrap();
        assert!(!journal_path.exists());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn journal_retains_unknown_crash_and_changed_identity() {
        let (root, config, files, backups) = fixture().await;
        let id = Uuid::new_v4();
        let journal = prepare(&root, &config, id, 1, true, &files, &backups)
            .await
            .unwrap();
        let journal_path = journal.path.clone();
        tokio::fs::write(&journal_path, b"incomplete")
            .await
            .unwrap();
        assert!(journal.acknowledge().await.is_err());
        assert_eq!(tokio::fs::read(&journal_path).await.unwrap(), b"incomplete");
        assert!(
            prepare(&root, &config, id, 1, false, &files, &backups)
                .await
                .is_err()
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn journal_rejects_foreign_duplicate_traversal_and_oversized_backups() {
        let (root, config, files, backups) = fixture().await;
        let id = Uuid::new_v4();
        let outside = vec![(root.join("agent2/.env"), "other".into())];
        assert!(
            prepare(&root, &config, id, 1, false, &outside, &backups)
                .await
                .is_err()
        );
        let duplicate = vec![files[0].clone(), files[0].clone()];
        let duplicate_backups = vec![backups[0].clone(), backups[0].clone()];
        assert!(
            prepare(&root, &config, id, 1, false, &duplicate, &duplicate_backups)
                .await
                .is_err()
        );
        let alias = vec![files[0].clone(), (config.join(".ENV"), "other".into())];
        let alias_backups = vec![backups[0].clone(), (config.join(".ENV"), None)];
        assert!(
            prepare(&root, &config, id, 1, false, &alias, &alias_backups)
                .await
                .is_err()
        );
        let traversal = vec![(config.join("skills/../.env"), "other".into())];
        let traversal_backup = vec![(traversal[0].0.clone(), None)];
        assert!(
            prepare(&root, &config, id, 1, false, &traversal, &traversal_backup)
                .await
                .is_err()
        );
        let huge = vec![(backups[0].0.clone(), Some(vec![0; MAX_BACKUP_BYTES + 1]))];
        assert!(
            prepare(&root, &config, id, 1, false, &files, &huge)
                .await
                .is_err()
        );
        assert!(!config.join(JOURNAL_NAME).exists());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn journal_and_backup_reject_symlink_components() {
        let (root, config, files, backups) = fixture().await;
        std::os::unix::fs::symlink(&files[0].0, config.join(JOURNAL_NAME)).unwrap();
        assert!(
            prepare(&root, &config, Uuid::new_v4(), 1, false, &files, &backups)
                .await
                .is_err()
        );
        tokio::fs::remove_file(config.join(JOURNAL_NAME))
            .await
            .unwrap();
        std::os::unix::fs::symlink(&files[0].0, config.join("linked")).unwrap();
        assert!(read_backup(&root, &config.join("linked")).await.is_err());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn journal_drop_retains_backup_and_represents_absent_disabled_skill() {
        let (root, config, mut files, mut backups) = fixture().await;
        let skill = config.join("skills/disabled/SKILL.md");
        files.push((skill.clone(), String::new()));
        backups.push((skill, None));
        let id = Uuid::new_v4();
        let journal = prepare(&root, &config, id, 1, true, &files, &backups)
            .await
            .unwrap();
        let journal_path = journal.path.clone();
        drop(journal);
        let payload: serde_json::Value =
            serde_json::from_slice(&tokio::fs::read(&journal_path).await.unwrap()).unwrap();
        let payload = &payload["payload"];
        assert!(payload["was_running"].as_bool().unwrap());
        assert!(payload["files"][1]["previous_hex"].is_null());
        assert!(payload["files"][1]["expected_sha256"].is_null());
        assert!(
            prepare(&root, &config, id, 2, false, &files, &backups)
                .await
                .is_err()
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn backup_rejects_directories_and_oversized_regular_files_without_touching_them() {
        let (root, config, files, _) = fixture().await;
        assert!(read_backup(&root, &config).await.is_err());
        let large = tokio::fs::OpenOptions::new()
            .write(true)
            .open(&files[0].0)
            .await
            .unwrap();
        large.set_len((MAX_BACKUP_BYTES + 1) as u64).await.unwrap();
        drop(large);
        assert!(read_backup(&root, &files[0].0).await.is_err());
        assert_eq!(
            tokio::fs::metadata(&files[0].0).await.unwrap().len(),
            (MAX_BACKUP_BYTES + 1) as u64
        );
        assert!(
            read_backup(&root, &config.join("missing"))
                .await
                .unwrap()
                .is_none()
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn legacy_journal_blocks_without_moving_or_rewriting_sensitive_state() {
        let (root, config, files, backups) = fixture().await;
        tokio::fs::write(config.join(JOURNAL_NAME), b"legacy-unknown-secret")
            .await
            .unwrap();
        let id = Uuid::new_v4();
        assert!(
            prepare(&root, &config, id, 1, false, &files, &backups)
                .await
                .is_err()
        );
        assert_eq!(
            tokio::fs::read(config.join(JOURNAL_NAME)).await.unwrap(),
            b"legacy-unknown-secret"
        );
        assert!(!controller_journal_path(&root.join("controller"), id).exists());
        assert_eq!(tokio::fs::read(&files[0].0).await.unwrap(), b"old-secret");
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn missing_shared_overlapping_and_linked_controller_roots_are_denied() {
        use std::os::unix::fs::PermissionsExt;
        let (root, config, files, backups) = fixture().await;
        let id = Uuid::new_v4();
        let agents = root.join("agents");
        let controller = root.join("controller");
        let nested = agents.join("controller");
        tokio::fs::create_dir(&nested).await.unwrap();
        tokio::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o700))
            .await
            .unwrap();
        tokio::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
            .await
            .unwrap();
        std::os::unix::fs::symlink(&controller, root.join("alias")).unwrap();
        for bad in [
            PathBuf::new(),
            root.join("missing"),
            nested,
            root.clone(),
            root.join("alias"),
        ] {
            assert!(matches!(
                ActivationJournal::prepare(
                    JournalLocation {
                        agents_root: &agents,
                        config_directory: &config,
                        controller_root: &bad,
                    },
                    identity(id, 1),
                    false,
                    &files,
                    &backups,
                    "journal-test-secret",
                    super::super::activation_lock::ActivationLock::acquire(&controller, id)
                        .await
                        .unwrap(),
                )
                .await,
                Err(AppError::Unavailable(_))
            ));
        }
        for mode in [0o755, 0o770, 0o711] {
            tokio::fs::set_permissions(&controller, std::fs::Permissions::from_mode(mode))
                .await
                .unwrap();
            assert!(matches!(
                prepare(&root, &config, id, 1, false, &files, &backups).await,
                Err(AppError::Unavailable(_))
            ));
        }
        assert!(!controller_journal_path(&controller, id).exists());
        assert_eq!(tokio::fs::read(&files[0].0).await.unwrap(), b"old-secret");
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn acknowledgement_rejects_hardlinks_and_changed_directory_permissions() {
        use std::os::unix::fs::PermissionsExt;
        for hardlink in [true, false] {
            let (root, config, files, backups) = fixture().await;
            let journal = prepare(&root, &config, Uuid::new_v4(), 1, false, &files, &backups)
                .await
                .unwrap();
            let path = journal.path.clone();
            let original = tokio::fs::read(&path).await.unwrap();
            if hardlink {
                tokio::fs::hard_link(&path, root.join("foreign-link"))
                    .await
                    .unwrap();
            } else {
                tokio::fs::set_permissions(
                    root.join("controller"),
                    std::fs::Permissions::from_mode(0o755),
                )
                .await
                .unwrap();
            }
            assert!(journal.acknowledge().await.is_err());
            assert_eq!(tokio::fs::read(&path).await.unwrap(), original);
            tokio::fs::remove_dir_all(root).await.unwrap();
        }
    }

    #[tokio::test]
    async fn independent_agent_journals_are_isolated_in_controller_storage() {
        let (root, config, files, backups) = fixture().await;
        let first_id = Uuid::new_v4();
        let first = prepare(&root, &config, first_id, 1, false, &files, &backups)
            .await
            .unwrap();
        let other_config = root.join("agents/agent2/config");
        tokio::fs::create_dir_all(&other_config).await.unwrap();
        let other_path = other_config.join(".env");
        let second = prepare(
            &root,
            &other_config,
            Uuid::new_v4(),
            1,
            false,
            &[(other_path.clone(), "other-secret".into())],
            &[(other_path, None)],
        )
        .await
        .unwrap();
        let first_path = first.path.clone();
        let second_path = second.path.clone();
        first.acknowledge().await.unwrap();
        assert!(!first_path.exists());
        assert!(second_path.exists());
        assert!(!config.join(JOURNAL_NAME).exists());
        assert!(!other_config.join(JOURNAL_NAME).exists());
        second.acknowledge().await.unwrap();
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn rollback_requires_original_durable_document_and_held_lock() {
        let (root, config, files, backups) = fixture().await;
        let id = Uuid::new_v4();
        let journal = prepare(&root, &config, id, 1, false, &files, &backups)
            .await
            .unwrap();
        assert_eq!(journal.verified_backups().await.unwrap(), backups);
        assert!(
            super::super::activation_lock::ActivationLock::acquire(&root.join("controller"), id)
                .await
                .is_err()
        );
        let original = tokio::fs::read(&journal.path).await.unwrap();
        tokio::fs::write(&journal.path, b"changed-durable-backup")
            .await
            .unwrap();
        assert!(journal.verified_backups().await.is_err());
        assert_eq!(tokio::fs::read(&files[0].0).await.unwrap(), b"old-secret");
        tokio::fs::write(&journal.path, original).await.unwrap();
        assert_eq!(journal.verified_backups().await.unwrap(), backups);
        journal.acknowledge().await.unwrap();
        let successor =
            super::super::activation_lock::ActivationLock::acquire(&root.join("controller"), id)
                .await
                .unwrap();
        drop(successor);
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
