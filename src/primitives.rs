//! This module contains all the request and response objects that are used for interacting between the Mint and Wallet in Cashu.
//! All of these structs are serializable and deserializable using serde.

use std::{fmt::Display, str::FromStr};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::blind::{BlindedMessage, BlindedSignature};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, ToSchema, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CurrencyUnit {
    Ugx,
    Sat,
    Usd,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintQuoteBtcOnchainRequest {
    pub amount: u64,
    pub unit: CurrencyUnit,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintQuoteBtcOnchainResponse {
    pub quote: String,
    // pub address: String,
    pub state: MintBtcOnchainState,
    pub expiry: u64,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum MintBtcOnchainState {
    /// initial state. No payment received from the wallet yet
    Unpaid,

    Pending,

    Paid,

    Issued,
}

impl Display for MintBtcOnchainState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MintBtcOnchainState::Unpaid => write!(f, "UNPAID"),
            MintBtcOnchainState::Pending => write!(f, "PENDING"),
            MintBtcOnchainState::Paid => write!(f, "PAID"),
            MintBtcOnchainState::Issued => write!(f, "ISSUED"),
        }
    }
}

impl FromStr for MintBtcOnchainState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UNPAID" => Ok(MintBtcOnchainState::Unpaid),
            "PENDING" => Ok(MintBtcOnchainState::Pending),
            "PAID" => Ok(MintBtcOnchainState::Paid),
            "ISSUED" => Ok(MintBtcOnchainState::Issued),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BtcOnchainMintQuote {
    pub quote_id: Uuid,
    // pub address: String,
    pub unit: CurrencyUnit,
    pub amount: u64,
    pub expiry: u64,
    pub state: MintBtcOnchainState,
}

impl From<BtcOnchainMintQuote> for PostMintQuoteBtcOnchainResponse {
    fn from(quote: BtcOnchainMintQuote) -> Self {
        Self {
            quote: quote.quote_id.to_string(),
            // address: quote.address,
            state: quote.state,
            expiry: quote.expiry,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintBtcOnchainRequest {
    pub quote: String,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintBtcOnchainResponse {
    pub signatures: Vec<BlindedSignature>,
}
