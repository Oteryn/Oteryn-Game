//! WORLD-INTERACTION-0 §6.3: rope-spot ascent uses the existing movement owner.
use super::*;

// canary:data/global.lua:83-84 and data/libs/functions/tile.lua:96.
const ROPE_GROUND: &[u32] = &[
    386, 421, 7762, 12202, 12936, 14238, 17238, 21965, 21966, 21967, 21968, 23363,
];
/// Whether an authored ground or top-level item marks a rope spot.
pub fn is_rope_spot_item(source_id: u32, ground: bool) -> bool {
    if ground {
        ROPE_GROUND.contains(&source_id)
    } else {
        source_id == 12935
    }
}

// Accepted §6.3 order: south, north, east, west, then the diagonals.
const OFFSETS: &[(i32, i32)] = &[
    (0, 1),
    (0, -1),
    (1, 0),
    (-1, 0),
    (-1, 1),
    (1, 1),
    (-1, -1),
    (1, -1),
];

/// Exact map identity selected by USE; recreated projections have a new identity.
#[derive(Clone, Copy)]
pub enum LadderTarget {
    Base { key: u64, revision: u64 },
    Added { id: u64 },
}

struct RopeProof<'a> {
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    spot: LogicalCell,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
    destination: MovementLocalPosition,
    ladder: Option<(&'a crate::map::boot::BundleMap, LadderTarget)>,
}

/// Qualify a resolved, in-reach rope spot and a free upstairs landing. This produces the same
/// sealed position successor as floor traversal; no respawn or spell state is cleared.
#[allow(
    clippy::too_many_arguments,
    reason = "independently resolved owner facts"
)]
pub fn prepare_bundle_rope<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    spot: LogicalCell,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    let mut proof = RopeProof {
        cells,
        actor,
        session,
        expected,
        spot,
        pz_blocked,
        closed,
        destination: expected.position(),
        ladder: None,
    };
    proof.destination = proof.resolve(runtime)?;
    Ok(proof)
}

/// Qualify an immovable ladder and the same native upstairs landing used by rope.
#[allow(clippy::too_many_arguments)]
pub fn prepare_bundle_ladder<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    map: &'a crate::map::boot::BundleMap,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    spot: LogicalCell,
    target: LadderTarget,
    pz_blocked: bool,
    closed: &'a dyn CellBlocker,
) -> Result<impl crate::foundation::SourceStepCommitProof + 'a, MovementError> {
    let mut proof = RopeProof {
        cells,
        actor,
        session,
        expected,
        spot,
        pz_blocked,
        closed,
        destination: expected.position(),
        ladder: Some((map, target)),
    };
    proof.destination = proof.resolve(runtime)?;
    Ok(proof)
}

impl RopeProof<'_> {
    fn resolve(&self, runtime: &ChannelRuntimeV1) -> Result<MovementLocalPosition, MovementError> {
        qualify_origin(
            runtime,
            self.cells,
            self.actor,
            self.session,
            self.expected,
            None,
        )?;
        let origin = self.expected.position();
        if i32::from(origin.floor) != self.spot.z
            || origin.x.abs_diff(self.spot.x) > 1
            || origin.y.abs_diff(self.spot.y) > 1
        {
            return Err(MovementError::NotQualified);
        }
        if let Some((map, target)) = self.ladder {
            use crate::map::view::MapFacts;
            let NativeMovementCollisionIndex::Bundle(collision) = self.cells.index() else {
                return Err(MovementError::NotQualified);
            };
            if !std::ptr::eq(collision.map(), map) {
                return Err(MovementError::NotQualified);
            }
            let position = TilePos {
                x: self
                    .spot
                    .x
                    .try_into()
                    .map_err(|_| MovementError::NotQualified)?,
                y: self
                    .spot
                    .y
                    .try_into()
                    .map_err(|_| MovementError::NotQualified)?,
                floor: (-self.spot.z)
                    .try_into()
                    .map_err(|_| MovementError::NotQualified)?,
            };
            let entry = match target {
                LadderTarget::Base { key, revision } => {
                    let (pos, ordinal) =
                        crate::map::view::placement_of(key).ok_or(MovementError::NotQualified)?;
                    if pos != position
                        || map.facts.base_hidden(key)
                        || map.facts.object_revision(key) != revision
                    {
                        return Err(MovementError::NotQualified);
                    }
                    map.overlay
                        .base()
                        .tile(pos.x, pos.y, pos.floor)
                        .and_then(|tile| {
                            tile.ids()
                                .iter()
                                .zip(tile.depths())
                                .filter(|(_, d)| **d == 0)
                                .nth(usize::from(ordinal))
                        })
                        .and_then(|(id, _)| map.facts.base_entry(pos, ordinal, *id))
                }
                LadderTarget::Added { id } => {
                    let (pos, entry) = map
                        .facts
                        .movement_item(id)
                        .ok_or(MovementError::NotQualified)?;
                    if pos != position {
                        return Err(MovementError::NotQualified);
                    }
                    map.facts
                        .movement_items(pos)
                        .into_iter()
                        .find(|item| item.source == crate::map::view::EntrySource::Added(entry))
                        .map(|item| item.facts)
                }
            }
            .ok_or(MovementError::NotQualified)?;
            if entry.pickupable
                || !crate::map::ladder_catalogue::LADDERS.contains(&entry.appearance_id)
            {
                return Err(MovementError::NotQualified);
            }
        } else {
            let tile = self
                .cells
                .spell_tiles()
                .lookup(self.cells.scope(), self.spot)
                .map_err(|_| MovementError::NotQualified)?;
            if !tile.ground_present()
                || !(tile
                    .ground_source_id()
                    .is_some_and(|id| is_rope_spot_item(id, true))
                    || tile
                        .top_source_ids()
                        .iter()
                        .any(|&id| is_rope_spot_item(id, false)))
            {
                return Err(MovementError::NotQualified);
            }
        }
        bundle_upstairs_landing(
            self.cells,
            self.spot,
            self.pz_blocked,
            self.closed,
            |next| {
                runtime
                    .position_occupied_by_other(self.actor, next)
                    .map(|occupied| !occupied)
                    .map_err(MovementError::Actor)
            },
        )
    }
}

