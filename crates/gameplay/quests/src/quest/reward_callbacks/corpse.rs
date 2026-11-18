use serde::{Deserialize, Serialize};

/// A source corpse appearance is insufficient without its immutable minted species.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CorpseTarget {
    pub appearance: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub minimum_age_ms: u32,
    pub maximum_age_ms: u32,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub reusable_tool: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speech: Option<String>,
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

impl CorpseTarget {
    pub(super) fn valid(&self, callback: &super::RewardCallback) -> bool {
        self.appearance > 0
            && (1..=3_600_000).contains(&self.maximum_age_ms)
            && self.minimum_age_ms < self.maximum_age_ms
            && callback.creature_target.is_some()
            && callback.carried_item.is_some()
            && !callback.capture_creature
            && !callback.fresh_corpse
            && self.speech.as_ref().is_none_or(|speech| {
                !speech.trim().is_empty()
                    && speech.chars().count() <= 255
                    && !speech.chars().any(char::is_control)
            })
            && (!self.reusable_tool
                || (callback.take.is_empty()
                    && callback.give.is_some()
                    && callback.per_occurrence_mint))
    }
}
