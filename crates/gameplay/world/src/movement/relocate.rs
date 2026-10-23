//! Creature destinations: the Movement owner's read-only admission, shared by search and commit.
use crate::content::LogicalCell;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, MovementPositionSnapshot,
};
use oteryn_gameplay_domain::domain::carrier_values::CarrierError;

pub fn creature_cell_admissible(
    cells: &crate::content::NativeEntryMovementCells,
    doors: &dyn crate::map::floor::CellBlocker,
    cell: MovementLocalPosition,
) -> bool {
    let target = LogicalCell {
        x: cell.x,
        y: cell.y,
        z: i32::from(cell.floor),
    };
    let tile = match cells.spell_tiles().lookup(cells.scope(), target) {
        Ok(tile) => Some((tile.flags().protection_zone, tile.flags().floor_change)),
        // An accepted entry room without a tile document (ENTRY-SPELL-TILES-1 gave
        // `native_entry_room.json` one) answers every lookup `Unknown`. Its Movement owner admits
        // a player step on terrain and doors alone ("accepted entry movement",
        // `field_step_ingress`), so a creature step follows the same rule.
        Err(crate::content::SpellTileLookupError::Unknown) if accepted_entry_movement(cells) => {
            Some((false, false))
        }
        // Unknown cells of a tile-qualified map, an absent tile or another scope refuse.
        Err(_) => None,
    };
    cell_open(
        cells.index().lookup(cells.scope(), target) == Ok(crate::content::CollisionClass::Walkable),
        doors.blocks(target),
        crate::map::floor::is_teleport(cells, target),
        tile,
    )
}

/// Whether `cells` move by the accepted entry movement: the entry-room collision index (not a
/// source world, not a bundle) outside the source profile. Its tiles are qualified from the room's
/// own optional tile document, so `Unknown` there means only "no document"; a source-qualified
/// map's `Unknown` names a genuinely unknown cell and a bundle's lookup is another scope.
pub fn accepted_entry_movement(cells: &crate::content::NativeEntryMovementCells) -> bool {
    matches!(
        cells.index(),
        crate::content::native_cell_lookup::NativeMovementCollisionIndex::Entry(_)
    ) && !crate::movement::source_floor_change::is_source_profile(cells)
}

pub const fn cell_open(
    walkable: bool,
    door: bool,
    teleport: bool,
    tile: Option<(bool, bool)>,
) -> bool {
    match tile {
        Some((protection_zone, floor_change)) => {
            walkable && !door && !teleport && !protection_zone && !floor_change
        }
        None => false,
    }
}

/// Relocation admits exactly one destination and commits one position successor. The carrier
/// checks exact creature identity and current-owner snapshot again; players are refused.
pub fn relocate_creature(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    cells: &crate::content::NativeEntryMovementCells,
    blocking: &dyn crate::map::floor::CellBlocker,
    destination: MovementLocalPosition,
) -> Result<MovementPositionSnapshot, CarrierError> {
    if !runtime.contains_live_creature(actor) || runtime.native_summon_role(actor)?.is_some() {
        return Err(CarrierError::NotCreature);
    }
    let before = runtime.read_actor_position(actor)?;
    if cells.scope().world_id != before.world_id()
        || cells.scope().generation_digest != runtime.content_pin().server_artifact_digest()
        || !creature_cell_admissible(cells, blocking, destination)
        || runtime.position_occupied_by_other(actor, destination)?
    {
        return Err(CarrierError::WrongScope);
    }
    runtime
        .borrow_movement_position()
        .commit_creature_relocation(actor, before, destination)
}

/// Reuse the native endpoint-exclusive sight trace; unknown projectile facts fail closed.
/// Dynamic blockers are read through the same Movement-owned view used for admission.
pub fn creature_sight_clear(
    cells: &crate::content::NativeEntryMovementCells,
    blocking: &dyn crate::map::floor::CellBlocker,
    from: MovementLocalPosition,
    to: MovementLocalPosition,
) -> bool {
    crate::durability::native_world_read::source_sight_steps(from, to).is_ok_and(|steps| {
        steps.into_iter().all(|(at, exempt)| {
            let cell = LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            };
            exempt
                || (!blocking.blocks(cell)
                    && cells
                        .spell_tiles()
                        .lookup(cells.scope(), cell)
                        .is_ok_and(|tile| !tile.flags().block_projectile))
        })
    })
}
