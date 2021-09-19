pub use async_trait;
pub use reqwest;

#[cfg(feature = "retroqwest-derive")]
pub use retroqwest_derive::retroqwest;

#[derive(thiserror::Error, Debug)]
pub enum RetroqwestError {
    #[error("Failed to build client: {0}")]
    FailedToBuildClient(#[source] reqwest::Error),

    #[error("Error sending request: {0}")]
    RequestError(#[source] reqwest::Error),

    #[error("Response status code ({status}) indicates error: {source}")]
    ResponseError { status: reqwest::StatusCode, #[source] source: reqwest::Error },

    #[error("Failed to parse json: {0}")]
    JsonParse(#[source] reqwest::Error),
}
