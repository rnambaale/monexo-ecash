use std::collections::HashMap;

use monexo_core::{amount::Amount, blind::{BlindedMessage, BlindedSignature, BlindingFactor}, dhke::Dhke, keyset::KeysetId, primitives::{MintBtcOnchainState, PostMintQuoteBtcOnchainResponse}, proof::{Proof, Proofs}, token::TokenV3};
use secp256k1::PublicKey;
use url::Url;

use crate::{client::CashuClient, error::MonexoWalletError, http::CrossPlatformHttpClient, localstore::{LocalStore, WalletKeyset}, secret::DeterministicSecret};

#[derive(Clone)]
pub struct Wallet<L, C>
where
    L: LocalStore,
    C: CashuClient,
{
    client: C,
    dhke: Dhke,
    localstore: L,
    secret: DeterministicSecret,
}

pub struct WalletBuilder<L, C: CashuClient = CrossPlatformHttpClient>
where
    L: LocalStore,
    C: CashuClient + Default,
{
    client: Option<C>,
    localstore: Option<L>,
}

impl<L, C> WalletBuilder<L, C>
where
    L: LocalStore,
    C: CashuClient + Default,
{
    fn new() -> Self {
        Self {
            client: Some(C::default()),
            localstore: None,
        }
    }

    pub fn with_client(mut self, client: C) -> Self {
        self.client = Some(client);
        self
    }

    pub fn with_localstore(mut self, localstore: L) -> Self {
        self.localstore = Some(localstore);
        self
    }

    pub async fn build(self) -> Result<Wallet<L, C>, MonexoWalletError> {
        let client = self.client.unwrap_or_default();
        let localstore = self.localstore.expect("localstore is required");

        let mut tx = localstore.begin_tx().await?;
        let seed_words = localstore.get_seed(&mut tx).await?;
        let seed = match seed_words {
            Some(seed) => seed,
            None => {
                let seed = DeterministicSecret::generate_random_seed_words()?;
                localstore.add_seed(&mut tx, &seed).await?;
                seed
            }
        };

        tx.commit().await?;

        Ok(Wallet::new(
            client as C,
            localstore,
            DeterministicSecret::from_seed_words(&seed)?,
        ))
    }
}

impl<L, C> Default for WalletBuilder<L, C>
where
    C: CashuClient + Default,
    L: LocalStore,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<L, C> Wallet<L, C>
