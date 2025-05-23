//! This module contains all the request and response objects that are used for interacting between the Mint and Wallet in Cashu.
//! All of these structs are serializable and deserializable using serde.

use std::{collections::HashMap, fmt::Display, str::FromStr};

use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    blind::{BlindedMessage, BlindedSignature},
    proof::Proofs,
};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, ToSchema, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CurrencyUnit {
    Ugx,
    Usd,
    MUsd,
    Sat,
}

impl Display for CurrencyUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ugx => write!(f, "ugx"),
            Self::Usd => write!(f, "usd"),
            Self::MUsd => write!(f, "musd"),
            Self::Sat => write!(f, "sat"),
        }
    }
}

impl FromStr for CurrencyUnit {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ugx" => Ok(Self::Ugx),
            "usd" => Ok(Self::Usd),
            "musd" => Ok(Self::MUsd),
            "sat" => Ok(Self::Sat),
            _ => Err(()),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintQuoteOnchainRequest {
    #[schema(example = "1500")]
    pub amount: u64,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintQuoteOnchainResponse {
    pub quote: String,
    pub reference: String,
    pub fee: u64,
    pub state: MintOnchainState,
    pub expiry: u64,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum MintOnchainState {
    /// initial state. No payment received from the wallet yet
    Unpaid,

    Pending,

    Paid,

    Issued,
}

impl Display for MintOnchainState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MintOnchainState::Unpaid => write!(f, "UNPAID"),
            MintOnchainState::Pending => write!(f, "PENDING"),
            MintOnchainState::Paid => write!(f, "PAID"),
            MintOnchainState::Issued => write!(f, "ISSUED"),
        }
    }
}

impl FromStr for MintOnchainState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UNPAID" => Ok(MintOnchainState::Unpaid),
            "PENDING" => Ok(MintOnchainState::Pending),
            "PAID" => Ok(MintOnchainState::Paid),
            "ISSUED" => Ok(MintOnchainState::Issued),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OnchainMintQuote {
    pub quote_id: Uuid,
    pub reference: String,
    pub fee_total: u64,
    // pub unit: CurrencyUnit,
    pub amount: u64,
    pub expiry: u64,
    pub state: MintOnchainState,
}

