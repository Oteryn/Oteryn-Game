//! Native STEP while the existing actor is in named House custody.
//! Current session, equipment and floor items are read under their original
//! owners; movement never constructs a Channel actor or substitutes its fence.
use super::*;
use crate::content::{CollisionClass, LogicalCell, NativeEntryMovementCells};
use crate::durability::house_interior::{HouseStepReadRequest, HouseStepSnapshot};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, GameSessionState, MovementFacing,
    MovementLocalPosition,
};
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::gameplay_transport::connection::StepOutcome;
use crate::gameplay_transport::world_spatial::{StepDirection, StepDisposition};
use crate::map::floor::CellBlocker;
use crate::movement::speed::{
    EngineeringGroundSpeed, QualifiedCellGroundSpeed, player_step_duration,
};
use crate::movement::{CardinalStep, MovementError};
use oteryn_gameplay_entities::carrier::house_custody::HouseMovementSnapshot;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

type Reply = (StepOutcome, Option<Duration>);
type ItemCells = BTreeMap<LogicalCell, (bool, u16)>;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn house_paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
    ) -> Reply {
        // An uncertain native food commit must settle before movement reads its
        // SPEED predecessor. The resolver takes and releases this same lane.
        if !crate::gameplay_transport::hireling::resolve_pending_lamp(self, actor, session).await
            || !crate::gameplay_transport::item_use::gourmet::resolve(self, actor, session).await
            || !self.ensure_source_map_initialized().await
        {
            return (StepOutcome::rejected(), None);
        }
        let Some(_lane) = self.spell_lane_permit().await else {
            return (StepOutcome::rejected(), None);
        };
        let Ok((current, _)) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(session)
            .await
        else {
            return (StepOutcome::rejected(), None);
        };
        if current.session_state() != GameSessionState::Active {
            return (StepOutcome::rejected(), None);
        }
        let Some(fence) = crate::gameplay_transport::item_fence_of(current) else {
            return (StepOutcome::rejected(), None);
        };
        let doors = self.door.lock().await.blocking_cells().clone();
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let Ok(expected) = runtime.read_house_movement(actor, session) else {
            return (StepOutcome::rejected(), None);
        };
        let state = expected.state();
        if state.scope != fence.runtime_scope
            || state.generation != fence.scope_ownership_generation
            || state.lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || state.lease.generation() != fence.character_lease_generation
            || states.is_dead(actor)
            || states.has_pending_spell_commit(actor, session)
            || self.movement_cells.scope().world_id != self.world_id
            || self.movement_cells.scope().generation_digest
                != runtime.content_pin().server_artifact_digest()
            || crate::gameplay_transport::item_ref_admission::pinned_index(
                &runtime,
                self.active_generation,
            )
            .is_none()
        {
            return (StepOutcome::rejected(), None);
        }
        let Some(house) = HouseId::new(self.world_id, &state.house) else {
            return (StepOutcome::rejected(), None);
        };
        let request = HouseStepReadRequest {
            fence,
            origin_channel: *self.channel_id.as_bytes(),
            house,
            content_digest: runtime.content_pin().server_artifact_digest(),
        };
        let mut turn = Turn {
            owner: self,
            runtime: &mut runtime,
            states: &mut states,
            actor,
            session,
            fence,
            expected,
            direction,
            doors,
            outcome: (StepOutcome::rejected(), None),
        };
        let _ = self
            .root
            .with_house_step_read(
                self.character,
                self.holder,
                request,
                &mut turn,
                |read, turn| turn.apply(read),
            )
            .await;
        let outcome = turn.outcome;
        // Native publication is known even if SQL cleanup or entry release failed.
        if outcome.0.disposition == StepDisposition::Moved {
            self.states.chat.close_out_of_range(&runtime, session);
        }
        outcome
    }
}

struct Turn<'turn, 'root, 'authority, 'scope> {
    owner: &'turn ComposedFreshAdmission<'root, 'authority, 'scope>,
    runtime: &'turn mut ChannelRuntimeV1,
    states: &'turn mut ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    expected: HouseMovementSnapshot,
    direction: StepDirection,
    doors: BTreeSet<LogicalCell>,
    outcome: Reply,
}

