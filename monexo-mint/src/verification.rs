use monexo_core::{amount::Amount, blind::BlindedMessage, primitives::CurrencyUnit};
use tracing::instrument;

use crate::{error::MonexoMintError, mint::Mint};

/// Verification result
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Verification {
    /// Value in request
    pub amount: Amount,
    /// Unit of request
    pub unit: Option<CurrencyUnit>,
}

impl Mint {
    /// Verifies outputs
    /// Checks outputs are unique, of the same unit and not signed before
    #[instrument(skip_all)]
    pub async fn verify_outputs(
        &self,
        outputs: &[BlindedMessage],
    ) -> Result<Verification, MonexoMintError> {
        if outputs.is_empty() {
            return Ok(Verification {
                amount: Amount(0),
                unit: None,
            });
        }

        if Self::has_duplicate_pubkeys(outputs) {
            return Err(MonexoMintError::SwapHasDuplicatePromises);
        }

        // todo!()

        // self.check_output_already_signed(outputs).await?;

        // let unit = self.verify_outputs_keyset(outputs).await?;

        // let amount = Amount::try_sum(outputs.iter().map(|o| o.amount).collect::<Vec<Amount>>())?;
        // let amount = outputs.total_amount();
        // let amount = outputs.iter().fold(0, |acc, x| acc + x.amount);

        todo!()

        // Ok(Verification {
        //     amount: Amount(amount),
        //     unit: Some(unit),
        // })
    }
}
