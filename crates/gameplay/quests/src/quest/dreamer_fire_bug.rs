//! Crystal 201189f: reusable Fire Bug ignition and the shared entrance lifecycle.
use super::world_callbacks::{Callback, ItemOnTarget, NativeSelector, Position, Selector};

// Crystal src/utils/utils_definitions.hpp:63,66 and actions_fire_bug.lua:18-27.
pub const IGNITION_EFFECT: u16 = 8;
pub const IGNITION_POSITIONS: [(u16, u16); 8] = [
    (32848, 32230),
    (32849, 32230),
    (32847, 32231),
    (32848, 32231),
    (32849, 32231),
    (32850, 32231),
    (32848, 32232),
    (32849, 32232),
];
pub const QUEST: &str = "oteryn:quest.dreamer_s_challenge_quest";
pub const IGNITE: &str = "oteryn:quest-callback/dreamers-challenge-fire-bug-ignite";
pub const SOURCE: Position = Position {
    x: 32849,
    y: 32233,
    floor: -9,
};
pub const DESTINATION: Position = Position {
    x: 32857,
    y: 32234,
    floor: -11,
};

pub(crate) fn callbacks() -> [Callback; 1] {
    let mut ignite = super::source_world_objects::uneasy_crystal();
    ignite.key = IGNITE.into();
    ignite.quest = QUEST.into();
    ignite.selector = Selector {
        position_aliases: Vec::new(),
        ordinal: None,
        native: Some(NativeSelector {
            position: SOURCE,
            ordinal: 1,
            appearance: 3134,
        }),
        position: Some(SOURCE),
        unique: Some(2243),
        action: None,
        appearance: Some(3134),
    };
    ignite.storage.clear();
    ignite.effects = Default::default();
    ignite.effects.transform = Some(1949);
    ignite.effects.reset_after_ms = Some(300_000);
    ignite.item_on_target = Some(ItemOnTarget {
        on_step: false,
        item: "oteryn:item.tibia.i5467".into(),
        alternatives: vec![],
        count: 1,
        replacement: None,
        target_replacement: None,
        reusable: true,
        selected_instance: false,
        target_remove: false,
        ingredients: Vec::new(),
    });
    ignite.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/others/actions_fire_bug.lua:99-109,172-173".into()];
    [ignite]
}

pub fn shared_input(callback: &Callback) -> bool {
    callback.key == IGNITE && callbacks().first() == Some(callback)
}

/// The retained-item owner admits only this exact empty-progress receipt.
pub fn shared_transition(transition: &super::QuestTransition) -> bool {
    transition.key == IGNITE
        && super::world_callbacks::primary_transition(&callbacks()[0])
            .is_ok_and(|expected| expected == *transition)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dreamer_shared_fire_bug_has_no_personal_progress() -> Result<(), String> {
        let callbacks = callbacks();
        super::super::world_callbacks::validate_callbacks(&callbacks)?;
        assert!(
            callbacks
                .iter()
                .all(|callback| !callback.has_personal_state())
        );
        let transition = super::super::world_callbacks::primary_transition(&callbacks[0])?;
        assert!(transition.effects.is_empty());
        super::super::QuestStateCatalogue::new("dreamer-fire-bug", vec![], vec![transition])
            .map_err(|error| format!("{error:?}"))?;
        Ok(())
    }
    #[test]
    fn dreamer_fire_bug_bindings_are_closed_and_registered() -> Result<(), String> {
        let authored = callbacks();
        let loaded = super::super::world_callbacks::load()?;
        assert!(authored.iter().all(|callback| loaded.contains(callback)));
        assert!(authored[0].requires_durable_commit());
        for mutation in 0..4 {
            let mut bad = authored[0].clone();
            match mutation {
                0 => bad.selector.native.as_mut().ok_or("native source")?.ordinal += 1,
                1 => bad.item_on_target.as_mut().ok_or("tool")?.reusable = false,
                2 => bad.effects.reset_after_ms = Some(1),
                _ => bad.storage.push(super::super::world_callbacks::Storage {
        set_if: None,
        add: None,
        read_only: false,
                    track: "oteryn:quest-progress/fixture".into(),
                    minimum: 0,
                    maximum: 0,
                    set: 1,
                    preserve: false,
                    increment: false,
                    at_least: false,
                }),
            }
            assert!(super::super::world_callbacks::validate_callbacks(&[bad]).is_err());
        }
        let mut forged = super::super::world_callbacks::primary_transition(&authored[0])?;
        forged.key = "oteryn:quest-callback/fixture-empty".into();
        assert!(super::super::QuestStateCatalogue::new("fixture", vec![], vec![forged]).is_err());
        Ok(())
    }
}
