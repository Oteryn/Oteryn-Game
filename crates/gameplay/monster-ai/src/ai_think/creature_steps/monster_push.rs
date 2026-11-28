use super::creature_step_timer::walk_back_player_in_view;
use super::*;
use crate::durability::spell_item_transaction::SpellItemError;
use crate::movement::pacing::PacingTime;
use crate::movement::{
    CardinalStep, monster_map::CurrentMonsterMap, speed::creature_step_duration,
};
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::collections::BTreeMap;

impl CreatureAiTable {
    /// Revalidate the exact retained step before any custody change. A durable
    /// shove is committed separately, then the ordinary step rereads the tile.
    pub async fn clear_due_monster_step(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        map: &mut CurrentMonsterMap<'_, '_>,
        source: &(impl CurrentCreatureSpeedReader + Sync),
        players: &[TargetCandidate],
        through: u64,
        now: u64,
    ) -> Result<bool, SpellItemError> {
        let Some(mut timers) = self.step_timers.clone() else {
            return Ok(false);
        };
        let Ok(fence) = runtime.owner_fence() else {
            return Ok(false);
        };
        let mut first = true;
        let Some(timer) = timers
            .drain_due_matching(
                &crate::foundation::owner_timer::VirtualOwnerClock::new(
                    crate::foundation::owner_timer::SemanticTimeMicros::from_micros(through),
                ),
                fence,
                |actor| runtime.contains_live_creature(actor),
                |_, _| std::mem::take(&mut first),
            )
            .into_iter()
            .next()
        else {
            return Ok(false);
        };
        let actor = timer.occurrence;
        let Some(entry) = self.entry(actor) else {
            return Ok(false);
        };
        if entry.motion.shove_checked {
            return Ok(false);
        }
        let result = self
            .try_clear_monster_step(runtime, map, source, players, now, actor)
            .await;
        if matches!(result, Ok(false))
            && let Some(entry) = self.entry_mut(actor)
        {
            entry.motion.shove_checked = true;
        }
        result
    }

    async fn try_clear_monster_step(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &mut CurrentMonsterMap<'_, '_>,
        source: &(impl CurrentCreatureSpeedReader + Sync),
        players: &[TargetCandidate],
        now: u64,
        actor: ExactActorRef,
    ) -> Result<bool, SpellItemError> {
        let Some(entry) = self.entry(actor) else {
            return Ok(false);
        };
        let Some(due) = entry.motion.due else {
            return Ok(false);
        };
        let motion = &entry.motion;
        let movement = &entry.state.profile.behavior().movement;
        let Some((_, next, near)) = motion.prepared else {
            return Ok(false);
        };
        let Ok(before) = runtime.read_actor_position(actor) else {
            return Ok(false);
        };
        if !(movement.push_items || movement.push_creatures)
            || motion.content != runtime.content_pin().server_artifact_digest()
            || motion.navigation != runtime.content_pin().map_revision_digest()
            || motion.expected != Some(before)
            || !motion.target_is_current(runtime)
            || motion.goal.target().is_some_and(|(target, at)| {
                !runtime
                    .read_actor_position(target)
                    .is_ok_and(|pose| pose.position() == at)
            })
            || runtime.native_summon_role(actor) != Ok(None)
            || runtime.native_callback_movement_locked(actor) != Ok(false)
            || (matches!(motion.goal, MovementGoal::WalkBack { .. })
                && walk_back_player_in_view(
                    runtime,
                    &entry.state.profile,
                    before.position(),
                    players,
                ))
        {
            return Ok(false);
        }
        let clear = map.read_for_shove(runtime, next).await? == Some(true);
        let clock = SemanticTimeMicros::from_micros(now);
        if !map
            .view()
            .admits_monster_approach(next, movement.push_items)
            || creature_step_duration(runtime, actor, source, map.view(), cell(next), clock)
                .and_then(|duration| duration.checked_mul(if near { 2 } else { 1 }))
                .and_then(|duration| clock.checked_after(duration))
                .is_none()
        {
            return Ok(false);
        }

        let occupied = runtime.position_occupied_by_other(actor, next);
        if occupied.is_err() || (clear && occupied == Ok(false)) {
            return Ok(false);
        }
        let blocker = if occupied == Ok(true) {
            if !movement.push_creatures {
                return Ok(false);
            }
            let Some(blocker) = self.current_pushable_occupants(runtime).at(next) else {
                return Ok(false);
            };
            Some(blocker)
        } else {
            None
        };
        let landings: Vec<_> = [
            CardinalStep::North,
            CardinalStep::East,
            CardinalStep::South,
            CardinalStep::West,
        ]
        .into_iter()
        .filter_map(|direction| destination(next, direction))
        .filter(|at| {
            *at != before.position() && runtime.position_occupied_by_other(actor, *at) == Ok(false)
        })
        .collect();
        let mut creature_landing = None;
        if blocker.is_some() {
            for &landing in &landings {
                if map.read_for_shove(runtime, landing).await? == Some(true) {
                    creature_landing = Some(landing);
                    break;
                }
            }
            if creature_landing.is_none() {
                return Ok(false);
            }
        }
        if !clear {
            let item_landings: Vec<_> = landings
                .into_iter()
                .filter(|landing| Some(*landing) != creature_landing)
                .collect();
            return map
                .push_blocking_items(runtime, actor, due, next, &item_landings)
                .await;
        }
        if let (Some(blocker), Some(landing)) = (blocker, creature_landing)
            && map.validate_shove_transaction(runtime).await?
        {
            let _ = map.view().relocate(runtime, blocker, landing);
        }
        Ok(false)
    }

    pub(super) fn current_pushable_occupants(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> PushableOccupants {
        let mut cells = BTreeMap::new();
        if !self
            .entries
            .values()
            .any(|entry| entry.state.profile.behavior().movement.push_creatures)
            || runtime.owner_fence().is_err()
        {
            return PushableOccupants(cells);
        }
        let Ok(census) = runtime.positioned_melee_census() else {
            return PushableOccupants(cells);
        };
        for (actor, position, session) in census {
            let at = position.position();
            let pushable = session.is_none()
                && self
                    .entry(actor)
                    .is_some_and(|entry| entry.state.profile.behavior().movement.pushable)
                && runtime.native_summon_role(actor) == Ok(None)
                && runtime.native_callback_movement_locked(actor) == Ok(false)
                && runtime
                    .companion_snapshot(actor)
                    .is_ok_and(|snapshot| snapshot.state.master.is_none());
            cells
                .entry((at.x, at.y, at.floor))
                .and_modify(|occupant| *occupant = None)
                .or_insert(pushable.then_some(actor));
        }
        for npc in runtime.visible_entities().npcs {
            let at = npc.position;
            cells.remove(&(at.x, at.y, at.floor));
        }
        PushableOccupants(cells)
    }
}

pub(super) struct PushableOccupants(BTreeMap<(i32, i32, i16), Option<ExactActorRef>>);
impl PushableOccupants {
    pub(super) fn at(&self, at: MovementLocalPosition) -> Option<ExactActorRef> {
        self.0.get(&(at.x, at.y, at.floor)).copied().flatten()
    }
}
