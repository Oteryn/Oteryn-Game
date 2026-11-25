//! Authored Top of the City pursuit and Jimmy's sealed doorway.

pub const STAGE: &str = "oteryn:quest-progress/authored/top-of-the-city/stage";
pub const PURSUIT: &str = "oteryn:quest-progress/authored/top-of-the-city/pursuit";
pub const PURSUIT_COMPLETE: i64 = 10;
pub const JIMMY_DOOR_ACTION: u16 = 65534;
pub const JIMMY_DOOR: super::world_callbacks::Position = super::world_callbacks::Position {
    x: 32858,
    y: 31374,
    floor: -6,
};

const QUEST: &str = "oteryn:quest.authored.top_of_the_city_quest";
// Generated with the deployed authored binding from main before the pursuit.
const PUBLISHED_DEFINITION: [u8; 32] = [
    0x02, 0xb2, 0x92, 0x00, 0x8d, 0xae, 0xa0, 0x05, 0xcd, 0xe5, 0xb3, 0xc6, 0xe9, 0xe7, 0x6d, 0x7e,
    0xf1, 0x96, 0x3b, 0x1b, 0x0b, 0x5d, 0xee, 0x69, 0x33, 0x98, 0x51, 0x64, 0xa5, 0xf0, 0x52, 0x46,
];

pub(super) fn retain_predecessor(catalogue: &mut super::QuestStateCatalogue) {
    if catalogue.track(PURSUIT).is_some() {
        catalogue
            .predecessors
            .entry(QUEST.into())
            .or_default()
            .push(super::NpcDefinitionPredecessor {
                hash: PUBLISHED_DEFINITION,
                defaults: Default::default(),
            });
    }
}

#[cfg(test)]
#[test]
fn top_of_the_city_published_save_has_a_registered_predecessor() -> Result<(), String> {
    let loaded = super::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    assert_ne!(catalogue.definition_hash(QUEST), Some(PUBLISHED_DEFINITION));
    assert!(super::npc_rules::legacy_defaults(catalogue, QUEST, &PUBLISHED_DEFINITION).is_some());
    assert_eq!(catalogue.track(PURSUIT).ok_or("pursuit track")?.initial, 0);
    let mut unrelated = PUBLISHED_DEFINITION;
    unrelated[0] ^= 1;
    assert!(super::npc_rules::legacy_defaults(catalogue, QUEST, &unrelated).is_none());
    Ok(())
}
