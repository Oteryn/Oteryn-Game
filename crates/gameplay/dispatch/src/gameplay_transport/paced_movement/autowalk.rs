//! Item-action walks reuse Movement's bounded cardinal A* and the ordinary paced step.
use super::*;
use crate::foundation::{MovementLocalPosition, MovementPositionSnapshot};
use crate::movement::path_search::{
    CHASE_SEARCH_DISTANCE, PathOccupancy, ROUTE_STEPS, chebyshev, route_next_to,
};
use std::collections::VecDeque;

/// Server-resolved action target; coordinates from map views are converted before searching.
#[derive(Debug, Clone)]
pub enum AutoWalkTarget {
    Item(item_view::ItemKey),
    Object(oteryn_protocol_oteryn::world_object::WorldObjectTarget),
}

/// Ephemeral queue owned by the running connection's Movement pacer, never resumed or persisted.
pub struct AutoWalkRoute {
    pub(in crate::gameplay_transport) expected: MovementPositionSnapshot,
    pub(in crate::gameplay_transport) steps: VecDeque<StepDirection>,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn item_walk_route(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        target: AutoWalkTarget,
    ) -> Result<Option<AutoWalkRoute>, ()> {
        if !self.ensure_source_map_initialized().await {
            return Ok(None);
        }
        // Ground is read without the Channel lock, then all movement/search facts are cut
        // under that lock. The eventual action independently revalidates the source.
        let ground = if let Some(map) = self.bundle_map() {
            let index = {
                let runtime = self.runtime.lock().await;
                item_ref_admission::pinned_index(&runtime, self.active_generation)
            };
            loop_move::loop_ground(self, map.overlay.base(), index).await
        } else {
            None
        };
        let mut runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        if states.is_dead(actor)
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return Ok(None);
        }
        let expected = runtime
            .borrow_movement_position()
            .read(actor)
            .map_err(|_| ())?;
        if expected.context() != runtime.pinned_movement_context() {
            return Ok(None);
        }
        let native = |pos: crate::map::overlay::TilePos| -> MovementLocalPosition {
            MovementLocalPosition {
                x: i32::from(pos.x),
                y: i32::from(pos.y),
                floor: -i16::from(pos.floor),
            }
        };
        let position = match target {
            AutoWalkTarget::Item(item_view::ItemKey::Entity(identity)) => {
                let Some(corpse) = runtime.bound_corpse(identity) else {
                    return Ok(None);
                };
                MovementLocalPosition {
                    x: corpse.position.x,
                    y: corpse.position.y,
                    floor: corpse.position.floor,
                }
            }
            target => {
                if let Some(map) = self.bundle_map() {
                    let facts = map.facts.pinned();
                    let source = world_map::MapViewSource {
                        overlay: ground
                            .as_ref()
                            .map_or(&map.overlay, |ground| ground.overlay()),
                        facts: &facts,
                        content_generation: runtime.content_pin().client_artifact_digest(),
                        reset_epoch: self.reset_epoch,
                    };
                    let pos = match target {
                        AutoWalkTarget::Item(key) => {
                            match world_map::resolve_map_handle(&source, key) {
                                Ok(
                                    world_map::MapHandleTarget::Base { pos, .. }
                                    | world_map::MapHandleTarget::Added { pos, .. }
                                    | world_map::MapHandleTarget::Ground { pos, .. },
                                ) => pos,
                                Err(_) => return Ok(None),
                            }
                        }
                        AutoWalkTarget::Object(target) => {
                            match world_map::resolve_map_target(&source, &target, |_| Some(())) {
                                Ok(target) => target.pos,
                                Err(_) => return Ok(None),
                            }
                        }
                    };
                    native(pos)
                } else if let AutoWalkTarget::Object(target) = target {
                    if let Some(chest) = Self::chest_target(self.chest, &target.placement) {
                        MovementLocalPosition {
                            x: chest.address.cell.x,
                            y: chest.address.cell.y,
                            floor: i16::try_from(chest.address.cell.z).map_err(|_| ())?,
                        }
                    } else if let Some((_, content)) =
                        self.daily_content().filter(|(_, content)| {
                            target.placement == content.entry_shrine.placement.as_bytes()
                        })
                    {
                        MovementLocalPosition {
                            x: content.entry_shrine.x,
                            y: content.entry_shrine.y,
                            floor: content.entry_shrine.floor,
                        }
                    } else {
                        let door = self.door.lock().await;
                        if target.placement != door.placement_key().as_str().as_bytes() {
                            return Ok(None);
                        }
                        let Some(cell) = door.collision_cells().iter().min_by_key(|cell| {
                            i64::from(cell.x).abs_diff(i64::from(expected.position().x))
                                + i64::from(cell.y).abs_diff(i64::from(expected.position().y))
                        }) else {
                            return Ok(None);
                        };
                        MovementLocalPosition {
                            x: cell.x,
                            y: cell.y,
                            floor: i16::try_from(cell.z).map_err(|_| ())?,
                        }
                    }
                } else {
                    return Ok(None);
                }
            }
        };
        // A move may be TooFar because of its destination. Only a distant source starts a
        // walk; in particular, do not step away from an item under the player's own feet.
        if expected.position().floor == position.floor
            && chebyshev(expected.position(), position) <= 1
        {
            return Ok(None);
        }
        let (none, legacy, bundle_doors);
        let blocking: &dyn crate::map::floor::CellBlocker = if self
            .rooms
            .gate_source_world()
            .is_some_and(|room| room.source_world().is_some())
        {
            none = std::collections::BTreeSet::<crate::content::LogicalCell>::new();
            &none
        } else if let Some(map) = self.bundle_map() {
            bundle_doors = crate::map::floor::BundleDoors(map.facts.doors());
            &bundle_doors
        } else {
            legacy = self.door.lock().await.blocking_cells().clone();
            &legacy
        };
        let pvp_blocking = pvp_live::PvpBlocker {
            inner: blocking,
            cells: self.movement_cells,
            blocked: pvp_live::pz_blocked(&states, actor, session, self.owner_now()),
        };
        let occupancy = PathOccupancy::current(&runtime);
        let steps = route_next_to(expected.position(), position, CHASE_SEARCH_DISTANCE, |at| {
            use crate::map::floor::CellBlocker as _;
            let cell = crate::content::LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            };
            let cells = self.movement_cells;
            // Bundle spell facts deliberately become unknown over movable items and doors.
            // Movement instead owns complete floor-change facts in the booted floor table.
            let ordinary = if let Some(floors) = crate::map::floor::table_of(cells) {
                crate::content::bundle_spell_tiles::tile_pos(cell)
                    .is_some_and(|pos| !floors.contains(pos))
            } else {
                match cells.spell_tiles().lookup(cells.scope(), cell) {
                    Ok(tile) => !tile.flags().floor_change,
                    Err(crate::content::SpellTileLookupError::Unknown) => {
                        let entry = matches!(
                            cells.index(),
                            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Entry(
                                _
                            )
                        );
                        entry && !crate::movement::source_floor_change::is_source_profile(cells)
                    }
                    Err(_) => false,
                }
            };
            (ordinary
                && cells.index().lookup(cells.scope(), cell)
                    == Ok(crate::content::CollisionClass::Walkable)
                && !pvp_blocking.blocks(cell)
                && !crate::map::floor::is_teleport(cells, cell))
            .then(|| occupancy.extra_cost(at, expected.position()))
        })
        .ok_or(())?;
        debug_assert!(steps.len() <= ROUTE_STEPS);
        Ok(Some(AutoWalkRoute {
            expected,
            steps: steps
                .into_iter()
                .map(|step| match step {
                    crate::movement::CardinalStep::North => StepDirection::North,
                    crate::movement::CardinalStep::East => StepDirection::East,
                    crate::movement::CardinalStep::South => StepDirection::South,
                    crate::movement::CardinalStep::West => StepDirection::West,
                })
                .collect(),
        }))
    }

    pub(in crate::gameplay_transport) async fn item_walk_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        expected: MovementPositionSnapshot,
        direction: StepDirection,
    ) -> (
        StepOutcome,
        Option<Duration>,
        Option<MovementPositionSnapshot>,
    ) {
        let (outcome, duration) = self
            .tutorial_paced_step_from(actor, session, direction, Some(expected))
            .await;
        let mut runtime = self.runtime.lock().await;
        let current = runtime
            .borrow_movement_position()
            .read(actor)
            .ok()
            .filter(|current| {
                let (dx, dy) = match direction {
                    StepDirection::North => (0, -1),
                    StepDirection::East => (1, 0),
                    StepDirection::South => (0, 1),
                    StepDirection::West => (-1, 0),
                };
                expected.revision().checked_add(1) == Some(current.revision())
                    && expected.context() == current.context()
                    && expected.position().x.checked_add(dx) == Some(current.position().x)
                    && expected.position().y.checked_add(dy) == Some(current.position().y)
                    && expected.position().floor == current.position().floor
                    && !self.states.newhaven_travel.held(session)
            });
        (outcome, duration, current)
    }
}
