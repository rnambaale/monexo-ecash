use solana_client::{
    nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config,
};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::signer::EncodableKey;
use solana_sdk::{signature::Keypair, signer::Signer};
use std::str::FromStr;

use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{Duration, Utc};
use monexo_core::primitives::{
    MeltOnchainState, MintOnchainState, OnchainMeltQuote, OnchainMintQuote, PostMeltOnchainRequest,
    PostMeltOnchainResponse, PostMeltQuoteOnchainRequest, PostMeltQuoteOnchainResponse,
    PostMintOnchainRequest, PostMintOnchainResponse, PostMintQuoteOnchainRequest,
    PostMintQuoteOnchainResponse,
};
use solana_transaction_status::UiTransactionTokenBalance;
use solana_transaction_status_client_types::option_serializer::OptionSerializer;
use solana_transaction_status_client_types::{
    UiInstruction, UiMessage, UiParsedInstruction, UiTransactionEncoding,
};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::{database::Database, error::MonexoMintError, mint::Mint};

#[utoipa::path(
    post,
    path = "/v1/mint/quote/onchain",
    request_body = PostMintQuoteOnchainRequest,
    responses(
        (status = 200, description = "post mint quote", body = [PostMintQuoteOnchainResponse])
    ),
)]
#[instrument(name = "post_mint_quote_onchain", skip(mint), err)]
pub async fn post_mint_quote_onchain(
    State(mint): State<Mint>,
    Json(request): Json<PostMintQuoteOnchainRequest>,
) -> Result<Json<PostMintQuoteOnchainResponse>, MonexoMintError> {
    let onchain_config = mint.config.onchain_backend.unwrap_or_default();

    if request.amount < onchain_config.min_amount {
        return Err(MonexoMintError::InvalidAmount(format!(
            "amount is too low. Min amount is {}",
            onchain_config.min_amount
        )));
    }

    if request.amount > onchain_config.max_amount {
        return Err(MonexoMintError::InvalidAmount(format!(
            "amount is too high. Max amount is {}",
            onchain_config.max_amount
        )));
    }

    let quote_id = Uuid::new_v4();
    let reference = Keypair::new().pubkey().to_string();
    // let reference = Pubkey::new_unique(); ??

    let quote = OnchainMintQuote {
        quote_id,
        reference,
        amount: request.amount,
        fee_total: ((request.amount as f64) * 0.01) as u64,
        expiry: quote_onchain_expiry(),
        state: MintOnchainState::Unpaid,
    };

    let mut tx = mint.db.begin_tx().await?;
    mint.db.add_onchain_mint_quote(&mut tx, &quote).await?;
    tx.commit().await?;
    Ok(Json(quote.into()))
}

#[utoipa::path(
    get,
    path = "/v1/mint/quote/onchain/{quote_id}",
    responses(
        (status = 200, description = "get mint quote by id", body = [PostMintQuoteOnchainResponse])
    ),
    params(
        ("quote_id" = String, Path, description = "quote id"),
    )
)]
#[instrument(name = "get_mint_quote_onchain", skip(mint), err)]
pub async fn get_mint_quote_onchain(
    Path(quote_id): Path<String>,
    State(mint): State<Mint>,
) -> Result<Json<PostMintQuoteOnchainResponse>, MonexoMintError> {
    info!("get_quote onchain: {}", quote_id);

    let mut tx = mint.db.begin_tx().await?;
    let quote = mint
        .db
        .get_onchain_mint_quote(&mut tx, &Uuid::from_str(quote_id.as_str())?)
        .await?;
    tx.commit().await?;

    let state = match quote.state {
        MintOnchainState::Issued => quote.state,
        _ => {
            let monexo_wallet_keypair =
                Keypair::read_from_file(mint.config.derivation_path.unwrap())
                    .expect("Failed to load keypair");

            let monexo_wallet_pub_key = monexo_wallet_keypair
                .try_pubkey()
                .expect("Failed to load mint pubkey");

            let verified = is_paid_onchain(
                quote.amount,
                &quote.reference,
                &monexo_wallet_pub_key.to_string(),
            )
            .await;

            match verified {
                false => MintOnchainState::Unpaid,
                true => MintOnchainState::Paid,
            }
        }
    };

    Ok(Json(OnchainMintQuote { state, ..quote }.into()))
}

