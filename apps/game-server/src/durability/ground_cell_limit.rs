//! Detached Ground-root admission check; no custody writer or authority grant.
//!
//! ACCEPT-BATCH-2026-10-15 §§2–3 sets 64 items per tile and forbids periodic
//! ground cleanup between resets. This check has no clock, expiry or state.

use super::item_mint::GroundItemInstance;
use super::spell_items_abi::{SpellGroundTarget, decode_ground_cell};
use crate::foundation::{ChannelId, WorldId};

pub(crate) const GROUND_CELL_ROOT_LIMIT: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GroundCellLimitError {
    IncompatibleInput,
    NotEnoughRoom,
}

impl GroundCellLimitError {
    pub(crate) fn user_message(self) -> Option<&'static str> {
        match self {
            Self::NotEnoughRoom => Some("not enough room"),
            Self::IncompatibleInput => None,
        }
    }
}

/// Checks the physical Ground roots that would coexist after an admission.
///
/// `remaining` must contain the complete live native Ground rows at this tile
/// after the transaction's actual removals; `inserted` contains only actual new
/// roots after the native stack planner has resolved merges. A stack of any
/// quantity counts once. Contained items, static map decoration and the separate
/// per-channel dropped-item counter are not Ground roots supplied to this API.
///
/// All inputs are unqualified data. Matching them proves neither completeness,
/// freshness nor authority. The writer must independently resolve current scope
/// ownership/session fences, lock the tile/custody rows, read the complete live
/// snapshot, and check and insert atomically in the same physical transaction.
/// Never use a truncated projection, receipt or cache as an empty-tile fallback.
/// Recheck on retry/reload; this result reserves no room. Rebuild and removal-only
/// paths must preserve existing durable truth and do not use admission rejection
/// to retire or hide existing items. Inventory overflow must be resolved by its
/// existing writer before calling; this function never guesses a destination.
pub(crate) fn check_ground_cell_root_admission(
    world_id: WorldId,
    channel_id: ChannelId,
    ownership_generation: u64,
    target: &SpellGroundTarget,
    remaining: &[GroundItemInstance],
    inserted: &[GroundItemInstance],
) -> Result<(), GroundCellLimitError> {
    decode_ground_cell(&target.spatial_position)
        .map_err(|_| GroundCellLimitError::IncompatibleInput)?;

    // Check lengths without addition overflow, before inspecting unbounded input.
    if remaining.len() > GROUND_CELL_ROOT_LIMIT
        || inserted.len() > GROUND_CELL_ROOT_LIMIT - remaining.len()
    {
        return Err(GroundCellLimitError::NotEnoughRoom);
    }

    let mut seen = [[0; 16]; GROUND_CELL_ROOT_LIMIT];
    for (index, item) in remaining.iter().chain(inserted).enumerate() {
        if item.world_id != world_id
            || item.channel_id != channel_id
            || item.runtime_scope_ownership_generation != ownership_generation
            || item.ground.spatial_position != target.spatial_position
            || item.ground.map_revision != target.map_revision
            || item.ground.content_revision != target.content_revision
            || item.ground.native_room_placement_context != target.placement_context
            || seen[..index].contains(&item.item_instance_id)
        {
            return Err(GroundCellLimitError::IncompatibleInput);
        }
        seen[index] = item.item_instance_id;
    }
    Ok(())
}

#[cfg(test)]
#[path = "ground_cell_limit_tests.rs"]
mod tests;
