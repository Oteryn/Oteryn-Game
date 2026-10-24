//! DEPOT-1 storage bounds. Town partitioning is the owner's 2026-10-23 override
//! of DEPOT-0's shared-town layout; channels never partition this custody.
//!
//! These are storage rules, not locker, session or Premium authority. No move
//! planner is exposed before DEPOT-CONTENT-1 / DEPOT-WIRE-1 supply that authority.

/// Tibia manual, World / Depots; DEPOT-0 §3 fixes the same compartment count.
pub const DEPOT_BOXES: u8 = 17;
/// https://github.com/opentibiabr/canary/blob/04b83b512114bfd888000d6e1433ed8ecaec7c5b/src/items/containers/depot/depotchest.cpp
/// `DepotChest::DepotChest`: the free-account admission limit.
pub const DEPOT_FREE_ENTRIES: u32 = 2_000;
/// Tibia manual, World / Depots; DEPOT-0 §3 database ceiling.
pub const DEPOT_HARD_ENTRIES: u32 = 15_000;
/// The same Canary constructor sets `maxSize = 32` and enables pagination.
pub const DEPOT_PAGE_ENTRIES: u32 = 32;
/// Canary `DepotChest::queryAdd`: a whole stack outside this depot adds one
/// entry regardless of its quantity. Contents remain BAGS-DEPOT-1's scope.
pub const DEPOT_WHOLE_ITEM_ENTRIES: u32 = 1;

#[cfg(test)]
#[path = "depot_tests.rs"]
mod tests;
