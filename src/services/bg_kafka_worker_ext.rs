use anyhow::{Context, Result};
use chrono::{DateTime, TimeDelta, Utc};
use rdkafka::producer::Producer;
use redis::AsyncCommands;
use std::ops::Sub;
use std::sync::Arc;
use std::time::Duration;

use crate::adapter::kafka;
use crate::app::BgKafkaWorker;
use crate::common::consts;
use crate::model::error::BgKafkaError;
use crate::model::ReqDownstream;

impl BgKafkaWorker {
    #[tracing::instrument(level = "debug")]
    pub fn start_bg(self) -> anyhow::Result<tokio::task::JoinHandle<Result<()>>> {
        tracing::debug!("start BG Kafka worker");
        let selfp = Arc::new(self);
        Ok(tokio::spawn(async move {
            tracing::debug!("spawn BG Kafka worker");
            let cnf = crate::config::get();
            let rest_rate = cnf.worker_rest;
            let batch_size = cnf.worker_batch_size;
            let cancel_token = &selfp.cancel_token;
            let mut heartbeat = std::pin::pin!(tokio::time::sleep(std::time::Duration::from_secs(
                rest_rate
            )));

            loop {
                let timer = heartbeat.as_mut();
                tracing::debug!("BG Kafka worker loop");
                tokio::select! {
                    biased;
                    _ = cancel_token.cancelled() => {
                    tracing::info!("cleaning up remaining jobs...");
                    selfp.kafka_producer().flush(Duration::from_secs(30))?;
                    tracing::info!("terminating background job...");
                    return Result::<()>::Ok(())
                    },
                    _ = timer => {
                        tracing::info!("bg worker woke up!");
                        let mut redis_conn = selfp.redis_conn();
                        let mut cmd = redis::cmd("SCAN");
                        cmd.arg(0).arg("TYPE").arg("ZSET"); // build Cmd
                        let mut queues: redis::AsyncIter<String> = cmd.iter_async(&mut redis_conn).await?;

                        let mut handlers = vec![];
                        while let Some(queue) = queues.next_item().await {
                            tracing::info!("handling a batch of requests in {queue:?}");
                            let migrating_self = selfp.clone();
                            let handler = tokio::spawn(migrating_self.handle_redis_queue(queue, batch_size));
                            handlers.push(handler);
                        }
                        let joined = futures::future::join_all(handlers).await;
                        tracing::debug!("joined result of the last batch of requests push: {joined:#?}");

                        heartbeat.as_mut().reset((std::time::Instant::now() + Duration::from_secs(rest_rate)).into());
                    }
                }
            }
        }))
    }

    async fn handle_redis_queue(self: Arc<Self>, queue: String, batch_size: u64) -> Result<()> {
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

        // if request has been in queue for too long (> 5 days) without being delivered to downstream, it's usually better to just drop it
        let triggered_at = req_de.headers.get(consts::XSHOPIFY_TRIGGERED_AT).map_or(
            chrono::DateTime::<Utc>::MIN_UTC,
            |h| {
                let payload = h.to_str().unwrap_or("");
                payload
                    .parse::<DateTime<Utc>>()
                    .unwrap_or(chrono::DateTime::<Utc>::MIN_UTC)
            },
        );
        // check timestamp, if too old => drop
        let now = Utc::now();
        let five_days_ago = now.sub(TimeDelta::try_days(5).unwrap());
        if triggered_at.lt(&five_days_ago) {
            tracing::debug!("request older than 5 days");
            self.delete_request(queue, req.as_slice())
                .await
                .inspect_err(|e| tracing::error!("{e:?}"))?;
            tracing::debug!("deleted request in queue {queue:?}");
            return Err(anyhow::anyhow!("request older than 3 days"));
        };
        self.push_kafka(&req_de).await?;
        self.delete_request(queue, req.as_slice())
            .await
            .inspect_err(|e| tracing::error!("{e:?}"))?;
        tracing::debug!("deleted request in queue {queue:?}");
        Ok(())
    }

    //lul at the name, totally coindidental
    #[tracing::instrument(level = "debug")]
    async fn pull_requests(
        &self,
        queue_name: &str,
        range_start: u64,
        range_size: u64,
    ) -> anyhow::Result<Vec<Vec<u8>>> {
        let mut redis_conn = self.redis_conn();
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
        let mut redis_conn = self.redis_conn();
        let rv: i32 = redis_conn
            .zrem(queue_name, member)
            .await
            .context("failed to remove requests from {queue_name}")?;
        tracing::debug!("removed an item {rv:?} in {queue_name}");
        Ok(())
    }

    #[tracing::instrument(level = "debug")]
    async fn push_kafka(&self, req: &ReqDownstream) -> Result<(), BgKafkaError> {
        let kafka_key = req
            .headers
            .get(consts::XSHOPIFY_TOPIC)
            .map_or("unknown_topic", |h| h.to_str().unwrap_or("unknown_topic"))
            .to_string();
        let kafka_topic = &crate::config::get().kafka_topic;
        let kafka_msg = serde_json::to_vec(&req)?;

        let producer = self.kafka_producer();
        producer.begin_transaction()?;
        let kafka_payload = kafka::make_kafka_payload(&kafka_msg, kafka_topic, &kafka_key);
        let res = producer
            .send(kafka_payload, std::time::Duration::from_millis(5000))
            .await;
        match res {
            Ok((partition, offset)) => {
                tracing::debug!("partition = {partition:?}, offset = {offset:?}");
                tracing::debug!("pushed request to kafka");
                producer.commit_transaction(Duration::from_millis(10000))?;
                return Ok(());
            }
            Err(e) => {
                tracing::debug!("error: {e:#?}");
                tracing::debug!("failed to request to kafka");
                producer.abort_transaction(std::time::Duration::from_millis(10000))?;
                return Err(e.into());
            }
        }
    }
}
