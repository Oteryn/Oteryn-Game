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
        let temporary_restore = if let Some(effect) = &callback.effects.temporary_transform {
            if map.facts.house_tile(pos) {
                return UseDisposition::Rejected;
            }
            let Some(restore) = map
                .facts
                .quest_callback_replacement(key, effect.restore)
                .or_else(|| map.facts.movement_item_facts(effect.restore))
            else {
                return UseDisposition::Rejected;
            };
            if map.facts.movement_item_facts(effect.appearance).is_none()
                || expected_revision.checked_add(2).is_none()
            {
                return UseDisposition::Rejected;
            }
            Some(restore)
        } else {
            None
        };
        if map.facts.object_revision(key) != expected_revision {
            return UseDisposition::StaleState;
        }
        let close = callback.trigger == Trigger::StepOut
            || callback
                .lever
                .is_some_and(|pair| after.appearance_id == pair[1]);
        if close && callback.lever.is_some() && !callback.reversible {
            return UseDisposition::NothingToUse;
        }
        if let Some(pair) = callback.lever {
            after.appearance_id = pair[usize::from(!close)];
        }
        if let Some(appearance) = callback.effects.transform {
            let Some(replacement) = map.facts.quest_callback_replacement(key, appearance) else {
                return UseDisposition::Rejected;
            };
            after = replacement;
        }
        let projection = match self.prepare_quest_projection(map, callback, close, &mut after) {
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
                        || passage.replacement == Some(f.appearance_id))
                        && !map.facts.house_tile(p))
                    .then_some(k)
                })
                .collect::<Vec<_>>();
            let [passage] = candidates.as_slice() else {
                return UseDisposition::Rejected;
            };
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
        let teleport = match callback.teleport {
            Some(to) => match crate::map::floor::prepare_quest_callback_teleport_facing(
                runtime,
                self.movement_cells,
                actor,
                session,
                expected,
                local(to),
                blocked,
                &closed,
                callback.effects.facing.map(super::effects::facing),
            ) {
                Ok(proof) => Some(proof),
                Err(_) => return UseDisposition::NotPossible,
            },
            None => None,
        };
        if !map.facts.quest_callback_capacity(
            key,
            expected_revision,
            if temporary_restore.is_some() {
                2
            } else {
                callback.passages.len()
            },
        ) {
            return UseDisposition::Exhausted;
        }
        if callback.storage.is_empty()
            && world_callbacks::world_change_requirement(&callback.key).is_some()
        {
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
        let mut success_message = if callback.storage.is_empty() {
            callback.success_message.clone()
        } else {
            None
        };
        let mut projection_time = self.owner_now().get();
        if !callback.storage.is_empty() {
            let (Some(fence), Some(slot), Some(catalogue)) =
                (fence, slot, self.quest_catalogue.as_ref())
            else {
                return UseDisposition::Rejected;
            };
            let context = QuestWorldContext {
                projection_time_us: temporary_restore.map(|_| projection_time),
                carried: None,
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
            if let Some(frozen) = context.projection_time_us {
                projection_time = frozen;
            }
            let outcome = if callback.effects.item_reward.is_some() {
                let Some(native) = self
                    .active_generation
                    .and_then(|generation| generation.native_gameplay())
                else {
                    return UseDisposition::Rejected;
                };
                let weights =
                    crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
                        .with_wheel(self.premium_coordinator);
                slot.commit_world_reward(
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
                    crate::durability::reward_claim_mint::QuestRewardAdmission {
                        actor_conditions: None,
                        native,
                        weights: &weights,
                        fee_payment: None,
                        carried_use: None,
                    },
                )
                .await
            } else {
                slot.commit_world_callback(
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
                )
                .await
            };
            let fresh = matches!(&outcome, Ok(QuestTransitionOutcome::Committed(_)));
            if callback.effects.item_reward.is_some()
                && matches!(
                    &outcome,
                    Ok(QuestTransitionOutcome::Committed(_)
                        | QuestTransitionOutcome::AlreadyCommitted(_))
                )
            {
                self.states.loop_ground.invalidate();
            }
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
        // Dryad watersplash54 is source metadata only until a live Item
        // presentation consumer exists; the existing teleport owner still commits.
        // A pure per-character gift has no map projection. The transform owner
        // increments object revisions even for identical facts; retain the
        // source object's identity and revision for every other player.
        if let (Some(effect), Some(restore)) =
            (&callback.effects.temporary_transform, temporary_restore)
        {
            if !map.facts.start_quest_callback_decay(
                key,
                expected_revision,
                restore,
                effect.appearance,
                projection_time,
                effect.lifetime_us,
            ) {
                return UseDisposition::Rejected;
            }
            // A recovered committed occurrence keeps its original deadline.
            map.facts.expire_movement_items(self.owner_now().get());
            states.owner_wake.notify_waiters();
        }
        if callback.effects.item_reward.is_none()
            && callback.effects.temporary_transform.is_none()
            && !map.facts.apply_quest_callback(
                key,
                expected_revision,
                after,
                &passages,
                &replacements,
                close,
                projection.as_ref(),
            )
        {
            return UseDisposition::Rejected;
        }
        if callback
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
        if let Ok(mut pending) = self.states.quest_callbacks.records.lock() {
            pending.remove(&session);
        }
        if let Some(message) = success_message {
            self.states.chat.push_quest_feedback(session, &message);
        }
        UseDisposition::Committed
    }
}
