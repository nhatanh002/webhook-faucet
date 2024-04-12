use crate::model::ReqDownstream;

pub trait IWebhookRequestHandleService: Send + Sync + 'static {
    fn handle_webhook_request(
        &self,
        request: ReqDownstream,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}
