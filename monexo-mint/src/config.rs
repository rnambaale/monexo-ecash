use std::{env, net::SocketAddr};

use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(arg_required_else_help(true))]
pub struct Opts {
    #[clap(long, env = "MINT_PRIVATE_KEY")]
    pub privatekey: String,
    #[clap(long, env = "MINT_DERIVATION_PATH")]
    pub derivation_path: Option<String>,
    #[clap(long, env = "UGX_MINT_DERIVATION_PATH")]
    pub ugx_derivation_path: Option<String>,
    #[clap(flatten)]
    pub info: MintInfoConfig,
    #[clap(flatten)]
    pub server: ServerConfig,
    #[clap(flatten)]
    pub database: DatabaseConfig,
    #[clap(flatten)]
    pub tracing: Option<TracingConfig>,
}

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

#[derive(Debug, Clone, Default, Parser)]
pub struct TracingConfig {
    #[clap(long, env = "MINT_TRACING_ENDPOINT")]
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MintConfig {
    pub privatekey: String,
    pub derivation_path: Option<String>,
    pub ugx_derivation_path: Option<String>,
    pub info: MintInfoConfig,
    pub server: ServerConfig,
    pub onchain_backend: Option<OnchainConfig>,
    pub tracing: Option<TracingConfig>,
    pub database: DatabaseConfig,
}

impl From<(Opts, OnchainConfig)> for MintConfig {
    fn from((opts, onchain_config): (Opts, OnchainConfig)) -> Self {
        Self {
            privatekey: opts.privatekey,
            derivation_path: opts.derivation_path,
            ugx_derivation_path: opts.ugx_derivation_path,
            info: opts.info,
            server: opts.server,
            onchain_backend: Some(onchain_config),
            tracing: opts.tracing,
            database: opts.database,
        }
    }
}

impl MintConfig {
    pub fn read_config_with_defaults() -> Self {
        let opts: Opts = Opts::parse();

        let onchain_config: OnchainConfig = OnchainConfig::parse();

        (opts, onchain_config).into()
    }
}

impl MintConfig {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        private_key: String,
        derivation_path: Option<String>,
        ugx_derivation_path: Option<String>,
        info: MintInfoConfig,
        server: ServerConfig,
        database: DatabaseConfig,
        onchain_backend: Option<OnchainConfig>,
        tracing: Option<TracingConfig>,
    ) -> Self {
        Self {
            privatekey: private_key,
            server,
            derivation_path,
            ugx_derivation_path,
            info,
            onchain_backend,
            database,
            tracing,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Parser)]
pub struct OnchainConfig {
    #[clap(
        long,
        default_value_t = 1,
        env = "MINT_ONCHAIN_BACKEND_MIN_CONFIRMATIONS"
    )]
    pub min_confirmations: u8,

    #[clap(
        long,
        default_value_t = 10_000_000,
        env = "MINT_ONCHAIN_BACKEND_MIN_AMOUNT"
    )]
    pub min_amount: u64,

    #[clap(
        long,
        default_value_t = 1_000_000_000,
        env = "MINT_ONCHAIN_BACKEND_MAX_AMOUNT"
    )]
    pub max_amount: u64,
}

impl Default for OnchainConfig {
    fn default() -> Self {
        Self {
            min_confirmations: 1,
            min_amount: 10_000,
            max_amount: 1_000_000,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct BuildParams {
    pub commit_hash: Option<String>,
    pub build_time: Option<String>,
    pub cargo_pkg_version: Option<String>,
}

impl BuildParams {
    pub fn from_env() -> Self {
        Self {
            commit_hash: env::var("COMMITHASH").ok(),
            build_time: env::var("BUILDTIME").ok(),
            cargo_pkg_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }
    }

    pub fn full_version(&self) -> String {
        format!(
            "monexo-mint/{}-{}",
            self.cargo_pkg_version
                .as_ref()
                .unwrap_or(&"unknown".to_string()),
            self.commit_hash.as_ref().unwrap_or(&"unknown".to_string())
        )
    }
}

#[derive(Debug, Clone, Parser)]
pub struct ServerConfig {
    #[clap(long, default_value = "[::]:3338", env = "MINT_HOST_PORT")]
    pub host_port: SocketAddr,
    // #[clap(long, env = "MINT_SERVE_WALLET_PATH")]
    // pub serve_wallet_path: Option<PathBuf>,
    #[clap(long, env = "MINT_API_PREFIX")]
    pub api_prefix: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host_port: "[::]:3338".to_string().parse().expect("invalid host port"),
            // serve_wallet_path: None,
            api_prefix: None,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default, Parser)]
pub struct MintInfoConfig {
    #[clap(long, default_value = "monexo-mint", env = "MINT_INFO_NAME")]
    pub name: Option<String>,

    #[clap(long, default_value_t = true, env = "MINT_INFO_VERSION")]
    pub version: bool,

    #[clap(long, env = "MINT_INFO_DESCRIPTION")]
    pub description: Option<String>,

    #[clap(long, env = "MINT_INFO_DESCRIPTION_LONG")]
    pub description_long: Option<String>,

    #[clap(long, env = "MINT_INFO_CONTACT_EMAIL")]
    pub contact_email: Option<String>,

    #[clap(long, env = "MINT_INFO_CONTACT_TWITTER")]
    pub contact_twitter: Option<String>,

    #[clap(long, env = "MINT_INFO_CONTACT_NOSTR")]
    pub contact_nostr: Option<String>,

    #[clap(long, env = "MINT_INFO_MOTD")]
    pub motd: Option<String>,
    // FIXME add missing fields for v1/info endpoint nut4/nut5 payment_methods, nut4 disabled flag
}

// impl From<MintInfoConfig> for Vec<ContactInfoResponse> {
//     fn from(info: MintInfoConfig) -> Vec<ContactInfoResponse> {
//         [
//             info.contact_email.map(ContactInfoResponse::email),
//             info.contact_twitter.map(ContactInfoResponse::twitter),
//             info.contact_nostr.map(ContactInfoResponse::nostr),
//         ]
//         .iter()
//         .filter_map(|contact| contact.to_owned())
//         .collect()
//     }
// }