where
    C: CashuClient + Default,
    L: LocalStore,
{
    fn new(client: C, localstore: L, secret: DeterministicSecret) -> Self {
        Self {
            client,
            dhke: Dhke::new(),
            localstore,
            secret,
        }
    }

    pub fn builder() -> WalletBuilder<L, C> {
        WalletBuilder::default()
    }

    pub async fn create_quote_onchain(
        &self,
        mint_url: &Url,
        amount: u64,
    ) -> Result<PostMintQuoteBtcOnchainResponse, MonexoWalletError> {
        self.client
            .post_mint_quote_onchain(mint_url, amount)
            .await
    }

    pub async fn is_quote_paid(
        &self,
        mint_url: &Url,
        quote: String,
    ) -> Result<bool, MonexoWalletError> {
        Ok(matches!(
            self.client
                .get_mint_quote_onchain(mint_url, quote)
                .await?
                .state,
            MintBtcOnchainState::Paid | MintBtcOnchainState::Issued
        ))
    }

    // pub async fn is_onchain_paid(
    //     &self,
    //     mint_url: &Url,
    //     quote: String,
    // ) -> Result<bool, MokshaWalletError> {
    //     // FIXME add method get_onchain_state
    //     Ok(self
    //         .client
    //         .get_melt_quote_onchain(mint_url, quote)
    //         .await?
    //         .state
    //         == MeltBtcOnchainState::Paid)
    // }

    pub async fn get_wallet_keysets(&self) -> Result<Vec<WalletKeyset>, MonexoWalletError> {
        let mut tx = self.localstore.begin_tx().await?;
        let keysets = self.localstore.get_keysets(&mut tx).await?;
        tx.commit().await?;
        Ok(keysets)
    }

    // pub async fn get_mint_urls(&self) -> Result<Vec<Url>, MonexoWalletError> {
    //     let keysets = self.get_wallet_keysets().await?;
    //     let mints: HashSet<Url> = keysets.into_iter().map(|k| k.mint_url).collect();
    //     Ok(mints.into_iter().collect())
    // }

    pub async fn get_balance(&self) -> Result<u64, MonexoWalletError> {
        let mut tx = self.localstore.begin_tx().await?;
        let total_amount = self.localstore.get_proofs(&mut tx).await?.total_amount();
        tx.commit().await?;
        Ok(total_amount)
    }

    // pub async fn send_tokens(
    //     &self,
    //     wallet_keyset: &WalletKeyset,
    //     amount: u64,
    // ) -> Result<TokenV3, MonexoWalletError> {
    //     let balance = self.get_balance().await?;
    //     if amount > balance {
    //         return Err(MonexoWalletError::NotEnoughTokens);
    //     }

    //     let mut tx = self.localstore.begin_tx().await?;
    //     let all_proofs = self
    //         .localstore
    //         .get_proofs(&mut tx)
    //         .await?
    //         .proofs_by_keyset(&wallet_keyset.keyset_id);
    //     tx.commit().await?;

    //     let selected_proofs = all_proofs.proofs_for_amount(amount)?;
    //     let selected_tokens = (wallet_keyset.mint_url.to_owned(), selected_proofs.clone()).into();

    //     let (remaining_tokens, result) = self
    //         .swap_tokens(wallet_keyset, &selected_tokens, amount.into())
    //         .await?;

    //     let mut tx = self.localstore.begin_tx().await?;
    //     self.localstore
    //         .delete_proofs(&mut tx, &selected_proofs)
    //         .await?;

    //     self.localstore
    //         .add_proofs(&mut tx, &remaining_tokens.proofs())
    //         .await?;
    //     tx.commit().await?;
    //     Ok(result)
    // }

    // pub async fn receive_tokens(
    //     &self,
    //     wallet_keyset: &WalletKeyset,
    //     tokens: &TokenV3,
    // ) -> Result<(), MonexoWalletError> {
    //     let total_amount = tokens.total_amount();
    //     let (_, redeemed_tokens) = self
    //         .swap_tokens(wallet_keyset, tokens, total_amount.into())
    //         .await?;
    //     let mut tx = self.localstore.begin_tx().await?;
    //     self.localstore
    //         .add_proofs(&mut tx, &redeemed_tokens.proofs())
    //         .await?;
    //     tx.commit().await?;
    //     Ok(())
    // }

    // pub async fn get_mint_quote(
    //     &self,
    //     mint_url: &Url,
    //     amount: Amount,
    //     currency: CurrencyUnit,
    // ) -> Result<PostMintQuoteBolt11Response, MokshaWalletError> {
    //     self.client
    //         .post_mint_quote_bolt11(mint_url, amount.0, currency)
    //         .await
    // }

    // pub async fn get_melt_quote_btconchain(
    //     &self,
    //     mint_url: &Url,
    //     address: String,
    //     amount: u64,
    // ) -> Result<Vec<PostMeltQuoteBtcOnchainResponse>, MokshaWalletError> {
    //     self.client
    //         .post_melt_quote_onchain(mint_url, address, amount, CurrencyUnit::Sat)
    //         .await
    // }

    // pub async fn pay_onchain(
    //     &self,
    //     wallet_keyset: &WalletKeyset,
    //     melt_quote: &PostMeltQuoteBtcOnchainResponse,
    // ) -> Result<PostMeltBtcOnchainResponse, MokshaWalletError> {
    //     let mut tx = self.localstore.begin_tx().await?;
    //     let all_proofs = self.localstore.get_proofs(&mut tx).await?;
    //     tx.commit().await?;

    //     let ln_amount = melt_quote.amount + melt_quote.fee;

    //     if ln_amount > all_proofs.total_amount() {
    //         return Err(MokshaWalletError::NotEnoughTokens);
    //     }
    //     let selected_proofs = all_proofs.proofs_for_amount(ln_amount)?;

    //     let mut tx = self.localstore.begin_tx().await?;
    //     let total_proofs = {
    //         let selected_tokens =
    //             (wallet_keyset.mint_url.to_owned(), selected_proofs.clone()).into();
    //         let swap_result = self
    //             .swap_tokens(wallet_keyset, &selected_tokens, ln_amount.into())
    //             .await?;
    //         self.localstore
    //             .delete_proofs(&mut tx, &selected_proofs)
    //             .await?;
    //         self.localstore
    //             .add_proofs(&mut tx, &swap_result.0.proofs())
    //             .await?;

    //         swap_result.1.proofs()
    //     };

    //     let melt_response = self
    //         .client
    //         .post_melt_onchain(
    //             &wallet_keyset.mint_url,
    //             total_proofs.clone(),
    //             melt_quote.quote.clone(),
    //         )
    //         .await?;

    //     if melt_response.state == MeltBtcOnchainState::Paid {
    //         self.localstore
    //             .delete_proofs(&mut tx, &total_proofs)
    //             .await?;
    //     }
    //     tx.commit().await?;
    //     Ok(melt_response)
    // }

    async fn create_secrets(
        &self,
        keyset_id: &KeysetId,
        amount: u32,
    ) -> Result<Vec<(String, BlindingFactor)>, MonexoWalletError> {
        let mut tx = self.localstore.begin_tx().await?;
        let all_keysets = self.localstore.get_keysets(&mut tx).await?;
        let keyset = all_keysets
            .iter()
            .find(|k| k.keyset_id == *keyset_id)
            .expect("keyset not found create-secrets");

        let start_index = (keyset.last_index + 1) as u32;
        let secret_range = self.secret.derive_range(keyset_id, start_index, amount)?;

        self.localstore
            .update_keyset_last_index(
                &mut tx,
                &WalletKeyset {
                    last_index: (start_index + amount - 1) as u64,
                    ..keyset.clone()
                },
            )
            .await?;
        tx.commit().await?;
        Ok(secret_range)
    }

    // pub async fn swap_tokens(
    //     &self,
    //     wallet_keyset: &WalletKeyset,
    //     tokens: &TokenV3,
    //     splt_amount: Amount,
    // ) -> Result<(TokenV3, TokenV3), MokshaWalletError> {
    //     let total_token_amount = tokens.total_amount();
    //     let first_amount: Amount = (total_token_amount - splt_amount.0).into();
    //     let first_secrets = self
    //         .create_secrets(&wallet_keyset.keyset_id, first_amount.split().len() as u32)
    //         .await?;
    //     let first_outputs = self.create_blinded_messages(
    //         &wallet_keyset.keyset_id,
    //         first_amount,
    //         first_secrets.clone(),
    //     )?;

    //     // ############################################################################

    //     let second_amount = splt_amount.clone();
    //     let second_secrets = self
    //         .create_secrets(&wallet_keyset.keyset_id, second_amount.split().len() as u32)
    //         .await?;
    //     let second_outputs = self.create_blinded_messages(
    //         &wallet_keyset.keyset_id,
    //         second_amount,
    //         second_secrets.clone(),
    //     )?;

    //     let mut total_outputs = vec![];
    //     total_outputs.extend(get_blinded_msg(first_outputs.clone()));
    //     total_outputs.extend(get_blinded_msg(second_outputs.clone()));

    //     if tokens.total_amount() != total_outputs.total_amount() {
    //         return Err(MokshaWalletError::InvalidProofs);
    //     }

    //     let split_result = self
    //         .client
    //         .post_swap(&wallet_keyset.mint_url, tokens.proofs(), total_outputs)
    //         .await?;

    //     if split_result.signatures.is_empty() {
    //         return Ok((TokenV3::empty(), TokenV3::empty()));
    //     }

    //     let len_first = first_secrets.len();
    //     let secrets = [first_secrets, second_secrets].concat();
    //     let outputs = [first_outputs, second_outputs].concat();

    //     let secrets = secrets.into_iter().map(|(s, _)| s).collect::<Vec<String>>();

    //     let proofs = self
    //         .create_proofs_from_blinded_signatures(
    //             &wallet_keyset.keyset_id,
    //             &wallet_keyset.public_keys,
    //             split_result.signatures,
    //             secrets,
    //             outputs,
    //         )?
    //         .proofs();

    //     let first_tokens: TokenV3 = (
    //         wallet_keyset.mint_url.to_owned(),
    //         wallet_keyset.currency_unit.clone(),
    //         proofs[0..len_first].to_vec().into(),
    //     )
    //         .into();
    //     let second_tokens: TokenV3 = (
    //         wallet_keyset.mint_url.to_owned(),
    //         wallet_keyset.currency_unit.clone(),
    //         proofs[len_first..proofs.len()].to_vec().into(),
    //     )
    //         .into();

    //     if tokens.total_amount() != first_tokens.total_amount() + second_tokens.total_amount() {
    //         println!(
    //             "Error in swap: input {:?} != output {:?} + {:?}",
    //             tokens.total_amount(),
    //             first_tokens.total_amount(),
    //             second_tokens.total_amount()
    //         );
    //     }

    //     Ok((first_tokens, second_tokens))
    // }

    // pub async fn get_mint_info(
    //     &self,
    // ) -> Result<MintInfoResponse, MokshaWalletError> {
    //     self.client.get_info().await
    // }

    // async fn melt_token(
    //     &self,
    //     mint_url: &Url,
    //     quote_id: String,
    //     proofs: &Proofs,
    //     fee_blinded_messages: Vec<BlindedMessage>,
    // ) -> Result<PostMeltBolt11Response, MokshaWalletError> {
    //     let melt_response = self
    //         .client
    //         .post_melt_bolt11(mint_url, proofs.clone(), quote_id, fee_blinded_messages)
    //         .await?;

    //     if melt_response.paid {
    //         let mut tx = self.localstore.begin_tx().await?;
    //         self.localstore.delete_proofs(&mut tx, proofs).await?;
    //         tx.commit().await?;
    //     }
    //     Ok(melt_response)
    // }

    // fn decode_invoice(payment_request: &str) -> Result<LNInvoice, MokshaWalletError> {
    //     LNInvoice::from_str(payment_request)
    //         .map_err(|err| MokshaWalletError::DecodeInvoice(payment_request.to_owned(), err))
    // }

    // fn get_invoice_amount(payment_request: &str) -> Result<u64, MokshaWalletError> {
    //     let invoice = Self::decode_invoice(payment_request)?;
    //     Ok(invoice
    //         .amount_milli_satoshis()
    //         .ok_or_else(|| MokshaWalletError::InvalidInvoice(payment_request.to_owned()))?
    //         / 1000)
    // }

    pub async fn mint_tokens(
        &self,
        wallet_keyset: &WalletKeyset,
        amount: Amount,
        quote_id: String,
    ) -> Result<TokenV3, MonexoWalletError> {
        let split_amount = amount.split();

        let secret_range = self
            .create_secrets(&wallet_keyset.keyset_id, split_amount.len() as u32)
            .await?;

        let blinded_messages = split_amount
            .into_iter()
            .zip(secret_range)
            .map(|(amount, (secret, blinding_factor))| {
                let b_ = self.dhke.step1_alice(&secret, &blinding_factor)?;
                Ok((
                    BlindedMessage {
                        amount,
                        b_,
                        id: wallet_keyset.keyset_id.to_string(), // FIXME use keyset_id
                    },
                    blinding_factor,
                    secret,
                ))
            })
            .collect::<Result<Vec<(_, _, _)>, MonexoWalletError>>()?;

        let signatures = self
            .client
            .post_mint_onchain(
                &wallet_keyset.mint_url,
                quote_id,
                blinded_messages
                    .clone()
                    .into_iter()
                    .map(|(msg, _, _)| msg)
                    .collect::<Vec<BlindedMessage>>(),
            )
            .await?.signatures;

        // step 3: unblind signatures
        let current_keyset_id = wallet_keyset.keyset_id.to_string(); // FIXME

        let proofs = signatures
            .iter()
            .zip(blinded_messages)
            .map(|(p, (_, priv_key, secret))| {
                let key = wallet_keyset
                    .public_keys
                    .get(&p.amount)
                    .expect("msg amount not found in mint keys");
                let pub_alice = self.dhke.step3_alice(p.c_, priv_key, *key).unwrap();
                Proof::new(p.amount, secret, pub_alice, current_keyset_id.clone())
            })
            .collect::<Vec<Proof>>()
            .into();

        let tokens: TokenV3 = (wallet_keyset.mint_url.to_owned(), proofs).into();
        let mut tx = self.localstore.begin_tx().await?;
        self.localstore
            .add_proofs(&mut tx, &tokens.proofs())
            .await?;
        tx.commit().await?;

        Ok(tokens)
    }

    pub async fn create_blank(
        &self,
        fee_reserve: Amount,
        keyset_id: &KeysetId,
    ) -> Result<Vec<(BlindedMessage, BlindingFactor, String)>, MonexoWalletError> {
        if fee_reserve.0 == 0 {
            return Ok(vec![]);
        }

        let fee_reserve_float = fee_reserve.0 as f64;
        let count = (fee_reserve_float.log2().ceil() as u64).max(1);

        let secret_range = self.create_secrets(keyset_id, count as u32).await?;
        let blinded_messages = secret_range
            .into_iter()
            .map(|(secret, blinding_factor)| {
                let b_ = self.dhke.step1_alice(secret.clone(), &blinding_factor)?;
                Ok((
                    BlindedMessage {
                        amount: 1,
                        b_,
                        id: keyset_id.to_string(),
                    },
                    blinding_factor,
                    secret,
                ))
            })
            .collect::<Result<Vec<(_, _, _)>, MonexoWalletError>>()?;

        Ok(blinded_messages)
    }

    #[allow(dead_code)]
    fn create_blinded_messages(
        &self,
        keyset_id: &KeysetId,
        amount: Amount,
        secrets_factors: Vec<(String, BlindingFactor)>,
    ) -> Result<Vec<(BlindedMessage, BlindingFactor)>, MonexoWalletError> {
        let split_amount = amount.split();

        split_amount
            .into_iter()
            .zip(secrets_factors)
            .map(|(amount, (secret, blinding_factor))| {
                let b_ = self.dhke.step1_alice(secret, &blinding_factor)?;
                Ok((
                    BlindedMessage {
                        amount,
                        b_,
                        id: keyset_id.to_string(),
                    },
                    blinding_factor,
                ))
            })
            .collect::<Result<Vec<(_, _)>, MonexoWalletError>>()
    }

    #[allow(dead_code)]
    fn create_proofs_from_blinded_signatures(
        &self,
        keyset_id: &KeysetId,
        pub_keys: &HashMap<u64, PublicKey>,
        signatures: Vec<BlindedSignature>,
        secrets: Vec<String>,
        outputs: Vec<(BlindedMessage, BlindingFactor)>,
    ) -> Result<Proofs, MonexoWalletError> {
        let current_keyset_id = keyset_id.to_string(); // FIXME

        let blinding_factors = outputs
            .into_iter()
            .map(|(_, secret)| secret)
            .collect::<Vec<BlindingFactor>>();

        Ok(signatures
            .iter()
            .zip(blinding_factors)
            .zip(secrets)
            .map(|((p, blinding_factor), secret)| {
                let key = pub_keys
                    .get(&p.amount)
                    .ok_or(MonexoWalletError::PubkeyNotFound)?;
                let pub_alice = self
                    .dhke
                    .step3_alice(p.c_, blinding_factor.to_owned(), *key)?;
                Ok(Proof::new(
                    p.amount,
                    secret,
                    pub_alice,
                    current_keyset_id.clone(),
                ))
            })
            .collect::<Result<Vec<_>, MonexoWalletError>>()?
            .into())
    }

    pub async fn get_proofs(&self) -> Result<Proofs, MonexoWalletError> {
        let mut tx = self.localstore.begin_tx().await?;
        let proofs = self.localstore.get_proofs(&mut tx).await?;
        tx.commit().await?;
        Ok(proofs)
    }
}