#[utoipa::path(
    post,
    path = "/v1/mint/onchain",
    request_body = PostMintOnchainRequest,
    responses(
        (status = 200, description = "post mint", body = [PostMintOnchainResponse])
    ),
)]
#[instrument(name = "post_mint_onchain", skip(mint), err)]
pub async fn post_mint_onchain(
    State(mint): State<Mint>,
    Json(request): Json<PostMintOnchainRequest>,
) -> Result<Json<PostMintOnchainResponse>, MonexoMintError> {
    // TODO Figure out the quote has been paid, only then do you mint the tokens
    // TODO Check that the sum of secrets is equal to the quote.amount

    let mut tx = mint.db.begin_tx().await?;
    let signatures = mint
        .mint_tokens(&mut tx, request.quote.clone(), &request.outputs, false)
        .await?;

    let old_quote = &mint
        .db
        .get_onchain_mint_quote(&mut tx, &Uuid::from_str(request.quote.as_str())?)
        .await?;

    mint.db
        .update_onchain_mint_quote(
            &mut tx,
            &OnchainMintQuote {
                state: MintOnchainState::Issued,
                ..old_quote.clone()
            },
        )
        .await?;
    tx.commit().await?;
    Ok(Json(PostMintOnchainResponse { signatures }))
}

#[utoipa::path(
    post,
    path = "/v1/melt/quote/onchain",
    request_body = PostMeltQuoteOnchainRequest,
    responses(
        (status = 200, description = "post melt quote", body = [Vec<PostMeltQuoteOnchainResponse>])
    ),
)]
#[instrument(name = "post_melt_quote_onchain", skip(mint), err)]
pub async fn post_melt_quote_onchain(
    State(mint): State<Mint>,
    Json(melt_request): Json<PostMeltQuoteOnchainRequest>,
) -> Result<Json<Vec<PostMeltQuoteOnchainResponse>>, MonexoMintError> {
    let PostMeltQuoteOnchainRequest { address, amount } = melt_request;

    let onchain_config = mint.config.onchain_backend.unwrap_or_default();

    if amount < onchain_config.min_amount {
        return Err(MonexoMintError::InvalidAmount(format!(
            "amount is too low. Min amount is {}",
            onchain_config.min_amount
        )));
    }

    if amount > onchain_config.max_amount {
        return Err(MonexoMintError::InvalidAmount(format!(
            "amount is too high. Max amount is {}",
            onchain_config.max_amount
        )));
    }

    let reference = Keypair::new().pubkey().to_string();

    let quote = OnchainMeltQuote {
        quote_id: Uuid::new_v4(),
        address,
        reference,
        amount,
        fee_total: ((amount as f64) * 0.01) as u64,
        fee_sat_per_vbyte: 0, //fee_response.sat_per_vbyte,
        expiry: quote_onchain_expiry(),
        state: MeltOnchainState::Unpaid,
        description: None,
    };

    let mut tx = mint.db.begin_tx().await?;
    mint.db.add_onchain_melt_quote(&mut tx, &quote).await?;
    tx.commit().await?;

    Ok(Json(vec![quote.into()]))
}

#[utoipa::path(
    get,
    path = "/v1/melt/quote/onchain/{quote_id}",
    responses(
        (status = 200, description = "post mint quote", body = [PostMeltQuoteOnchainResponse])
    ),
    params(
        ("quote_id" = String, Path, description = "quote id"),
    )
)]
#[instrument(name = "get_melt_quote_onchain", skip(mint), err)]
pub async fn get_melt_quote_onchain(
    Path(quote_id): Path<String>,
    State(mint): State<Mint>,
) -> Result<Json<PostMeltQuoteOnchainResponse>, MonexoMintError> {
    info!("get_melt_quote onchain: {}", quote_id);
    let mut tx = mint.db.begin_tx().await?;
    let quote = mint
        .db
        .get_onchain_melt_quote(&mut tx, &Uuid::from_str(quote_id.as_str())?)
        .await?;
    let expected_paid_amount = quote.amount - quote.fee_total;
    let paid = is_paid_onchain(expected_paid_amount, &quote.reference, &quote.address).await;

    let state = match paid {
        true => MeltOnchainState::Paid,
        false => MeltOnchainState::Unpaid,
    };

    if paid {
        mint.db
            .update_onchain_melt_quote(
                &mut tx,
                &OnchainMeltQuote {
                    state: state.clone(),
                    ..quote.clone()
                },
            )
            .await?;
    }

    Ok(Json(OnchainMeltQuote { state, ..quote }.into()))
}

