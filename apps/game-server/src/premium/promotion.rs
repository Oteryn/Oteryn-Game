//! PREM-2b pure promotion policy; not registered in the runtime.
//!
//! ACCEPT_BATCH (2026-10-15) releases the NPC source and accepts WHEEL-0 §6.2's
//! pre-delivery exception. Callers borrow the native PremiumStatus inside the gate
//! closure once per command/use; the exception retains its native PreDelivery witness.
//! No entitlement classification or reusable authority is constructed here.

use crate::domain::premium::{PremiumRefusal, PremiumStatus, require_promotion_premium};

/// PREMIUM_ACTIVATION §4.2: promotion starts at level 20.
pub(crate) const PROMOTION_MINIMUM_LEVEL: u32 = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromotionRefusal {
    Premium(PremiumRefusal),
    LevelTooLow,
    AlreadyPromoted,
}

/// Business eligibility only. Level/promotion inputs are unqualified values, not a locked
/// Character read or a session fence. A successful check neither spends coins nor grants a
/// durable promotion; the future fenced transaction must read and check its current facts.
pub(crate) fn check_promotion_eligibility(
    level: u32,
    durable_promoted: bool,
    premium: &PremiumStatus<'_>,
) -> Result<(), PromotionRefusal> {
    require_promotion_premium(premium).map_err(PromotionRefusal::Premium)?;
    if level < PROMOTION_MINIMUM_LEVEL {
        return Err(PromotionRefusal::LevelTooLow);
    }
    if durable_promoted {
        return Err(PromotionRefusal::AlreadyPromoted);
    }
    Ok(())
}

/// The promoted name/title projection chosen at login (§4.2 D73/D76). It is not a gameplay
/// permission: benefits re-read the native rule at each use, and a lapse never clears durable
/// promotion. Vocation/title selection and login composition stay with their existing owners.
#[must_use]
pub(crate) const fn effective_promotion_at_login(
    durable_promoted: bool,
    premium: &PremiumStatus<'_>,
) -> bool {
    durable_promoted && require_promotion_premium(premium).is_ok()
}
