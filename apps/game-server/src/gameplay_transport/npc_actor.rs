//! NPC-BEHAVIOUR0 §4–5: source-bound walking preparation for a native NPC actor.
//!
//! The Channel's budgeted NPC think must call this once per native think occurrence. This
//! module owns only the interval and deterministic proposal; the actor's position remains in
//! Movement. A proposal does not prove catalogue membership in the active runtime, current
//! field/trigger absence, or permission to commit a step. Those owner handoffs are not installed.

#![allow(
    dead_code,
    reason = "NPC walking preparation awaits the source pin and budgeted owner caller"
)]

use std::collections::BTreeMap;
use std::time::Duration;

use crate::ai_think::profile_schedule::decision_occurrence;
use crate::ai_think::targeting::sees;
use crate::ai_think::{CREATURE_THINK_INTERVAL_MILLIS, ThinkOccurrence};
use crate::content::{
    CollisionClass, LogicalCell, NpcDataCatalogue, NpcServiceModel, ProjectV2AuthoringProfileData,
    ProjectV2BehaviorAuthoring, ProjectV2CandidateField, ProjectV2CandidateValue,
    ProjectV2Declaration, QualifiedNativeEntryRoom,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementFacing, MovementLocalPosition,
    MovementPositionContext, MovementPositionSnapshot,
};
use crate::movement::speed::{StepSpeedTable, effective_speed, qualified_ground_speed};
use oteryn_simulation_determinism::{GameplayDecisionRoot, deterministic_decision_u64};

use super::chat_intent::ChatRuntime;

