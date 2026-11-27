//! Tutorial step interception around the existing owner movement path.
use super::*;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;

mod autowalk;
mod blocking;
pub(super) mod drunk;
mod protection_zone;
pub use autowalk::{AutoWalkRoute, AutoWalkTarget};
pub(super) mod monster_trap;
pub(super) mod special_movement;
pub(super) mod underwater_drown;

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
        let probe = protection_zone::Probe::default();
        let result = self
            .ordinary_paced_step_attempt(actor, session, direction, expected, follow, &probe)
            .await;
        if result.0.disposition != StepDisposition::Moved && probe.needs_refresh() {
            if self.refresh_pvp_locks(actor, session).await {
                return self
                    .ordinary_paced_step_attempt(
                        actor,
                        session,
                        direction,
                        expected,
                        follow,
                        &protection_zone::Probe::default(),
                    )
                    .await;
            }
            operator_event("event=step_blocked cause=protection_zone_authority");
        }
        result
    }

    async fn step_spell_lane_permit(
        &self,
    ) -> Option<crate::durability::spell_owner_commit::SpellLanePermit> {
        if let Some(permit) = self.spell_lane_permit().await {
            return Some(permit);
        }
        // An unresolved durable position owner must finish before we can move.
        // Give its owning lane the original command's bounded holder window.
        let retry = AdmissionRetryBudget::for_foreground_command(self.root).ok()?;
        retry
            .retry_unavailable(async || {
                self.spell_lane_permit()
                    .await
                    .ok_or(connection::AdmissionRefusal::Unavailable)
            })
            .await
            .ok()
    }

    async fn ordinary_paced_step_attempt(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
        expected: Option<crate::foundation::MovementPositionSnapshot>,
        follow: Option<(ExactActorRef, crate::foundation::MovementPositionSnapshot)>,
        pz_probe: &protection_zone::Probe,
    ) -> (StepOutcome, Option<Duration>) {
        if !self.resume_dawnport_departure(actor, session).await {
            #[cfg(test)]
            eprintln!("M25 movement preflight rejected at {}", line!());
            operator_event("event=step_rejected_dawnport");
            return (StepOutcome::rejected(), None);
        }
        let source_map_initialized = self.ensure_source_map_initialized().await;
        // DEATH-2 §4.2: a dead player takes no input until its respawn.
        if self.states.spell_states.lock().await.is_dead(actor) {
            #[cfg(test)]
            eprintln!("M25 movement preflight rejected at {}", line!());
            operator_event("event=step_rejected_dead");
            return (StepOutcome::rejected(), None);
        }
        use crate::movement::{CardinalStep, MovementError};
        // Preflight SQL shares the field owner's bounded durability holder.
        // Take the lane before those reads so a concurrent field turn cannot
        // exhaust the holder or invalidate their player-state predecessor.
        let Some(mut permit) = self.step_spell_lane_permit().await else {
            #[cfg(test)]
            eprintln!("M25 movement preflight rejected at {}", line!());
            operator_event("event=step_rejected_lane");
            return (StepOutcome::rejected(), None);
        };
        if !source_map_initialized {
            #[cfg(test)]
            eprintln!("M25 movement preflight rejected at {}", line!());
            operator_event("event=step_rejected_source_map");
            return (StepOutcome::rejected(), None);
        }
        // The collision index keeps its last known item facts while a cask
        // projection cannot refresh. Its read is not movement qualification.
        let _ = super::loop_move::reconcile_cask_collision(self).await;
        self.refresh_cyclopedia_speed(actor, session).await;
        let equipment = self.read_movement_equipment(actor, session).await;
        let optional_pvp = self.special_movement_policy(&permit, actor, session).await;
        // Lock order (§1.2): the lane before any Channel guard. These guards
        // freeze steering and the actual bundle landing for the ordinary turn.
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let mut now = self.owner_now().get();
        // USE-AUTOWALK-1: a relocation (even away and back) invalidates the queued step.
        let stale = |runtime: &mut ChannelRuntimeV1, states: &ChannelSpellStates| {
            follow.is_some_and(|(target, expected)| {
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
        };
        if stale(&mut runtime, &states) {
            operator_event("event=step_rejected_stale_position");
            return (StepOutcome::rejected(), None);
        }
        // The lane settles real parked commits. A marker for an unrelated
        // pending spell effect does not own the walking position.
        let requested_cardinal = match direction {
            StepDirection::North => CardinalStep::North,
            StepDirection::East => CardinalStep::East,
            StepDirection::South => CardinalStep::South,
            StepDirection::West => CardinalStep::West,
            StepDirection::NorthEast => CardinalStep::NorthEast,
            StepDirection::SouthEast => CardinalStep::SouthEast,
            StepDirection::SouthWest => CardinalStep::SouthWest,
            StepDirection::NorthWest => CardinalStep::NorthWest,
        };
        // Resolve steering before any direction-dependent field/floor read. Pass
        // this same frozen `now` to the common movement owner after preflight.
        let cardinal = match if runtime.forced_player_step_current(actor, session) {
            Ok(requested_cardinal)
        } else {
            states.drome_step_direction(&runtime, actor, session, now, requested_cardinal)
        } {
            Ok(direction) => direction,
            Err(MovementError::Blocked) => {
                operator_event("event=step_blocked cause=movement_condition");
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
        let direction = match cardinal {
            CardinalStep::North => StepDirection::North,
            CardinalStep::East => StepDirection::East,
            CardinalStep::South => StepDirection::South,
            CardinalStep::West => StepDirection::West,
            CardinalStep::NorthEast => StepDirection::NorthEast,
            CardinalStep::SouthEast => StepDirection::SouthEast,
            CardinalStep::SouthWest => StepDirection::SouthWest,
            CardinalStep::NorthWest => StepDirection::NorthWest,
        };
        let (direction, hicks) = if runtime.forced_player_step_current(actor, session) {
            (direction, false)
        } else {
            drunk::admitted_direction(&mut runtime, &states, actor, session, direction, now)
        };
        let mut cardinal = match direction {
            StepDirection::North => CardinalStep::North,
            StepDirection::East => CardinalStep::East,
            StepDirection::South => CardinalStep::South,
            StepDirection::West => CardinalStep::West,
            StepDirection::NorthEast => CardinalStep::NorthEast,
            StepDirection::SouthEast => CardinalStep::SouthEast,
            StepDirection::SouthWest => CardinalStep::SouthWest,
            StepDirection::NorthWest => CardinalStep::NorthWest,
        };
        let captured_source = self
            .rooms()
            .gate_source_world()
            .is_some_and(|room| room.source_world().is_some());
        let origin = runtime.read_actor_position(actor).ok();
        let room = self.rooms().gate_field_step_ingress();
        let native = self
            .active_generation
            .and_then(|active| active.native_gameplay());
        let field_ingress = native.is_some()
            && (room.is_some()
                || self.bundle_map().is_none()
                || self
                    .required_capabilities
                    .effective()
                    .binary_search(&30)
                    .is_ok());
        let clear = field_ingress
            && !captured_source
            && native.is_some_and(|native| native.source_digest() == runtime.content_pin().server_artifact_digest())
            && (runtime.forced_player_step_current(actor, session)
                || states.drome_step_direction(&runtime, actor, session, now, cardinal) == Ok(cardinal))
            && !crate::movement::source_floor_change::is_source_profile(self.movement_cells())
            && self.bundle_map().zip(room).is_some_and(|(map, room)| {
                let doors = crate::map::floor::BundleDoors(&map.facts);
                let pvp_blocking = protection_zone::StepBlocker::new(
                    &doors, self.movement_cells(), origin, &states, (actor, session), now, pz_probe,
                );
                let portals = super::quest_callbacks::CallbackPortals {
                    inner: &pvp_blocking,
                    map: Some(map),
                    callbacks: self.callback_content(),
                };
                origin.and_then(|origin| {
                    super::actor_spell::field_step_destination(
                        &runtime, self.movement_cells(), actor, session, origin, cardinal, &portals,
                    ).ok()
                }).is_some_and(|destination| {
                    !states.cloak_blood_cells.contains(&(destination.x, destination.y, destination.floor))
                        && crate::durability::native_world_items::SpellGroundTarget::for_native_tile_read(
                        room, &runtime, destination,
                    ).is_ok()
                        && self.states.loop_ground.proves_clear_movement_ground(
                            &runtime, map.overlay.base(), self.reset_epoch, destination,
                        )
                })
            });
        let field_pass = if field_ingress && !clear {
            // A busy holder waits outside the runtime and spell-state guards.
            // The lane stays owned, and the durable path requalifies its landing.
            drop((states, runtime));
            let pass = self.acquire_field_step_pass().await;
            runtime = self.runtime.lock().await;
            states = self.states.spell_states.lock().await;
            now = self.owner_now().get();
            if runtime.read_actor_position(actor).ok() != origin || stale(&mut runtime, &states) {
                operator_event("event=step_rejected_stale_position");
                drop((states, runtime));
                let _ = Self::return_unused_field_step_pass(pass).await;
                return (StepOutcome::rejected(), None);
            }
            // Drome steering samples owner time. Read the durable tile for the
            // same direction the common movement owner will use after the wait.
            if runtime.is_tibiadrome(actor)
                && !runtime.forced_player_step_current(actor, session)
                && let Ok(steered) =
                    states.drome_step_direction(&runtime, actor, session, now, requested_cardinal)
            {
                cardinal = steered;
            }
            Some(pass)
        } else {
            None
        };
        if runtime.note_rotten_movement_time(now).is_err() {
            operator_event("event=step_rejected_movement kind=OwnerClock");
            drop((states, runtime));
            if let Some(pass) = field_pass {
                let _ = Self::return_unused_field_step_pass(pass).await;
            }
            return (StepOutcome::rejected(), None);
        }

        // Re-read legacy doors after any checkout wait. Bundle doors and the
        // teleport landing are always resolved from the current collision facts.
        let (none, bundle_doors, legacy);
        let blocking: &dyn crate::map::floor::CellBlocker = if captured_source {
            none = std::collections::BTreeSet::<crate::content::LogicalCell>::new();
            &none
        } else if let Some(map) = self.bundle_map() {
            bundle_doors = crate::map::floor::BundleDoors(&map.facts);
            &bundle_doors
        } else {
            legacy = self.door.lock().await.blocking_cells().clone();
            &legacy
        };
        let pvp_blocking = protection_zone::StepBlocker::new(
            blocking,
            self.movement_cells(),
            origin,
            &states,
            (actor, session),
            now,
            pz_probe,
        );
        let callback_portals = super::quest_callbacks::CallbackPortals {
            inner: &pvp_blocking,
            map: self.bundle_map(),
            callbacks: self.callback_content(),
        };
        let blocking: &dyn crate::map::floor::CellBlocker = &callback_portals;
        let equipped_speed_delta = equipment
            .as_ref()
            .and_then(|read| read.current_delta(&runtime, &states, actor, session));
        let snow_origin = self.bundle_map().is_some_and(|map| {
            origin.is_some_and(|at| special_movement::snow_at(map, at.position()))
        });
        let outcome = if let Some(pass) = field_pass {
            self.step_with_field_ingress(
                &mut permit,
                pass,
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
        } else {
            states.step_with_non_player_field_contact(
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
        };
        if hicks
            && matches!(
                &outcome,
                Ok(_)
                    | Err(MovementError::Blocked
                        | MovementError::BlockedByCreature(_)
                        | MovementError::BlockedByItem(_)
                        | MovementError::Cell(
                            crate::content::static_cell_engine::StaticCellEngineError::Absent,
                        ))
            )
        {
            drunk::speech(self, &runtime, actor, session, origin);
        }
        #[cfg(test)]
        if let Err(error) = &outcome {
            eprintln!("M25 movement commit error={error:?}");
        }
        match outcome {
            Ok((snapshot, duration)) => {
                // Crystal hazard_primal.lua:23-55: actual accepted STEP consumes
                // the exact physical pod, with HP owned by this same player turn.
                super::primal_pods::step(
                    self,
                    &mut runtime,
                    &mut states,
                    actor,
                    session,
                    now / 1000,
                );
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
                    );
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
                let spite_staged = super::spite_step::stage(
                    self,
                    &mut runtime,
                    &mut states,
                    actor,
                    session,
                    snapshot,
                )
                .is_ok();
                drop(states);
                drop(runtime);
                if !spite_staged || !self.resolve_spite_steps(&permit).await {
                    let runtime = self.runtime.lock().await;
                    return (
                        StepOutcome {
                            disposition: StepDisposition::Moved,
                            moved_to: self.wire_observation(&runtime, snapshot.position()),
                        },
                        Some(duration),
                    );
                }
                let _ = self.resolve_cloak_blood_steps(&permit).await;
                self.resolve_rotten_movement_callbacks(&permit).await;
                super::soul_war_world::committed_step(self, actor, session, origin, snapshot, now)
                    .await;
                super::primal_callbacks::step_in(self, &permit, actor, session, snapshot, now)
                    .await;
                super::rotten_blood_callbacks::step_in(
                    self, &permit, actor, session, snapshot, now,
                )
                .await;
                super::echo_warden::step_in(self, actor, session, now / 1000).await;
                if let Some(event) = snowbunny_progress_operation(
                    snow_origin,
                    actor.placement_identity(),
                    session,
                    now,
                ) && !self.add_snowbunny_progress(&permit, session, event).await
                {
                    super::operator_event("snowbunny_step_transaction_rolled_back");
                }
                if let Some(origin) = origin {
                    self.close_departed_door(
                        &permit,
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
                            .soulpit_lobby_step(&permit, actor, session, snapshot)
                            .await;
                        let snapshot = self
                            .step_quest_callback(&permit, actor, session, origin, snapshot)
                            .await;
                        let snapshot = self
                            .step_grizzly_boss(&permit, actor, session, origin, snapshot)
                            .await;
                        self.dawnport_trial_step(actor, session, snapshot).await;
                        self.cake_errand_step(&permit, actor, session, snapshot)
                            .await;
                        snapshot
                    }
                    None => snapshot,
                };
                self.cyclopedia_step(actor, session, snapshot).await;
                self.refresh_cyclopedia_speed(actor, session).await;
                let runtime = self.runtime.lock().await;
                // NPC-TALK-2: a customer who stepped out of talk range ends its conversation now.
                self.states.chat.close_out_of_range(&runtime, session);
                let outcome = StepOutcome {
                    disposition: StepDisposition::Moved,
                    moved_to: self.wire_observation(&runtime, snapshot.position()),
                };
                (outcome, Some(duration))
            }
            Err(
                error @ (MovementError::Blocked
                | MovementError::BlockedByCreature(_)
                | MovementError::BlockedByItem(_)
                | MovementError::Cell(
                    crate::content::static_cell_engine::StaticCellEngineError::Absent,
                )),
            ) => {
                if error == MovementError::Blocked && pz_probe.blocked_entry() {
                    operator_event("event=step_blocked cause=protection_zone");
                } else if !pz_probe.needs_refresh() {
                    blocking::log(self, origin, cardinal, error);
                }
                (
                    StepOutcome {
                        disposition: StepDisposition::Blocked,
                        moved_to: None,
                    },
                    None,
                )
            }
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
                    MovementError::Blocked
                    | MovementError::BlockedByCreature(_)
                    | MovementError::BlockedByItem(_) => {
                        "event=step_rejected_movement kind=Blocked"
                    }
                    MovementError::CapacityExceeded => {
                        "event=step_rejected_movement kind=CapacityExceeded"
                    }
                    MovementError::NotQualified => "event=step_rejected_movement kind=NotQualified",
                    MovementError::HolderUnavailable => {
                        "event=step_rejected_movement kind=HolderUnavailable"
                    }
                    MovementError::FieldAuthorityUnavailable => {
                        "event=step_rejected_movement kind=FieldAuthorityUnavailable"
                    }
                });
                (StepOutcome::rejected(), None)
            }
        }
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
    let (body, _) = source
        .split_once("\n#[cfg(test)]\nmod snowbunny_operation_tests")
        .ok_or("production body")?;
    for branch in [
        "dawnport",
        "source_map",
        "dead",
        "lane",
        "stale_position",
        "drome",
        "movement",
    ] {
        assert!(
            body.contains(&format!("event=step_rejected_{branch}")),
            "{branch}"
        );
    }
    assert!(include_str!("field_step_ingress.rs").contains("event=step_rejected_holder_busy"));
    // The source-owned Rotten movement-time check adds one closed refusal.
    assert_eq!(
        body.matches("runtime.note_rotten_movement_time(now).is_err()")
            .count(),
        1
    );
    assert_eq!(body.matches("StepOutcome::rejected()").count(), 8);
    assert!(body.contains("event=step_blocked cause=protection_zone_authority"));
    assert!(body.contains("event=step_blocked cause=movement_condition"));
    assert!(body.contains("event=step_blocked cause=protection_zone"));
    Ok(())
}
