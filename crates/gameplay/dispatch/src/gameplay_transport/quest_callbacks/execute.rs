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
        let Some(mut after) = map.facts.base_entry(pos, ordinal, id) else {
            return UseDisposition::Rejected;
        };
        if map.facts.object_revision(key) != expected_revision {
            return UseDisposition::StaleState;
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
        if !map
            .facts
            .quest_callback_capacity(key, expected_revision, callback.passages.len())
        {
            return UseDisposition::Exhausted;
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
        let mut success_message = if callback.storage.is_empty() {
            callback.success_message.clone()
        } else {
            None
        };
        if !callback.storage.is_empty() {
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
                    ingress_settled: false,
                    participants: party.as_ref().map_or_else(Vec::new, |plan| plan.sessions()),
                });
                record.context.get_or_insert(context).clone()
            } else {
                return UseDisposition::Rejected;
            };
            let outcome = slot
                .commit_world_callback(
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
                    // Every committed callback must be visible to the next lethal turn,
                    // including native objective activation. The cadence still reloads
                    // the durable copy for recovery and pending XP obligations.
                    if let Ok(mut sessions) = self.states.quest_sessions.lock()
                        && let Some(entry) = sessions.get_mut(&session)
                    {
                        entry.copy = entry.copy.take().and_then(|mut copy| {
                            // A reconciled old receipt must not roll a newer copy back.
                            if copy.character_revision().is_some_and(|revision|
                                revision.get() >= committed.committed_character_revision.get()) {
                                return Some(copy);
                            }
                            let next = entry.version.checked_add(1)?;
                            if !copy.apply(committed) { return None; }
                            entry.version = next;
                            Some(copy)
                        });
                    }
                    self.refresh_grizzly_kill_snapshot_in_runtime(runtime, session).await;
                    self.refresh_native_objective_snapshot_in_runtime(runtime, session).await;
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
        if callback
            .effects
            .portals
            .as_ref()
            .is_some_and(|pair| pair.lifetime_us.is_some())
        {
            states.owner_wake.notify_one();
        }
        if let Some(proof) = teleport {
            let Ok(snapshot) = runtime.commit_source_step(proof) else {
                return UseDisposition::Rejected;
            };
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
