//! Crystal actions_power_burn.lua consumes its pedestal ring as a shared map effect.
use super::QuestTransition;
use super::world_callbacks::{Callback, Position, Trigger};

pub const KEY: &str = "oteryn:quest-callback/authored-outlaw-camp-power-burn";
const QUEST: &str = "oteryn:quest.the_outlaw_camp_quest";
pub const RING: &str = "oteryn:item.tibia.i3050";
pub const BARREL: &str = "oteryn:item.tibia.i2523";
pub const RING_POSITION: Position = Position {
    x: 32613,
    y: 32220,
    floor: -10,
};
pub const BARREL_POSITION: Position = Position {
    x: 32614,
    y: 32209,
    floor: -10,
};

pub fn shared_input(callback: &Callback) -> bool {
    callback.key == KEY
        && callback.quest == QUEST
        && callback.trigger == Trigger::Use
        && callback.lever == Some([2772, 2773])
        && callback.reversible
        && callback.selector.position
            == Some(Position {
                x: 32616,
                y: 32222,
                floor: -10,
            })
        && callback.selector.ordinal == Some(2)
        && callback.selector.appearance == Some(2772)
        && callback.world_change.is_none()
        && !callback.has_state()
        && !callback.completes
        && callback.effects.scene.is_some()
        && matches!(callback.offerings.as_slice(), [offering]
            if offering.position == RING_POSITION && offering.item == RING && offering.count == 1)
}

pub fn shared_transition(transition: &QuestTransition) -> bool {
    transition.key == KEY
        && transition.quest == QUEST
        && transition.effects.is_empty()
        && transition.world_change.is_none()
        && !transition.completes
        && transition.experience.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outlaw_camp_burn_loads_a_shared_ring_consumption_transition() -> Result<(), String> {
        let loaded = super::super::loader::load_embedded_quest_state("outlaw-camp")
            .map_err(|error| format!("{error:?}"))?;
        let catalogue = loaded.catalogue();
        let callbacks = super::super::world_callbacks::load_for(catalogue)?;
        let burn = callbacks
            .iter()
            .find(|callback| callback.key == KEY)
            .ok_or("burn lever")?;
        assert!(shared_input(burn));
        assert!(burn.requires_durable_commit());
        assert!(shared_transition(
            catalogue.transition(KEY).ok_or("burn transition")?
        ));
        let reward = callbacks
            .iter()
            .find(|callback| callback.key == "oteryn:quest-callback/authored-outlaw-camp-reward")
            .ok_or("reward chest")?;
        assert!(reward.completes);
        assert_eq!(
            reward.item_reward().ok_or("chest contents")?.contents.len(),
            2
        );
        Ok(())
    }
}
