//! Bounded source timers share the Channel source-owner loop and native carrier.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use crate::map::{
    facts::QuestCallbackProjection,
    view::{EntryFacts, EntrySource, MapFacts},
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

pub(super) const MAX_TIMED_ENCOUNTERS: usize = 64;

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
    merchant_loot: Option<(u64, Vec<(TilePos, u64)>)>,
}
impl TimedEncounter {
    pub(super) fn holds_source_key(&self, key: u64) -> bool {
        self.key == key && self.merchant_loot.is_none()
    }

    pub(super) fn overlaps_merchant_wave(&self, callback: &Callback) -> bool {
        if !self.callback.starts_merchant_wave()
            || !callback.starts_merchant_wave()
            || self.merchant_loot.is_some()
        {
            return false;
        }
        let Some((active, requested)) = self
            .callback
            .effects
            .encounter
            .as_ref()
            .zip(callback.effects.encounter.as_ref())
        else {
            return false;
        };
        active.room_min.floor == requested.room_min.floor
            && active.room_min.x <= requested.room_max.x
            && requested.room_min.x <= active.room_max.x
            && active.room_min.y <= requested.room_max.y
            && requested.room_min.y <= active.room_max.y
    }

    pub(super) fn next_due_us(&self) -> u64 {
        if let Some((deadline, _)) = &self.merchant_loot {
            *deadline
        } else if self.attempted_spawn {
            self.reset_at
        } else {
            self.spawn_at
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
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
        projection: Option<&QuestCallbackProjection>,
    ) -> UseDisposition {
        let Some(encounter) = &callback.effects.encounter else {
            return UseDisposition::Rejected;
        };
        let Some(timing) = encounter.timing else {
            return UseDisposition::Rejected;
        };
        let personal =
            oteryn_gameplay_quests::quest::world_callbacks::personal_trial::admitted(callback);
        let merchant = callback.starts_merchant_wave()
            && matches!(
                projection,
                Some(QuestCallbackProjection::ItemPresence { .. })
            );
        if callback.lever.is_none() && !personal && !merchant {
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
            || pending.iter().any(|pending| {
                pending.holds_source_key(key) || pending.overlaps_merchant_wave(callback)
            })
            || pending.try_reserve(1).is_err()
        {
            return UseDisposition::Exhausted;
        }
        if !map.facts.apply_quest_callback(
            key,
            revision,
            after,
            &[],
            &[],
            false,
            if merchant { projection } else { None },
        ) {
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
            merchant_loot: None,
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
                || (memo.merchant_loot.is_none()
                    && map.facts.object_revision(memo.key) != memo.revision)
            {
                pending.remove(index);
                continue;
            }
            if now < memo.next_due_us() {
                index += 1;
                continue;
            }
            let Some(encounter) = &memo.callback.effects.encounter else {
                pending.remove(index);
                continue;
            };
            if let Some((_, loot)) = memo.merchant_loot.take() {
                for (pos, key) in loot {
                    map.facts.remove_movement_item(pos, key, 12189);
                }
                pending.remove(index);
                continue;
            }
            if memo.callback.starts_merchant_wave() && now >= memo.reset_at {
                super::personal_trial::cleanup(&mut runtime, &mut memo.spawned);
                let mut loot = Vec::new();
                for x in encounter.room_min.x..=encounter.room_max.x {
                    for y in encounter.room_min.y..=encounter.room_max.y {
                        let pos = TilePos {
                            x,
                            y,
                            floor: encounter.room_min.floor,
                        };
                        for entry in map.facts.movement_items(pos) {
                            let EntrySource::Added(id) = entry.source else {
                                continue;
                            };
                            let key = id.get();
                            if entry.facts.appearance_id == 12189
                                || (entry.facts.appearance_id == 12187
                                    && map.facts.compare_transform_movement_item(
                                        key,
                                        map.facts.object_revision(key),
                                        12189,
                                    ))
                            {
                                loot.push((pos, key));
                            }
                        }
                    }
                }
                memo.merchant_loot = Some((now.saturating_add(300_000_000), loot));
                index += 1;
                continue;
            }
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