impl Turn<'_, '_, '_, '_> {
    fn apply(&mut self, read: HouseStepSnapshot) -> bool {
        match self.commit(read) {
            Ok((position, duration)) => {
                self.outcome = (
                    StepOutcome {
                        disposition: StepDisposition::Moved,
                        moved_to: Some(ComposedFreshAdmission::observation(self.runtime, position)),
                    },
                    Some(duration),
                )
            }
            Err(
                MovementError::Blocked
                | MovementError::Cell(
                    crate::content::static_cell_engine::StaticCellEngineError::Absent,
                ),
            ) => {
                self.outcome = (
                    StepOutcome {
                        disposition: StepDisposition::Blocked,
                        moved_to: None,
                    },
                    None,
                );
            }
            Err(_) => {}
        }
        self.outcome.0.disposition == StepDisposition::Moved
    }

    fn commit(
        &mut self,
        read: HouseStepSnapshot,
    ) -> Result<(MovementLocalPosition, Duration), MovementError> {
        let failure = MovementError::NotQualified;
        let now = self.owner.owner_now();
        let content = self
            .owner
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .ok_or(failure)?;
        let current = self
            .runtime
            .read_house_movement(self.actor, self.session)
            .map_err(MovementError::Actor)?;
        if current != self.expected
            || read.interior.house.house_key() != current.state().house.as_str()
            || read.interior.ownership_generation != current.state().generation.get()
            || read.equipment.character != *self.fence.character_id.as_bytes()
            || read.equipment.content_digest != self.runtime.content_pin().server_artifact_digest()
            || content.source_digest() != read.equipment.content_digest
            || self
                .states
                .has_pending_spell_commit(self.actor, self.session)
        {
            return Err(MovementError::NotQualified);
        }
        let items = item_cells(&read.interior.items, content).ok_or(MovementError::NotQualified)?;
        // This is the native bounded projection read under the same property
        // lock as placement/return. A delayed transport refresh cannot let a
        // step enter a Hireling cell that is already durably occupied.
        let projections = read
            .hirelings
            .into_iter()
            .map(|hireling| {
                if hireling.house.as_deref() != Some(current.state().house.as_str())
                    || hireling.world != *self.fence.runtime_scope.world_id().as_bytes()
                {
                    return None;
                }
                let (x, y, floor) = hireling.position?;
                Some(crate::foundation::HirelingProjection {
                    key: hireling.npc_key(),
                    name: hireling.name,
                    placement: crate::foundation::NpcPlacement {
                        x,
                        y,
                        floor,
                        direction: MovementFacing::South,
                    },
                    look_type: hireling.look_type,
                    colors: hireling.colors,
                })
            })
            .collect::<Option<Vec<_>>>()
            .ok_or(MovementError::NotQualified)?;
        self.runtime
            .reconcile_house_hirelings(self.actor, self.session, &projections)
            .map_err(|_| MovementError::NotQualified)?;
        let origin = movement(current.state().position);
        let (direction, facing) = cardinal(self.direction);
        let cells = self.owner.movement_cells;
        let blocker = HouseBlocker {
            cells,
            bundle: self.owner.bundle_map(),
            house: &current.state().house,
            items: &items,
            doors: &self.doors,
        };
        let pvp = crate::gameplay_transport::pvp_live::PvpBlocker {
            inner: &blocker,
            cells,
            blocked: crate::gameplay_transport::pvp_live::pz_blocked(
                self.states,
                self.actor,
                self.session,
                now,
            ),
        };
        let (destination, stairhop) = resolve_destination(cells, origin, direction, &pvp, &items)?;
        if pvp.blocks(cell(destination)) {
            return Err(MovementError::Blocked);
        }
        // As in the existing movement owner, creature occupancy is checked on
        // the resolved landing, after static floor/teleport destination queries.
        if self
            .runtime
            .house_cell_occupied_except(
                self.actor,
                self.session,
                LocalPosition {
                    x: destination.x,
                    y: destination.y,
                    floor: destination.floor,
                },
            )
            .map_err(MovementError::Actor)?
        {
            return Err(MovementError::Blocked);
        }
        let equipment =
            crate::gameplay_transport::equipment_move::equipment_effects_owner::speed_term(
                &read.equipment,
                content,
            )
            .ok_or(MovementError::NotQualified)?;
        let delta = self
            .runtime
            .house_actor_conditions_at(self.actor, self.session, now.get())
            .map_err(|_| MovementError::NotQualified)?
            .speed_delta_at(now.get());
        let state = self
            .states
            .house_state_mut(self.runtime, self.actor, self.session, self.fence)
            .ok_or(MovementError::NotQualified)?;
        if !state.owned_conditions().accepts_time(now.get()) {
            return Err(MovementError::NotQualified);
        }
        let speed = crate::spell::actor_conditions::movement_speed_with(
            state,
            now.get(),
            delta,
            i64::from(equipment),
        );
        let duration = if crate::movement::source_floor_change::is_source_profile(cells) {
            player_step_duration(
                &QualifiedCellGroundSpeed { cells },
                cell(destination),
                speed,
            )
        } else {
            // Identical to the existing native bundle/entry STEP owner. The
            // map cutover owns switching this established ground-speed source.
            player_step_duration(&EngineeringGroundSpeed, cell(destination), speed)
        }
        .ok_or(MovementError::NotQualified)?;
        let cooldowns = if stairhop {
            use oteryn_gameplay_spell_rules::cast::{STAIRHOP_COOLDOWN_KEY, STAIRHOP_MICROS};
            let until = now
                .get()
                .checked_add(STAIRHOP_MICROS)
                .ok_or(MovementError::NotQualified)?;
            let mut next = state.cooldowns.clone();
            next.groups.insert(
                STAIRHOP_COOLDOWN_KEY.into(),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(until),
            );
            Some(next)
        } else {
            None
        };
        self.runtime
            .compare_move_house(
                &current,
                LocalPosition {
                    x: destination.x,
                    y: destination.y,
                    floor: destination.floor,
                },
                facing,
            )
            .map_err(MovementError::Actor)?;
        if let Some(cooldowns) = cooldowns {
            state.cooldowns = cooldowns;
        }
        Ok((destination, duration))
    }
}

