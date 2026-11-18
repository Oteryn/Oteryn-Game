//! Crystal 201189f, creaturescripts_bosses_kill.lua:1-32.
//! Earned shrine access is a permanent completion proof. Admission cooldowns
//! belong to the encounter owner and do not write these tracks.
use super::{Comparison, Effect, Rule};
use crate::quest::{NpcDefinitionPredecessor, QuestStateCatalogue, QuestTrack, definition_hash};
use std::collections::BTreeMap;

const QUEST: &str = "oteryn:quest.forgotten_knowledge_quest";
const PREFIX: &str = "oteryn:quest-transition/forgotten-knowledge/shrine-credit/";
const REPLICA_PREFIX: &str = "oteryn:quest-transition/forgotten-knowledge/replica-credit/";
const REPLICAS: &[&str] = &["diamond", "golden"];
const MAX_REPLICA_CREDIT: i64 = i32::MAX as i64;

fn replica_rules() -> impl Iterator<Item = Rule> {
    // Crystal 201189f, creaturescripts_replica_servants.lua:1-33.
    REPLICAS.iter().map(|kind| Rule {
        creature: format!("oteryn:creature.{kind}_servant_replica"),
        transition: format!("{REPLICA_PREFIX}{kind}"),
        effects: vec![Effect {
            track: format!(
                "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/{kind}_servant_counter"
            ),
            default: 0,
            from: Comparison {
                kind: "lt".into(),
                value: MAX_REPLICA_CREDIT,
            },
            set: None,
            add: Some(1),
        }],
        room: None,
        top_damage_only: true,
        define_transition: true,
        // Another queued death may advance the same counter before this one commits.
        epoch_tracks: Some(Vec::new()),
        quest: QUEST.into(),
    })
}

const BOSSES: &[(&str, &str)] = &[
    ("lady_tenebris", "lady_tenebris"),
    ("lloyd", "lloyd"),
    ("melting_frozen_horror", "horror"),
    ("soul_of_dragonking_zyrtarch", "dragonking"),
    ("the_blazing_time_guardian", "time_guardian"),
    ("the_enraged_thorn_knight", "thorn_knight"),
    ("the_freezing_time_guardian", "time_guardian"),
    ("the_last_lore_keeper", "last_lore"),
    ("the_time_guardian", "time_guardian"),
];

pub(super) fn rules() -> impl Iterator<Item = Rule> {
    shrine_rules().chain(replica_rules())
}

fn shrine_rules() -> impl Iterator<Item = Rule> {
    BOSSES.iter().map(|(creature, storage)| Rule {
        creature: format!("oteryn:creature.{creature}"),
        transition: format!("{PREFIX}{creature}"),
        effects: vec![Effect {
            track: format!(
                "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/{storage}_killed"
            ),
            default: 0,
            from: Comparison {
                kind: "eq".into(),
                value: 0,
            },
            set: Some(1),
            add: None,
        }],
        room: None,
        top_damage_only: false,
        define_transition: true,
        epoch_tracks: None,
        quest: QUEST.into(),
    })
}

pub(in crate::quest) fn extend(active: &mut QuestStateCatalogue) -> Result<(), String> {
    let Some(hash) = active.definition_hash(QUEST) else {
        return Ok(());
    };
    for rule in shrine_rules() {
        if rule.effects.iter().any(|effect| {
            active.track(&effect.track).is_none_or(|track| {
                track.quest != QUEST || track.initial != 0 || track.min != 0 || track.max < 1
            })
        }) {
            return Err("Forgotten Knowledge shrine-credit track missing or incompatible".into());
        }
        let transition = rule.transition()?;
        if active
            .transitions
            .insert(transition.key.clone(), transition)
            .is_some()
        {
            return Err("duplicate Forgotten Knowledge shrine-credit transition".into());
        }
    }
    active.hashes.insert(
        QUEST.into(),
        definition_hash(QUEST, &active.tracks, &active.transitions),
    );
    active
        .predecessors
        .entry(QUEST.into())
        .or_default()
        .push(NpcDefinitionPredecessor {
            hash,
            defaults: BTreeMap::new(),
        });
    Ok(())
}

pub(in crate::quest) fn extend_replica_credit(
    active: &mut QuestStateCatalogue,
) -> Result<(), String> {
    let Some(hash) = active.definition_hash(QUEST) else {
        return Ok(());
    };
    for rule in replica_rules() {
        let effect = rule
            .effects
            .first()
            .ok_or("missing replica credit effect")?;
        let track = QuestTrack {
            key: effect.track.clone(),
            quest: QUEST.into(),
            initial: 0,
            min: 0,
            max: MAX_REPLICA_CREDIT,
        };
        if active
            .tracks
            .insert(track.key.clone(), track.clone())
            .is_some_and(|old| old != track)
        {
            return Err("conflicting Forgotten Knowledge replica counter".into());
        }
        let transition = rule.transition()?;
        if active
            .transitions
            .insert(transition.key.clone(), transition)
            .is_some()
        {
            return Err("duplicate Forgotten Knowledge replica credit transition".into());
        }
    }
    active.hashes.insert(
        QUEST.into(),
        definition_hash(QUEST, &active.tracks, &active.transitions),
    );
    active
        .predecessors
        .entry(QUEST.into())
        .or_default()
        .push(NpcDefinitionPredecessor {
            hash,
            defaults: BTreeMap::new(),
        });
    Ok(())
}

pub(super) fn owns(rule: &Rule) -> bool {
    rule.transition.starts_with(PREFIX) || rule.transition.starts_with(REPLICA_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::{QuestComparison, QuestEffectKind, loader::load_embedded_quest_state};

    #[test]
    fn fknow_replica_credit_registers_bounded_personal_counters() -> Result<(), String> {
        let loaded = load_embedded_quest_state("fknow-replica-credit")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        for flavor in REPLICAS {
            let key = format!("{REPLICA_PREFIX}{flavor}");
            let rule = super::super::rule(&key).ok_or("missing replica death rule")?;
            assert!(rule.top_damage_only);
            assert!(rule.active(&BTreeMap::new()));
            let transition = catalogue
                .transition(&key)
                .ok_or("missing replica transition")?;
            let [effect] = transition.effects.as_slice() else {
                return Err("replica death must update exactly one counter".into());
            };
            let track = catalogue
                .track(&effect.track)
                .ok_or("missing replica track")?;
            assert_eq!(
                (track.initial, track.min, track.max),
                (0, 0, MAX_REPLICA_CREDIT)
            );
            assert_eq!(effect.from, QuestComparison::Lt(MAX_REPLICA_CREDIT));
            assert_eq!(effect.effect, QuestEffectKind::Add(1));
        }
        Ok(())
    }
}
