
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Parser)]
pub struct DatabaseConfig {
    #[clap(long, env = "MINT_DB_URL")]
    pub db_url: String,

    #[clap(long, default_value_t = 5, env = "MINT_DB_MAX_CONNECTIONS")]
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            db_url: "".to_owned(),
            max_connections: 5,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MintConfig {
    // pub privatekey: String,
    // pub derivation_path: Option<String>,
    // pub info: MintInfoConfig,
    // pub lightning_fee: LightningFeeConfig,
    // pub server: ServerConfig,
    pub btconchain_backend: Option<BtcOnchainConfig>,
    // pub lightning_backend: Option<LightningType>,
    // pub tracing: Option<TracingConfig>,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Parser)]
pub struct BtcOnchainConfig {
    #[clap(
        long,
        default_value_t = 1,
        env = "MINT_BTC_ONCHAIN_BACKEND_MIN_CONFIRMATIONS"
    )]
    pub min_confirmations: u8,

    #[clap(
        long,
        default_value_t = 10_000,
        env = "MINT_BTC_ONCHAIN_BACKEND_MIN_AMOUNT"
    )]
    pub min_amount: u64,

    #[clap(
        long,
        default_value_t = 1_000_000,
        env = "MINT_BTC_ONCHAIN_BACKEND_MAX_AMOUNT"
    )]
    pub max_amount: u64,
}

impl Default for BtcOnchainConfig {
    fn default() -> Self {
        Self {
            min_confirmations: 1,
            min_amount: 10_000,
            max_amount: 1_000_000,
        }
    }
}
