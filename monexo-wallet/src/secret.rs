use std::str::FromStr;

use bip32::{Seed, XPrv};
use bip39::Mnemonic;
use monexo_core::{blind::BlindingFactor, keyset::KeysetId};
use rand::Rng;
use secp256k1::SecretKey;

use crate::error::MonexoWalletError;

enum DerivationType {
    Secret = 0,
    Blinding = 1,
}

pub struct DeterministicSecret {
    pub seed: Seed,
}

impl Clone for DeterministicSecret {
    fn clone(&self) -> Self {
        Self {
            seed: Seed::new(*self.seed.as_bytes()),
        }
    }
}

impl DeterministicSecret {
    pub fn from_seed_words(seed_words: &str) -> Result<Self, MonexoWalletError> {
        let mnemonic = Mnemonic::from_str(seed_words)?;
        let seed = Seed::new(mnemonic.to_seed(""));
        Ok(Self { seed })
    }

    pub fn from_random_seed() -> Result<Self, MonexoWalletError> {
        let mut rng = rand::thread_rng();
        let entropy: [u8; 16] = rng.gen(); // 16 bytes for 12 words mnemonic
        let mnemonic = Mnemonic::from_entropy(&entropy)?;
        let seed = Seed::new(mnemonic.to_seed(""));
        Ok(Self { seed })
    }

    pub fn generate_random_seed_words() -> Result<String, MonexoWalletError> {
        let mut rng = rand::thread_rng();
        let entropy: [u8; 16] = rng.gen(); // 16 bytes for 12 words mnemonic
        let mnemonic = Mnemonic::from_entropy(&entropy)?;
        Ok(mnemonic.words().collect::<Vec<&str>>().join(" "))
    }

    fn derive_private_key(
        &self,
        keyset_id: u32,
        counter: u32,
        secret_or_blinding: DerivationType,
    ) -> Result<Vec<u8>, MonexoWalletError> {
        let secret_or_blinding = secret_or_blinding as u32;
        let derivation_path = format!("m/129372'/0'/{keyset_id}'/{counter}'/{secret_or_blinding}");
        let derivation_path = bip32::DerivationPath::from_str(&derivation_path)?;
        let key = XPrv::derive_from_path(&self.seed, &derivation_path)?;
        Ok(key.private_key().to_bytes().to_vec())
    }

    fn derive_secret(&self, keyset_id: u32, counter: u32) -> Result<String, MonexoWalletError> {
        let key = self.derive_private_key(keyset_id, counter, DerivationType::Secret)?;
        Ok(hex::encode(key))
    }

    pub fn derive_range(
        &self,
        keyset_id: &KeysetId,
        start: u32,
        length: u32,
    ) -> Result<Vec<(String, BlindingFactor)>, MonexoWalletError> {
        let keyset_id = keyset_id.as_int()?;
        Ok((start..start + length)
            .map(|i| {
                let key = self.derive_secret(keyset_id, i).unwrap();
                let blinding_factor = self.derive_blinding_factor(keyset_id, i).unwrap(); // FIXME
                (key, blinding_factor)
            })
            .collect::<Vec<(String, BlindingFactor)>>())
    }

    fn derive_blinding_factor(
        &self,
        keyset_id: u32,
        counter: u32,
    ) -> Result<BlindingFactor, MonexoWalletError> {
        let key = self.derive_private_key(keyset_id, counter, DerivationType::Blinding)?;
        Ok(SecretKey::from_slice(&key)?.into())
    }
}

pub fn convert_hex_to_int(keyset_id_hex: &str) -> Result<u32, MonexoWalletError> {
    let bytes = hex::decode(keyset_id_hex)?;
    let bytes_array: [u8; 8] = bytes[0..8].try_into()?;
    let num = u64::from_be_bytes(bytes_array);
    Ok((num % (2u64.pow(31) - 1)) as u32)
}

#[cfg(test)]
mod tests {

    use monexo_core::keyset::KeysetId;

    use super::{convert_hex_to_int, DeterministicSecret};

    #[test]
    fn test_keyset_id_conversion() -> anyhow::Result<()> {
        let int_value = convert_hex_to_int("009a1f293253e41e")?;
        assert_eq!(864559728, int_value);
        Ok(())
    }

