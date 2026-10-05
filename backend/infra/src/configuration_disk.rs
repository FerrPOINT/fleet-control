use shared::AppError;
use std::{io::ErrorKind, path::Path};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

#[cfg(test)]
tokio::task_local! {
    pub(crate) static SYNC_FAILURE: (std::path::PathBuf, std::cell::Cell<usize>);
}

fn uncertain() -> AppError {
    AppError::Unavailable(
        "configuration filesystem persistence is unconfirmed; reconciliation is required".into(),
    )
}

pub(crate) async fn sync_directory(path: &Path) -> Result<(), AppError> {
    if !tokio::fs::symlink_metadata(path)
        .await
        .map_err(|_| uncertain())?
        .is_dir()
    {
        return Err(uncertain());
    }
    #[cfg(test)]
    if SYNC_FAILURE
        .try_with(|(directory, remaining)| {
            if directory != path {
                return false;
            }
            let before = remaining.get();
            remaining.set(before.saturating_sub(1));
            before == 0
        })
        .unwrap_or(false)
    {
        return Err(uncertain());
    }
    #[cfg(unix)]
    {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || std::fs::File::open(path)?.sync_all())
            .await
            .map_err(|_| uncertain())?
            .map_err(|_| uncertain())?;
    }
    // Windows directory durability is not certified; Linux remains the release gate.
    Ok(())
}

pub(crate) async fn create_directory(root: &Path, path: &Path) -> Result<(), AppError> {
    crate::reject_symlink_components(root, path).await?;
    if !path.starts_with(root) {
        return Err(AppError::validation(
            "configuration directory escapes agents root",
        ));
    }
    if !tokio::fs::symlink_metadata(root)
        .await
        .map_err(|_| uncertain())?
        .is_dir()
    {
        return Err(uncertain());
    }
    tokio::fs::create_dir_all(path)
        .await
        .map_err(|_| uncertain())?;
    crate::reject_symlink_components(root, path).await?;
    // A new nested skill directory needs each ancestor entry persisted, leaf to root.
    let mut current = path;
    loop {
        sync_directory(current).await?;
        if current == root {
            return Ok(());
        }
        current = current.parent().ok_or_else(uncertain)?;
    }
}

pub(crate) async fn write(path: &Path, body: &[u8]) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(uncertain)?;
    sync_directory(parent).await?;
    let temporary = path.with_file_name(format!(".fleet-next-{}", Uuid::new_v4()));
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&temporary).await.map_err(|_| uncertain())?;
    file.write_all(body).await.map_err(|_| uncertain())?;
    file.sync_all().await.map_err(|_| uncertain())?;
    drop(file);
    tokio::fs::rename(&temporary, path)
        .await
        .map_err(|_| uncertain())?;
    // File fsync does not make rename durable. Do not acknowledge DB state first.
    sync_directory(parent).await
}

