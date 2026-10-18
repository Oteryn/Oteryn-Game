//! House scope entry rules (HOUSE-RUNTIME-0 §4.1-§4.3, SCOPE-HANDOFF-1).
//!
//! The durable scope and handoff live in `crate::durability::house_scope_handoff`; this module adds the pure entry checks that run
//! before a handoff is prepared. D1373 additionally owns the actual staged
//! House slot/materialization consumer below. It retains the caller's native
//! transaction and remains unbound until all allocated owner seams are present.

pub mod handoff;

pub use crate::durability::house_scope_handoff::{
    HOUSE_SCOPE_CHARACTERS_MAX, HouseAbortOutcome, HouseCommitOutcome, HouseEntryCommit,
    HouseEntryRefusal, HouseEntryRequest, HouseHandoffDirection, HouseHandoffError,
    HouseHandoffRecord, HouseHandoffState, HouseId, HousePrepareOutcome, HouseReconcileReport,
    HouseRecoveryReport, HouseReleaseOutcome,
};

/// HOUSERT0-RL-02: door transition latency target, p99, measured before activation
/// (HOUSE-RUNTIME-1); not a registered allocation bound.
pub const HOUSE_DOOR_TRANSITION_P99_MS: u64 = 500;

/// Actor-local facts read before a house entry is prepared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HouseEntryPrecheck {
    pub has_access: bool,
    pub house_open: bool,
    /// Fight, PZ or another combat lock.
    pub combat_locked: bool,
    /// Trade, an unresolved mutation or another pending operation.
    pub operation_pending: bool,
}

impl HouseEntryPrecheck {
    /// The first refusal in the §4.1 order, or `None` when the entry may be prepared.
    #[must_use]
    pub const fn evaluate(self) -> Option<HouseEntryRefusal> {
        if !self.has_access {
            Some(HouseEntryRefusal::NoAccess)
        } else if !self.house_open {
            Some(HouseEntryRefusal::HouseClosed)
        } else if self.combat_locked {
            Some(HouseEntryRefusal::InCombat)
        } else if self.operation_pending {
            Some(HouseEntryRefusal::Busy)
        } else {
            None
        }
    }
}

/// Leaving a house into a Channel scope, and the §4.3 fallback, await ADMIT-0.
pub const fn exit_into_channel() -> Result<(), HouseHandoffError> {
    Err(HouseHandoffError::ExitNotAdmitted)
}

use crate::content::{LogicalCell, QualifiedNativeEntryRoom, encode_spatial_position};
use crate::durability::fresh_admission::PreparedHouseScopeTransitionClaims;
use crate::durability::house_interior::{HOUSE_INTERIOR_ITEMS_MAX, HouseInteriorSnapshot};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionState, HouseActorAuthorityV1, HouseActorDirectory,
    HouseEntryPlacement, HousePlacementError, HouseSourceActorHold, MovementLocalPosition,
    RuntimeScopeRefV1,
};
use sqlx::{Postgres, Row, Transaction};
use std::collections::BTreeSet;

#[derive(Debug)]
pub(crate) enum HouseEntryStageError {
    NativeAuthorityMismatch,
    QualifiedHouseUnavailable,
    ContentBindingMismatch,
    InteriorLocationMismatch,
    Placement(HousePlacementError),
    Database(sqlx::Error),
}
impl From<sqlx::Error> for HouseEntryStageError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}
impl From<HousePlacementError> for HouseEntryStageError {
    fn from(error: HousePlacementError) -> Self {
        Self::Placement(error)
    }
}

/// Exact qualified source identity retained alongside the original loader
/// snapshot. Neither this value nor 0047 history grants active Content or ADMIT.
struct StagedHouseContentIdentity {
    house_revision: String,
    server_artifact_digest: [u8; 32],
    qualified_placement_digest: [u8; 32],
}

/// Actual native materialization preparation with both physical owners held.
/// There is deliberately no bind/commit method at this boundary: the current
/// ChannelSpellStates owner has not transferred its actual PlayerSpellState,
/// and the ordered FND/ingress/active-Content integration is still held by CP.
/// Dropping the stage releases the same House slot and exact source fence.
/// Borrowing the real SQL transaction prevents its caller from committing it
/// behind this preparation's back while the physical reservations remain live.
pub(crate) struct StagedHouseEntry<'owner, 'transaction, 'connection> {
    transaction: &'transaction mut Transaction<'connection, Postgres>,
    prepared: PreparedHouseScopeTransitionClaims,
    placement: HouseEntryPlacement<'owner>,
    source_hold: HouseSourceActorHold<'owner>,
    content: StagedHouseContentIdentity,
}

