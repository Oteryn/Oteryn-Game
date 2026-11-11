//! Original physical33949 callbacks retained at the actual position commit.
//! The Channel owns the queue independently of this transaction/map future.
use super::*;
use crate::durability::creature_source_items::{
    PreparedSourceCreatureItem, SourceCreatureGroundStep, SourceCreatureGroundTile,
    SourceGroundStep, SourceGroundStepKind,
};
use crate::foundation::SourceActorReservation;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

/// Opaque allocation identity of the actual retained parent pass. It carries
/// no clock or actor grant, and is never constructed from a digest or census.
pub type SourceCreatureStepParent = Arc<dyn std::any::Any + Send + Sync>;

#[derive(Default)]
pub(super) struct SourceStepTiles {
    binding: Option<(Arc<SourceCreatureStepQueue>, u64)>,
    parent: Option<SourceCreatureStepParent>,
    tiles: BTreeMap<(i32, i32, i16), Arc<SourceCreatureGroundTile>>,
}

#[derive(Default)]
pub struct SourceCreatureStepQueue {
    pending: Mutex<VecDeque<PendingSourceCreatureStep>>,
}
impl std::fmt::Debug for SourceCreatureStepQueue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SourceCreatureStepQueue")
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct PendingSourceCreatureStep {
    proof: Arc<SourceCreatureGroundStep>,
    reservation: SourceActorReservation,
    inputs: Arc<Vec<SourceGroundStep>>,
    cursor: usize,
    removed: bool,
    cells: NativeEntryMovementCells,
    following: Option<special_movement::PreparedStepIn>,
    parent: Option<SourceCreatureStepParent>,
}
impl std::fmt::Debug for PendingSourceCreatureStep {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PendingSourceCreatureStep")
            .field("binding", &self.proof.binding())
            .field("cursor", &self.cursor)
            .finish_non_exhaustive()
    }
}
impl PendingSourceCreatureStep {
    pub fn input(&self) -> Option<&SourceGroundStep> {
        self.inputs.get(self.cursor)
    }
}

pub(super) struct PreparedSourceCreatureStep<'a> {
    queue: MutexGuard<'a, VecDeque<PendingSourceCreatureStep>>,
    proof: Arc<SourceCreatureGroundStep>,
    reservation: SourceActorReservation,
    inputs: Arc<Vec<SourceGroundStep>>,
    templates: Vec<PreparedSourceCreatureItem>,
    cells: NativeEntryMovementCells,
    following: Option<special_movement::PreparedStepIn>,
    parent: Option<SourceCreatureStepParent>,
}

impl SourceCreatureStepQueue {
    pub fn front(&self) -> Result<Option<PendingSourceCreatureStep>, CarrierError> {
        self.pending
            .lock()
            .map(|queue| queue.front().cloned())
            .map_err(|_| CarrierError::PlanConflict)
    }
    /// Revalidate the same original front across an older source family's
    /// awaited DATA read. An equal digest or a replacement cursor is not the
    /// retained continuation that established that family's cutoff.
    pub fn retains_head(
        &self,
        runtime: &ChannelRuntimeV1,
        expected: &PendingSourceCreatureStep,
    ) -> Result<bool, CarrierError> {
        let queue = self
            .pending
            .lock()
            .map_err(|_| CarrierError::PlanConflict)?;
        let Some(pending) = queue.front() else {
            return Ok(false);
        };
        if !Arc::ptr_eq(&pending.proof, &expected.proof) || pending.cursor != expected.cursor {
            return Ok(false);
        }
        runtime
            .validate_scope_creature_reservation(&pending.reservation, pending.proof.as_ref())?;
        Ok(true)
    }
    pub fn pending_items(&self) -> Result<usize, CarrierError> {
        self.pending
            .lock()
            .map_err(|_| CarrierError::PlanConflict)?
            .iter()
            .try_fold(0usize, |count, pending| {
                count
                    .checked_add(pending.inputs.len().saturating_sub(pending.cursor))
                    .ok_or(CarrierError::CapacityArithmeticOverflow)
            })
    }
    pub fn is_pending(&self) -> bool {
        self.pending.lock().map_or(true, |queue| !queue.is_empty())
    }
    /// Original movement time for the shared native-work arbiter. A new STEP
    /// may not overtake an older retained timer or copied SourceDeath cursor.
    pub fn next_due_us(&self, runtime: &ChannelRuntimeV1) -> Result<Option<u64>, CarrierError> {
        let queue = self
            .pending
            .lock()
            .map_err(|_| CarrierError::PlanConflict)?;
        let Some(pending) = queue.front() else {
            return Ok(None);
        };
        runtime
            .validate_scope_creature_reservation(&pending.reservation, pending.proof.as_ref())?;
        pending
            .proof
            .observed_us()
            .map(Some)
            .ok_or(CarrierError::PlanConflict)
    }
    pub fn is_continuation_of<P: std::any::Any + Send + Sync>(
        &self,
        runtime: &ChannelRuntimeV1,
        parent: &Arc<P>,
    ) -> Result<bool, CarrierError> {
        let parent: SourceCreatureStepParent = parent.clone();
        self.continues_same_pass(runtime, &parent)
    }
    pub fn continues_same_pass(
        &self,
        runtime: &ChannelRuntimeV1,
        parent: &SourceCreatureStepParent,
    ) -> Result<bool, CarrierError> {
        let queue = self
            .pending
            .lock()
            .map_err(|_| CarrierError::PlanConflict)?;
        let Some(pending) = queue.front() else {
            return Ok(false);
        };
        runtime
            .validate_scope_creature_reservation(&pending.reservation, pending.proof.as_ref())?;
        Ok(pending
            .parent
            .as_ref()
            .is_some_and(|original| Arc::ptr_eq(original, parent)))
    }