/// Pull the exact live creature from its current cell through the movement
/// carrier. The carrier retains summon separation and movement reservation rules.
pub fn commit_bundle_rope_creature(
    runtime: &mut ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    expected: MovementPositionSnapshot,
    closed: &dyn CellBlocker,
) -> Result<(), MovementError> {
    if expected.context() != runtime.pinned_movement_context()
        || cells.scope().world_id != expected.world_id()
        || cells.scope().generation_digest != runtime.content_pin().server_artifact_digest()
    {
        return Err(MovementError::NotQualified);
    }
    let from = expected.position();
    let source = LogicalCell {
        x: from.x,
        y: from.y,
        z: i32::from(from.floor),
    };
    let destination = bundle_upstairs_landing(cells, source, false, closed, |next| {
        if !crate::movement::relocate::creature_cell_admissible(cells, closed, next) {
            return Ok(false);
        }
        runtime
            .position_occupied_by_other(actor, next)
            .map(|occupied| !occupied)
            .map_err(MovementError::Actor)
    })?;
    runtime
        .borrow_movement_position()
        .commit_creature_relocation(actor, expected, destination)
        .map(|_| ())
        .map_err(MovementError::Actor)
}

/// Resolve the south-first landing one floor above a rope source. The caller
/// supplies the occupancy policy for the actor or item being moved.
pub fn bundle_upstairs_landing(
    cells: &NativeEntryMovementCells,
    source: LogicalCell,
    pz_blocked: bool,
    closed: &dyn CellBlocker,
    mut qualifies: impl FnMut(MovementLocalPosition) -> Result<bool, MovementError>,
) -> Result<MovementLocalPosition, MovementError> {
    let floor = source
        .z
        .checked_sub(1)
        .filter(|z| (0..=MAX_Z).contains(z))
        .ok_or(MovementError::CoordinateOverflow)?;
    let ground = ground_of(cells);
    let landable = landable_of(cells, pz_blocked, closed);
    for &(dx, dy) in OFFSETS {
        let (Some(x), Some(y)) = (source.x.checked_add(dx), source.y.checked_add(dy)) else {
            continue;
        };
        let next = MovementLocalPosition {
            x,
            y,
            floor: i16::try_from(floor).map_err(|_| MovementError::CoordinateOverflow)?,
        };
        if ground((x, y, floor)) == Ground::Walkable && landable((x, y, floor)) && qualifies(next)?
        {
            return Ok(next);
        }
    }
    Err(MovementError::Blocked)
}

impl crate::foundation::source_step_seal::Sealed for RopeProof<'_> {}
impl crate::foundation::SourceStepCommitProof for RopeProof<'_> {
    fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), CarrierError> {
        if self.resolve(runtime).ok() != Some(self.destination) {
            return Err(CarrierError::PlanConflict);
        }
        Ok(())
    }

    fn parts(
        &self,
    ) -> (
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
        MovementFacing,
    ) {
        (
            self.actor,
            self.expected,
            self.destination,
            self.expected.facing().unwrap_or(MovementFacing::South),
        )
    }
}
