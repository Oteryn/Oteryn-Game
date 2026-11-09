//! Crystal 201189f movement_carrot.lua:9-32. A carried offering, not quest progress.
use super::world_callbacks::{Callback, ItemOnTarget, Position, Selector, Trigger};

pub const KEY: &str = "oteryn:quest-callback/dreamers-challenge-carrot-entry";
pub const ITEM: &str = "oteryn:item.tibia.i3595";
pub const PREMIUM_SPEECH: &str = "It seems that Hugo doesn't like free accounts";
pub const DESTINATION: Position = Position {
    x: 32861,
    y: 32235,
    floor: -9,
};

pub(crate) fn callback() -> Callback {
    let mut callback = super::source_world_objects::uneasy_crystal();
    callback.key = KEY.into();
    callback.quest = super::dreamer_fire_bug::QUEST.into();
    callback.trigger = Trigger::StepIn;
    callback.selector = Selector {
        native: None,
        position: None,
        unique: Some(2241),
        action: None,
        appearance: None,
    };
    callback.storage.clear();
    callback.effects = Default::default();
    callback.effects.teleport_effect = Some(11);
    callback.teleport = Some(DESTINATION);
    callback.requires_premium = true;
    callback.return_on_failure = true;
    callback.item_on_target = Some(ItemOnTarget {
        on_step: false,
        item: ITEM.into(),
        alternatives: vec![],
        count: 1,
        replacement: None,
        target_replacement: None,
        reusable: false,
        selected_instance: false,
        target_remove: false,
        ingredients: Vec::new(),
    });
    callback.sources=vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/movement_carrot.lua:9-32".into()];
    callback
}

pub fn shared_input(candidate: &Callback) -> bool {
    candidate.key == KEY && *candidate == callback()
}

pub fn shared_transition(transition: &super::QuestTransition) -> bool {
    transition.key == KEY
        && super::world_callbacks::primary_transition(&callback())
            .is_ok_and(|expected| expected == *transition)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn carrot_entrance_requires_a_real_carrot_and_live_premium_without_progress()
    -> Result<(), String> {
        let callback = callback();
        super::super::world_callbacks::validate_callbacks(std::slice::from_ref(&callback))?;
        assert!(!callback.has_personal_state());
        assert!(callback.requires_durable_commit());
        assert!(callback.requires_live_premium());
        assert!(callback.return_on_failure);
        assert_eq!(callback.selector.unique, Some(2241));
        assert_eq!(callback.teleport, Some(DESTINATION));
        let transition = super::super::world_callbacks::primary_transition(&callback)?;
        assert!(transition.effects.is_empty() && !transition.completes);
        super::super::QuestStateCatalogue::new("carrot", vec![], vec![transition])
            .map_err(|e| format!("{e:?}"))?;
        Ok(())
    }
    #[test]
    fn carrot_binding_cannot_drop_premium_or_change_the_offering() -> Result<(), String> {
        let source = callback();
        assert!(super::super::world_callbacks::load()?.contains(&source));
        for mutation in 0..4 {
            let mut bad = source.clone();
            match mutation {
                0 => bad.requires_premium = false,
                1 => bad.item_on_target.as_mut().ok_or("carrot")?.reusable = true,
                2 => bad.selector.unique = Some(2242),
                _ => bad.teleport.as_mut().ok_or("landing")?.x += 1,
            }
            assert!(super::super::world_callbacks::validate_callbacks(&[bad]).is_err());
        }
        Ok(())
    }
}
