use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::model::error::BgError;
use crate::model::ReqDownstream;
use crate::services::congestion_control::CongestionControlState;
use crate::services::i_wh_req_handler::IWebhookRequestHandleService;
use crate::services::wh_req_handler::ProductServiceImpl;

#[derive(Debug, Clone)]
pub struct AppEnv<ProductService: IWebhookRequestHandleService + Clone = ProductServiceImpl> {
    pub request_handle_svc: ProductService,
}

impl<ProductService> AppEnv<ProductService>
where
    ProductService: IWebhookRequestHandleService + Clone,
{
    pub fn new(request_handle_svc: ProductService) -> Self {
        Self { request_handle_svc }
    }
}

#[derive(Debug)]
pub struct BgWorker {
    redis_conn: MultiplexedConnection,
    cancel_token: CancellationToken,
    client: reqwest::Client,
    cc_state: Mutex<CongestionControlState>,
}

impl BgWorker {
    pub fn new(
        redis_conn: MultiplexedConnection,
        cancel_token: CancellationToken,
        client: reqwest::Client,
    ) -> Self {
        let cc_state = Mutex::new(CongestionControlState::default());
        Self {
            redis_conn,
            cancel_token,
            client,
            cc_state,
        }
    }

    #[tracing::instrument(level = "debug")]
    pub fn start_bg(self: Arc<Self>) -> anyhow::Result<tokio::task::JoinHandle<Result<()>>> {
        tracing::debug!("start BG worker");
        Ok(tokio::spawn(async move {
            tracing::debug!("spawn BG worker");
            let rest_rate = 5;
            let batch_size = 1000;
            let cancel_token = &self.cancel_token;
            let mut heartbeat = std::pin::pin!(tokio::time::sleep(std::time::Duration::from_secs(
                rest_rate
            )));

            loop {
                let timer = heartbeat.as_mut();
                tracing::debug!("BG worker loop");
                tokio::select! {
                    biased;
                    _ = cancel_token.cancelled() => {
                    tracing::info!("terminating background job...");
                    return Result::<()>::Ok(())
                    },
                    _ = timer => {
                        tracing::info!("bg worker woke up!");
                        let mut redis_conn = self.redis_conn.clone();
                        let mut cmd = redis::cmd("SCAN");
                        cmd.arg(0).arg("TYPE").arg("ZSET"); // build Cmd
                        let mut queues: redis::AsyncIter<String> = cmd.iter_async(&mut redis_conn).await?;

                        let mut handlers = vec![];
                        while let Some(queue) = queues.next_item().await {
                            tracing::info!("handling a batch of requests in {queue:?}");
                            let migrating_self = self.clone();
                            let handler = tokio::spawn(migrating_self.handle_queue(queue, batch_size));
                            handlers.push(handler);
                        }
                        let joined = futures::future::join_all(handlers).await;
                        tracing::debug!("joined result of the last batch of requests push: {joined:#?}");

                        heartbeat.as_mut().reset((std::time::Instant::now() + std::time::Duration::from_secs(rest_rate)).into());
                    }
                }
            }
        }))
    }

    async fn handle_queue(self: Arc<Self>, queue: String, batch_size: u32) -> Result<()> {
        let mut range_start = 0;
        loop {
            let queued_requests = self
                .pull_requests(&queue, range_start, batch_size)
                .await
                .inspect_err(|e| tracing::error!("{e:?}"))?;
            tracing::debug!("queued requests: {queued_requests:?}");
            if queued_requests.is_empty() {
                tracing::debug!("requests queue empty");
                break;
            }
            range_start += batch_size;

            for req in queued_requests {
                let migrating_self = self.clone();
                migrating_self.handle_request(&queue, &req).await?;
            }
        }
        tracing::info!("finished handling requests in {queue:?}");
        Ok(())
    }

