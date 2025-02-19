use std::str::FromStr;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;

use axum::{extract::{Path, State}, Json};
use monexo_core::primitives::{BtcOnchainMeltQuote, BtcOnchainMintQuote, MeltBtcOnchainState, MintBtcOnchainState, PostMeltBtcOnchainRequest, PostMeltBtcOnchainResponse, PostMeltQuoteBtcOnchainRequest, PostMeltQuoteBtcOnchainResponse, PostMintBtcOnchainRequest, PostMintBtcOnchainResponse, PostMintQuoteBtcOnchainRequest, PostMintQuoteBtcOnchainResponse};
use solana_transaction_status::UiTransactionTokenBalance;
use solana_transaction_status_client_types::option_serializer::OptionSerializer;
use tracing::{info, instrument};
use uuid::Uuid;
use chrono::{Duration, Utc};
use solana_transaction_status_client_types::{UiTransactionEncoding, UiMessage, UiInstruction, UiParsedInstruction};
// use solana_transaction_status::{
//     EncodedConfirmedTransactionWithStatusMeta, UiInstruction, UiMessage, UiParsedMessage,
// };

use crate::{database::Database, error::MonexoMintError, mint::Mint};

#[utoipa::path(
    post,
    path = "/v1/mint/quote/btconchain",
    request_body = PostMintQuoteBtcOnchainRequest,
    responses(
        (status = 200, description = "post mint quote", body = [PostMintQuoteBtcOnchainResponse])
    ),
)]
#[instrument(name = "post_mint_quote_btconchain", skip(mint), err)]
pub async fn post_mint_quote_btconchain(
    State(mint): State<Mint>,
    Json(request): Json<PostMintQuoteBtcOnchainRequest>,
) -> Result<Json<PostMintQuoteBtcOnchainResponse>, MonexoMintError> {
    let onchain_config = mint.config.btconchain_backend.unwrap_or_default();

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

    let quote = BtcOnchainMintQuote {
        quote_id,
        reference,
        amount: request.amount,
        expiry: quote_onchain_expiry(),
        state: MintBtcOnchainState::Unpaid,
    };

    let mut tx = mint.db.begin_tx().await?;
    mint.db.add_onchain_mint_quote(&mut tx, &quote).await?;
    tx.commit().await?;
    Ok(Json(quote.into()))
}

#[utoipa::path(
    get,
    path = "/v1/mint/quote/btconchain/{quote_id}",
    responses(
        (status = 200, description = "get mint quote by id", body = [PostMintQuoteBtcOnchainResponse])
    ),
    params(
        ("quote_id" = String, Path, description = "quote id"),
    )
)]
#[instrument(name = "get_mint_quote_btconchain", skip(mint), err)]
pub async fn get_mint_quote_btconchain(
    Path(quote_id): Path<String>,
    State(mint): State<Mint>,
) -> Result<Json<PostMintQuoteBtcOnchainResponse>, MonexoMintError> {
    info!("get_quote onchain: {}", quote_id);

    let mut tx = mint.db.begin_tx().await?;
    let quote = mint
        .db
        .get_onchain_mint_quote(&mut tx, &Uuid::from_str(quote_id.as_str())?)
        .await?;
    tx.commit().await?;

    // let min_confs = mint
    //     .config
    //     .btconchain_backend
    //     .unwrap_or_default()
    //     .min_confirmations;

    // let paid = mint
    //     .onchain
    //     .as_ref()
    //     .expect("onchain backend not configured")
    //     .is_paid(&quote.address, quote.amount, min_confs)
    //     .await?;

    // FIXME compute correct state
    // let state = match paid {
    //     true => MintBtcOnchainState::Paid,
    //     false => MintBtcOnchainState::Unpaid,
    // };

    // Extract and parse transaction logs
    let verified = is_paid(quote.amount, &quote.reference).await;

    let state = match verified {
        false => MintBtcOnchainState::Unpaid,
        true => MintBtcOnchainState::Paid,
    };

    Ok(Json(BtcOnchainMintQuote { state, ..quote }.into()))
}

