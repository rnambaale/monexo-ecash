//! This module contains all the request and response objects that are used for interacting between the Mint and Wallet in Cashu.
//! All of these structs are serializable and deserializable using serde.

use std::{collections::HashMap, fmt::Display, str::FromStr};

use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{blind::{BlindedMessage, BlindedSignature}, proof::Proofs};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, ToSchema, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CurrencyUnit {
    Ugx,
    Sat,
    Usd,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintQuoteBtcOnchainRequest {
    #[schema(example = "1500")]
    pub amount: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintQuoteBtcOnchainResponse {
    pub quote: String,
    pub reference: String,
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
    pub reference: String,
    // pub unit: CurrencyUnit,
    pub amount: u64,
    pub expiry: u64,
    pub state: MintBtcOnchainState,
}

impl From<BtcOnchainMintQuote> for PostMintQuoteBtcOnchainResponse {
    fn from(quote: BtcOnchainMintQuote) -> Self {
        Self {
            quote: quote.quote_id.to_string(),
            reference: quote.reference,
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

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltQuoteBtcOnchainRequest {
    pub amount: u64,
    /// onchain address
    pub address: String,
    // pub unit: CurrencyUnit,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltQuoteBtcOnchainResponse {
    pub quote: String,
    pub description: Option<String>,
    pub amount: u64,
    pub fee: u64,
    pub state: MeltBtcOnchainState,
    pub expiry: u64,
}

impl From<BtcOnchainMeltQuote> for PostMeltQuoteBtcOnchainResponse {
    fn from(quote: BtcOnchainMeltQuote) -> Self {
        Self {
            quote: quote.quote_id.to_string(),
            amount: quote.amount,
            fee: quote.fee_total,
            expiry: quote.expiry,
            state: quote.state,
            description: quote.description,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum MeltBtcOnchainState {
    /// initial state. No payment received from the wallet yet
    Unpaid,

    /// the mint received the payment from the wallet, but did not broadcast the transaction yet
    Pending,

    /// the mint broadcasted the btc onchain transaction
    Paid,
}

impl Display for MeltBtcOnchainState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeltBtcOnchainState::Unpaid => write!(f, "UNPAID"),
            MeltBtcOnchainState::Pending => write!(f, "PENDING"),
            MeltBtcOnchainState::Paid => write!(f, "PAID"),
        }
    }
}

impl FromStr for MeltBtcOnchainState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UNPAID" => Ok(MeltBtcOnchainState::Unpaid),
            "PENDING" => Ok(MeltBtcOnchainState::Pending),
            "PAID" => Ok(MeltBtcOnchainState::Paid),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BtcOnchainMeltQuote {
    pub quote_id: Uuid,
    pub amount: u64,
    pub address: String,
    pub fee_total: u64,
    pub fee_sat_per_vbyte: u32,
    pub expiry: u64,
    pub state: MeltBtcOnchainState,
    pub description: Option<String>,
}


#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltBtcOnchainRequest {
    pub quote: String,
    pub inputs: Proofs,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltBtcOnchainResponse {
    pub state: MeltBtcOnchainState,
    pub txid: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PostSwapRequest {
    pub inputs: Proofs,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, ToSchema)]
pub struct PostSwapResponse {
    pub signatures: Vec<BlindedSignature>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Default, ToSchema)]
pub struct KeysResponse {
    pub keysets: Vec<KeyResponse>,
}

impl KeysResponse {
    pub fn new(keyset: KeyResponse) -> Self {
        Self {
            keysets: vec![keyset],
        }
    }
}

#[derive(serde::Deserialize, Serialize, Clone, Debug, PartialEq, Eq, ToSchema)]
pub struct KeyResponse {
    pub id: String, // TODO use new type for keyset_id
    pub unit: CurrencyUnit,
    #[schema(value_type = HashMap<u64, String>)]
    pub keys: HashMap<u64, PublicKey>,
}
