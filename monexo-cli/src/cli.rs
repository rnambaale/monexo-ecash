use std::process::exit;
use std::time::Duration;

use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use monexo_wallet::error::MonexoWalletError;
use monexo_wallet::{
    http::CrossPlatformHttpClient, localstore::sqlite::SqliteLocalStore, wallet::Wallet,
};
use num_format::Locale;
use num_format::ToFormattedString;

pub fn progress_bar() -> anyhow::Result<ProgressBar> {
    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.cyan} {msg}")?);
    Ok(pb)
}

pub async fn show_total_balance(
    wallet: &Wallet<SqliteLocalStore, CrossPlatformHttpClient>,
) -> anyhow::Result<()> {
    let term = Term::stdout();
    term.write_line(&format!(
        "New total balance {} (micro usd)",
        style(wallet.get_balance().await?.to_formatted_string(&Locale::en)).cyan()
    ))?;
    Ok(())
}

pub async fn choose_mint(
    wallet: &Wallet<SqliteLocalStore, CrossPlatformHttpClient>,
) -> Result<u64, MonexoWalletError> {
    let mints = get_mints_with_balance(wallet).await?;

    if mints.is_empty() {
        println!("No mints found.");
        exit(0)
    }

    Ok(mints[0].clone())
}

pub async fn get_mints_with_balance(
    wallet: &Wallet<SqliteLocalStore, CrossPlatformHttpClient>,
) -> Result<Vec<u64>, MonexoWalletError> {
    let all_proofs = wallet.get_proofs().await?;

    let keysets = wallet.get_wallet_keysets().await?;
    if keysets.is_empty() {
        println!("No mints found. Add a mint first with 'monexo-cli add-mint <mint-url>'");
        exit(0)
    }
    Ok(keysets
        .into_iter()
        .filter(|k| k.active)
        .map(|k| all_proofs.proofs_by_keyset(&k.keyset_id).total_amount())
        .collect::<Vec<u64>>())
}
