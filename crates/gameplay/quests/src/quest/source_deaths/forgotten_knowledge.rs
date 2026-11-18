//! Crystal 201189f, creaturescripts_bosses_kill.lua:1-32.
//! Earned shrine access is a permanent completion proof. Admission cooldowns
//! belong to the encounter owner and do not write these tracks.
use super::{Comparison, Effect, Rule};
use crate::quest::{NpcDefinitionPredecessor, QuestStateCatalogue, definition_hash};
use std::collections::BTreeMap;

const QUEST: &str = "oteryn:quest.forgotten_knowledge_quest";
const PREFIX: &str = "oteryn:quest-transition/forgotten-knowledge/shrine-credit/";
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
    for rule in rules() {
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

pub(super) fn owns(rule: &Rule) -> bool {
    rule.transition.starts_with(PREFIX)
}
