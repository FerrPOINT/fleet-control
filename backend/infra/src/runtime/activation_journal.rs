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

// Contains resolved env secrets: never implement Debug or expose the document through API.
pub(super) struct ActivationJournal {
    root: PathBuf,
    path: PathBuf,
    document: Vec<u8>,
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
        root: &Path,
        config_directory: &Path,
        agent_id: Uuid,
        revision: i64,
        was_running: bool,
        files: &[(PathBuf, String)],
        backups: &[(PathBuf, Option<Vec<u8>>)],
    ) -> Result<Self, AppError> {
        if revision <= 0
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
            entries.push(serde_json::json!({
                "path": relative,
                "previous_hex": previous.as_ref().map(hex::encode),
                "expected_sha256": if absent { None } else { Some(hex::encode(Sha256::digest(body.as_bytes()))) },
            }));
        }
        let document = serde_json::to_vec(&serde_json::json!({
            "contract_version": 1, "agent_id": agent_id, "revision": revision,
            "was_running": was_running, "files": entries,
        }))
        .map_err(|_| AppError::internal("configuration journal serialization failed"))?;
        if document.len() > MAX_JOURNAL_BYTES {
            return Err(AppError::validation(
                "configuration journal exceeds its byte limit",
            ));
        }
        let path = config_directory.join(JOURNAL_NAME);
        crate::reject_symlink_components(root, &path).await?;
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
        sync_directory(config_directory).await?;
        Ok(Self {
            root: root.to_path_buf(),
            path,
            document,
        })
    }

    pub(super) async fn acknowledge(self) -> Result<(), AppError> {
        crate::reject_symlink_components(&self.root, &self.path).await?;
        if !tokio::fs::symlink_metadata(&self.path)
            .await
            .map_err(|_| {
                AppError::Unavailable(
                    "configuration journal acknowledgement requires reconciliation".into(),
                )
            })?
            .is_file()
        {
            return Err(AppError::Unavailable(
                "configuration journal is not a regular file".into(),
            ));
        }
        let file = tokio::fs::File::open(&self.path).await.map_err(|_| {
            AppError::Unavailable(
                "configuration journal acknowledgement requires reconciliation".into(),
            )
        })?;
        let mut observed = Vec::new();
        file.take((MAX_JOURNAL_BYTES + 1) as u64)
            .read_to_end(&mut observed)
            .await
            .map_err(|_| AppError::Unavailable("configuration journal readback failed".into()))?;
        if observed != self.document {
            return Err(AppError::Unavailable(
                "configuration journal identity changed; preserved for reconciliation".into(),
            ));
        }
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
        let config = root.join("agent1/config");
        tokio::fs::create_dir_all(&config).await.unwrap();
        let path = config.join(".env");
        tokio::fs::write(&path, b"old-secret").await.unwrap();
        (
            root,
            config,
            vec![(path.clone(), "new-secret".into())],
            vec![(path, Some(b"old-secret".to_vec()))],
        )
    }

    #[tokio::test]
    async fn journal_preserves_backup_and_exclusive_hold_until_acknowledgement() {
        let (root, config, files, backups) = fixture().await;
        let journal =
            ActivationJournal::prepare(&root, &config, Uuid::new_v4(), 1, false, &files, &backups)
                .await
                .unwrap();
        let payload: serde_json::Value =
            serde_json::from_slice(&tokio::fs::read(config.join(JOURNAL_NAME)).await.unwrap())
                .unwrap();
        assert_eq!(
            payload["files"][0]["previous_hex"],
            hex::encode(b"old-secret")
        );
        assert_eq!(
            payload["files"][0]["expected_sha256"],
            hex::encode(Sha256::digest(b"new-secret"))
        );
        assert!(
            ActivationJournal::prepare(&root, &config, Uuid::new_v4(), 2, false, &files, &backups)
                .await
                .is_err()
        );
        assert_eq!(tokio::fs::read(&files[0].0).await.unwrap(), b"old-secret");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                tokio::fs::metadata(config.join(JOURNAL_NAME))
                    .await
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        journal.acknowledge().await.unwrap();
        assert!(!config.join(JOURNAL_NAME).exists());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn journal_retains_unknown_crash_and_changed_identity() {
        let (root, config, files, backups) = fixture().await;
        let journal =
            ActivationJournal::prepare(&root, &config, Uuid::new_v4(), 1, true, &files, &backups)
                .await
                .unwrap();
        tokio::fs::write(config.join(JOURNAL_NAME), b"incomplete")
            .await
            .unwrap();
        assert!(journal.acknowledge().await.is_err());
        assert_eq!(
            tokio::fs::read(config.join(JOURNAL_NAME)).await.unwrap(),
            b"incomplete"
        );
        assert!(
            ActivationJournal::prepare(&root, &config, Uuid::new_v4(), 1, false, &files, &backups)
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
            ActivationJournal::prepare(&root, &config, id, 1, false, &outside, &backups)
                .await
                .is_err()
        );
        let duplicate = vec![files[0].clone(), files[0].clone()];
        let duplicate_backups = vec![backups[0].clone(), backups[0].clone()];
        assert!(
            ActivationJournal::prepare(
                &root,
                &config,
                id,
                1,
                false,
                &duplicate,
                &duplicate_backups
            )
            .await
            .is_err()
        );
        let alias = vec![files[0].clone(), (config.join(".ENV"), "other".into())];
        let alias_backups = vec![backups[0].clone(), (config.join(".ENV"), None)];
        assert!(
            ActivationJournal::prepare(&root, &config, id, 1, false, &alias, &alias_backups)
                .await
                .is_err()
        );
        let traversal = vec![(config.join("skills/../.env"), "other".into())];
        let traversal_backup = vec![(traversal[0].0.clone(), None)];
        assert!(
            ActivationJournal::prepare(&root, &config, id, 1, false, &traversal, &traversal_backup)
                .await
                .is_err()
        );
        let huge = vec![(backups[0].0.clone(), Some(vec![0; MAX_BACKUP_BYTES + 1]))];
        assert!(
            ActivationJournal::prepare(&root, &config, id, 1, false, &files, &huge)
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
            ActivationJournal::prepare(&root, &config, Uuid::new_v4(), 1, false, &files, &backups)
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
        let journal =
            ActivationJournal::prepare(&root, &config, Uuid::new_v4(), 1, true, &files, &backups)
                .await
                .unwrap();
        drop(journal);
        let payload: serde_json::Value =
            serde_json::from_slice(&tokio::fs::read(config.join(JOURNAL_NAME)).await.unwrap())
                .unwrap();
        assert!(payload["was_running"].as_bool().unwrap());
        assert!(payload["files"][1]["previous_hex"].is_null());
        assert!(payload["files"][1]["expected_sha256"].is_null());
        assert!(
            ActivationJournal::prepare(&root, &config, Uuid::new_v4(), 2, false, &files, &backups)
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
}