impl StagedHouseEntry<'_, '_, '_> {
    /// The actual #2163 loader result is retained by the owning stage. It is
    /// never reconstructed from expected HouseItem DTOs or an empty fallback.
    pub(crate) fn interior(&self) -> &HouseInteriorSnapshot {
        self.prepared.interior()
    }

    pub(crate) fn reserved_tile(&self) -> &[u8] {
        self.prepared.reserved_tile()
    }

    /// Revalidate the exact physical reservation and original native TX. This
    /// writes no receipt, session, lease, transport binding or generation.
    pub(crate) async fn verify_consumed_in_owning_transaction(
        &mut self,
    ) -> Result<(), HouseEntryStageError> {
        verify_stage_transaction(self.transaction, &self.prepared).await?;
        self.placement.verify_consumed(&self.prepared)?;
        Ok(())
    }
}

async fn verify_stage_transaction(
    tx: &mut Transaction<'_, Postgres>,
    prepared: &PreparedHouseScopeTransitionClaims,
) -> Result<(), HouseEntryStageError> {
    let actual: String = sqlx::query_scalar("SELECT txid_current()::text")
        .fetch_one(&mut **tx)
        .await?;
    if actual != prepared.transaction_identity() {
        return Err(HouseEntryStageError::NativeAuthorityMismatch);
    }
    Ok(())
}

fn validate_loaded_interior(
    interior: &HouseInteriorSnapshot,
    house: &HouseId,
    generation: u64,
    qualified_cells: &BTreeSet<Vec<u8>>,
) -> Result<(), HouseEntryStageError> {
    if &interior.house != house
        || interior.ownership_generation != generation
        || u64::try_from(interior.items.len())
            .ok()
            .is_none_or(|count| count > HOUSE_INTERIOR_ITEMS_MAX)
        || interior
            .items
            .iter()
            .any(|item| !qualified_cells.contains(&item.spatial_position))
    {
        return Err(HouseEntryStageError::InteriorLocationMismatch);
    }
    Ok(())
}