fn movement(position: LocalPosition) -> MovementLocalPosition {
    MovementLocalPosition {
        x: position.x,
        y: position.y,
        floor: position.floor,
    }
}
fn cell(position: MovementLocalPosition) -> LogicalCell {
    LogicalCell {
        x: position.x,
        y: position.y,
        z: i32::from(position.floor),
    }
}
fn cardinal(direction: StepDirection) -> (CardinalStep, MovementFacing) {
    match direction {
        StepDirection::North => (CardinalStep::North, MovementFacing::North),
        StepDirection::East => (CardinalStep::East, MovementFacing::East),
        StepDirection::South => (CardinalStep::South, MovementFacing::South),
        StepDirection::West => (CardinalStep::West, MovementFacing::West),
    }
}

fn item_cells(
    items: &[crate::durability::house_interior::HouseInteriorLoadedItem],
    content: &crate::content::native_gameplay::NativeGameplayState,
) -> Option<ItemCells> {
    let mut cells = BTreeMap::<LogicalCell, (bool, u16)>::new();
    for item in items {
        if item.definition.family != "Item" || item.spatial_position.len() != 12 {
            return None;
        }
        let part = |index| {
            Some(i32::from_be_bytes(
                item.spatial_position
                    .get(index..index + 4)?
                    .try_into()
                    .ok()?,
            ))
        };
        let position = LogicalCell {
            x: part(0)?,
            y: part(4)?,
            z: part(8)?,
        };
        let policy = content.item_policy(
            &item.definition.production_key,
            &item.definition.revision_ref,
        )?;
        if policy.source_digest() != content.source_digest() {
            return None;
        }
        let attributes = &policy.record().attributes;
        let current = cells.entry(position).or_default();
        current.0 |= attributes.blocks_movement?;
        current.1 = current.1.checked_add(u16::from(attributes.has_height?))?;
    }
    Some(cells)
}

