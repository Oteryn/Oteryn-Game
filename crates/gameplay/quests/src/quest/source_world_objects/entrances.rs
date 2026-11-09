//! Crystal 201189f actions_fire_bug.lua:110-128, OTS_HYPOTHESIS_ONLY.
use super::*;
use crate::quest::world_callbacks::{ItemOnTarget, SourcePortal};

pub(crate) fn dreamer_entrance() -> Callback {
    let mut callback = uneasy_crystal();
    let position = Position {
        x: 32849,
        y: 32233,
        floor: -9,
    };
    callback.key = "oteryn:quest-callback/m44-dreamer-entrance".into();
    callback.quest = "oteryn:quest.dreamers_challenge_quest".into();
    callback.storage.clear();
    callback.requires_world_admission = true;
    callback.selector = Selector {
        native: Some(NativeSelector {
            position,
            ordinal: 1,
            appearance: 3134,
        }),
        position: Some(position),
        unique: Some(2243),
        action: None,
        appearance: Some(3134),
    };
    callback.item_on_target = Some(ItemOnTarget {
        item: "oteryn:item.tibia.i5467".into(),
        count: 1,
        alternatives: Vec::new(),
        replacement: None,
        target_replacement: None,
        reusable: true,
        selected_instance: false,
        target_remove: false,
        ingredients: Vec::new(),
    });
    callback.effects = CallbackEffectSet::default();
    callback.effects.source_portal = Some(SourcePortal {
        position,
        destination: Position {
            x: 32857,
            y: 32234,
            floor: -11,
        },
        appearance: 1949,
        lifetime_us: 300_000_000,
        restore_source: true,
        group: "oteryn:quest-source/dreamer-entrance".into(),
    });
    callback.sources=vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/others/actions_fire_bug.lua:110-128".into()];
    callback
}
