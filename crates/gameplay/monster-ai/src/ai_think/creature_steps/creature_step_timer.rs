//! DEADLINE_STATE steps: one pending deadline per exact actor, cancelled on idle/refusal.
use super::*;

use crate::ai_think::behaviour_profile::CreatureBehaviourProfile;
use crate::ai_think::targeting::{self, TargetCandidate};

use crate::foundation::owner_timer::{
    CatchUpPolicy, FamilyPolicy, SemanticTimeMicros, VirtualOwnerClock,
};
use crate::movement::pacing::PacingTime;
use crate::movement::speed::creature_step_duration;
use crate::movement::{MovementEngineeringSelection, step_cardinal};

impl CreatureAiTable {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::ai_think) fn fire_steps_through(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &NativeEntryMovementCells,
        blocking: &dyn CellBlocker,
        current_map: Option<&crate::movement::monster_map::MonsterMap>,
        source: &dyn CurrentCreatureSpeedReader,
        ground: &dyn GroundSpeedSource,
        root: &GameplayDecisionRoot,
        players: &[TargetCandidate],
        through_us: u64,
        now: u64,
    ) {
        if through_us > now {
            return;
        }
        if current_map.is_some_and(|map| map.source_ground_step_pending()) {
            return;
        }
        let cadence_now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now);
        if self.step_timers.is_none() {
            let b = runtime.binding();
            self.step_timers = OwnerTimerLane::for_generation(
                b.runtime_scope(),
                b.scope_generation(),
                [(
                    StepFamily,
                    FamilyPolicy {
                        max_pending: 1,
                        catch_up: CatchUpPolicy::DeadlineState,
                    },
                )],
            )
            .ok();
        }
        let Some(mut timers) = self.step_timers.take() else {
            return;
        };
        for entry in self.entries.values_mut() {
            if entry.motion.due.is_none() {
                timers.cancel(StepFamily, entry.state.actor);
            }
        }
        loop {
            if current_map.is_some_and(|map| map.source_ground_step_pending()) {
                // The completed movement published its original Item cohort.
                // Leave every later timer and prepared motion exactly where
                // its real owner scheduled it, including the original due.
                self.step_timers = Some(timers);
                return;
            }
            let Ok(fence) = runtime.owner_fence() else {
                self.step_timers = Some(timers);
                return;
            };
            let mut first = true;
            let fired = timers.drain_due_matching(
                &VirtualOwnerClock::new(SemanticTimeMicros::from_micros(through_us)),
                fence,
                |actor| runtime.contains_live_creature(actor),
                |_, actor| {
                    if !std::mem::take(&mut first) {
                        return false;
                    }
                    let Some(map) = current_map else { return true };
                    let Some(entry) = self.entry(actor) else {
                        return true;
                    };
                    let Some((_, next, _)) = entry.motion.prepared else {
                        return true;
                    };
                    let movement = &entry.state.profile.behavior().movement;
                    entry.motion.shove_checked
                        || !(movement.push_items
                            && !map.admits(next)
                            && map.admits_monster_approach(next, true)
                            || movement.push_creatures
                                && runtime.position_occupied_by_other(actor, next) == Ok(true))
                },
            );
            let Some(timer) = fired.into_iter().next() else {
                break;
            };
            let actor = timer.occurrence;
            let Some(entry) = self.entry_mut(actor) else {
                continue;
            };
            let motion = &mut entry.motion;
            motion.due = None;
            let Some((step, next, near)) = motion.prepared.take() else {
                continue;
            };
            let Ok(snapshot) = runtime.read_actor_position(actor) else {
                motion.stop();
                continue;
            };
            if motion.content != runtime.content_pin().server_artifact_digest()
                || motion.expected.is_some_and(|expected| expected != snapshot)
                || !motion.target_is_current(runtime)
                || motion.goal.target().is_some_and(|(target, at)| {
                    !runtime
                        .read_actor_position(target)
                        .is_ok_and(|position| position.position() == at)
                })
                || !admits(cells, blocking, current_map, next)
                || runtime.position_occupied_by_other(actor, next) != Ok(false)
                || runtime.native_summon_role(actor) != Ok(None)
            {
                motion.stop();
                continue;
            }
            // Crystal monster.cpp:1604-1612 checks for returning players on every
            // walk-back step. The movement cadence can fire between two thinks.
            if matches!(motion.goal, MovementGoal::WalkBack { .. })
                && walk_back_player_in_view(
                    runtime,
                    &entry.state.profile,
                    snapshot.position(),
                    players,
                )
            {
                motion.stop();
                entry.due_us = Some(entry.due_us.map_or(now, |due| due.min(now)));
                continue;
            }
            // Preflight the next cadence before publishing a position at an exhausted clock.
            let duration =
                creature_step_duration(runtime, actor, source, ground, cell(next), cadence_now);
            if duration
                .and_then(|duration| duration.checked_mul(if near { 2 } else { 1 }))
                .and_then(|duration| cadence_now.checked_after(duration))
                .is_none()
            {
                motion.stop();
                continue;
            }
            let selection = MovementEngineeringSelection {
                owner_context: snapshot.context(),
                content_scope: cells.scope(),
            };
            let moved = match current_map {
                Some(map) => map.step(runtime, cells, actor, snapshot, step),
                None => step_cardinal(
                    &mut runtime.borrow_movement_position(),
                    actor,
                    snapshot,
                    &selection,
                    cells.index(),
                    step,
                ),
            };
            if moved.is_err() {
                motion.stop();
                continue;
            }
            if let (Ok(after), Some(duration)) = (
                moved.as_ref(),
                duration.and_then(|duration| duration.checked_mul(if near { 2 } else { 1 })),
            ) {
                runtime.record_walk(actor, *after, duration);
            }
            entry.state.attack_facing = step;
            motion.expected = moved.ok();
            if motion.route.is_some() {
                motion.cursor += 1;
            }
            motion.step_sequence = motion.step_sequence.saturating_add(1);
            if let MovementGoal::WalkBack { spawn } = motion.goal
                && (walk_back_player_in_view(runtime, &entry.state.profile, next, players)
                    || spawn == next)
            {
                motion.stop();
                // The next think owns the idle transition and its damage reset.
                entry.due_us = Some(entry.due_us.map_or(now, |due| due.min(now)));
            }
        }
        let pushable = current_map.map(|_| self.current_pushable_occupants(runtime));
        for entry in self.entries.values_mut() {
            let actor = entry.state.actor;
            let motion = &mut entry.motion;
            if motion.due.is_some() || matches!(motion.goal, MovementGoal::Stand) {
                continue;
            }
            let Ok(snapshot) = runtime.read_actor_position(actor) else {
                motion.stop();
                continue;
            };
            let at = snapshot.position();
            if !motion.target_is_current(runtime) {
                motion.stop();
                continue;
            }
            if let Some((target, _)) = motion.goal.target() {
                let Ok(position) = runtime.read_actor_position(target) else {
                    motion.stop();
                    continue;
                };
                let target_at = position.position();
                if target_at.floor != at.floor {
                    motion.stop();
                    continue;
                }
                motion.goal = match motion.goal {
                    MovementGoal::Chase { .. } => MovementGoal::Chase {
                        actor: target,
                        at: target_at,
                    },
                    MovementGoal::KeepDistance { distance, .. } => MovementGoal::KeepDistance {
                        actor: target,
                        at: target_at,
                        distance,
                    },
                    MovementGoal::Flee { .. } => MovementGoal::Flee {
                        actor: target,
                        at: target_at,
                    },
                    other => other,
                };
            }
            if let Some(route) = &motion.route
                && route
                    .revalidate(super::creature_paths::provenance(runtime, actor, motion))
                    .is_err()
            {
                motion.stop();
                continue;
            }
            let near = motion.near.evaluate(at, motion.goal);
            let Some(step) = motion.choose_step(
                actor,
                at,
                entry.state.profile.targeting().static_attack_chance_ppm,
                motion
                    .goal
                    .target()
                    .is_none_or(|(_, target)| sight(cells, blocking, current_map, at, target)),
                root,
                |next| {
                    let movement = &entry.state.profile.behavior().movement;
                    let terrain = current_map.map_or_else(
                        || admits(cells, blocking, None, next),
                        |map| map.admits_monster_approach(next, movement.push_items),
                    );
                    terrain
                        && match runtime.position_occupied_by_other(actor, next) {
                            Ok(false) => true,
                            Ok(true) if movement.push_creatures => pushable
                                .as_ref()
                                .is_some_and(|occupants| occupants.at(next).is_some()),
                            _ => false,
                        }
                },
            ) else {
                // Budget-waiting requests keep their goal/path and hold without retrying a search.
                if motion.pending.is_none() {
                    motion.stop();
                }
                continue;
            };
            let Some(next) = destination(at, step) else {
                motion.stop();
                continue;
            };
            let Some(duration) =
                creature_step_duration(runtime, actor, source, ground, cell(next), cadence_now)
                    .and_then(|duration| duration.checked_mul(if near { 2 } else { 1 }))
            else {
                motion.stop();
                continue;
            };
            let duration = match motion.goal {
                MovementGoal::Wander { interval_ms, .. } => {
                    duration.max(std::time::Duration::from_millis(interval_ms))
                }
                _ => duration,
            };
            let Some(due) = cadence_now
                .checked_after(duration)
                .map(|deadline| deadline.get())
            else {
                motion.stop();
                continue;
            };
            let Ok(stamp) = runtime.issue_owner_work() else {
                motion.stop();
                continue;
            };
            let Ok(fence) = runtime.owner_fence() else {
                motion.stop();
                continue;
            };
            if timers
                .schedule(
                    fence,
                    stamp,
                    StepFamily,
                    actor,
                    Some(actor),
                    SemanticTimeMicros::from_micros(due),
                )
                .is_ok()
            {
                motion.expected = Some(snapshot);
                motion.target_snapshot = motion.goal.target().and_then(|(target, _)| {
                    runtime
                        .read_actor_position(target)
                        .ok()
                        .map(|snapshot| (target, snapshot))
                });
                motion.due = Some(due);
                motion.shove_checked = false;
                motion.prepared = Some((step, next, near));
            }
        }
        self.step_timers = Some(timers);
    }
}

pub(super) fn walk_back_player_in_view(
    runtime: &ChannelRuntimeV1,
    profile: &CreatureBehaviourProfile,
    at: MovementLocalPosition,
    players: &[TargetCandidate],
) -> bool {
    players.iter().any(|player| {
        runtime
            .read_actor_position(player.actor)
            .is_ok_and(|position| {
                targeting::perceives(profile.is_native(), at, position.position())
            })
    })
}