#[utoipa::path(
    post,
    path = "/v1/mint/btconchain",
    request_body = PostMintBtcOnchainRequest,
    responses(
        (status = 200, description = "post mint", body = [PostMintBtcOnchainResponse])
    ),
)]
#[instrument(name = "post_mint_btconchain", skip(mint), err)]
pub async fn post_mint_btconchain(
    State(mint): State<Mint>,
    Json(request): Json<PostMintBtcOnchainRequest>,
) -> Result<Json<PostMintBtcOnchainResponse>, MonexoMintError> {
    // TODO Figure out the quote has been paid, only then do you mint the tokens

    let mut tx = mint.db.begin_tx().await?;
    let signatures = mint
        .mint_tokens(
            &mut tx,
            request.quote.clone(),
            &request.outputs,
            &mint.keyset,
            false,
        )
        .await?;

    let old_quote = &mint
        .db
        .get_onchain_mint_quote(&mut tx, &Uuid::from_str(request.quote.as_str())?)
        .await?;

    mint.db
        .update_onchain_mint_quote(
            &mut tx,
            &BtcOnchainMintQuote {
                state: MintBtcOnchainState::Issued,
                ..old_quote.clone()
            },
        )
        .await?;
    tx.commit().await?;
    Ok(Json(PostMintBtcOnchainResponse { signatures }))
}

#[utoipa::path(
    post,
    path = "/v1/melt/quote/btconchain",
    request_body = PostMeltQuoteBtcOnchainRequest,
    responses(
        (status = 200, description = "post melt quote", body = [Vec<PostMeltQuoteBtcOnchainResponse>])
    ),
)]
#[instrument(name = "post_melt_quote_btconchain", skip(mint), err)]
pub async fn post_melt_quote_btconchain(
    State(mint): State<Mint>,
    Json(melt_request): Json<PostMeltQuoteBtcOnchainRequest>,
) -> Result<Json<Vec<PostMeltQuoteBtcOnchainResponse>>, MonexoMintError> {
    let PostMeltQuoteBtcOnchainRequest {
        address,
        amount,
    } = melt_request;

    let onchain_config = mint.config.btconchain_backend.unwrap_or_default();

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

    // TODO Figure out how to get fees on solana
    // let fee_response = mint
    //     .onchain
    //     .as_ref()
    //     .expect("onchain backend not configured")
    //     .estimate_fee(&address, amount)
    //     .await?;

    // info!("post_melt_quote_onchain fee_reserve: {:#?}", &fee_response);

    let quote = BtcOnchainMeltQuote {
        quote_id: Uuid::new_v4(),
        address,
        amount,
        fee_total: 0, // fee_response.fee_in_sat,
        fee_sat_per_vbyte: 0, //fee_response.sat_per_vbyte,
        expiry: quote_onchain_expiry(),
        state: MeltBtcOnchainState::Unpaid,
        // description: Some(format!("{} sat per vbyte", fee_response.sat_per_vbyte)),
        description: None,
    };

    let mut tx = mint.db.begin_tx().await?;
    mint.db.add_onchain_melt_quote(&mut tx, &quote).await?;
    tx.commit().await?;

    Ok(Json(vec![quote.into()]))
}

#[utoipa::path(
    get,
    path = "/v1/melt/quote/btconchain/{quote_id}",
    responses(
        (status = 200, description = "post mint quote", body = [PostMeltQuoteBtcOnchainResponse])
    ),
    params(
        ("quote_id" = String, Path, description = "quote id"),
    )
)]
#[instrument(name = "get_melt_quote_btconchain", skip(mint), err)]
pub async fn get_melt_quote_btconchain(
    Path(quote_id): Path<String>,
    State(mint): State<Mint>,
) -> Result<Json<PostMeltQuoteBtcOnchainResponse>, MonexoMintError> {
    info!("get_melt_quote onchain: {}", quote_id);
    let mut tx = mint.db.begin_tx().await?;
    let quote = mint
        .db
        .get_onchain_melt_quote(&mut tx, &Uuid::from_str(quote_id.as_str())?)
        .await?;

    let paid = is_onchain_paid(&mint, &quote).await?;

    let state = match paid {
        true => MeltBtcOnchainState::Paid,
        false => MeltBtcOnchainState::Unpaid,
    };

    if paid {
        mint.db
            .update_onchain_melt_quote(
                &mut tx,
                &BtcOnchainMeltQuote {
                    state: state.clone(),
                    ..quote.clone()
                },
            )
            .await?;
    }

    Ok(Json(BtcOnchainMeltQuote { state, ..quote }.into()))
}

