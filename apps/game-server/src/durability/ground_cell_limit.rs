//! Ground-root admission: native same-transaction census plus the shared count policy.
//! No custody writer or authority grant.
//!
//! ACCEPT-BATCH-2026-10-15 §§2–3 sets 64 items per tile and forbids periodic
//! ground cleanup between resets. This check has no clock, expiry or state.

use super::character_progression::uuid_text;
use super::item_mint::GroundItemInstance;
use super::item_mint::{GroundPlacement, TypedDefinitionRef};
use super::spell_item_transaction::{
    SpellItemError, SpellItemScopeAuthority, assert_spell_item_scope_with_recovery,
    locked_tile_rows_for_scope,
};
use super::spell_items_abi::{SpellGroundTarget, decode_ground_cell};
use super::spell_owner_commit::SpellLanePermit;
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::foundation::{ChannelId, RuntimeScopeRefV1, WorldId};
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

impl GroundCellAdmissionError {
    pub(crate) fn into_spell_error(self) -> SpellItemError {
        match self {
            Self::Limit(error) => SpellItemError::Rejected(
                error
                    .user_message()
                    .unwrap_or("incompatible Ground admission"),
            ),
            Self::Source(error) => error,
        }
    }

    pub(crate) fn into_item_mint_error(self) -> super::item_mint::ItemMintError {
        use super::item_mint::ItemMintError;
        match self {
            Self::Limit(GroundCellLimitError::NotEnoughRoom) => ItemMintError::CapacityExceeded,
            Self::Limit(GroundCellLimitError::IncompatibleInput) => ItemMintError::InvalidInput,
            Self::Source(SpellItemError::Rejected("current timer scope/node authority")) => {
                ItemMintError::AuthorityRejected
            }
            Self::Source(error) => ItemMintError::Unavailable(source_error(error)),
        }
    }

    pub(crate) fn into_map_item_mint_error(self) -> super::map_item_mint::MapItemMintError {
        use super::map_item_mint::MapItemMintError;
        match self {
            Self::Limit(GroundCellLimitError::NotEnoughRoom) => MapItemMintError::CapacityExceeded,
            Self::Limit(GroundCellLimitError::IncompatibleInput) => MapItemMintError::InvalidInput,
            Self::Source(SpellItemError::Rejected("current timer scope/node authority")) => {
                MapItemMintError::AuthorityRejected
            }
            Self::Source(error) => MapItemMintError::Unavailable(source_error(error)),
        }
    }
}

fn source_error(error: SpellItemError) -> super::DurabilityError {
    match error {
        SpellItemError::Database(error) => error.into(),
        SpellItemError::Durability(error) => error,
        SpellItemError::Rejected(_) => super::DurabilityError::InvalidStoredState,
        // The scope issuer/census never emits training, but preserve a genuine
        // unavailable cause if a future native source extends that contract.
        SpellItemError::Training(
            super::character_progression::CharacterProgressionError::Unavailable(error),
        ) => error,
        SpellItemError::Training(_) => super::DurabilityError::InvalidStoredState,
    }
}

/// Admit exactly one new physical root for an existing native writer. Scope and
/// generation are expected DATA, independently checked by the existing issuer
/// against the real recovery record, current Node and assignment in this TX.
/// The caller retains its own source/session/content/proof checks and exact
/// inserts. No remaining/removal DTO, alternative authority or writer is added.
pub(crate) async fn check_ground_cell_root_insert_with_recovery(
    tx: &mut Transaction<'_, Postgres>,
    permit: &SpellLanePermit,
    recovery: &CharacterRecoveryFenceV1,
    node: &super::runtime_scope_assignment::NodeIncarnationProof,
    expected_scope: (RuntimeScopeRefV1, u64),
    ground: &GroundPlacement,
    item_instance_id: &[u8; 16],
) -> Result<(), GroundCellAdmissionError> {
    let authority = assert_spell_item_scope_with_recovery(
        tx,
        permit,
        recovery,
        node,
        expected_scope.0,
        expected_scope.1,
    )
    .await?;
    check_ground_cell_root_insert_in_transaction(tx, &authority, ground, item_instance_id).await
}

