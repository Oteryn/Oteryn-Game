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
