use shared::AppError;
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn held() -> AppError {
    AppError::Unavailable("configuration activation ownership requires reconciliation".into())
}

// Keep the descriptor through DB settlement; never unlink the lock's inode.
pub(super) struct ActivationLock {
    root: PathBuf,
    path: PathBuf,
    file: std::fs::File,
}

impl ActivationLock {
    pub(super) async fn acquire(root: &Path, agent_id: Uuid) -> Result<Self, AppError> {
        if agent_id.is_nil() {
            return Err(AppError::validation(
                "activation lock requires a concrete agent",
            ));
        }
        let root = super::activation_journal::private_controller_directory(root).await?;
        #[cfg(target_os = "linux")]
        {
            use rustix::fs::{FlockOperation, Mode, OFlags, flock, open};
            let path = root.join(format!("{agent_id}.activation.lock"));
            let descriptor = open(
                &path,
                OFlags::RDWR | OFlags::CREATE | OFlags::CLOEXEC | OFlags::NOFOLLOW,
                Mode::RUSR | Mode::WUSR,
            )
            .map_err(|_| held())?;
            let lock = Self {
                root,
                path,
                file: descriptor.into(),
            };
            lock.verify().await?;
            flock(&lock.file, FlockOperation::NonBlockingLockExclusive).map_err(|_| held())?;
            lock.verify().await?;
            lock.file.sync_all().map_err(|_| held())?;
            std::fs::File::open(&lock.root)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| held())?;
            Ok(lock)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = root;
            Err(held())
        }
    }

    pub(super) async fn verify(&self) -> Result<(), AppError> {
        if super::activation_journal::private_controller_directory(&self.root).await? != self.root {
            return Err(held());
        }
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::MetadataExt;
            let owner = std::fs::metadata(&self.root).map_err(|_| held())?.uid();
            let opened = self.file.metadata().map_err(|_| held())?;
            let named = std::fs::symlink_metadata(&self.path).map_err(|_| held())?;
            for metadata in [&opened, &named] {
                if !metadata.is_file()
                    || metadata.uid() != owner
                    || metadata.nlink() != 1
                    || metadata.mode() & 0o777 != 0o600
                    || metadata.len() != 0
                {
                    return Err(held());
                }
            }
            if opened.dev() != named.dev() || opened.ino() != named.ino() {
                return Err(held());
            }
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        Err(held())
    }

    pub(super) async fn verify_for(&self, root: &Path, agent_id: Uuid) -> Result<(), AppError> {
        if self.root != root || self.path != root.join(format!("{agent_id}.activation.lock")) {
            return Err(held());
        }
        self.verify().await
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tokio::{io::AsyncBufReadExt, time::Duration};

    async fn fixture() -> PathBuf {
        let root = std::env::temp_dir().join(format!("fleet-activation-lock-{}", Uuid::new_v4()));
        tokio::fs::create_dir(&root).await.unwrap();
        tokio::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
            .await
            .unwrap();
        root
    }

    #[tokio::test]
    async fn exclusive_until_drop_and_independent_per_agent() {
        let root = fixture().await;
        let agent = Uuid::new_v4();
        let first = ActivationLock::acquire(&root, agent).await.unwrap();
        assert!(ActivationLock::acquire(&root, agent).await.is_err());
        let other = ActivationLock::acquire(&root, Uuid::new_v4())
            .await
            .unwrap();
        let path = first.path.clone();
        drop(first);
        assert!(path.exists());
        let successor = ActivationLock::acquire(&root, agent).await.unwrap();
        drop((other, successor));
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn rejects_replacement_symlink_hardlink_permissions_and_content() {
        let root = fixture().await;
        let agent = Uuid::new_v4();
        let lock = ActivationLock::acquire(&root, agent).await.unwrap();
        let path = lock.path.clone();
        let displaced = root.join("displaced.lock");
        tokio::fs::rename(&path, &displaced).await.unwrap();
        tokio::fs::write(&path, b"").await.unwrap();
        tokio::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .await
            .unwrap();
        assert!(lock.verify().await.is_err());
        tokio::fs::remove_file(&path).await.unwrap();
        std::os::unix::fs::symlink(&displaced, &path).unwrap();
        assert!(ActivationLock::acquire(&root, agent).await.is_err());
        tokio::fs::remove_file(&path).await.unwrap();
        tokio::fs::hard_link(&displaced, &path).await.unwrap();
        assert!(ActivationLock::acquire(&root, agent).await.is_err());
        tokio::fs::remove_file(&path).await.unwrap();
        tokio::fs::rename(&displaced, &path).await.unwrap();
        tokio::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
            .await
            .unwrap();
        assert!(lock.verify().await.is_err());
        assert!(ActivationLock::acquire(&root, agent).await.is_err());
        tokio::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .await
            .unwrap();
        tokio::fs::write(&path, b"foreign").await.unwrap();
        assert!(lock.verify().await.is_err());
        assert!(ActivationLock::acquire(&root, agent).await.is_err());
        assert_eq!(tokio::fs::read(&path).await.unwrap(), b"foreign");
        drop(lock);
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[test]
    fn child_lock_holder() {
        let Some(root) = std::env::var_os("FLEET_ACTIVATION_LOCK_TEST_ROOT") else {
            return;
        };
        let agent =
            Uuid::parse_str(&std::env::var("FLEET_ACTIVATION_LOCK_TEST_AGENT").unwrap()).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let _lock = runtime
            .block_on(ActivationLock::acquire(Path::new(&root), agent))
            .unwrap();
        use std::io::{Read, Write};
        println!("ACTIVATION_LOCK_HELD");
        std::io::stdout().flush().unwrap();
        let _ = std::io::stdin().read(&mut [0u8; 1]);
    }

    #[tokio::test]
    async fn actual_process_death_releases_lock_but_not_journal() {
        let root = fixture().await;
        let agent = Uuid::new_v4();
        let journal = super::super::activation_journal::controller_journal_path(&root, agent);
        tokio::fs::write(&journal, b"retained-original-journal")
            .await
            .unwrap();
        let mut child = tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "runtime::activation_lock::tests::child_lock_holder",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("FLEET_ACTIVATION_LOCK_TEST_ROOT", &root)
            .env("FLEET_ACTIVATION_LOCK_TEST_AGENT", agent.to_string())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let mut lines = tokio::io::BufReader::new(child.stdout.take().unwrap()).lines();
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let line = lines
                    .next_line()
                    .await
                    .unwrap()
                    .expect("child exited early");
                if line.contains("ACTIVATION_LOCK_HELD") {
                    break;
                }
            }
        })
        .await
        .unwrap();
        assert!(ActivationLock::acquire(&root, agent).await.is_err());
        child.kill().await.unwrap();
        let exit = tokio::time::timeout(Duration::from_secs(10), child.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(!exit.success());
        let successor = ActivationLock::acquire(&root, agent).await.unwrap();
        assert_eq!(
            tokio::fs::read(&journal).await.unwrap(),
            b"retained-original-journal"
        );
        drop(successor);
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