const BASE_SPEED: &str = "oteryn:source.npc.base_speed";
const FLOOR_CHANGE: &str = "oteryn:source.npc.floor_change";
const PURPOSE: &str = "NPC_WANDER";
// Rejection is extraordinarily unlikely for the only bounds (2, 3, 4). A bounded refusal
// nevertheless keeps a malformed/unavailable decision source inside the owner think budget.
const MAX_DRAWS_PER_CHOICE: u64 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WalkError {
    ContentSourceMismatch,
    NotCurrentNpc,
    InitialPlacementUnavailable,
    OccurrenceSuperseded,
    ConversationUnavailable,
    PositionChanged,
    TerrainUnavailable,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WalkProfile {
    interval_ms: u64,
    radius_tiles: u16,
    base_speed: u16,
}

impl WalkProfile {
    fn from_source(
        fields: &[ProjectV2CandidateField],
        behavior: &ProjectV2BehaviorAuthoring,
    ) -> Option<Self> {
        let wander = behavior.movement.wander?;
        if !behavior.movement.can_walk || wander.interval_ms == 0 {
            return None;
        }
        // These exact typed fields are emitted by NPC-CONTENT-2. There is no runtime default
        // for an NPC without a source, a malformed field, or an unsupported floor-change mode.
        let mut speed = fields.iter().filter(|field| field.field_path == BASE_SPEED);
        let ProjectV2CandidateValue::Integer(base_speed) = &speed.next()?.value else {
            return None;
        };
        let base_speed = u16::try_from(*base_speed)
            .ok()
            .filter(|speed| *speed != 0)?;
        if speed.next().is_some() {
            return None;
        }
        let mut floor = fields
            .iter()
            .filter(|field| field.field_path == FLOOR_CHANGE);
        if floor.next()?.value != ProjectV2CandidateValue::Boolean(false) || floor.next().is_some()
        {
            return None;
        }
        Some(Self {
            interval_ms: wander.interval_ms,
            radius_tiles: wander.radius_tiles,
            base_speed,
        })
    }
}

/// Candidate source data. Matching this catalogue and model does not bind either to the
/// running bundle; the still-missing catalogue→active-runtime producer must establish that.
#[derive(Debug)]
pub(crate) struct NpcWalkBook {
    source_tree_digest: String,
    project_revision: String,
    profiles: BTreeMap<String, WalkProfile>,
}

impl NpcWalkBook {
    pub(crate) fn from_catalogue(
        catalogue: &NpcDataCatalogue,
        model: &NpcServiceModel,
    ) -> Result<Self, WalkError> {
        if catalogue.source_tree_digest() != model.source_tree_digest()
            || catalogue.project_revision() != model.project_revision()
        {
            return Err(WalkError::ContentSourceMismatch);
        }
        let mut profiles = BTreeMap::new();
        // Only the model's admitted NPCs participate. Resolve its exact NPC and Behavior
        // references, including revision, through the native catalogue rather than key joins.
        for reference in model.npcs().keys() {
            let Some(ProjectV2Declaration::Npc {
                behavior: Some(behavior),
                fields,
                ..
            }) = catalogue.declaration(reference)
            else {
                continue;
            };
            let Some(profile) = catalogue.profile(behavior) else {
                continue;
            };
            let ProjectV2AuthoringProfileData::Behavior(behavior) = &profile.data else {
                continue;
            };
            if let Some(profile) = WalkProfile::from_source(fields, behavior)
                && profiles.insert(reference.key.clone(), profile).is_some()
            {
                return Err(WalkError::ContentSourceMismatch);
            }
        }
        Ok(Self {
            source_tree_digest: catalogue.source_tree_digest().to_owned(),
            project_revision: catalogue.project_revision().to_owned(),
            profiles,
        })
    }

    pub(crate) fn source_tree_digest(&self) -> &str {
        &self.source_tree_digest
    }

    pub(crate) fn project_revision(&self) -> &str {
        &self.project_revision
    }
}

#[derive(Debug)]
pub(crate) enum WalkThink {
    Idle,
    Talking,
    Waiting,
    Propose(NpcWalkProposal),
    /// No second interval advance or proposal for the same native occurrence.
    Replay,
}

/// Ephemeral interval state for one exact NPC. `anchor` is the immutable, real initial
/// placement; this type never stores or replaces the actor's mutable current position.
#[derive(Debug)]
pub(crate) struct NpcWalkState {
    actor: ExactActorRef,
    key: String,
    context: MovementPositionContext,
    anchor: MovementLocalPosition,
    profile: WalkProfile,
    ticks_ms: u64,
    last_sequence: Option<u64>,
}

impl NpcWalkState {
    /// Construct at native placement, before the first move. Recreating this state from a
    /// later position would silently move the radius anchor and is therefore refused.
    pub(crate) fn new(
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        book: &NpcWalkBook,
    ) -> Result<Option<Self>, WalkError> {
        runtime
            .owner_fence()
            .map_err(|_| WalkError::NotCurrentNpc)?;
        let key = runtime
            .npc_key(actor)
            .ok_or(WalkError::NotCurrentNpc)?
            .to_owned();
        let Some(profile) = book.profiles.get(&key) else {
            return Ok(None);
        };
        let snapshot = runtime
            .borrow_movement_position()
            .read(actor)
            .map_err(|_| WalkError::NotCurrentNpc)?;
        if snapshot.revision() != 1 {
            return Err(WalkError::InitialPlacementUnavailable);
        }
        Ok(Some(Self {
            actor,
            key,
            context: snapshot.context(),
            anchor: snapshot.position(),
            profile: *profile,
            ticks_ms: 0,
            last_sequence: None,
        }))
    }

    /// Consume one scheduled NPC think. Active thinks add exactly 1,000 ms, idle thinks do
    /// not accumulate time, and a real customer resets the interval. There is no wall-clock
    /// catch-up or alternate timer queue. The caller still owns native occurrence/timer and
    /// source admission; accepting their data here is not a proof of those authorities.
    pub(crate) fn prepare(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        chat: &ChatRuntime,
        occurrence: ThinkOccurrence,
        root: &GameplayDecisionRoot,
    ) -> Result<WalkThink, WalkError> {
        runtime
            .owner_fence()
            .map_err(|_| WalkError::NotCurrentNpc)?;
        if occurrence.actor != self.actor
            || runtime.pinned_movement_context() != self.context
            || runtime.npc_key(self.actor) != Some(self.key.as_str())
        {
            return Err(WalkError::NotCurrentNpc);
        }
        if let Some(last) = self.last_sequence {
            if occurrence.sequence < last {
                return Err(WalkError::OccurrenceSuperseded);
            }
            if occurrence.sequence == last {
                return Ok(WalkThink::Replay);
            }
        }
        let expected = runtime
            .borrow_movement_position()
            .read(self.actor)
            .map_err(|_| WalkError::NotCurrentNpc)?;
        let perceived = runtime
            .visible_entities()
            .players
            .iter()
            .any(|player| sees(player.position, expected.position()));
        // Read the supplied ChatRuntime under its actual lock, held through preparation.
        // The composing Channel owner must supply its own ChatRuntime: this type itself does
        // not certify a Channel binding. None means unavailable, never an empty queue.
        chat.with_npc_customer_state(self.actor, |talking| {
            let (next_ticks, result) = if talking {
                (0, WalkThink::Talking)
            } else if !perceived {
                (self.ticks_ms, WalkThink::Idle)
            } else {
                let ticks = self.ticks_ms.saturating_add(CREATURE_THINK_INTERVAL_MILLIS);
                if ticks < self.profile.interval_ms {
                    (ticks, WalkThink::Waiting)
                } else {
                    let directions = shuffled_directions(root, occurrence)?;
                    (
                        0,
                        WalkThink::Propose(NpcWalkProposal {
                            occurrence,
                            expected,
                            anchor: self.anchor,
                            radius_tiles: self.profile.radius_tiles,
                            base_speed: self.profile.base_speed,
                            directions,
                        }),
                    )
                }
            };
            // A failed decision leaves both counters unchanged, allowing the same occurrence
            // to be retried deterministically. A successful preparation is never replayed.
            self.ticks_ms = next_ticks;
            self.last_sequence = Some(occurrence.sequence);
            Ok(result)
        })
        .ok_or(WalkError::ConversationUnavailable)?
    }
}

/// An ordered proposal from a real Movement snapshot. It cannot mutate Movement. In
/// particular, none of its fields certify current terrain, trigger or content authority.
#[derive(Debug)]
pub(crate) struct NpcWalkProposal {
    occurrence: ThinkOccurrence,
    expected: MovementPositionSnapshot,
    anchor: MovementLocalPosition,
    radius_tiles: u16,
    base_speed: u16,
    directions: [MovementFacing; 4],
}

/// A native static terrain observation with the real SPEED-1 duration. Current ground items,
/// damaging fields and ON_ENTER/ON_LEAVE owners must still admit the same candidate; callers
/// must not treat this data as a successful step or a reusable movement authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NpcStepCandidate {
    pub(crate) destination: MovementLocalPosition,
    pub(crate) duration: Duration,
}

