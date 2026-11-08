//! Preserve the lane's existing reward wire field alongside main's structured effects.
use super::Callback;
impl Callback {
    pub fn item_reward(&self) -> &Option<super::super::reward_callbacks::ItemReward> {
        if self.item_reward.is_some() {
            &self.item_reward
        } else {
            &self.effects.item_reward
        }
    }
}
impl From<super::effects::CallbackEffects> for super::effects::CallbackEffectSet {
    fn from(shared: super::effects::CallbackEffects) -> Self {
        Self::from_shared(shared)
    }
}

/// Keep the delegated, unqualified second-phase route as authoring evidence.
/// QENGINE2 owns Feaster and must supply phase-discriminated STEP admission
/// before this duplicate selector can enter the strict runtime registry.
pub(super) fn registered(callback: &Callback) -> bool {
    callback.key != "oteryn:quest-callback/questpc7-feaster-purification-entry"
}

/// The reused smith programme owns its native cache prop and guardian on this branch.
/// Retain that exact placement instead of registering the peer's old cache twice.
pub(super) fn retained_authored(callback: &Callback) -> bool {
    !matches!(
        callback.key.as_str(),
        "oteryn:quest-callback/authored-smith-cache"
            | "oteryn:quest-callback/authored-smith-cache-reward"
    )
}
