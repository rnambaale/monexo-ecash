use axum::{extract::State, Json};
use monexo_core::primitives::{PostCurrencyExchangeRequest, PostCurrencyExchangeResponse};

use crate::{error::MonexoMintError, mint::Mint};
use tracing::instrument;

#[utoipa::path(
    post,
    path = "/v1/exchange",
    request_body = PostCurrencyExchangeRequest,
    responses(
        (status = 200, description = "post exchange", body = [PostCurrencyExchangeResponse])
    ),
)]
#[instrument(name = "post_exchange", skip(mint), err)]
pub async fn post_exchange(
    State(mint): State<Mint>,
    Json(exchange_request): Json<PostCurrencyExchangeRequest>,
) -> Result<Json<PostCurrencyExchangeResponse>, MonexoMintError> {
    let response = mint
        .exchange(
            exchange_request.amount,
            &exchange_request.inputs,
            &exchange_request.outputs,
        )
        .await?;

    Ok(Json(PostCurrencyExchangeResponse {
        signatures: response,
    }))
}
