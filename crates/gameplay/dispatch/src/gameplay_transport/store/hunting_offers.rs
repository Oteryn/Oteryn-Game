//! The Platform charges an immutable price per key. Only the currently purchasable
//! XP tier is projected as the XP Boost, and the durable owner rechecks it at debit.
use super::*;
use crate::durability::store::StoreHuntingOffer;

pub(super) const HIDDEN_XP_TIERS: usize = 5;

pub(super) fn xp_tier(key: &str) -> Option<i16> {
    match StoreHuntingOffer::from_key(key)? {
        StoreHuntingOffer::XpBoost(tier) => Some(tier),
        StoreHuntingOffer::PermanentPreySlot | StoreHuntingOffer::PreyWildcards => None,
    }
}

pub(super) fn is_offer(key: &str) -> bool {
    StoreHuntingOffer::from_key(key).is_some()
}

pub(super) fn visible(key: &str, tier: Option<i16>) -> bool {
    xp_tier(key).is_none_or(|offer_tier| tier == Some(offer_tier))
}

pub(super) async fn current_tier(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> Option<i16> {
    let fence = gameplay_fence(owner, admitted).await?;
    owner
        .root
        .next_store_xp_boost_tier(owner.character, owner.holder, fence)
        .await
        .ok()?
}
