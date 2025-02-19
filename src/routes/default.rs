use axum::{extract::State, Json};
use monexo_core::primitives::{MintInfoResponse, PostSwapRequest, PostSwapResponse};

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

#[utoipa::path(
    get,
    path = "/v1/info",
    responses(
        (status = 200, description = "get mint info", body = [MintInfoResponse])
    )
)]
#[instrument(name = "get_info", skip(_mint), err)]
pub async fn get_info(State(_mint): State<Mint>) -> Result<Json<MintInfoResponse>, MonexoMintError> {
    // let mint_info = mint.config.info.clone();

    let mint_info = MintInfoResponse {
        // name: mint.config.info.name,
        name: None,
        version: None,
        usdc_address: String::from("HVasUUKPrmrAuBpDFiu8BxQKzrMYY5DvyuNXamvaG2nM"),
        usdc_token_mint: String::from("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU&reference=5t6gQ7Mnr3mmsFYquFGwgEKokq9wrrUgCpwWab93LmLL")
    };
    Ok(Json(mint_info))
}
