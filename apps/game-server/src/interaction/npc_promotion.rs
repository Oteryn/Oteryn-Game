//! PREM-5 detached NPC policies. No NPC dispatch, money writer or promotion/blessing grant.
//!
//! A quote or successful Premium check is data, not a payment/authority permit. The future
//! owner must recheck current Character facts and Premium inside the actual command boundary,
//! and commit the coins-only GOLD-FEE-1b burn and promotion/provenance together, idempotently.

use crate::domain::premium::{PremiumGate, PremiumRefusal, PremiumStatus};
use crate::premium::promotion::{PromotionRefusal, check_promotion_eligibility};

/// ACCEPT_BATCH §3 and PREMIUM_ACTIVATION §4.2: one-time promotion, paid in coins.
pub(crate) const PROMOTION_FEE_GOLD_UNITS: u64 = 20_000;

/// Price policy only; funds, coin selection/change, burn ledger and atomicity are GOLD-FEE-1b
/// responsibilities. No bank fallback is selected or authorized by this coins-only quote.
pub(crate) fn promotion_coin_fee_quote(
    level: u32,
    durable_promoted: bool,
    premium: &PremiumStatus<'_>,
) -> Result<u64, PromotionRefusal> {
    check_promotion_eligibility(level, durable_promoted, premium)?;
    Ok(PROMOTION_FEE_GOLD_UNITS)
}

/// The three Premium blessing NPC purchases of PREMIUM_ACTIVATION §4.4. No content keys,
/// persistence bits or unsourced blessing prices are assigned here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PremiumBlessingService {
    WisdomOfSolitude,
    BloodOfTheMountain,
    HeartOfTheMountain,
}

/// Premium gates purchase only. Held blessings are not inspected, removed or consumed by this
/// policy; lapse does not take them away (§4.4). This does not grant a blessing or payment.
pub(crate) const fn check_premium_blessing_purchase(
    _service: PremiumBlessingService,
    premium: PremiumGate,
) -> Result<(), PremiumRefusal> {
    premium.require()
}
