//! This module defines the `BlindedMessage` and `BlindedSignature` structs, which are used for representing blinded messages and signatures in Cashu as described in [Nut-00](https://github.com/cashubtc/nuts/blob/main/00.md)
//!
//! The `BlindedMessage` struct represents a blinded message, with an `amount` field for the amount in USDs and a `b_` field for the public key of the blinding factor.
//!
//! The `BlindedSignature` struct represents a blinded signature, with an `amount` field for the amount in USDs, a `c_` field for the public key of the blinding factor, and an optional `id` field for the ID of the signature.
//!
//! Both the `BlindedMessage` and `BlindedSignature` structs are serializable and deserializable using serde.
//!
//! The `TotalAmount` trait is also defined in this module, which provides a `total_amount` method for calculating the total amount of a vector of `BlindedMessage` or `BlindedSignature` structs. The trait is implemented for both `Vec<BlindedMessage>` and `Vec<BlindedSignature>`.

use secp256k1::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{dhke::Dhke, error::MonexoCoreError};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlindedSignature {
    /// Amount
    ///
    /// The value of the blinded token.
    pub amount: u64,

    /// Blinded signature (C_)
    ///
    /// The blinded signature on the secret message `B_` of [BlindedMessage].
    #[serde(rename = "C_")]
    #[schema(value_type=String)]
    pub c_: PublicKey,

    /// Keyset ID
    ///
    /// ID of the mint keys that signed the token.
    pub id: String,

    /// DLEQ Proof
    ///
    /// <https://github.com/cashubtc/nuts/blob/main/12.md>
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dleq: Option<BlindSignatureDleq>,
}

/// Blinded Signature on Dleq
///
/// Defined in [NUT12](https://github.com/cashubtc/nuts/blob/main/12.md)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
// #[cfg_attr(feature = "swagger", derive(utoipa::ToSchema))]
pub struct BlindSignatureDleq {
    /// e
    #[schema(value_type = String)]
    pub e: SecretKey,
    /// s
    #[schema(value_type = String)]
    pub s: SecretKey,
}

impl BlindedSignature {
    /// New DLEQ
    #[inline]
    pub fn new(
        amount: u64,
        blinded_signature: PublicKey,
        keyset_id: String,
        blinded_message: &PublicKey,
        mint_secretkey: SecretKey,
    ) -> Result<Self, MonexoCoreError> {
        Ok(Self {
            amount,
            id: keyset_id,
            c_: blinded_signature,
            dleq: Some(calculate_dleq(
                blinded_signature,
                blinded_message,
                &mint_secretkey,
            )?),
        })
    }
}

fn calculate_dleq(
    blinded_signature: PublicKey, // C'
    blinded_message: &PublicKey,  // B'
    mint_secret_key: &SecretKey,  // a
) -> Result<BlindSignatureDleq, MonexoCoreError> {
    // Random nonce
    // let r: SecretKey = SecretKey::generate();
    let r = Dhke::new();

    // R1 = r*G
    let r1 = r.public_key();
    // let r1 = r.;

    // R2 = r*B'
    let r_scal: Scalar = r.as_scalar();
    let r2: PublicKey = blinded_message.mul_tweak(&SECP256K1, &r_scal)?.into();

    // e = hash(R1,R2,A,C')
    let e: [u8; 32] = hash_e([r1, r2, mint_secret_key.public_key(), blinded_signature]);
    let e_sk: SecretKey = SecretKey::from_slice(&e)?;

    // s1 = e*a
    let s1: SecretKey = e_sk.mul_tweak(&mint_secret_key.as_scalar())?.into();

    // s = r + s1
    let s: SecretKey = r.add_tweak(&s1.to_scalar())?.into();

    Ok(BlindSignatureDleq { e: e_sk, s })
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BlindedMessage {
    pub amount: u64,
    #[serde(rename = "B_")]
    #[schema(value_type=String)]
    pub b_: PublicKey,
    // FIXME use KeysetId
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct BlindingFactor(SecretKey);

impl From<SecretKey> for BlindingFactor {
    fn from(sk: SecretKey) -> Self {
        BlindingFactor(sk)
    }
}

impl TryFrom<&str> for BlindingFactor {
    type Error = MonexoCoreError;

    fn try_from(hex: &str) -> Result<Self, Self::Error> {
        use std::str::FromStr;
        Ok(secp256k1::SecretKey::from_str(hex)?.into())
    }
}

impl BlindingFactor {
    pub fn as_hex(&self) -> String {
        hex::encode(&self.0[..])
    }

    pub fn to_secret_key(&self) -> SecretKey {
        self.0
    }
}

pub trait TotalAmount {
    fn total_amount(&self) -> u64;
}

impl TotalAmount for Vec<BlindedSignature> {
    fn total_amount(&self) -> u64 {
        self.iter().fold(0, |acc, x| acc + x.amount)
    }
}

impl TotalAmount for Vec<BlindedMessage> {
    fn total_amount(&self) -> u64 {
        self.iter().fold(0, |acc, x| acc + x.amount)
    }
}