    async fn handle_request(self: Arc<Self>, queue: &str, req: &Vec<u8>) -> Result<()> {
        // tracing::debug!("processing request: {req:?} in queue {queue:?}");
        tracing::debug!("processing a request in queue {queue:?}");
        let req_de: ReqDownstream = bitcode::deserialize(&zstd::bulk::decompress(req, 100000)?)?;

        let mut retry_interval = 1;
        let mut next_retry = 1;
        let max_retry_attempts = 10;
        for rep in 0..max_retry_attempts {
            tracing::debug!("attempt number {rep} to call downstream...");
            let start = std::time::Instant::now();
            let downstream_response = self.push_downstream(&req_de).await;
            // .inspect_err(|e| tracing::error!("{e:?}"))
            // .map_err(anyhow::Error::from)?;
            let elapsed = start.elapsed();
            tracing::debug!("elapsed time for request: {elapsed:?}");

            let next_delay: Duration;
            {
                // to limit the scope of the mutex
                let mut cc_state = self.cc_state.lock().await;
                cc_state.update_cc_state(&elapsed); // update congestion control state
                next_delay = cc_state.sleep_duration;
            }

            if let Err(e) = &downstream_response {
                tracing::error!("downstream requesing error: {e:?}");
                match e {
                    BgError::ReqwestError(re)
                        if let Some(status) = re.status()
                            && status.is_client_error() =>
                    {
                        self.delete_request(queue, req.as_slice())
                            .await
                            .inspect_err(|e| tracing::error!("{e:?}"))?;
                        tracing::debug!("faulty requests in queue {queue:?}");
                        break;
                    }
                    BgError::ReqwestError(re)
                        if let Some(status) = re.status()
                            && status.is_server_error() =>
                    {
                        // retry logic
                        tracing::debug!(
                            "downstream error {re:#?}, retrying in {retry_interval} seconds..."
                        );
                    }
                    BgError::ReqwestError(re) if re.is_timeout() => {
                        // retry logic
                        tracing::debug!(
                            "request timeout: {re:#?}, retrying in {retry_interval} seconds..."
                        );
                    }
                    BgError::ReqwestError(re) => {
                        tracing::error!("unable to handle downstream request error: {re:?}");
                        break;
                    }
                    BgError::ParseError(re) => {
                        tracing::error!("downstream url parsing error: {re:?}");
                        break;
                    }
                };
                tracing::debug!("retrying...");
                tokio::time::sleep(Duration::from_millis(500 * retry_interval)).await;
                let tmp = retry_interval + next_retry;
                retry_interval = next_retry;
                next_retry = tmp;
                continue;
            } else {
                tracing::debug!("ret = {downstream_response:?}");
                tracing::debug!("pushed requests");
                self.delete_request(queue, req.as_slice())
                    .await
                    .inspect_err(|e| tracing::error!("{e:?}"))?;
                tracing::debug!("deleted requests in queue {queue:?}");

                tracing::debug!("adjusting request traffic, sleep for {:?}...", next_delay);
                tokio::time::sleep(next_delay).await;
                break;
            }
        }
        Ok(())
    }

    //lul at the name, totally coindidental
    #[tracing::instrument(level = "debug")]
    async fn pull_requests(
        &self,
        queue_name: &str,
        range_start: u32,
        range_size: u32,
    ) -> anyhow::Result<Vec<Vec<u8>>> {
        let mut redis_conn = self.redis_conn.clone();
        let rv: redis::RedisResult<Vec<Vec<u8>>> = redis_conn
            .zrange(
                queue_name,
                range_start.try_into()?,
                (range_start + range_size - 1).try_into()?,
            )
            .await;
        Ok(rv?)
    }

    async fn delete_request(&self, queue_name: &str, member: &[u8]) -> anyhow::Result<()> {
        let mut redis_conn = self.redis_conn.clone();
        let rv: i32 = redis_conn
            .zrem(queue_name, member)
            .await
            .context("failed to remove requests from {queue_name}")?;
        tracing::debug!("removed an item {rv:?} in {queue_name}");
        Ok(())
    }

    #[tracing::instrument(level = "debug")]
    async fn push_downstream(&self, req: &ReqDownstream) -> Result<reqwest::Response, BgError> {
        let base_url = &crate::config::get().downstream_app_url;
        let endpoint = &req.endpoint;
        let endpoint_url = format!("{base_url}{endpoint}");
        let url = reqwest::Url::parse(&endpoint_url).map_err(|e| BgError::ParseError(e.into()))?;
        tracing::debug!("url: {url}");
        let resp = self
            .client
            .request(req.method.to_owned(), url)
            .headers(req.headers.to_owned())
            .query(&req.queries)
            .body(req.payload.to_owned())
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(BgError::ReqwestError)?;
        // .context("failed to push event")
        // .map_err(anyhow::Error::from)?;
        // reqwest::Error;
        // resp.error_for_status().map_err(Into::into)
        match resp.status() {
            status if status >= http::StatusCode::INTERNAL_SERVER_ERROR => {
                // consider status >= 500 as error
                resp.error_for_status().map_err(BgError::ReqwestError)
            }
            _ => Ok(resp),
        }
    }
}