impl NpcWalkProposal {
    pub(crate) const fn occurrence(&self) -> ThinkOccurrence {
        self.occurrence
    }

    pub(crate) const fn expected_position(&self) -> MovementPositionSnapshot {
        self.expected
    }

    /// Preserve the one shuffled order, removing only non-native coordinates, radius exits
    /// and genuinely occupied positions. Terrain has not yet been considered by this read.
    pub(crate) fn ordered_destinations(
        &self,
        runtime: &mut ChannelRuntimeV1,
    ) -> Result<Vec<MovementLocalPosition>, WalkError> {
        runtime
            .owner_fence()
            .map_err(|_| WalkError::NotCurrentNpc)?;
        if runtime.npc_key(self.occurrence.actor).is_none() {
            return Err(WalkError::NotCurrentNpc);
        }
        let current = runtime
            .borrow_movement_position()
            .read(self.occurrence.actor)
            .map_err(|_| WalkError::NotCurrentNpc)?;
        if current != self.expected {
            return Err(WalkError::PositionChanged);
        }
        let mut positions = Vec::with_capacity(4);
        for direction in self.directions {
            let Some(next) = cardinal_destination(current.position(), direction) else {
                continue;
            };
            if next.floor != self.anchor.floor
                || next.x.abs_diff(self.anchor.x) > u32::from(self.radius_tiles)
                || next.y.abs_diff(self.anchor.y) > u32::from(self.radius_tiles)
                || runtime
                    .position_occupied_by_other(self.occurrence.actor, next)
                    .map_err(|_| WalkError::NotCurrentNpc)?
            {
                continue;
            }
            positions.push(next);
        }
        Ok(positions)
    }

