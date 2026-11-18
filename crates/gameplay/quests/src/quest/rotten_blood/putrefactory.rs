//! Crystal 201189f8, creaturescripts_putrefactory-x.lua:1-38,55-68,216.

use crate::quest::{
    NpcDefinitionPredecessor, QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue,
    QuestTrack, QuestTransition, world_callbacks::Position,
};

pub const CHARGE_TRACK: &str =
    "oteryn:quest-progress/crystalserver/quest/u13_20/rotten_blood/putrefactory_task";
pub const PASSIVE_CLOCK: &str = "oteryn:quest-progress/rotten-blood/putrefactory/last-passive-tick";
pub const PASSIVE_TICK: &str = "oteryn:quest-transition/rotten-blood/putrefactory/passive-tick";
pub const PASSIVE_PERIOD_SECONDS: u64 = 20;
// Crystal creaturescripts_putrefactory-x.lua:158-171,195-224.
pub const RESERVOIR_ACTION: u16 = 47953;
pub const RESERVOIR_EFFECT: u16 = 241;
pub(crate) const CANONICAL_QUEST: &str = "oteryn:quest.rotten_blood_quest";

/// Crystal uses an independent exhaustion counter for each position, keyed by
/// its one-based index in creaturescripts_putrefactory-x.lua:156-171,174-192.
pub(crate) fn reservoir_clock_key(position: Position) -> Option<String> {
    let index = [
        (34081, 31704, -14),
        (34013, 31744, -14),
        (34044, 31712, -14),
        (34051, 31706, -15),
        (34060, 31735, -15),
        (34095, 31684, -15),
    ]
    .iter()
    .position(|&(x, y, floor)| (position.x, position.y, position.floor) == (x, y, floor))?;
    Some(format!(
        "oteryn:quest-progress/rotten-blood/putrefactory/reservoir-{}-used-at",
        index + 1
    ))
}

/// Crystal creaturescripts_putrefactory-x.lua:41-51,140-153.
#[must_use]
pub fn accrues_charge_at(x: i32, y: i32, z: i16) -> bool {
    (34002..=34115).contains(&x) && (31658..=31770).contains(&y) && (14..=15).contains(&z)
}

/// Source icon is present in the Putrefactory, its lever room and vestibule.
/// The charge track can outlive the icon when the player leaves these rooms.
#[must_use]
pub fn shows_charge_icon(x: i32, y: i32, z: i16) -> bool {
    let putrefactory = accrues_charge_at(x, y, z);
    let lever_room = (32969..=32983).contains(&x) && (32328..=32338).contains(&y) && z == 15;
    let vestibule =
        (34095..=34105).contains(&x) && (31675..=31684).contains(&y) && (13..=14).contains(&z);
    putrefactory || lever_room || vestibule
}

pub(crate) fn passive_transition() -> QuestTransition {
    QuestTransition {
        world_change: None,
        key: PASSIVE_TICK.into(),
        quest: super::QUEST.into(),
        effects: vec![
            QuestEffect {
                track: CHARGE_TRACK.into(),
                // An absent -1 becomes zero in Lua. setWhiteCrossIcon then
                // returns without writing, so passive charge cannot seed it.
                from: QuestComparison::Between(0, 99),
                effect: QuestEffectKind::Add(1),
            },
            QuestEffect {
                track: PASSIVE_CLOCK.into(),
                from: QuestComparison::ElapsedAtLeast(PASSIVE_PERIOD_SECONDS as i64),
                effect: QuestEffectKind::SetNow,
            },
        ],
        completes: false,
        experience: None,
    }
}

pub(crate) fn extend(catalogue: &mut QuestStateCatalogue) -> Result<(), String> {
    if !crate::quest::in_runtime_scope(super::QUEST) {
        return Ok(());
    }
    let previous = catalogue
        .definition_hash(super::QUEST)
        .ok_or("missing Rotten Blood definition")?;
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    tracks.push(QuestTrack {
        key: PASSIVE_CLOCK.into(),
        quest: super::QUEST.into(),
        initial: 0,
        min: 0,
        max: i64::MAX,
    });
    transitions.push(passive_transition());
    let extended =
        QuestStateCatalogue::new(catalogue.content_revision.clone(), tracks, transitions)
            .map_err(|error| format!("{error:?}"))?;
    let current = extended
        .definition_hash(super::QUEST)
        .ok_or("missing updated Rotten Blood definition")?;
    catalogue.tracks = extended.tracks;
    catalogue.transitions = extended.transitions;
    catalogue.hashes.insert(super::QUEST.into(), current);
    // Retain the immediately preceding production definition for characters
    // whose quest is already underway. Existing tracks keep their meanings.
    catalogue
        .predecessors
        .entry(super::QUEST.into())
        .or_default()
        .push(NpcDefinitionPredecessor {
            hash: previous,
            defaults: Default::default(),
        });
    Ok(())
}
