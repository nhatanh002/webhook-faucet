use chrono::{DateTime, TimeDelta, Utc};
use core::ops::Sub;
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;

use crate::common::{self, consts};
use crate::config;
use crate::model::ReqDownstream;

use super::i_wh_req_handler::IWebhookRequestHandleService;

#[derive(Debug, Clone)]
pub struct ProductServiceImpl {
    redis_conn: MultiplexedConnection,
}

impl ProductServiceImpl {
    pub fn new(redis_conn: MultiplexedConnection) -> Self {
        Self { redis_conn }
    }
}

impl IWebhookRequestHandleService for ProductServiceImpl {
    async fn handle_webhook_request(&self, request: ReqDownstream) -> anyhow::Result<()> {
        let mut redis_conn = self.redis_conn.clone();
        let ser = zstd::bulk::compress(&bitcode::serialize(&request)?, 0)?;
        // let de_ser = zstd::bulk::decompress(&ser, 100000)?;

        let shop = request
            .headers
            .get(consts::XSHOPIFY_SHOP_DOMAIN)
            .map_or(
                Err::<&str, _>(anyhow::anyhow!("no shop domain header")),
                |h| {
                    h.to_str().map_err(|e| {
                        let str = e.to_string().to_owned();
                        anyhow::anyhow!(str)
                    })
                },
            )?
            .to_owned();

        let topic = request
            .headers
            .get(consts::XSHOPIFY_TOPIC)
            .map_or(Err::<&str, _>(anyhow::anyhow!("no topic header")), |h| {
                h.to_str().map_err(|e| {
                    let str = e.to_string().to_owned();
                    anyhow::anyhow!(str)
                })
            })?
            .to_owned();

        let triggered_at = request.headers.get(consts::XSHOPIFY_TRIGGERED_AT).map_or(
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
        let three_days_ago = now.sub(TimeDelta::try_days(3).unwrap());
        if triggered_at.lt(&three_days_ago) {
            // return Err(anyhow::anyhow!("event older than 3 days"));
        };

        let hmac_sig = request.headers.get(consts::XSHOPIFY_HMAC_SHA256).map_or(
            Err::<&str, _>(anyhow::anyhow!("no hmac header")),
            |h| {
                h.to_str().map_err(|e| {
                    let str = e.to_string().to_owned();
                    anyhow::anyhow!(str)
                })
            },
        )?;

        let key = config::get().shopify_client_secret.as_bytes();
        let payload = request.payload.as_bytes();

        match common::crypt::hmac_256_verify(key, payload, hmac_sig) {
            Ok(_) => {
                tracing::debug!("hmac verified");
            }
            err @ Err(_) => {
                tracing::debug!("hmac verification failed");
                return err;
            }
        }

        // to close the conn more quickly when load is high
        // probably need
        // tokio::spawn(async move {
        redis_conn
            .zadd(format!("{}:{}", shop, topic), ser, triggered_at.timestamp())
            .await?;
        //     Ok::<(), redis::RedisError>(())
        // });
        Ok(())
    }
}
