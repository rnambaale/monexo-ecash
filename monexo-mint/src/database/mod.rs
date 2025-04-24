use async_trait::async_trait;
use monexo_core::{
    primitives::{OnchainMeltQuote, OnchainMintQuote},
    proof::Proofs,
};

use crate::error::MonexoMintError;
use uuid::Uuid;

pub mod postgres;

#[async_trait]
pub trait Database {
    type DB: sqlx::Database;
    async fn begin_tx(&self) -> Result<sqlx::Transaction<Self::DB>, sqlx::Error>;

    async fn get_used_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Proofs, MonexoMintError>;

    async fn add_used_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MonexoMintError>;

    async fn add_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMintQuote,
    ) -> Result<(), MonexoMintError>;

    async fn get_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: &Uuid,
    ) -> Result<OnchainMintQuote, MonexoMintError>;

    async fn update_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMintQuote,
    ) -> Result<(), MonexoMintError>;

    async fn add_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMeltQuote,
    ) -> Result<(), MonexoMintError>;

    async fn get_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: &Uuid,
    ) -> Result<OnchainMeltQuote, MonexoMintError>;

    async fn update_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMeltQuote,
    ) -> Result<(), MonexoMintError>;
}
