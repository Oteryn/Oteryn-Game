//! Explicit Game-owned test spawn; position is NOT a public Canary spawn record.
//! Source-qualified Rat policy/geometry and actual scope/Item owners remain independent.
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{CollisionClass, LogicalCell, QualifiedNativeEntryRoom};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityRoot, character_authority::ReconciledCharacterAuthority};
use crate::foundation::{
    ChannelRuntimeV1, CompanionSnapshot, MovementLocalPosition, RuntimeScopeRefV1,
};

pub(super) const POSITION: MovementLocalPosition = MovementLocalPosition {
    x: 1,
    y: 11,
    floor: -7,
};
pub(super) const DECLARATION: &str = "oteryn:qualification.spawn/thalom-rat-r1";

#[allow(clippy::too_many_arguments)]
pub(super) async fn realize_rat_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    root: &DurabilityRoot,
    recovery: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    runtime: &mut ChannelRuntimeV1,
) -> Result<CompanionSnapshot, String> {
    let pin = runtime.content_pin().clone();
    let cells = room.movement_cells();
    if runtime.owner_fence().is_err()
        || native.source_digest() != pin.server_artifact_digest()
        || room.compiled().server_digest() != pin.server_artifact_digest()
        || room.frame_binding().digest() != pin.frame_binding_digest()
        || room.map_revision_digest() != pin.map_revision_digest()
        || cells.scope().world_id != runtime.binding().world_id()
        || cells.scope().generation_digest != pin.server_artifact_digest()
    {
        return Err("qualification spawn current Content/scope mismatch".into());
    }
    let cell = LogicalCell {
        x: POSITION.x,
        y: POSITION.y,
        z: i32::from(POSITION.floor),
    };
    // This native declaration requires completely qualified source metadata.
    // Pending source placements cannot be treated as absent durable Items.
    if room.source_world().is_none_or(|world| {
        world
            .pending_mutable_placements()
            .iter()
            .any(|placement| placement.cell() == cell)
    }) || cells.index().lookup(cells.scope(), cell) != Ok(CollisionClass::Walkable)
    {
        return Err(
            "qualification spawn cell is unknown, blocked or pending materialization".into(),
        );
    }
    let tile = cells
        .spell_tiles()
        .lookup(cells.scope(), cell)
        .map_err(|_| "qualification spawn has no exact tile metadata")?;
    let flags = tile.flags();
    if !tile.ground_present() || flags.block_solid || flags.protection_zone || flags.floor_change {
        return Err("qualification spawn ground/solidity/PZ/floor eligibility refused".into());
    }
    let target = crate::spell::world_items_execution::SpellGroundTarget::for_native_tile_read(
        room, runtime, POSITION,
    )
    .map_err(str::to_owned)?;
    let scope =
        RuntimeScopeRefV1::channel(runtime.binding().world_id(), runtime.binding().channel_id());
    let owned = crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
        tx,
        root,
        recovery,
        node,
        scope,
        runtime.binding().scope_generation().get(),
    )
    .await
    .map_err(|error| format!("qualification spawn scope fence: {error:?}"))?;
    let items =
        crate::durability::spell_item_transaction::read_qualification_scope_tile_in_transaction(
            tx, &owned, &target,
        )
        .await
        .map_err(|error| format!("qualification spawn current Item tile: {error:?}"))?;
    if items
        .items
        .iter()
        .any(|item| item.blocks_movement || item.immovable_block_solid)
    {
        return Err("qualification spawn blocked by actual current Item owner".into());
    }
    native
        .install_companion_policies(runtime)
        .map_err(|error| format!("qualification spawn active Creature policies: {error:?}"))?;
    runtime
        .realize_native_qualification_spawn(pin, "canary:creature/rat", POSITION)
        .map_err(|error| format!("qualification spawn physical realization: {error:?}"))
}
