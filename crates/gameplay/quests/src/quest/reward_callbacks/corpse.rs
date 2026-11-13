use serde::{Deserialize, Serialize};

/// A source corpse appearance is insufficient without its immutable minted species.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CorpseTarget {
    pub appearance: u32,
    pub maximum_age_ms: u32,
}

impl CorpseTarget {
    pub(super) fn valid(&self, callback: &super::RewardCallback) -> bool {
        self.appearance > 0
            && (1..=3_600_000).contains(&self.maximum_age_ms)
            && callback.creature_target.is_some()
            && callback.carried_item.is_some()
            && !callback.capture_creature
    }
}