pub(crate) async fn remove(root: &Path, path: &Path) -> Result<(), AppError> {
    crate::reject_symlink_components(root, path).await?;
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) if metadata.is_file() => {}
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        _ => return Err(uncertain()),
    }
    let parent = path.parent().ok_or_else(uncertain)?;
    sync_directory(parent).await?;
    tokio::fs::remove_file(path)
        .await
        .map_err(|_| uncertain())?;
    sync_directory(parent).await
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fixture() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("fleet-durable-config-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&root).await.unwrap();
        root
    }

    #[tokio::test]
    async fn nested_configuration_creation_write_replace_and_remove() {
        let root = fixture().await;
        let directory = root.join("agent1/config/skills/developer");
        create_directory(&root, &directory).await.unwrap();
        let path = directory.join("SKILL.md");
        write(&path, b"first revision").await.unwrap();
        write(&path, b"second revision").await.unwrap();
        assert_eq!(tokio::fs::read(&path).await.unwrap(), b"second revision");
        assert_eq!(
            tokio::fs::read_dir(&directory)
                .await
                .unwrap()
                .next_entry()
                .await
                .unwrap()
                .unwrap()
                .file_name(),
            "SKILL.md"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                tokio::fs::metadata(&path)
                    .await
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        remove(&root, &path).await.unwrap();
        remove(&root, &path).await.unwrap();
        assert!(!path.exists());
        assert!(
            tokio::fs::read_dir(&directory)
                .await
                .unwrap()
                .next_entry()
                .await
                .unwrap()
                .is_none()
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn removal_does_not_delete_a_directory_or_escape_the_root() {
        let root = fixture().await;
        let directory = root.join("config");
        create_directory(&root, &directory).await.unwrap();
        assert!(matches!(
            remove(&root, &directory).await,
            Err(AppError::Unavailable(_))
        ));
        let outside = root.with_extension("outside");
        assert!(remove(&root, &outside).await.is_err());
        assert!(create_directory(&root, &outside).await.is_err());
        assert!(!outside.exists());
        let missing_root = root.join("missing-root");
        assert!(
            create_directory(&missing_root, &missing_root.join("config"))
                .await
                .is_err()
        );
        assert!(!missing_root.exists());
        assert!(directory.is_dir());
        remove(&root, &directory.join("missing/SKILL.md"))
            .await
            .unwrap();
        assert!(!directory.join("missing").exists());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn invalid_parent_or_rename_preserves_existing_content_and_reports_uncertainty() {
        let root = fixture().await;
        let directory = root.join("target");
        create_directory(&root, &directory).await.unwrap();
        let error = write(&directory, b"private configuration value")
            .await
            .unwrap_err();
        assert!(matches!(error, AppError::Unavailable(_)));
        assert!(!error.to_string().contains("private configuration value"));
        assert!(directory.is_dir());
        let existing = root.join("existing");
        write(&existing, b"last working configuration")
            .await
            .unwrap();
        assert!(matches!(
            write(&existing.join("child"), b"new").await,
            Err(AppError::Unavailable(_))
        ));
        assert_eq!(
            tokio::fs::read(&existing).await.unwrap(),
            b"last working configuration"
        );
        assert!(matches!(
            write(&root.join("missing/child"), b"new").await,
            Err(AppError::Unavailable(_))
        ));
        assert!(!root.join("missing").exists());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn visible_rename_is_not_acknowledged_when_the_directory_barrier_fails() {
        let root = fixture().await;
        let path = root.join("SOUL.md");
        write(&path, b"previous").await.unwrap();
        let result = SYNC_FAILURE
            .scope(
                (root.clone(), std::cell::Cell::new(1)),
                write(&path, b"next"),
            )
            .await;
        assert!(matches!(result, Err(AppError::Unavailable(_))));
        assert_eq!(tokio::fs::read(&path).await.unwrap(), b"next");
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn visible_unlink_is_not_acknowledged_when_the_directory_barrier_fails() {
        let root = fixture().await;
        let path = root.join("SKILL.md");
        write(&path, b"enabled skill").await.unwrap();
        let result = SYNC_FAILURE
            .scope(
                (root.clone(), std::cell::Cell::new(1)),
                remove(&root, &path),
            )
            .await;
        assert!(matches!(result, Err(AppError::Unavailable(_))));
        assert!(!path.exists());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn directory_sync_and_managed_removal_reject_symlink_aliases() {
        let root = fixture().await;
        let real = root.join("real");
        create_directory(&root, &real).await.unwrap();
        write(&real.join("secret"), b"original").await.unwrap();
        let alias = root.join("alias");
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        assert!(sync_directory(&alias).await.is_err());
        assert!(
            create_directory(&root, &alias.join("nested"))
                .await
                .is_err()
        );
        assert!(remove(&root, &alias.join("secret")).await.is_err());
        assert!(write(&alias.join("secret"), b"replacement").await.is_err());
        assert_eq!(
            tokio::fs::read(real.join("secret")).await.unwrap(),
            b"original"
        );
        let file_alias = root.join("file-alias");
        std::os::unix::fs::symlink(real.join("secret"), &file_alias).unwrap();
        assert!(remove(&root, &file_alias).await.is_err());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
