//! Genuine source-map STEP destination rules. Only the explicit Canary map
//! profile binds source z to native floor=-z; other movement profiles are untouched.
use super::{CardinalStep, MovementError};
use crate::content::{
    LogicalCell, NativeEntryMovementCells, QualifiedSpellTile, SourceFloorChange,
    SpellTileLookupError,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementFacing, MovementLocalPosition,
    MovementPositionSnapshot,
};

const FRAME: &str = "oteryn:frame/canary-thalom-spell-r3";
const MAP: &str = "oteryn:map/canary-thalom-spell-r3";
const MAX_FLOOR_DESTINATIONS: usize = 16;

/// Private construction consumes actual qualified cells, not a caller's target.
pub(crate) struct SourceStepProof<'a> {
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    direction: CardinalStep,
    destination: MovementLocalPosition,
    current_tiles: Option<
        std::collections::BTreeMap<(i32, i32, i16), (Option<QualifiedSpellTile>, bool, u16)>,
    >,
    _transaction: std::marker::PhantomData<&'a mut ()>,
}
impl SourceStepProof<'_> {
    pub(crate) fn matches_request(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: CardinalStep,
    ) -> bool {
        self.actor == actor && self.session == session && self.direction == direction
    }
    pub(crate) const fn expected(&self) -> MovementPositionSnapshot {
        self.expected
    }
    pub(crate) const fn destination(&self) -> MovementLocalPosition {
        self.destination
    }
    pub(crate) fn origin_ground_speed(&self) -> Option<u16> {
        if let Some(tiles) = &self.current_tiles {
            current_tile(tiles, self.expected.position())
                .ok()
                .flatten()?
                .ground_speed()
        } else {
            tile(self.cells, self.expected.position())
                .ok()
                .flatten()?
                .ground_speed()
        }
    }
    pub(crate) fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        qualify_origin(runtime, self.cells, self.actor, self.session, self.expected)?;
        let observed = if let Some(tiles) = &self.current_tiles {
            destination_with_lookup(self.expected.position(), self.direction, |p| {
                current_tile(tiles, p)
            })?
        } else {
            destination(self.cells, self.expected.position(), self.direction)?
        };
        if observed != self.destination {
            return Err(MovementError::SnapshotMismatch);
        }
        Ok(())
    }
    pub(crate) fn parts(
        &self,
    ) -> (
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
        MovementFacing,
    ) {
        let facing = match self.direction {
            CardinalStep::North => MovementFacing::North,
            CardinalStep::East => MovementFacing::East,
            CardinalStep::South => MovementFacing::South,
            CardinalStep::West => MovementFacing::West,
        };
        (self.actor, self.expected, self.destination, facing)
    }
}
pub(crate) fn is_source_profile(cells: &NativeEntryMovementCells) -> bool {
    cells.scope().coordinate_frame.as_str() == FRAME && cells.scope().map_revision.as_str() == MAP
}
pub(crate) fn prepare_source_step<'a>(
    runtime: &ChannelRuntimeV1,
    cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    direction: CardinalStep,
) -> Result<SourceStepProof<'a>, MovementError> {
    qualify_origin(runtime, cells, actor, session, expected)?;
    let destination = destination(cells, expected.position(), direction)?;
    Ok(SourceStepProof {
        cells,
        actor,
        session,
        expected,
        direction,
        destination,
        current_tiles: None,
        _transaction: std::marker::PhantomData,
    })
}
fn qualify_origin(
    runtime: &ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
) -> Result<(), MovementError> {
    if !is_source_profile(cells)
        || runtime.actor_spell_reserved(actor)
        || cells.scope().world_id != runtime.binding().world_id()
        || cells.scope().generation_digest != runtime.content_pin().server_artifact_digest()
        || expected.context() != runtime.pinned_movement_context()
        || runtime
            .read_actor_position(actor)
            .map_err(MovementError::Actor)?
            != expected
        || !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|c| c.control_loss.is_none())
    {
        return Err(MovementError::NotQualified);
    }
    Ok(())
}
fn shifted(
    p: MovementLocalPosition,
    dx: i32,
    dy: i32,
    df: i16,
) -> Result<MovementLocalPosition, MovementError> {
    Ok(MovementLocalPosition {
        x: p.x
            .checked_add(dx)
            .ok_or(MovementError::CoordinateOverflow)?,
        y: p.y
            .checked_add(dy)
            .ok_or(MovementError::CoordinateOverflow)?,
        floor: p
            .floor
            .checked_add(df)
            .ok_or(MovementError::CoordinateOverflow)?,
    })
}
fn tile(
    cells: &NativeEntryMovementCells,
    p: MovementLocalPosition,
) -> Result<Option<&QualifiedSpellTile>, MovementError> {
    match cells.spell_tiles().lookup(
        cells.scope(),
        LogicalCell {
            x: p.x,
            y: p.y,
            z: i32::from(p.floor),
        },
    ) {
        Ok(t) => Ok(Some(t)),
        Err(SpellTileLookupError::Absent) => Ok(None),
        Err(SpellTileLookupError::Unknown) => Err(MovementError::NotQualified),
        Err(SpellTileLookupError::ScopeMismatch) => Err(MovementError::ScopeMismatch),
    }
}
fn changes(t: &QualifiedSpellTile) -> Result<&[SourceFloorChange], MovementError> {
    Ok(&t
        .source_step()
        .ok_or(MovementError::NotQualified)?
        .floor_changes)
}
fn has(t: &QualifiedSpellTile, k: SourceFloorChange) -> Result<bool, MovementError> {
    Ok(changes(t)?.contains(&k))
}
fn has_height(t: &TileView<'_>) -> Result<bool, MovementError> {
    Ok(t.height_count >= 3)
}
#[derive(Clone, Copy)]
struct TileView<'a> {
    tile: &'a QualifiedSpellTile,
    solid: bool,
    height_count: u16,
}
impl std::ops::Deref for TileView<'_> {
    type Target = QualifiedSpellTile;
    fn deref(&self) -> &Self::Target {
        self.tile
    }
}
fn current_tile<'a>(
    tiles: &'a std::collections::BTreeMap<(i32, i32, i16), (Option<QualifiedSpellTile>, bool, u16)>,
    p: MovementLocalPosition,
) -> Result<Option<TileView<'a>>, MovementError> {
    let (tile, solid, height_count) = tiles
        .get(&(p.x, p.y, p.floor))
        .ok_or(MovementError::NotQualified)?;
    Ok(tile.as_ref().map(|tile| TileView {
        tile,
        solid: *solid,
        height_count: *height_count,
    }))
}
fn destination(
    cells: &NativeEntryMovementCells,
    origin: MovementLocalPosition,
    direction: CardinalStep,
) -> Result<MovementLocalPosition, MovementError> {
    destination_with_lookup(origin, direction, |p| {
        tile(cells, p)?
            .map(|tile| {
                Ok(TileView {
                    tile,
                    solid: tile.flags().block_solid,
                    height_count: tile
                        .source_step()
                        .ok_or(MovementError::NotQualified)?
                        .height_count,
                })
            })
            .transpose()
    })
}
fn clear_terminal_destination(
    position: MovementLocalPosition,
    current: &TileView<'_>,
) -> Result<MovementLocalPosition, MovementError> {
    if !current.ground_present() || current.solid {
        return Err(MovementError::Blocked);
    }
    Ok(position)
}
fn destination_with_lookup<'a>(
    origin: MovementLocalPosition,
    direction: CardinalStep,
    mut lookup: impl FnMut(MovementLocalPosition) -> Result<Option<TileView<'a>>, MovementError>,
) -> Result<MovementLocalPosition, MovementError> {
    let (dx, dy) = match direction {
        CardinalStep::North => (0, -1),
        CardinalStep::East => (1, 0),
        CardinalStep::South => (0, 1),
        CardinalStep::West => (-1, 0),
    };
    let mut next = shifted(origin, dx, dy, 0)?;
    // game.cpp2048..2079 uses source z8/7 boundaries and height3.
    let origin_tile = lookup(origin)?.ok_or(MovementError::Blocked)?;
    if origin.floor != -8 && has_height(&origin_tile)? {
        let above = lookup(shifted(origin, 0, 0, 1)?)?;
        if above.is_none_or(|t| !t.ground_present() && !t.solid) {
            if let Some(upper) = lookup(shifted(next, 0, 0, 1)?)?
                && upper.ground_present()
                && !upper.solid
                && changes(&upper)?.is_empty()
            {
                next = shifted(next, 0, 0, 1)?;
            }
        }
    }
    if origin.floor != -7 && origin.floor == next.floor {
        let here = lookup(next)?;
        if here.is_none_or(|t| !t.ground_present() && !t.solid) {
            if let Some(lower) = lookup(shifted(next, 0, 0, -1)?)?
                && has_height(&lower)?
            {
                next = shifted(next, 0, 0, -1)?;
            }
        }
    }
    let entered = lookup(next)?.ok_or(MovementError::Blocked)?;
    if !entered.ground_present() || entered.solid {
        return Err(MovementError::Blocked);
    }
    // internalMoveCreature queries successive source tile destinations, bounded
    // by MAP_MAX_LAYERS. Missing capture facts refuse; genuine absent target
    // leaves the creature on the entered tile, exactly as queryDestination does.
    for _ in 0..MAX_FLOOR_DESTINATIONS {
        let current = lookup(next)?.ok_or(MovementError::NotQualified)?;
        let flags = changes(&current)?;
        if flags.is_empty() {
            return clear_terminal_destination(next, &current);
        }
        let target = if flags.contains(&SourceFloorChange::Down) {
            let below = shifted(next, 0, 0, -1)?;
            let south = lookup(shifted(below, 0, -1, 0)?)?;
            if south
                .map(|t| has(&t, SourceFloorChange::Southalt))
                .transpose()?
                .unwrap_or(false)
            {
                shifted(below, 0, -2, 0)?
            } else {
                let east = lookup(shifted(below, -1, 0, 0)?)?;
                if east
                    .map(|t| has(&t, SourceFloorChange::Eastalt))
                    .transpose()?
                    .unwrap_or(false)
                {
                    shifted(below, -2, 0, 0)?
                } else if let Some(down) = lookup(below)? {
                    let (mut x, mut y) = (0, 0);
                    for kind in changes(&down)? {
                        match kind {
                            SourceFloorChange::North => y += 1,
                            SourceFloorChange::South => y -= 1,
                            SourceFloorChange::Southalt => y -= 2,
                            SourceFloorChange::East => x -= 1,
                            SourceFloorChange::Eastalt => x -= 2,
                            SourceFloorChange::West => x += 1,
                            SourceFloorChange::Down => {}
                        }
                    }
                    shifted(below, x, y, 0)?
                } else {
                    return clear_terminal_destination(next, &current);
                }
            }
        } else {
            let (mut x, mut y) = (0, 0);
            for kind in flags {
                match kind {
                    SourceFloorChange::North => y -= 1,
                    SourceFloorChange::South => y += 1,
                    SourceFloorChange::Southalt => y += 2,
                    SourceFloorChange::East => x += 1,
                    SourceFloorChange::Eastalt => x += 2,
                    SourceFloorChange::West => x -= 1,
                    SourceFloorChange::Down => {}
                }
            }
            shifted(next, x, y, 1)?
        };
        if lookup(target)?.is_none() {
            return clear_terminal_destination(next, &current);
        }
        next = target;
    }
    Err(MovementError::CapacityExceeded)
}

