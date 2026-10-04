use shared::AppError;
use std::time::Duration;
use tokio::process::Child;

pub(super) async fn terminate(child: &mut Child) -> Result<(), AppError> {
    if child.try_wait().map_err(|_| unknown())?.is_some() {
        return Ok(());
    }
    let killed = tokio::time::timeout(Duration::from_secs(10), child.kill()).await;
    // Even an error may race a natural exit. Only wait readback acknowledges termination.
    if matches!(killed, Ok(Ok(()))) || child.try_wait().map_err(|_| unknown())?.is_some() {
        Ok(())
    } else {
        Err(unknown())
    }
}

fn unknown() -> AppError {
    AppError::Unavailable(
        "runtime process termination is unconfirmed; reconciliation is required".into(),
    )
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use tokio::process::Command;

    #[tokio::test]
    async fn termination_requires_wait_and_is_replayable_after_exit() {
        let mut child = Command::new("sleep")
            .arg("30")
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        assert!(child.try_wait().unwrap().is_none());
        terminate(&mut child).await.unwrap();
        assert!(child.try_wait().unwrap().is_some());
        terminate(&mut child).await.unwrap();
    }

    #[tokio::test]
    async fn natural_exit_is_confirmed_without_another_signal() {
        let mut child = Command::new("sleep")
            .arg("0")
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        child.wait().await.unwrap();
        terminate(&mut child).await.unwrap();
        assert!(child.id().is_none());
    }
}