/// Consume the actual native preparation in its caller-owned SQL transaction.
/// `destination` is only a proposal: its canonical bytes must equal the locked
/// PREPARED tile, its real qualified House cells must match the immutable 0047
/// binding, and the source actor/lease/pin are read from their current owner.
/// The accepted loader must have succeeded before this function can be called.
///
/// This is a real physical stage, not current-content/ADMIT activation. The
/// native full-bundle House catalogue is Unknown today and therefore refuses;
/// an explicitly qualified source can be used for isolated qualification after
/// #2167 and the narrow shared read accessors land. No entry-catalogue fallback
/// is substituted for Unknown full-bundle metadata.
pub(crate) async fn stage_house_entry_in_transaction<'owner, 'transaction, 'connection>(
    tx: &'transaction mut Transaction<'connection, Postgres>,
    prepared: PreparedHouseScopeTransitionClaims,
    room: &QualifiedNativeEntryRoom,
    source_runtime: &'owner mut ChannelRuntimeV1,
    source_actor: ExactActorRef,
    destination: MovementLocalPosition,
    house_actors: &'owner mut HouseActorDirectory,
) -> Result<StagedHouseEntry<'owner, 'transaction, 'connection>, HouseEntryStageError> {
    verify_stage_transaction(tx, &prepared).await?;
    let current = prepared.source_authority();
    let binding = source_runtime.binding();
    let pinned = source_runtime.content_pin();
    if current.session_state() != GameSessionState::Active
        || current.current_transport().is_none()
        || current.current_control_loss_epoch().is_some()
        || current.current_game_session_id() != prepared.source_session()
        || current.current_character_lease() != prepared.source_character_lease()
        || current.current_runtime_scope()
            != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
        || current.current_scope_generation() != binding.scope_generation()
        || prepared.house().world_id() != binding.world_id()
        || source_runtime
            .bound_attacker_lease(source_actor, prepared.source_session())
            .map_err(|_| HouseEntryStageError::NativeAuthorityMismatch)?
            != prepared.source_character_lease()
        || source_runtime
            .positioned_player_for_session(prepared.source_session())
            .map_err(|_| HouseEntryStageError::NativeAuthorityMismatch)?
            .is_none_or(|(actor, position)| {
                actor != source_actor
                    || position.context() != source_runtime.pinned_movement_context()
            })
    {
        return Err(HouseEntryStageError::NativeAuthorityMismatch);
    }
    let cells = room.movement_cells();
    if cells.scope().world_id != binding.world_id()
        || cells.scope().generation_digest != pinned.server_artifact_digest()
        || room.compiled().server_digest() != pinned.server_artifact_digest()
        || room.compiled().client_digest() != pinned.client_artifact_digest()
        || room.frame_binding().digest() != pinned.frame_binding_digest()
        || room.map_revision_digest() != pinned.map_revision_digest()
        || encode_spatial_position(destination) != prepared.reserved_tile()
    {
        return Err(HouseEntryStageError::ContentBindingMismatch);
    }
    let house = cells
        .house_tiles()
        .house_at(
            cells.scope(),
            LogicalCell {
                x: destination.x,
                y: destination.y,
                z: i32::from(destination.floor),
            },
        )
        .map_err(|_| HouseEntryStageError::QualifiedHouseUnavailable)?
        .ok_or(HouseEntryStageError::QualifiedHouseUnavailable)?;
    if house.house().key != prepared.house().house_key() {
        return Err(HouseEntryStageError::ContentBindingMismatch);
    }
    // entry() is an exterior fallback in the accepted qualifier. It is not
    // used as the interior reservation; house_at() proves actual membership.
    let content = StagedHouseContentIdentity {
        house_revision: house.house().revision.clone(),
        server_artifact_digest: pinned.server_artifact_digest(),
        qualified_placement_digest: house.placement_digest(),
    };
    let allocated = sqlx::query(
        "SELECT h.house_revision,h.compatible_content_digest,h.qualified_placement_digest \
         FROM game_world_house_instances h JOIN game_house_ownership o USING(world_id,house_key) \
         WHERE h.world_id=encode($1,'hex')::uuid AND h.house_key=$2 \
           AND h.instance_id=encode($3,'hex')::uuid FOR SHARE OF h,o",
    )
    .bind(prepared.house().world_id().as_bytes().as_slice())
    .bind(prepared.house().house_key())
    .bind(prepared.house().instance_id().as_slice())
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(HouseEntryStageError::ContentBindingMismatch)?;
    if allocated.try_get::<String, _>("house_revision")? != content.house_revision
        || allocated
            .try_get::<Vec<u8>, _>("compatible_content_digest")?
            .as_slice()
            != content.server_artifact_digest
        || allocated
            .try_get::<Vec<u8>, _>("qualified_placement_digest")?
            .as_slice()
            != content.qualified_placement_digest
        || content.server_artifact_digest == [0; 32]
        || content.qualified_placement_digest == [0; 32]
    {
        return Err(HouseEntryStageError::ContentBindingMismatch);
    }
    // Read only this actual qualified House (at most 4096 cells), once. Every
    // native loaded location must be in it; no global catalogue reset/scan and
    // no copied spatial encoder are used to validate the retained snapshot.
    let qualified_cells = house
        .physical_cells()
        .map(|cell| {
            crate::content::checked_movement_position(cell)
                .map(encode_spatial_position)
                .ok_or(HouseEntryStageError::ContentBindingMismatch)
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    validate_loaded_interior(
        prepared.interior(),
        prepared.house(),
        prepared.scope_generation().get(),
        &qualified_cells,
    )?;
    let source_hold =
        source_runtime.hold_house_source_actor(source_actor, prepared.source_session())?;
    let mut placement = house_actors.reserve_entry(&prepared)?;
    placement.materialize(&source_hold)?;
    placement.consume_reservation()?;
    placement.verify_consumed(&prepared)?;
    Ok(StagedHouseEntry {
        transaction: tx,
        prepared,
        placement,
        source_hold,
        content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precheck_refuses_in_order() {
        let open = HouseEntryPrecheck {
            has_access: true,
            house_open: true,
            ..HouseEntryPrecheck::default()
        };
        assert_eq!(open.evaluate(), None);
        assert_eq!(
            HouseEntryPrecheck::default().evaluate(),
            Some(HouseEntryRefusal::NoAccess)
        );
        let closed = HouseEntryPrecheck {
            house_open: false,
            combat_locked: true,
            ..open
        };
        assert_eq!(closed.evaluate(), Some(HouseEntryRefusal::HouseClosed));
        let fighting = HouseEntryPrecheck {
            combat_locked: true,
            operation_pending: true,
            ..open
        };
        assert_eq!(fighting.evaluate(), Some(HouseEntryRefusal::InCombat));
        let busy = HouseEntryPrecheck {
            operation_pending: true,
            ..open
        };
        assert_eq!(busy.evaluate(), Some(HouseEntryRefusal::Busy));
    }

    #[test]
    fn exit_stays_refused() {
        assert!(matches!(
            exit_into_channel(),
            Err(HouseHandoffError::ExitNotAdmitted)
        ));
    }
}