impl crate::foundation::source_step_seal::Sealed for SourceStepProof<'_> {}
impl crate::foundation::SourceStepCommitProof for SourceStepProof<'_> {
    fn validate_current(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Result<(), crate::foundation::CarrierError> {
        SourceStepProof::validate_current(self, runtime).map_err(|error| match error {
            MovementError::Actor(error) => error,
            MovementError::SnapshotMismatch => {
                crate::foundation::CarrierError::PositionSnapshotMismatch
            }
            MovementError::ContextMismatch => {
                crate::foundation::CarrierError::PositionContextMismatch
            }
            _ => crate::foundation::CarrierError::PlanConflict,
        })
    }
    fn parts(
        &self,
    ) -> (
        ExactActorRef,
        MovementPositionSnapshot,
        MovementLocalPosition,
        MovementFacing,
    ) {
        SourceStepProof::parts(self)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::content::native_gameplay::{
        NativeGameplayInput, NativeGameplayMapProfile, PinnedGameplayBytes,
    };
    fn pin(bytes: &[u8]) -> PinnedGameplayBytes {
        PinnedGameplayBytes {
            bytes: bytes.to_vec(),
            sha256: <sha2::Sha256 as sha2::Digest>::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        }
    }
    fn id(n: u8) -> [u8; 16] {
        [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n]
    }
    fn owner() -> (
        ChannelRuntimeV1,
        crate::content::QualifiedNativeEntryRoom,
        ExactActorRef,
        GameSessionId,
    ) {
        let world = crate::foundation::WorldId::decode(&id(1)).unwrap();
        let input = NativeGameplayInput {
            native_map_profile: NativeGameplayMapProfile::AcceptedEntryR1,
            catalog: pin(include_bytes!(
                "../../../../tools/content-schema/spell-authoring/samples/executable-spell-catalog.json"
            )),
            source_selection: pin(include_bytes!(
                "../../../../tools/content-schema/spell-authoring/samples/executable-spell-source-selection.json"
            )),
            creature_profiles: pin(include_bytes!(
                "../../../../tools/content-schema/native-gameplay/creature_profiles.json"
            )),
            presentation_profiles: pin(include_bytes!(
                "../../../../tools/content-schema/native-gameplay/presentation_profiles.json"
            )),
            item_profiles: None,
            spell_appearances: None,
            build_training: None,
            familiar_config: None,
            familiar_defenses: None,
            wheel_profile: None,
            source_world: None,
        };
        let room = crate::content::qualify_native_source_spell_world_with_gameplay(
            world,
            &input,
            include_bytes!(
                "../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            ),
        )
        .unwrap();
        let start = room.entry_start();
        let pin = crate::foundation::ChannelContentPin::from_activation(
            world,
            1,
            room.compiled().server_digest(),
            room.compiled().client_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            crate::foundation::ChannelId::decode(&id(2)).unwrap(),
            crate::foundation::NodeId::decode(&id(3)).unwrap(),
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .unwrap();
        let session = GameSessionId::decode(&id(4)).unwrap();
        let reservation = runtime.reserve_fresh_session(session).unwrap();
        let actor = runtime.commit_fresh_session(reservation).unwrap();
        runtime.initialize_first_entry_position(actor).unwrap();
        (runtime, room, actor, session)
    }
    #[test]
    fn genuine_temple_steps_reach_source_stairs_without_relocation() {
        let (mut runtime, room, actor, session) = owner();
        let cells = room.movement_cells();
        let start = runtime.read_actor_position(actor).unwrap().position();
        let goal = MovementLocalPosition {
            x: 0,
            y: 11,
            floor: -7,
        };
        // Search only independently qualified, source STEP destinations. No
        // invented neighbours or direct position writes participate in the path.
        let key = |p: MovementLocalPosition| (p.x, p.y, p.floor);
        let mut visited = std::collections::BTreeSet::from([key(start)]);
        let mut queue = std::collections::VecDeque::from([(start, Vec::new())]);
        let mut found = None;
        while let Some((at, path)) = queue.pop_front() {
            if at == goal {
                found = Some(path);
                break;
            }
            for dir in [
                CardinalStep::North,
                CardinalStep::East,
                CardinalStep::South,
                CardinalStep::West,
            ] {
                if let Ok(next) = destination(cells, at, dir)
                    && visited.insert(key(next))
                {
                    let mut path = path.clone();
                    path.push(dir);
                    queue.push_back((next, path));
                }
            }
        }
        let path = found.expect("actual captured source stairs connect Temple to floor7");
        assert!(path.len() > 1);
        let mut floors = std::collections::BTreeSet::from([-5]);
        for dir in path {
            let before = runtime.read_actor_position(actor).unwrap();
            let proof = prepare_source_step(&runtime, cells, actor, session, before, dir).unwrap();
            let after = runtime.commit_source_step(proof).unwrap();
            assert_eq!(after.revision(), before.revision() + 1);
            floors.insert(after.position().floor);
            assert_eq!(
                after.facing(),
                Some(match dir {
                    CardinalStep::North => MovementFacing::North,
                    CardinalStep::East => MovementFacing::East,
                    CardinalStep::South => MovementFacing::South,
                    CardinalStep::West => MovementFacing::West,
                })
            );
        }
        assert_eq!(runtime.read_actor_position(actor).unwrap().position(), goal);
        assert_eq!(floors, std::collections::BTreeSet::from([-7, -6, -5]));
    }
    #[test]
    fn source_algorithm_uses_current_collision_and_keeps_static_unknown_closed() {
        let (runtime, room, _actor, _session) = owner();
        let map = room.source_world().unwrap();
        let cells = room.movement_cells();
        let mut found = false;
        for placement in map
            .pending_mutable_placements()
            .iter()
            .filter(|p| p.source_kind().is_none() && !p.block_solid())
        {
            let cell = placement.cell();
            if !map
                .immutable_tile(map.scope(), cell)
                .unwrap()
                .ground_present()
            {
                continue;
            }
            for (direction, dx, dy) in [
                (CardinalStep::North, 0, 1),
                (CardinalStep::East, -1, 0),
                (CardinalStep::South, 0, -1),
                (CardinalStep::West, 1, 0),
            ] {
                let origin = MovementLocalPosition {
                    x: cell.x + dx,
                    y: cell.y + dy,
                    floor: cell.z.try_into().unwrap(),
                };
                if !matches!(
                    map.lookup(
                        map.scope(),
                        LogicalCell {
                            x: origin.x,
                            y: origin.y,
                            z: cell.z
                        }
                    ),
                    Ok(crate::content::CollisionClass::Walkable)
                ) {
                    continue;
                }
                // Pure source algorithm fixture only; these views cannot mint
                // a live SourceStepProof or stand in for the SQL owner collector.
                let lookup = |p: MovementLocalPosition| {
                    let logical = LogicalCell {
                        x: p.x,
                        y: p.y,
                        z: i32::from(p.floor),
                    };
                    match map.immutable_tile(map.scope(), logical) {
                        Ok(tile) => Ok(Some(TileView {
                            tile,
                            solid: tile.flags().block_solid
                                || map
                                    .pending_mutable_placements()
                                    .iter()
                                    .any(|p| p.cell() == logical && p.block_solid()),
                            height_count: tile.source_step().unwrap().height_count,
                        })),
                        Err(crate::content::static_cell_engine::StaticCellEngineError::Absent) => {
                            Ok(None)
                        }
                        Err(_) => Err(MovementError::NotQualified),
                    }
                };
                if destination_with_lookup(origin, direction, lookup)
                    .is_ok_and(|p| p.x == cell.x && p.y == cell.y && i32::from(p.floor) == cell.z)
                {
                    assert!(destination(cells, origin, direction).is_err());
                    assert!(
                        destination_with_lookup(origin, direction, |p| {
                            let Some(mut tile) = lookup(p)? else {
                                return Ok(None);
                            };
                            if p.x == cell.x && p.y == cell.y && i32::from(p.floor) == cell.z {
                                tile.solid = true;
                            }
                            Ok(Some(tile))
                        })
                        .is_err()
                    );
                    found = true;
                    break;
                }
            }
            if found {
                break;
            }
        }
        assert!(
            found,
            "actual source pickup cells connect to genuine qualified geometry"
        );
        assert_eq!(
            runtime.read_actor_position(_actor).unwrap().position(),
            MovementLocalPosition {
                x: room.entry_start().x,
                y: room.entry_start().y,
                floor: room.entry_start().floor
            }
        );
    }
    #[test]
    fn genuine_downstairs_step_inspects_solid_probe_and_lands_on_clear_floor() {
        let (_runtime, room, _actor, _session) = owner();
        let cells = room.movement_cells();
        let origin = MovementLocalPosition {
            x: 0,
            y: 4,
            floor: -5,
        };
        let probe = MovementLocalPosition {
            x: 0,
            y: 4,
            floor: -6,
        };
        assert!(tile(cells, probe).unwrap().unwrap().flags().block_solid);
        let mut queried = std::collections::BTreeSet::new();
        let lookup = |p: MovementLocalPosition| {
            tile(cells, p)?
                .map(|tile| {
                    Ok(TileView {
                        tile,
                        solid: tile.flags().block_solid,
                        height_count: tile
                            .source_step()
                            .ok_or(MovementError::NotQualified)?
                            .height_count,
                    })
                })
                .transpose()
        };
        let destination = destination_with_lookup(origin, CardinalStep::South, |p| {
            queried.insert((p.x, p.y, p.floor));
            lookup(p)
        })
        .unwrap();
        assert!(queried.contains(&(probe.x, probe.y, probe.floor)));
        assert_eq!(
            destination,
            MovementLocalPosition {
                x: 0,
                y: 6,
                floor: -6
            }
        );
        // The broken standing/combat adapter refused a necessary solid probe.
        assert!(
            destination_with_lookup(origin, CardinalStep::South, |p| {
                let fact = lookup(p)?;
                if fact.is_some_and(|tile| tile.solid) {
                    return Err(MovementError::NotQualified);
                }
                Ok(fact)
            })
            .is_err()
        );
        // Allowing a probe never allows a solid final landing.
        assert!(
            destination_with_lookup(origin, CardinalStep::South, |p| {
                let mut fact = lookup(p)?;
                if p == destination
                    && let Some(tile) = &mut fact
                {
                    tile.solid = true;
                }
                Ok(fact)
            })
            .is_err()
        );
    }

    #[test]
    fn absent_floor_followups_never_make_a_solid_flagged_stair_a_landing() {
        let (_runtime, room, _actor, _session) = owner();
        let p = |x, y, floor| MovementLocalPosition { x, y, floor };
        let cells = room.movement_cells();
        let clear = tile(cells, p(0, 4, -5)).unwrap().unwrap();
        let down = tile(cells, p(0, 5, -5)).unwrap().unwrap();
        let north = tile(cells, p(0, 5, -6)).unwrap().unwrap();
        let west = tile(cells, p(-10, 11, -7)).unwrap().unwrap();
        assert_eq!(changes(down).unwrap(), &[SourceFloorChange::Down]);
        assert_eq!(changes(west).unwrap(), &[SourceFloorChange::West]);
        // Isolated query graph uses genuine qualified tile facts. It constructs
        // neither a SourceStepProof nor current Item/movement authority.
        for second in [down, west] {
            for solid in [false, true] {
                let view = |tile, solid| TileView {
                    tile,
                    solid,
                    height_count: 0,
                };
                let graph = std::collections::BTreeMap::from([
                    ((0, 0, -5), view(clear, false)),
                    ((0, 1, -5), view(down, false)),
                    ((0, 1, -6), view(north, false)),
                    ((0, 2, -6), view(second, solid)),
                ]);
                let result = destination_with_lookup(p(0, 0, -5), CardinalStep::South, |p| {
                    Ok(graph.get(&(p.x, p.y, p.floor)).copied())
                });
                assert_eq!(
                    result,
                    if solid {
                        Err(MovementError::Blocked)
                    } else {
                        Ok(p(0, 2, -6))
                    }
                );
            }
        }
        let groundless = tile(cells, p(-14, -2, -5)).unwrap().unwrap();
        assert!(!groundless.ground_present());
        assert_eq!(
            clear_terminal_destination(
                p(0, 2, -6),
                &TileView {
                    tile: groundless,
                    solid: false,
                    height_count: 0,
                }
            ),
            Err(MovementError::Blocked)
        );
    }

    #[test]
    fn source_proof_rejects_intervening_actual_step_and_original_profile() {
        let (mut runtime, room, actor, session) = owner();
        let before = runtime.read_actor_position(actor).unwrap();
        let candidates = [
            CardinalStep::North,
            CardinalStep::East,
            CardinalStep::South,
            CardinalStep::West,
        ];
        let dir = *candidates
            .iter()
            .find(|&&d| {
                prepare_source_step(&runtime, room.movement_cells(), actor, session, before, d)
                    .is_ok()
            })
            .unwrap();
        let stale =
            prepare_source_step(&runtime, room.movement_cells(), actor, session, before, dir)
                .unwrap();
        let current =
            prepare_source_step(&runtime, room.movement_cells(), actor, session, before, dir)
                .unwrap();
        runtime.commit_source_step(current).unwrap();
        let after = runtime.read_actor_position(actor).unwrap();
        assert!(runtime.commit_source_step(stale).is_err());
        assert_eq!(runtime.read_actor_position(actor).unwrap(), after);
        let original =
            crate::content::qualify_native_entry_room(runtime.binding().world_id()).unwrap();
        assert!(!is_source_profile(original.movement_cells()));
        assert!(
            prepare_source_step(
                &runtime,
                original.movement_cells(),
                actor,
                session,
                after,
                dir
            )
            .is_err()
        );
    }
}

/// Prepared reads retain no mutation authority. Binding below rechecks the
/// originating physical transaction before a sealed synchronous owner commit.
pub(crate) struct PreparedCurrentSourceStep<'room> {
    cells: &'room NativeEntryMovementCells,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    direction: CardinalStep,
    destination: MovementLocalPosition,
    tiles: std::collections::BTreeMap<(i32, i32, i16), (Option<QualifiedSpellTile>, bool, u16)>,
    transaction_id: String,
}
impl PreparedCurrentSourceStep<'_> {
    pub(crate) fn destination(&self) -> MovementLocalPosition {
        self.destination
    }
    pub(crate) fn tile(&self, position: MovementLocalPosition) -> Option<&QualifiedSpellTile> {
        self.tiles
            .get(&(position.x, position.y, position.floor))
            .and_then(|(tile, _, _)| tile.as_ref())
    }
    pub(crate) fn validate_current(&self, runtime: &ChannelRuntimeV1) -> Result<(), MovementError> {
        qualify_origin(runtime, self.cells, self.actor, self.session, self.expected)?;
        if destination_with_lookup(self.expected.position(), self.direction, |p| {
            current_tile(&self.tiles, p)
        })? != self.destination
        {
            return Err(MovementError::SnapshotMismatch);
        }
        Ok(())
    }
}

/// Replay the unchanged bounded source algorithm while resolving only the
/// source cells that it actually requests. Each request performs a fresh sealed
/// current-player Item read and retains Ground serialization through the turn.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn collect_current_source_step<'room>(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    root: &crate::durability::DurabilityRoot,
    recovery: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
    fence: &crate::durability::item_transfer::CurrentCharacterItemFence,
    room: &'room crate::content::QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    objects: &crate::world_runtime::LocalObjectRuntime,
    content: &crate::content::native_gameplay::NativeGameplayState,
    actor: ExactActorRef,
    session: GameSessionId,
    expected: MovementPositionSnapshot,
    direction: CardinalStep,
) -> Result<PreparedCurrentSourceStep<'room>, MovementError> {
    let cells = room.movement_cells();
    qualify_origin(runtime, cells, actor, session, expected)?;
    let mut tiles = std::collections::BTreeMap::new();
    for _ in 0..128 {
        let requested = std::cell::Cell::new(None);
        let result = destination_with_lookup(expected.position(), direction, |p| {
            if !tiles.contains_key(&(p.x, p.y, p.floor)) {
                requested.set(Some(p));
                return Err(MovementError::NotQualified);
            }
            current_tile(&tiles, p)
        });
        if let Ok(destination) = result {
            let transaction_id: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
                .fetch_one(&mut **tx)
                .await
                .map_err(|_| MovementError::NotQualified)?;
            return Ok(PreparedCurrentSourceStep {
                cells,
                actor,
                session,
                expected,
                direction,
                destination,
                tiles,
                transaction_id,
            });
        }
        let Some(position) = requested.get() else {
            return Err(result.err().unwrap_or(MovementError::NotQualified));
        };
        let target = crate::spell::world_items_execution::SpellGroundTarget::for_native_tile_read(
            room, runtime, position,
        )
        .map_err(|_| MovementError::NotQualified)?;
        let read =
            crate::durability::spell_item_transaction::read_standing_player_tile_in_transaction(
                tx,
                root,
                recovery,
                node,
                fence,
                runtime.content_pin().server_artifact_digest(),
                &target,
            )
            .await
            .map_err(|_| MovementError::NotQualified)?;
        let logical = LogicalCell {
            x: position.x,
            y: position.y,
            z: i32::from(position.floor),
        };
        let tile = match cells.spell_tiles().lookup(cells.scope(), logical) {
            Err(SpellTileLookupError::Absent) => {
                // Genuine captured source air still needs the actual ordered
                // owner read. A nonempty unknown stack cannot become empty air.
                if !read.items().is_empty() {
                    return Err(MovementError::NotQualified);
                }
                None
            }
            _ => Some(
                crate::spell::world_execution::qualified_source_step_probe_in_transaction(
                    tx, &read, room, runtime, objects, position,
                )
                .await
                .map_err(|_| MovementError::NotQualified)?
                .qualified_tile()
                .clone(),
            ),
        };
        let solid = tile.as_ref().is_some_and(|t| t.flags().block_solid)
            || read.items().iter().any(|i| i.blocks_movement);
        let mut height_count = if let Some(tile) = &tile {
            tile.source_step()
                .ok_or(MovementError::NotQualified)?
                .height_count
        } else {
            0
        };
        for item in read.items() {
            let policy = content
                .item_policy(
                    &item.definition.production_key,
                    &item.definition.revision_ref,
                )
                .ok_or(MovementError::NotQualified)?;
            if policy.source_digest() != runtime.content_pin().server_artifact_digest() {
                return Err(MovementError::NotQualified);
            }
            if policy
                .record()
                .attributes
                .has_height
                .ok_or(MovementError::NotQualified)?
            {
                height_count = height_count
                    .checked_add(1)
                    .ok_or(MovementError::CapacityExceeded)?;
            }
        }
        tiles.insert(
            (position.x, position.y, position.floor),
            (tile, solid, height_count),
        );
    }
    Err(MovementError::CapacityExceeded)
}

pub(crate) async fn bind_current_source_step<'owner>(
    tx: &'owner mut sqlx::Transaction<'_, sqlx::Postgres>,
    runtime: &ChannelRuntimeV1,
    prepared: PreparedCurrentSourceStep<'owner>,
) -> Result<SourceStepProof<'owner>, MovementError> {
    let current: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| MovementError::NotQualified)?;
    if current != prepared.transaction_id {
        return Err(MovementError::NotQualified);
    }
    prepared.validate_current(runtime)?;
    Ok(SourceStepProof {
        cells: prepared.cells,
        actor: prepared.actor,
        session: prepared.session,
        expected: prepared.expected,
        direction: prepared.direction,
        destination: prepared.destination,
        current_tiles: Some(prepared.tiles),
        _transaction: std::marker::PhantomData,
    })
}
