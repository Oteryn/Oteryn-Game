use serde::{Deserialize, Serialize};

/// A source corpse appearance is insufficient without its immutable minted species.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CorpseTarget {
    pub appearance: u32,
    pub maximum_age_ms: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_creatures: Vec<String>,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub retain_tool: bool,
}

impl CorpseTarget {
    pub(super) fn valid(&self, callback: &super::RewardCallback) -> bool {
        self.appearance > 0
            && (1..=3_600_000).contains(&self.maximum_age_ms)
            && callback.creature_target.is_some()
            && callback.carried_item.is_some()
            && !callback.capture_creature
            && self.additional_creatures.len() <= 7
            && self
                .additional_creatures
                .iter()
                .enumerate()
                .all(|(index, creature)| {
                    creature.starts_with("oteryn:creature.")
                        && crate::quest::valid_quest_key(creature)
                        && !self.additional_creatures[..index].contains(creature)
                        && callback.creature_target.as_ref() != Some(creature)
                })
            && (!self.retain_tool
                || (callback.take.is_empty()
                    && callback.carried_target.is_none()
                    && callback.give.is_none()
                    && callback.carried_item.as_ref().is_some_and(|key| {
                        callback
                            .present
                            .iter()
                            .any(|input| &input.item == key && input.count == 1)
                    })))
    }
}
