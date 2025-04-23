pub mod crossplatform;

use async_trait::async_trait;

#[cfg(test)]
use mockall::automock;
use monexo_core::{
    blind::BlindedMessage,
    keyset::Keysets,
    primitives::{
        KeysResponse, MintInfoResponse, PostMeltOnchainResponse, PostMeltQuoteOnchainResponse,
        PostMintOnchainResponse, PostMintQuoteOnchainResponse, PostSwapResponse,
    },
    proof::Proofs,
};
use url::Url;

use crate::error::MonexoWalletError;

#[cfg_attr(test, automock)]
#[async_trait(?Send)]
pub trait CashuClient {
    async fn get_keys(&self, mint_url: &Url) -> Result<KeysResponse, MonexoWalletError>;

    async fn get_keys_by_id(
        &self,
        mint_url: &Url,
        keyset_id: String,
    ) -> Result<KeysResponse, MonexoWalletError>;

    async fn get_keysets(&self, mint_url: &Url) -> Result<Keysets, MonexoWalletError>;

    async fn post_swap(
        &self,
        mint_url: &Url,
        proofs: Proofs,
        output: Vec<BlindedMessage>,
    ) -> Result<PostSwapResponse, MonexoWalletError>;

    async fn post_mint_onchain(
        &self,
        mint_url: &Url,
        quote: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintOnchainResponse, MonexoWalletError>;

    async fn post_mint_quote_onchain(
        &self,
        mint_url: &Url,
        amount: u64,
    ) -> Result<PostMintQuoteOnchainResponse, MonexoWalletError>;

    async fn get_mint_quote_onchain(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMintQuoteOnchainResponse, MonexoWalletError>;

    async fn post_melt_onchain(
        &self,
        mint_url: &Url,
        proofs: Proofs,
        quote: String,
    ) -> Result<PostMeltOnchainResponse, MonexoWalletError>;

    async fn post_melt_quote_onchain(
        &self,
        mint_url: &Url,
        address: String,
        amount: u64,
    ) -> Result<Vec<PostMeltQuoteOnchainResponse>, MonexoWalletError>;

    async fn get_melt_quote_onchain(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMeltQuoteOnchainResponse, MonexoWalletError>;

    async fn get_info(&self, mint_url: &Url) -> Result<MintInfoResponse, MonexoWalletError>;

    async fn is_v1_supported(&self, mint_url: &Url) -> Result<bool, MonexoWalletError>;
}
