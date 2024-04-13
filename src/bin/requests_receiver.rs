use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use tokio::signal;
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};
use webhook_svc::app::AppEnv;
use webhook_svc::{http::router, *};

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let cnf = Lazy::force(config::get());
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::CLOSE))
        .init();

    let redis_cli =
        redis::Client::open(cnf.redis_url.as_str()).context("failed to init redis client")?;

    let redis_conn = redis_cli
        .get_multiplexed_async_connection()
        .await
        .context("failed to connect to redis for consumer")?;

    let product_svc = services::wh_req_handler::ProductServiceImpl::new(redis_conn);
    let app = AppEnv::new(product_svc);
    let router = router::new(app).await;

    tracing::info!("starting axum server");
    let socket_addr = format!("{}:{}", cnf.app_host, cnf.app_port);
    // let listener = tokio::net::TcpListener::bind(&socket_addr).await?;
    let sock = tokio::net::TcpSocket::new_v4()?;
    // sock.set_recv_buffer_size(200000)?;
    sock.set_reuseport(true)?;
    sock.bind(socket_addr.parse()?)?;
    let listener = sock.listen(10000)?;

    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
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
        })
        .await
        .context("axum server failed")?;
    tracing::info!("process terminated");
    Ok(())
}