/// A genuine Player/Creature physical writer already holds its sealed scope
/// and native Channel lock. Read the actual post-removal census immediately
/// before its fresh root. Ground placement and identity are data, not grants.
pub(crate) async fn check_ground_cell_root_insert_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    ground: &GroundPlacement,
    item_instance_id: &[u8; 16],
) -> Result<(), GroundCellAdmissionError> {
    let target = SpellGroundTarget {
        spatial_position: ground.spatial_position.clone(),
        map_revision: ground.map_revision.clone(),
        content_revision: ground.content_revision.clone(),
        placement_context: ground.native_room_placement_context.clone(),
    };
    let remaining =
        check_ground_cell_root_admission_in_transaction(tx, authority, &target, &[]).await?;
    if remaining.len() >= GROUND_CELL_ROOT_LIMIT {
        return Err(GroundCellAdmissionError::Limit(
            GroundCellLimitError::NotEnoughRoom,
        ));
    }
    if remaining
        .iter()
        .any(|item| &item.item_instance_id == item_instance_id)
    {
        return Err(GroundCellAdmissionError::Limit(
            GroundCellLimitError::IncompatibleInput,
        ));
    }
    Ok(())
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
/// Returns only the actual native census DATA used by the check, for immediate
/// same-transaction insertion policy composition. It grants no authority or room.
pub(crate) async fn check_ground_cell_root_admission_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    target: &SpellGroundTarget,
    inserted: &[GroundItemInstance],
) -> Result<Vec<GroundItemInstance>, GroundCellAdmissionError> {
    decode_ground_cell(&target.spatial_position)
        .map_err(|_| GroundCellAdmissionError::Limit(GroundCellLimitError::IncompatibleInput))?;
    let rows = locked_tile_rows_for_scope(tx, authority, target).await?;
    let world = WorldId::decode(&authority.world_bytes())
        .map_err(|_| GroundCellAdmissionError::Limit(GroundCellLimitError::IncompatibleInput))?;
    let channel = ChannelId::decode(&authority.channel_bytes())
        .map_err(|_| GroundCellAdmissionError::Limit(GroundCellLimitError::IncompatibleInput))?;
    let remaining = native_roots(rows, authority, world, channel)?;
    check_ground_cell_root_admission(
        world,
        channel,
        authority.generation(),
        target,
        &remaining,
        inserted,
    )
    .map_err(GroundCellAdmissionError::Limit)?;
    Ok(remaining)
}

fn native_roots(
    rows: Vec<sqlx::postgres::PgRow>,
    authority: &SpellItemScopeAuthority,
    world: WorldId,
    channel: ChannelId,
) -> Result<Vec<GroundItemInstance>, SpellItemError> {
    rows.into_iter()
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
        .collect()
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
/// to retire or hide existing items. With no new roots, even overfull input incurs
/// no capacity charge; identity, scope and placement checks still apply.
/// Inventory overflow must be resolved by its
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

    // Only actual new roots incur a capacity charge. Check without addition
    // overflow; existing overfull truth still needs input validation below.
    if !inserted.is_empty()
        && (remaining.len() > GROUND_CELL_ROOT_LIMIT
            || inserted.len() > GROUND_CELL_ROOT_LIMIT - remaining.len())
    {
        return Err(GroundCellLimitError::NotEnoughRoom);
    }

    let mut seen = std::collections::BTreeSet::new();
    for item in remaining.iter().chain(inserted) {
        if item.world_id != world_id
            || item.channel_id != channel_id
            || item.runtime_scope_ownership_generation != ownership_generation
            || item.ground.spatial_position != target.spatial_position
            || item.ground.map_revision != target.map_revision
            || item.ground.content_revision != target.content_revision
            || item.ground.native_room_placement_context != target.placement_context
            || !seen.insert(item.item_instance_id)
        {
            return Err(GroundCellLimitError::IncompatibleInput);
        }
    }
    Ok(())
}
