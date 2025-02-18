pub mod client;
pub mod error;
pub mod http;
pub mod localstore;
pub mod secret;
pub mod config_path;
pub mod wallet;

// cd monexo-wallet
// sqlx database drop --database-url sqlite://wallet.db
// sqlx database create --database-url sqlite://wallet.db
// sqlx migrate run --database-url sqlite://wallet.db
// cargo sqlx prepare --database-url sqlite://wallet.db
