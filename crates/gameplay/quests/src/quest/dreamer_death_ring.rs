//! Crystal 201189f: the shared Death Ring room and its UID9234 reset.
use super::world_callbacks::Position;
pub const RESET: &str = "oteryn:quest-callback/dreamers-challenge-death-ring-reset";
pub const TREES: [Position; 3] = [
    Position {
        x: 32857,
        y: 32231,
        floor: -11,
    },
    Position {
        x: 32857,
        y: 32232,
        floor: -11,
    },
    Position {
        x: 32857,
        y: 32233,
        floor: -11,
    },
];
pub const DESTINATION: Position = Position {
    x: 32819,
    y: 32347,
    floor: -9,
};

pub(crate) fn reset_callback() -> super::world_callbacks::Callback {
    use super::world_callbacks::{Passage, Selector, Trigger};
    let mut callback = super::source_world_objects::uneasy_crystal();
    callback.key = RESET.into();
    callback.quest = super::dreamer_fire_bug::QUEST.into();
    callback.trigger = Trigger::StepIn;
    callback.selector = Selector {
        native: None,
        position: None,
        unique: Some(9234),
        action: None,
        appearance: None,
    };
    callback.storage.clear();
    callback.effects = Default::default();
    callback.effects.teleport_effect = Some(11);
    callback.passages = TREES
        .into_iter()
        .map(|position| Passage {
            position,
            appearance: 3636,
            replacement: None,
            create_on_close: None,
        })
        .collect();
    callback.teleport = Some(DESTINATION);
    callback.sources=vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/dreamers_challenge_quest/movement_death_ring_teleport.lua:1-28".into()];
    callback
}

pub fn shared_reset(callback: &super::world_callbacks::Callback) -> bool {
    callback.key == RESET && *callback == reset_callback()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn death_ring_reset_is_a_real_registered_step_with_three_source_trees() -> Result<(), String> {
        let callbacks = super::super::world_callbacks::load()?;
        let callback = callbacks
            .iter()
            .find(|callback| callback.key == RESET)
            .ok_or("missing Death Ring reset caller")?;
        assert_eq!(callback.selector.unique, Some(9234));
        assert_eq!(
            callback.trigger,
            super::super::world_callbacks::Trigger::StepIn
        );
        assert_eq!(
            callback
                .passages
                .iter()
                .map(|p| p.position)
                .collect::<Vec<_>>(),
            TREES
        );
        assert!(!callback.requires_durable_commit());
        Ok(())
    }
}
