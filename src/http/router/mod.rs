use crate::app::AppEnv;
use std::sync::Arc;

use axum::routing::post;
use axum::Extension;
use axum::{routing::get, Router};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer, normalize_path::NormalizePathLayer, trace::TraceLayer,
};

mod webhook;

pub async fn new(app: AppEnv) -> Router {
    let app_state = Arc::new(app);
    Router::new()
        .route("/", get(|| async { "Hello!" }))
        // .route("/webhook", get(webhook::home))
        .route("/webhook/:resource/:topic", post(webhook::webhook_handler))
        .with_state(app_state)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(NormalizePathLayer::trim_trailing_slash())
                .layer(Extension(())),
        )
}
