//! Compatibility with the Tinder Box approach credit removed by QCX87.
use crate::quest::{NpcDefinitionPredecessor, QuestStateCatalogue};
use std::collections::BTreeMap;

pub(super) fn retain_approach_predecessor(catalogue: &mut QuestStateCatalogue) {
    // Definition hash captured from the real callback at base 2c1e9158c.
    let predecessor = NpcDefinitionPredecessor {
        hash: [
            0x67, 0xa5, 0x3f, 0xc8, 0x46, 0x7f, 0x4a, 0x8f, 0x22, 0x9c, 0x96, 0x1c, 0x58, 0x81,
            0x0c, 0x82, 0x47, 0xe3, 0xfa, 0xa5, 0x36, 0x7f, 0x48, 0x62, 0x14, 0xe0, 0xfe, 0x08,
            0x5f, 0xa6, 0xfa, 0x37,
        ],
        defaults: BTreeMap::new(),
    };
    catalogue
        .predecessors
        .entry("oteryn:quest.tinder_box_quest_chyllfroest".into())
        .or_default()
        .push(predecessor);
}
