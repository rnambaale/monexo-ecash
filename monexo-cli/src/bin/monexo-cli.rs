use clap::{Parser, Subcommand};
use console::{style, Term};
use dialoguer::Confirm;
use monexo_core::{
    primitives::{MeltOnchainState, PostMeltOnchainResponse, PostMintQuoteOnchainResponse},
    token::TokenV3,
};
use monexo_wallet::{http::CrossPlatformHttpClient, localstore::WalletKeysetFilter};
use monexocli::cli::{self, choose_mint, get_mints_with_balance};
use num_format::{Locale, ToFormattedString};
use qrcode::{render::unicode, QrCode};
use url::Url;

use std::{path::PathBuf, str::FromStr};

#[derive(Parser)]
#[command(arg_required_else_help(true))]
struct Opts {
    #[clap(short, long)]
    db_dir: Option<PathBuf>,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone)]
enum Command {
    /// Mint tokens
    Mint { amount: u64 },

    /// Pay micro USDC on chain
    PayOnchain { address: String, amount: u64 },

    /// Send tokens
    Send { amount: u64 },

    /// Receive tokens
    Receive { token: String },

    /// Show local balance
    Balance,

    /// Show version and configuration
    Info,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use monexo_wallet::localstore::sqlite::SqliteLocalStore;

    let cli = Opts::parse();

    let db_path = match cli.db_dir {
        Some(dir) => {
            std::fs::create_dir_all(dir.clone())?;
            dir.join("wallet2.db").to_str().unwrap().to_string()
        }

        None => monexo_wallet::config_path::db_path(),
    };

    let term = Term::stdout();
    let localstore = SqliteLocalStore::with_path(db_path.clone()).await?;
    let client = CrossPlatformHttpClient::new();

    let wallet = monexo_wallet::wallet::WalletBuilder::default()
        .with_client(client)
        .with_localstore(localstore)
        .build()
        .await
        .map_err(|e| {
            if matches!(
                e,
                monexo_wallet::error::MonexoWalletError::UnsupportedApiVersion
            ) {
                term.write_line("Error: Mint does not support /v1 api")
                    .expect("write_line failed");
                std::process::exit(1);
            }
            e
        })?;

    let mint_url = Url::parse("http://127.0.0.1:3338/").unwrap();

    let wallet_keysets = wallet.get_wallet_keysets().await;
    if wallet_keysets.unwrap().len() == 0 {
        wallet.add_mint_keysets(&mint_url).await?;
    }

