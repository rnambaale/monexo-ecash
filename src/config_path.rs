use dirs::home_dir;
use std::{fs::create_dir, path::PathBuf};

pub const ENV_DB_PATH: &str = "WALLET_DB_PATH";

/// Returns the path to the wallet database file.
///
/// The path is determined by the value of the `WALLET_DB_PATH` environment variable. If the
/// variable is not set, the function creates a `.monexo` directory in the user's home directory
/// and returns a path to a `wallet2.db` file in that directory.
///
/// # Examples
///
/// ```
/// let db_path = monexo_wallet::config_path::db_path();
/// println!("Database path: {}", db_path);
/// ```
pub fn db_path() -> String {
    std::env::var(ENV_DB_PATH).unwrap_or_else(|_| {
        let home = home_dir()
            .expect("home dir not found")
            .to_str()
            .expect("home dir is invalid")
            .to_owned();
        // in a sandboxed environment on mac the path looks like
        // /Users/$USER_NAME/Library/Containers/..... so we have are just ising the first 2 parts
        let home = home
            .split('/')
            .take(3)
            .collect::<Vec<&str>>()
            .join(std::path::MAIN_SEPARATOR_STR);
        let monexo_dir = format!("{}{}.monexo", home, std::path::MAIN_SEPARATOR);

        if !std::path::Path::new(&monexo_dir).exists() {
            create_dir(std::path::Path::new(&monexo_dir)).expect("failed to create .monexo dir");
        }

        format!("{monexo_dir}/wallet2.db")
    })
}

pub fn config_dir() -> PathBuf {
    let home = home_dir()
        .expect("home dir not found")
        .to_str()
        .expect("home dir is invalid")
        .to_owned();
    // in a sandboxed environment on mac the path looks like
    // /Users/$USER_NAME/Library/Containers/..... so we have are just ising the first 2 parts
    let home = home
        .split('/')
        .take(3)
        .collect::<Vec<&str>>()
        .join(std::path::MAIN_SEPARATOR_STR);
    let moksha_dir = format!("{}{}.moksha", home, std::path::MAIN_SEPARATOR);

    if !std::path::Path::new(&moksha_dir).exists() {
        create_dir(std::path::Path::new(&moksha_dir)).expect("failed to create .moksha dir");
    }
    PathBuf::from(moksha_dir)
}
