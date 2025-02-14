use monexo_core::{blind::{BlindedMessage, BlindedSignature}, dhke::Dhke, keyset::MintKeyset};
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
                &config.derivation_path.clone(),
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

    pub fn with_derivation_path(mut self, derivation_path: String) -> Self {
        self.derivation_path = Some(derivation_path);
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
                self.derivation_path.expect("deriation path not set"),
                self.mint_info_settings.unwrap_or_default(),
                self.server_config.unwrap_or_default(),
                db_config,
                self.btc_onchain_config,
            ),
            BuildParams::from_env(),
        ))
    }
}
