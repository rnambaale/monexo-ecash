use std::collections::HashSet;

use crate::{
    config::{
        BuildParams, DatabaseConfig, MintConfig, MintInfoConfig, OnchainConfig, ServerConfig,
        TracingConfig,
    },
    database::{postgres::PostgresDB, Database},
    error::MonexoMintError,
};
use monexo_core::{
    blind::{BlindedMessage, BlindedSignature, TotalAmount},
    dhke::Dhke,
    keyset::MintKeyset,
    primitives::OnchainMeltQuote,
    proof::Proofs,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::instruction::Instruction;
use solana_sdk::signature::Signature;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Keypair,
    signer::{EncodableKey, Signer},
};
use spl_token::instruction::transfer_checked;
use sqlx::Transaction;
use std::str::FromStr;
use tracing::instrument;

#[derive(Clone)]
pub struct Mint<DB: Database = PostgresDB> {
    pub keyset: MintKeyset,
    pub ugx_keyset: MintKeyset,
    pub db: DB,
    pub dhke: Dhke,
    pub config: MintConfig,
    pub build_params: BuildParams,
}

impl<DB> Mint<DB>
where
    DB: Database,
{
    pub fn new(db: DB, config: MintConfig, build_params: BuildParams) -> Self {
        Self {
            keyset: MintKeyset::new(
                &config.privatekey.clone(),
                &config.derivation_path.clone().unwrap_or_default(),
            ),
            ugx_keyset: MintKeyset::new(
                &config.privatekey.clone(),
                &config.ugx_derivation_path.clone().unwrap_or_default(),
            ),
            db,
            dhke: Dhke::new(),
            config,
            build_params,
        }
    }

    pub fn create_blinded_signatures(
        &self,
        blinded_messages: &[BlindedMessage],
    ) -> Result<Vec<BlindedSignature>, MonexoMintError> {
        blinded_messages
            .iter()
            .map(|blinded_msg| {
                let mint_keyset = self.get_mint_keyset(&blinded_msg.id)?;

                let private_key = mint_keyset
                    .private_keys
                    .get(&blinded_msg.amount)
                    .ok_or(MonexoMintError::PrivateKeyNotFound)?;
                let blinded_sig = self.dhke.step2_bob(blinded_msg.b_, private_key)?;
                Ok(BlindedSignature {
                    id: mint_keyset.keyset_id.clone(),
                    amount: blinded_msg.amount,
                    c_: blinded_sig,
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }

    #[instrument(level = "debug", skip(self, outputs), err)]
    pub async fn mint_tokens(
        &self,
        tx: &mut Transaction<'_, <DB as Database>::DB>,
        key: String,
        outputs: &[BlindedMessage],
        return_error: bool,
    ) -> Result<Vec<BlindedSignature>, MonexoMintError> {
        self.create_blinded_signatures(outputs)
    }

    pub fn get_mint_keyset(&self, keyset_id: &str) -> Result<&MintKeyset, MonexoMintError> {
        if keyset_id.to_string() == self.keyset.keyset_id {
            return Ok(&self.keyset);
        }
        if keyset_id.to_string() == self.ugx_keyset.keyset_id {
            return Ok(&self.ugx_keyset);
        }

        Err(MonexoMintError::PrivateKeyNotFound)
    }

    pub async fn check_used_proofs(
        &self,
        tx: &mut Transaction<'_, <DB as Database>::DB>,
        proofs: &Proofs,
    ) -> Result<(), MonexoMintError> {
        let used_proofs = self.db.get_used_proofs(tx).await?.proofs();
        for used_proof in used_proofs {
            if proofs.proofs().contains(&used_proof) {
                return Err(MonexoMintError::ProofAlreadyUsed(format!("{used_proof:?}")));
            }
        }
        Ok(())
    }

    #[instrument(level = "debug", skip(self, proofs), err)]
    pub async fn melt_onchain(
        &self,
        quote: &OnchainMeltQuote,
        proofs: &Proofs,
    ) -> Result<Signature, MonexoMintError> {
        let proofs_amount = proofs.total_amount();

        if proofs_amount < quote.amount {
            return Err(MonexoMintError::NotEnoughTokens(quote.amount));
        }

        let mut tx = self.db.begin_tx().await?;
        self.check_used_proofs(&mut tx, proofs).await?;

        // TODO: Confirm valid mint signatures on all the proofs

        let amount_to_send = quote.amount - quote.fee_total;
        let send_response =
            Self::send_coins(&quote.address, &quote.reference, amount_to_send).await?;

        self.db.add_used_proofs(&mut tx, proofs).await?;
        tx.commit().await?;

        Ok(send_response)
    }

    fn has_duplicate_pubkeys(outputs: &[BlindedMessage]) -> bool {
        let mut uniq = HashSet::new();
        !outputs.iter().all(move |x| uniq.insert(x.b_))
    }

    #[instrument(level = "debug", skip_all, err)]
    pub async fn swap(
        &self,
        proofs: &Proofs,
        blinded_messages: &[BlindedMessage],
    ) -> Result<Vec<BlindedSignature>, MonexoMintError> {
        let mut tx = self.db.begin_tx().await?;
        self.check_used_proofs(&mut tx, proofs).await?;

        if Self::has_duplicate_pubkeys(blinded_messages) {
            return Err(MonexoMintError::SwapHasDuplicatePromises);
        }

        // TODO: Confirm valid mint signatures on all the proofs

        let sum_proofs = proofs.total_amount();

        let promises = self.create_blinded_signatures(blinded_messages)?;
        let amount_promises = promises.total_amount();
        if sum_proofs != amount_promises {
            return Err(MonexoMintError::SwapAmountMismatch(format!(
                "Swap amount mismatch: {sum_proofs} != {amount_promises}"
            )));
        }

        self.db.add_used_proofs(&mut tx, proofs).await?;
        tx.commit().await?;
        Ok(promises)
    }

    #[instrument(level = "debug", skip_all, err)]
    pub async fn exchange(
        &self,
        amount_to_exchange: u64,
        proofs: &Proofs,
        blinded_messages: &[BlindedMessage],
    ) -> Result<Vec<BlindedSignature>, MonexoMintError> {
        let mut tx = self.db.begin_tx().await?;
        self.check_used_proofs(&mut tx, proofs).await?;

        if Self::has_duplicate_pubkeys(blinded_messages) {
            return Err(MonexoMintError::SwapHasDuplicatePromises);
        }

        // TODO: Confirm valid mint signatures on all the proofs

        let sum_proofs = proofs.total_amount();

        let promises = self.create_blinded_signatures(blinded_messages)?;
        // let amount_promises = promises.total_amount();
        // TODO: fetch the current rate and use it to check that the sum on blinded messages adds up
        if sum_proofs != amount_to_exchange {
            return Err(MonexoMintError::SwapAmountMismatch(format!(
                "Exchange amount mismatch: {sum_proofs} != {amount_to_exchange}"
            )));
        }

        self.db.add_used_proofs(&mut tx, proofs).await?;
        tx.commit().await?;
        Ok(promises)
    }

    async fn send_coins(
        recipient: &str,
        reference: &str,
        amount: u64,
    ) -> Result<Signature, MonexoMintError> {
        let rpc_url = "https://api.devnet.solana.com";
        let client = RpcClient::new(rpc_url.to_string());

        let sender_keypair =
            Keypair::read_from_file("./../wallet.json").expect("Failed to load keypair");

        // Step 3: Define USDC Mint Address on Devnet
        let usdc_mint = Pubkey::from_str("4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU").unwrap();

        // Step 4: Compute Source ATA (must be owned by the sender)
        let source_ata = spl_associated_token_account::get_associated_token_address(
            &sender_keypair
                .try_pubkey()
                .expect("Failed to load mint pubkey"),
            &usdc_mint,
        );

        // Step 5: Define recipient and compute their USDC ATA
        let recipient_pubkey = Pubkey::from_str(recipient)?;
        let recipient_ata = spl_associated_token_account::get_associated_token_address(
            &recipient_pubkey,
            &usdc_mint,
        );

        // Step 6: Check if the recipient ATA exists
        let recipient_ata_info = client.get_account(&recipient_ata).await;
        let mut instructions = vec![];

        if recipient_ata_info.is_err() {
            // If recipient ATA does not exist, create it
            let create_ata_ix =
                spl_associated_token_account::instruction::create_associated_token_account(
                    &sender_keypair.pubkey(), // Payer
                    &recipient_pubkey,        // Wallet owner
                    &usdc_mint,               // Token mint
                    &spl_token::id(),
                );
            instructions.push(create_ata_ix);
        }

        // Step 7: Transfer USDC (with decimals checked)
        let transfer_ix = transfer_checked(
            &spl_token::id(),            // SPL Token Program ID
            &source_ata,                 // Source ATA
            &usdc_mint,                  // Token Mint Address
            &recipient_ata,              // Destination ATA
            &sender_keypair.pubkey(),    // Authority (signer)
            &[&sender_keypair.pubkey()], // Signer list
            amount, //This is already a micro-usd Amount (1 USDC = 1_000_000 because of 6 decimal places)
            6,      // USDC has 6 decimals
        )?;

        // Add reference key to the transaction
        let mut accounts = transfer_ix.accounts;
        let reference = Pubkey::from_str(&reference).expect("reference is not a valid public key");

        accounts.push(solana_sdk::instruction::AccountMeta::new_readonly(
            reference, false,
        ));

        let transfer_ix = Instruction {
            program_id: transfer_ix.program_id,
            accounts,
            data: transfer_ix.data,
        };

        instructions.push(transfer_ix);

        // Step 8: Create and Send Transaction
        let recent_blockhash = client.get_latest_blockhash().await?;
        let tx = solana_sdk::transaction::Transaction::new_signed_with_payer(
            &instructions,
            Some(&sender_keypair.pubkey()),
            &[&sender_keypair],
            recent_blockhash,
        );

        let signature = client.send_and_confirm_transaction(&tx).await?;

        Ok(signature)
    }
}

#[derive(Debug, Default)]
pub struct MintBuilder {
    private_key: Option<String>,
    derivation_path: Option<String>,
    ugx_derivation_path: Option<String>,
    db_config: Option<DatabaseConfig>,
    mint_info_settings: Option<MintInfoConfig>,
    server_config: Option<ServerConfig>,
    onchain_config: Option<OnchainConfig>,
    tracing_config: Option<TracingConfig>,
}

impl MintBuilder {
    pub fn new() -> Self {
        MintBuilder {
            private_key: None,
            derivation_path: None,
            ugx_derivation_path: None,
            db_config: None,
            mint_info_settings: None,
            server_config: None,
            onchain_config: None,
            tracing_config: None,
        }
    }

    pub fn with_db(mut self, db_config: Option<DatabaseConfig>) -> Self {
        self.db_config = db_config;
        self
    }

    pub fn with_mint_info(mut self, mint_info: Option<MintInfoConfig>) -> Self {
        self.mint_info_settings = mint_info;
        self
    }

    pub fn with_server(mut self, server_config: Option<ServerConfig>) -> Self {
        self.server_config = server_config;
        self
    }

    pub fn with_derivation_path(mut self, derivation_path: Option<String>) -> Self {
        self.derivation_path = derivation_path;
        self
    }

    pub fn with_ugx_derivation_path(mut self, ugx_derivation_path: Option<String>) -> Self {
        self.ugx_derivation_path = ugx_derivation_path;
        self
    }

    pub fn with_private_key(mut self, private_key: String) -> Self {
        self.private_key = Some(private_key);
        self
    }

    pub fn with_onchain(mut self, onchain_config: Option<OnchainConfig>) -> Self {
        self.onchain_config = onchain_config;
        self
    }

    pub fn with_tracing(mut self, tracing_config: Option<TracingConfig>) -> Self {
        self.tracing_config = tracing_config;
        self
    }

    pub async fn build(self) -> Result<Mint<PostgresDB>, MonexoMintError> {
        let db_config = self.db_config.expect("db-config not set");
        let db = PostgresDB::new(&db_config).await?;
        db.migrate().await;

        Ok(Mint::new(
            db,
            MintConfig::new(
                self.private_key.expect("private-key not set"),
                self.derivation_path,
                self.ugx_derivation_path,
                self.mint_info_settings.unwrap_or_default(),
                self.server_config.unwrap_or_default(),
                db_config,
                self.onchain_config,
                self.tracing_config,
            ),
            BuildParams::from_env(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use monexo_core::blind::{BlindedMessage, TotalAmount};
    use monexo_core::dhke;
    use monexo_core::fixture::read_fixture_as;
    use monexo_core::primitives::PostSwapRequest;
    use monexo_core::proof::Proofs;
    use testcontainers::runners::AsyncRunner;
    use testcontainers::{ContainerAsync, ImageExt};
    use testcontainers_modules::postgres::Postgres;

    use crate::{
        config::{DatabaseConfig, MintConfig},
        database::postgres::PostgresDB,
        mint::Mint,
    };

    async fn create_postgres_image() -> anyhow::Result<ContainerAsync<Postgres>> {
        Ok(Postgres::default()
            .with_host_auth()
            .with_tag("16.6-alpine")
            .start()
            .await?)
    }

    async fn create_mock_db_empty(port: u16) -> anyhow::Result<PostgresDB> {
        let connection_string =
            &format!("postgres://postgres:postgres@127.0.0.1:{}/postgres", port);
        let db = PostgresDB::new(&DatabaseConfig {
            db_url: connection_string.to_owned(),
            ..Default::default()
        })
        .await?;
        db.migrate().await;
        Ok(db)
    }

    async fn create_mint_from_mocks(mock_db: PostgresDB) -> anyhow::Result<Mint> {
        Ok(Mint::new(
            mock_db,
            MintConfig {
                privatekey: "TEST_PRIVATE_KEY".to_string(),
                derivation_path: Some("0/0/0/0".to_string()),
                ugx_derivation_path: Some("0/0/0/1".to_string()),
                ..Default::default()
            },
            Default::default(),
        ))
    }

    #[tokio::test]
    async fn test_create_blind_signatures() -> anyhow::Result<()> {
        let node = create_postgres_image().await?;

        let mint = create_mint_from_mocks(
            create_mock_db_empty(node.get_host_port_ipv4(5432).await?).await?,
        )
        .await?;

        let blinded_messages = vec![BlindedMessage {
            amount: 8,
            b_: dhke::public_key_from_hex(
                "02634a2c2b34bec9e8a4aba4361f6bf202d7fa2365379b0840afe249a7a9d71239",
            ),
            id: "00f4683f9caf8793".to_owned(),
        }];

        let result = mint.create_blinded_signatures(&blinded_messages)?;

        assert_eq!(1, result.len());
        assert_eq!(8, result[0].amount);
        assert_eq!(
            dhke::public_key_from_hex(
                "037074c4f53e326ee14ed67125f387d160e0e729351471b69ad41f7d5d21071e15"
            ),
            result[0].c_
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_swap_zero() -> anyhow::Result<()> {
        let node = create_postgres_image().await?;
        let blinded_messages = vec![];
        let mint = create_mint_from_mocks(
            create_mock_db_empty(node.get_host_port_ipv4(5432).await?).await?,
        )
        .await?;

        let proofs = Proofs::empty();
        let result = mint.swap(&proofs, &blinded_messages).await?;

        assert!(result.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_swap_64_in_20() -> anyhow::Result<()> {
        let node = create_postgres_image().await?;
        let mint = create_mint_from_mocks(
            create_mock_db_empty(node.get_host_port_ipv4(5432).await?).await?,
        )
        .await?;
        let request = read_fixture_as::<PostSwapRequest>("post_swap_request_64_20.json")?;

        let result = mint.swap(&request.inputs, &request.outputs).await?;
        assert_eq!(result.total_amount(), 64);

        let prv_last = result.get(result.len() - 2).expect("element not found");
        let last = result.last().expect("element not found");

        assert_eq!(prv_last.amount, 4);
        assert_eq!(last.amount, 16);
        Ok(())
    }

    #[tokio::test]
    async fn test_swap_duplicate_key() -> anyhow::Result<()> {
        let node = create_postgres_image().await?;
        let mint = create_mint_from_mocks(
            create_mock_db_empty(node.get_host_port_ipv4(5432).await?).await?,
        )
        .await?;
        let request = read_fixture_as::<PostSwapRequest>("post_swap_request_duplicate_key.json")?;

        let result = mint.swap(&request.inputs, &request.outputs).await;
        assert!(result.is_err());
        Ok(())
    }
}
