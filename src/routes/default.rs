use axum::{extract::State, Json};
use monexo_core::primitives::{PostSwapRequest, PostSwapResponse};

use crate::{error::MonexoMintError, mint::Mint};

use tracing::instrument;

#[utoipa::path(
    post,
    path = "/v1/swap",
    request_body = PostSwapRequest,
    responses(
        (status = 200, description = "post swap", body = [PostSwapResponse])
    ),
)]
#[instrument(name = "post_swap", skip(mint), err)]
pub async fn post_swap(
    State(mint): State<Mint>,
    Json(swap_request): Json<PostSwapRequest>,
) -> Result<Json<PostSwapResponse>, MonexoMintError> {
    let response = mint
        .swap(&swap_request.inputs, &swap_request.outputs, &mint.keyset)
        .await?;

    Ok(Json(PostSwapResponse {
        signatures: response,
    }))
}
