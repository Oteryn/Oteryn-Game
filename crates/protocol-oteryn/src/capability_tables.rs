//! Registered capability command/domain ownership and superseded core domains.

use crate::world_map::CAPABILITY_WORLD_MAP_VIEW_V1;
use crate::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY;

/// `ITEM_LOOK_V1`, registry capability 28.
pub const CAPABILITY_ITEM_LOOK_V1: u32 = 28;
/// `ITEM_LOOK_INTENT`, registry command type 30, gated by capability 28.
pub const COMMAND_TYPE_ITEM_LOOK_INTENT: u32 = 30;
pub(crate) const CAPABILITY_MAP_STATE_V1: u32 = 29;
pub(crate) const CAPABILITY_WORLD_SPATIAL_FIELDS: u32 = 30;

// Whole-command/domain gates and offer sets are generated in ascending capability order.
// Capability 6 extends core domain 1; its codecs gate the payload rather than the whole domain.
include!(concat!(env!("OUT_DIR"), "/capability_tables.rs"));

/// Core domains a selected capability replaces, as (capability, domains). MAP-WIRE-1 contract §3
/// "One object domain per World" (`MAP_WIRE_1_WORLD_MAP_VIEW_CANDIDATE_V1.md`, accepted) with
/// ARCH-MAP-WIRE-1 §1.5: a World booted from a bundle serves domain 17 and does not serve
/// `WORLD_OBJECT_OVERLAY` (domain 2); its doors, levers and other object states reach the client
/// as tile changes of domain 17. Capability 18 is selected exactly on a bundle World (only it
/// offers 18, and it requires 18), so a session that selected 18 is never sent domain 2 on join,
/// resume or delta, and a client that selected 18 refuses domain 2. A fixture World never offers
/// 18 and keeps domain 2.
pub const SUPERSEDED: &[(u32, &[u32])] = &[(
    CAPABILITY_WORLD_MAP_VIEW_V1,
    &[STATE_DOMAIN_WORLD_OBJECT_OVERLAY],
)];

#[cfg(test)]
#[path = "capability_tables_tests.rs"]
mod tests;
