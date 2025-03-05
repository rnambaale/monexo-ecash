use std::str::FromStr;

use axum::{extract::{Path, State}, Json};
use monexo_core::{keyset::Keysets, primitives::{CurrencyUnit, KeyResponse, KeysResponse, MintInfoResponse, PostSwapRequest, PostSwapResponse}};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::{EncodableKey, Signer}};

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
#[instrument(name = "get_info", skip(mint), err)]
pub async fn get_info(State(mint): State<Mint>) -> Result<Json<MintInfoResponse>, MonexoMintError> {
    // let mint_info = mint.config.info.clone();

    let usdc_mint_address = Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").unwrap();
    let monexo_wallet_keypair = Keypair::read_from_file(
        mint.config.derivation_path.unwrap()
    ).expect("Failed to load keypair");
    let monexo_usdc_ata = spl_associated_token_account::get_associated_token_address(
        &monexo_wallet_keypair.try_pubkey().expect("Failed to load mint pubkey"),
        &usdc_mint_address,
    );

    let mint_info = MintInfoResponse {
        // name: mint.config.info.name,
        name: None,
        version: None,
        usdc_address: monexo_usdc_ata.to_string(),
        usdc_token_mint: usdc_mint_address.to_string()
    };

    Ok(Json(mint_info))
}

#[utoipa::path(
    get,
    path = "/v1/keys",
    responses(
        (status = 200, description = "get keys", body = [KeysResponse])
    )
)]
#[instrument(skip(mint), err)]
pub async fn get_keys(State(mint): State<Mint>) -> Result<Json<KeysResponse>, MonexoMintError> {
    Ok(Json(KeysResponse {
        keysets: vec![KeyResponse {
            id: mint.keyset.keyset_id.clone(),
            unit: CurrencyUnit::Usd,
            keys: mint.keyset.public_keys,
        }],
    }))
}

#[utoipa::path(
    get,
    path = "/v1/keysets",
    responses(
        (status = 200, description = "get keysets", body = [Keysets])
    ),
)]
#[instrument(skip(mint), err)]
pub async fn get_keysets(State(mint): State<Mint>) -> Result<Json<Keysets>, MonexoMintError> {
    Ok(Json(Keysets::new(
        mint.keyset.keyset_id,
        CurrencyUnit::Usd,
        true,
    )))
}

#[instrument(skip(mint), err)]
pub async fn get_keys_by_id(
    Path(id): Path<String>,
    State(mint): State<Mint>,
) -> Result<Json<KeysResponse>, MonexoMintError> {
    if id != mint.keyset.keyset_id {
        return Err(MonexoMintError::KeysetNotFound(id));
    }

    Ok(Json(KeysResponse {
        keysets: vec![KeyResponse {
            id: mint.keyset.keyset_id.clone(),
            unit: CurrencyUnit::Usd,
            keys: mint.keyset.public_keys,
        }],
    }))
}

// Usage
// use solana_client::nonblocking::rpc_client::RpcClient;
// let rpc_url = "https://api.devnet.solana.com";
// let client = RpcClient::new(rpc_url.to_string());
// let source_ata = create_ata(
//     &client,
//     &monexo_wallet_keypair,
//     &monexo_wallet_keypair.try_pubkey().expect("Failed to load mint pubkey"),
//     &usdc_mint_address
// ).await.expect("failed to create ata");

// let sender_balance = client.get_token_account_balance(&source_ata).await?;
// info!("Sender's USDC Balance: {:?}", sender_balance);
// info!(
//     "source_ata: {:?}", source_ata
// );

// async fn create_ata(
//     rpc_client: &RpcClient,
//     payer: &Keypair, // Your server wallet
//     owner: &Pubkey,
//     mint: &Pubkey
// ) -> Result<Pubkey, Box<dyn std::error::Error>> {

//     let ata = spl_associated_token_account::get_associated_token_address(owner, mint);

//     let ix = spl_associated_token_account::instruction::create_associated_token_account(
//         &payer.pubkey(),
//         owner,
//         mint,
//         &spl_token::id()
//     );
//     let recent_blockhash = rpc_client.get_latest_blockhash().await?;

//     let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
//         &[ix],
//         Some(&payer.pubkey()),
//         &[payer],
//         recent_blockhash,
//     );

//     let sig = rpc_client.send_and_confirm_transaction(&tx).await?;
//     println!("ATA created: {:?}", sig);

//     Ok(ata)
// }
