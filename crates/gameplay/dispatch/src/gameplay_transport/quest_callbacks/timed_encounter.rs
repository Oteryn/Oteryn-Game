//! Bounded source timers share the Channel source-owner loop and native carrier.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use crate::map::view::EntryFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

pub(super) const MAX_TIMED_ENCOUNTERS: usize = 64;

fn draw_below(upper: u16) -> Option<u16> {
    if upper == 0 || upper > 256 {
        return None;
    }
    loop {
        let [byte] = crate::gameplay_transport::SecureIdentifiers::random::<1>()?;
        let roll = u16::from(byte);
        if roll < 256 - 256 % upper {
            return Some(roll % upper);
        }
    }
}

pub(super) struct TimedEncounter {
    callback: Callback,
    key: u64,
    revision: u64,
    content: [u8; 32],
    map: [u8; 32],
    scope: (RuntimeScopeRefV1, ScopeOwnershipGeneration),
    spawn_at: u64,
    reset_at: u64,
    attempted_spawn: bool,
    spawned: Vec<ExactActorRef>,
}
impl TimedEncounter {
    pub(super) fn source_key(&self) -> u64 {
        self.key
    }

    pub(super) fn next_due_us(&self) -> u64 {
        if self.attempted_spawn {
            self.reset_at
        } else {
            self.spawn_at
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "binds the shared timer to the exact native source occurrence"
    )]
    pub(super) fn summon_on_quest_step(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        revision: u64,
        at: MovementLocalPosition,
    ) -> UseDisposition {
        let Some(summon) = &callback.effects.step_summon else {
            return UseDisposition::Rejected;
        };
        if runtime.owner_fence().is_err() || map.facts.object_revision(key) != revision {
            return UseDisposition::StaleState;
        }
        let now = self.owner_now().get();
        let scope = runtime.encounter_owner_scope();
        let content = runtime.content_pin().server_artifact_digest();
        let map_digest = map.overlay.base().digest();
        let Ok(mut pending) = self.states.quest_callbacks.timed.lock() else {
            return UseDisposition::Rejected;
        };
        pending.retain(|memo| {
            memo.callback.effects.step_summon.is_none()
                || (memo.scope == scope
                    && memo.content == content
                    && memo.map == map_digest
                    && now < memo.reset_at
                    && map.facts.object_revision(memo.key) == memo.revision)
        });
        if pending.iter().any(|memo| memo.key == key) {
            return UseDisposition::Committed;
        }
        if pending.len() >= MAX_TIMED_ENCOUNTERS || pending.try_reserve(1).is_err() {
            return UseDisposition::Exhausted;
        }
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return UseDisposition::Rejected;
        };
        if native.install_companion_policies(runtime).is_err() {
            return UseDisposition::Rejected;
        }
        let Ok(policy) = runtime.companion_policy(&summon.creature) else {
            return UseDisposition::Rejected;
        };
        let health = policy.maximum_health;
        let [minimum, maximum] = summon.cooldown_minutes.map(u16::from);
        let Some(roll) = draw_below(maximum - minimum + 1) else {
            return UseDisposition::Rejected;
        };
        let Some(reset_at) = now.checked_add(u64::from(minimum + roll) * 60_000_000) else {
            return UseDisposition::Exhausted;
        };
        // The source AID cools down even if native monster admission refuses.
        if let Some(landing) = self.quest_step_summon_landing(runtime, map, at) {
            let _ =
                runtime.commit_quest_creature_spawn(&[(summon.creature.as_str(), health, landing)]);
        }
        pending.push(TimedEncounter {
            callback: callback.clone(),
            key,
            revision,
            content,
            map: map_digest,
            scope,
            spawn_at: now,
            reset_at,
            attempted_spawn: true,
            spawned: Vec::new(),
        });
        states.owner_wake.notify_one();
        UseDisposition::Committed
    }

    /// Crystal Map::placeCreature retries the eight neighbouring cells in random
    /// order when the arriving player occupies the requested spawn cell.
    fn quest_step_summon_landing(
        &self,
        runtime: &ChannelRuntimeV1,
        map: &BundleMap,
        at: MovementLocalPosition,
    ) -> Option<MovementLocalPosition> {
        let base = map.overlay.base();
        let origin = native(at)?;
        if base
            .tile(origin.x, origin.y, origin.floor)?
            .protection_zone()
        {
            return None;
        }
        let census = runtime.positioned_melee_census().ok()?;
        let cells = self.movement_cells();
        let candidates: Vec<_> = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ]
        .into_iter()
        .filter_map(|(dx, dy)| {
            let landing = MovementLocalPosition {
                x: at.x.checked_add(dx)?,
                y: at.y.checked_add(dy)?,
                floor: at.floor,
            };
            let pos = native(landing)?;
            let cell = LogicalCell {
                x: landing.x,
                y: landing.y,
                z: i32::from(landing.floor),
            };
            (!base.tile(pos.x, pos.y, pos.floor)?.protection_zone()
                && !map.floors.contains(pos)
                && !crate::map::floor::is_teleport(cells, cell)
                && cells.index().lookup(cells.scope(), cell) == Ok(CollisionClass::Walkable)
                && !map.facts.quest_callback_blocked(base, pos, false)
                && !census
                    .iter()
                    .any(|(_, snapshot, _)| snapshot.position() == landing))
            .then_some(landing)
        })
        .collect();
        let index = draw_below(u16::try_from(candidates.len()).ok()?)?;
        candidates.get(usize::from(index)).copied()
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the held native USE occurrence owns one source timer"
    )]
    pub(super) fn schedule_timed_quest_encounter(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        revision: u64,
        after: EntryFacts,
    ) -> UseDisposition {
        let Some(encounter) = &callback.effects.encounter else {
            return UseDisposition::Rejected;
        };
        let Some(timing) = encounter.timing else {
            return UseDisposition::Rejected;
        };
        let personal =
            oteryn_gameplay_quests::quest::world_callbacks::personal_trial::admitted(callback);
        if callback.lever.is_none() && !personal {
            return UseDisposition::Rejected;
        }
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return UseDisposition::Rejected;
        };
        // Bind the authoritative artifact before looking up a native creature. A
        // source switch must not depend on an unrelated summon/AI initialization.
        if native.install_companion_policies(runtime).is_err() {
            return UseDisposition::Rejected;
        }
        if !encounter.players.is_empty()
            || states
                .get(runtime, actor, session)
                .is_none_or(|state| state.character_facts().level < encounter.minimum_level)
            || runtime.owner_fence().is_err()
            || callback.lever.is_some_and(|pair| {
                map.facts.quest_callback_replacement(key, pair[0]).is_none()
                    || map.facts.quest_callback_replacement(key, pair[1]).is_none()
            })
            || encounter
                .monsters
                .iter()
                .any(|monster| runtime.companion_policy(&monster.creature).is_err())
        {
            return UseDisposition::Rejected;
        }
        let now = self.owner_now().get();
        let (Some(spawn_at), Some(reset_at), Some(next_revision)) = (
            now.checked_add(u64::from(timing.spawn_after_ms) * 1000),
            now.checked_add(u64::from(timing.reset_after_ms) * 1000),
            revision.checked_add(1),
        ) else {
            return UseDisposition::Exhausted;
        };
        let Ok(mut pending) = self.states.quest_callbacks.timed.lock() else {
            return UseDisposition::Rejected;
        };
        if pending.len() >= MAX_TIMED_ENCOUNTERS
            || pending.iter().any(|pending| pending.key == key)
            || pending.try_reserve(1).is_err()
        {
            return UseDisposition::Exhausted;
        }
        if !map
            .facts
            .apply_quest_callback(key, revision, after, &[], &[], false, None)
        {
            return UseDisposition::Rejected;
        }
        pending.push(TimedEncounter {
            callback: callback.clone(),
            key,
            revision: next_revision,
            content: runtime.content_pin().server_artifact_digest(),
            map: map.overlay.base().digest(),
            scope: runtime.encounter_owner_scope(),
            spawn_at,
            reset_at,
            attempted_spawn: false,
            spawned: Vec::new(),
        });
        states.owner_wake.notify_one();
        UseDisposition::Committed
    }

    pub(in crate::gameplay_transport) async fn drain_quest_encounter_deadlines(&self) {
        let Some(_permit) = self.spell_lane_permit().await else {
            return;
        };
        let mut runtime = self.runtime.lock().await;
        let Some(map) = self.bundle_map() else { return };
        if runtime.owner_fence().is_err()
            || self
                .active_generation
                .and_then(|active| active.native_gameplay())
                .is_none_or(|native| {
                    native.source_digest() != runtime.content_pin().server_artifact_digest()
                })
        {
            return;
        }
        let Ok(mut pending) = self.states.quest_callbacks.timed.lock() else {
            return;
        };
        let now = self.owner_now().get();
        let mut index = 0;
        while index < pending.len() {
            let memo = &mut pending[index];
            if memo.content != runtime.content_pin().server_artifact_digest()
                || memo.map != map.overlay.base().digest()
                || memo.scope != runtime.encounter_owner_scope()
                || map.facts.object_revision(memo.key) != memo.revision
            {
                pending.remove(index);
                continue;
            }
            if now < memo.next_due_us() {
                index += 1;
                continue;
            }
            if memo.callback.effects.step_summon.is_some() {
                // Crystal re-arms the tile without removing the monster.
                pending.remove(index);
                continue;
            }
            let Some(encounter) = &memo.callback.effects.encounter else {
                pending.remove(index);
                continue;
            };
            let Ok(census) = runtime.current_live_creature_census() else {
                index += 1;
                continue;
            };
            if now >= memo.reset_at {
                if oteryn_gameplay_quests::quest::world_callbacks::personal_trial::admitted(
                    &memo.callback,
                ) {
                    // Personal trials own only their exact spawns, never unrelated room actors.
                    super::personal_trial::cleanup(&mut runtime, &mut memo.spawned);
                    pending.remove(index);
                    continue;
                }
                let Some(pair) = memo.callback.lever else {
                    pending.remove(index);
                    continue;
                };
                let Some(restore) = map.facts.quest_callback_replacement(memo.key, pair[0]) else {
                    index += 1;
                    continue;
                };
                for actor in census {
                    let in_room = runtime
                        .read_actor_position(actor)
                        .ok()
                        .and_then(|snapshot| native(snapshot.position()))
                        .is_some_and(|position| {
                            position.floor == encounter.room_min.floor
                                && (encounter.room_min.x..=encounter.room_max.x)
                                    .contains(&position.x)
                                && (encounter.room_min.y..=encounter.room_max.y)
                                    .contains(&position.y)
                        });
                    if in_room {
                        let _ = runtime.remove_encounter_actor(actor);
                    }
                }
                if map.facts.apply_quest_callback(
                    memo.key,
                    memo.revision,
                    restore,
                    &[],
                    &[],
                    true,
                    None,
                ) {
                    pending.remove(index);
                } else {
                    index += 1;
                }
                continue;
            }
            // Source Game.createMonster attempts once at its due instant. An occupied/full
            // room does not invent a successful spawn or retry after the source deadline.
            let mut monsters = Vec::new();
            let mut admissible = true;
            for monster in &encounter.monsters {
                let at = local(monster.position);
                if self.movement_cells.index().lookup(
                    self.movement_cells.scope(),
                    LogicalCell {
                        x: at.x,
                        y: at.y,
                        z: i32::from(at.floor),
                    },
                ) != Ok(CollisionClass::Walkable)
                    || map.facts.quest_callback_blocked(
                        map.overlay.base(),
                        tile(monster.position),
                        false,
                    )
                    || runtime.positioned_melee_census().map_or(true, |census| {
                        census
                            .iter()
                            .any(|(_, position, _)| position.position() == at)
                    })
                {
                    admissible = false;
                    break;
                }
                let Ok(policy) = runtime.companion_policy(&monster.creature) else {
                    admissible = false;
                    break;
                };
                monsters.push((monster.creature.as_str(), policy.maximum_health, at));
            }
            if admissible && let Ok(spawned) = runtime.commit_quest_creature_spawn(&monsters) {
                memo.spawned = spawned;
            }
            memo.attempted_spawn = true;
            index += 1;
        }
        // Death/despawn and source cleanup release only their source-owned ladder.
        super::monsters::settle_authored_monster_puzzles(&runtime, map);
    }
}
