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
        read_only_gate: Option<&ReadOnlyGate>,
        carried_tool: Option<[u8; 16]>,
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
        if !callback.is_guarded_source_departure()
            && map.facts.object_revision(key) != expected_revision
        {
            return UseDisposition::StaleState;
        }
        // Model validation keeps this predicate stateless. The runtime owner remains
        // held, with no await between the current placement read-set and map/movement commit.
        if !conditions::satisfied(map, &callback.object_conditions) {
            return UseDisposition::NotPossible;
        }
        if callback.objects.is_some() {
            let outcome = self.apply_callback_objects(map, callback, key, expected_revision, after);
            if outcome == UseDisposition::Committed {
                if let Ok(mut pending) = self.states.quest_callbacks.records.lock() {
                    pending.remove(&session);
                }
                if let Some(message) = &callback.success_message {
                    self.states.chat.push_quest_feedback(session, message);
                }
            }
            return outcome;
        }
        let close = match callback.trigger {
            Trigger::Use => callback
                .lever
                .is_some_and(|pair| after.appearance_id == pair[1]),
            Trigger::StepIn => false,
            Trigger::StepOut => !callback.is_guarded_source_departure(),
        };
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
                && callback.eject_occupants.is_none()
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
        if !callback.is_guarded_source_departure()
            && !map
                .facts
                .quest_callback_capacity(key, expected_revision, callback.passages.len())
        {
            return UseDisposition::Exhausted;
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
        let mut success_message = if callback.storage.is_empty() || callback.is_read_only() {
            callback.success_message.clone()
        } else {
            None
        };
        if !callback.storage.is_empty() && !callback.is_read_only() {
            let (Some(fence), Some(slot), Some(catalogue)) =
                (fence, slot, self.quest_catalogue.as_ref())
            else {
                return UseDisposition::Rejected;
            };
            let context = QuestWorldContext {
                carried_tool,
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
        // Progress-only callbacks have no Channel projection. Retained tools must
        // preserve the ordinary door owner's current appearance and revision.
        if callback.is_guarded_source_departure() || callback.is_progress_only_tool() {
            if let Ok(mut pending) = self.states.quest_callbacks.records.lock() {
                pending.remove(&session);
            }
            if let Some(message) = success_message {
                self.states.chat.push_quest_feedback(session, &message);
            }
            return UseDisposition::Committed;
        }
        // Dryad watersplash54 is source metadata only until a live Item
        // presentation consumer exists; the existing teleport owner still commits.
        let project = || {
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
        if callback.eject_occupants.is_some() {
            let result = self.eject_quest_occupants(
                runtime,
                states,
                map,
                callback,
                key,
                optional_pvp,
                project,
            );
            if result != UseDisposition::Committed {
                return result;
            }
        } else if !project() {
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
