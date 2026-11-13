//! Durable callback commit followed by its prepared map effects.
use super::*;
use crate::durability::character_revision_sequencer::RevisionSlot;

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn execute_quest_callback(
        &self,
        permit: &SpellLanePermit,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        expected_revision: u64,
        cause: QuestCause,
        fence: Option<CurrentCharacterGameplayFence>,
        slot: Option<&mut RevisionSlot>,
        observed_world: Option<&ObservedWorldCallback>,
        optional_pvp: Option<bool>,
        read_only_gate: Option<&ReadOnlyGate>,
        selected_tool: Option<&([u8; 16], Vec<u8>)>,
    ) -> UseDisposition {
        if callback.effects.encounter.is_none()
            && let Some(catalogue) = self.quest_catalogue.as_deref()
            && let Some(bindings) = catalogue.authored_bindings()
        {
            let sources: Vec<_> = bindings
                .quests
                .iter()
                .filter(|quest| quest.quest == callback.quest)
                .flat_map(|quest| &quest.stages)
                .filter_map(|stage| match &stage.trigger {
                    oteryn_gameplay_quests::quest::authored_bindings::Trigger::Kill { counter }
                        if callback.storage.iter().any(|gate| {
                            gate.track == counter.track && gate.minimum >= counter.maximum
                        }) =>
                    {
                        counter.encounter_source.as_deref()
                    }
                    _ => None,
                })
                .collect();
            if !sources.is_empty() {
                let Some(hash) = catalogue.definition_hash(&callback.quest) else {
                    return UseDisposition::Rejected;
                };
                let Ok(lease) = runtime.bound_attacker_lease(actor, session) else {
                    return UseDisposition::Rejected;
                };
                let Ok(attack) = self.states.attack.try_lock() else {
                    return UseDisposition::Rejected;
                };
                let Ok(capture) = attack.kills().1.lock() else {
                    return UseDisposition::Rejected;
                };
                if sources.iter().any(|source| {
                    !capture.encounter_door_allowed(
                        source,
                        hash,
                        actor,
                        session,
                        lease.generation(),
                    )
                }) {
                    return UseDisposition::Rejected;
                }
            }
        }
        if !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|f| f.control_loss.is_none())
            || (callback.requires_live_drunk()
                && !states.get(runtime, actor, session).is_some_and(|state| {
                    state.owned_conditions().has_status(
                        oteryn_gameplay_domain::domain::condition::StatusKind::Drunk,
                        self.owner_now().get(),
                    )
                }))
            || states.is_dead(actor)
            || !states.spell_idle(runtime, actor, session)
            || super::super::item_ref_admission::pinned_index(runtime, self.active_generation)
                .is_none()
            || !self.states.player_trades.physical(session, None)
        {
            return UseDisposition::Rejected;
        }
        if callback.effects.party_transfer.is_some()
            && self
                .states
                .quest_callbacks
                .records
                .lock()
                .map_or(true, |records| {
                    records.iter().any(|(owner, pending)| {
                        *owner != session && pending.key == key && !pending.participants.is_empty()
                    })
                })
        {
            return UseDisposition::Rejected;
        }
        let (pos, mut after) =
            if oteryn_gameplay_items::event_planting::world_change_item_source(key).is_some() {
                let Some(source) = shared_added_source(map, callback, key) else {
                    return UseDisposition::Rejected;
                };
                source
            } else {
                let Some((pos, ordinal)) = crate::map::view::placement_of(key) else {
                    return UseDisposition::Rejected;
                };
                let Some(base) = map.overlay.base().tile(pos.x, pos.y, pos.floor) else {
                    return UseDisposition::Rejected;
                };
                let Some(id) = base
                    .ids()
                    .iter()
                    .zip(base.depths())
                    .filter(|(_, d)| **d == 0)
                    .nth(usize::from(ordinal))
                    .map(|(id, _)| *id)
                else {
                    return UseDisposition::Rejected;
                };
                let Some(after) = map.facts.base_entry(pos, ordinal, id) else {
                    return UseDisposition::Rejected;
                };
                (pos, after)
            };
        if map.facts.object_revision(key) != expected_revision {
            // A known-commit source visual batch may have published its first
            // fixed object before a retry. Only that retained exact post-CAS
            // revision is replayable; a later party's revision never aliases it.
            let retained_flame = self
                .states
                .quest_callbacks
                .records
                .lock()
                .ok()
                .and_then(|records| records.get(&session).and_then(|p| p.flame.clone()));
            let replayed_visual = retained_flame.as_ref().is_some_and(|input| {
                input.current_scope(runtime)
                    && input.visual_changes().iter().flatten().any(|visual| {
                        visual.key == key
                            && map.facts.object_revision(key) == visual.revision + 1
                            && after.appearance_id == visual.after
                    })
            });
            if !replayed_visual {
                return UseDisposition::StaleState;
            }
        }
        if goblin_elevator::owns(&callback.key) {
            if callback.selector.appearance != Some(after.appearance_id) {
                return UseDisposition::NothingToUse;
            }
            return self.execute_goblin_elevator(
                runtime,
                states,
                actor,
                session,
                map,
                callback,
                optional_pvp,
            );
        }
        if !oteryn_gameplay_quests::quest::source_world_objects::source_is_ready(
            callback,
            after.appearance_id,
        ) {
            return UseDisposition::NothingToUse;
        }
        if callback.effects.reward_door.is_some() {
            return self
                .execute_reward_door(
                    runtime,
                    states,
                    actor,
                    session,
                    map,
                    callback,
                    key,
                    expected_revision,
                    fence,
                    optional_pvp,
                )
                .await;
        }
        let tree_reset = oteryn_gameplay_quests::quest::dreamer_death_ring::shared_reset(callback);
        let close = tree_reset
            || callback.trigger == Trigger::StepOut
            || callback
                .lever
                .is_some_and(|pair| after.appearance_id == pair[1]);
        if !close
            && (!world_requirements_match(map, callback)
                || !self.ground_requirements_match(runtime, map, callback)
                || !super::clear_area::area_is_clear(runtime, callback))
        {
            if !world_requirements_match(map, callback)
                && self.ground_requirements_match(runtime, map, callback)
                && super::clear_area::area_is_clear(runtime, callback)
                && super::timed_passages::lion_is_busy(map, callback)
            {
                return UseDisposition::NothingToUse;
            }
            return UseDisposition::Rejected;
        }
        if close && callback.lever.is_some() && !callback.reversible {
            return UseDisposition::NothingToUse;
        }
        if oteryn_gameplay_quests::quest::world_callbacks::personal_trial::admitted(callback)
            && !self.personal_trial_ready(runtime, callback, key)
        {
            return UseDisposition::Occupied;
        }
        let before = after;
        if let Some(pair) = callback.lever {
            after.appearance_id = pair[usize::from(!close)];
        }
        if let Some(appearance) = callback.effects.transform {
            let Some(replacement) = map.facts.quest_callback_replacement(key, appearance) else {
                return UseDisposition::Rejected;
            };
            after = replacement;
        }
        let mut projection = match self.prepare_quest_projection(map, callback, close, &mut after) {
            Ok(projection) => projection,
            Err(result) => {
                #[cfg(test)]
                if callback.effects.scene.is_some() {
                    eprintln!("QUEST_SCENE_REFUSED projection close={close} result={result:?}");
                }
                return result;
            }
        };
        // Both lifecycle deadlines belong to this operation, even when preparing
        // its remote object read set takes time.
        let lifecycle_now = self.owner_now().get();
        if !close && let Some(delay) = callback.effects.reset_after_ms {
            let Some(due) = lifecycle_now.checked_add(u64::from(delay) * 1_000) else {
                return UseDisposition::Exhausted;
            };
            projection = Some(crate::map::facts::QuestCallbackProjection::TimedTransform {
                due,
                restore: before,
            });
        }
        if callback.key == oteryn_gameplay_quests::quest::dreamer_fire_bug::IGNITE {
            let Some(crate::map::facts::QuestCallbackProjection::TimedTransform { due, restore }) =
                projection
            else {
                return UseDisposition::Rejected;
            };
            projection = Some(crate::map::facts::QuestCallbackProjection::TimedEntrance {
                due,
                restore,
                position: tile(oteryn_gameplay_quests::quest::dreamer_fire_bug::SOURCE),
                destination: tile(oteryn_gameplay_quests::quest::dreamer_fire_bug::DESTINATION),
            });
        }
        projection = match super::conditional_cells::prepare(map, callback, projection) {
            Ok(projection) => projection,
            Err(result) => return result,
        };
        let projection_target = match &projection {
            Some(crate::map::facts::QuestCallbackProjection::TimedTargetTransform {
                key, ..
            }) => Some(*key),
            _ => None,
        };
        if let Some(target) =
            projection_target.or_else(|| callback.has_fenced_regrowth_reward().then_some(key))
            && !self
                .states
                .quest_callbacks
                .source_target_available(actor, session, &cause, key, target)
        {
            return UseDisposition::Rejected;
        }
        if callback
            .item_on_target
            .as_ref()
            .is_some_and(|item| item.target_remove)
        {
            if projection.is_some() {
                return UseDisposition::Rejected;
            }
            projection = Some(crate::map::facts::QuestCallbackProjection::ItemCycle {
                position: pos,
                remove: Some(key),
                create: None,
            });
        }
        if let Some(appearance) = callback
            .item_on_target
            .as_ref()
            .and_then(|item| item.target_replacement)
        {
            let Some(replacement) = map.facts.quest_callback_replacement(key, appearance) else {
                return UseDisposition::Rejected;
            };
            after = replacement;
        }
        let closes_source = callback
            .passages
            .iter()
            .any(|p| p.replacement_passable == Some(false));
        let closing_position = if closes_source {
            Some(pos)
        } else if let Some(crate::map::facts::QuestCallbackProjection::ItemCycle {
            position,
            create: Some(_),
            ..
        }) = &projection
        {
            Some(*position)
        } else {
            None
        };
        if let Some(position) = closing_position {
            let Some(at) = runtime
                .read_actor_position(actor)
                .ok()
                .and_then(|p| native(p.position()))
            else {
                return UseDisposition::Rejected;
            };
            if super::super::map_door::doorway_occupied(runtime, actor, at, position) {
                return UseDisposition::Occupied;
            }
        }
        let mut restored_trees = Vec::new();
        let mut passages = Vec::new();
        let mut replacements = Vec::new();
        for passage_definition in &callback.passages {
            let passage = passage_definition;
            let p = tile(passage.position);
            if (close || passage_definition.replacement_passable == Some(false))
                && !tree_reset
                && super::super::map_door::doorway_occupied(
                    runtime,
                    actor,
                    native(runtime.read_actor_position(actor).map_or(
                        MovementLocalPosition {
                            x: 0,
                            y: 0,
                            floor: 0,
                        },
                        |s| s.position(),
                    ))
                    .unwrap_or(pos),
                    p,
                )
            {
                return UseDisposition::Occupied;
            }
            let Some(t) = map.overlay.base().tile(p.x, p.y, p.floor) else {
                return UseDisposition::Rejected;
            };
            let candidates = t
                .ids()
                .iter()
                .zip(t.depths())
                .filter(|(_, d)| **d == 0)
                .enumerate()
                .filter_map(|(o, (id, _))| {
                    let ordinal = u8::try_from(o).ok()?;
                    let k = placement_key(p, ordinal)?;
                    let f = map.facts.base_entry(p, ordinal, *id)?;
                    ((f.appearance_id == passage.appearance
                        || passage.replacement == Some(f.appearance_id)
                        || passage.create_on_close == Some(f.appearance_id))
                        && !map.facts.house_tile(p)
                        && (!passage.if_present || !map.facts.base_hidden(k)))
                    .then_some(k)
                })
                .collect::<Vec<_>>();
            if candidates.is_empty() && passage.if_present {
                continue;
            }
            let [passage] = candidates.as_slice() else {
                return UseDisposition::Rejected;
            };
            if let Some(appearance) = passage_definition.create_on_close {
                let Some(created) = map.facts.quest_callback_replacement(*passage, appearance)
                else {
                    return UseDisposition::Rejected;
                };
                let Some((_, wall_ordinal)) = crate::map::view::placement_of(*passage) else {
                    return UseDisposition::Rejected;
                };
                let remove = !close
                    && !map.facts.base_hidden(*passage)
                    && t.ids()
                        .iter()
                        .zip(t.depths())
                        .filter(|(_, d)| **d == 0)
                        .nth(usize::from(wall_ordinal))
                        .and_then(|(id, _)| map.facts.base_entry(p, wall_ordinal, *id))
                        .is_some_and(|f| f.appearance_id == passage_definition.appearance);
                if !remove && !close {
                    // Source line9-10 creates1025 without transforming lever2772.
                    after.appearance_id =
                        callback.lever.map_or(after.appearance_id, |pair| pair[0]);
                }
                return if map.facts.apply_quest_wall_cycle(
                    key,
                    expected_revision,
                    after,
                    *passage,
                    created,
                    remove,
                ) {
                    UseDisposition::Committed
                } else {
                    UseDisposition::Exhausted
                };
            }
            // Crystal giant_smithhammer/lever.lua:7-14 handles every USE but
            // changes neither object in the two discordant source states.
            if callback.key == "oteryn:quest-callback/giant-smithhammer"
                && map.facts.base_hidden(*passage) != close
            {
                return UseDisposition::Committed;
            }
            // Source asymmetric levers refuse to toggle if a remote passage is
            // already in the requested state. Check before any map/quest write.
            if passage_definition.require_state && map.facts.base_hidden(*passage) != close {
                return UseDisposition::NothingToUse;
            }
            if let Some(appearance) = passage_definition.replacement {
                let Some(facts) = map.facts.quest_callback_replacement(
                    *passage,
                    if close {
                        passage_definition.appearance
                    } else {
                        appearance
                    },
                ) else {
                    return UseDisposition::Rejected;
                };
                replacements.push((
                    *passage,
                    facts,
                    passage_definition.replacement_passable.unwrap_or(!close),
                ));
            } else {
                if tree_reset && map.facts.base_hidden(*passage) {
                    restored_trees.push(p);
                }
                passages.push(*passage);
            }
        }
        projection = match super::timed_passages::prepare(
            map,
            callback,
            (key, expected_revision),
            lifecycle_now,
            projection,
            &mut passages,
            &mut replacements,
        ) {
            Ok(projection) => projection,
            Err(result) => return result,
        };
        let projection_target = match &projection {
            Some(crate::map::facts::QuestCallbackProjection::TimedTargetTransform {
                key, ..
            }) => Some(*key),
            Some(crate::map::facts::QuestCallbackProjection::TimedPassages { entries, .. })
                if entries.len() == 1 =>
            {
                entries.first().map(|entry| entry.key)
            }
            _ => None,
        };
        if let Some(target) = projection_target
            && !self
                .states
                .quest_callbacks
                .source_target_available(actor, session, &cause, key, target)
        {
            return UseDisposition::Exhausted;
        }
        let Ok(expected) = runtime.read_actor_position(actor) else {
            return UseDisposition::Rejected;
        };
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let blocked = super::super::pvp_live::pz_blocked(states, actor, session, self.owner_now());
        let mut destination = callback.teleport;
        if let Some(remote) = &callback.remote_teleport {
            let p = tile(remote.position);
            let Some(base) = map.overlay.base().tile(p.x, p.y, p.floor) else {
                return UseDisposition::Rejected;
            };
            let present = base
                .ids()
                .iter()
                .zip(base.depths())
                .filter(|(_, depth)| **depth == 0)
                .enumerate()
                .any(|(ordinal, (id, _))| {
                    let Ok(ordinal) = u8::try_from(ordinal) else {
                        return false;
                    };
                    let Some(key) = placement_key(p, ordinal) else {
                        return false;
                    };
                    !map.facts.base_hidden(key)
                        && map
                            .facts
                            .base_entry(p, ordinal, *id)
                            .is_some_and(|facts| facts.appearance_id == remote.appearance)
                });
            if present {
                destination = Some(remote.destination);
            }
        }
        let scene_moves =
            match self.prepare_quest_scene_moves(runtime, states, map, callback, close) {
                Ok(proof) => proof,
                Err(disposition) => {
                    #[cfg(test)]
                    eprintln!("QUEST_SCENE_REFUSED prepare close={close} result={disposition:?}");
                    return disposition;
                }
            };
        let scene_observations = scene_moves
            .as_ref()
            .map(|proof| proof.observations())
            .unwrap_or_default();
        use crate::foundation::MovementFacing;
        let facing = match callback.facing {
            None => None,
            Some(0) => Some(MovementFacing::North),
            Some(1) => Some(MovementFacing::East),
            Some(2) => Some(MovementFacing::South),
            Some(3) => Some(MovementFacing::West),
            Some(_) => return UseDisposition::Rejected,
        };
        let teleport = if let Some(recipe) =
            oteryn_gameplay_quests::quest::carried_step::recipe(&callback.key)
        {
            let (Some(fence), QuestCause::Step(command), Some(native)) = (
                fence,
                cause,
                self.active_generation.and_then(|g| g.native_gameplay()),
            ) else {
                return UseDisposition::Rejected;
            };
            let prepare = |to| {
                crate::map::floor::prepare_scripted_quest_teleport_facing(
                    runtime,
                    self.movement_cells(),
                    actor,
                    session,
                    expected,
                    local(to),
                    blocked,
                    &closed,
                    facing.or(callback.effects.facing.map(super::effects::facing)),
                    callback.effects.scripted_teleport,
                )
                .ok()
            };
            let success = prepare(recipe.success);
            let missing = prepare(recipe.missing);
            let context = QuestWorldContext {
                map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                content_revision: format!(
                    "sha256:{}",
                    hex(&runtime.content_pin().server_artifact_digest())
                ),
                placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                reset_epoch: self.reset_epoch,
                actor_placement: actor.placement_identity(),
                movement_revision: expected.revision(),
                party: Vec::new(),
            };
            let context = if let Ok(mut records) = self.states.quest_callbacks.records.lock() {
                let pending = records.entry(session).or_insert(Pending {
                    actor,
                    session,
                    expected,
                    callback: callback.key.clone(),
                    key,
                    revision: expected_revision,
                    cause,
                    origin: None,
                    context: None,
                    flame: None,
                    ingress_settled: false,
                    participants: Vec::new(),
                    flame_effect_sent: false,
                    projection_target: None,
                });
                pending.context.get_or_insert(context).clone()
            } else {
                return UseDisposition::Rejected;
            };
            let mut placement = map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            // Reserve the same projection capacity before any physical debit.
            // The original post-prepare check below remains for ordinary callbacks.
            if !map
                .facts
                .quest_callback_capacity(key, expected_revision, callback.passages.len())
            {
                if let Ok(mut records) = self.states.quest_callbacks.records.lock() {
                    records.remove(&session);
                }
                return UseDisposition::Exhausted;
            }
            let result = self
                .root
                .commit_carried_step(
                    permit,
                    self.character,
                    self.holder,
                    fence,
                    command,
                    native,
                    crate::durability::quest_world_callback::carried_step::Request {
                        recipe: recipe.key.into(),
                        placement,
                        object_revision: expected_revision,
                        context,
                        success_prepared: success.is_some(),
                        missing_prepared: missing.is_some(),
                    },
                )
                .await;
            match result {
                Ok(Some(receipt)) => {
                    #[cfg(test)]
                    if self
                        .states
                        .quest_callbacks
                        .interrupt_after_commit
                        .swap(false, std::sync::atomic::Ordering::AcqRel)
                    {
                        return UseDisposition::Rejected;
                    }
                    let selected = if receipt.consumed { success } else { missing };
                    let Some(selected) = selected else {
                        return UseDisposition::NotPossible;
                    };
                    Some(selected)
                }
                Ok(None) => {
                    if let Ok(mut records) = self.states.quest_callbacks.records.lock() {
                        records.remove(&session);
                    }
                    return UseDisposition::Rejected;
                }
                Err(error) => {
                    #[cfg(test)]
                    eprintln!("CARRIED_STEP commit error: {error:?}");
                    #[cfg(not(test))]
                    let _ = error;
                    return UseDisposition::Rejected;
                }
            }
        } else {
            match destination {
                Some(to) => match crate::map::floor::prepare_scripted_quest_teleport_facing(
                    runtime,
                    self.movement_cells(),
                    actor,
                    session,
                    expected,
                    local(to),
                    blocked,
                    &closed,
                    facing.or(callback.effects.facing.map(super::effects::facing)),
                    callback.effects.scripted_teleport,
                ) {
                    Ok(proof) => Some(proof),
                    Err(_) => return UseDisposition::NotPossible,
                },
                None => None,
            }
        };
        let source_flame =
            oteryn_gameplay_quests::quest::rotten_blood::flames::by_key(&callback.key).is_some();
        if !callback.is_read_only()
            && !map.facts.quest_callback_capacity(
                key,
                if source_flame {
                    map.facts.object_revision(key)
                } else {
                    expected_revision
                },
                if source_flame {
                    3
                } else {
                    callback.passages.len()
                },
            )
        {
            return UseDisposition::Exhausted;
        }
        if oteryn_gameplay_quests::quest::shared_switches::recipe(&callback.key).is_some() {
            if expected_revision.checked_add(2).is_none()
                || !map.facts.quest_callback_capacity(key, expected_revision, 8)
            {
                return UseDisposition::Exhausted;
            }
            let (Some(fence), QuestCause::Use(command), Some(native)) = (
                fence,
                cause,
                self.active_generation.and_then(|g| g.native_gameplay()),
            ) else {
                return UseDisposition::Rejected;
            };
            let context = QuestWorldContext {
                map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                content_revision: format!(
                    "sha256:{}",
                    hex(&runtime.content_pin().server_artifact_digest())
                ),
                placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                reset_epoch: self.reset_epoch,
                actor_placement: actor.placement_identity(),
                movement_revision: expected.revision(),
                party: Vec::new(),
            };
            let context = if let Ok(mut records) = self.states.quest_callbacks.records.lock() {
                let pending = records.entry(session).or_insert(Pending {
                    actor,
                    session,
                    expected,
                    callback: callback.key.clone(),
                    key,
                    revision: expected_revision,
                    cause,
                    origin: None,
                    context: None,
                    flame: None,
                    ingress_settled: false,
                    participants: Vec::new(),
                    flame_effect_sent: false,
                    projection_target: None,
                });
                pending.context.get_or_insert(context).clone()
            } else {
                return UseDisposition::Rejected;
            };
            let mut placement = map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let result = self
                .root
                .activate_shared_switch(
                    permit,
                    self.character,
                    self.holder,
                    fence,
                    command,
                    native,
                    {
                        #[allow(unused_mut)]
                        let mut request =
                            crate::durability::quest_world_callback::shared_switch::Request::new(
                                callback.key.clone(),
                                placement,
                                expected_revision,
                                context,
                            );
                        #[cfg(test)]
                        if let Some(occurred) = self
                            .states
                            .quest_callbacks
                            .shared_switch_occurred_test
                            .lock()
                            .ok()
                            .and_then(|v| *v)
                        {
                            request = request.for_test_occurrence(occurred);
                        }
                        request
                    },
                )
                .await;
            match result {
                Ok(Some(receipt)) => {
                    // The response observation is current even on replay. Anchor after
                    // the RPC so transport latency cannot restore the lever early.
                    let remaining = receipt
                        .expires_at_unix_ms
                        .saturating_sub(receipt.observed_at_unix_ms)
                        .max(0);
                    let Some(due) = u64::try_from(remaining)
                        .ok()
                        .and_then(|ms| ms.checked_mul(1_000))
                        .and_then(|us| self.owner_now().get().checked_add(us))
                    else {
                        return UseDisposition::Rejected;
                    };
                    projection = Some(crate::map::facts::QuestCallbackProjection::TimedTransform {
                        due,
                        restore: before,
                    });
                    #[cfg(test)]
                    if self
                        .states
                        .quest_callbacks
                        .interrupt_after_commit
                        .swap(false, std::sync::atomic::Ordering::AcqRel)
                    {
                        return UseDisposition::Rejected;
                    }
                }
                Ok(None) => {
                    if let Ok(mut records) = self.states.quest_callbacks.records.lock() {
                        records.remove(&session);
                    }
                    return UseDisposition::Rejected;
                }
                Err(error) => {
                    #[cfg(test)]
                    eprintln!("SHARED_SWITCH commit error: {error:?}");
                    #[cfg(not(test))]
                    let _ = error;
                    return UseDisposition::Rejected;
                }
            }
        }
        if !self
            .admit_shared_switch_gate(
                permit,
                runtime,
                actor,
                session,
                map,
                callback,
                key,
                expected_revision,
                cause,
                fence,
                expected,
            )
            .await
        {
            return UseDisposition::Rejected;
        }
        if stateless_observation_required(callback) {
            let Some(observed) = observed_world else {
                return UseDisposition::Rejected;
            };
            if runtime.owner_fence().is_err()
                || observed.actor != actor
                || observed.session != session
                || observed.callback != callback.key
                || observed.key != key
                || observed.revision != expected_revision
                || observed.expected != expected
                || runtime.read_actor_position(actor) != Ok(observed.expected)
                || map.facts.object_revision(key) != expected_revision
                || !matches(map, callback, key)
                || observed.context.reset_epoch != self.reset_epoch
                || observed.context.map_revision
                    != crate::map::overlay::map_revision(map.overlay.base())
                || observed.context.placement_context.as_slice()
                    != runtime.content_pin().frame_binding_digest().as_slice()
                || observed.context.content_revision
                    != format!(
                        "sha256:{}",
                        hex(&runtime.content_pin().server_artifact_digest())
                    )
            {
                return UseDisposition::StaleState;
            }
        }
        if callback.is_read_only()
            && !read_only_gate.is_some_and(|gate| {
                gate.actor == actor
                    && gate.session == session
                    && gate.callback == callback.key
                    && runtime.read_actor_position(actor) == Ok(gate.expected)
            })
        {
            return UseDisposition::Rejected;
        }
        let party = if let Some(party) = &callback.effects.party_transfer {
            if after.appearance_id != party.activation_appearance {
                return if map.facts.apply_quest_callback(
                    key,
                    expected_revision,
                    after,
                    &[],
                    &[],
                    close,
                    None,
                ) {
                    UseDisposition::Committed
                } else {
                    UseDisposition::StaleState
                };
            }
            match self.prepare_party_transfer(runtime, states, &closed, callback, session) {
                Ok(plan) => Some(plan),
                Err(refusal) => {
                    // Source toggles before validating the rite. Refusal changes
                    // only that appearance; no participant or offering changes.
                    return if map.facts.apply_quest_callback(
                        key,
                        expected_revision,
                        after,
                        &[],
                        &[],
                        close,
                        None,
                    ) {
                        refusal
                    } else {
                        UseDisposition::StaleState
                    };
                }
            }
        } else {
            None
        };
        let party_context = if let Some(plan) = &party {
            let Some(context) = self.party_context(runtime, plan).await else {
                return UseDisposition::Rejected;
            };
            context
        } else {
            Vec::new()
        };
        let durable = callback.requires_durable_commit()
            && !callback.is_read_only()
            && !(close && callback.opening_only);
        let mut fresh_effects = !durable;
        let mut success_message = if !durable {
            callback.success_message.clone()
        } else {
            None
        };
        if durable && callback.effects.encounter.is_some() {
            let preflight = self.execute_quest_encounter(
                runtime,
                states,
                actor,
                session,
                map,
                callback,
                key,
                expected_revision,
                after,
                true,
                cause,
                None,
            );
            if preflight != UseDisposition::Committed {
                return preflight;
            }
        }
        let mut success_preface = None;
        if callback.effects.encounter.is_some() && callback.has_personal_state() && !durable {
            return UseDisposition::Rejected;
        }
        let mut personal_reservation =
            if callback.effects.encounter.is_some() && callback.has_personal_state() {
                let (Some(fence), Some(definition)) = (
                    fence,
                    self.quest_catalogue
                        .as_ref()
                        .and_then(|catalogue| catalogue.definition_hash(&callback.quest)),
                ) else {
                    return UseDisposition::Rejected;
                };
                match self.states.quest_callbacks.personal.reserve(
                    runtime,
                    &callback.key,
                    personal_encounter::Origin::new(
                        *fence.character_id.as_bytes(),
                        cause,
                        definition,
                        &callback.quest,
                        fence.expected_character_revision.get(),
                    ),
                ) {
                    Ok(reservation) => Some(reservation),
                    Err(disposition) => return disposition,
                }
            } else {
                None
            };
        if callback.effects.encounter.is_some() && callback.has_personal_state() {
            let ready = self.preflight_personal_quest_encounter(
                runtime,
                states,
                actor,
                session,
                map,
                callback,
                key,
                expected_revision,
                after,
                cause,
            );
            if ready != UseDisposition::Committed {
                return ready;
            }
        }
        if durable {
            let (Some(fence), Some(slot), Some(catalogue)) =
                (fence, slot, self.quest_catalogue.as_ref())
            else {
                return UseDisposition::Rejected;
            };
            let context = QuestWorldContext {
                map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                content_revision: format!(
                    "sha256:{}",
                    hex(&runtime.content_pin().server_artifact_digest())
                ),
                placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                reset_epoch: self.reset_epoch,
                actor_placement: actor.placement_identity(),
                movement_revision: expected.revision(),
                party: party_context,
            };
            let context = if let Ok(mut pending) = self.states.quest_callbacks.records.lock() {
                let record = pending.entry(session).or_insert(Pending {
                    actor,
                    session,
                    expected,
                    callback: callback.key.clone(),
                    key,
                    revision: expected_revision,
                    cause,
                    origin: None,
                    context: None,
                    flame: None,
                    ingress_settled: false,
                    participants: party.as_ref().map_or_else(Vec::new, |plan| plan.sessions()),
                    flame_effect_sent: false,
                    projection_target: None,
                });
                record.projection_target = projection_target;
                record.context.get_or_insert(context).clone()
            } else {
                return UseDisposition::Rejected;
            };
            let context = if let Some((source, intent)) = selected_tool {
                let Some(context) = context.with_selected_tool(*source, intent.clone()) else {
                    return UseDisposition::Rejected;
                };
                context
            } else {
                context.into()
            };
            let reward_weights = self
                .active_generation
                .and_then(|g| g.native_gameplay())
                .map(|native| {
                    crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
                        .with_wheel(self.premium_coordinator)
                });
            let condition_proof = if callback.effects.requires_item_disguise.is_some() {
                let Some(native) = self.active_generation.and_then(|g| g.native_gameplay()) else {
                    return UseDisposition::Rejected;
                };
                let Some(current) = states.get(runtime, actor, session) else {
                    return UseDisposition::Rejected;
                };
                let Some(proof) = crate::durability::reward_claim_mint::QuestActorConditionProof::read_world_current(
                    runtime, actor, fence, cause, &callback.key, native, current.owned_conditions(), self.owner_now().get(),
                ) else { return UseDisposition::Rejected; };
                Some(proof)
            } else {
                None
            };
            let reward_admission = if callback.item_reward().is_some()
                || callback.effects.requires_item_disguise.is_some()
            {
                self.active_generation
                    .and_then(|g| g.native_gameplay())
                    .zip(reward_weights.as_ref())
                    .map(|(native, weights)| {
                        crate::durability::reward_claim_mint::QuestRewardAdmission {
                            actor_conditions: condition_proof.as_ref(),
                            carried_use: None,
                            creature_use: None,
                            peer_use: None,

                            native,
                            weights,
                            fee_payment: None,
                        }
                    })
            } else {
                None
            };
            let flame =
                if oteryn_gameplay_quests::quest::rotten_blood::flames::by_key(&callback.key)
                    .is_some()
                {
                    let Ok(mut records) = self.states.quest_callbacks.records.lock() else {
                        return UseDisposition::Rejected;
                    };
                    let Some(record) = records.get_mut(&session) else {
                        return UseDisposition::Rejected;
                    };
                    if record.flame.is_none() {
                        record.flame = super::rotten_flame::qualify(
                            self,
                            runtime,
                            actor,
                            cause,
                            (*context).clone(),
                            map,
                            key,
                            expected_revision,
                        );
                    }
                    let Some(input) = record.flame.clone() else {
                        return UseDisposition::Rejected;
                    };
                    Some(input)
                } else {
                    None
                };
            let outcome = if let Some(input) = &flame {
                slot.commit_rotten_flame_use(
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    input.clone(),
                    std::sync::Arc::clone(catalogue),
                    permit,
                )
                .await
            } else {
                slot.commit_world_callback_with_items_selected(
                    self.root,
                    self.character,
                    self.holder,
                    fence,
                    QuestTransitionRequest {
                        transition_key: callback.key.clone(),
                        cause,
                    },
                    std::sync::Arc::clone(catalogue),
                    permit,
                    context,
                    self.active_generation
                        .and_then(|generation| generation.native_gameplay()),
                    reward_admission,
                    &self.premium_coordinator,
                )
                .await
            };
            if callback.key == oteryn_gameplay_quests::quest::dreamer_carrot::KEY
                && matches!(&outcome,Ok(QuestTransitionOutcome::Refused(
                    oteryn_gameplay_quests::quest::QuestRefusal::SourceOfferingMissing
                    | oteryn_gameplay_quests::quest::QuestRefusal::SourceOfferingPremiumRequired)))
            {
                if matches!(
                    &outcome,
                    Ok(QuestTransitionOutcome::Refused(
                        oteryn_gameplay_quests::quest::QuestRefusal::SourceOfferingPremiumRequired
                    ))
                ) {
                    self.states.chat.publish_quest_speech(
                        runtime,
                        actor,
                        session,
                        oteryn_gameplay_quests::quest::dreamer_carrot::PREMIUM_SPEECH,
                    );
                }
                if let Ok(mut refusal) = self.states.quest_callbacks.carrot_refusal.lock() {
                    *refusal = Some((actor, session, expected));
                }
            }
            let fresh = matches!(&outcome, Ok(QuestTransitionOutcome::Committed(_)));
            if callback.item_reward().is_some()
                && matches!(
                    &outcome,
                    Ok(QuestTransitionOutcome::Committed(_)
                        | QuestTransitionOutcome::AlreadyCommitted(_))
                )
            {
                self.states.loop_ground.invalidate();
            }

            #[cfg(test)]
            if !matches!(
                &outcome,
                Ok(QuestTransitionOutcome::Committed(_)
                    | QuestTransitionOutcome::AlreadyCommitted(_))
            ) {
                eprintln!(
                    "QUEST_CALLBACK commit refused key={} outcome={outcome:?}",
                    callback.key
                );
            }
            match outcome {
                Ok(
                    QuestTransitionOutcome::Committed(ref committed)
                    | QuestTransitionOutcome::AlreadyCommitted(ref committed),
                ) => {
                    fresh_effects = fresh;
                    if fresh {
                        success_message = callback.committed_message(&committed.changes).map(str::to_owned);
                        // Crystal action_mead.lua:8-11 says both lines on the winning sip.
                        if callback.native_rule == Some(oteryn_gameplay_quests::quest::native_rules::NativeRule::BarbarianMead)
                            && committed.changes.iter().any(|change|
                                change.track == oteryn_gameplay_quests::quest::barbarian_mead::TRACKS[0]
                                    && change.before == 2 && change.after == 3)
                        {
                            success_preface = Some("The world seems to spin but you manage to stay on your feet.");
                        }
                    }
                    #[cfg(test)]
                    if self.states.quest_callbacks.interrupt_after_commit.swap(false, std::sync::atomic::Ordering::AcqRel) {
                        return UseDisposition::Rejected;
                    }
                    if fresh
                        && callback.native_rule == Some(oteryn_gameplay_quests::quest::native_rules::NativeRule::BarbarianMead)
                        && let QuestCause::Use(command) = cause {
                        self.project_mead_blackout(runtime,states,actor,command,&committed.changes,map);
                    }
                    self.apply_committed_quest_copy(session, fresh, committed);
                    if callback.effects.encounter.is_some()
                        && !self.refresh_grizzly_kill_snapshot_in_turn(runtime, session)
                    {
                        self.schedule_quest_refresh(session);
                        return UseDisposition::Rejected;
                    }
                    if callback.effects.encounter.is_none() {
                        self.refresh_grizzly_kill_snapshot_in_runtime(runtime, session).await;
                    }
                    self.refresh_native_objective_snapshot_in_runtime(runtime, session).await;
                    self.states.loop_ground.invalidate();
                    self.schedule_quest_refresh(session);
                }
                Err(ref error) if matches!(error, crate::durability::character_progression::CharacterProgressionError::Unavailable(_)) => {
                    #[cfg(test)]
                    eprintln!("quest callback {} unavailable: {error:?}", callback.key);
                    // A storage error can hide a committed receipt; reconcile its original cause.
                    return UseDisposition::Rejected;
                }
                Ok(QuestTransitionOutcome::Refused(oteryn_gameplay_quests::quest::QuestRefusal::StageMismatch))
                    if flame.as_ref().and_then(|input|input.no_change()).is_some() => {
                    if flame.as_ref().and_then(|input|input.no_change())==Some(crate::durability::quest_state::rotten_flame::FlameNoChange::AlreadyActive){
                        self.states.chat.push_quest_feedback(session,"This flame has already been activated.");
                    }
                    if let Ok(mut pending)=self.states.quest_callbacks.records.lock(){pending.remove(&session);}
                    return UseDisposition::Committed;
                }
                Ok(_) | Err(_) => {
                    if oteryn_gameplay_quests::quest::death_objectives::load().is_ok_and(|objectives|
                        objectives.iter().any(|objective| objective.quest == callback.quest && objective.guardian.is_some()))
                    {
                        self.schedule_quest_refresh(session);
                    }
                    if let Ok(mut pending) = self.states.quest_callbacks.records.lock() {
                        pending.remove(&session);
                    }
                    // Returning STEP callbacks emit their refusal after restoring
                    // the origin in refuse_quest_step. Only one owner emits it.
                    if (callback.trigger != Trigger::StepIn || !callback.return_on_failure)
                        && let Some(message) = &callback.failure_message
                    {
                        self.states.chat.push_quest_feedback(session, message);
                    }
                    if party.is_some() {
                        // The source lever still flips on a missing sacrifice.
                        map.facts.apply_quest_callback(key, expected_revision, after, &[], &[], close, None);
                    }
                    return UseDisposition::Rejected;
                }
            }
        }
        if let Some(plan) = party {
            return self.commit_party_transfer(
                runtime,
                states,
                map,
                plan,
                key,
                expected_revision,
                after,
                close,
                session,
                optional_pvp,
            );
        }
        if callback.effects.encounter.is_some() && !close {
            let disposition = self.execute_quest_encounter(
                runtime,
                states,
                actor,
                session,
                map,
                callback,
                key,
                expected_revision,
                after,
                false,
                cause,
                personal_reservation.as_mut(),
            );
            if disposition == UseDisposition::Committed {
                if let Ok(mut pending) = self.states.quest_callbacks.records.lock() {
                    pending.remove(&session);
                }
                if callback.has_personal_state()
                    && let Some(message) = success_message
                {
                    self.states.chat.push_quest_feedback(session, &message);
                }
            }
            return disposition;
        }
        let flame = self
            .states
            .quest_callbacks
            .records
            .lock()
            .ok()
            .and_then(|records| records.get(&session).and_then(|p| p.flame.clone()));
        if let Some(input) = flame {
            if !super::rotten_flame::publish(runtime, map, &input) {
                return UseDisposition::Rejected;
            }
            let Ok(mut records) = self.states.quest_callbacks.records.lock() else {
                return UseDisposition::Rejected;
            };
            let Some(record) = records.get_mut(&session) else {
                return UseDisposition::Rejected;
            };
            if !record.flame_effect_sent {
                let Some(at) = native(expected.position()) else {
                    return UseDisposition::Rejected;
                };
                if !map.facts.emit_movement_effect(at, 13) {
                    return UseDisposition::Rejected;
                }
                record.flame_effect_sent = true;
            }
        } else {
            // Durable admission may wait on PostgreSQL. Start source reset/regrowth
            // at publication, so commit latency cannot consume its visible lifetime.
            // Readset projections carry the same timer for the Children lever owner.
            if !close && let Some(delay) = callback.effects.reset_after_ms {
                let Some(published_due) =
                    self.owner_now().get().checked_add(u64::from(delay) * 1_000)
                else {
                    return UseDisposition::Exhausted;
                };
                match projection.as_mut() {
                    Some(crate::map::facts::QuestCallbackProjection::TimedTransform {
                        due,
                        ..
                    })
                    | Some(crate::map::facts::QuestCallbackProjection::ReadsetTransforms {
                        reset: Some((due, _)),
                        ..
                    }) => *due = published_due,
                    _ => {}
                }
            }
            if let Some(target) = &callback.effects.target_transform
                && let Some(crate::map::facts::QuestCallbackProjection::TimedTargetTransform {
                    due,
                    ..
                }) = projection.as_mut()
            {
                let Some(published_due) = self
                    .owner_now()
                    .get()
                    .checked_add(u64::from(target.reset_after_ms) * 1_000)
                else {
                    return UseDisposition::Exhausted;
                };
                *due = published_due;
            }
            // Crystal actions_lions_rock.lua:69: start the shared reset after
            // durable success. Commit latency must not shorten its visible lifetime.
            if let Some(crate::map::facts::QuestCallbackProjection::TimedPassages {
                due,
                source_reset,
                ..
            }) = &mut projection
            {
                let Some(delay) = callback.effects.passages_reset_after_ms else {
                    return UseDisposition::Rejected;
                };
                let published = self.owner_now().get();
                let Some(deadline) = published.checked_add(u64::from(delay) * 1_000) else {
                    return UseDisposition::Exhausted;
                };
                *due = deadline;
                if let Some((source_due, _)) = source_reset {
                    let Some(delay) = callback.effects.reset_after_ms else {
                        return UseDisposition::Rejected;
                    };
                    let Some(deadline) = published.checked_add(u64::from(delay) * 1_000) else {
                        return UseDisposition::Exhausted;
                    };
                    *source_due = deadline;
                }
            }
            if callback.key == oteryn_gameplay_quests::quest::dreamer_fire_bug::IGNITE
                && let Some(crate::map::facts::QuestCallbackProjection::TimedEntrance {
                    due, ..
                }) = &mut projection
            {
                let Some(deadline) = self.owner_now().get().checked_add(300_000_000) else {
                    return UseDisposition::Exhausted;
                };
                *due = deadline;
            }
            // A per-character gift does not consume or revise its shared source object.
            // Real scene changes and shared-world progress still publish their original CAS,
            // including the revision consumed by the post-publication removal below.
            let personal_gift_preserves_object = callback.item_reward().is_some()
                && callback.effects.item_reward_cycle.is_none()
                && !callback.effects.item_reward_per_use
                && callback.world_change.is_none()
                && oteryn_gameplay_quests::quest::shared_world_progress::binding(&callback.key)
                    .is_none()
                && callback.lever.is_none()
                && callback.teleport.is_none()
                && callback.remote_teleport.is_none()
                && !callback.reversible
                && !close
                && before == after
                && passages.is_empty()
                && replacements.is_empty()
                && projection.is_none()
                && scene_moves.is_none();
            let apply_scene = || {
                map.facts.apply_quest_callback(
                    key,
                    expected_revision,
                    after,
                    &passages,
                    &replacements,
                    close,
                    projection.as_ref(),
                )
            };
            if let Some(proof) = scene_moves {
                let committed = match runtime.commit_source_scene_relocations(proof, apply_scene) {
                    Ok(committed) => committed,
                    Err(error) => {
                        #[cfg(test)]
                        eprintln!("QUEST_SCENE_REFUSED commit close={close} error={error:?}");
                        let _ = error;
                        return UseDisposition::Rejected;
                    }
                };
                let now = self.owner_now().get();
                for ((moved_actor, moved_session, before), after) in
                    scene_observations.into_iter().zip(committed)
                {
                    if let Some(moved_session) = moved_session {
                        states.stage_special_movement_step(
                            runtime,
                            map,
                            moved_actor,
                            moved_session,
                            now,
                            Some(before.position()),
                            after.position(),
                            optional_pvp,
                        );
                        states.stage_underwater_step(
                            runtime,
                            moved_actor,
                            moved_session,
                            now,
                            underwater(map, before.position()),
                            underwater(map, after.position()),
                        );
                    }
                }
            } else if !callback.is_read_only() && !personal_gift_preserves_object && !apply_scene()
            {
                return UseDisposition::Rejected;
            }
        }
        if matches!(
            projection,
            Some(
                crate::map::facts::QuestCallbackProjection::TimedTransform { .. }
                    | crate::map::facts::QuestCallbackProjection::TimedRemoteTransform { .. }
                    | crate::map::facts::QuestCallbackProjection::TimedEntrance { .. }
            )
        ) || callback.effects.reset_after_ms.is_some()
            || callback.effects.passages_reset_after_ms.is_some()
            || callback.effects.target_transform.is_some()
            || callback
                .effects
                .portals
                .as_ref()
                .is_some_and(|pair| pair.lifetime_us.is_some())
        {
            states.owner_wake.notify_one();
        }
        if oteryn_gameplay_quests::quest::shared_world_progress::binding(&callback.key).is_some_and(
            |binding| {
                !matches!(
                    binding.action,
                    oteryn_gameplay_quests::quest::shared_world_progress::SharedAction::HiveTask(_)
                )
            },
        ) && !map
            .facts
            .remove_world_change_callback_item(key, expected_revision + 1)
        {
            return UseDisposition::Rejected;
        }
        if oteryn_gameplay_quests::quest::children_puzzle::binding(&callback.key)
            .is_some_and(|(_, solved)| solved)
        {
            let _ = map
                .facts
                .emit_movement_speech(crate::map::facts::MovementSpeech {
                    speaker: actor,
                    position: native(expected.position()).unwrap_or(pos),
                    talk_type: 36,
                    text: oteryn_gameplay_quests::quest::children_puzzle::SPEECH.into(),
                    captured_name: None,
                });
        }
        if callback.effects.restore_disguise {
            let Some(current) = states.get(runtime, actor, session) else {
                return UseDisposition::Rejected;
            };
            let mut next = current.clone();
            let mut conditions = current.owned_conditions().clone();
            if conditions.remove_type(crate::domain::condition::ConditionType::Outfit) {
                if next
                    .apply_batch_conditions(current.owned_conditions(), &conditions)
                    .is_err()
                    || next.advance_batch_revision().is_err()
                    || !states.commit(runtime, actor, session, next)
                {
                    return UseDisposition::Rejected;
                }
                states.owner_wake.notify_one();
            }
        }
        if let Some(proof) = teleport {
            let Ok(snapshot) = runtime.commit_source_step(proof) else {
                return UseDisposition::Rejected;
            };
            // Crystal 201189f data-global/scripts/actions/other/dryad_garden.lua:18-19:
            // publish at the committed destination, never at the source or on refusal.
            if matches!(
                callback.key.as_str(),
                "oteryn:quest-callback/dryad-garden-enter"
                    | "oteryn:quest-callback/dryad-garden-leave"
            ) {
                states.sounds.record_item_effect(
                    runtime,
                    actor,
                    snapshot.position(),
                    oteryn_gameplay_items::event_planting::DRYAD_GARDEN_EFFECT_ID,
                    false,
                );
            }
            states.stage_special_movement_step(
                runtime,
                map,
                actor,
                session,
                self.owner_now().get(),
                Some(expected.position()),
                snapshot.position(),
                optional_pvp,
            );
            states.stage_underwater_step(
                runtime,
                actor,
                session,
                self.owner_now().get(),
                underwater(map, expected.position()),
                underwater(map, snapshot.position()),
            );
        }
        if let Some(magic) = &callback.effects.magic {
            map.facts
                .emit_movement_effect(tile(magic.position), magic.effect);
        }
        if let Some(effect) = callback.effects.teleport_effect
            && let Some(destination) = callback.teleport
        {
            map.facts.emit_movement_effect(tile(destination), effect);
        }
        if let Ok(mut pending) = self.states.quest_callbacks.records.lock() {
            pending.remove(&session);
        }
        if fresh_effects && callback.key == oteryn_gameplay_quests::quest::dreamer_fire_bug::IGNITE
        {
            for (x, y) in oteryn_gameplay_quests::quest::dreamer_fire_bug::IGNITION_POSITIONS {
                map.facts.emit_movement_effect(
                    crate::map::overlay::TilePos { x, y, floor: -9 },
                    oteryn_gameplay_quests::quest::dreamer_fire_bug::IGNITION_EFFECT,
                );
            }
        }
        for position in restored_trees {
            map.facts.emit_movement_effect(position, 13);
        }
        for effect in &callback.effects {
            // Presentation uses the existing bounded movement occurrence stream;
            // it cannot turn a committed movement into a failed command.
            map.facts
                .emit_movement_effect(tile(effect.position), effect.effect);
        }
        if let Some(message) = success_preface {
            self.states.chat.push_quest_feedback(session, message);
        }
        if fresh_effects && let Some(speech) = &callback.effects.player_speech {
            self.states
                .chat
                .publish_quest_speech(runtime, actor, session, speech);
        }
        if let Some(message) = success_message {
            self.states.chat.push_quest_feedback(session, &message);
        }
        UseDisposition::Committed
    }
}
