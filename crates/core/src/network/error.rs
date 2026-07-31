use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ActionRequestError {
    #[error("unrecognized command: {0}")]
    UnrecognizedCommand(String),

    #[error("something with the api: {0:?}")]
    ApiError(CodedErrorObject),

    #[error("failed to construct request: {0}")]
    UreqHttp(#[from] ureq::http::Error),

    #[error("request error: {0}")]
    Ureq(#[from] ureq::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// This should be mapped to individual error-types but
/// represents an all purpose error type for most of the
/// api-calls
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct CodedErrorObject {
    pub code: u64,
    pub message: Box<str>,
    pub data: Option<serde_json::Value>,
}
