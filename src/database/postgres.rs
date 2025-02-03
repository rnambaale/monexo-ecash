use std::str::FromStr;

use async_trait::async_trait;

use monexo_core::{dhke, primitives::{BtcOnchainMintQuote, CurrencyUnit, MintBtcOnchainState}, proof::{Proof, Proofs}};
use sqlx::postgres::PgPoolOptions;
use tracing::instrument;
use uuid::Uuid;

use crate::{config::DatabaseConfig, error::MonexoMintError, model::Invoice};

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

    #[instrument(level = "debug", skip(self))]
    async fn get_pending_invoice(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: String,
    ) -> Result<Invoice, MonexoMintError> {
        let invoice: Invoice = sqlx::query!(
            "SELECT amount, payment_request FROM pending_invoices WHERE key = $1",
            key
        )
        .map(|row| Invoice {
            amount: row.amount as u64,
            payment_request: row.payment_request,
        })
        .fetch_one(&mut **tx)
        .await?;

        Ok(invoice)
    }

    #[instrument(level = "debug", skip(self))]
    async fn add_pending_invoice(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: String,
        invoice: &Invoice,
    ) -> Result<(), MonexoMintError> {
        sqlx::query!(
            "INSERT INTO pending_invoices (key, amount, payment_request) VALUES ($1, $2, $3)",
            key,
            invoice.amount as i64,
            invoice.payment_request
        )
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn delete_pending_invoice(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        key: String,
    ) -> Result<(), MonexoMintError> {
        sqlx::query!("DELETE FROM pending_invoices WHERE key = $1", key)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn add_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMintQuote,
    ) -> Result<(), MonexoMintError> {
        sqlx::query!(
            "INSERT INTO onchain_mint_quotes (id, amount, expiry, state) VALUES ($1, $2, $3, $4)",
            quote.quote_id,
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
    ) -> Result<BtcOnchainMintQuote, MonexoMintError> {
        let quote: BtcOnchainMintQuote = sqlx::query!(
            "SELECT id, amount, expiry, state  FROM onchain_mint_quotes WHERE id = $1",
            key
        )
        .map(|row| BtcOnchainMintQuote {
            quote_id: row.id,
            expiry: row.expiry as u64,
            state: MintBtcOnchainState::from_str(&row.state).expect("invalid state in mint quote"),
            amount: row.amount as u64,
            unit: CurrencyUnit::Usd,
        })
        .fetch_one(&mut **tx)
        .await?;

        Ok(quote)
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn update_onchain_mint_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMintQuote,
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
}
