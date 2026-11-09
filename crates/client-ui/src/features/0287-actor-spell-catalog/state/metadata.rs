//! Reference professions do not grant casting eligibility; the server owns that decision.
include!(concat!(env!("OUT_DIR"), "/spell_metadata.rs"));
pub const VOCATIONS: [&str; 11] = [
    "none",
    "druid",
    "elder_druid",
    "sorcerer",
    "master_sorcerer",
    "knight",
    "elite_knight",
    "paladin",
    "royal_paladin",
    "monk",
    "exalted_monk",
];
#[must_use]
pub fn professions(
    entry: &oteryn_client_runtime::spell_catalog::SpellCatalogEntry,
) -> &'static [&'static str] {
    PROFESSIONS
        .iter()
        .find(|(name, words, _)| {
            name.eq_ignore_ascii_case(&entry.name) && words.eq_ignore_ascii_case(&entry.words)
        })
        .map_or(&[], |(_, _, vocations)| *vocations)
}
