//! Crystal201189f rottenEssences.lua:1-28: one carried essence -> one Player storage.
use super::*;

pub const ESSENCE_CAUSE: u32 = 4;

pub(in crate::durability) fn storage_key(definition: &TypedDefinitionRef) -> Option<&'static str> {
    if definition.family != "Item" || definition.revision_ref != "definition-r1" {
        return None;
    }
    oteryn_gameplay_quests::quest::rotten_blood::essence(&definition.production_key)
        .map(|source| source.track)
}
