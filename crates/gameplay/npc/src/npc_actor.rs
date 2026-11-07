//! NPC-BEHAVIOUR0 §4–5: source-bound walking preparation for a native NPC actor.
//!
//! The Channel's budgeted NPC think must call this once per native think occurrence. This
//! module owns only the interval and deterministic proposal; the actor's position remains in
//! Movement. A proposal does not prove catalogue membership in the active runtime, current
//! field/trigger absence, or permission to commit a step. Those owner handoffs are not installed.

use std::collections::BTreeMap;
use std::time::Duration;

use crate::content::{
    CollisionClass, LogicalCell, NpcDataCatalogue, NpcServiceModel, ProjectV2AuthoringProfileData,
    ProjectV2BehaviorAuthoring, ProjectV2CandidateField, ProjectV2CandidateValue,
    ProjectV2Declaration, QualifiedNativeEntryRoom,
};
use crate::foundation::ThinkOccurrence;
use crate::foundation::{
    ChannelRuntimeV1, MovementFacing, MovementLocalPosition, MovementPositionSnapshot,
};
use crate::movement::speed::{StepSpeedTable, effective_speed, qualified_ground_speed};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

pub const BASE_SPEED: &str = "oteryn:source.npc.base_speed";
pub const FLOOR_CHANGE: &str = "oteryn:source.npc.floor_change";
const PURPOSE: &str = "NPC_WANDER";
// Rejection is extraordinarily unlikely for the only bounds (2, 3, 4). A bounded refusal
// nevertheless keeps a malformed/unavailable decision source inside the owner think budget.
const MAX_DRAWS_PER_CHOICE: u64 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkError {
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
pub struct WalkProfile {
    pub interval_ms: u64,
    pub radius_tiles: u16,
    pub base_speed: u16,
}

impl WalkProfile {
    pub fn from_source(
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

#[derive(Debug)]
pub enum WalkThink {
    Idle,
    Talking,
    Waiting,
    Propose(Box<NpcWalkProposal>),
    /// No second interval advance or proposal for the same native occurrence.
    Replay,
}

impl WalkProfile {
    /// Prepare the interval and proposal from current owner observations. The caller keeps
    /// its conversation lock held and advances counters only after successful preparation.
    #[allow(
        clippy::too_many_arguments,
        reason = "one native think and its owner observations"
    )]
    pub fn prepare(
        self,
        occurrence: ThinkOccurrence,
        expected: MovementPositionSnapshot,
        anchor: MovementLocalPosition,
        ticks_ms: u64,
        talking: bool,
        perceived: bool,
        interval_millis: u64,
        root: &GameplayDecisionRoot,
        decision: DecisionOccurrenceId,
    ) -> Result<(u64, WalkThink), WalkError> {
        Ok(if talking {
            (0, WalkThink::Talking)
        } else if !perceived {
            (ticks_ms, WalkThink::Idle)
        } else {
            let ticks = ticks_ms.saturating_add(interval_millis);
            if ticks < self.interval_ms {
                (ticks, WalkThink::Waiting)
            } else {
                (
                    0,
                    WalkThink::Propose(Box::new(NpcWalkProposal::new(
                        occurrence, expected, anchor, self, root, decision,
                    )?)),
                )
            }
        })
    }
}

/// Candidate source data. Matching this catalogue and model does not bind either to the
/// running bundle; the still-missing catalogue→active-runtime producer must establish that.
#[derive(Debug)]
pub struct NpcWalkBook {
    source_tree_digest: String,
    project_revision: String,
    profiles: BTreeMap<String, WalkProfile>,
}

impl NpcWalkBook {
    pub fn from_catalogue(
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

    pub fn profile(&self, key: &str) -> Option<WalkProfile> {
        self.profiles.get(key).copied()
    }

    pub fn source_tree_digest(&self) -> &str {
        &self.source_tree_digest
    }

    pub fn project_revision(&self) -> &str {
        &self.project_revision
    }
}

/// An ordered proposal from a real Movement snapshot. It cannot mutate Movement. In
/// particular, none of its fields certify current terrain, trigger or content authority.
#[derive(Debug)]
pub struct NpcWalkProposal {
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
pub struct NpcStepCandidate {
    pub destination: MovementLocalPosition,
    pub duration: Duration,
}

impl NpcWalkProposal {
    pub fn new(
        occurrence: ThinkOccurrence,
        expected: MovementPositionSnapshot,
        anchor: MovementLocalPosition,
        profile: WalkProfile,
        root: &GameplayDecisionRoot,
        decision: DecisionOccurrenceId,
    ) -> Result<Self, WalkError> {
        Ok(Self {
            occurrence,
            expected,
            anchor,
            radius_tiles: profile.radius_tiles,
            base_speed: profile.base_speed,
            directions: shuffled_directions(root, decision)?,
        })
    }

    pub const fn occurrence(&self) -> ThinkOccurrence {
        self.occurrence
    }

    pub const fn expected_position(&self) -> MovementPositionSnapshot {
        self.expected
    }

    /// Preserve the one shuffled order, removing only non-native coordinates, radius exits
    /// and genuinely occupied positions. Terrain has not yet been considered by this read.
    pub fn ordered_destinations(
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
    pub fn terrain_candidates(
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

pub fn cell(position: MovementLocalPosition) -> LogicalCell {
    LogicalCell {
        x: position.x,
        y: position.y,
        z: i32::from(position.floor),
    }
}

pub fn cardinal_destination(
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

pub fn shuffled_directions(
    root: &GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
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
            let draw = deterministic_decision_u64(root, occurrence, PURPOSE, draw_index)
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
