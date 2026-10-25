//! Tutorial step interception around the existing owner movement path.
use super::*;

mod autowalk;
pub use autowalk::{AutoWalkRoute, AutoWalkTarget};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn ordinary_paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
        expected: Option<crate::foundation::MovementPositionSnapshot>,
    ) -> (StepOutcome, Option<Duration>) {
        if !self.ensure_source_map_initialized().await {
            return (StepOutcome::rejected(), None);
        }
        // DEATH-2 §4.2: a dead player takes no input until its respawn.
        if self.states.spell_states.lock().await.is_dead(actor) {
            return (StepOutcome::rejected(), None);
        }
        use crate::movement::{CardinalStep, MovementError};
        let cardinal = match direction {
            StepDirection::North => CardinalStep::North,
            StepDirection::East => CardinalStep::East,
            StepDirection::South => CardinalStep::South,
            StepDirection::West => CardinalStep::West,
        };
        if !self.refresh_pvp_locks(actor, session).await {
            return (StepOutcome::rejected(), None);
        }
        let equipment = self.read_movement_equipment(actor, session).await;
        let now = self.owner_now().get();
        // Lock order (§1.2): the lane before any Channel guard.
        let Some(permit) = self.spell_lane_permit().await else {
            return (StepOutcome::rejected(), None);
        };
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        // USE-AUTOWALK-1: a relocation (even away and back) invalidates the queued step.
        // Compare while holding the same owner lock as the movement commit.
        if states.is_dead(actor)
            || expected.is_some_and(|expected| {
                runtime.borrow_movement_position().read(actor) != Ok(expected)
            })
        {
            return (StepOutcome::rejected(), None);
        }
        if states.has_pending_spell_commit(actor, session) {
            return (StepOutcome::rejected(), None);
        }
        // The closed bundle doors are read live per candidate cell, not collected per step: a
        // teleport landing never relocates a player onto one (WORLD-INTERACTION-0 §6.6).
        let (none, bundle_doors, legacy);
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
        let pvp_blocking = super::pvp_live::PvpBlocker {
            inner: blocking,
            cells: self.movement_cells,
            blocked: super::pvp_live::pz_blocked(&states, actor, session, self.owner_now()),
        };
        let blocking: &dyn crate::map::floor::CellBlocker = &pvp_blocking;
        let equipped_speed_delta = equipment
            .as_ref()
            .and_then(|read| read.current_delta(&runtime, &states, actor, session));
        // MAP-CUTOVER-1b (§1.2): a bundle World has no field items; a step reads only the bundle
        // collision index.
        let outcome = if self.bundle_map().is_some() {
            states.step_with_creature_field_contact(
                &mut runtime,
                self.movement_cells,
                actor,
                session,
                now,
                cardinal,
                blocking,
                equipped_speed_delta,
                None,
                None,
            )
        } else {
            self.step_with_field_ingress(
                &permit,
                &mut runtime,
                &mut states,
                actor,
                session,
                now,
                cardinal,
                blocking,
                equipped_speed_delta,
            )
            .await
        };
        match outcome {
            Ok((snapshot, duration)) => {
                // NPC-TALK-2: a customer who stepped out of talk range ends its conversation now.
                self.states.chat.close_out_of_range(&runtime, session);
                (
                    StepOutcome {
                        disposition: StepDisposition::Moved,
                        moved_to: Some(Self::observation(&runtime, snapshot.position())),
                    },
                    Some(duration),
                )
            }
            Err(
                MovementError::Blocked
                | MovementError::Cell(
                    crate::content::static_cell_engine::StaticCellEngineError::Absent,
                ),
            ) => (
                StepOutcome {
                    disposition: StepDisposition::Blocked,
                    moved_to: None,
                },
                None,
            ),
            Err(_) => (StepOutcome::rejected(), None),
        }
    }
}
