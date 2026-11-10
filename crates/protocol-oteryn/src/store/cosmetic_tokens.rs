//! Character-bound Cosmetic Token balance and Store payment.
use super::StoreWireError;
pub const CAPABILITY: u32 = 479;
pub const BALANCE: u32 = 6187;
pub const PURCHASE: u32 = 6188;
pub const CURRENCY: &str = "COSMETIC_TOKEN";
/// Eligibility belongs to the server's catalogue category, never an offer title.
pub fn eligible(category: &str) -> bool {
    [
        "outfits",
        "mounts",
        "cosmetics.outfits",
        "cosmetics.mounts",
        "houses.decorations",
        "houses.furniture",
    ]
    .iter()
    .any(|prefix| {
        category == *prefix
            || category
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('.'))
    })
}
pub fn decode_balance(bytes: &[u8]) -> Result<u64, StoreWireError> {
    let value = u64::from_le_bytes(bytes.try_into().map_err(|_| StoreWireError::Malformed)?);
    if value > i64::MAX as u64 {
        return Err(StoreWireError::Malformed);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    #[test]
    fn cosmetic_token_balance_is_bounded_and_categories_are_exact() {
        use super::*;
        assert_eq!(decode_balance(&0_u64.to_le_bytes()), Ok(0));
        assert_eq!(
            decode_balance(&(i64::MAX as u64).to_le_bytes()),
            Ok(i64::MAX as u64)
        );
        assert!(decode_balance(&u64::MAX.to_le_bytes()).is_err());
        assert!(decode_balance(&[0; 7]).is_err());
        assert!(decode_balance(&[0; 9]).is_err());
        for category in [
            "cosmetics.outfits",
            "cosmetics.mounts",
            "houses.decorations",
            "houses.furniture.beds",
        ] {
            assert!(eligible(category));
        }
        for category in [
            "consumables.exercise-weapons",
            "cosmetics.outfits-extra",
            "houses.hireling-dresses",
            "useful-things.charms",
        ] {
            assert!(!eligible(category));
        }
    }
}