#[utoipa::path(
    post,
    path = "/v1/melt/onchain",
    request_body = PostMeltOnchainRequest,
    responses(
        (status = 200, description = "post melt", body = [PostMeltOnchainResponse])
    ),
)]
#[instrument(name = "post_melt_onchain", skip(mint), err)]
pub async fn post_melt_onchain(
    State(mint): State<Mint>,
    Json(melt_request): Json<PostMeltOnchainRequest>,
) -> Result<Json<PostMeltOnchainResponse>, MonexoMintError> {
    let mut tx = mint.db.begin_tx().await?;
    let quote = mint
        .db
        .get_onchain_melt_quote(&mut tx, &Uuid::from_str(melt_request.quote.as_str())?)
        .await?;

    let txid = mint.melt_onchain(&quote, &melt_request.inputs).await?;
    let expected_paid_amount = quote.amount - quote.fee_total;
    let paid = is_paid_onchain(expected_paid_amount, &quote.reference, &quote.address).await;

    // FIXME  compute correct state
    let state = match paid {
        true => MeltOnchainState::Paid,
        false => MeltOnchainState::Unpaid,
    };

    mint.db
        .update_onchain_melt_quote(
            &mut tx,
            &OnchainMeltQuote {
                state: state.clone(),
                ..quote
            },
        )
        .await?;
    tx.commit().await?;

    Ok(Json(PostMeltOnchainResponse {
        state,
        txid: Some(txid.to_string()),
    }))
}

#[allow(dead_code)]
fn quote_onchain_expiry() -> u64 {
    let now = Utc::now() + Duration::try_minutes(30).expect("invalid duration");
    now.timestamp() as u64
}

