use std::string::FromUtf8Error;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MonexoWalletError {
    #[error("UnexpectedResponse - {0}")]
    UnexpectedResponse(String),

    #[error("{0}")]
    MintError(String),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("ReqwestError - {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("SerdeJsonError - {0}")]
    Json(#[from] serde_json::Error),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("InvalidHeaderValueError - {0}")]
    InvalidHeaderValue(#[from] reqwest::header::InvalidHeaderValue),

    #[error("URLParseError - {0}")]
    Url(#[from] url::ParseError),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("DB Error {0}")]
    Db(#[from] sqlx::Error),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("Migrate Error {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[cfg(not(target_arch = "wasm32"))]
    #[error("Sqlite Error {0}")]
    Sqlite(#[from] sqlx::sqlite::SqliteError),

    #[error("Bip32Error {0}")]
    Bip32(#[from] bip32::Error),

    #[error("Bip39Error {0}")]
    Bip39(#[from] bip39::Error),

    #[error("Secp256k1 {0}")]
    Secp256k1(#[from] secp256k1::Error),

    #[error("MokshaCoreError - {0}")]
    MokshaCore(#[from] monexo_core::error::MonexoCoreError),

    #[error("Utf8 Error {0}")]
    Utf8(#[from] FromUtf8Error),

    #[error("Not valid hex string")]
    Hex(#[from] hex::FromHexError),

    #[error("Invalid Keyset-ID")]
    Slice(#[from] std::array::TryFromSliceError),

    #[error("Primarykey not set for keyset")]
    IdNotSet,

    #[error("Found multiple seeds in the database. This is not supported.")]
    MultipleSeeds,

    #[error("Pubkey not found")]
    PubkeyNotFound,

    #[error("Unsupported version: Only mints with /v1 api are supported")]
    UnsupportedApiVersion,

    #[error("{1}")]
    InvoiceNotPaidYet(u64, String),
}
