use serde::{Deserialize, Serialize};

/// A source corpse appearance is insufficient without its immutable minted species.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CorpseTarget {
    pub appearance: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_age_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub additional_creatures: Vec<String>,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub retain_tool: bool,
}

impl CorpseTarget {
    pub(super) fn valid(&self, callback: &super::RewardCallback) -> bool {
        self.appearance > 0
            && callback.creature_target.is_some()
            && !callback.capture_creature
            && !callback.fresh_corpse
            && (if callback.carried_item.is_some() {
                // Crystal 201189f8 actions_mission10_a_message_of_freedom_sceptre.lua:20–32
                // limits the player's reuse of the sceptre independently of corpse age.
                self.maximum_age_ms
                    .map_or(self.retain_tool, |age| (1..=3_600_000).contains(&age))
            } else {
                // Crystal 201189f8 actions_mission03_the_keeper.lua:19–24
                // uses the original corpse directly, with no extra freshness window.
                self.maximum_age_ms.is_none()
                    && !self.retain_tool
                    && callback.npc.is_empty()
                    && callback.keywords.is_empty()
                    && callback.topic == 0
                    && callback.next_topic == 0
                    && callback.reply.is_empty()
                    && callback.take.is_empty()
                    && callback.give.is_some()
                    && callback.fee.is_none()
                    && callback.world_claim.is_none()
                    && callback.placement.is_none()
                    && callback.participants.is_empty()
                    && callback.storage.iter().any(|state| {
                        !state.preserve && !(state.minimum..=state.maximum).contains(&state.set)
                    })
            })
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
