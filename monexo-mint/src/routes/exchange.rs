use axum::{extract::State, Json};
use monexo_core::primitives::{PostCurrencyExchangeRequest, PostCurrencyExchangeResponse};

use crate::{database::Database, error::MonexoMintError, mint::Mint};
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

    let mut tx = mint.db.begin_tx().await?;
    mint.db
        .add_blind_signatures(&mut tx, &exchange_request.outputs, &response, None)
        .await?;
    tx.commit().await?;

    Ok(Json(PostCurrencyExchangeResponse {
        signatures: response,
    }))
}
