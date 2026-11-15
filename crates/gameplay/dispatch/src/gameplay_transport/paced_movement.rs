//! Tutorial step interception around the existing owner movement path.
use super::actor_spell::ChannelSpellStates;
use super::*;

mod autowalk;
pub use autowalk::{AutoWalkRoute, AutoWalkTarget};
pub(super) mod monster_trap;
pub(super) mod special_movement;
pub(super) mod underwater_drown;

/// Physical movement is already committed while its field transaction may be unknown.
/// Keep callbacks outside the retained player predecessor until the lane reconciles it.
#[derive(Debug, Clone, Copy)]
pub(in crate::gameplay_transport) struct PacedStepCallbacks {
    actor: ExactActorRef,
    session: GameSessionId,
    direction: StepDirection,
    origin: Option<crate::foundation::MovementPositionSnapshot>,
    pub(in crate::gameplay_transport) snapshot: crate::foundation::MovementPositionSnapshot,
    now: u64,
    optional_pvp: Option<bool>,
    snow_origin: bool,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::gameplay_transport) struct PendingSnowProgress {
    actor: ExactActorRef,
    session: GameSessionId,
    snapshot: crate::foundation::MovementPositionSnapshot,
    event: [u8; 16],
    command: crate::foundation::CommandRef,
    fence: Option<crate::durability::character_progression::CurrentCharacterGameplayFence>,
}

impl ChannelSpellStates {
    pub(in crate::gameplay_transport) fn retain_paced_step_callbacks(
        &mut self,
        step: PacedStepCallbacks,
    ) {
        if self.paced_step_callbacks.is_some() {
            return;
        }
        if let Some(event) = snowbunny_progress_operation(
            step.snow_origin,
            step.actor.placement_identity(),
            step.session,
            step.now,
        ) && let Ok(id) = crate::foundation::CommandId::new(step.now.max(1))
        {
            self.pending_snow_progress.push_back(PendingSnowProgress {
                actor: step.actor,
                session: step.session,
                snapshot: step.snapshot,
                event,
                command: crate::foundation::CommandRef::new(step.session, id),
                fence: None,
            });
        }
        self.paced_step_callbacks = Some(step);
    }
}

