use crate::services::congestion_control::CongestionControlState;
use crate::services::i_wh_req_handler::IWebhookRequestHandleService;
use crate::services::wh_req_handler::ProductServiceImpl;
use redis::aio::MultiplexedConnection;
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

#[derive(Debug)]
pub struct BgWorker {
    pub(crate) redis_conn: MultiplexedConnection,
    pub(crate) cancel_token: CancellationToken,
    pub(crate) client: reqwest::Client,
    pub(crate) cc_state: Mutex<CongestionControlState>,
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
}
