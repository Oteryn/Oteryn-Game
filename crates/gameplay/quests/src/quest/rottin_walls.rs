//! Crystal hammer.lua:13-20,64-101. Bind the six broken walls on the retained map.
use super::world_callbacks::{Callback, ItemOnTarget, NativeSelector, Position, Selector, Storage};

pub const PREFIX: &str = "oteryn:quest-callback/rottin-repair-";
pub const QUEST: &str = "oteryn:quest.rottin_wood_and_the_married_men_quest";

/// Admit only the published hammer bindings to the carried-tool route.
pub fn accepts(callback: &Callback) -> bool {
    callback.key.starts_with(PREFIX) && callbacks().contains(callback)
}

pub(crate) fn callbacks() -> [Callback; 6] {
    [
        (42501, 32647, 32216, -7, 2, 12183, 6474),
        (42502, 32660, 32213, -7, 2, 12183, 6474),
        (42503, 32644, 32183, -6, 2, 12185, 6473),
        (42504, 32660, 32201, -7, 2, 12184, 6473),
        (42505, 32652, 32200, -5, 2, 12185, 6473),
        (42506, 32679, 32210, -6, 4, 12185, 6473),
    ]
    .map(|(action, x, y, floor, ordinal, appearance, repaired)| {
        let position = Position { x, y, floor };
        let mut callback = super::source_world_objects::uneasy_crystal();
        callback.key = format!("{PREFIX}{action}");
        callback.quest = QUEST.into();
        callback.requires_world_admission = true;
        callback.selector = Selector {
            native: Some(NativeSelector { position, ordinal, appearance }),
            position: Some(position),
            action: Some(action),
            appearance: Some(appearance),
            position_aliases: Vec::new(),
            ordinal: None,
            unique: None,
        };
        callback.storage = vec![Storage {
            track: super::classic_talk::ROTTIN_WALLCOUNT.into(),
            minimum: 0,
            maximum: 5,
            set: 1,
            increment: true,
            preserve: false,
            at_least: false,
            read_only: false,
            set_if: None,
            add: None,
        }];
        callback.effects.transform = Some(repaired);
        callback.effects.reset_after_ms = Some(120_000);
        callback.item_on_target = Some(ItemOnTarget {
            item: "oteryn:item.tibia.i3460".into(),
            alternatives: Vec::new(),
            count: 1,
            replacement: None,
            target_replacement: None,
            target_remove: false,
            reusable: true,
            on_step: false,
            selected_instance: true,
            ingredients: Vec::new(),
        });
        callback.success_message = Some("You fixed this broken wall.".into());
        callback.failure_message = Some("You already fixed many broken walls today.".into());
        callback.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/tools/hammer.lua:13-20,64-101".into()];
        callback
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rottin_wall_bindings_are_registered_and_bounded() -> Result<(), String> {
        let expected = callbacks();
        super::super::world_callbacks::validate_callbacks(&expected)?;
        let registered = super::super::world_callbacks::load()?;
        assert!(
            expected
                .iter()
                .all(|callback| registered.contains(callback))
        );
        let loaded = super::super::loader::load_embedded_quest_state("rottin-wall-test")
            .map_err(|err| format!("{err:?}"))?;
        let catalogue = loaded.catalogue();
        assert_eq!(
            catalogue
                .track(super::super::classic_talk::ROTTIN_WALLCOUNT)
                .ok_or("wall counter")?
                .max,
            6
        );
        assert!(
            expected
                .iter()
                .all(|callback| catalogue.transition(&callback.key).is_some())
        );
        let reset = catalogue
            .transition("oteryn:quest-transition/classic/rottin-wood/start-disrepair")
            .ok_or("wall counter reset")?;
        let validate = |transition| {
            super::super::admission_binding::validate_resolved_world(
                catalogue,
                transition,
                false,
                &registered,
            )
        };
        validate(reset)?;
        let mut alias = reset.clone();
        alias.key.push_str("-alias");
        assert!(validate(&alias).is_err());
        Ok(())
    }
}
