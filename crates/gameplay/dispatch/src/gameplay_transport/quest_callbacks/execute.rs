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
    ) -> UseDisposition {
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
            return UseDisposition::StaleState;
        }
        let close = callback.trigger == Trigger::StepOut
            || callback
                .lever
                .is_some_and(|pair| after.appearance_id == pair[1]);
        if !close
            && (!world_requirements_match(map, callback)
                || !self.ground_requirements_match(runtime, map, callback))
        {
            return UseDisposition::Rejected;
        }
        if close && callback.lever.is_some() && !callback.reversible {
            return UseDisposition::NothingToUse;
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
            Err(result) => return result,
        };
        if !close && let Some(delay) = callback.effects.reset_after_ms {
            let Some(due) = self.owner_now().get().checked_add(u64::from(delay) * 1_000) else {
                return UseDisposition::Exhausted;
            };
            projection = Some(crate::map::facts::QuestCallbackProjection::TimedTransform {
                due,
                restore: before,
            });
        }
        projection = match super::conditional_cells::prepare(map, callback, projection) {
            Ok(projection) => projection,
            Err(result) => return result,
        };
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
        if let Some(crate::map::facts::QuestCallbackProjection::ItemCycle {
            position,
            create: Some(_),
            ..
        }) = &projection
        {
            let Some(at) = runtime
                .read_actor_position(actor)
                .ok()
                .and_then(|p| native(p.position()))
            else {
                return UseDisposition::Rejected;
            };
            if super::super::map_door::doorway_occupied(runtime, actor, at, *position) {
                return UseDisposition::Occupied;
            }
        }
        let mut passages = Vec::new();
        let mut replacements = Vec::new();
        for passage_definition in &callback.passages {
            let passage = passage_definition;
            let p = tile(passage.position);
            if close
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
                        && !map.facts.house_tile(p))
                    .then_some(k)
                })
                .collect::<Vec<_>>();
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
                replacements.push((*passage, facts, !close));
            } else {
                passages.push(*passage);
            }
        }
        let Ok(expected) = runtime.read_actor_position(actor) else {
            return UseDisposition::Rejected;
        };
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let blocked = super::super::pvp_live::pz_blocked(states, actor, session, self.owner_now());
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
                crate::map::floor::prepare_quest_callback_teleport_facing(
                    runtime,
                    self.movement_cells(),
                    actor,
                    session,
                    expected,
                    local(to),
                    blocked,
                    &closed,
                    facing.or(callback.effects.facing.map(super::effects::facing)),
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
                    ingress_settled: false,
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
            match callback.teleport {
                Some(to) => match crate::map::floor::prepare_quest_callback_teleport_facing(
                    runtime,
                    self.movement_cells(),
                    actor,
                    session,
                    expected,
                    local(to),
                    blocked,
                    &closed,
                    facing.or(callback.effects.facing.map(super::effects::facing)),
                ) {
                    Ok(proof) => Some(proof),
                    Err(_) => return UseDisposition::NotPossible,
                },
                None => None,
            }
        };
        if !map
            .facts
            .quest_callback_capacity(key, expected_revision, callback.passages.len())
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
                    ingress_settled: false,
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
        let durable = callback.has_personal_state() && !(close && callback.opening_only);
        let mut success_message = if !durable {
            callback.success_message.clone()
        } else {
            None
        };
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
                    ingress_settled: false,
                });
                record.context.get_or_insert(context).clone()
            } else {
                return UseDisposition::Rejected;
            };
            let reward_weights = self
                .active_generation
                .and_then(|g| g.native_gameplay())
                .map(|native| {
                    crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
                        .with_wheel(self.premium_coordinator)
                });
            let reward_admission = if callback.effects.item_reward.is_some() {
                self.active_generation
                    .and_then(|g| g.native_gameplay())
                    .zip(reward_weights.as_ref())
                    .map(|(native, weights)| {
                        crate::durability::reward_claim_mint::QuestRewardAdmission {
                            actor_conditions: None,
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
            let outcome = slot
                .commit_world_callback_with_items(
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
                .await;
            let fresh = matches!(&outcome, Ok(QuestTransitionOutcome::Committed(_)));
            match outcome {
                Ok(
                    QuestTransitionOutcome::Committed(ref committed)
                    | QuestTransitionOutcome::AlreadyCommitted(ref committed),
                ) => {
                    if fresh {
                        success_message = callback.committed_message(&committed.changes).map(str::to_owned);
                    }
                    #[cfg(test)]
                    if self.states.quest_callbacks.interrupt_after_commit.swap(false, std::sync::atomic::Ordering::AcqRel) {
                        return UseDisposition::Rejected;
                    }
                    self.states.loop_ground.invalidate();
                    // Shrine bonuses must see the committed flag before the next swing;
                    // the cadence still reloads the durable copy for recovery and XP.
                    if callback.quest == "oteryn:quest.the_way_of_the_monk_quest"
                        && let Ok(mut sessions) = self.states.quest_sessions.lock()
                        && let Some(entry) = sessions.get_mut(&session)
                    {
                        entry.copy = entry.copy.take().and_then(|mut copy| {
                            let next = entry.version.checked_add(1)?;
                            if !copy.apply(committed) { return None; }
                            entry.version = next;
                            Some(copy)
                        });
                    }
                    self.schedule_quest_refresh(session);
                }
                Err(crate::durability::character_progression::CharacterProgressionError::Unavailable(_)) => {
                    // A storage error can hide a committed receipt; reconcile its original cause.
                    return UseDisposition::Rejected;
                }
                Ok(_) | Err(_) => {
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
                    return UseDisposition::Rejected;
                }
            }
        }
        if callback.effects.encounter.is_some() {
            return self.execute_quest_encounter(
                runtime,
                states,
                actor,
                session,
                map,
                callback,
                key,
                expected_revision,
                after,
            );
        }
        if !map.facts.apply_quest_callback(
            key,
            expected_revision,
            after,
            &passages,
            &replacements,
            close,
            projection.as_ref(),
        ) {
            return UseDisposition::Rejected;
        }
        if matches!(
            projection,
            Some(crate::map::facts::QuestCallbackProjection::TimedTransform { .. })
        ) || callback.effects.reset_after_ms.is_some()
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
                    text: oteryn_gameplay_quests::quest::children_puzzle::SPEECH,
                });
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
        for effect in &callback.effects {
            // Presentation uses the existing bounded movement occurrence stream;
            // it cannot turn a committed movement into a failed command.
            map.facts
                .emit_movement_effect(tile(effect.position), effect.effect);
        }
        if let Some(message) = success_message {
            self.states.chat.push_quest_feedback(session, &message);
        }
        UseDisposition::Committed
    }
}
