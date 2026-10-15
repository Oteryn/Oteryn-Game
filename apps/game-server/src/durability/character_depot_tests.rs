#![allow(clippy::expect_used)]
use super::*;
use crate::domain::premium::PremiumStatus;

#[test]
fn depot_boxes_use_exact_accepted_bounds() {
    assert!(DepotBox::new(0).is_none());
    assert_eq!(DepotBox::new(1).expect("first").get(), 1);
    assert_eq!(DepotBox::new(17).expect("last").get(), 17);
    assert!(DepotBox::new(18).is_none());
    assert!(DepotBox::new(u8::MAX).is_none());
    assert_eq!(PAGE_ENTRIES, 32);
}

#[test]
fn depot_capacity_is_per_entry_and_native_unknown_premium_reads_free() {
    // Pure classification data, never a current fence/mutation authority fixture.
    let premium = PremiumStatus::Current.gate();
    for free in [
        PremiumGate::CLOSED,
        PremiumGate::default(),
        PremiumStatus::NotCurrent.gate(),
    ] {
        assert!(insertion_fits(1_999, free));
        assert!(!insertion_fits(2_000, free));
        assert!(!insertion_fits(15_000, free));
    }
    assert!(insertion_fits(14_999, premium));
    assert!(!insertion_fits(15_000, premium));
    assert!(!insertion_fits(u32::MAX, premium));
}

#[test]
fn depot_ordinals_never_wrap_or_renumber() {
    assert_eq!(next_ordinal(0).map(NonZeroU64::get), Some(1));
    assert_eq!(next_ordinal(90).map(NonZeroU64::get), Some(91));
    assert_eq!(
        next_ordinal(u64::MAX - 1).map(NonZeroU64::get),
        Some(u64::MAX)
    );
    assert!(next_ordinal(u64::MAX).is_none());
}
