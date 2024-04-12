use crate::app::AppEnv;
use crate::model::error::AppError;
use crate::model::ReqDownstream;
use crate::services::i_wh_req_handler::IWebhookRequestHandleService;
use crate::services::wh_req_handler::ProductServiceImpl;
use axum::extract::{Host, Query, State};
use http::{HeaderMap, Method, Uri};
use std::collections::HashMap;
use std::sync::Arc;

#[tracing::instrument(level = "debug")]
pub async fn home() -> String {
    "Hello hook".to_string()
}

#[tracing::instrument(level = "debug")]
#[axum::debug_handler]
pub async fn webhook_handler(
    State(app): State<Arc<AppEnv<ProductServiceImpl>>>,
    headers: HeaderMap,
    method: Method,
    Host(host): Host,
    uri: Uri,
    Query(queries): Query<HashMap<String, String>>,
    payload: String,
    // request: Request<Body>,
) -> Result<String, AppError> {
    let request = ReqDownstream {
        endpoint: uri.to_string(),
        method,
        headers,
        queries,
        payload,
    };

    // tracing::debug!("Received request {request:#?}");

    if let Err(err) = app.request_handle_svc.handle_webhook_request(request).await {
        tracing::error!("error while handling request: {err:?}");
        return Err(err.into());
    }

    Ok("webhook request enqueued for downstream".to_string())
}