fn resolve_destination(
    cells: &NativeEntryMovementCells,
    origin: MovementLocalPosition,
    direction: CardinalStep,
    blocked: &dyn CellBlocker,
    items: &ItemCells,
) -> Result<(MovementLocalPosition, bool), MovementError> {
    if crate::map::floor::table_of(cells).is_some() {
        return crate::map::floor::resolve_house_step(cells, origin, direction, false, blocked);
    }
    if crate::movement::source_floor_change::is_source_profile(cells) {
        let destination = crate::movement::source_floor_change::resolve_house_step(
            cells,
            origin,
            direction,
            |at| {
                Some((
                    blocked.blocks(cell(at)),
                    items.get(&cell(at)).map_or(0, |item| item.1),
                ))
            },
        )?;
        return Ok((destination, destination.floor != origin.floor));
    }
    let (dx, dy) = crate::domain::direction::CARDINAL_DIRECTION_DELTAS[direction as usize];
    let destination = MovementLocalPosition {
        x: origin
            .x
            .checked_add(dx)
            .ok_or(MovementError::CoordinateOverflow)?,
        y: origin
            .y
            .checked_add(dy)
            .ok_or(MovementError::CoordinateOverflow)?,
        floor: origin.floor,
    };
    if blocked.blocks(cell(destination))
        || cells.index().lookup(cells.scope(), cell(destination)) != Ok(CollisionClass::Walkable)
    {
        return Err(MovementError::Blocked);
    }
    Ok((destination, false))
}

struct HouseBlocker<'a> {
    cells: &'a NativeEntryMovementCells,
    bundle: Option<&'a crate::map::boot::BundleMap>,
    house: &'a str,
    items: &'a ItemCells,
    doors: &'a BTreeSet<LogicalCell>,
}
impl CellBlocker for HouseBlocker<'_> {
    fn blocks(&self, cell: LogicalCell) -> bool {
        if i16::try_from(cell.z).is_err() {
            return true;
        }
        // A genuinely absent source cell must remain absent to the native
        // floor resolver (which may retain the entered tile). House membership
        // fences existing tiles; it never manufactures a solid tile in air.
        let outside = match self.cells.index().lookup(self.cells.scope(), cell) {
            Ok(_) => !is_house_cell(self.cells, self.bundle, self.house, cell),
            Err(crate::content::static_cell_engine::StaticCellEngineError::Absent) => false,
            Err(_) => true,
        };
        let door = self.bundle.map_or_else(
            || self.doors.contains(&cell),
            |bundle| crate::map::floor::BundleDoors(bundle.facts.doors()).blocks(cell),
        );
        outside || door || self.items.get(&cell).is_some_and(|item| item.0)
    }
}

fn is_house_cell(
    cells: &NativeEntryMovementCells,
    bundle: Option<&crate::map::boot::BundleMap>,
    house: &str,
    cell: LogicalCell,
) -> bool {
    if let Some(bundle) = bundle {
        let (Ok(x), Ok(y), Ok(floor)) = (
            u16::try_from(cell.x),
            u16::try_from(cell.y),
            i8::try_from(cell.z),
        ) else {
            return false;
        };
        let Some(floor) = floor.checked_neg() else {
            return false;
        };
        let tile = crate::map::overlay::TilePos { x, y, floor };
        return crate::gameplay_transport::house_query::source_house_id(house)
            .is_some_and(|source| bundle.facts.house_id(tile) == Some(source));
    }
    cells
        .house_tiles()
        .house_at(cells.scope(), cell)
        .is_ok_and(|placement| placement.is_some_and(|placement| placement.house().key == house))
}

