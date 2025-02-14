use std::env;
use monexomint::{self, config::MintConfig, mint::MintBuilder};

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let app_env = match env::var("MINT_APP_ENV") {
        Ok(v) if v.trim() == "dev" => AppEnv::Dev,
        _ => AppEnv::Prod,
    };

    println!("Running in {app_env} mode");

    if app_env == AppEnv::Dev {
        match dotenvy::dotenv() {
            Ok(path) => println!(".env read successfully from {}", path.display()),
            Err(e) => panic!("Could not load .env file: {e}"),
        };
    }

    let MintConfig {
        privatekey,
        derivation_path,
        info,
        server,
        btconchain_backend,
        database,
    } = MintConfig::read_config_with_defaults();

    let mint = MintBuilder::new()
        .with_server(Some(server))
        .with_mint_info(Some(info))
        .with_private_key(privatekey)
        .with_derivation_path(derivation_path)
        .with_db(Some(database))
        .with_btc_onchain(btconchain_backend)
        .build()
        .await;

    monexomint::server::run_server(mint?).await
}

#[derive(Debug, PartialEq, Eq)]
pub enum AppEnv {
    Dev,
    Prod,
}

impl core::fmt::Display for AppEnv {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Dev => write!(f, "dev"),
            Self::Prod => write!(f, "prod"),
        }
    }
}