impl From<OnchainMintQuote> for PostMintQuoteOnchainResponse {
    fn from(quote: OnchainMintQuote) -> Self {
        Self {
            quote: quote.quote_id.to_string(),
            reference: quote.reference,
            fee: quote.fee_total,
            state: quote.state,
            expiry: quote.expiry,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintOnchainRequest {
    pub quote: String,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMintOnchainResponse {
    pub signatures: Vec<BlindedSignature>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltQuoteOnchainRequest {
    pub amount: u64,
    /// onchain address
    pub address: String,
    // pub unit: CurrencyUnit,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltQuoteOnchainResponse {
    pub quote: String,
    pub description: Option<String>,
    pub amount: u64,
    pub fee: u64,
    pub state: MeltOnchainState,
    pub expiry: u64,
}

impl From<OnchainMeltQuote> for PostMeltQuoteOnchainResponse {
    fn from(quote: OnchainMeltQuote) -> Self {
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
pub enum MeltOnchainState {
    /// initial state. No payment received from the wallet yet
    Unpaid,

    /// the mint received the payment from the wallet, but did not broadcast the transaction yet
    Pending,

    /// the mint broadcasted the onchain transaction
    Paid,
}

impl Display for MeltOnchainState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeltOnchainState::Unpaid => write!(f, "UNPAID"),
            MeltOnchainState::Pending => write!(f, "PENDING"),
            MeltOnchainState::Paid => write!(f, "PAID"),
        }
    }
}

impl FromStr for MeltOnchainState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UNPAID" => Ok(MeltOnchainState::Unpaid),
            "PENDING" => Ok(MeltOnchainState::Pending),
            "PAID" => Ok(MeltOnchainState::Paid),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OnchainMeltQuote {
    pub quote_id: Uuid,
    pub amount: u64,
    pub address: String,
    pub reference: String,
    pub fee_total: u64,
    pub fee_sat_per_vbyte: u32,
    pub expiry: u64,
    pub state: MeltOnchainState,
    pub description: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltOnchainRequest {
    pub quote: String,
    pub inputs: Proofs,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostMeltOnchainResponse {
    pub state: MeltOnchainState,
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

#[skip_serializing_none]
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, ToSchema)]
pub struct MintInfoResponse {
    pub name: Option<String>,
    // #[schema(value_type = String)]
    // pub pubkey: PublicKey,
    pub version: Option<String>,
    // pub description: Option<String>,
    // pub description_long: Option<String>,
    // pub contact: Option<Vec<ContactInfoResponse>>,
    // pub motd: Option<String>,
    pub usdc_address: String,
    pub usdc_token_mint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PostCheckStateRequest {
    #[serde(rename = "Ys")]
    pub ys: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone, ToSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum ProofState {
    Unspent,
    Pending,
    Spent,
}

impl Display for ProofState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofState::Unspent => write!(f, "UNSPENT"),
            ProofState::Pending => write!(f, "PENDING"),
            ProofState::Spent => write!(f, "SPENT"),
        }
    }
}

impl FromStr for ProofState {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UNSPENT" => Ok(ProofState::Unspent),
            "PENDING" => Ok(ProofState::Pending),
            "SPENT" => Ok(ProofState::Spent),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ProofStatus {
    #[serde(rename = "Y")]
    pub y: String,
    pub state: ProofState,
    pub witness: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct PostCheckStateResponse {
    pub states: Vec<ProofStatus>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostCurrencyExchangeRequest {
    #[schema(example = "1500")]
    pub amount: u64,
    pub inputs: Proofs,
    pub outputs: Vec<BlindedMessage>,
}

#[derive(Deserialize, Serialize, Debug, Clone, ToSchema)]
pub struct PostCurrencyExchangeResponse {
    pub signatures: Vec<BlindedSignature>,
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::primitives::{KeyResponse, MintInfoResponse, PostSwapResponse};

    #[test]
    fn test_serialize_empty_swap_response() -> anyhow::Result<()> {
        let response = PostSwapResponse::default();
        let serialized = serde_json::to_string(&response)?;
        assert_eq!(serialized, "{\"signatures\":[]}");
        Ok(())
    }

    #[test]
    fn test_serialize_keyresponse() -> anyhow::Result<()> {
        let response = KeyResponse {
            id: "test".to_string(),
            unit: crate::primitives::CurrencyUnit::MUsd,
            keys: std::collections::HashMap::new(),
        };
        let serialized = serde_json::to_string(&response)?;
        assert_eq!(
            serialized,
            "{\"id\":\"test\",\"unit\":\"musd\",\"keys\":{}}"
        );
        Ok(())
    }

    #[test]
    fn test_serialize_mint_info() -> anyhow::Result<()> {
        let mint_info = MintInfoResponse {
            name: Some("Bob's Cashu mint".to_string()),
            version: Some("Nutshell/0.11.0".to_string()),
            usdc_address: String::from(
                "02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba2",
            ),
            usdc_token_mint: String::from(
                "02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba1",
            ),
        };
        let out = serde_json::to_string_pretty(&mint_info)?;
        assert!(!out.is_empty());
        assert!(out.contains("02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba2"));
        assert!(out.contains("02a9acc1e48c25eeeb9289b5031cc57da9fe72f3fe2861d264bdc074209b107ba1"));
        Ok(())
    }

    // #[test]
    // fn test_deserialize_nustash_mint_info() -> anyhow::Result<()> {
    //     let mint_info = read_fixture("nutshell_mint_info.json")?;
    //     let info = serde_json::from_str::<MintInfoResponse>(&mint_info);
    //     assert!(info.is_ok());
    //     let info = info?;
    //     assert_eq!("Nutshell/0.15.0", info.version.unwrap());
    //     Ok(())
    // }
}
