use std::time::Duration;

use console::{style, Term};
use indicatif::{ProgressBar, ProgressStyle};
use monexo_wallet::{http::CrossPlatformHttpClient, localstore::sqlite::SqliteLocalStore, wallet::Wallet};
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
        "New total balance {} (sat)",
        style(wallet.get_balance().await?.to_formatted_string(&Locale::en)).cyan()
    ))?;
    Ok(())
}
