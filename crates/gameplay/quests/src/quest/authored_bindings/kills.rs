//! Native counter bindings use the existing death queue and its fenced quest writer.
//! Threshold gate effects reuse questpc3 262618b3's original-snapshot operation.
use super::BoundQuest;
use crate::quest::reward_callbacks::OutfitReward;
use crate::quest::{QuestComparison, QuestEffect, QuestEffectKind};
use serde::{Deserialize, Serialize};

pub const MAX_KILL_BINDINGS: usize = 128;

/// Crystal201189f functions.lua:1064-1082 distinguishes party recipients from
/// direct Player damage-map entries. Summon-owner credit is an explicit opt-in.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DamageRecipients {
    AllDamagers,
    TopDamage,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DamageCredit {
    pub recipients: DamageRecipients,
    #[serde(default)]
    pub summon_owner: bool,
    pub recent_damage_ms: Option<u64>,
}

impl DamageCredit {
    pub fn admits_hit(self, at_ms: Option<u64>, death_ms: u64) -> bool {
        self.recent_damage_ms.is_none_or(|window| {
            at_ms
                .and_then(|hit| death_ms.checked_sub(hit))
                .is_some_and(|age| age <= window)
        })
    }
}

/// Inclusive victim bounds in native carrier coordinates (positive Tibia floor).
/// Eligibility is frozen from the original lethal facts, never the recipient's later position.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeathArea {
    pub minimum: [i32; 3],
    pub maximum: [i32; 3],
}
impl DeathArea {
    pub(crate) fn valid(self) -> bool {
        (0..3).all(|axis| {
            self.minimum[axis] >= 0
                && self.minimum[axis] <= self.maximum[axis]
                && self.maximum[axis] <= if axis == 2 { 15 } else { 65535 }
        })
    }
    pub fn contains(self, position: [i32; 3]) -> bool {
        (0..3).all(|axis| (self.minimum[axis]..=self.maximum[axis]).contains(&position[axis]))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KillCounter {
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub death_areas: Vec<DeathArea>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_credit: Option<DamageCredit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_outcome: Option<super::super::world_callbacks::OutcomeRequirement>,
    /// Store the credited defeat's Unix second for elapsed-time callback gates.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub record_defeat_time: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub set_to_maximum: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_change: Option<String>,
    /// Exact native encounter callback; absent for ordinary party hunts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encounter_source: Option<String>,
    /// Crystal 201189f8 arbaziloth_death.lua:29-41 grants the base outfit on
    /// the first credited defeat. Bind the outfit to a one-way journal write.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_defeat_outfit: Option<OutfitReward>,
}

impl KillCounter {
    pub(super) fn validate(&self, quest: &BoundQuest) -> Result<(), String> {
        let count = quest.tracks.iter().find(|track| track.key == self.track);
        let gate = quest.tracks.iter().find(|track| track.key == self.gate);
        if super::super::native_kills::counter(&self.key).is_some()
            || !super::super::valid_quest_key(&self.key)
            || !self.key.starts_with(super::super::native_kills::PREFIX)
            || !super::super::valid_quest_key(&self.creature)
            || !self.creature.starts_with("oteryn:creature.")
            || self.track == self.gate
                && (!self.set_to_maximum
                    || self.advance_gate_to.is_some()
                    || !(self.minimum..self.maximum).contains(&self.gate_value))
            || self.minimum >= self.maximum
            || self.first_defeat_outfit.as_ref().is_some_and(|reward| {
                self.track != self.gate
                    || !self.set_to_maximum
                    || self.damage_credit.is_none()
                    || reward.addons != 0
                    || reward.sex.is_some()
                    || reward.outfit.len() > 160
                    || !reward
                        .outfit
                        .strip_prefix("oteryn:outfit.")
                        .is_some_and(|key| {
                            !key.is_empty()
                                && key.bytes().all(|b| {
                                    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'
                                })
                        })
            })
            || self.required_outcome.as_ref().is_some_and(|required| {
                !required.valid()
                    || self.damage_credit.is_none()
                    || self.encounter_source.is_some()
                    || !quest.stages.iter().any(|stage| {
                        matches!(&stage.trigger, super::Trigger::MapUse { callback }
                        if callback.effects.encounter_outcome.as_ref().is_some_and(|source| {
                            source.can_credit(required, &self.creature)
                        }))
                    })
            })
            || self.record_defeat_time
                && (self.damage_credit.is_none()
                    || self.set_to_maximum
                    || self.advance_gate_to.is_some()
                    || self.minimum != 0
                    || self.maximum != i64::MAX)
            || self.death_area.is_some_and(|area| !area.valid())
            || self.death_areas.len() > 16
            || self.death_areas.iter().any(|area| !area.valid())
            || self.death_area.is_some() && !self.death_areas.is_empty()
            || self
                .damage_credit
                .is_some_and(|credit| credit.recent_damage_ms == Some(0))
            || self.encounter_source.is_some()
                && (self.damage_credit.is_some()
                    || self.set_to_maximum
                    || !self.death_areas.is_empty())
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
        if let Some(source) = &self.encounter_source {
            let encounter = quest
                .stages
                .iter()
                .find_map(|stage| match &stage.trigger {
                    super::Trigger::MapUse { callback } if callback.key == *source => {
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

    pub(super) fn effects(&self) -> Vec<QuestEffect> {
        if self.track == self.gate {
            // Crystal201189f the_new_frontier/creaturescripts_shard_of_corruption_kill.lua:5-10,
            // an_uneasy_alliance/creaturescripts_renegade_orc_kill.lua:3-8, and
            // the_thieves_guild_quest/creaturescripts_nomad_kill.lua:2-7 advance one exact stage.
            return vec![QuestEffect {
                track: self.track.clone(),
                from: QuestComparison::Eq(self.gate_value),
                effect: QuestEffectKind::Set(self.maximum),
            }];
        }
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
                        if self.set_to_maximum {
                            QuestEffectKind::Set(value)
                        } else {
                            QuestEffectKind::SetWhenAtLeast {
                                other_effect: 1,
                                minimum: self.maximum - 1,
                                value,
                            }
                        }
                    }),
            },
            QuestEffect {
                track: self.track.clone(),
                from: if self.record_defeat_time {
                    QuestComparison::ElapsedAtLeast(0)
                } else {
                    QuestComparison::Between(self.minimum, self.maximum - 1)
                },
                // Crystal201189f the_secret_library_quest/creaturescripts_kill.lua:10-39.
                effect: if self.set_to_maximum {
                    QuestEffectKind::Set(self.maximum)
                } else if self.record_defeat_time {
                    // Ferumbras creaturescripts_bosses_kill.lua:114-124.
                    QuestEffectKind::SetNow
                } else {
                    QuestEffectKind::Add(1)
                },
            },
        ]
    }

    pub(in crate::quest) fn counter<'a>(
        &'a self,
        quest: &'a str,
    ) -> super::super::native_kills::CounterRef<'a> {
        super::super::native_kills::CounterRef {
            transition: &self.key,
            creature: &self.creature,
            quest,
            track: &self.track,
            gate: &self.gate,
            gate_value: self.gate_value,
            minimum: self.minimum,
            maximum: self.maximum,
            death_area: self.death_area,
            death_areas: &self.death_areas,
            damage_credit: self.damage_credit,
            required_outcome: self.required_outcome.as_ref(),
            record_defeat_time: self.record_defeat_time,
            encounter_source: self.encounter_source.as_deref(),
            first_defeat_outfit: self.first_defeat_outfit.as_ref(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::{QuestRefusal, QuestStateCatalogue};
    use std::collections::BTreeMap;

    fn binding() -> Result<super::super::Bindings, String> {
        super::super::Bindings::parse(include_bytes!("fixtures/kill.json"))
    }

    #[test]
    fn threshold_uses_original_count_and_completes_only_at_goal() -> Result<(), String> {
        let bindings = binding()?;
        let base = QuestStateCatalogue::empty("kill-fixture").ok_or("revision")?;
        let catalogue = bindings.attach(&base)?;
        let counter = &bindings.quests[0].stages[1];
        let super::super::Trigger::Kill { counter } = &counter.trigger else {
            return Err("kill stage".into());
        };
        let transition = catalogue.transition(&counter.key).ok_or("transition")?;
        let mut state = BTreeMap::from([(counter.gate.clone(), 4), (counter.track.clone(), 0)]);
        for expected in 1..=3 {
            let changes = catalogue
                .evaluate(transition, &state, 0)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                catalogue.completes_after(transition, &changes),
                expected == 3
            );
            for change in changes {
                state.insert(change.track, change.after);
            }
            assert_eq!(state[&counter.track], expected);
            assert_eq!(state[&counter.gate], if expected == 3 { 5 } else { 4 });
        }
        assert_eq!(
            catalogue.evaluate(transition, &state, 0),
            Err(QuestRefusal::StageMismatch)
        );
        Ok(())
    }

    #[test]
    fn active_counter_is_owned_bounded_and_hash_binds_species() -> Result<(), String> {
        let mut bindings = binding()?;
        let base = QuestStateCatalogue::empty("kill-fixture").ok_or("revision")?;
        let catalogue = bindings.attach(&base)?;
        let super::super::Trigger::Kill { counter } = &bindings.quests[0].stages[1].trigger else {
            return Err("kill".into());
        };
        let state = BTreeMap::from([(counter.gate.clone(), 4)]);
        assert_eq!(
            crate::quest::native_kills::active_for(&catalogue, &state).as_slice(),
            std::slice::from_ref(&counter.key)
        );
        assert!(crate::quest::native_kills::active_for(&catalogue, &BTreeMap::new()).is_empty());
        let quest = bindings.quests[0].quest.clone();
        let before = catalogue.definition_hash(&quest);
        let super::super::Trigger::Kill { counter } = &mut bindings.quests[0].stages[1].trigger
        else {
            return Err("kill".into());
        };
        counter.creature = "oteryn:creature.dragon".into();
        assert_ne!(bindings.attach(&base)?.definition_hash(&quest), before);
        let super::super::Trigger::Kill { counter } = &mut bindings.quests[0].stages[1].trigger
        else {
            return Err("kill".into());
        };
        counter.track = "oteryn:quest-progress/foreign".into();
        assert!(bindings.validate().is_err());
        Ok(())
    }

    #[test]
    fn reserved_key_and_self_referential_effect_refuse() -> Result<(), String> {
        let mut bindings = binding()?;
        let super::super::Trigger::Kill { counter } = &mut bindings.quests[0].stages[1].trigger
        else {
            return Err("kill".into());
        };
        counter.key = crate::quest::native_kills::COUNTERS[0].transition.into();
        assert!(bindings.validate().is_err());
        let bindings = binding()?;
        let base = QuestStateCatalogue::empty("kill-fixture").ok_or("revision")?;
        let catalogue = bindings.attach(&base)?;
        let super::super::Trigger::Kill { counter } = &bindings.quests[0].stages[1].trigger else {
            return Err("kill".into());
        };
        let mut transition = catalogue
            .transition(&counter.key)
            .ok_or("transition")?
            .clone();
        transition.effects[0].effect = QuestEffectKind::SetWhenAtLeast {
            other_effect: 0,
            minimum: 0,
            value: 5,
        };
        assert_eq!(
            catalogue.evaluate(&transition, &BTreeMap::new(), 0),
            Err(QuestRefusal::NotSupported)
        );
        Ok(())
    }
    #[test]
    fn threshold_and_bound_set_have_distinct_definition_encoding_tags() {
        let mut threshold = Vec::new();
        QuestEffectKind::SetWhenAtLeast {
            other_effect: 0,
            minimum: 0,
            value: 0,
        }
        .encode(&mut threshold);
        let mut bound_set = Vec::new();
        QuestEffectKind::SetWithBinding {
            value: 0,
            binding: [0; 32],
        }
        .encode(&mut bound_set);
        assert_ne!(
            threshold[0], bound_set[0],
            "merged effect kinds must retain distinct encoding tags"
        );
    }
}
