use monexo_core::{blind::{BlindedMessage, BlindedSignature}, dhke::Dhke, keyset::MintKeyset};
use sqlx::Transaction;

use crate::{config::MintConfig, database::{postgres::PostgresDB, Database}, error::MonexoMintError};
use tracing::instrument;


#[derive(Clone)]
pub struct Mint<DB: Database = PostgresDB> {
    // pub lightning: Arc<dyn Lightning + Send + Sync>,
    // pub lightning_type: LightningType,
    pub keyset: MintKeyset,
    pub db: DB,
    pub dhke: Dhke,
    // pub onchain: Option<Arc<dyn BtcOnchain + Send + Sync>>,
    pub config: MintConfig,
    // pub build_params: BuildParams,
}

impl<DB> Mint<DB>
where
    DB: Database,
{
    pub fn create_blinded_signatures(
        &self,
        blinded_messages: &[BlindedMessage],
        keyset: &MintKeyset,
    ) -> Result<Vec<BlindedSignature>, MonexoMintError> {
        blinded_messages
            .iter()
            .map(|blinded_msg| {
                let private_key = keyset
                    .private_keys
                    .get(&blinded_msg.amount)
                    .ok_or(MonexoMintError::PrivateKeyNotFound)?;
                let blinded_sig = self.dhke.step2_bob(blinded_msg.b_, private_key)?;
                Ok(BlindedSignature {
                    id: keyset.keyset_id.clone(),
                    amount: blinded_msg.amount,
                    c_: blinded_sig,
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }

    #[instrument(level = "debug", skip(self, outputs, keyset), err)]
    pub async fn mint_tokens(
        &self,
        tx: &mut Transaction<'_, <DB as Database>::DB>,
        key: String,
        outputs: &[BlindedMessage],
        keyset: &MintKeyset,
        return_error: bool,
    ) -> Result<Vec<BlindedSignature>, MonexoMintError> {
        self.create_blinded_signatures(outputs, keyset)
    }
}
