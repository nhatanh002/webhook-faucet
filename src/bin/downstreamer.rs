use anyhow::{Context, Result};
use lockfile::Lockfile;
use once_cell::sync::Lazy;
use tokio::signal;
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};
use webhook_svc::*;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cnf = Lazy::force(config::get());
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::CLOSE))
        .init();
    let _lockfile = Lockfile::create(&cnf.bg_worker_lockfile)?;

    let token = tokio_util::sync::CancellationToken::new();
    let worker_token = token.clone();

    let redis_cli =
        redis::Client::open(cnf.redis_url.as_str()).context("failed to init redis client")?;

    let redis_conn = redis::aio::ConnectionManager::new(redis_cli)
        .await
        .context("failed to connect to redis")?;

    let worker = app::BgWorker::new(
        redis_conn,
        worker_token,
        reqwest::ClientBuilder::new()
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .build()?,
    );

    tracing::info!("starting background jobs");
    let worker_handler = worker.start_bg()?;
    let abort_worker = worker_handler.abort_handle();

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install C-c handler");
    };

    let sigterm = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install sigterm handler")
            .recv()
            .await
    };

    tokio::select! {
        _ = ctrl_c => {
        tracing::info!("ctrl-c hit, graceful shutdown...")
        },
        _ = sigterm => {
        tracing::info!("SIGTERM received, graceful shutdown...")
        },
    };
    tracing::info!("cancelling all background jobs...");
    token.cancel();
    // wait for 2 seconds for other tasks to finish cleaning up
    // or when bg_jobs finishes tearing down, whichever happens first
    tokio::select! {
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
            tracing::info!("background jobs couldn't finish after the graceful duration, forced shutting down..");
            abort_worker.abort();
        },
    // forced cancellation of bg_worker
        _ = worker_handler => {
            tracing::info!("background jobs cancelled, shutting down...");
        },
    }
    tracing::info!("process terminated");
    Ok(())
}
