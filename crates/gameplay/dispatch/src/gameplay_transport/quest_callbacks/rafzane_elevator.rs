//! Native USE consumer for the Goblin Merchant elevator's retained winches.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_rafzane_elevator(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        let source = super::super::world_map::MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: self.active_generation?.identity().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let resolved =
            super::super::world_map::resolve_map_target(&source, target, |_| None::<()>).ok()?;
        if resolved.pos.x != 33052
            || resolved.pos.y != 32099
            || ![-6, -7].contains(&resolved.pos.floor)
        {
            return None;
        }
        let result = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            let Some(permit) = self.spell_lane_permit().await else {
                return UseDisposition::Rejected;
            };
            let optional_pvp = self
                .special_movement_policy(&permit, actor, command.game_session_id)
                .await;
            let (mut runtime, mut states) =
                elevator_owners(&self.runtime, &self.states.spell_states).await;
            if !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|f| f.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                    .is_none()
                || map.facts.object_revision(resolved.placement_key) != target.expected_revision
            {
                return UseDisposition::Rejected;
            }
            let Ok(expected) = runtime.read_actor_position(actor) else {
                return UseDisposition::Rejected;
            };
            let Some(at) = native(expected.position()) else {
                return UseDisposition::Rejected;
            };
            if !super::super::map_door::reachable(at, resolved.pos) {
                return UseDisposition::TooFar;
            }
            if !self
                .states
                .world_gate
                .lock()
                .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
            {
                return UseDisposition::Exhausted;
            }
            let Some(base) =
                map.overlay
                    .base()
                    .tile(resolved.pos.x, resolved.pos.y, resolved.pos.floor)
            else {
                return UseDisposition::Rejected;
            };
            let Some(id) = base
                .ids()
                .iter()
                .zip(base.depths())
                .filter(|(_, d)| **d == 0)
                .nth(usize::from(resolved.ordinal))
                .map(|(id, _)| *id)
            else {
                return UseDisposition::Rejected;
            };
            let Some(winch) = map.facts.base_entry(resolved.pos, resolved.ordinal, id) else {
                return UseDisposition::Rejected;
            };
            let Some(plan) = map.facts.prepare_rafzane_elevator(
                map.overlay.base(),
                resolved.pos,
                winch.appearance_id,
            ) else {
                return UseDisposition::Rejected;
            };
            // The carrier has no same-tile stack order; as with native spell
            // targeting, only a sole occupant proves the source top creature.
            if !plan.calling
                && (at != plan.from
                    || !runtime
                        .is_sole_current_actor_on_cell(actor)
                        .unwrap_or(false))
            {
                self.states.chat.push_quest_feedback(
                    command.game_session_id,
                    "Step inside the elevator to use it.",
                );
                return UseDisposition::Rejected;
            }
            let from = local(Position {
                x: plan.from.x,
                y: plan.from.y,
                floor: plan.from.floor,
            });
            let landing = if plan.calling {
                plan.evacuation
            } else {
                plan.to
            };
            let to = local(Position {
                x: landing.x,
                y: landing.y,
                floor: landing.floor,
            });
            let closed = crate::map::floor::BundleDoors(map.facts.doors());
            let admits = |actor, session: Option<GameSessionId>, destination| {
                let blocked = session.is_some_and(|session| {
                    super::super::pvp_live::pz_blocked(&states, actor, session, self.owner_now())
                });
                crate::map::floor::quest_platform_landable(
                    &self.movement_cells,
                    destination,
                    blocked,
                    &closed,
                    plan.landing_walkable(&map.facts, map.overlay.base(), landing),
                )
            };
            let project = || plan.project(&map.facts, map.overlay.base());
            let moved = if plan.calling {
                runtime.commit_quest_platform_displacement(from, to, admits, project)
            } else {
                runtime.commit_quest_platform_ride(from, to, actor, admits, project)
            };
            let Ok(moved) = moved else {
                return UseDisposition::Rejected;
            };
            for movement in moved {
                if let Some(session) = movement.session {
                    states.stage_special_movement_step(
                        &runtime,
                        map,
                        movement.actor,
                        session,
                        self.owner_now().get(),
                        Some(movement.before.position()),
                        movement.after.position(),
                        optional_pvp,
                    );
                    states.stage_underwater_step(
                        &runtime,
                        movement.actor,
                        session,
                        self.owner_now().get(),
                        underwater(map, movement.before.position()),
                        underwater(map, movement.after.position()),
                    );
                }
            }
            let _ = map
                .facts
                .emit_movement_speech(crate::map::facts::MovementSpeech {
                    speaker: actor,
                    position: TilePos {
                        x: 33052,
                        ..plan.from
                    },
                    talk_type: 36,
                    text: plan.sound.to_owned(),
                    captured_name: None,
                });
            UseDisposition::Committed
        })
        .await
        .unwrap_or(UseDisposition::Rejected);
        Some(UseOutcome {
            disposition: result,
            ..UseOutcome::rejected()
        })
    }
}

async fn elevator_owners<'a>(
    runtime: &'a tokio::sync::Mutex<ChannelRuntimeV1>,
    states: &'a tokio::sync::Mutex<ChannelSpellStates>,
) -> (
    tokio::sync::MutexGuard<'a, ChannelRuntimeV1>,
    tokio::sync::MutexGuard<'a, ChannelSpellStates>,
) {
    loop {
        let runtime = runtime.lock().await;
        if let Ok(states) = states.try_lock() {
            return (runtime, states);
        }
        // Existing spell work can hold states while waiting for Runtime.
        // Yield both owners so neither lock order can strand the Channel.
        drop(runtime);
        drop(states.lock().await);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::Poll;

    #[test]
    fn rafzane_elevator_waits_without_holding_either_owner() -> Result<(), String> {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .map_err(|e| e.to_string())?
            .block_on(async {
                let (runtime, _, _, _) =
                    crate::gameplay_transport::spell_presentations::tests::runtime();
                let runtime = tokio::sync::Mutex::new(runtime);
                let states = tokio::sync::Mutex::new(ChannelSpellStates::default());
                {
                    let held = runtime.lock().await;
                    let mut acquiring = std::pin::pin!(elevator_owners(&runtime, &states));
                    std::future::poll_fn(|cx| {
                        assert!(acquiring.as_mut().poll(cx).is_pending());
                        Poll::Ready(())
                    })
                    .await;
                    assert!(
                        states.try_lock().is_ok(),
                        "an entity observer must be able to finish its spell-state read"
                    );
                    drop(held);
                    drop(acquiring.await);
                }
                {
                    let held = states.lock().await;
                    let mut acquiring = std::pin::pin!(elevator_owners(&runtime, &states));
                    std::future::poll_fn(|cx| {
                        assert!(acquiring.as_mut().poll(cx).is_pending());
                        Poll::Ready(())
                    })
                    .await;
                    assert!(
                        runtime.try_lock().is_ok(),
                        "a spell owner must be able to finish its runtime work"
                    );
                    drop(held);
                    drop(acquiring.await);
                }
                Ok(())
            })
    }
}
