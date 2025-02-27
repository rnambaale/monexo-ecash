use std::collections::HashMap;

use async_trait::async_trait;
use monexo_core::{keyset::KeysetId, proof::Proofs};
use secp256k1::PublicKey;

use crate::error::MonexoWalletError;

#[cfg(not(target_arch = "wasm32"))]
pub mod sqlite;

#[derive(Debug, Clone)]
pub struct WalletKeyset {
    /// primary key
    pub id: Option<u64>,
    pub keyset_id: KeysetId,
    // pub currency_unit: CurrencyUnit,
    /// last index used for deriving keys from the master key
    pub last_index: u64,
    pub public_keys: HashMap<u64, PublicKey>,
    pub active: bool,
}

impl WalletKeysetFilter for Vec<WalletKeyset> {
    fn get_active(&self) -> Option<&WalletKeyset> {
        self.iter()
            .find(|k| k.active)
    }
}

pub trait WalletKeysetFilter {
    fn get_active(&self) -> Option<&WalletKeyset>;
}

impl WalletKeyset {
    pub fn new(
        keyset_id: &KeysetId,
        last_index: u64,
        public_keys: HashMap<u64, PublicKey>,
        active: bool,
    ) -> Self {
        Self {
            id: None,
            keyset_id: keyset_id.to_owned(),
            last_index,
            public_keys,
            active,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait(?Send)]
pub trait LocalStore {
    type DB: sqlx::Database;
    async fn begin_tx(&self) -> Result<sqlx::Transaction<Self::DB>, MonexoWalletError>;

    async fn delete_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MonexoWalletError>;

    async fn add_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        proofs: &Proofs,
    ) -> Result<(), MonexoWalletError>;

    async fn get_proofs(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Proofs, MonexoWalletError>;

    async fn get_keysets(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Vec<WalletKeyset>, MonexoWalletError>;

    async fn upsert_keyset(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        keyset: &WalletKeyset,
    ) -> Result<(), MonexoWalletError>;

    async fn update_keyset_last_index(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        keyset: &WalletKeyset,
    ) -> Result<(), MonexoWalletError>;

    async fn add_seed(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
        seed_words: &str,
    ) -> Result<(), MonexoWalletError>;

    async fn get_seed(
        &self,
        tx: &mut sqlx::Transaction<Self::DB>,
    ) -> Result<Option<String>, MonexoWalletError>;
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use rand::thread_rng;

    use secp256k1::PublicKey;

    fn generate_test_map() -> HashMap<u32, PublicKey> {
        let mut map = HashMap::new();
        let secp = secp256k1::Secp256k1::new();

        for i in 0..10 {
            let secret_key = secp256k1::SecretKey::new(&mut thread_rng());
            let public_key = PublicKey::from_secret_key(&secp, &secret_key);
            map.insert(i, public_key);
        }

        map
    }

    #[test]
    fn test_() {
        //let x: HashMap<u64, PublicKey<Secp256k1>, RandomState>;
        let data = generate_test_map();
        let json = serde_json::to_string(&data).unwrap();
        println!("{:?}", json);
    }
}
