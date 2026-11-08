//! A repeatable craft spends material before advancing its reward epoch.
use super::Callback;

impl Callback {
    pub fn craft_reward_epoch(&self, track: &str) -> bool {
        if self.effects.item_reward_cycle.as_deref() != Some(track) {
            return false;
        }
        let Some(input) = &self.item_on_target else {
            return false;
        };
        let Some(reward) = &self.effects.item_reward else {
            return false;
        };
        // Sacrifice's first source stage only proves possession of the tear.
        if self.key == crate::quest::rotten_blood::SACRIFICE
            || self.native_rule.is_some()
            || input.reusable
            || input.count == 0
            || input.replacement.is_some()
        {
            return false;
        }
        let returns_material =
            |item: &str| item == input.item || input.alternatives.iter().any(|key| key == item);
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
