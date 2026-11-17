//! M03: chosen 25 Years recipe requires promotion and regular blessings.
//! Premium: content/quests/definitions/quests-00000-00099.json:1, record 89
//! (Pits of Inferno chosen recipe requirements).
//! Evidence: content/quests/definitions/quests-00200-00299.json:1 (record 67,
//! requirements); donor actions_time_machine.lua:26-35 supplies the world trigger,
//! not these owner-selected prerequisites.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CharacterRequirements {
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub premium: bool,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub promoted: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blessings: Vec<String>,
}

impl CharacterRequirements {
    pub(super) fn valid(&self) -> bool {
        (self.premium || self.promoted || !self.blessings.is_empty())
            && self.blessings.len() <= 8
            && self.blessings.iter().all(|key| {
                key.len() <= 128
                    && key.strip_prefix("oteryn:blessing.").is_some_and(|stem| {
                        !stem.is_empty()
                            && stem
                                .bytes()
                                .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
                    })
            })
            && self.blessings.iter().collect::<BTreeSet<_>>().len() == self.blessings.len()
    }
}

/// Closed chosen-entry profile; source provenance is not broadened to arbitrary URLs.
pub(super) fn entry_admitted(callback: &super::Callback) -> bool {
    use super::{ActorKind, Position, Trigger};
    let Some(requirements) = &callback.character_requirements else {
        return false;
    };
    let Some(native) = &callback.selector.native else {
        return false;
    };
    let (route, source, shape) = match callback.quest.as_str() {
        "oteryn:quest.25_years_of_tibia_quest" => {
            let Some(part) = (if callback.key == "oteryn:quest-callback/qm03/anniversary" {
                Some(5)
            } else {
                callback
                    .key
                    .strip_prefix("oteryn:quest-callback/qm03/anniversary-")
                    .and_then(|part| part.parse::<u16>().ok())
                    .and_then(|appearance| appearance.checked_sub(25091))
                    .filter(|part| *part < 9 && *part != 5)
            }) else {
                return false;
            };
            let regular = [
                "the_spiritual_shielding",
                "the_embrace_of_tibia",
                "the_fire_of_the_suns",
                "the_spark_of_the_phoenix",
                "the_wisdom_of_solitude",
            ];
            (
                "25_years_of_tibia_quest",
                "content/quests/definitions/quests-00200-00299.json#/records/67",
                callback.trigger == Trigger::Use
                    && callback.minimum_level == Some(20)
                    && requirements.promoted
                    && requirements.blessings.len() == 5
                    && regular.iter().all(|stem| {
                        requirements
                            .blessings
                            .contains(&format!("oteryn:blessing.{stem}"))
                    })
                    && native.position
                        == Position {
                            x: 32429 + part % 3,
                            y: 32164 + part / 3,
                            floor: -8,
                        }
                    && native.ordinal == 1
                    && native.appearance == 25091 + part
                    && callback.teleport
                        == Some(Position {
                            x: 33453,
                            y: 31030,
                            floor: -8,
                        }),
            )
        }
        "oteryn:quest.the_pits_of_inferno_quest" => (
            "the_pits_of_inferno_quest",
            "content/quests/definitions/quests-00000-00099.json#/records/89",
            callback.key == "oteryn:quest-callback/m03/pits-of-inferno-entry"
                && callback.trigger == Trigger::StepIn
                && callback.minimum_level == Some(80)
                && !requirements.promoted
                && requirements.blessings.is_empty()
                && native.position
                    == Position {
                        x: 32795,
                        y: 32335,
                        floor: -8,
                    }
                && native.ordinal == 0
                && native.appearance == 354
                && callback.teleport.is_none()
                && callback.sources.iter().any(|source| {
                    source == "https://www.tibiawiki.com.br/wiki/Pits_of_Inferno_Quest"
                }),
        ),
        _ => return false,
    };
    shape
        && requirements.premium
        && callback.requires_world_admission
        && callback.actor_kind == ActorKind::Player
        && callback.return_on_failure
        && callback.selector.position == Some(native.position)
        && callback.selector.appearance == Some(native.appearance)
        && callback.selector.unique.is_none()
        && callback.selector.action.is_none()
        && callback.lever.is_none()
        && !callback.reversible
        && !callback.opening_only
        && callback.passages.is_empty()
        && callback.offerings.is_empty()
        && callback.item_on_target.is_none()
        && callback.native_rule.is_none()
        && (callback.effects.is_empty()
            || (route == "25_years_of_tibia_quest"
                && callback.effects.iter().next().is_none()
                && *callback.effects
                    == super::CallbackEffects {
                        scripted_teleport: true,
                        ..Default::default()
                    }))
        && callback.operation_count() == 0
        && callback.experience.is_none()
        && callback.monk_reward.is_none()
        && callback.sources.iter().any(|entry| entry == source)
        && callback.storage.len() == 1
        && callback.storage.iter().all(|storage| {
            storage.track == format!("oteryn:quest-progress/chosen-source/{route}/s1")
                && storage.minimum == 0
                && storage.maximum == 1
                && storage.set == 1
                && storage.at_least
                && !storage.increment
                && !storage.preserve
        })
}

/// Chosen first-entry counters start at zero; legacy donor storage keeps -1.
pub(super) fn entry_initial(callback: &super::Callback, storage: &super::Storage) -> Option<i64> {
    (entry_admitted(callback) && callback.storage.first() == Some(storage)).then_some(0)
}