#[utoipa::path(
    post,
    path = "/v1/melt/btconchain",
    request_body = PostMeltBtcOnchainRequest,
    responses(
        (status = 200, description = "post melt", body = [PostMeltBtcOnchainResponse])
    ),
)]
#[instrument(name = "post_melt_btconchain", skip(mint), err)]
pub async fn post_melt_btconchain(
    State(mint): State<Mint>,
    Json(melt_request): Json<PostMeltBtcOnchainRequest>,
) -> Result<Json<PostMeltBtcOnchainResponse>, MonexoMintError> {
    let mut tx = mint.db.begin_tx().await?;
    let quote = mint
        .db
        .get_onchain_melt_quote(&mut tx, &Uuid::from_str(melt_request.quote.as_str())?)
        .await?;

    let txid = mint.melt_onchain(&quote, &melt_request.inputs).await?;
    let paid = is_onchain_paid(&mint, &quote).await?;

    // FIXME  compute correct state
    let state = match paid {
        true => MeltBtcOnchainState::Paid,
        false => MeltBtcOnchainState::Unpaid,
    };

    mint.db
        .update_onchain_melt_quote(
            &mut tx,
            &BtcOnchainMeltQuote {
                state: state.clone(),
                ..quote
            },
        )
        .await?;
    tx.commit().await?;

    Ok(Json(PostMeltBtcOnchainResponse {
        state,
        txid: Some(txid),
    }))
}

#[allow(dead_code)]
fn quote_onchain_expiry() -> u64 {
    let now = Utc::now() + Duration::try_minutes(5).expect("invalid duration");
    now.timestamp() as u64
}

#[allow(dead_code)]
async fn is_onchain_paid(
    _mint: &Mint,
    quote: &BtcOnchainMeltQuote,
) -> Result<bool, MonexoMintError> {
    // let min_confs = mint
    //     .config
    //     .btconchain_backend
    //     .clone()
    //     .unwrap_or_default()
    //     .min_confirmations;

    // mint.onchain
    //     .as_ref()
    //     .expect("onchain backend not configured")
    //     .is_paid(&quote.address, quote.amount, min_confs)
    //     .await

    let client = RpcClient::new("https://api.devnet.solana.com".into());
    let config = GetConfirmedSignaturesForAddress2Config {
        limit: Some(20),
        commitment: Some(CommitmentConfig::confirmed()),
        ..GetConfirmedSignaturesForAddress2Config::default()
    };

    let reference = Pubkey::from_str(&quote.address).expect("reference is not a valid public key");
    let signatures = client
        .get_signatures_for_address_with_config(&reference, config).await
        .expect("onchain backend not configured");

    Ok(signatures.len() > 0)
}

