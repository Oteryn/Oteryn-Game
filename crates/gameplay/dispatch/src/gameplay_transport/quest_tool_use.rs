//! Selected quest tools retain a live target through the fenced value commit.
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_ref_admission::NativeItemWeights,
    item_view::ItemKey,
};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent};
use std::sync::atomic::{AtomicBool, Ordering};

// A possibly committed capture cannot leave the target available to another
// action. Cancellation and an unknown acknowledgement retire this owner turn.
struct CreatureTurn<'a> {
    runtime: tokio::sync::MutexGuard<'a, ChannelRuntimeV1>,
    uncertain: AtomicBool,
}
impl Drop for CreatureTurn<'_> {
    fn drop(&mut self) {
        if self.uncertain.load(Ordering::Relaxed) {
            self.runtime.retire_owner_cycle();
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Ordinary carried quest tools borrow the same fenced quest/value owner as
    /// NPC exchanges. The selected instance and ingress remain in its input proof.
    pub(super) async fn use_carried_quest_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        corpse: Option<[u8; 16]>,
    ) -> Result<Option<UseOutcome>, ()> {
        use crate::durability::quest_state::{
            QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
        };
        use crate::durability::reward_claim_mint::{QuestCarriedUse, QuestRewardAdmission};
        use oteryn_gameplay_quests::quest::reward_callbacks;
        let Some(item_fence) = command.item_fence else {
            return Ok(None);
        };
        if !matches!(
            item_fence.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        ) {
            return Ok(None);
        }
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let encoded =
            oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent).map_err(|_| ())?;
        let outcome = |disposition| UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        };
        if let Some(matches) = self
            .root
            .reconcile_quest_carried_use(
                self.character,
                item_fence.character_id,
                command_ref,
                encoded.clone(),
            )
            .await
            .map_err(|_| ())?
        {
            return Ok(Some(outcome(if matches {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            })));
        }
        let Some(ItemKey::Instance(source)) = source else {
            return Ok(None);
        };
        let Ok(inventory) = self
            .root
            .read_item_use_inventory(self.character, self.holder, item_fence)
            .await
        else {
            return Ok(None);
        };
        let Some(item) = inventory
            .backpack
            .iter()
            .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item))
            .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
            .find(|item| item.item_instance_id == source)
        else {
            return Ok(None);
        };
        let Some(catalogue) = self.quest_catalogue.as_deref() else {
            return Ok(None);
        };
        let callbacks = reward_callbacks::load_for(catalogue).map_err(|_| ())?;
        let mut bound = callbacks
            .iter()
            .filter(|c| c.carried_item.as_deref() == Some(&item.definition.production_key))
            // Selected two-input recipes retain their own M20 admission owner.
            .filter(|c| c.carried_target.is_none() && !c.fresh_corpse)
            .filter(|c| c.corpse_target.is_some() == corpse.is_some())
            .collect::<Vec<_>>();
        if let Some(corpse) = corpse
            && bound.iter().any(|callback| {
                callback
                    .corpse_target
                    .as_ref()
                    .is_some_and(|target| target.reusable_tool)
            })
        {
            let appearance = self
                .root
                .read_ground_action_source(self.character, item_fence, corpse)
                .await
                .map_err(|_| ())?
                .and_then(|ground| {
                    ground
                        .definition()
                        .production_key
                        .strip_prefix("oteryn:item.tibia.i")?
                        .parse::<u32>()
                        .ok()
                });
            bound.retain(|callback| {
                callback.corpse_target.as_ref().is_none_or(|target| {
                    !target.reusable_tool || appearance == Some(target.appearance)
                })
            });
        }
        if bound.is_empty() {
            return Ok(None);
        }
        if (corpse.is_none() && intent.use_with_map_item.is_some())
            || (corpse.is_some()
                && (!matches!(
                    intent.use_with_map_item,
                    Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(_))
                ) || intent.use_with.is_some()))
            || intent.use_with_position.is_some()
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(source))
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
        }
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        };
        let (callback, tracking_cycle) = {
            let sessions = self.states.quest_sessions.lock().map_err(|_| ())?;
            let tracks = sessions
                .get(&command.game_session_id)
                .and_then(|s| s.copy.as_ref())
                .ok_or(())?
                .tracks();
            let eligible =
                |storage: &[oteryn_gameplay_quests::quest::world_callbacks::Storage]| {
                    storage.iter().all(|s| {
                        tracks
                            .get(&s.track)
                            .copied()
                            .or_else(|| catalogue.track(&s.track).map(|t| t.initial))
                            .is_some_and(|v| (s.minimum..=s.maximum).contains(&v))
                    })
                };
            let mut selected = bound.into_iter().filter(|c| {
                (if c.corpse_target.is_some() {
                    corpse.is_some()
                } else {
                    (c.peer_requires.is_some()
                        || c.creature_target.is_some()
                        || c.self_target.is_some())
                        == intent.use_with.is_some()
                }) && eligible(&c.storage)
                    && c.requires.iter().all(|r| r.holds(catalogue, tracks))
                    && c.state_ops
                        .iter()
                        .all(|op| op.advisory_holds(catalogue, tracks))
                    && c.participants.iter().all(|p| {
                        eligible(&p.storage)
                            && p.state_ops
                                .iter()
                                .all(|op| op.advisory_holds(catalogue, tracks))
                    })
            });
            let Some(callback) = selected.next() else {
                // The same tool's fresh-corpse callback keeps its target owner.
                if corpse.is_some()
                    && callbacks.iter().any(|c| {
                        c.fresh_corpse
                            && c.carried_item.as_deref() == Some(&item.definition.production_key)
                    })
                {
                    return Ok(None);
                }
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            if selected.next().is_some() {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            // Crystal gnomilly.lua:119-121 writes a fresh cooldown on each
            // completed report without changing it during the next assignment.
            let tracking_cycle = callback
                .carried_tracker
                .as_ref()
                .and_then(|tracker| tracker.cycle_track.as_deref())
                .and_then(|key| {
                    tracks
                        .get(key)
                        .copied()
                        .or_else(|| catalogue.track(key).map(|track| track.initial))
                });
            (callback.clone(), tracking_cycle)
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        };
        // A queued duplicate may have probed before the first capture committed.
        // Reconcile again after serialization, while its former target can be absent.
        if callback.capture_creature
            && let Some(matches) = self
                .root
                .reconcile_quest_carried_use(
                    self.character,
                    item_fence.character_id,
                    command_ref,
                    encoded.clone(),
                )
                .await
                .map_err(|_| ())?
        {
            return Ok(Some(outcome(if matches {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            })));
        }
        let fence = self
            .current_quest_fence(command.game_session_id)
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        if fence.item_fence() != item_fence {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        let peer = if callback.peer_requires.is_some() {
            match self.quest_peer_use(actor, command, intent).await {
                Ok(peer) => peer,
                Err(()) => return Ok(Some(outcome(UseDisposition::Rejected))),
            }
        } else {
            None
        };
        // Revision serialization precedes the runtime turn, matching the other
        // physical action owners. Keep the live target current through COMMIT.
        let mut revision = self.revision_sequencer.acquire(fence.character_id).await;
        let mut creature_runtime = if callback.creature_target.is_some()
            || callback.self_target.is_some()
            || callback.server_save_hotspot.is_some()
            || callback.carried_tracker.is_some()
        {
            Some(CreatureTurn {
                runtime: self.runtime.lock().await,
                uncertain: AtomicBool::new(false),
            })
        } else {
            None
        };
        let creature = if callback.creature_target.is_some() || callback.self_target.is_some() {
            let runtime = &*creature_runtime.as_ref().ok_or(())?.runtime;
            let proof = if let Some(corpse) = corpse {
                crate::durability::reward_claim_mint::quest_creature::QuestCreatureUseProof::read_current_corpse(
                    runtime, actor, fence, command_ref, &callback, intent, corpse)
            } else {
                crate::durability::reward_claim_mint::quest_creature::QuestCreatureUseProof::read_current(
                    runtime, actor, fence, command_ref, &callback, intent)
            };
            let Some(proof) = proof else {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            Some(proof)
        } else {
            None
        };
        let native = self
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or(())?;
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        // Crystal 201189f8 spike_tasks/actions_thermometer.lua:13-31.
        // Keep the actor at the sampled location through the gated hand-in.
        let mut tracking_probe = None;
        let location =
            if callback.server_save_hotspot.is_some() || callback.carried_tracker.is_some() {
                let runtime = &creature_runtime.as_ref().ok_or(())?.runtime;
                let control = runtime
                    .player_control_facts(actor, command.game_session_id)
                    .map_err(|_| ())?;
                if control.control_loss.is_some() {
                    return Ok(Some(outcome(UseDisposition::Rejected)));
                }
                let position = runtime
                    .read_actor_position(actor)
                    .map_err(|_| ())?
                    .position();
                let position = [position.x, position.y, i32::from(position.floor)];
                let world = self.world_id.as_bytes();
                let feedback = if let Some(spec) = &callback.server_save_hotspot {
                    (!spec.contains(&callback.key, world, self.reset_epoch, position))
                        .then(|| spec.cold_reply.clone())
                } else if let Some(tracker) = &callback.carried_tracker {
                    let target_at = |probe| {
                        tracker.target_at(
                            &callback.key,
                            world,
                            fence.character_id.as_bytes(),
                            self.reset_epoch,
                            tracking_cycle,
                            probe,
                        )
                    };
                    let target = if let Some(search) = &tracker.ground_search {
                        let Some(map) = self.bundle_map() else {
                            return Ok(Some(outcome(UseDisposition::Rejected)));
                        };
                        let last = search.attempts - 1;
                        // Crystal 201189f position.lua:148-161. The pinned
                        // ground keeps a Character's chosen target stable.
                        let probe = (0..last)
                            .find(|probe| {
                                let [x, y, z] = target_at(*probe);
                                let pos = crate::map::overlay::TilePos {
                                    x: x as u16,
                                    y: y as u16,
                                    floor: -(z as i8),
                                };
                                map.overlay
                                    .base()
                                    .tile(pos.x, pos.y, pos.floor)
                                    .filter(|tile| tile.walkable())
                                    .and_then(|tile| tile.ground())
                                    .and_then(|(ordinal, id)| {
                                        map.facts.quest_callback_base_appearance(pos, ordinal, id)
                                    })
                                    .is_some_and(|appearance| search.accepts(appearance))
                            })
                            .unwrap_or(last);
                        tracking_probe = Some(probe);
                        target_at(probe)
                    } else {
                        target_at(0)
                    };
                    (!tracker.reached(position, target)).then(|| tracker.heading(position, target))
                } else {
                    None
                };
                if let Some(feedback) = feedback {
                    self.states
                        .chat
                        .push_quest_feedback(command.game_session_id, &feedback);
                    return Ok(Some(outcome(UseDisposition::Committed)));
                }
                Some((self.reset_epoch, position))
            } else {
                None
            };
        let use_ = QuestCarriedUse {
            recipe_target: None,
            location,
            tracking_cycle,
            tracking_probe,
            source,
            intent: encoded,
        };
        let capture_target = creature
            .as_ref()
            .filter(|_| callback.capture_creature)
            .and_then(|proof| proof.target_actor());
        let capture_key = callback.creature_target.clone();
        if callback.capture_creature
            && let Some(turn) = &creature_runtime
        {
            turn.uncertain.store(true, Ordering::Relaxed);
        }
        let result = revision
            .commit_quest_reward(
                self.root,
                self.character,
                self.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: callback.key.clone(),
                    cause: QuestCause::Command(command_ref),
                },
                std::sync::Arc::clone(catalogue),
                QuestRewardAdmission {
                    native,
                    weights: &weights,
                    fee_payment: None,
                    actor_conditions: None,
                    carried_use: Some(&use_),
                    creature_use: creature.as_ref(),
                    peer_use: peer.as_ref(),
                },
                &self.premium_coordinator,
            )
            .await
            .map_err(|_| ())?;
        if let Some(turn) = &mut creature_runtime {
            if let (QuestTransitionOutcome::Committed(_), Some(spec), Some((_, [x, y, floor]))) =
                (&result, &callback.server_save_hotspot, location)
            {
                if let (Some(map), Ok(x), Ok(y), Ok(floor)) = (
                    self.bundle_map(),
                    u16::try_from(x),
                    u16::try_from(y),
                    i8::try_from(-floor),
                ) {
                    map.facts.emit_movement_effect(
                        crate::map::overlay::TilePos { x, y, floor },
                        u16::from(spec.hot_effect),
                    );
                }
            }
            if matches!(result, QuestTransitionOutcome::Committed(_)) && location.is_some() {
                for line in &callback.reply {
                    self.states
                        .chat
                        .push_quest_feedback(command.game_session_id, line);
                }
            }
            if matches!(result, QuestTransitionOutcome::Committed(_))
                && let Some(speech) = callback
                    .corpse_target
                    .as_ref()
                    .and_then(|target| target.speech.as_deref())
            {
                self.states.chat.publish_quest_speech(
                    &turn.runtime,
                    actor,
                    command.game_session_id,
                    speech,
                );
            }
            if matches!(result, QuestTransitionOutcome::Committed(_))
                && let Some(target) = capture_target
            {
                turn.runtime
                    .remove_mount_taming_subject(target, capture_key.as_deref().ok_or(())?)
                    .map_err(|_| ())?;
            }
            turn.uncertain.store(false, Ordering::Relaxed);
        }
        drop(creature_runtime);
        drop(revision);
        drop(permit);
        let disposition = if matches!(
            result,
            QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)
        ) {
            self.refresh_quest_session(command.game_session_id, false)
                .await;
            UseDisposition::Committed
        } else {
            UseDisposition::Rejected
        };
        Ok(Some(outcome(disposition)))
    }
}
