use std::str::FromStr;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer};
use solana_sdk::commitment_config::CommitmentConfig;

use axum::{extract::{Path, State}, Json};
use monexo_core::primitives::{BtcOnchainMeltQuote, BtcOnchainMintQuote, MeltBtcOnchainState, MintBtcOnchainState, PostMeltBtcOnchainRequest, PostMeltBtcOnchainResponse, PostMeltQuoteBtcOnchainRequest, PostMeltQuoteBtcOnchainResponse, PostMintBtcOnchainRequest, PostMintBtcOnchainResponse, PostMintQuoteBtcOnchainRequest, PostMintQuoteBtcOnchainResponse};
use tracing::{info, instrument};
use uuid::Uuid;
use chrono::{Duration, Utc};

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

    // ======================================================================

    let client = RpcClient::new("https://api.devnet.solana.com".into());
    let config = GetConfirmedSignaturesForAddress2Config {
        limit: Some(20),
        commitment: Some(CommitmentConfig::confirmed()),
        ..GetConfirmedSignaturesForAddress2Config::default()
    };

    let reference = Pubkey::from_str(&quote.reference).expect("reference is not a valid public key");
    let signatures = client
        .get_signatures_for_address_with_config(&reference, config).await
        .expect("onchain backend not configured");

    // TODO: Confirm that the correct amount was paid
    let state = match signatures.len() {
        0 => MintBtcOnchainState::Unpaid,
        _ => MintBtcOnchainState::Paid,
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