fn snowbunny_progress_operation(
    snow_origin: bool,
    actor: [u8; 16],
    session: GameSessionId,
    now: u64,
) -> Option<[u8; 16]> {
    use sha2::Digest;
    snow_origin.then(|| {
        let digest = sha2::Sha256::new()
            .chain_update(actor)
            .chain_update(session.as_bytes())
            .chain_update(now.to_be_bytes())
            .finalize();
        let mut event = [0_u8; 16];
        event.copy_from_slice(&digest[..16]);
        event
    })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn ordinary_paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
        expected: Option<crate::foundation::MovementPositionSnapshot>,
        follow: Option<(ExactActorRef, crate::foundation::MovementPositionSnapshot)>,
    ) -> (StepOutcome, Option<Duration>) {
        if !self.resume_dawnport_departure(actor, session).await {
            operator_event("event=step_rejected_dawnport");
            return (StepOutcome::rejected(), None);
        }
        if !self.ensure_source_map_initialized().await {
            operator_event("event=step_rejected_source_map");
            return (StepOutcome::rejected(), None);
        }
        // DEATH-2 §4.2: a dead player takes no input until its respawn.
        if self.states.spell_states.lock().await.is_dead(actor) {
            operator_event("event=step_rejected_dead");
            return (StepOutcome::rejected(), None);
        }
        use crate::movement::{CardinalStep, MovementError};
        let cardinal = match direction {
            StepDirection::North => CardinalStep::North,
            StepDirection::East => CardinalStep::East,
            StepDirection::South => CardinalStep::South,
            StepDirection::West => CardinalStep::West,
            StepDirection::NorthEast => CardinalStep::NorthEast,
            StepDirection::SouthEast => CardinalStep::SouthEast,
            StepDirection::SouthWest => CardinalStep::SouthWest,
            StepDirection::NorthWest => CardinalStep::NorthWest,
        };
        // Preflight SQL shares the field owner's bounded durability holder.
        // Take the lane before those reads so a concurrent field turn cannot
        // exhaust the holder or invalidate their player-state predecessor.
        let Some(mut permit) = self.spell_lane_permit().await else {
            operator_event("event=step_rejected_lane");
            return (StepOutcome::rejected(), None);
        };
        self.drain_paced_step_async_callbacks(&permit).await;
        self.retry_snow_progress(&permit).await;
        if !self.refresh_pvp_locks(actor, session).await {
            operator_event("event=step_rejected_pvp_locks");
            return (StepOutcome::rejected(), None);
        }
        self.refresh_cyclopedia_speed(actor, session).await;
        let equipment = self.read_movement_equipment(actor, session).await;
        let optional_pvp = self.special_movement_policy(&permit, actor, session).await;
        // Checkout may wait for background durability work. Never wait under
        // the Channel runtime or spell-state guards; SQL work itself runs once.
        let field_pass = if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_some()
            && !(self.bundle_map().is_some()
                && self
                    .required_capabilities
                    .effective()
                    .binary_search(&30)
                    .is_err())
        {
            self.acquire_field_step_pass().await
        } else {
            Err(crate::gameplay_transport::connection::AdmissionRefusal::Rejected)
        };
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
            operator_event("event=step_rejected_stale_position");
            return (StepOutcome::rejected(), None);
        }
        if states.has_pending_spell_commit(actor, session) {
            operator_event("event=step_rejected_pending_spell");
            return (StepOutcome::rejected(), None);
        }
        // Resolve steering before any direction-dependent field/floor read. Pass
        // this same frozen `now` to the common movement owner after preflight.
        let cardinal = match states.drome_step_direction(&runtime, actor, session, now, cardinal) {
            Ok(direction) => direction,
            Err(MovementError::Blocked) => {
                return (
                    StepOutcome {
                        disposition: StepDisposition::Blocked,
                        moved_to: None,
                    },
                    None,
                );
            }
            Err(_) => {
                operator_event("event=step_rejected_drome");
                return (StepOutcome::rejected(), None);
            }
        };
        // The closed bundle doors are read live per candidate cell, not collected per step: a
        // teleport landing never relocates a player onto one (WORLD-INTERACTION-0 §6.6).
        let (none, bundle_doors, legacy);
        let blocking: &dyn crate::map::floor::CellBlocker = if self
            .rooms()
            .gate_source_world()
            .is_some_and(|room| room.source_world().is_some())
        {
            none = std::collections::BTreeSet::<crate::content::LogicalCell>::new();
            &none
        } else if let Some(map) = self.bundle_map() {
            bundle_doors = crate::map::floor::BundleDoors(&map.facts);
            &bundle_doors
        } else {
            legacy = self.door.lock().await.blocking_cells().clone();
            &legacy
        };
        let pvp_blocking = super::pvp_live::PvpBlocker {
            inner: blocking,
            cells: self.movement_cells(),
            blocked: super::pvp_live::pz_blocked(&states, actor, session, self.owner_now()),
        };
        let callback_portals = super::quest_callbacks::CallbackPortals {
            inner: &pvp_blocking,
            map: self.bundle_map(),
            callbacks: self.callback_content(),
        };
        let blocking: &dyn crate::map::floor::CellBlocker = &callback_portals;
        let equipped_speed_delta = equipment
            .as_ref()
            .and_then(|read| read.current_delta(&runtime, &states, actor, session));
        // A field-enabled World reads dynamic Ground Items before committing its step.
        let origin = runtime.read_actor_position(actor).ok();
        let snow_origin = self.bundle_map().is_some_and(|map| {
            origin.is_some_and(|at| special_movement::snow_at(map, at.position()))
        });
        let outcome = if self.bundle_map().is_some()
            && self
                .required_capabilities
                .effective()
                .binary_search(&30)
                .is_err()
        {
            states.step_with_creature_field_contact(
                &mut runtime,
                self.movement_cells(),
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
                field_pass,
                &mut runtime,
                &mut states,
                actor,
                session,
                now,
                cardinal,
                blocking,
                equipped_speed_delta,
                origin.map(|snapshot| PacedStepCallbacks {
                    actor,
                    session,
                    direction,
                    origin,
                    snapshot,
                    now,
                    optional_pvp,
                    snow_origin,
                }),
            )
            .await
        };
        match outcome {
            Ok((snapshot, duration)) => {
                states.retain_paced_step_callbacks(PacedStepCallbacks {
                    actor,
                    session,
                    direction,
                    origin,
                    snapshot,
                    now,
                    optional_pvp,
                    snow_origin,
                });
                drop(states);
                drop(runtime);
                let snapshot = if permit.has_unresolved() {
                    snapshot
                } else {
                    self.drain_paced_step_callbacks(&permit).await;
                    self.drain_paced_step_async_callbacks(&permit)
                        .await
                        .unwrap_or(snapshot)
                };
                let runtime = self.runtime.lock().await;
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
            Err(error) => {
                // Only the closed variant name is operator evidence; payloads can
                // contain actor/cell facts and must never enter this event.
                operator_event(match error {
                    MovementError::Actor(_) => "event=step_rejected_movement kind=Actor",
                    MovementError::SnapshotMismatch => {
                        "event=step_rejected_movement kind=SnapshotMismatch"
                    }
                    MovementError::ContextMismatch => {
                        "event=step_rejected_movement kind=ContextMismatch"
                    }
                    MovementError::ScopeMismatch => {
                        "event=step_rejected_movement kind=ScopeMismatch"
                    }
                    MovementError::CoordinateOverflow => {
                        "event=step_rejected_movement kind=CoordinateOverflow"
                    }
                    MovementError::Cell(_) => "event=step_rejected_movement kind=Cell",
                    MovementError::Blocked => "event=step_rejected_movement kind=Blocked",
                    MovementError::CapacityExceeded => {
                        "event=step_rejected_movement kind=CapacityExceeded"
                    }
                    MovementError::NotQualified => "event=step_rejected_movement kind=NotQualified",
                });
                (StepOutcome::rejected(), None)
            }
        }
    }
    pub(in crate::gameplay_transport) async fn drain_paced_step_callbacks(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> Option<crate::foundation::MovementPositionSnapshot> {
        if permit.has_unresolved() {
            return None;
        }
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let step = states.paced_step_callbacks.take()?;
        let PacedStepCallbacks {
            actor,
            session,
            origin,
            snapshot,
            now,
            optional_pvp,
            ..
        } = step;
        if runtime.read_actor_position(actor).ok() != Some(snapshot)
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return None;
        }
        // Crystal hazard_primal.lua:23-55: actual accepted STEP consumes
        // the exact physical pod, with HP owned by this same player turn.
        super::primal_pods::step(self, &mut runtime, &mut states, actor, session, now / 1000);
        // PAR-M10: underwater ground drowns on entry and stops on exit, in the same
        // owner turn as the committed step.
        if let Some(map) = self.bundle_map() {
            states.stage_special_movement_step(
                &runtime,
                map,
                actor,
                session,
                now,
                origin.map(|origin| origin.position()),
                snapshot.position(),
                optional_pvp,
                &|id| {
                    self.active_generation
                        .and_then(|active| active.native_gameplay())
                        .and_then(|native| special_movement::configured_decay_us(native, id))
                },
            );
            states.stage_underwater_step(
                &runtime,
                actor,
                session,
                now,
                origin.is_some_and(|origin| underwater_drown::underwater(map, origin.position())),
                underwater_drown::underwater(map, snapshot.position()),
            );
        }
        states.paced_step_async_callbacks = Some(step);
        Some(snapshot)
    }

    async fn defer_snow_progress(&self) {
        let mut states = self.states.spell_states.lock().await;
        if let Some(pending) = states.pending_snow_progress.pop_front() {
            states.pending_snow_progress.push_back(pending);
        }
    }

    pub(in crate::gameplay_transport) async fn retry_snow_progress(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) {
        loop {
            let Some(mut pending) = self
                .states
                .spell_states
                .lock()
                .await
                .pending_snow_progress
                .front()
                .copied()
            else {
                return;
            };
            {
                let runtime = self.runtime.lock().await;
                if !runtime
                    .player_control_facts(pending.actor, pending.session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                    || !runtime
                        .read_actor_position(pending.actor)
                        .is_ok_and(|snapshot| snapshot.context() == pending.snapshot.context())
                {
                    self.defer_snow_progress().await;
                    return;
                }
            }
            if pending.fence.is_none() {
                let Ok(Some(fence)) = self.current_quest_fence(pending.session).await else {
                    self.defer_snow_progress().await;
                    return;
                };
                pending.fence = Some(fence);
                if let Some(held) = self
                    .states
                    .spell_states
                    .lock()
                    .await
                    .pending_snow_progress
                    .front_mut()
                {
                    held.fence = Some(fence);
                }
            }
            let Some(fence) = pending.fence else {
                self.defer_snow_progress().await;
                return;
            };
            {
                let runtime = self.runtime.lock().await;
                if runtime.binding().runtime_scope() != fence.runtime_scope
                    || runtime.binding().scope_generation() != fence.scope_ownership_generation
                    || !runtime
                        .bound_attacker_lease(pending.actor, pending.session)
                        .is_ok_and(|lease| {
                            lease.character_id().as_bytes() == fence.character_id.as_bytes()
                                && lease.generation() == fence.character_lease_generation
                        })
                {
                    self.defer_snow_progress().await;
                    return;
                }
            }
            if !self
                .add_snowbunny_progress(permit, fence, pending.event, pending.command)
                .await
            {
                self.defer_snow_progress().await;
                return;
            }
            self.states
                .spell_states
                .lock()
                .await
                .pending_snow_progress
                .pop_front();
        }
    }

    // SQL callbacks run on the movement/vitals owner rather than recursively inside
    // the erased lane resolver. Their physical predecessor remains fenced as well.
    pub(in crate::gameplay_transport) async fn drain_paced_step_async_callbacks(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> Option<crate::foundation::MovementPositionSnapshot> {
        if permit.has_unresolved() {
            return None;
        }
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let PacedStepCallbacks {
            actor,
            session,
            origin,
            snapshot,
            now,
            direction,
            ..
        } = states.paced_step_async_callbacks.take()?;
        if runtime.read_actor_position(actor).ok() != Some(snapshot)
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return None;
        }
        drop(states);
        drop(runtime);
        super::echo_warden::step_in(self, actor, session, now / 1000).await;
        self.retry_snow_progress(permit).await;
        if let Some(origin) = origin {
            self.close_departed_door(
                permit,
                actor,
                session,
                origin.position(),
                snapshot.revision(),
            )
            .await;
        }
        let snapshot = match origin {
            Some(origin) => {
                let snapshot = self
                    .citizen_step(actor, session, direction, origin, snapshot)
                    .await;
                let snapshot = self
                    .soulpit_lobby_step(permit, actor, session, snapshot)
                    .await;
                let snapshot = self
                    .step_quest_callback(permit, actor, session, origin, snapshot)
                    .await;
                let snapshot = self
                    .step_grizzly_boss(permit, actor, session, origin, snapshot)
                    .await;
                self.dawnport_trial_step(actor, session, snapshot).await;
                snapshot
            }
            None => snapshot,
        };
        self.cyclopedia_step(actor, session, snapshot).await;
        self.refresh_cyclopedia_speed(actor, session).await;
        let runtime = self.runtime.lock().await;
        // NPC-TALK-2: a customer who stepped out of talk range ends its conversation now.
        self.states.chat.close_out_of_range(&runtime, session);
        Some(snapshot)
    }
}

