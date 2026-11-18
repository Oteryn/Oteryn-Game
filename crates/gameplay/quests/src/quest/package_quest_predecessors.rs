//! Exact definition pins captured with the pre-package quest content at 871c1dec7.
const PREDECESSORS: [(&str, [u8; 32]); 3] = [
    (
        "oteryn:quest.dark_trails_quest",
        [
            0xd5, 0xaf, 0xbe, 0x8d, 0x80, 0xc2, 0xde, 0xea, 0x5f, 0x4b, 0x93, 0x2f, 0x58, 0xc9,
            0x9c, 0x7f, 0xa9, 0x18, 0x2e, 0xb7, 0x92, 0xaa, 0x49, 0x7f, 0xb1, 0x36, 0xa3, 0x6b,
            0xda, 0xb2, 0x19, 0x66,
        ],
    ),
    (
        "oteryn:quest.an_uneasy_alliance_quest",
        [
            0x5b, 0xd6, 0x91, 0x5a, 0x38, 0x24, 0xb4, 0x32, 0xe3, 0xe3, 0x12, 0xbb, 0xdc, 0xa7,
            0xd3, 0x01, 0x06, 0x57, 0x70, 0xee, 0x64, 0xcd, 0xa7, 0x8a, 0x1d, 0x1b, 0xc9, 0x25,
            0xb8, 0x87, 0x32, 0xf5,
        ],
    ),
    (
        "oteryn:quest.adventurers_guild",
        [
            0xae, 0x3b, 0x30, 0xa7, 0x5e, 0x15, 0xf4, 0x2f, 0xef, 0x07, 0x01, 0xe7, 0x44, 0xbb,
            0x27, 0xe9, 0x15, 0x96, 0xae, 0xc9, 0x2d, 0x79, 0x4c, 0x9b, 0x96, 0x92, 0x44, 0xd4,
            0x66, 0x8e, 0x7e, 0xae,
        ],
    ),
];

pub(super) fn register(catalogue: &mut super::QuestStateCatalogue) {
    for (quest, hash) in PREDECESSORS {
        catalogue
            .predecessors
            .entry(quest.into())
            .or_default()
            .push(super::NpcDefinitionPredecessor {
                hash,
                defaults: Default::default(),
            });
    }
}

#[cfg(test)]
#[test]
fn package_quest_saves_accept_only_exact_predecessor_pins() -> Result<(), String> {
    let state = super::loader::load_embedded_quest_state("content-1")
        .map_err(|error| format!("{error:?}"))?;
    let catalogue = state.catalogue();
    let dark_trails_before = super::dark_trails::pre_package_catalogue(catalogue)?;
    assert_eq!(
        dark_trails_before.definition_hash(super::dark_trails::QUEST),
        Some(PREDECESSORS[0].1)
    );
    for (quest, hash) in PREDECESSORS {
        assert_ne!(catalogue.definition_hash(quest).ok_or(quest)?, hash);
        assert_eq!(
            super::npc_rules::legacy_defaults(catalogue, quest, &hash),
            Some(&std::collections::BTreeMap::new()),
            "{quest}"
        );
        let mut unknown = hash;
        unknown[0] ^= 1;
        assert!(super::npc_rules::legacy_defaults(catalogue, quest, &unknown).is_none());
    }
    Ok(())
}