/// Current House resume placement, including reconnectable residents. The caller
/// invokes this outside its own lane/runtime locks before restoring control.
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn validate_house_resume(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        if !self.ensure_source_map_initialized().await {
            return false;
        }
        let Some(_lane) = self.spell_lane_permit().await else {
            return false;
        };
        let Ok((current, _)) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(session)
            .await
        else {
            return false;
        };
        if !matches!(
            current.session_state(),
            GameSessionState::Active | GameSessionState::Reconnectable
        ) {
            return false;
        }
        let Some(fence) = crate::gameplay_transport::item_fence_of(current) else {
            return false;
        };
        let runtime = self.runtime.lock().await;
        let Ok(state) = runtime.house_actor_state(actor, session) else {
            return false;
        };
        if state.scope != fence.runtime_scope
            || state.generation != fence.scope_ownership_generation
            || state.lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || state.lease.generation() != fence.character_lease_generation
        {
            return false;
        }
        let Some(house) = HouseId::new(self.world_id, &state.house) else {
            return false;
        };
        let doors = self.door.lock().await.blocking_cells().clone();
        let record = self
            .placed_record(&runtime, movement(state.position))
            .record;
        let mut context = PlacementCheck {
            owner: self,
            runtime: &runtime,
            candidate: &state,
            record: &record,
            doors: &doors,
            except: Some(actor),
            retain_hireling_overlap: true,
            valid: false,
        };
        let request = HouseStepReadRequest {
            fence,
            origin_channel: *self.channel_id.as_bytes(),
            house,
            content_digest: runtime.content_pin().server_artifact_digest(),
        };
        let result = self
            .root
            .with_house_step_read(
                self.character,
                self.holder,
                request,
                &mut context,
                |read, context| {
                    context.valid =
                        context.validates(crate::durability::house_transfer::HouseAdmissionRead {
                            interior: read.interior,
                            hirelings: read.hirelings,
                        });
                    false
                },
            )
            .await;
        result.is_ok() && context.valid
    }
}

pub(super) struct PlacementCheck<'a, 'root, 'authority, 'scope> {
    pub owner: &'a ComposedFreshAdmission<'root, 'authority, 'scope>,
    pub runtime: &'a ChannelRuntimeV1,
    pub candidate: &'a oteryn_gameplay_entities::carrier::house_custody::HouseActorState,
    pub record: &'a LastPositionRecord,
    pub doors: &'a BTreeSet<LogicalCell>,
    pub except: Option<ExactActorRef>,
    pub retain_hireling_overlap: bool,
    pub valid: bool,
}
impl PlacementCheck<'_, '_, '_, '_> {
    pub(super) fn validates(
        &self,
        read: crate::durability::house_transfer::HouseAdmissionRead,
    ) -> bool {
        let Some(content) = self
            .owner
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return false;
        };
        let pin = self.runtime.content_pin();
        if self.record.world_id != self.owner.world_id
            || self.record.coordinate_frame_digest != pin.frame_binding_digest()
            || self.record.server_artifact_digest != pin.server_artifact_digest()
            || self.record.map_revision_digest != self.runtime.map_revision_digest()
            || self.owner.movement_cells.scope().generation_digest != pin.server_artifact_digest()
            || content.source_digest() != pin.server_artifact_digest()
            || read.interior.house.house_key() != self.candidate.house
            || read.interior.house.runtime_scope().ok() != Some(self.candidate.scope)
            || read.interior.ownership_generation != self.candidate.generation.get()
        {
            return false;
        }
        let Some(items) = item_cells(&read.interior.items, content) else {
            return false;
        };
        let at = cell(movement(self.candidate.position));
        let cells = self.owner.movement_cells;
        let blocker = HouseBlocker {
            cells,
            bundle: self.owner.bundle_map(),
            house: &self.candidate.house,
            items: &items,
            doors: self.doors,
        };
        if cells.index().lookup(cells.scope(), at) != Ok(CollisionClass::Walkable)
            || blocker.blocks(at)
            || !is_house_cell(cells, self.owner.bundle_map(), &self.candidate.house, at)
            || !self
                .runtime
                .house_placement_clear(self.candidate, self.except)
        {
            return false;
        }
        for hireling in read.hirelings {
            if hireling.house.as_deref() != Some(self.candidate.house.as_str())
                || hireling.world != *self.owner.world_id.as_bytes()
            {
                return false;
            }
            let Some((x, y, floor)) = hireling.position else {
                return false;
            };
            if !self.retain_hireling_overlap
                && (i32::from(x), i32::from(y), -i16::from(floor))
                    == (at.x, at.y, self.candidate.position.floor)
            {
                return false;
            }
        }
        true
    }
}
