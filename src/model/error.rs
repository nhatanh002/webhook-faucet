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
