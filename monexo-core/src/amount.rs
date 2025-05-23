//! This module defines the `Amount` and `SplitAmount` structs, which are used for representing and splitting amounts.
//!
//! The `Amount` struct represents an amount in dollars, with a single `u64` field for the amount. The struct provides a `split` method that splits the amount into a `SplitAmount` struct.
//!
//! The `SplitAmount` struct represents a split amount, with a `Vec<u64>` field for the split amounts. The struct provides a `create_secrets` method that generates a vector of random strings for use as secrets in the split transaction. The struct also implements the `IntoIterator` trait, which allows it to be iterated over as a vector of `u64` values.
//!
//! Both the `Amount` and `SplitAmount` structs are serializable and deserializable using serde.

use serde::{Deserialize, Serialize};

// #[derive(Debug, Clone)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Amount(pub u64);

impl Amount {
    pub fn split(&self) -> SplitAmount {
        split_amount(self.0).into()
    }
}

impl From<u64> for Amount {
    fn from(amount: u64) -> Self {
        Self(amount)
    }
}

impl AsRef<u64> for Amount {
    fn as_ref(&self) -> &u64 {
        &self.0
    }
}

impl std::ops::Add for Amount {
    type Output = Amount;

    fn add(self, rhs: Amount) -> Self::Output {
        Amount(self.0.checked_add(rhs.0).expect("Addition error"))
    }
}

impl std::ops::AddAssign for Amount {
    fn add_assign(&mut self, rhs: Self) {
        self.0 = self.0.checked_add(rhs.0).expect("Addition error");
    }
}

impl std::ops::Sub for Amount {
    type Output = Amount;

    fn sub(self, rhs: Amount) -> Self::Output {
        Amount(self.0 - rhs.0)
    }
}

impl std::ops::SubAssign for Amount {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl std::ops::Mul for Amount {
    type Output = Self;

    fn mul(self, other: Self) -> Self::Output {
        Amount(self.0 * other.0)
    }
}

impl std::ops::Div for Amount {
    type Output = Self;

    fn div(self, other: Self) -> Self::Output {
        Amount(self.0 / other.0)
    }
}

#[derive(Debug, Clone)]
pub struct SplitAmount(Vec<u64>);

impl From<Vec<u64>> for SplitAmount {
    fn from(from: Vec<u64>) -> Self {
        Self(from)
    }
}

impl SplitAmount {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl IntoIterator for SplitAmount {
    type Item = u64;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// split a decimal amount into a vector of powers of 2
fn split_amount(amount: u64) -> Vec<u64> {
    format!("{amount:b}")
        .chars()
        .rev()
        .enumerate()
        .filter_map(|(i, c)| {
            if c == '1' {
                return Some(2_u64.pow(i as u32));
            }
            None
        })
        .collect::<Vec<u64>>()
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    #[test]
    fn test_split_amount() -> anyhow::Result<()> {
        let bits = super::split_amount(13);
        assert_eq!(bits, vec![1, 4, 8]);

        let bits = super::split_amount(63);
        assert_eq!(bits, vec![1, 2, 4, 8, 16, 32]);

        let bits = super::split_amount(64);
        assert_eq!(bits, vec![64]);
        Ok(())
    }
}
