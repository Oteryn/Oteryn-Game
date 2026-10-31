//! Tutorial step interception around the existing owner movement path.
use super::*;

mod autowalk;
pub use autowalk::{AutoWalkRoute, AutoWalkTarget};
pub(super) mod underwater_drown;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn ordinary_paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
        expected: Option<crate::foundation::MovementPositionSnapshot>,
        follow: Option<(ExactActorRef, crate::foundation::MovementPositionSnapshot)>,
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
        // Preflight SQL shares the field owner's bounded durability holder.
        // Take the lane before those reads so a concurrent field turn cannot
        // exhaust the holder or invalidate their player-state predecessor.
        let Some(mut permit) = self.spell_lane_permit().await else {
            return (StepOutcome::rejected(), None);
        };
        if !self.refresh_pvp_locks(actor, session).await {
            return (StepOutcome::rejected(), None);
        }
        let equipment = self.read_movement_equipment(actor, session).await;
        // Lock order (§1.2): the lane before any Channel guard.
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        // A periodic condition turn may finish while this step waits for the
        // lane or Channel locks. Sample time only after acquiring its owners;
        // field entry must not predate the condition predecessor it observes.
        let now = self.owner_now().get();
        // USE-AUTOWALK-1: a relocation (even away and back) invalidates the queued step.
        // Compare while holding the same owner lock as the movement commit.
        if follow.is_some_and(|(target, expected)| {
            states.is_dead(target)
                || (!runtime.contains_live_creature(target)
                    && !runtime
                        .visible_entities()
                        .players
                        .iter()
                        .any(|player| player.actor == target))
                || runtime.borrow_movement_position().read(target) != Ok(expected)
        }) || states.is_dead(actor)
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
        // A field-enabled World reads dynamic Ground Items before committing its step.
        let origin = runtime.read_actor_position(actor).ok();
        let outcome = if self.bundle_map().is_some()
            && self
                .required_capabilities
                .effective()
                .binary_search(&30)
                .is_err()
        {
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
                &mut permit,
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
                // PAR-M10: underwater ground drowns on entry and stops on exit, in the same
                // owner turn as the committed step.
                if let Some(map) = self.bundle_map() {
                    states.stage_underwater_step(
                        &runtime,
                        actor,
                        session,
                        now,
                        origin.is_some_and(|origin| {
                            underwater_drown::underwater(map, origin.position())
                        }),
                        underwater_drown::underwater(map, snapshot.position()),
                    );
                }
                drop(states);
                drop(runtime);
                let snapshot = match origin {
                    Some(origin) => {
                        let snapshot = self
                            .citizen_step(actor, session, direction, origin, snapshot)
                            .await;
                        let snapshot = self
                            .step_quest_callback(&permit, actor, session, origin, snapshot)
                            .await;
                        self.dawnport_trial_step(actor, session, snapshot).await;
                        snapshot
                    }
                    None => snapshot,
                };
                let runtime = self.runtime.lock().await;
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
