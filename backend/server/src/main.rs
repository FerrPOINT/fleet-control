use std::sync::Arc;

use server::run;
use shared::AppConfig;

#[tokio::main]
async fn main() {
    shared::telemetry::init_tracing("fleet-control");

    let config = Arc::new(AppConfig::from_env().expect("failed to load config"));
    let (ready_tx, _ready_rx) = tokio::sync::oneshot::channel();
    let (_shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    run(config, ready_tx, shutdown_rx).await;
}