#[cfg(test)]
mod snowbunny_operation_tests {
    use super::*;

    #[test]
    fn snowbunny_stages_at_most_one_progress_operation_per_action() -> Result<(), &'static str> {
        let mut bytes = [0; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        let session = GameSessionId::decode(&bytes).map_err(|_| "session")?;
        assert!(snowbunny_progress_operation(false, [3; 16], session, 1).is_none());
        let first = snowbunny_progress_operation(true, [3; 16], session, 1)
            .ok_or("snow action operation")?;
        assert_eq!(
            snowbunny_progress_operation(true, [3; 16], session, 1),
            Some(first),
            "one action is represented by one idempotent database operation"
        );
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn nbroute_step_rejections_have_closed_schema_operator_evidence() -> Result<(), &'static str> {
    let source = include_str!("paced_movement.rs");
    let body = source
        .split("#[cfg(test)]")
        .next()
        .ok_or("production body")?;
    for branch in [
        "dawnport",
        "source_map",
        "dead",
        "lane",
        "pvp_locks",
        "stale_position",
        "pending_spell",
        "drome",
        "movement",
    ] {
        assert!(
            body.contains(&format!("event=step_rejected_{branch}")),
            "{branch}"
        );
    }
    assert!(include_str!("field_step_ingress.rs").contains("event=step_rejected_holder_busy"));
    assert_eq!(body.matches("StepOutcome::rejected()").count(), 9);
    assert_eq!(body.matches("operator_event(").count(), 10);
    Ok(())
}
