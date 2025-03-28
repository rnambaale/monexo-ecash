use axum::{extract::State, Json};
use monexo_core::primitives::{PostCurrencyExchangeQuoteRequest, PostCurrentExchangeResponse};

use crate::{error::MonexoMintError, mint::Mint};
use tracing::instrument;

#[utoipa::path(
    post,
    path = "/v1/exchange",
    request_body = PostCurrencyExchangeQuoteRequest,
    responses(
        (status = 200, description = "post exchange", body = [PostCurrentExchangeResponse])
    ),
)]
#[instrument(name = "post_exchange", skip(mint), err)]
pub async fn post_exchange(
    State(mint): State<Mint>,
    Json(exchange_request): Json<PostCurrencyExchangeQuoteRequest>,
) -> Result<Json<PostCurrentExchangeResponse>, MonexoMintError> {
    let response = mint
        .exchange(
            exchange_request.amount,
            &exchange_request.inputs,
            &exchange_request.outputs,
        )
        .await?;

    Ok(Json(PostCurrentExchangeResponse {
        signatures: response,
    }))
}
