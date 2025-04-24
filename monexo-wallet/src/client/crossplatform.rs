use async_trait::async_trait;
use monexo_core::{
    blind::BlindedMessage,
    keyset::Keysets,
    primitives::{
        KeysResponse, MintInfoResponse, PostMeltOnchainRequest, PostMeltOnchainResponse,
        PostMeltQuoteOnchainRequest, PostMeltQuoteOnchainResponse, PostMintOnchainRequest,
        PostMintOnchainResponse, PostMintQuoteOnchainRequest, PostMintQuoteOnchainResponse,
        PostSwapRequest, PostSwapResponse,
    },
    proof::Proofs,
};

use crate::{error::MonexoWalletError, http::CrossPlatformHttpClient};

use super::CashuClient;
use url::Url;

#[async_trait(?Send)]
impl CashuClient for CrossPlatformHttpClient {
    async fn get_keys(&self, mint_url: &Url) -> Result<KeysResponse, MonexoWalletError> {
        self.do_get(&mint_url.join("v1/keys")?).await
    }

    async fn get_keys_by_id(
        &self,
        mint_url: &Url,
        keyset_id: String,
    ) -> Result<KeysResponse, MonexoWalletError> {
        self.do_get(&mint_url.join(&format!("v1/keys/{}", keyset_id))?)
            .await
    }

    async fn get_keysets(&self, mint_url: &Url) -> Result<Keysets, MonexoWalletError> {
        self.do_get(&mint_url.join("v1/keysets")?).await
    }

    async fn post_swap(
        &self,
        mint_url: &Url,
        inputs: Proofs,
        outputs: Vec<BlindedMessage>,
    ) -> Result<PostSwapResponse, MonexoWalletError> {
        let body = PostSwapRequest { inputs, outputs };

        self.do_post(&mint_url.join("v1/swap")?, &body).await
    }

    async fn post_mint_onchain(
        &self,
        mint_url: &Url,
        quote: String,
        blinded_messages: Vec<BlindedMessage>,
    ) -> Result<PostMintOnchainResponse, MonexoWalletError> {
        let body = PostMintOnchainRequest {
            quote,
            outputs: blinded_messages,
        };
        self.do_post(&mint_url.join("v1/mint/btconchain")?, &body)
            .await
    }

    async fn post_mint_quote_onchain(
        &self,
        mint_url: &Url,
        amount: u64,
    ) -> Result<PostMintQuoteOnchainResponse, MonexoWalletError> {
        let body = PostMintQuoteOnchainRequest { amount };
        self.do_post(&mint_url.join("v1/mint/quote/btconchain")?, &body)
            .await
    }

    async fn get_mint_quote_onchain(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMintQuoteOnchainResponse, MonexoWalletError> {
        self.do_get(&mint_url.join(&format!("v1/mint/quote/btconchain/{}", quote))?)
            .await
    }

    async fn post_melt_onchain(
        &self,
        mint_url: &Url,
        inputs: Proofs,
        quote: String,
    ) -> Result<PostMeltOnchainResponse, MonexoWalletError> {
        let body = PostMeltOnchainRequest { quote, inputs };
        self.do_post(&mint_url.join("v1/melt/btconchain")?, &body)
            .await
    }

    async fn post_melt_quote_onchain(
        &self,
        mint_url: &Url,
        address: String,
        amount: u64,
    ) -> Result<Vec<PostMeltQuoteOnchainResponse>, MonexoWalletError> {
        let body = PostMeltQuoteOnchainRequest { address, amount };
        self.do_post(&mint_url.join("v1/melt/quote/btconchain")?, &body)
            .await
    }

    async fn get_melt_quote_onchain(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMeltQuoteOnchainResponse, MonexoWalletError> {
        self.do_get(&mint_url.join(&format!("/v1/melt/quote/btconchain/{quote}"))?)
            .await
    }

    async fn get_info(&self, mint_url: &Url) -> Result<MintInfoResponse, MonexoWalletError> {
        self.do_get(&mint_url.join("v1/info")?).await
    }

    async fn is_v1_supported(&self, mint_url: &Url) -> Result<bool, MonexoWalletError> {
        self.get_status(&mint_url.join("v1/info")?)
            .await
            .map(|s| s == 200)
    }
}