    #[test]
    fn test_generate_seed_words() -> anyhow::Result<()> {
        let seed_words = DeterministicSecret::generate_random_seed_words()?;
        println!("{}", seed_words);
        assert_eq!(12, seed_words.split_whitespace().count());
        Ok(())
    }

    #[test]
    fn test_secret_derivation() -> anyhow::Result<()> {
        let phrase =
            "half depart obvious quality work element tank gorilla view sugar picture humble";
        let deterministic_secret = DeterministicSecret::from_seed_words(phrase)?;

        let secrets = [
            "485875df74771877439ac06339e284c3acfcd9be7abf3bc20b516faeadfe77ae",
            "8f2b39e8e594a4056eb1e6dbb4b0c38ef13b1b2c751f64f810ec04ee35b77270",
            "bc628c79accd2364fd31511216a0fab62afd4a18ff77a20deded7b858c9860c8",
            "59284fd1650ea9fa17db2b3acf59ecd0f2d52ec3261dd4152785813ff27a33bf",
            "576c23393a8b31cc8da6688d9c9a96394ec74b40fdaf1f693a6bb84284334ea0",
        ];

        for (i, secret) in secrets.iter().enumerate() {
            let key = deterministic_secret.derive_secret(864559728, i as u32)?;
            assert_eq!(secret.to_owned(), key);
        }

        let blinding_factors = [
            "ad00d431add9c673e843d4c2bf9a778a5f402b985b8da2d5550bf39cda41d679",
            "967d5232515e10b81ff226ecf5a9e2e2aff92d66ebc3edf0987eb56357fd6248",
            "b20f47bb6ae083659f3aa986bfa0435c55c6d93f687d51a01f26862d9b9a4899",
            "fb5fca398eb0b1deb955a2988b5ac77d32956155f1c002a373535211a2dfdc29",
            "5f09bfbfe27c439a597719321e061e2e40aad4a36768bb2bcc3de547c9644bf9",
        ];

        for (i, factor) in blinding_factors.iter().enumerate() {
            let key = deterministic_secret.derive_blinding_factor(864559728, i as u32)?;
            assert_eq!(factor.to_owned(), key.as_hex());
        }
        Ok(())
    }

    #[test]
    fn test_secret_range() -> anyhow::Result<()> {
        let phrase =
            "half depart obvious quality work element tank gorilla view sugar picture humble";
        let deterministic_secret = DeterministicSecret::from_seed_words(phrase)?;

        let secrets = [
            "485875df74771877439ac06339e284c3acfcd9be7abf3bc20b516faeadfe77ae",
            "8f2b39e8e594a4056eb1e6dbb4b0c38ef13b1b2c751f64f810ec04ee35b77270",
            "bc628c79accd2364fd31511216a0fab62afd4a18ff77a20deded7b858c9860c8",
            "59284fd1650ea9fa17db2b3acf59ecd0f2d52ec3261dd4152785813ff27a33bf",
            "576c23393a8b31cc8da6688d9c9a96394ec74b40fdaf1f693a6bb84284334ea0",
        ];

        let blinding_factors = [
            "ad00d431add9c673e843d4c2bf9a778a5f402b985b8da2d5550bf39cda41d679",
            "967d5232515e10b81ff226ecf5a9e2e2aff92d66ebc3edf0987eb56357fd6248",
            "b20f47bb6ae083659f3aa986bfa0435c55c6d93f687d51a01f26862d9b9a4899",
            "fb5fca398eb0b1deb955a2988b5ac77d32956155f1c002a373535211a2dfdc29",
            "5f09bfbfe27c439a597719321e061e2e40aad4a36768bb2bcc3de547c9644bf9",
        ];

        let keyset_id = KeysetId::new("009a1f293253e41e")?;

        let range = deterministic_secret.derive_range(&keyset_id, 0, 5)?;

        for (i, (secret, blinding_factor)) in range.iter().enumerate() {
            assert_eq!(secrets[i], secret);
            assert_eq!(blinding_factors[i], &blinding_factor.as_hex());
        }
        Ok(())
    }
}
