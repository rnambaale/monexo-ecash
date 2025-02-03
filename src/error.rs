
use axum::{response::{IntoResponse, Response}, Json, http::StatusCode};
use thiserror::Error;
use serde_json::json;
use tracing::{event, Level};

#[derive(Error, Debug)]
pub enum MonexoMintError {
    #[error("DB Error {0}")]
    Db(#[from] sqlx::Error),

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Invalid quote {0}")]
    InvalidQuote(String),

    #[error("Invalid quote uuid {0}")]
    InvalidUuid(#[from] uuid::Error),

    #[error("PrivateKey in keyset not found")]
    PrivateKeyNotFound,

    #[error("MokshaCoreError: {0}")]
    MokshaCore(#[from] monexo_core::error::MonexoCoreError),
}

impl IntoResponse for MonexoMintError {
    fn into_response(self) -> Response {
        event!(Level::ERROR, "error in mint: {:?}", self);

        let body = Json(json!({
            "code": 0,
            "detail": self.to_string(),
        }));

        (StatusCode::BAD_REQUEST, body).into_response()
    }
}