    /// Partial native terrain preparation, in the original shuffled order. Full room digests
    /// must match the running Channel. Absent metadata never becomes a default empty tile;
    /// notably the current bundle has unknown height, so it cannot yield a candidate here.
    /// This does not replace the missing current field/trigger or active-NPC-source handoff.
    pub(crate) fn terrain_candidates(
        &self,
        runtime: &mut ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
    ) -> Result<Vec<NpcStepCandidate>, WalkError> {
        let destinations = self.ordered_destinations(runtime)?;
        let pin = runtime.content_pin();
        if room.movement_cells().scope().world_id != self.expected.world_id()
            || room.compiled().server_digest() != pin.server_artifact_digest()
            || room.compiled().client_digest() != pin.client_artifact_digest()
            || room.frame_binding().digest() != pin.frame_binding_digest()
            || room.map_revision_digest() != pin.map_revision_digest()
        {
            return Err(WalkError::ContentSourceMismatch);
        }
        let cells = room.movement_cells();
        let anchor = cells
            .spell_tiles()
            .lookup(cells.scope(), cell(self.anchor))
            .map_err(|_| WalkError::TerrainUnavailable)?;
        let mut candidates = Vec::with_capacity(4);
        for destination in destinations {
            let next = cell(destination);
            if cells.index().lookup(cells.scope(), next) != Ok(CollisionClass::Walkable) {
                continue;
            }
            let Ok(tile) = cells.spell_tiles().lookup(cells.scope(), next) else {
                continue;
            };
            let flags = tile.flags();
            let Some(step) = tile.source_step() else {
                continue;
            };
            if flags.block_solid
                || flags.floor_change
                || flags.protection_zone != anchor.flags().protection_zone
                || step.height_count != 0
                || !step.floor_changes.is_empty()
                || crate::map::floor::is_teleport(cells, next)
            {
                continue;
            }
            let Some(duration) = StepSpeedTable::embedded().and_then(|table| {
                table.step_duration(
                    effective_speed(i64::from(self.base_speed), 0, 0),
                    qualified_ground_speed(tile.ground_present(), tile.ground_speed()),
                )
            }) else {
                continue;
            };
            candidates.push(NpcStepCandidate {
                destination,
                duration,
            });
        }
        Ok(candidates)
    }
}

fn cell(position: MovementLocalPosition) -> LogicalCell {
    LogicalCell {
        x: position.x,
        y: position.y,
        z: i32::from(position.floor),
    }
}

fn cardinal_destination(
    from: MovementLocalPosition,
    direction: MovementFacing,
) -> Option<MovementLocalPosition> {
    let mut next = from;
    match direction {
        MovementFacing::North => next.y = from.y.checked_sub(1)?,
        MovementFacing::West => next.x = from.x.checked_sub(1)?,
        MovementFacing::East => next.x = from.x.checked_add(1)?,
        MovementFacing::South => next.y = from.y.checked_add(1)?,
    }
    u16::try_from(next.x).ok()?;
    u16::try_from(next.y).ok()?;
    (0..=15).contains(&next.floor).then_some(next)
}

fn shuffled_directions(
    root: &GameplayDecisionRoot,
    occurrence: ThinkOccurrence,
) -> Result<[MovementFacing; 4], WalkError> {
    let mut directions = [
        MovementFacing::North,
        MovementFacing::West,
        MovementFacing::East,
        MovementFacing::South,
    ];
    let mut draw_index = 0;
    // Fisher–Yates with bounded rejection sampling; each draw has a distinct index within
    // the native exact-actor/sequence occurrence and the NPC_WANDER purpose.
    for end in (1..directions.len()).rev() {
        let upper = u64::try_from(end + 1).map_err(|_| WalkError::Draw)?;
        let threshold = upper.wrapping_neg() % upper;
        let mut selected = None;
        for _ in 0..MAX_DRAWS_PER_CHOICE {
            let draw = deterministic_decision_u64(
                root,
                decision_occurrence(occurrence),
                PURPOSE,
                draw_index,
            )
            .map_err(|_| WalkError::Draw)?;
            draw_index += 1; // At most 3 × 32 draws.
            if draw >= threshold {
                selected = Some(usize::try_from(draw % upper).map_err(|_| WalkError::Draw)?);
                break;
            }
        }
        directions.swap(end, selected.ok_or(WalkError::Draw)?);
    }
    Ok(directions)
}

#[cfg(test)]
#[path = "npc_actor_tests.rs"]
mod tests;
