//! Exact inputs for contribution callbacks published by the quest content owner.
use super::{QuestTransition, reward_callbacks::RewardCallback, world_callbacks};

pub const AWASH: &str = "oteryn:quest.awash_world_change";
pub const STEAMSHIP: &str = "oteryn:quest.steamship_world_change";
pub const SWAMP_FEVER: &str = "oteryn:quest.swamp_fever_world_change";

const COAL: &str = "oteryn:item.tibia.i12600";
const MEDICINE: &str = "oteryn:item.tibia.i12517";

pub fn ship_coal(callback: &RewardCallback) -> Option<u32> {
    let [coal] = callback.take.as_slice() else {
        return None;
    };
    (callback.quest == STEAMSHIP
        && callback.npc == "oteryn:npc.junkar_thais"
        && callback.carried_item.is_none()
        && callback.world_claim.is_none()
        && callback.give.is_none()
        && callback.fee.is_none()
        && callback.experience.is_none()
        && coal.item == COAL
        && coal.fluid_subtype.is_none()
        && coal.count > 0)
        .then_some(u32::from(coal.count))
}

pub fn medicine(callback: &RewardCallback) -> Option<u32> {
    let [medicine] = callback.take.as_slice() else {
        return None;
    };
    (callback.npc == "oteryn:npc.ottokar"
        && (callback.quest == SWAMP_FEVER
            || callback.key == "oteryn:quest-reward/l9-afflicted-pouch")
        && callback.carried_item.is_none()
        && callback.world_claim.is_none()
        && medicine.item == MEDICINE
        && medicine.fluid_subtype.is_none()
        && medicine.count == 1)
        .then_some(1)
}

pub fn pump_coal(callback: &world_callbacks::Callback) -> Option<u32> {
    let input = callback.item_on_target.as_ref()?;
    let selector = callback.selector.native.as_ref()?;
    let position = selector.position;
    // Crystal 201189f8 data-global/world/world.otbm and data/items/items.xml.
    let pump = (position.x, position.y, position.floor, selector.appearance);
    (callback.quest == AWASH
        && callback.requires_world_admission
        && callback.trigger == world_callbacks::Trigger::Use
        && callback.actor_kind == world_callbacks::ActorKind::Player
        && matches!(
            pump,
            (32558, 32007, -9, 12666) | (32560, 32014, -9, 12667) | (32567, 32024, -9, 12666)
        )
        && selector.ordinal == 1
        && callback.selector.position.is_none()
        && callback.selector.unique.is_none()
        && callback.selector.action.is_none()
        && callback.passages.is_empty()
        && input.item == COAL
        && input.selected_instance
        && !input.on_step
        && !input.reusable
        && !input.target_remove
        && input.ingredients.is_empty()
        && input.replacement.is_none()
        && input.alternatives.is_empty()
        && input.count == 1)
        .then_some(1)
}

pub(super) fn awash_transition(transition: &QuestTransition) -> bool {
    transition.quest == AWASH
        && transition.key.starts_with("oteryn:quest-callback/")
        && transition.experience.is_none()
}

pub(super) fn is_stateless_contribution(stage: &super::authored_bindings::Stage) -> bool {
    use super::authored_bindings::Trigger;
    !stage.completes
        && match &stage.trigger {
            Trigger::NpcDialogue {
                callback,
                alternatives,
            } => alternatives.is_empty() && is_stateless_npc_contribution(callback),
            Trigger::MapUse { callback } => pump_coal(callback).is_some(),
            _ => false,
        }
}

pub(super) fn is_stateless_npc_contribution(callback: &RewardCallback) -> bool {
    ship_coal(callback).is_some() || (callback.quest == SWAMP_FEVER && medicine(callback).is_some())
}