    fn prepare<'a>(
        &'a self,
        runtime: &ChannelRuntimeV1,
        cells: &NativeEntryMovementCells,
        tile: Arc<SourceCreatureGroundTile>,
        actor: ExactActorRef,
        before: MovementPositionSnapshot,
        parent: Option<SourceCreatureStepParent>,
    ) -> Result<PreparedSourceCreatureStep<'a>, MovementError> {
        let mut queue = self
            .pending
            .lock()
            .map_err(|_| MovementError::NotQualified)?;
        let proof = SourceCreatureGroundStep::prepare(runtime, tile, actor, before)
            .ok_or(MovementError::NotQualified)?;
        if !queue.is_empty() {
            return Err(MovementError::NotQualified);
        }
        queue
            .try_reserve(1)
            .map_err(|_| MovementError::NotQualified)?;
        let mut inputs = Vec::new();
        inputs
            .try_reserve_exact(proof.item_count())
            .map_err(|_| MovementError::NotQualified)?;
        let templates = proof.prepare_item_inputs();
        let following = (!proof.stops_dispatch())
            .then(|| special_movement::prepare_step_in(runtime, cells, actor, proof.position()))
            .flatten();
        let reservation = runtime
            .prepare_scope_creature_reservation(&proof)
            .map_err(MovementError::Actor)?;
        Ok(PreparedSourceCreatureStep {
            queue,
            proof: Arc::new(proof),
            reservation,
            inputs: Arc::new(inputs),
            templates,
            cells: cells.clone(),
            following,
            parent,
        })
    }

    /// Called only after the same original physical Item writer returned a
    /// committed/reconciled result. Unknown outcomes retain queue and slot.
    pub fn publish_known_item_outcome(
        &self,
        runtime: &mut ChannelRuntimeV1,
        expected: &PendingSourceCreatureStep,
        removed: bool,
    ) -> Result<(), CarrierError> {
        let mut queue = self
            .pending
            .lock()
            .map_err(|_| CarrierError::PlanConflict)?;
        let pending = queue.front_mut().ok_or(CarrierError::PlanConflict)?;
        if !Arc::ptr_eq(&pending.proof, &expected.proof)
            || pending.cursor != expected.cursor
            || pending.input().map(SourceGroundStep::binding)
                != expected.input().map(SourceGroundStep::binding)
            || pending.cursor >= pending.inputs.len()
        {
            return Err(CarrierError::PlanConflict);
        }
        runtime
            .validate_scope_creature_reservation(&pending.reservation, pending.proof.as_ref())?;
        let complete = pending.cursor + 1 == pending.inputs.len();
        let any_removed = pending.removed || removed;
        if complete {
            if any_removed && pending.proof.removes_creature() {
                // Lua removes the Splinter administratively. The following
                // transform of the already removed Item is a C++ no-op: no33950.
                runtime.remove_scope_creature_after_observed_outcome(
                    pending.reservation.clone(),
                    pending.proof.as_ref(),
                    pending.proof.actor(),
                    pending.proof.definition_key(),
                    pending.proof.content_digest(),
                )?;
            } else {
                runtime.release_scope_creatures_after_observed_outcome(
                    pending.reservation.clone(),
                    pending.proof.as_ref(),
                )?;
            }
        }
        if removed && let NativeMovementCollisionIndex::Bundle(collision) = pending.cells.index() {
            let at = pending.proof.position();
            if let (Ok(x), Ok(y), Some(floor)) = (
                u16::try_from(at.x),
                u16::try_from(at.y),
                at.floor
                    .checked_neg()
                    .and_then(|floor| i8::try_from(floor).ok()),
            ) {
                collision
                    .map()
                    .facts
                    .emit_movement_effect(crate::map::overlay::TilePos { x, y, floor }, 50);
            }
        }
        if complete {
            if let Some(following) = &pending.following {
                // Uses the original captured DATA. A removed Splinter cannot
                // execute a second actor callback; other Creatures continue.
                following.publish(runtime);
            }
            queue.pop_front();
        } else {
            pending.removed = any_removed;
            pending.cursor += 1;
        }
        Ok(())
    }
}

