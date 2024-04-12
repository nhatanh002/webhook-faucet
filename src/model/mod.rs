use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub mod error;

#[derive(Debug, Clone)]
pub struct Product {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReqDownstream {
    pub endpoint: String,
    #[serde(with = "http_serde::method")]
    pub method: http::Method,
    #[serde(with = "http_serde::header_map")]
    pub headers: http::HeaderMap,
    pub queries: HashMap<String, String>,
    pub payload: String,
}
