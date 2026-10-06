use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared::AppError;
use std::{io::ErrorKind, path::Path};
use tokio::io::AsyncReadExt;

const MAX_DOTENV_BYTES: u64 = 64 * 1024;

// Private launch input, not proof of dotenv expansion or external secret loading.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ContainerEnvironmentSnapshot {
    version: u8,
    dotenv: Option<String>,
    dotenv_sha256: Option<String>,
}

fn held() -> AppError {
    AppError::Unavailable("original container environment requires reconciliation".into())
}

pub(super) async fn capture(
    agents_root: &Path,
    config_root: &Path,
    expected: Option<&str>,
) -> Result<ContainerEnvironmentSnapshot, AppError> {
    let path = config_root.join(".env");
    crate::reject_symlink_components(agents_root, &path)
        .await
        .map_err(|_| held())?;
    let metadata = match tokio::fs::symlink_metadata(&path).await {
        Ok(metadata) => Some(metadata),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(_) => return Err(held()),
    };
    let Some(metadata) = metadata else {
        if expected.is_some() {
            return Err(held());
        }
        return Ok(ContainerEnvironmentSnapshot {
            version: 1,
            dotenv: None,
            dotenv_sha256: None,
        });
    };
    if !metadata.is_file() || metadata.len() > MAX_DOTENV_BYTES {
        return Err(held());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1 {
            return Err(held());
        }
    }
    let file = tokio::fs::File::open(&path).await.map_err(|_| held())?;
    let mut bytes = Vec::new();
    file.take(MAX_DOTENV_BYTES + 1)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| held())?;
    if bytes.len() as u64 != metadata.len()
        || bytes.len() as u64 > MAX_DOTENV_BYTES
        || expected.is_some_and(|expected| bytes != expected.as_bytes())
    {
        return Err(held());
    }
    let dotenv_sha256 = hex::encode(Sha256::digest(&bytes));
    let dotenv = String::from_utf8(bytes).map_err(|_| held())?;
    Ok(ContainerEnvironmentSnapshot {
        version: 1,
        dotenv: Some(dotenv),
        dotenv_sha256: Some(dotenv_sha256),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fixture() -> std::path::PathBuf {
        let root =
            std::env::temp_dir().join(format!("fleet-container-env-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir_all(root.join("agent1/config"))
            .await
            .unwrap();
        root
    }

    #[tokio::test]
    async fn exact_bytes_are_frozen_without_interpreting_native_dotenv() {
        let root = fixture().await;
        let config = root.join("agent1/config");
        let body = "TOKEN=\"old-${OTHER}-\\n-credential\"\n";
        tokio::fs::write(config.join(".env"), body).await.unwrap();
        let snapshot = capture(&root, &config, Some(body)).await.unwrap();
        assert_eq!(snapshot.dotenv.as_deref(), Some(body));
        assert_eq!(
            snapshot.dotenv_sha256,
            Some(hex::encode(Sha256::digest(body.as_bytes())))
        );
        tokio::fs::write(config.join(".env"), "TOKEN=rotated\n")
            .await
            .unwrap();
        assert!(capture(&root, &config, Some(body)).await.is_err());
        assert_eq!(snapshot.dotenv.as_deref(), Some(body));
        let value = serde_json::to_value(&snapshot).unwrap();
        let restored: ContainerEnvironmentSnapshot = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), value);
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn absent_empty_invalid_and_oversized_inputs_are_distinct() {
        let root = fixture().await;
        let config = root.join("agent1/config");
        assert!(capture(&root, &config, Some("")).await.is_err());
        let absent = capture(&root, &config, None).await.unwrap();
        assert!(absent.dotenv.is_none());
        tokio::fs::write(config.join(".env"), "").await.unwrap();
        let empty = capture(&root, &config, Some("")).await.unwrap();
        assert_eq!(empty.dotenv.as_deref(), Some(""));
        assert_ne!(
            serde_json::to_value(absent).unwrap(),
            serde_json::to_value(empty).unwrap()
        );
        for bytes in [vec![0xff], vec![b'x'; MAX_DOTENV_BYTES as usize + 1]] {
            tokio::fs::write(config.join(".env"), bytes).await.unwrap();
            assert!(capture(&root, &config, None).await.is_err());
        }
        tokio::fs::remove_file(config.join(".env")).await.unwrap();
        tokio::fs::create_dir(config.join(".env")).await.unwrap();
        assert!(capture(&root, &config, None).await.is_err());
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn symlink_hardlink_and_foreign_root_are_rejected() {
        let root = fixture().await;
        let config = root.join("agent1/config");
        let foreign = root.with_extension("foreign");
        tokio::fs::write(&foreign, "TOKEN=foreign").await.unwrap();
        std::os::unix::fs::symlink(&foreign, config.join(".env")).unwrap();
        assert!(capture(&root, &config, None).await.is_err());
        tokio::fs::remove_file(config.join(".env")).await.unwrap();
        tokio::fs::hard_link(&foreign, config.join(".env"))
            .await
            .unwrap();
        assert!(capture(&root, &config, None).await.is_err());
        assert!(
            capture(&root, foreign.parent().unwrap(), None)
                .await
                .is_err()
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
        tokio::fs::remove_file(foreign).await.unwrap();
    }
}
