use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Invoice {
    pub amount: u64,
    pub payment_request: String,
}

impl Invoice {
    pub const fn new(amount: u64, payment_request: String) -> Self {
        Self {
            amount,
            payment_request,
        }
    }
}
