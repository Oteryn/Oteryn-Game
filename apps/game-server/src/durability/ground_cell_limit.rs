//! Ground-root admission: native same-transaction census plus the shared count policy.
//! No custody writer or authority grant.
//!
//! ACCEPT-BATCH-2026-10-15 §§2–3 sets 64 items per tile and forbids periodic
//! ground cleanup between resets. This check has no clock, expiry or state.

use super::character_progression::uuid_text;
use super::item_mint::GroundItemInstance;
use super::item_mint::{GroundPlacement, TypedDefinitionRef};
use super::spell_item_transaction::{
    SpellItemError, SpellItemScopeAuthority, locked_tile_rows_for_scope,
};
use super::spell_items_abi::{SpellGroundTarget, decode_ground_cell};
use crate::foundation::{ChannelId, WorldId};
use sqlx::{Postgres, Row, Transaction};

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

#[derive(Debug)]
pub(crate) enum GroundCellAdmissionError {
    Limit(GroundCellLimitError),
    /// Preserve native authority, source-integrity and database failures. Unknown
    /// occupancy is never interpreted as an empty tile or as a capacity refusal.
    Source(SpellItemError),
}

impl From<SpellItemError> for GroundCellAdmissionError {
    fn from(value: SpellItemError) -> Self {
        Self::Source(value)
    }
}

/// Check new physical roots against the actual complete live Ground census.
/// The caller already holds the native scope authority in this physical transaction;
/// its native admission and Channel owner locks last until commit/rollback. Read after
/// actual removals, before the exact proposed insertions. No remaining/removal DTO is
/// accepted, and no query, lock, authority predicate or custody writer is duplicated.
///
/// `target` and `inserted` are DATA: matching an empty tile does not qualify its current
/// admitted map/content/frame. The eventual writer must independently qualify them,
/// validate current Character/item authority and perform exactly those insertions in
/// this transaction. Success is neither insertion permission nor a reservation that
/// survives commit. Every insertion route must integrate this check separately.
///
/// With no new roots, a merge/removal/rebuild incurs no capacity charge, including a
/// preexisting overfull tile. Native source/same-transaction checks still apply. This
/// function never removes/hides truth, runs cleanup, or guesses overflow/merge rules.
pub(crate) async fn check_ground_cell_root_admission_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    target: &SpellGroundTarget,
    inserted: &[GroundItemInstance],
) -> Result<(), GroundCellAdmissionError> {
    decode_ground_cell(&target.spatial_position)
        .map_err(|_| GroundCellAdmissionError::Limit(GroundCellLimitError::IncompatibleInput))?;
    let rows = locked_tile_rows_for_scope(tx, authority, target).await?;
    if inserted.is_empty() {
        return Ok(());
    }
    let world = WorldId::decode(&authority.world_bytes())
        .map_err(|_| GroundCellAdmissionError::Limit(GroundCellLimitError::IncompatibleInput))?;
    let channel = ChannelId::decode(&authority.channel_bytes())
        .map_err(|_| GroundCellAdmissionError::Limit(GroundCellLimitError::IncompatibleInput))?;
    let remaining = rows
        .into_iter()
        .map(|row| -> Result<_, SpellItemError> {
            let quantity = u32::try_from(row.try_get::<i64, _>("quantity")?)
                .map_err(|_| SpellItemError::Rejected("invalid live Ground quantity"))?;
            if quantity == 0 {
                return Err(SpellItemError::Rejected("empty live Ground root"));
            }
            Ok(GroundItemInstance {
                item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
                world_id: world,
                channel_id: channel,
                definition: TypedDefinitionRef {
                    family: row.try_get("definition_family")?,
                    production_key: row.try_get("definition_production_key")?,
                    revision_ref: row.try_get("definition_revision_ref")?,
                },
                quantity,
                runtime_scope_ownership_generation: authority.generation(),
                ground: GroundPlacement {
                    spatial_position: row.try_get("spatial_position")?,
                    corpse_ref: row.try_get("corpse_ref")?,
                    map_revision: row.try_get("map_revision")?,
                    content_revision: row.try_get("content_revision")?,
                    native_room_placement_context: row.try_get("native_room_placement_context")?,
                },
                minted_transaction_id: uuid_text(row.try_get("minted_transaction_id")?)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    check_ground_cell_root_admission(
        world,
        channel,
        authority.generation(),
        target,
        &remaining,
        inserted,
    )
    .map_err(GroundCellAdmissionError::Limit)
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
