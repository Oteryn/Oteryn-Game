//! Current transaction reads feed the existing goal, search and DEADLINE_STATE owners.
use super::*;
use crate::movement::monster_map::CurrentMonsterMap;
use crate::movement::path_search::PathOccupancy;
use crate::movement::path_search::current_path::{PathTileReader, measured_current_route};

struct PathRead<'a, 'tx, 'db> {
    map: &'a mut CurrentMonsterMap<'tx, 'db>,
    runtime: &'a ChannelRuntimeV1,
    occupancy: &'a PathOccupancy,
    start: MovementLocalPosition,
    target: MovementLocalPosition,
    exact: bool,
}
impl PathTileReader for PathRead<'_, '_, '_> {
    async fn cost(&mut self, at: MovementLocalPosition) -> Option<u32> {
        if (!self.exact && at == self.target) || self.map.read(self.runtime, at).await != Some(true)
        {
            return None;
        }
        Some(self.occupancy.extra_cost(at, self.start))
    }
}
impl CreatureAiTable {
    #[allow(clippy::too_many_arguments)]
    pub async fn drive_current_movement(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        cells: &NativeEntryMovementCells,
        blocking: &dyn CellBlocker,
        source: &(impl CurrentCreatureSpeedReader + Sync),
        root: &GameplayDecisionRoot,
        now: u64,
        budget: &mut ChannelPathBudget,
        player_waiting: bool,
        map: &mut CurrentMonsterMap<'_, '_>,
    ) -> Option<u64> {
        if !map.view().matches(runtime)
            || cells.scope().world_id != runtime.binding().world_id()
            || cells.scope().generation_digest != runtime.content_pin().server_artifact_digest()
        {
            return None;
        }
        self.forget_dead(runtime);
        // All awaited reads precede goal/timer publication. Dropping this future retains inputs,
        // route cursor and cadence. A fresh physical transaction always starts with a fresh cache.
        for entry in self.entries.values() {
            let Ok(snapshot) = runtime.read_actor_position(entry.state.actor) else {
                continue;
            };
            if snapshot.context() != runtime.pinned_movement_context() {
                return None;
            }
            let at = snapshot.position();
            map.read(runtime, at).await;
            for step in [
                crate::movement::CardinalStep::North,
                crate::movement::CardinalStep::East,
                crate::movement::CardinalStep::South,
                crate::movement::CardinalStep::West,
            ] {
                if let Some(next) = destination(at, step) {
                    map.read(runtime, next).await;
                }
            }
            if let Some(home) = entry.state.spawn {
                map.read(runtime, home).await;
            }
            if let Some((_, target)) = entry.state.last.as_ref().and_then(|r| r.goal.target()) {
                map.read_sight(runtime, at, target).await;
            }
            if let Some((_, next, _)) = entry.motion.prepared {
                map.read(runtime, next).await;
            }
            // Reachability from a retained route rereads its entire remaining suffix.
            if let Some(route) = &entry.motion.route {
                let mut cursor = at;
                for step in route.steps().iter().skip(entry.motion.cursor) {
                    let direction = match step.node() {
                        0 => crate::movement::CardinalStep::North,
                        1 => crate::movement::CardinalStep::East,
                        2 => crate::movement::CardinalStep::South,
                        _ => crate::movement::CardinalStep::West,
                    };
                    if let Some(next) = destination(cursor, direction) {
                        map.read(runtime, next).await;
                        cursor = next;
                    }
                }
            }
        }
        if !map.validate_transaction(runtime).await {
            return None;
        }
        let navigation = runtime.content_pin().map_revision_digest();
        self.refresh_goals(
            runtime,
            cells,
            blocking,
            Some(map.view()),
            navigation,
            now,
            source,
        );
        let mut order: Vec<_> = self
            .entries
            .values()
            .filter_map(|entry| {
                Some((
                    entry.motion.pending?,
                    entry.state.actor.actor_local_id(),
                    entry.state.actor,
                ))
            })
            .collect();
        order.sort_by_key(|(due, identity, _)| (*due, *identity));
        budget.declare_creatures(order.len());
        let occupancy = PathOccupancy::current(runtime);
        for (_, _, actor) in order {
            if !budget.admits(now, false, player_waiting) {
                break;
            }
            let Some(entry) = self.entry(actor) else {
                continue;
            };
            let (target, exact) = match entry.motion.goal {
                MovementGoal::Chase { at, .. } | MovementGoal::KeepDistance { at, .. } => {
                    (at, false)
                }
                MovementGoal::WalkBack { spawn } => (spawn, true),
                _ => continue,
            };
            let start = runtime.read_actor_position(actor).ok()?.position();
            let mut reader = PathRead {
                map,
                runtime,
                occupancy: &occupancy,
                start,
                target,
                exact,
            };
            let (route, work) =
                measured_current_route(start, target, exact, &mut reader, budget).await;
            // No await between charging the completed search and retaining its result.
            if let Some(entry) = self.entry_mut(actor) {
                super::creature_paths::install_route(entry, runtime, start, route, work);
            }
        }
        if !map.validate_transaction(runtime).await {
            return None;
        }
        self.fire_steps(
            runtime,
            cells,
            blocking,
            Some(map.view()),
            source,
            map.view(),
            root,
            now,
        );
        let next = self
            .entries
            .values()
            .filter_map(|entry| entry.motion.due)
            .min();
        if self
            .entries
            .values()
            .any(|entry| entry.motion.pending.is_some())
        {
            let window = (now / crate::movement::path_budget::PATH_WINDOW_US)
                .checked_add(1)?
                .checked_mul(crate::movement::path_budget::PATH_WINDOW_US)?;
            Some(next.map_or(window, |next| next.min(window)))
        } else {
            next
        }
    }
}

impl CreatureAiTable {
    pub(in crate::ai_think) fn retained_summon_route(
        &self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
    ) -> Option<(ExactActorRef, Vec<MovementLocalPosition>)> {
        let entry = self.entry(actor)?;
        let motion = &entry.motion;
        let (target, target_at) = motion.goal.target()?;
        if motion.content != runtime.content_pin().server_artifact_digest()
            || motion.navigation != runtime.content_pin().map_revision_digest()
            || runtime.read_actor_position(target).ok()?.position() != target_at
        {
            return None;
        }
        let snapshot = runtime.read_actor_position(actor).ok()?;
        let mut at = snapshot.position();
        if motion.expected != Some(snapshot) || !motion.target_is_current(runtime) {
            return None;
        }
        let mut suffix = Vec::new();
        for step in motion.route.as_ref()?.steps().iter().skip(motion.cursor) {
            let direction = match step.node() {
                0 => crate::movement::CardinalStep::North,
                1 => crate::movement::CardinalStep::East,
                2 => crate::movement::CardinalStep::South,
                3 => crate::movement::CardinalStep::West,
                _ => return None,
            };
            at = destination(at, direction)?;
            suffix.push(at);
        }
        (crate::movement::path_search::chebyshev(at, target_at) == 1).then_some((target, suffix))
    }
}
