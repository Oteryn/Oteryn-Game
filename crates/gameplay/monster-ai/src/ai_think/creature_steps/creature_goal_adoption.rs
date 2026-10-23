//! Adopt think goals and bind path requests to the exact content, map and spawn owner.
use super::*;

impl CreatureAiTable {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn refresh_goals(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &NativeEntryMovementCells,
        blocking: &dyn CellBlocker,
        current_map: Option<&crate::movement::monster_map::MonsterMap>,
        navigation: [u8; 32],
        now: u64,
        source: &dyn CurrentCreatureSpeedReader,
    ) {
        for entry in self.entries.values_mut() {
            let Some(report) = entry.state.last.as_ref() else {
                continue;
            };
            if entry.motion.seen_think == report.sequence {
                if entry.motion.content != runtime.content_pin().server_artifact_digest()
                    || entry.motion.navigation != navigation
                {
                    entry.motion.stop();
                }
                continue;
            }
            entry.motion.seen_think = report.sequence;
            let goal = report.goal;
            let actor = entry.state.actor;
            let Ok(snapshot) = runtime.read_actor_position(actor) else {
                continue;
            };
            let at = snapshot.position();
            if let Some(spawn) = entry.state.spawn
                && ((i64::from(at.x) - i64::from(spawn.x)).abs() > 50
                    || (i64::from(at.y) - i64::from(spawn.y)).abs() > 50
                    || (i32::from(at.floor) - i32::from(spawn.floor)).abs() > 2)
            {
                let _ = match current_map {
                    Some(map) => map.relocate(runtime, actor, spawn),
                    None => crate::movement::relocate::relocate_creature(
                        runtime, actor, cells, blocking, spawn,
                    ),
                };
                entry.due_us = None;
                entry.state.target = None;
                entry.motion.stop();
                continue;
            }
            if crate::movement::speed::runtime_creature_speed(
                runtime,
                actor,
                source,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now),
            )
            .is_none()
            {
                entry.motion.stop();
                continue;
            }
            let content = runtime.content_pin().server_artifact_digest();
            if entry.motion.goal != goal
                || entry.motion.content != content
                || entry.motion.navigation != navigation
            {
                entry.motion.stop();
            }
            entry.motion.goal = goal;
            entry.motion.content = content;
            entry.motion.navigation = navigation;
            if matches!(goal, MovementGoal::Stand) {
                entry.motion.stop();
            }
            if entry.motion.route.is_none()
                && entry.motion.pending.is_none()
                && match goal {
                    MovementGoal::Chase { at: target, .. } => {
                        crate::movement::path_search::chebyshev(at, target) > 1
                    }
                    MovementGoal::KeepDistance {
                        at: target,
                        distance,
                        ..
                    } => {
                        !sight(cells, blocking, current_map, at, target)
                            || (goal
                                .local_step(at, 1_000_000, 0, |next| {
                                    admits(cells, blocking, current_map, next)
                                        && runtime.position_occupied_by_other(actor, next)
                                            == Ok(false)
                                })
                                .is_none()
                                && crate::movement::path_search::chebyshev(at, target)
                                    != i64::from(distance))
                    }
                    MovementGoal::WalkBack { spawn } => at != spawn,
                    _ => false,
                }
            {
                entry.motion.pending = Some(now);
            }
        }
    }
}
