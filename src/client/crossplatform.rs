use async_trait::async_trait;
use monexo_core::{blind::BlindedMessage, keyset::Keysets, primitives::{KeysResponse, PostMeltBtcOnchainRequest, PostMeltBtcOnchainResponse, PostMeltQuoteBtcOnchainRequest, PostMeltQuoteBtcOnchainResponse, PostMintBtcOnchainRequest, PostMintBtcOnchainResponse, PostMintQuoteBtcOnchainRequest, PostMintQuoteBtcOnchainResponse, PostSwapRequest, PostSwapResponse}, proof::Proofs};

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
    ) -> Result<PostMintBtcOnchainResponse, MonexoWalletError> {
        let body = PostMintBtcOnchainRequest {
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
    ) -> Result<PostMintQuoteBtcOnchainResponse, MonexoWalletError> {
        let body = PostMintQuoteBtcOnchainRequest { amount };
        self.do_post(&mint_url.join("v1/mint/quote/btconchain")?, &body)
            .await
    }

    async fn get_mint_quote_onchain(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMintQuoteBtcOnchainResponse, MonexoWalletError> {
        self.do_get(&mint_url.join(&format!("v1/mint/quote/btconchain/{}", quote))?)
            .await
    }

    async fn post_melt_onchain(
        &self,
        mint_url: &Url,
        inputs: Proofs,
        quote: String,
    ) -> Result<PostMeltBtcOnchainResponse, MonexoWalletError> {
        let body = PostMeltBtcOnchainRequest { quote, inputs };
        self.do_post(&mint_url.join("v1/melt/btconchain")?, &body)
            .await
    }

    async fn post_melt_quote_onchain(
        &self,
        mint_url: &Url,
        address: String,
        amount: u64,
        // unit: CurrencyUnit,
    ) -> Result<Vec<PostMeltQuoteBtcOnchainResponse>, MonexoWalletError> {
        let body = PostMeltQuoteBtcOnchainRequest {
            address,
            amount,
        };
        self.do_post(&mint_url.join("v1/melt/quote/btconchain")?, &body)
            .await
    }

    async fn get_melt_quote_onchain(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<PostMeltQuoteBtcOnchainResponse, MonexoWalletError> {
        self.do_get(&mint_url.join(&format!("/v1/melt/quote/btconchain/{quote}"))?)
            .await
    }
}
