//! D1230a/b GUI-qualified single Cyclopedia donation amount, in gold units.
//! This pure value does not debit gold or authorize a donation/map-area operation.
//! The separate aggregate bonus threshold is10,000,000gp (CIPSOFT S3), not this cap.

pub(crate) const MIN_DONATION_GP: i64 = 1;
pub(crate) const MAX_SINGLE_DONATION_GP: i64 = 10_000_000;

/// D1242a: each unresolved group blocks the entire runtime feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CyclopediaParityPending {
    PoiCount,
    FoundRadius,
    SpeedBonus,
    DonationEligibilityAndPremium,
    DrawOrder,
    FactorCompositionAndOfficialRandomChance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NotWiredInRuntime {
    pub(crate) parity_pending: [CyclopediaParityPending; 6],
}

/// Closed runtime port until separately qualified parity and integration exist.
/// Source-region admission and a valid amount cannot open this boundary.
pub(crate) const fn require_cyclopedia_runtime() -> Result<(), NotWiredInRuntime> {
    Err(NotWiredInRuntime {
        parity_pending: [
            CyclopediaParityPending::PoiCount,
            CyclopediaParityPending::FoundRadius,
            CyclopediaParityPending::SpeedBonus,
            CyclopediaParityPending::DonationEligibilityAndPremium,
            CyclopediaParityPending::DrawOrder,
            CyclopediaParityPending::FactorCompositionAndOfficialRandomChance,
        ],
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DonationAmountOutOfRange;

/// Validated single payment. No Default or unchecked/deserializing constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DonationAmount(u32);

impl DonationAmount {
    pub(crate) fn new(gold: i64) -> Result<Self, DonationAmountOutOfRange> {
        if !(MIN_DONATION_GP..=MAX_SINGLE_DONATION_GP).contains(&gold) {
            return Err(DonationAmountOutOfRange);
        }
        u32::try_from(gold)
            .map(Self)
            .map_err(|_| DonationAmountOutOfRange)
    }

    pub(crate) const fn get(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_single_payment_bounds_reject_negative_zero_and_above_cap() {
        for invalid in [i64::MIN, -1, 0, 10_000_001, i64::MAX] {
            assert_eq!(DonationAmount::new(invalid), Err(DonationAmountOutOfRange));
        }
        assert_eq!(DonationAmount::new(1).map(DonationAmount::get), Ok(1));
        assert_eq!(
            DonationAmount::new(10_000_000).map(DonationAmount::get),
            Ok(10_000_000)
        );
    }
}
