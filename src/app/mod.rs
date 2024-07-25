use crate::services::congestion_control::CongestionControlState;
use crate::services::i_wh_req_handler::IWebhookRequestHandleService;
use crate::services::wh_req_handler::ProductServiceImpl;
use rdkafka::producer::FutureProducer;
use redis::aio::{ConnectionLike, ConnectionManager};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

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

// #[derive(Debug)]
pub struct BgWorker {
    pub(crate) redis_conn: ConnectionManager,
    pub(crate) cancel_token: CancellationToken,
    pub(crate) client: reqwest::Client,
    pub(crate) cc_state: Mutex<CongestionControlState>,
}

impl BgWorker {
    pub fn new(
        redis_conn: ConnectionManager,
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
}

impl std::fmt::Debug for BgWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BgWorker")
            .field("redis_conn: ", &self.redis_conn.get_db())
            .finish()
    }
}

pub struct BgKafkaWorker {
    kakfa_producer: FutureProducer,
    redis_conn: ConnectionManager,
    pub(crate) cancel_token: CancellationToken,
}

impl std::fmt::Debug for BgKafkaWorker {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "BgKafkaWorkder {{ kafka_producer: {}, cancel_token: {:#?} }}",
            "{...}", self.cancel_token
        )
    }
}

impl BgKafkaWorker {
    pub fn new(
        kakfa_producer: FutureProducer,
        redis_conn: ConnectionManager,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            kakfa_producer,
            redis_conn,
            cancel_token,
        }
    }

    pub fn kafka_producer(&self) -> FutureProducer {
        self.kakfa_producer.clone()
    }

    pub fn redis_conn(&self) -> ConnectionManager {
        self.redis_conn.clone()
    }
}
