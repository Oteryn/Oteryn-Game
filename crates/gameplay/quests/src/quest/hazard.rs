//! HAZARD: Primal qualification and selected level in the existing Character quest store.
//! The first twelve levels follow canary:data/libs/systems/hazard.lua:74-150.
use std::collections::BTreeMap;

use super::{
    QuestCatalogueError, QuestComparison as C, QuestEffect, QuestEffectKind as K,
    QuestStateCatalogue, QuestTrack, QuestTransition,
};

pub const QUEST: &str = "oteryn:hazard/gnomprona-gardens";
pub const CURRENT: &str = "oteryn:hazard/gnomprona-gardens/current-level";
pub const UNLOCKED: &str = "oteryn:hazard/gnomprona-gardens/max-level";
/// Provisional ceiling from the pinned references; not verified against Global.
/// Global supports levels above twelve; this slice does not invent their qualification range.
pub const MAX_LEVEL: u8 = 12;
pub const GNOMADNESS: &str = "oteryn:npc.gnomadness";
pub const PRIMAL_MENACE: &str = "oteryn:creature.the_primal_menace";
pub const MAGMA_BUBBLE_KILLED: &str =
    "oteryn:quest-progress/quest/u12_90/primal_ordeal/bosses/magma_bubble_killed";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HazardLevel {
    pub current: u8,
    pub unlocked: u8,
}

/// Small read API for the Gnomadness reward consumer. Missing rows start at level one;
/// malformed or inconsistent stored values fail closed.
#[must_use]
pub fn level(tracks: &BTreeMap<String, i64>) -> Option<HazardLevel> {
    let current = u8::try_from(tracks.get(CURRENT).copied().unwrap_or(1)).ok()?;
    let unlocked = u8::try_from(tracks.get(UNLOCKED).copied().unwrap_or(1)).ok()?;
    (current >= 1 && current <= unlocked && unlocked <= MAX_LEVEL)
        .then_some(HazardLevel { current, unlocked })
}

#[must_use]
pub fn select_transition(level: u8) -> String {
    format!("{QUEST}/select-{level}")
}

#[must_use]
pub fn advance_transition(level: u8) -> String {
    format!("{QUEST}/advance-{level}")
}

fn effect(track: &str, from: C, effect: K) -> QuestEffect {
    QuestEffect {
        track: track.into(),
        from,
        effect,
    }
}

/// Adds two bounded tracks and the named, receipt-backed transitions to the boot catalogue.
/// The death owner captures the selected level; an advance checks qualification under the
/// Character row lock. Changing selection while a captured death settles cannot lose credit.
pub fn extend(catalogue: &QuestStateCatalogue) -> Result<QuestStateCatalogue, QuestCatalogueError> {
    let tracks = [CURRENT, UNLOCKED].map(|key| QuestTrack {
        key: key.into(),
        quest: QUEST.into(),
        initial: 1,
        min: 1,
        max: i64::from(MAX_LEVEL),
    });
    let mut transitions = Vec::new();
    for level in 1..=MAX_LEVEL {
        let value = i64::from(level);
        transitions.push(QuestTransition {
            key: select_transition(level),
            quest: QUEST.into(),
            completes: false,
            experience: None,
            effects: vec![
                effect(CURRENT, C::Any, K::Set(value)),
                effect(UNLOCKED, C::Ge(value), K::Add(0)),
            ],
        });
        if level < MAX_LEVEL {
            transitions.push(QuestTransition {
                key: advance_transition(level),
                quest: QUEST.into(),
                completes: false,
                experience: None,
                effects: vec![
                    effect(CURRENT, C::Any, K::Add(0)),
                    effect(UNLOCKED, C::Eq(value), K::Add(1)),
                ],
            });
        }
    }
    catalogue.with_quest(tracks.to_vec(), transitions)
}

/// Native reference coordinates (`z` is the runtime's positive underground floor).
/// Magma Bubble's two rectangles are subtracted, including their boundaries.
#[must_use]
pub fn contains(x: i32, y: i32, z: i16) -> bool {
    (33502..=33796).contains(&x)
        && (32740..=32996).contains(&y)
        && (13..=15).contains(&z)
        && !(z == 15
            && (((33633..=33649).contains(&x) && (32915..=32928).contains(&y))
                || ((33630..=33672).contains(&x) && (32887..=32921).contains(&y))))
}

#[cfg(test)]
#[path = "hazard_tests.rs"]
mod tests;
