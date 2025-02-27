pub mod amount;
pub mod blind;
pub mod dhke;
pub mod error;
pub mod fixture;
pub mod keyset;
pub mod primitives;
pub mod proof;
pub mod token;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
