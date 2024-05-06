use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum AppError {
    #[error("request not originated from shopify")]
    NotShopifyOriginated,
    #[error("some of the expected services are unavailable: {source}")]
    ServiceUnavailable {
        #[from]
        source: Box<dyn std::error::Error>,
    },
    #[error("unable to get app state")]
    UnableToGetAppState,
    #[error("error: {0}")]
    WrappedError(anyhow::Error),
    // #[error("http request to backend failed: {0}")]
    // OmegaErrorResponse(anyhow::Error),
    #[error("error: {0}")]
    GeneralError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{:?}", self)).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::WrappedError(err)
    }
}

#[derive(Debug, ThisError)]
pub enum BgError {
    #[error("error: {0}")]
    ParseError(anyhow::Error),
    #[error("error: {0}")]
    ReqwestError(reqwest::Error),
}

#[derive(Debug, ThisError)]
pub enum BgKafkaError {
    #[error("RDKafka error: {0}")]
    RDKafka(#[from] rdkafka::error::RDKafkaError),
    #[error("Kafka error: {0}")]
    KafkaError(#[from] rdkafka::error::KafkaError),
    #[error("json conversion error: {0}")]
    JsonDerserError(#[from] serde_json::Error),
}

impl<'a>
    From<(
        rdkafka::error::KafkaError,
        rdkafka::producer::FutureRecord<'a, str, std::vec::Vec<u8>>,
    )> for BgKafkaError
{
    fn from(
        e: (
            rdkafka::error::KafkaError,
            rdkafka::producer::FutureRecord<'a, str, std::vec::Vec<u8>>,
        ),
    ) -> Self {
        Self::KafkaError(e.0)
    }
}

impl From<(rdkafka::error::KafkaError, rdkafka::message::OwnedMessage)> for BgKafkaError {
    fn from(e: (rdkafka::error::KafkaError, rdkafka::message::OwnedMessage)) -> Self {
        Self::KafkaError(e.0)
    }
}
