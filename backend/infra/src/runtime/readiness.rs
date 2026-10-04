use shared::AppError;
use std::{future::Future, time::Duration};
use tokio::time::{Instant, sleep, timeout_at};

pub(super) async fn wait<T, F, Fut>(
    name: &str,
    timeout: Duration,
    poll: Duration,
    mut probe: F,
) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, AppError>>,
{
    let deadline = Instant::now() + timeout;
    let mut last_error = "readiness probe did not run".to_string();
    // One absolute deadline includes probe IO and polling delays, not just sleeps.
    let result = timeout_at(deadline, async {
        loop {
            if Instant::now() >= deadline {
                return None;
            }
            match probe().await {
                Ok(value) => return Some(value),
                Err(error) => last_error = crate::redact_text(&error.to_string()),
            }
            sleep(poll).await;
        }
    })
    .await;
    match result {
        Ok(Some(value)) if Instant::now() < deadline => Ok(value),
        _ => Err(AppError::validation(format!(
            "{} did not become ready: {last_error}",
            crate::redact_text(name)
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[tokio::test]
    async fn hung_probe_is_cancelled_at_the_absolute_deadline() {
        let started = Instant::now();
        let calls = AtomicUsize::new(0);
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            wait(
                "test",
                Duration::from_millis(60),
                Duration::from_millis(5),
                || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    std::future::pending::<Result<(), AppError>>()
                },
            ),
        )
        .await
        .expect("hung probe escaped the readiness deadline");
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(started.elapsed() >= Duration::from_millis(60));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn slow_probes_and_polling_share_one_budget_and_preserve_the_last_error() {
        let started = Instant::now();
        let calls = Arc::new(AtomicUsize::new(0));
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            wait(
                "test",
                Duration::from_millis(180),
                Duration::from_millis(20),
                || {
                    let attempt = calls.fetch_add(1, Ordering::SeqCst) + 1;
                    async move {
                        sleep(Duration::from_millis(100)).await;
                        Err::<(), _>(AppError::validation(format!(
                            "attempt {attempt} token=fixture-secret"
                        )))
                    }
                },
            ),
        )
        .await
        .expect("slow probes escaped the readiness deadline");
        let detail = result.unwrap_err().to_string();
        assert!(detail.contains("attempt 1"));
        assert!(!detail.contains("fixture-secret"));
        // At most a second probe starts; its IO must be cancelled inside the same budget.
        let count = calls.load(Ordering::SeqCst);
        assert!((1..=2).contains(&count));
        assert!(started.elapsed() >= Duration::from_millis(180));
        assert!(started.elapsed() < Duration::from_secs(1));
        sleep(Duration::from_millis(50)).await;
        assert_eq!(calls.load(Ordering::SeqCst), count);
    }

    #[tokio::test]
    async fn immediate_success_returns_the_value_without_polling() {
        let calls = AtomicUsize::new(0);
        let value = wait(
            "test",
            Duration::from_secs(1),
            Duration::from_secs(5),
            || {
                calls.fetch_add(1, Ordering::SeqCst);
                std::future::ready(Ok::<_, AppError>(42))
            },
        )
        .await
        .unwrap();
        assert_eq!(value, 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn elapsed_deadline_never_invokes_a_probe() {
        let calls = AtomicUsize::new(0);
        let result = wait("test", Duration::ZERO, Duration::ZERO, || {
            calls.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Ok::<_, AppError>(()))
        })
        .await;
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn polling_sleep_cannot_outlive_the_deadline() {
        let calls = AtomicUsize::new(0);
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            wait(
                "test",
                Duration::from_millis(60),
                Duration::from_secs(10),
                || {
                    calls.fetch_add(1, Ordering::SeqCst);
                    std::future::ready(Err::<(), _>(AppError::validation("last probe failed")))
                },
            ),
        )
        .await
        .expect("polling sleep escaped the readiness deadline");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("last probe failed")
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
