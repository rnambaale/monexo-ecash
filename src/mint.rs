use std::collections::HashSet;

use monexo_core::{blind::{BlindedMessage, BlindedSignature, TotalAmount}, dhke::Dhke, keyset::MintKeyset, primitives::BtcOnchainMeltQuote, proof::Proofs};
use sqlx::Transaction;

use crate::{config::{BtcOnchainConfig, BuildParams, DatabaseConfig, MintConfig, MintInfoConfig, ServerConfig}, database::{postgres::PostgresDB, Database}, error::MonexoMintError};
use tracing::instrument;


#[derive(Clone)]
pub struct Mint<DB: Database = PostgresDB> {
    // pub lightning: Arc<dyn Lightning + Send + Sync>,
    // pub lightning_type: LightningType,
    pub keyset: MintKeyset,
    pub db: DB,
    pub dhke: Dhke,
    // pub onchain: Option<Arc<dyn BtcOnchain + Send + Sync>>,
    pub config: MintConfig,
    pub build_params: BuildParams,
}

impl<DB> Mint<DB>
where
    DB: Database,
{
    pub fn new(
        // lightning: Arc<dyn Lightning + Send + Sync>,
        // lightning_type: LightningType,
        db: DB,
        config: MintConfig,
        build_params: BuildParams,
    ) -> Self {
        Self {
            // lightning,
            // lightning_type,
            keyset: MintKeyset::new(
                &config.privatekey.clone(),
                &config.derivation_path.clone().unwrap_or_default(),
            ),
            db,
            dhke: Dhke::new(),
            config,
            build_params,
        }
    }

    pub fn create_blinded_signatures(
        &self,
        blinded_messages: &[BlindedMessage],
        keyset: &MintKeyset,
    ) -> Result<Vec<BlindedSignature>, MonexoMintError> {
        blinded_messages
            .iter()
            .map(|blinded_msg| {
                let private_key = keyset
                    .private_keys
                    .get(&blinded_msg.amount)
                    .ok_or(MonexoMintError::PrivateKeyNotFound)?;
                let blinded_sig = self.dhke.step2_bob(blinded_msg.b_, private_key)?;
                Ok(BlindedSignature {
                    id: keyset.keyset_id.clone(),
                    amount: blinded_msg.amount,
                    c_: blinded_sig,
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }

    #[instrument(level = "debug", skip(self, outputs, keyset), err)]
    pub async fn mint_tokens(
        &self,
        tx: &mut Transaction<'_, <DB as Database>::DB>,
        key: String,
        outputs: &[BlindedMessage],
        keyset: &MintKeyset,
        return_error: bool,
    ) -> Result<Vec<BlindedSignature>, MonexoMintError> {
        self.create_blinded_signatures(outputs, keyset)
    }

    pub async fn check_used_proofs(
        &self,
        tx: &mut Transaction<'_, <DB as Database>::DB>,
        proofs: &Proofs,
    ) -> Result<(), MonexoMintError> {
        let used_proofs = self.db.get_used_proofs(tx).await?.proofs();
        for used_proof in used_proofs {
            if proofs.proofs().contains(&used_proof) {
                return Err(MonexoMintError::ProofAlreadyUsed(format!("{used_proof:?}")));
            }
        }
        Ok(())
    }

    #[instrument(level = "debug", skip(self, proofs), err)]
    pub async fn melt_onchain(
        &self,
        quote: &BtcOnchainMeltQuote,
        proofs: &Proofs,
    ) -> Result<String, MonexoMintError> {
        let proofs_amount = proofs.total_amount();

        if proofs_amount < quote.amount {
            return Err(MonexoMintError::NotEnoughTokens(quote.amount));
        }

        let mut tx = self.db.begin_tx().await?;
        self.check_used_proofs(&mut tx, proofs).await?;

        // TODO: How do we actually send USDC coins on Solana
        // let send_response = self
        //     .onchain
        //     .as_ref()
        //     .expect("onchain backend not set")
        //     .send_coins(&quote.address, quote.amount, quote.fee_sat_per_vbyte)
        //     .await?;

        self.db.add_used_proofs(&mut tx, proofs).await?;
        tx.commit().await?;

        // Ok(send_response.txid)
        Ok("some placeholder txid".to_string())
    }

    fn has_duplicate_pubkeys(outputs: &[BlindedMessage]) -> bool {
        let mut uniq = HashSet::new();
        !outputs.iter().all(move |x| uniq.insert(x.b_))
    }

    #[instrument(level = "debug", skip_all, err)]
    pub async fn swap(
        &self,
        proofs: &Proofs,
        blinded_messages: &[BlindedMessage],
        keyset: &MintKeyset,
    ) -> Result<Vec<BlindedSignature>, MonexoMintError> {
        let mut tx = self.db.begin_tx().await?;
        self.check_used_proofs(&mut tx, proofs).await?;

        if Self::has_duplicate_pubkeys(blinded_messages) {
            return Err(MonexoMintError::SwapHasDuplicatePromises);
        }

        let sum_proofs = proofs.total_amount();

        let promises = self.create_blinded_signatures(blinded_messages, keyset)?;
        let amount_promises = promises.total_amount();
        if sum_proofs != amount_promises {
            return Err(MonexoMintError::SwapAmountMismatch(format!(
                "Swap amount mismatch: {sum_proofs} != {amount_promises}"
            )));
        }

        self.db.add_used_proofs(&mut tx, proofs).await?;
        tx.commit().await?;
        Ok(promises)
    }
}

#[derive(Debug, Default)]
pub struct MintBuilder {
    private_key: Option<String>,
    derivation_path: Option<String>,
    db_config: Option<DatabaseConfig>,
    mint_info_settings: Option<MintInfoConfig>,
    server_config: Option<ServerConfig>,
    btc_onchain_config: Option<BtcOnchainConfig>,
}

impl MintBuilder {
    pub fn new() -> Self {
        MintBuilder {
            private_key: None,
            derivation_path: None,
            db_config: None,
            mint_info_settings: None,
            server_config: None,
            btc_onchain_config: None,
        }
    }

    pub fn with_db(mut self, db_config: Option<DatabaseConfig>) -> Self {
        self.db_config = db_config;
        self
    }

    pub fn with_mint_info(mut self, mint_info: Option<MintInfoConfig>) -> Self {
        self.mint_info_settings = mint_info;
        self
    }

    pub fn with_server(mut self, server_config: Option<ServerConfig>) -> Self {
        self.server_config = server_config;
        self
    }

    pub fn with_derivation_path(mut self, derivation_path: Option<String>) -> Self {
        self.derivation_path = derivation_path;
        self
    }

    pub fn with_private_key(mut self, private_key: String) -> Self {
        self.private_key = Some(private_key);
        self
    }

    pub fn with_btc_onchain(mut self, btc_onchain_config: Option<BtcOnchainConfig>) -> Self {
        self.btc_onchain_config = btc_onchain_config;
        self
    }

    pub async fn build(self) -> Result<Mint<PostgresDB>, MonexoMintError> {
        let db_config = self.db_config.expect("db-config not set");
        let db = PostgresDB::new(&db_config).await?;
        db.migrate().await;

        Ok(Mint::new(
            db,
            MintConfig::new(
                self.private_key.expect("private-key not set"),
                self.derivation_path,
                self.mint_info_settings.unwrap_or_default(),
                self.server_config.unwrap_or_default(),
                db_config,
                self.btc_onchain_config,
            ),
            BuildParams::from_env(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use monexo_core::blind::{BlindedMessage, TotalAmount};
    use monexo_core::dhke;
    use monexo_core::fixture::read_fixture_as;
    use monexo_core::primitives::PostSwapRequest;
    use monexo_core::proof::Proofs;
    use testcontainers_modules::postgres::Postgres;
    use testcontainers::runners::AsyncRunner;
    use testcontainers::{ContainerAsync, ImageExt};

    use crate::{
        config::{DatabaseConfig, MintConfig}, database::postgres::PostgresDB, mint::Mint
    };

    async fn create_postgres_image() -> anyhow::Result<ContainerAsync<Postgres>> {
        Ok(Postgres::default()
            .with_host_auth()
            .with_tag("16.6-alpine")
            .start()
            .await?)
    }

    async fn create_mock_db_empty(port: u16) -> anyhow::Result<PostgresDB> {
        let connection_string =
            &format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port);
        let db = PostgresDB::new(&DatabaseConfig {
            db_url: connection_string.to_owned(),
            ..Default::default()
        })
        .await?;
        db.migrate().await;
        Ok(db)
    }

    async fn create_mint_from_mocks(
        mock_db: PostgresDB,
    ) -> anyhow::Result<Mint> {

        Ok(Mint::new(
            mock_db,
            MintConfig {
                privatekey: "TEST_PRIVATE_KEY".to_string(),
                derivation_path: Some("0/0/0/0".to_string()),
                ..Default::default()
            },
            Default::default(),
        ))
    }

    #[tokio::test]
    async fn test_create_blind_signatures() -> anyhow::Result<()> {
        let node = create_postgres_image().await?;

        let mint = create_mint_from_mocks(
            create_mock_db_empty(node.get_host_port_ipv4(5432).await?).await?,
        )
        .await?;

        let blinded_messages = vec![BlindedMessage {
            amount: 8,
            b_: dhke::public_key_from_hex(
                "02634a2c2b34bec9e8a4aba4361f6bf202d7fa2365379b0840afe249a7a9d71239",
            ),
            id: "00ffd48b8f5ecf80".to_owned(),
        }];

        let result = mint.create_blinded_signatures(&blinded_messages, &mint.keyset)?;

        assert_eq!(1, result.len());
        assert_eq!(8, result[0].amount);
        assert_eq!(
            dhke::public_key_from_hex(
                "037074c4f53e326ee14ed67125f387d160e0e729351471b69ad41f7d5d21071e15"
            ),
            result[0].c_
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_swap_zero() -> anyhow::Result<()> {
        let node = create_postgres_image().await?;
        let blinded_messages = vec![];
        let mint = create_mint_from_mocks(
            create_mock_db_empty(node.get_host_port_ipv4(5432).await?).await?,
        )
        .await?;

        let proofs = Proofs::empty();
        let result = mint.swap(&proofs, &blinded_messages, &mint.keyset).await?;

        assert!(result.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_swap_64_in_20() -> anyhow::Result<()> {
        let node = create_postgres_image().await?;
        let mint = create_mint_from_mocks(
            create_mock_db_empty(node.get_host_port_ipv4(5432).await?).await?,
        )
        .await?;
        let request = read_fixture_as::<PostSwapRequest>("post_swap_request_64_20.json")?;

        let result = mint
            .swap(&request.inputs, &request.outputs, &mint.keyset)
            .await?;
        assert_eq!(result.total_amount(), 64);

        let prv_last = result.get(result.len() - 2).expect("element not found");
        let last = result.last().expect("element not found");

        assert_eq!(prv_last.amount, 4);
        assert_eq!(last.amount, 16);
        Ok(())
    }

    #[tokio::test]
    async fn test_swap_duplicate_key() -> anyhow::Result<()> {
        let node = create_postgres_image().await?;
        let mint = create_mint_from_mocks(
            create_mock_db_empty(node.get_host_port_ipv4(5432).await?).await?,
        )
        .await?;
        let request = read_fixture_as::<PostSwapRequest>("post_swap_request_duplicate_key.json")?;

        let result = mint
            .swap(&request.inputs, &request.outputs, &mint.keyset)
            .await;
        assert!(result.is_err());
        Ok(())
    }
}