    match cli.command {
        Command::Mint { amount } => {
            let mint_info = wallet.get_mint_info(&mint_url).await?;

            let (quote, fee) = {
                // TODO: Fetch this from backend
                let min_amount: u64 = 10_000_000;
                if amount < min_amount {
                    term.write_line(&format!(
                        "Amount too low. Minimum amount is {} (micro usd)",
                        min_amount.to_formatted_string(&Locale::en)
                    ))?;
                    return Ok(());
                }

                // TODO: Fetch this from backend
                let max_amount: u64 = 1_000_000_000;
                if amount > max_amount {
                    term.write_line(&format!(
                        "Amount too high. Maximum amount is {} (micro usd)",
                        max_amount.to_formatted_string(&Locale::en)
                    ))?;
                    return Ok(());
                }

                let PostMintQuoteOnchainResponse {
                    reference,
                    quote,
                    fee,
                    ..
                } = wallet.create_quote_onchain(&mint_url, amount).await?;

                term.write_line(&format!(
                    "Pay onchain to mint tokens,
                    \n amount: {amount}
                    \n fee: {fee}
                    \n you will receive tokens worth {} micro usd",
                    (amount - fee)
                ))?;

                let amount_usd = amount as f64 / 1_000_000 as f64;
                let address_string = mint_info.usdc_address;
                let token_mint = mint_info.usdc_token_mint;
                let bip21_code = format!("solana:{}?amount={}&spl-token={}&reference={}&label=Monexo&message=Thank%20you!", address_string, amount_usd, token_mint, reference);
                let image = QrCode::new(bip21_code)?
                    .render::<unicode::Dense1x2>()
                    .quiet_zone(true)
                    .build();
                term.write_line(&image)?;
                (quote, fee)
            };

            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets.get_active().expect("Keyset not found");

            let progress_bar = cli::progress_bar()?;
            progress_bar.set_message("Waiting for payment ...");

            loop {
                tokio::time::sleep_until(
                    tokio::time::Instant::now() + std::time::Duration::from_millis(500),
                )
                .await;

                if !wallet.is_quote_paid(&mint_url, quote.clone()).await? {
                    continue;
                }

                // FIXME store quote in db and add option to retry minting later
                let amount = amount - fee;
                let mint_result = wallet
                    .mint_tokens(&mint_url, wallet_keyset, amount.into(), quote.clone())
                    .await;

                match mint_result {
                    Ok(_) => {
                        progress_bar.finish_with_message("Tokens minted successfully.\n");
                        cli::show_total_balance(&wallet).await?;
                        break;
                    }
                    Err(monexo_wallet::error::MonexoWalletError::InvoiceNotPaidYet(_, _)) => {
                        continue;
                    }
                    Err(e) => {
                        term.write_line(&format!("General Error: {}", e))?;
                        break;
                    }
                }
            }
        }
        Command::Balance => {
            let total_balance = wallet.get_balance().await?;
            if total_balance > 0 {
                let mints = get_mints_with_balance(&wallet).await?;
                term.write_line(&format!(
                    "You have balances in {} mints",
                    style(mints.len()).cyan()
                ))?;

                for mint in mints {
                    term.write_line(&format!(
                        " - {} (micro usd)",
                        style(mint.to_formatted_string(&Locale::en)).cyan()
                    ))?;
                }
            }
            cli::show_total_balance(&wallet).await?;
        }
        Command::Info => {
            let wallet_version = style(env!("CARGO_PKG_VERSION")).cyan();
            let db_path = style(db_path).cyan();
            term.write_line(&format!("Version: {wallet_version}"))?;
            term.write_line(&format!("DB: {db_path}"))?;
        }
        Command::Send { amount } => {
            let mint_balance = choose_mint(&wallet).await?;
            if mint_balance < amount {
                term.write_line("Error: Not enough tokens in mint")?;
                return Ok(());
            }

            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets.get_active().expect("no active keyset found");

            term.write_line(&format!("Sending tokens from mint"))?;
            let result = wallet.send_tokens(&mint_url, wallet_keyset, amount).await?;
            let tokens: String = result.try_into()?;

            term.write_line(&format!("Result {amount} (micro usd):\n{tokens}"))?;
            cli::show_total_balance(&wallet).await?;
        }
        Command::Receive { token } => {
            let token: TokenV3 = TokenV3::from_str(&token)?;
            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets.get_active().expect("no active keyset found");

            wallet
                .receive_tokens(&mint_url, wallet_keyset, &token)
                .await?;
            cli::show_total_balance(&wallet).await?;
        }
        Command::PayOnchain { address, amount } => {
            // TODO: Fetch this from backend
            let min_amount: u64 = 10_000_000;
            if amount < min_amount {
                term.write_line(&format!(
                    "Amount too low. Minimum amount is {} (micro usd)",
                    min_amount.to_formatted_string(&Locale::en)
                ))?;
                return Ok(());
            }

            // TODO: Fetch this from backend
            let max_amount: u64 = 1_000_000_000;
            if amount > max_amount {
                term.write_line(&format!(
                    "Amount too high. Maximum amount is {} (micro usd)",
                    max_amount.to_formatted_string(&Locale::en)
                ))?;
                return Ok(());
            }

            let mint_balance = choose_mint(&wallet).await?;
            if mint_balance < amount {
                term.write_line("Error: Not enough tokens in mint")?;
                return Ok(());
            }

            let wallet_keysets = wallet.get_wallet_keysets().await?;
            let wallet_keyset = wallet_keysets.get_active().expect("no active keyset found");

            let quotes = wallet
                .get_melt_quote_onchain(&mint_url, address.clone(), amount)
                .await?;

            if quotes.is_empty() {
                term.write_line("Error: No quotes found")?;
                return Ok(());
            }

            let quote = quotes.first().expect("No quotes found");

            term.write_line(&format!(
                "Create onchain transaction to melt tokens: amount {} - fee {} = {} (micro usd)\n{}\n",
                amount,
                quote.fee,
                amount - quote.fee,
                address
            ))?;

            let pay_confirmed = Confirm::new().with_prompt("Confirm payment?").interact()?;

            if !pay_confirmed {
                return Ok(());
            }

            let PostMeltOnchainResponse { state, txid } =
                wallet.pay_onchain(&mint_url, wallet_keyset, quote).await?;

            if let Some(txid) = txid.clone() {
                term.write_line(&format!("Created transaction: {}\n", &txid))?;
            }

            let progress_bar = cli::progress_bar()?;
            progress_bar.set_message("Waiting for payment confirmation ...");

            loop {
                tokio::time::sleep(std::time::Duration::from_millis(2_000)).await;

                // FIXME
                if state == MeltOnchainState::Paid
                    || wallet
                        .is_onchain_paid(&mint_url, quote.quote.clone())
                        .await?
                {
                    progress_bar.finish_with_message("\nTokens melted successfully\n");
                    cli::show_total_balance(&wallet).await?;
                    break;
                } else {
                    continue;
                }
            }
        }
    }
    Ok(())
}
