//! Shared destination projection for the item-use input adapters.
use oteryn_client_runtime::world_interaction::Target;
pub fn destination(
    rune: bool,
    target: Option<Target>,
    position: oteryn_client_runtime::item_use::ActorPosition,
) -> Target {
    match target {
        Some(target @ Target::Creature(_)) => target,
        Some(target) if !rune => target,
        _ => Target::Position(position),
    }
}
