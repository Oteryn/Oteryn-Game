//! ITEM-MOVE-WIRE-0 §4.3: opening a corpse with `USE` and closing it again.
//!
//! Opening is a non-durable view action: it opens domain 11 for the corpse and writes nothing. It
//! is not a GAME-INTERACTION transition and has no occurrence of its own. Anyone within reach may
//! open a corpse; D133 and D134 are enforced when an item is taken (ITEM-MOVE-1), on the database
//! clock.
//!
//! **D133 disclosure (`PARITY_PENDING`).** During the D133 exclusivity window a non-owner in reach
//! can see a corpse's contents but cannot take them. Tibia refuses to open it; Oteryn does not gate
//! opening, because a gate on the runtime clock would disagree with the writer's database clock.

use oteryn_protocol_oteryn::world_spatial::ActorPosition;

pub use oteryn_gameplay_items::corpse_open::OpenDecision;
pub type UseItemTarget<C> = oteryn_gameplay_items::corpse_open::UseItemTarget<C, ActorPosition>;
pub type CloseTrigger<K> = oteryn_gameplay_items::corpse_open::CloseTrigger<K, ActorPosition>;

/// Reach (§4.3): same floor, Chebyshev distance at most 1.
pub fn within_reach(actor: ActorPosition, corpse: ActorPosition) -> bool {
    oteryn_gameplay_items::reach::within_reach(
        (actor.x, actor.y, i32::from(actor.floor)),
        (corpse.x, corpse.y, i32::from(corpse.floor)),
    )
}

pub fn decide_open<C>(actor: ActorPosition, target: &UseItemTarget<C>) -> OpenDecision {
    oteryn_gameplay_items::corpse_open::decide_open(actor, target, within_reach)
}

/// Whether `trigger` closes the corpse `open` at `position`.
pub fn closes<K: PartialEq>(open: &K, position: ActorPosition, trigger: &CloseTrigger<K>) -> bool {
    oteryn_gameplay_items::corpse_open::closes(open, position, trigger, within_reach)
}