impl PreparedSourceCreatureStep<'_> {
    #[allow(
        clippy::expect_used,
        reason = "one uninterrupted successful position commit must match its preallocated exact-slot and destination proof"
    )]
    pub(super) fn publish(mut self, runtime: &mut ChannelRuntimeV1) {
        // The first original callback returned nil for this nonplayer. This
        // is a synchronous refusal, with no Item write or uncertain outcome;
        // no later original callback may be selected from the same movement.
        if self.templates.is_empty() {
            return;
        }
        let proof = Arc::get_mut(&mut self.proof).expect("unpublished unique Creature STEP proof");
        let movement = runtime
            .committed_movement_occurrence(proof.actor())
            .expect("successful native position commit actor")
            .expect("successful native translation occurrence");
        proof
            .bind_committed_movement(runtime, movement)
            .expect("same committed original destination");
        runtime
            .reserve_scope_creatures_after_committed_step(&mut self.reservation, proof, movement)
            .expect("only the actual position and tile order changed after preflight");
        let inputs = Arc::get_mut(&mut self.inputs).expect("unpublished unique Item cohort");
        for template in self.templates {
            inputs.push(
                template
                    .bind(proof)
                    .expect("preallocated exact committed Item input"),
            );
        }
        self.queue.push_back(PendingSourceCreatureStep {
            proof: self.proof,
            reservation: self.reservation,
            inputs: self.inputs,
            cursor: 0,
            removed: false,
            cells: self.cells,
            following: self.following,
            parent: self.parent,
        });
    }
}

impl MonsterMap {
    /// A real committed STEP owns its original Item continuation. The movement
    /// driver must retain later due inputs until this queue has reconciled.
    pub fn source_ground_step_pending(&self) -> bool {
        self.source_steps
            .binding
            .as_ref()
            .is_some_and(|(queue, _)| queue.is_pending())
    }
    pub(super) fn prepare_source_ground_step(
        &self,
        runtime: &ChannelRuntimeV1,
        cells: &NativeEntryMovementCells,
        actor: ExactActorRef,
        before: MovementPositionSnapshot,
        destination: MovementLocalPosition,
    ) -> Result<Option<PreparedSourceCreatureStep<'_>>, MovementError> {
        let Some((queue, _)) = &self.source_steps.binding else {
            return Ok(None);
        };
        let Some(tile) =
            self.source_steps
                .tiles
                .get(&(destination.x, destination.y, destination.floor))
        else {
            return Ok(None);
        };
        queue
            .prepare(
                runtime,
                cells,
                Arc::clone(tile),
                actor,
                before,
                self.source_steps.parent.clone(),
            )
            .map(Some)
    }
}

impl CurrentMonsterMap<'_, '_> {
    /// Optional only at construction, before the first cached read. Other maps
    /// retain their existing movement behavior with no new queue or SQL read.
    pub fn bind_source_ground_steps(
        &mut self,
        queue: Arc<SourceCreatureStepQueue>,
        reset_epoch: u64,
    ) -> Result<(), CarrierError> {
        if !self.view.tiles.is_empty() || self.view.source_steps.binding.is_some() {
            return Err(CarrierError::PlanConflict);
        }
        self.view.source_steps.binding = Some((queue, reset_epoch));
        Ok(())
    }
    /// Post-AI movement belongs to this exact retained pass even if its retry
    /// executes later than the pass's original due. The movement keeps its own
    /// observed time; the shared arbiter can first finish this dependency when
    /// selecting the actual parent. Initial pre-pass movement stays unlinked.
    pub fn link_source_ground_steps_to_pass<P: std::any::Any + Send + Sync>(
        &mut self,
        parent: &Arc<P>,
    ) -> Result<(), CarrierError> {
        if self.view.source_steps.binding.is_none() {
            return Err(CarrierError::PlanConflict);
        }
        let identity: SourceCreatureStepParent = parent.clone();
        if self
            .view
            .source_steps
            .parent
            .as_ref()
            .is_some_and(|old| !Arc::ptr_eq(old, &identity))
        {
            return Err(CarrierError::PlanConflict);
        }
        self.view.source_steps.parent = Some(identity);
        Ok(())
    }
    pub(super) async fn capture_source_ground_tile(
        &mut self,
        runtime: &ChannelRuntimeV1,
        at: MovementLocalPosition,
        items: &crate::durability::spell_items_abi::DurableTileItems,
    ) -> Option<()> {
        let Some((_, reset_epoch)) = self.view.source_steps.binding.as_ref() else {
            return Some(());
        };
        if !items.items.iter().any(|item| {
            SourceGroundStepKind::from_definition(&item.definition)
                == Some(SourceGroundStepKind::DeadAspect)
        }) {
            return Some(());
        }
        let selected = SourceCreatureGroundTile::read_in_transaction(
            self.tx,
            self.authority,
            runtime,
            self.room,
            self.content,
            *reset_epoch,
            at,
        )
        .await
        .ok()??;
        self.view
            .source_steps
            .tiles
            .insert((at.x, at.y, at.floor), selected);
        Some(())
    }
}
