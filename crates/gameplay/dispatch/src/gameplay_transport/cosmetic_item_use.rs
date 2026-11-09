//! USE cosmetic items through the shared item-action transaction and existing Premium owner.
pub(super) mod capture_commit;
pub(super) mod quest_targets;

use super::ComposedFreshAdmission;
use super::connection::{UseCommand, UseOutcome};
use super::item_ref_admission::NativeItemWeights;
use super::item_view::ItemKey;
use super::spell_premium_coordinator::RegisteredSpellAccessOwner;
use crate::durability::consume_transform::{ConsumeTransformRequest, GroundActionContext};
use crate::durability::consume_transform_cosmetics::ItemCosmeticPremium;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::gameplay_transport::actor_spell::NoveltyEffectRequest;
use crate::gameplay_transport::spell_access_facts::CurrentSpellAccessOwner;
use oteryn_gameplay_items::novelty_items::{NoveltyEffects, NoveltyKind};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent};

impl ItemCosmeticPremium for RegisteredSpellAccessOwner {
    fn current(&self, account: [u8; 16]) -> bool {
        self.account_id() == Some(account) && self.premium_gate().premium_current()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_cosmetic_item_impl(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        // Named-House actions belong to the existing House item owner. In particular,
        // do not run a Channel-only receipt probe before that owner can see the command.
        if matches!(
            command.item_fence.map(|fence| fence.runtime_scope),
            Some(crate::foundation::RuntimeScopeRefV1::Instance { .. })
        ) {
            return Ok(None);
        }
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return Ok(None);
        };
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let encoded = oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent)
            .map_err(|_| ())?;
        let pending_source = {
            let states = self.states.spell_states.lock().await;
            states
                .pending_item_effects
                .iter()
                .find(|memo| {
                    memo.actor == actor
                        && memo.request.command == command_ref
                        && memo.request.intent == encoded
                        && memo.effects.kind == NoveltyKind::Cosmetic
                })
                .map(|memo| memo.request.source)
        };
        let source = pending_source
            .or(match source {
                Some(ItemKey::Instance(id)) => Some(id),
                _ => None,
            })
            .unwrap_or([0; 16]);
        let request = ConsumeTransformRequest {
            command: command_ref,
            source,
            target: None,
            intent: encoded,
        };
        let outcome = |disposition| UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        };
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let content = NativeItemWeights(native);
        let replay = self
            .root
            .consume_transform_detailed(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &content,
                true,
            )
            .await
            .map_err(|_| ())?;
        if let Some(receipt) = replay.as_ref()
            && receipt.effects.get("cosmetic").is_some()
        {
            return self
                .finish_cosmetic_item(actor, &request, replay.ok_or(())?)
                .await
                .map(Some);
        }
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        let item = inventory
            .backpack
            .iter()
            .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item))
            .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
            .find(|item| item.item_instance_id == source);
        let (definition, ground_source) = if let Some(item) = item {
            (item.definition.clone(), None)
        } else {
            let Some(ground) = self
                .root
                .read_ground_action_source(self.character, fence, source)
                .await
                .map_err(|_| ())?
            else {
                return Ok(None);
            };
            (ground.definition().clone(), Some(ground))
        };
        let Some(rule) = oteryn_gameplay_items::item_cosmetics::rule(&definition.production_key)
        else {
            return Ok(None);
        };
        // NX-5 owner scope excludes Crystal's OT-only random addon/mount actions.
        // Historical receipts above remain readable; no new source is spent.
        if matches!(
            rule.mode,
            oteryn_gameplay_items::item_cosmetics::Mode::RandomOutfit
                | oteryn_gameplay_items::item_cosmetics::Mode::RandomMount
        ) {
            return Ok(Some(UseOutcome {
                disposition: UseDisposition::NotPossible,
                ..UseOutcome::rejected()
            }));
        }

        if let Some(receipt) = replay {
            return Ok(Some(outcome(receipt.disposition)));
        }
        if rule.donor.starts_with("data-crystal/scripts/special/") {
            return Ok(Some(outcome(UseDisposition::NotPossible)));
        }
        if intent.use_with.is_some()
            || intent.use_with_map_item.is_some()
            || intent.use_with_position.is_some()
        {
            return Ok(Some(outcome(UseDisposition::NothingToUse)));
        }
        // Afflicted chooses its remaining inputs internally. The existing service
        // exclusion closes/refuses an active offer before resolving those pieces.
        let trade_allows = if rule.mode == oteryn_gameplay_items::item_cosmetics::Mode::Pieces {
            self.states
                .player_trades
                .physical_service(command.game_session_id)
        } else {
            self.states
                .player_trades
                .physical(command.game_session_id, Some(source))
        };
        if !trade_allows {
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
        let ground = if let Some(source) = ground_source {
            let Some(room) = self.rooms().gate_ground_item_action() else {
                return Ok(Some(outcome(UseDisposition::NotPossible)));
            };
            let runtime = self.runtime.lock().await;
            match GroundActionContext::from_native_owner(
                room,
                &runtime,
                actor,
                command.game_session_id,
                source,
            ) {
                Ok(context) => Some(context),
                Err(_) => return Ok(Some(outcome(UseDisposition::TargetIllegal))),
            }
        } else {
            None
        };
        // The registered session selects the existing account Premium owner. Cosmetic
        // acquisition does not depend on a spell build or promotion. The durable helper
        // independently matches this view to the fenced Character's account and rechecks
        // its trusted clock after waiting for transaction locks.
        let premium = match (
            self.premium_coordinator,
            self.session_account(command.game_session_id),
        ) {
            (Some(coordinator), Some(account)) => {
                let _ = coordinator.premium_gate(account).await;
                coordinator.view(account)
            }
            _ => std::sync::Arc::new(RegisteredSpellAccessOwner::unavailable()),
        };
        if native.cosmetics().is_some() {
            let choice = oteryn_gameplay_items::item_cosmetics::select_for_command(
                rule,
                *command.game_session_id.as_bytes(),
                command.command_id,
                source,
            )
            .ok_or(())?;
            let effects = NoveltyEffects {
                kind: NoveltyKind::Cosmetic,
                source_item_id: rule.item_id,
                donor: rule.donor.clone(),
                cues: choice.cues.clone(),
                speech: choice.speech.clone(),
                private_speech: choice.private_speech,
                require_head: false,
                require_player_target: false,
                dice_value: None,
                costume: None,
                damage: None,
                achievement: None,
                achievement_progress: Vec::new(),
                timed_output: None,
                ground: None,
            };
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if let Err(disposition) = states.prepare_novelty_effect(
                &runtime,
                native,
                NoveltyEffectRequest {
                    actor,
                    fence,
                    request: &request,
                    effects,
                    target: None,
                    now_us: self.owner_now().get(),
                },
            ) {
                return Ok(Some(outcome(disposition)));
            }
        }
        // Source cosmetic effects remain on the player even when the acquisition
        // item lies on Ground; only its custody proof changes.
        let receipt = if let Some(ground) = ground.as_ref() {
            self.root
                .consume_transform_on_ground_with_cosmetic_premium(
                    &permit,
                    self.character,
                    self.holder,
                    fence,
                    request.clone(),
                    native,
                    &content,
                    ground,
                    Some(premium.as_ref()),
                )
                .await
        } else {
            self.root
                .consume_transform_with_cosmetic_premium(
                    &permit,
                    self.character,
                    self.holder,
                    fence,
                    request.clone(),
                    native,
                    &content,
                    false,
                    Some(premium.as_ref()),
                )
                .await
        }
        .map_err(|_| ())?
        .ok_or(())?;
        self.finish_cosmetic_item(actor, &request, receipt)
            .await
            .map(Some)
    }

    pub(in crate::gameplay_transport) async fn finish_cosmetic_item(
        &self,
        actor: ExactActorRef,
        request: &ConsumeTransformRequest,
        receipt: crate::durability::consume_transform::ConsumeTransformResult,
    ) -> Result<UseOutcome, ()> {
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let original_pending = states.pending_item_effects.iter().any(|memo| {
            memo.actor == actor
                && memo.request.command == request.command
                && memo.request.source == request.source
                && memo.request.target == request.target
                && memo.request.intent == request.intent
        });
        if let Some((effects, position)) =
            states.settle_novelty_effect(&runtime, actor, request, &receipt)?
        {
            states.publish_novelty_visuals(&runtime, actor, position, &effects);
            self.states.chat.publish_item_speech(
                &runtime,
                actor,
                request.command.game_session_id(),
                &effects,
            );
        }
        // Already-owned outcomes suppress success speech, but retain the donor's
        // feedback. Only the original pending turn may deliver it, including an
        // unknown-ack retry; a historical receipt with no pending turn stays silent.
        if original_pending
            && receipt.disposition == UseDisposition::Committed
            && let Some(feedback) = receipt
                .effects
                .get("cosmetic")
                .and_then(|effect| effect.get("feedback"))
                .and_then(serde_json::Value::as_str)
        {
            self.states
                .chat
                .push_quest_feedback(request.command.game_session_id(), feedback);
        }
        Ok(UseOutcome {
            disposition: receipt.disposition,
            ..UseOutcome::rejected()
        })
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Ordinary carried quest tools borrow the same fenced quest/value owner as
    /// NPC exchanges. The selected instance and ingress remain in its input proof.
    pub(super) async fn use_carried_quest_item_with_target(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        self.use_carried_quest_item_targeted(actor, command, intent, source, target)
            .await
    }

    pub(super) async fn use_carried_quest_item_targeted(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
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
        // Recipe receipts remain replayable after both inventory handles retire.
        // Fresh map tools and unowned World targets belong to their existing owner.
        if matches!(
            intent.use_with_map_item.as_ref(),
            Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(_))
        ) && !matches!(target, Some(ItemKey::Instance(_)))
        {
            return Ok(None);
        }
        let source = match (&intent.target, source) {
            (
                oteryn_protocol_oteryn::world_object::UseTarget::Item(_),
                Some(ItemKey::Instance(item)),
            ) => Some(item),
            (oteryn_protocol_oteryn::world_object::UseTarget::ItemByDefinition(_), None) => None,
            _ => return Ok(None),
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
            .find(|item| {
                super::mount_taming::quest_objects::matches_source(
                    &intent.target,
                    source,
                    item.item_instance_id,
                    self.active_generation
                        .and_then(crate::content::ActiveGeneration::native_gameplay)
                        .and_then(|native| native.item_index())
                        .and_then(|index| {
                            index.definition_ref(
                                &item.definition.family,
                                &item.definition.production_key,
                                &item.definition.revision_ref,
                            )
                        }),
                )
            })
        else {
            return Ok(None);
        };
        let source = item.item_instance_id;
        if let Some(rules) =
            oteryn_gameplay_quests::quest::item_disguises::rules().filter(|rules| {
                rules
                    .iter()
                    .any(|rule| rule.item == item.definition.production_key)
            })
        {
            // This source is retained. No quest transition, value grant or receipt is
            // invented for an action that only changes the live appearance conflict.
            if intent.use_with.is_some()
                || intent.use_with_map_item.is_some()
                || intent.use_with_position.is_some()
                || !self
                    .states
                    .player_trades
                    .physical(command.game_session_id, Some(source))
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let Some(_permit) = self.spell_lane_permit().await else {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            let _slot = self
                .revision_sequencer
                .acquire(item_fence.character_id)
                .await;
            // DROP, MOVE, EQUIP and trade share this exclusive scope lane. The
            // preliminary route lookup predates it, so re-prove exact custody here.
            let retained = self
                .root
                .read_item_use_inventory(self.character, self.holder, item_fence)
                .await
                .map_err(|_| ())?;
            if !retained
                .backpack
                .iter()
                .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item))
                .chain(retained.nested.iter().map(|(_, entry)| &entry.item))
                .any(|current| {
                    current.item_instance_id == source && current.definition == item.definition
                })
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let copy = self
                .root
                .read_character_quest_state(self.character, item_fence.character_id)
                .await
                .map_err(|_| ())?;
            let Some(catalogue) = self.quest_catalogue.as_ref() else {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            let mut eligible = rules.iter().filter(|rule| {
                rule.item == item.definition.production_key
                    && copy
                        .tracks()
                        .get(&rule.track)
                        .copied()
                        .or_else(|| catalogue.track(&rule.track).map(|track| track.initial))
                        == Some(rule.required)
                    && rule
                        .requires
                        .iter()
                        .all(|required| required.holds(catalogue, copy.tracks()))
            });
            let rule = eligible.next();
            if eligible.next().is_some() {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            if self
                .current_quest_fence(command.game_session_id)
                .await
                .map_err(|_| ())?
                .is_none_or(|fence| fence.item_fence() != item_fence)
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let Some(native) = self
                .active_generation
                .and_then(|generation| generation.native_gameplay())
            else {
                return Err(());
            };
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            if !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
                || !runtime
                    .bound_attacker_lease(actor, command.game_session_id)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == item_fence.character_id.as_bytes()
                            && lease.generation() == item_fence.character_lease_generation
                    })
                || runtime.binding().runtime_scope() != item_fence.runtime_scope
                || runtime.binding().scope_generation() != item_fence.scope_ownership_generation
                || super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                    .is_none()
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || !self
                    .states
                    .player_trades
                    .physical(command.game_session_id, Some(source))
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            // Crystal returns true even outside the quest stage, with no effect.
            let Some(rule) = rule else {
                return Ok(Some(outcome(UseDisposition::Committed)));
            };
            let Some(map) = self.bundle_map() else {
                return Err(());
            };
            let at = runtime
                .read_actor_position(actor)
                .map_err(|_| ())?
                .position();
            let position = crate::map::overlay::TilePos {
                x: u16::try_from(at.x).map_err(|_| ())?,
                y: u16::try_from(at.y).map_err(|_| ())?,
                floor: i8::try_from(-at.floor).map_err(|_| ())?,
            };
            if map.overlay.runtime_scope() != runtime.binding().runtime_scope()
                || map
                    .overlay
                    .base()
                    .tile(position.x, position.y, position.floor)
                    .is_none()
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            if rule.activation.is_some_and(|required| {
                required.x != position.x
                    || required.y != position.y
                    || required.floor != position.floor
            }) {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let appearance = format!("oteryn:item.tibia.i{}", rule.appearance);
            if native
                .item_index()
                .and_then(|index| index.definition_ref("Item", &appearance, "definition-r1"))
                .is_none()
                || native
                    .spell_appearances()
                    .and_then(|appearances| appearances.for_item(&appearance, "definition-r1"))
                    .is_none()
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let Some(definition) =
                crate::foundation::ConditionDefinition::new_permanent_item_outfit(&rule.key, 1)
                    .and_then(|definition| {
                        definition.with_item_appearance(
                            &appearance,
                            "definition-r1",
                            native.source_digest(),
                        )
                    })
            else {
                return Err(());
            };
            let Some(current) = states.get(&runtime, actor, command.game_session_id) else {
                return Err(());
            };
            let mut next = current.clone();
            let mut conditions = current.owned_conditions().clone();
            let decision = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
                native.source_digest(),
            );
            let facts = crate::foundation::ApplicationFacts {
                now: self.owner_now().get(),
                base_speed: 1,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &decision,
                occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                    [0; 16],
                ),
            };
            let changed = match conditions.apply(
                &definition,
                None,
                crate::foundation::ConditionSourceKind::SelfUse,
                &[],
                &facts,
            ) {
                Ok(_) => true,
                Err(crate::domain::condition::ConditionRefusal::KeptCurrent) => false,
                Err(_) => return Ok(Some(outcome(UseDisposition::Rejected))),
            };
            if changed {
                next.apply_batch_conditions(current.owned_conditions(), &conditions)
                    .map_err(|_| ())?;
                next.advance_batch_revision().map_err(|_| ())?;
                if !states.commit(&runtime, actor, command.game_session_id, next) {
                    return Err(());
                }
            }
            map.facts
                .emit_movement_speech(crate::map::facts::MovementSpeech {
                    speaker: actor,
                    position,
                    talk_type: rule.speech_type,
                    text: rule.use_text.to_owned(),
                    captured_name: None,
                });
            map.facts.emit_movement_effect(position, rule.use_effect);
            states.owner_wake.notify_one();
            return Ok(Some(outcome(UseDisposition::Committed)));
        }
        let recipe_target = match target {
            Some(ItemKey::Instance(id))
                if id != source
                    && inventory
                        .backpack
                        .iter()
                        .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item))
                        .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
                        .any(|item| item.item_instance_id == id) =>
            {
                Some(id)
            }
            Some(ItemKey::Instance(_)) if intent.use_with_map_item.is_some() => None,
            Some(_) => return Ok(None),
            None => None,
        };
        let target_item = recipe_target.and_then(|selected| {
            inventory
                .backpack
                .iter()
                .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item))
                .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
                .find(|item| item.item_instance_id == selected)
        });
        if recipe_target.is_some() && target_item.is_none() {
            return Ok(None);
        }
        let Some(catalogue) = self.quest_catalogue.as_deref() else {
            return Ok(None);
        };
        let callbacks = reward_callbacks::load_for(catalogue).map_err(|_| ())?;
        let bound = callbacks
            .iter()
            .filter(|c| c.carried_item.as_deref() == Some(&item.definition.production_key))
            .filter(|c| {
                c.carried_target.as_deref()
                    == target_item.map(|item| item.definition.production_key.as_str())
            })
            .collect::<Vec<_>>();
        if bound.is_empty() {
            return Ok(None);
        }
        if (intent.use_with_map_item.is_some()
            && !(recipe_target.is_some()
                && matches!(
                    intent.use_with_map_item.as_ref(),
                    Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(_))
                )))
            || intent.use_with_position.is_some()
            || recipe_target.is_some_and(|id| {
                !self
                    .states
                    .player_trades
                    .physical(command.game_session_id, Some(id))
            })
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
        let callback = {
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
                quest_targets::accepts(c, intent)
                    && eligible(&c.storage)
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
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            if selected.next().is_some() {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            callback.clone()
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        };
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
        let corpse_operand = if callback.fresh_corpse {
            let Some(ItemKey::Instance(item)) = target else {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            };
            let Some(ground) = self
                .root
                .read_ground_action_source(self.character, item_fence, item)
                .await
                .map_err(|_| ())?
            else {
                return Ok(Some(outcome(UseDisposition::StaleState)));
            };
            Some(ground.quest_corpse_operand())
        } else {
            None
        };
        // Lane -> Character sequence -> runtime is retained through COMMIT.
        // The proof's borrow prevents detaching a live target snapshot.
        let mut sequence = self.revision_sequencer.acquire(fence.character_id).await;
        let mut creature_runtime =
            if callback.creature_target.is_some() || callback.self_target.is_some() {
                Some(self.runtime.lock().await)
            } else {
                None
            };
        let mut capture_guard = creature_runtime.as_mut().map(|runtime| {
            capture_commit::CaptureCommitGuard::new(runtime, callback.capture_creature)
        });
        let creature = if let Some(guard) = capture_guard.as_ref() {
            let runtime = guard.runtime();
            use crate::durability::reward_claim_mint::quest_creature::QuestCreatureUseProof;
            let proof = if callback.corpse_target.is_some() {
                let Some(ItemKey::Instance(corpse)) = target else {
                    return Ok(Some(outcome(UseDisposition::Rejected)));
                };
                QuestCreatureUseProof::read_current_corpse(
                    runtime,
                    actor,
                    fence,
                    command_ref,
                    &callback,
                    intent,
                    corpse,
                )
            } else if let Some(operand) = corpse_operand {
                QuestCreatureUseProof::read_corpse(
                    runtime,
                    actor,
                    fence,
                    command_ref,
                    &callback,
                    intent,
                    operand,
                    &permit,
                )
            } else {
                QuestCreatureUseProof::read_current(
                    runtime,
                    actor,
                    fence,
                    command_ref,
                    &callback,
                    intent,
                )
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
        let use_ = QuestCarriedUse {
            source,
            recipe_target,
            intent: encoded,
        };
        let capture_target = creature.as_ref().and_then(|proof| proof.live_target());
        let capture_key = callback.creature_target.clone();
        if let Some(guard) = capture_guard.as_ref() {
            guard.arm();
        }
        let result = sequence
            .commit_quest_reward(
                self.root,
                self.character,
                self.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: callback.key,
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
        drop(creature);
        if let Some(guard) = capture_guard.as_mut() {
            guard.finish(&result, capture_target, capture_key.as_deref())?;
        }
        drop(capture_guard);
        drop(creature_runtime);
        drop(sequence);
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