async fn is_paid_onchain(
    amount: u64,
    transaction_reference: &str,
    destination_wallet_pub_key: &str,
) -> bool {
    // Expected values:
    // let usdc_spl_mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
    // let transaction_reference = "5t6gQ7Mnr3mmsFYquFGwgEKokq9wrrUgCpwWab93LmLL";
    // let expected_owner = "HVasUUKPrmrAuBpDFiu8BxQKzrMYY5DvyuNXamvaG2nM";
    // let expected_amount_str = "10"; // as reported in uiAmountString

    let client = RpcClient::new("https://api.devnet.solana.com".into());
    let config = GetConfirmedSignaturesForAddress2Config {
        limit: Some(20),
        commitment: Some(CommitmentConfig::confirmed()),
        ..GetConfirmedSignaturesForAddress2Config::default()
    };

    let reference =
        Pubkey::from_str(&transaction_reference).expect("reference is not a valid public key");
    let signatures = client
        .get_signatures_for_address_with_config(&reference, config)
        .await
        .expect("onchain backend not configured");

    let first_signature = match signatures.first() {
        Some(sig) => {
            Signature::from_str(&sig.signature).expect("could not parse transaction signature")
        }
        None => {
            eprintln!("No transaction signatures found");
            return false;
        }
    };

    let tx = match client
        .get_transaction(&first_signature, UiTransactionEncoding::JsonParsed)
        .await
    {
        Ok(tx) => tx,
        _ => {
            eprintln!("could not fetch transaction details");
            return false;
        }
    };

    let usdc_spl_mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";

    // === 1. Verify the post-transaction token balances ===
    let meta = match &tx.transaction.meta {
        Some(m) => m,
        None => {
            eprintln!("No meta data in transaction");
            return false;
        }
    };

    let pre_ata_token_balance = get_mint_token_balance(
        &meta.pre_token_balances,
        &usdc_spl_mint,
        destination_wallet_pub_key,
    );

    let post_ata_token_balance = get_mint_token_balance(
        &meta.post_token_balances,
        usdc_spl_mint,
        destination_wallet_pub_key,
    );

    let mint_balance_change = post_ata_token_balance - pre_ata_token_balance;

    if mint_balance_change < amount {
        eprintln!("Post token balance verification at destination failed.");
        return false;
    }

    // === 2. Verify the transfer instruction details ===
    // We expect the transaction message to be parsed.
    let ui_tx = match &tx.transaction.transaction {
        // If the transaction is encoded as JSON, extract the parsed message.
        solana_transaction_status::EncodedTransaction::Json(ui_tx) => ui_tx,
        _ => {
            eprintln!("Transaction is not JSON parsed");
            return false;
        }
    };

    let parsed_msg = match &ui_tx.message {
        UiMessage::Parsed(msg) => msg,
        _ => {
            eprintln!("Transaction message is not parsed");
            return false;
        }
    };

    // Iterate over the instructions to find our transfer
    let mut transfer_verified = false;
    for inst in &parsed_msg.instructions {
        // We expect the instructions to be of the parsed variant.
        if let UiInstruction::Parsed(parsed_inst_1) = inst {
            // We're looking for a transferChecked instruction from the spl-token program.
            if let UiParsedInstruction::Parsed(parsed_inst) = parsed_inst_1 {
                if parsed_inst.program == "spl-token"
                    && parsed_inst.parsed.get("type").and_then(|t| t.as_str())
                        == Some("transferChecked")
                {
                    if let Some(info) = parsed_inst.parsed.get("info").and_then(|v| v.as_object()) {
                        // Check that the mint is correct.
                        let mint = info.get("mint").and_then(|v| v.as_str()).unwrap_or("");
                        if mint != usdc_spl_mint {
                            continue;
                        }

                        // Check for the reference in the signers array.
                        if let Some(signers) = info.get("signers").and_then(|v| v.as_array()) {
                            let reference_found = signers
                                .iter()
                                .any(|s| s.as_str() == Some(transaction_reference));
                            if !reference_found {
                                continue;
                            }
                        } else {
                            continue;
                        }

                        // Check that the transferred amount is 10 USDC.
                        if let Some(token_amount) = info.get("tokenAmount") {
                            if token_amount.get("amount").and_then(|s| s.as_str())
                                == Some(amount.to_string().as_str())
                            {
                                transfer_verified = true;
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    if !transfer_verified {
        eprintln!("Transfer instruction verification failed.");
        return false;
    }

    println!("Transaction verification passed.");
    true
}

///
/// Gets a Vec<UiTransactionTokenBalance> and determins the balance on of mint tokens
/// on the given asscoiated token account address
///
fn get_mint_token_balance(
    token_balances: &OptionSerializer<Vec<UiTransactionTokenBalance>>,
    token_address: &str,
    wallet_pub_key: &str,
) -> u64 {
    let token_balances = match token_balances {
        OptionSerializer::Some(balances) => balances,
        _ => {
            eprintln!("No pre token balances found in transaction meta");
            return 0;
        }
    };

    let ata_token_balance = match token_balances.iter().find(|token_balance| {
        token_balance.mint == token_address.to_string()
            && token_balance.owner == OptionSerializer::Some(wallet_pub_key.to_string())
    }) {
        Some(token_balance) => token_balance.ui_token_amount.amount.as_str(),
        _ => {
            return 0;
        }
    };

    ata_token_balance.parse::<u64>().unwrap_or(0)
}

#[allow(dead_code)]
async fn get_estimated_fees(
    amount: u64,
    source_address: &str,
    destination_address: &str,
) -> Result<f64, MonexoMintError> {
    let rpc_url = "https://api.devnet.solana.com";
    let client = RpcClient::new(rpc_url.to_string());

    // Fetch the latest blockhash
    let latest_blockhash = client.get_latest_blockhash().await?;

    // Addresses
    let source_address = Pubkey::from_str(source_address)?; // Replace with actual sender address
    let destination_address = Pubkey::from_str(destination_address)?; // Replace with actual recipient address
    let usdc_mint = Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU")
        .expect("reference is not a valid public key"); // USDC Mint on Devnet

    // Get ATA (Associated Token Account) addresses
    let source_ata =
        spl_associated_token_account::get_associated_token_address(&source_address, &usdc_mint);
    let destination_ata = spl_associated_token_account::get_associated_token_address(
        &destination_address,
        &usdc_mint,
    );

    // Create `transfer_checked` instruction
    let transfer_ix = spl_token::instruction::transfer_checked(
        &spl_token::ID,   // Token program ID
        &source_ata,      // Sender ATA
        &usdc_mint,       // USDC Mint
        &destination_ata, // Recipient ATA
        &source_address,  // Owner of sender ATA
        &[],              // No additional signers
        amount,           // micro-usd Amount
        6,
    )?;

    // Create message
    let message = solana_sdk::message::Message::new_with_blockhash(
        &[transfer_ix],
        Some(&source_address),
        &latest_blockhash,
    );

    // Get estimated fee // 5000
    let fee_lamports = client.get_fee_for_message(&message).await?;

    // Convert fee to SOL // 0.000005
    let fee_sol = fee_lamports as f64 / 1_000_000_000.0; // 1 SOL = 1B lamports

    // Fetch SOL/USDC price 173.19
    let sol_usdc_price = fetch_sol_usdc_price().await?;

    // Convert fee to USDC // 0.00086595
    let fee_usdc = fee_sol * sol_usdc_price;

    info!("fee_usdc: {}", fee_usdc);

    Ok(fee_usdc)
}

#[allow(dead_code)]
async fn fetch_sol_usdc_price() -> Result<f64, MonexoMintError> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd";
    let response = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .expect("could not reach price exchange provider")
        .text()
        .await
        .expect("could not reach price exchange provider");

    let json: serde_json::Value =
        serde_json::from_str(&response).expect("could not reach price exchange provider");
    let price = json["solana"]["usd"]
        .as_f64()
        .ok_or("Failed to get SOL/USDC price")
        .expect("failed parse exchange response");

    Ok(price)
}
