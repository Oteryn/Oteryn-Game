//! General declared creature objectives. Counts, stage guards and threshold effects
//! execute through the existing native death queue and fenced Quest progression owner.
//! Crystal 201189f: functions.lua:1064-1083 and party.lua:129-139 select the
//! present most-damage player and, with shared experience active, their source party.
use super::authored_bindings::BoundQuest;
pub use super::authored_bindings::kills::DeathArea;
use super::{QuestComparison, QuestEffect, QuestEffectKind};
use serde::{Deserialize, Serialize};

/// Crystal callbacks choose their credit helper explicitly; no XP-share threshold is inferred.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CreditRule {
    #[default]
    MostDamageParty,
    MostDamage,
    LastHit,
    DamagingPlayers,
}
impl CreditRule {
    pub fn is_default(&self) -> bool {
        *self == Self::MostDamageParty
    }
}

/// An exact creature type and inclusive native area; never a global name search.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CreatureRemoval {
    pub creature: String,
    pub area: DeathArea,
}
impl CreatureRemoval {
    fn valid(&self) -> bool {
        self.creature.starts_with("oteryn:creature.")
            && self.creature.len() > "oteryn:creature.".len()
            && super::valid_quest_key(&self.creature)
            && self.area.valid()
    }
    pub fn matches(&self, creature: &str, position: [i32; 3]) -> bool {
        self.creature == creature && self.area.contains(position)
    }
}

/// Despawn intent attached to any quest stage, including NPC and world callbacks.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StageRemoval {
    pub track: String,
    pub value: i64,
    pub creatures: Vec<CreatureRemoval>,
}
impl StageRemoval {
    pub(crate) fn validate(&self, quest: &BoundQuest) -> Result<(), String> {
        if quest
            .tracks
            .iter()
            .find(|track| track.key == self.track)
            .is_none_or(|track| {
                self.value == track.initial
                    || !(track.minimum..=track.maximum).contains(&self.value)
            })
            || self.creatures.is_empty()
            || self.creatures.len() > 16
            || self.creatures.iter().any(|removal| !removal.valid())
        {
            return Err("quest creature removal stage/owner/bounds".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KillCounter {
    #[serde(default, skip_serializing_if = "CreditRule::is_default")]
    pub credit: CreditRule,
    pub key: String,
    pub creature: String,
    pub track: String,
    pub gate: String,
    pub gate_value: i64,
    #[serde(default)]
    pub minimum: i64,
    pub maximum: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advance_gate_to: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death_area: Option<DeathArea>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_change: Option<String>,
    /// Exact native encounter callback; absent for ordinary party hunts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encounter_source: Option<String>,
    /// Named creature projections retired only when this objective advances its stage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_on_advance: Vec<CreatureRemoval>,
}

impl KillCounter {
    pub(crate) fn validate(&self, quest: &BoundQuest) -> Result<(), String> {
        let count = quest.tracks.iter().find(|track| track.key == self.track);
        let gate = quest.tracks.iter().find(|track| track.key == self.gate);
        if super::native_kills::counter(&self.key).is_some()
            || !super::valid_quest_key(&self.key)
            || !self.key.starts_with(super::native_kills::PREFIX)
            || !super::valid_quest_key(&self.creature)
            || !self.creature.starts_with("oteryn:creature.")
            || self.track == self.gate
            || self.minimum >= self.maximum
            || self.death_area.is_some_and(|area| !area.valid())
            || count
                .is_none_or(|track| track.minimum > self.minimum || track.maximum < self.maximum)
            || gate.is_none_or(|track| {
                !(track.minimum..=track.maximum).contains(&self.gate_value)
                    || self
                        .advance_gate_to
                        .is_some_and(|value| !(track.minimum..=track.maximum).contains(&value))
            })
        {
            return Err("authored kill identity/owner/bounds".into());
        }
        if self.remove_on_advance.len() > 16
            || (!self.remove_on_advance.is_empty() && self.advance_gate_to.is_none())
            || self
                .remove_on_advance
                .iter()
                .any(|removal| !removal.valid())
        {
            return Err("creature removal declaration bounds".into());
        }
        if self.encounter_source.is_some() && !self.credit.is_default() {
            return Err("encounter counter has its own recipient owner".into());
        }
        if let Some(source) = &self.encounter_source {
            let encounter = quest
                .stages
                .iter()
                .find_map(|stage| match &stage.trigger {
                    super::authored_bindings::Trigger::MapUse { callback }
                        if callback.key == *source =>
                    {
                        callback.effects.encounter.as_ref()
                    }
                    _ => None,
                })
                .ok_or("authored encounter counter source")?;
            let defenders = encounter
                .monsters
                .iter()
                .filter(|monster| monster.creature == self.creature)
                .count();
            if self.minimum != 0
                || encounter.timing.is_some()
                || encounter.players.is_empty()
                || encounter.players.len() > 8
                || encounter.monsters.is_empty()
                || encounter.monsters.len() > 8
                || i64::try_from(defenders) != Ok(self.maximum)
            {
                return Err("authored encounter counter roster/bounds".into());
            }
        }
        Ok(())
    }

    pub(crate) fn effects(&self) -> Vec<QuestEffect> {
        if self.encounter_source.is_some() && self.advance_gate_to.is_none() {
            // The source/epoch owner checks the gate without rewriting its revision
            // for every death queued from the same admission.
            return vec![QuestEffect {
                track: self.track.clone(),
                from: QuestComparison::Between(self.minimum, self.maximum - 1),
                effect: QuestEffectKind::Add(1),
            }];
        }
        vec![
            QuestEffect {
                track: self.gate.clone(),
                from: QuestComparison::Eq(self.gate_value),
                effect: self
                    .advance_gate_to
                    .map_or(QuestEffectKind::Add(0), |value| {
                        QuestEffectKind::SetWhenAtLeast {
                            other_effect: 1,
                            minimum: self.maximum - 1,
                            value,
                        }
                    }),
            },
            QuestEffect {
                track: self.track.clone(),
                from: QuestComparison::Between(self.minimum, self.maximum - 1),
                effect: QuestEffectKind::Add(1),
            },
        ]
    }

    pub(in crate::quest) fn counter<'a>(
        &'a self,
        quest: &'a str,
    ) -> super::native_kills::CounterRef<'a> {
        super::native_kills::CounterRef {
            transition: &self.key,
            creature: &self.creature,
            quest,
            track: &self.track,
            gate: &self.gate,
            gate_value: self.gate_value,
            minimum: self.minimum,
            maximum: self.maximum,
            death_area: self.death_area,
            encounter_source: self.encounter_source.as_deref(),
            credit: self.credit,
        }
    }
}

/// One production proof binding, without changing the quest's source mission stages.
/// Crystal 201189f data-global/scripts/quests/the_rookie_guard/mission03_rational_request.lua:7-13:
/// `Mission03 == 1`, `counter < 5`, `RatKills = counter + 1` through onDeathForParty.
pub(crate) const ROOKIE_RATS: super::native_kills::Counter = super::native_kills::Counter {
    transition: "oteryn:quest-transition/native-kills/qkill-rookie-rats",
    creature: "oteryn:creature.rat",
    quest: "oteryn:quest.the_rookie_guard_quest",
    track: "oteryn:quest-progress/quest/u9_1/the_rookie_guard/rat_kills",
    gate: "oteryn:quest-progress/quest/u9_1/the_rookie_guard/mission03",
    maximum: 5,
    authored_stage: false,
    world_change: None,
};
