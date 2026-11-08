//! A repeatable craft spends bounded material before advancing its reward epoch.
use super::Callback;

impl Callback {
    pub fn craft_reward_epoch(&self, track: &str) -> bool {
        if self.effects.item_reward_cycle.as_deref() != Some(track)
            || self.key == crate::quest::rotten_blood::SACRIFICE
            || self.native_rule.is_some()
            || (self.item_on_target.is_none() && self.offerings.is_empty())
            || self.offerings.iter().any(|cost| cost.count == 0)
        {
            return false;
        }
        let Some(reward) = &self.effects.item_reward else {
            return false;
        };
        if self.item_on_target.as_ref().is_some_and(|input| {
            input.reusable
                || input.count == 0
                || input.replacement.is_some()
                || input.ingredients.iter().any(|cost| cost.count == 0)
        }) {
            return false;
        }
        let returns_material = |item: &str| {
            self.item_on_target.as_ref().is_some_and(|input| {
                item == input.item
                    || input.alternatives.iter().any(|key| key == item)
                    || input.ingredients.iter().any(|cost| cost.item == item)
            }) || self.offerings.iter().any(|cost| cost.item == item)
        };
        if returns_material(&reward.item)
            || reward
                .contents
                .iter()
                .chain(&reward.additional)
                .any(|output| returns_material(&output.item))
        {
            return false;
        }
        self.storage.iter().any(|state| {
            state.track == track
                && state.increment
                && state.set == 1
                && !state.preserve
                && !state.at_least
                && state.minimum >= 0
        })
    }
}
