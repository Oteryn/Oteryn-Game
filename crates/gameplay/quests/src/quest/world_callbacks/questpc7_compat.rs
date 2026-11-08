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
