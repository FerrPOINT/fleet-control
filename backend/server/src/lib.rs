use std::{future::IntoFuture, sync::Arc};

use app::AppContext;
use infra::{FilesystemProvisioner, PostgresFleetRepository, connect_database, run_migrations};
use shared::AppConfig;
use tokio::sync::oneshot;
use tracing::{error, warn};

const SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

pub async fn run(
    config: Arc<AppConfig>,
    ready: oneshot::Sender<std::net::SocketAddr>,
    shutdown: oneshot::Receiver<()>,
) {
    run_migrations(config.database.clone())
        .await
        .expect("failed to run migrations");
    let db = connect_database(config.database.clone())
        .await
        .expect("failed to connect database");
    let repo = Arc::new(PostgresFleetRepository::new(db));
    let provisioner = Arc::new(FilesystemProvisioner);
    let (events, _) = tokio::sync::broadcast::channel(256);
    let runtime = Arc::new(infra::runtime::LocalRuntimeSupervisor::new(
        config.clone(),
        repo.clone(),
        events.clone(),
    ));
    let ctx = Arc::new(AppContext::new(
        config.clone(),
        repo.clone(),
        provisioner,
        runtime,
        events,
    ));
    if let Err(err) = ctx.ensure_seed_agents().await {
        warn!("failed to seed default agents: {err}");
    }

    let address = config.server_addr();
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind server");
    let bound_addr = listener.local_addr().expect("local addr");
    let _ = ready.send(bound_addr);

    let (shutdown_started_tx, shutdown_started_rx) = tokio::sync::oneshot::channel();
    let server = axum::serve(listener, api::router(ctx.clone()).with_state(ctx))
        .with_graceful_shutdown(async move {
            let _ = shutdown.await;
            let _ = shutdown_started_tx.send(());
        })
        .into_future();
    tokio::pin!(server);

    tokio::select! {
        result = &mut server => {
            if let Err(err) = result {
                error!("server error: {err}");
            }
        }
        _ = shutdown_started_rx => {
            if tokio::time::timeout(SHUTDOWN_TIMEOUT, &mut server).await.is_err() {
                warn!("graceful shutdown exceeded {SHUTDOWN_TIMEOUT:?}; dropping active connections");
            }
        }
    }
}