async fn is_paid(amount: u64, expected_reference: &str) -> bool {
    // Expected values:
    // let expected_mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
    // let expected_reference = "5t6gQ7Mnr3mmsFYquFGwgEKokq9wrrUgCpwWab93LmLL";
    // let expected_owner = "HVasUUKPrmrAuBpDFiu8BxQKzrMYY5DvyuNXamvaG2nM";
    // let expected_amount_str = "10"; // as reported in uiAmountString

    let client = RpcClient::new("https://api.devnet.solana.com".into());
    let config = GetConfirmedSignaturesForAddress2Config {
        limit: Some(20),
        commitment: Some(CommitmentConfig::confirmed()),
        ..GetConfirmedSignaturesForAddress2Config::default()
    };

    let reference = Pubkey::from_str(&expected_reference).expect("reference is not a valid public key");
    let signatures = client
        .get_signatures_for_address_with_config(&reference, config).await
        .expect("onchain backend not configured");

    let first_signature = match signatures.first() {
        Some(sig) =>
            Signature::from_str(&sig.signature).expect("could not parse transaction signature"),
        None => {
            eprintln!("No transaction signatures found");
            return false;
        }
    };

    let tx
        = match client.get_transaction(
            &first_signature,
            UiTransactionEncoding::JsonParsed
        ).await
    {
        Ok(tx) => tx,
        _ => {
            eprintln!("could not fetch transaction details");
            return false;
        }
    };

    let expected_mint = "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU";
    let expected_owner  = "HVasUUKPrmrAuBpDFiu8BxQKzrMYY5DvyuNXamvaG2nM";

    let amount_string_representation = amount.to_string();
    let expected_amount_str = amount_string_representation.as_str();

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
        &expected_mint,
        expected_owner
    );

    let post_ata_token_balance = get_mint_token_balance(
        &meta.post_token_balances,
        expected_mint,
        expected_owner
    );

    let mint_balance_change = post_ata_token_balance - pre_ata_token_balance;

    if mint_balance_change < amount as f64 {
        eprintln!("Post token balance verification failed.");
        return false;
    }

    // // Check that one of the post-token balances shows the expected mint,
    // // destination (owner) and amount.
    // let balance_ok = post_token_balances.iter().any(|balance| {
    //     // info!("balance.mint == expected_mint.to_string(): {}", balance.mint == expected_mint.to_string());
    //     // info!("balance.owner == OptionSerializer::Some(expected_owner.to_string()): {}", balance.owner == OptionSerializer::Some(expected_owner.to_string()));
    //     // info!("balance.ui_token_amount.ui_amount_string == expected_amount_str: {}", balance.ui_token_amount.ui_amount_string == expected_amount_str);
    //     return balance.mint == expected_mint.to_string()
    //         && balance.owner == OptionSerializer::Some(expected_owner.to_string())
    //         && balance.ui_token_amount.ui_amount_string == expected_amount_str
    // });
    // if !balance_ok {
    //     eprintln!("Post token balance verification failed.");
    //     return false;
    // }

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
                    && parsed_inst.parsed.get("type").and_then(|t| t.as_str()) == Some("transferChecked")
                {
                    if let Some(info) = parsed_inst.parsed.get("info").and_then(|v| v.as_object()) {
                        // Check that the mint is correct.
                        let mint = info.get("mint").and_then(|v| v.as_str()).unwrap_or("");
                        if mint != expected_mint {
                            continue;
                        }

                        // Check for the reference in the signers array.
                        if let Some(signers) = info.get("signers").and_then(|v| v.as_array()) {
                            let reference_found = signers.iter().any(|s| s.as_str() == Some(expected_reference));
                            if !reference_found {
                                continue;
                            }
                        } else {
                            continue;
                        }

                        // Check that the transferred amount is 10 USDC.
                        if let Some(token_amount) = info.get("tokenAmount") {
                            if token_amount.get("uiAmountString").and_then(|s| s.as_str()) == Some(expected_amount_str) {
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
    mint_ata_address: &str) -> f64 {
    let token_balances = match token_balances {
        OptionSerializer::Some(balances) => balances,
        _ => {
            eprintln!("No pre token balances found in transaction meta");
            return 0.0;
        }
    };

    let ata_token_balance = match token_balances.iter().find(|token_balance|
        token_balance.mint == token_address.to_string()
        && token_balance.owner == OptionSerializer::Some(mint_ata_address.to_string())
    ) {
        Some(token_balance) => token_balance.ui_token_amount.ui_amount,
        _ => {
            return 0.0;
        }
    };

    if ata_token_balance.is_none() {
        return 0.0;
    }

    ata_token_balance.unwrap()
}
