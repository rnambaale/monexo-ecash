use std::str::FromStr;

use async_trait::async_trait;

use monexo_core::{
    dhke,
    keyset::{KeysetId, MintKeySetInfo},
    primitives::{
        BtcOnchainMeltQuote, BtcOnchainMintQuote, CurrencyUnit, MeltBtcOnchainState,
        MintBtcOnchainState,
    },
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
        quote: &BtcOnchainMintQuote,
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
    ) -> Result<BtcOnchainMintQuote, MonexoMintError> {
        let quote: BtcOnchainMintQuote = sqlx::query!(
            "SELECT id, reference, fee_total, amount, expiry, state  FROM onchain_mint_quotes WHERE id = $1",
            key
        )
        .map(|row| BtcOnchainMintQuote {
            quote_id: row.id,
            reference: row.reference,
            fee_total: row.fee_total as u64,
            expiry: row.expiry as u64,
            state: MintBtcOnchainState::from_str(&row.state).expect("invalid state in mint quote"),
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

    #[instrument(level = "debug", skip(self), err)]
    async fn add_onchain_melt_quote(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        quote: &BtcOnchainMeltQuote,
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
    ) -> Result<BtcOnchainMeltQuote, MonexoMintError> {
        let quote: BtcOnchainMeltQuote = sqlx::query!(
            "SELECT id, amount,address, reference, fee_total, fee_sat_per_vbyte, expiry, state, description  FROM onchain_melt_quotes WHERE id = $1",
            key
        )
        .map(|row| BtcOnchainMeltQuote {
            quote_id: row.id,
            address: row.address,
            reference: row.reference,
            amount: row.amount as u64,
            fee_total: row.fee_total as u64,
            fee_sat_per_vbyte: row.fee_sat_per_vbyte as u32,
            expiry: row.expiry as u64,
            state: MeltBtcOnchainState::from_str(&row.state).expect("invalid state in melt quote"),
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
        quote: &BtcOnchainMeltQuote,
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

    #[instrument(level = "debug", skip(self), err)]
    async fn add_keyset_info(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        keyset: MintKeySetInfo,
    ) -> Result<(), MonexoMintError> {
        sqlx::query!(
            "INSERT INTO keysets (id, unit, active, valid_from, valid_to, derivation_path, max_order, input_fee_ppk, derivation_path_index) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT(id) DO UPDATE SET
            unit = excluded.unit,
            active = excluded.active,
            valid_from = excluded.valid_from,
            valid_to = excluded.valid_to,
            derivation_path = excluded.derivation_path,
            max_order = excluded.max_order,
            input_fee_ppk = excluded.input_fee_ppk,
            derivation_path_index = excluded.derivation_path_index",
            keyset.id,
            keyset.unit.to_string(),
            keyset.active,
            keyset.valid_from as i64,
            keyset.valid_to.map(|v| v as i32),
            keyset.derivation_path,
            keyset.max_order as i64,
            keyset.input_fee_ppk as i64,
            keyset.derivation_path_index,
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    #[instrument(level = "debug", skip(self), err)]
    async fn get_keyset_info(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        id: &KeysetId,
    ) -> Result<MintKeySetInfo, MonexoMintError> {
        let keyset: MintKeySetInfo = sqlx::query!(
            "SELECT id, unit, active, valid_from, valid_to, derivation_path, max_order, input_fee_ppk, derivation_path_index FROM keysets WHERE id = $1",
            id.to_string()
        )
        .map(|row| {
            let row_valid_to = row.valid_to;
            let row_valid_to: Option<u64> = row_valid_to.and_then(|n| n.try_into().ok());
            let row_derivation_path = Some(row.derivation_path);

            MintKeySetInfo {
                id: row.id,
                unit: CurrencyUnit::from_str(&row.unit).expect("invalid currency unit in keyset"),
                active: row.active,
                valid_from: row.valid_from as u64,
                valid_to: row_valid_to,
                derivation_path: row_derivation_path,
                max_order: row.max_order as u8,
                input_fee_ppk: row.input_fee_ppk.unwrap() as u64,
                derivation_path_index: row.derivation_path_index,
            }
        })
        .fetch_one(&mut **tx)
        .await?;

        Ok(keyset)
    }

    async fn get_keyset_infos(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Vec<MintKeySetInfo>, MonexoMintError> {
        let keysets = sqlx::query!("SELECT * FROM keysets")
            .fetch_all(&mut **tx)
            .await?
            .into_iter()
            .map(|row| {
                let row_valid_to = row.valid_to;
                let row_valid_to: Option<u64> = row_valid_to.and_then(|n| n.try_into().ok());
                let row_derivation_path = Some(row.derivation_path);

                MintKeySetInfo {
                    id: row.id,
                    unit: CurrencyUnit::from_str(&row.unit)
                        .expect("invalid currency unit in keyset"),
                    active: row.active,
                    valid_from: row.valid_from as u64,
                    valid_to: row_valid_to,
                    derivation_path: row_derivation_path,
                    max_order: row.max_order as u8,
                    input_fee_ppk: row.input_fee_ppk.unwrap() as u64,
                    derivation_path_index: row.derivation_path_index,
                }
            })
            .collect::<Vec<MintKeySetInfo>>();

        Ok(keysets)
    }

    //     async fn get_keyset_infos(&self) -> Result<Vec<MintKeySetInfo>, Self::Err> {
    //         let mut transaction = self.pool.begin().await.map_err(Error::from)?;
    //         let recs = sqlx::query(
    //             r#"
    // SELECT *
    // FROM keyset;
    //         "#,
    //         )
    //         .fetch_all(&mut *transaction)
    //         .await
    //         .map_err(Error::from);

    //         match recs {
    //             Ok(recs) => {
    //                 transaction.commit().await.map_err(Error::from)?;
    //                 Ok(recs
    //                     .into_iter()
    //                     .map(sqlite_row_to_keyset_info)
    //                     .collect::<Result<_, _>>()?)
    //             }
    //             Err(err) => {
    //                 tracing::error!("SQLite could not get keyset info");
    //                 if let Err(err) = transaction.rollback().await {
    //                     tracing::error!("Could not rollback sql transaction: {}", err);
    //                 }
    //                 Err(err.into())
    //             }
    //         }
    //     }
}
