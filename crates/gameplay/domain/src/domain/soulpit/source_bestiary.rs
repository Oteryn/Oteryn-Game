//! Pinned Crystal selection metadata. These names never admit native Creature identities.
//!
//! The source MonsterType registry also contains types without a Bestiary race ID.
//! They remain valid conversion targets, while only the ascending, unique race-ID
//! map participates in category/difficulty draws. Textual class and numeric race
//! are separate source fields; neither is inferred from native content.
//!
//! Regenerate with tools/content-schema/monster-authoring/capture_soulpit_bestiary.py.
//! Full source hashes and equivalent first-wins collisions are retained in
//! docs/reference/content-prep/soulpit/source-bestiary-201189f.json.

#[path = "source_bestiary_catalogue.rs"]
mod catalogue;

pub use catalogue::SOURCE_BESTIARY_ROSTER_SHA256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceMonsterFacts {
    /// Case-insensitive key of Game.createMonsterType/MonsterType lookup.
    pub registration_name: &'static str,
    /// Final display name returned by MonsterType:name(), used for core lookup.
    pub name: &'static str,
    /// Setter presence matters: Some(0) is a real source map entry.
    pub race_id: Option<u16>,
    pub class: &'static str,
    /// Exact BestiaryType_t value, including NONE=0 independently of class text.
    pub race: u8,
    pub stars: u8,
}

/// Exact source registration lookup. No aliases, native keys, or identities are invented.
pub fn source_monster(name: &str) -> Option<&'static SourceMonsterFacts> {
    catalogue::MONSTERS
        .iter()
        .find(|monster| monster.registration_name.eq_ignore_ascii_case(name))
}

/// The complete source Bestiary map, in its ascending race-ID iteration order.
/// Missing core bindings remain in this iterator; admission happens after the draw.
pub fn source_bestiary_candidates()
-> impl ExactSizeIterator<Item = &'static SourceMonsterFacts> + DoubleEndedIterator + Clone {
    catalogue::BESTIARY_INDICES
        .iter()
        .map(|index| &catalogue::MONSTERS[*index])
}
