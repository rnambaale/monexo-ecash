use std::str::FromStr;

use axum::{extract::{Path, State}, Json};
use monexo_core::primitives::{BtcOnchainMintQuote, MintBtcOnchainState, PostMintBtcOnchainRequest, PostMintBtcOnchainResponse, PostMintQuoteBtcOnchainRequest, PostMintQuoteBtcOnchainResponse};
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

    // This will act as the transaction memo
    let quote_id = Uuid::new_v4();

    let quote = BtcOnchainMintQuote {
        quote_id,
        // address, // shared address for ann usdc deposits
        unit: request.unit,
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

    Ok(Json(BtcOnchainMintQuote { ..quote }.into()))
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

#[allow(dead_code)]
fn quote_onchain_expiry() -> u64 {
    // FIXME add config option for expiry
    let now = Utc::now() + Duration::try_minutes(5).expect("invalid duration");
    now.timestamp() as u64
}
