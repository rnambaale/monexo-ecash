use std::str::FromStr;

use async_trait::async_trait;

use monexo_core::{
    dhke,
    primitives::{MeltOnchainState, MintOnchainState, OnchainMeltQuote, OnchainMintQuote},
    proof::{Proof, Proofs},
};
use sqlx::postgres::PgPoolOptions;
use tracing::instrument;
use uuid::Uuid;

use crate::{config::DatabaseConfig, error::MonexoMintError};

use super::Database;

#[derive(Clone)]
pub struct PostgresDB {
    pool: sqlx::Pool<sqlx::Postgres>,
}

impl PostgresDB {
    pub async fn new(config: &DatabaseConfig) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: PgPoolOptions::new()
                .max_connections(config.max_connections)
                .connect(config.db_url.as_str())
                .await?,
        })
    }

    pub async fn migrate(&self) {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .expect("Could not run migrations");
    }
}

#[async_trait]
impl Database for PostgresDB {
    type DB = sqlx::Postgres;

    async fn begin_tx(&self) -> Result<sqlx::Transaction<Self::DB>, sqlx::Error> {
        self.pool.begin().await
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn get_used_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Proofs, MonexoMintError> {
        let proofs = sqlx::query!("SELECT * FROM used_proofs")
            .fetch_all(&mut **tx)
            .await?
            .into_iter()
            .map(|row| Proof {
                amount: row.amount as u64,
                secret: row.secret,
                c: dhke::public_key_from_hex(&row.c).to_owned(),
                keyset_id: row.keyset_id,
                script: None,
            })
            .collect::<Vec<Proof>>();

        Ok(proofs.into())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn add_used_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MonexoMintError> {
        for proof in proofs.proofs() {
            sqlx::query!(
                "INSERT INTO used_proofs (amount, secret, c, keyset_id) VALUES ($1, $2, $3, $4)",
                proof.amount as i64,
                proof.secret,
                proof.c.to_string(),
                proof.keyset_id.to_string()
            )
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn add_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMintQuote,
    ) -> Result<(), MonexoMintError> {
        sqlx::query!(
            "INSERT INTO onchain_mint_quotes (id, reference, fee_total, amount, expiry, state) VALUES ($1, $2, $3, $4, $5, $6)",
            quote.quote_id,
            quote.reference,
            quote.fee_total as i64,
            quote.amount as i64,
            quote.expiry as i64,
            quote.state.to_string(),
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn get_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: &Uuid,
    ) -> Result<OnchainMintQuote, MonexoMintError> {
        let quote: OnchainMintQuote = sqlx::query!(
            "SELECT id, reference, fee_total, amount, expiry, state  FROM onchain_mint_quotes WHERE id = $1",
            key
        )
        .map(|row| OnchainMintQuote {
            quote_id: row.id,
            reference: row.reference,
            fee_total: row.fee_total as u64,
            expiry: row.expiry as u64,
            state: MintOnchainState::from_str(&row.state).expect("invalid state in mint quote"),
            amount: row.amount as u64,
        })
        .fetch_one(&mut **tx)
        .await?;

        Ok(quote)
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn update_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMintQuote,
    ) -> Result<(), MonexoMintError> {
        sqlx::query!(
            "UPDATE onchain_mint_quotes SET state = $1 WHERE id = $2",
            quote.state.to_string(),
            quote.quote_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn add_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMeltQuote,
    ) -> Result<(), MonexoMintError> {
        sqlx::query!(
            "INSERT INTO onchain_melt_quotes (id, amount, address, reference, fee_total, fee_sat_per_vbyte, expiry, state, description) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            quote.quote_id,
            quote.amount as i64,
            quote.address,
            quote.reference,
            quote.fee_total as i64,
            quote.fee_sat_per_vbyte as i64,
            quote.expiry as i64,
            quote.state.to_string(),
            quote.description
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn get_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: &Uuid,
    ) -> Result<OnchainMeltQuote, MonexoMintError> {
        let quote: OnchainMeltQuote = sqlx::query!(
            "SELECT id, amount,address, reference, fee_total, fee_sat_per_vbyte, expiry, state, description  FROM onchain_melt_quotes WHERE id = $1",
            key
        )
        .map(|row| OnchainMeltQuote {
            quote_id: row.id,
            address: row.address,
            reference: row.reference,
            amount: row.amount as u64,
            fee_total: row.fee_total as u64,
            fee_sat_per_vbyte: row.fee_sat_per_vbyte as u32,
            expiry: row.expiry as u64,
            state: MeltOnchainState::from_str(&row.state).expect("invalid state in melt quote"),
            description: row.description
        })
        .fetch_one(&mut **tx)
        .await?;

        Ok(quote)
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn update_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &OnchainMeltQuote,
    ) -> Result<(), MonexoMintError> {
        sqlx::query!(
            "UPDATE onchain_melt_quotes SET state = $1 WHERE id = $2",
            quote.state.to_string(),
            quote.quote_id
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}
