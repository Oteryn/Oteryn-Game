//! Ordinary USE -> the shared consume/transform receipt -> existing condition and chat owners.
use super::ComposedFreshAdmission;
use super::connection::{UseCommand, UseOutcome};
use super::item_ref_admission::NativeItemWeights;
use super::item_view::ItemKey;
use crate::durability::consume_transform::{ConsumeTransformRequest, ConsumeTransformResult};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{CommandId, CommandRef, ExactActorRef, MovementLocalPosition};
use crate::gameplay_transport::actor_spell::NoveltyEffectRequest;
use oteryn_gameplay_items::novelty_items::{
    self, GHOSTSILVER_LANTERN_UNLIT, NoveltyEffects, NoveltyKind,
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};
#[path = "novelty_ground_use.rs"]
mod ground;
#[path = "novelty_item_pending.rs"]
mod pending;

#[derive(Debug)]
pub(super) struct PendingItemEffect {
    pub actor: ExactActorRef,
    pub fence: CurrentCharacterItemFence,
    pub request: ConsumeTransformRequest,
    pub effects: NoveltyEffects,
    pub cue_position: crate::foundation::MovementLocalPosition,
    pub before_revision: u64,
    pub after: Option<crate::spell::cast::PlayerSpellState>,
    pub death: Option<super::actor_spell::PlayerDeath>,
}

fn is_watch(key: &str) -> bool {
    matches!(key, "oteryn:item.tibia.i2906" | "oteryn:item.tibia.i6091")
}

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Outer None delegates a non-action item. Err keeps an ambiguous owner turn unsequenced.
    pub(super) async fn use_event_item_impl(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
        visible_target: bool,
    ) -> Result<Option<UseOutcome>, ()> {
        if intent.use_with.is_none()
            && intent.use_with_map_item.is_none()
            && intent.use_with_position.is_none()
            && let Some(outcome) = self
                .use_shared_callback_handle(actor, command, source, None)
                .await
        {
            return Ok(Some(outcome));
        }
        if let Some(outcome) = self
            .use_map_harvest_handle(actor, command, &intent, source)
            .await?
        {
            return Ok(Some(outcome));
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
        let definition = if let UseTarget::ItemByDefinition(wanted) = intent.target {
            if intent.use_with.is_some()
                || intent.use_with_map_item.is_some()
                || intent.use_with_position.is_some()
            {
                return Ok(None);
            }
            let runtime = self.runtime.lock().await;
            let Some((key, revision)) =
                super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                    .and_then(|index| index.definition_at(wanted))
            else {
                return Ok(None);
            };
            if !is_watch(key)
                && novelty_items::rule_for_key(key)
                    .is_none_or(|rule| rule.kind != NoveltyKind::CarriedTransform)
            {
                return Ok(None);
            }
            Some(TypedDefinitionRef {
                family: "Item".into(),
                production_key: key.into(),
                revision_ref: revision.into(),
            })
        } else {
            None
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
                })
                .map(|memo| memo.request.source)
        };
        let id = pending_source
            .or(match source {
                Some(ItemKey::Instance(id)) => Some(id),
                _ => None,
            })
            .unwrap_or([0; 16]);
        let mut request = ConsumeTransformRequest {
            command: command_ref,
            source: id,
            target: None,
            intent: encoded,
        };
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let content = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        // The original bounded USE bytes identify history even when both item and view handle
        // have retired. No current inventory or target read may precede this probe.
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
        if let Some(receipt) = replay.as_ref() {
            if receipt.disposition == UseDisposition::Committed {
                if serde_json::from_value::<NoveltyEffects>(receipt.effects.clone()).is_ok() {
                    return self
                        .finish_novelty_item(actor, &request, replay.ok_or(())?)
                        .await
                        .map(Some);
                }
                return Ok(Some(outcome(receipt.disposition)));
            }
            // The original lottery route has its historical instance-tagged intent.
            // It must classify that receipt itself; all raw USE conflicts stay rejected.
            if receipt.recipe.as_deref() != Some("oteryn:lottery-ticket.v1") {
                return Ok(Some(outcome(receipt.disposition)));
            }
            return Ok(None);
        }
        if definition.is_none()
            && let Some(outcome) = self
                .use_ground_event_item(actor, command, &intent, &request, &permit)
                .await?
        {
            return Ok(Some(outcome));
        }
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        if let Some(definition) = definition {
            let states = self.states.spell_states.lock().await;
            let reserved = states
                .item_uses
                .reserved_sources(actor, command.game_session_id);
            request.source = match inventory.first_unreserved(&definition, reserved) {
                Ok(id) => id,
                Err(oteryn_gameplay_items::hotkey_search::HotkeyRefusal::BoundExceeded) => {
                    return Ok(Some(outcome(UseDisposition::Rejected)));
                }
                Err(_) => return Ok(Some(outcome(UseDisposition::NothingToUse))),
            };
        }
        let id = request.source;
        let carried = inventory
            .backpack
            .iter()
            .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item))
            .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
            .chain(inventory.equipment.iter())
            .find(|item| item.item_instance_id == id);
        let Some(item) = carried else {
            return Ok(None);
        };
        // Crystal 201189f8 data/scripts/actions/tools/watch.lua:3-8 and
        // data/libs/functions/functions.lua:42-50. Watches read the same World
        // clock as the connection's CLOCK-1 projection; they never consume an item.
        if is_watch(&item.definition.production_key) {
            if intent.use_with.is_some()
                || intent.use_with_map_item.is_some()
                || intent.use_with_position.is_some()
            {
                return Ok(None);
            }
            let runtime = self.runtime.lock().await;
            if !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
                || !runtime
                    .bound_attacker_lease(actor, command.game_session_id)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
                || runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let now =
                super::connection::FreshAdmissionAuthority::world_clock_utc_ms(self).ok_or(())?;
            let minute = crate::world_clock::WorldClockSource::new(
                crate::world_clock::DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS,
            )
            .tibian_minute(now)
            .ok_or(())?;
            self.states.chat.push_quest_feedback(
                command.game_session_id,
                &format!("The time is {}:{:02}.", minute / 60, minute % 60),
            );
            return Ok(Some(outcome(UseDisposition::Committed)));
        }
        // Reward containers retain the existing craft receipt owner and its route.
        if !novelty_items::is_novelty(&item.definition.production_key) {
            return Ok(None);
        }
        let plan = novelty_items::plan(
            native,
            &item.definition,
            *command.game_session_id.as_bytes(),
            command.command_id,
            id,
        );
        let Some(plan) = plan else {
            return Ok(Some(outcome(UseDisposition::NotPossible)));
        };
        let effects: NoveltyEffects = serde_json::from_value(plan.effects).map_err(|_| ())?;
        if effects.kind == NoveltyKind::CarriedTransform
            && (intent.use_with.is_some()
                || intent.use_with_map_item.is_some()
                || intent.use_with_position.is_some())
        {
            return Ok(None);
        }
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(id))
        {
            return Ok(Some(outcome(UseDisposition::Rejected)));
        }
        {
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let target = if effects.require_player_target {
                let Some(wanted) = intent.use_with else {
                    return Ok(Some(outcome(UseDisposition::NoTarget)));
                };
                let visible = runtime.visible_entities();
                let Some(target) = visible.players.iter().find(|entry| {
                    entry.actor.placement_identity() == wanted.identity
                        && entry.generation == wanted.generation
                }) else {
                    return Ok(Some(outcome(UseDisposition::TargetIllegal)));
                };
                if !visible_target && target.actor != actor {
                    return Ok(Some(outcome(UseDisposition::TargetIllegal)));
                }
                let at = runtime
                    .read_actor_position(actor)
                    .map_err(|_| ())?
                    .position();
                // Crystal actions.cpp default Action reach.
                if !oteryn_gameplay_items::reach::within_reach(
                    (at.x, at.y, i32::from(at.floor)),
                    (
                        target.position.x,
                        target.position.y,
                        i32::from(target.position.floor),
                    ),
                ) {
                    return Ok(Some(outcome(UseDisposition::TooFar)));
                }
                Some(target.actor)
            } else {
                None
            };
            if let Err(disposition) = states.prepare_novelty_effect(
                &runtime,
                native,
                NoveltyEffectRequest {
                    actor,
                    fence,
                    request: &request,
                    effects,
                    target,
                    now_us: self.owner_now().get(),
                },
            ) {
                return Ok(Some(outcome(disposition)));
            }
        }
        let receipt = self
            .root
            .consume_transform_detailed(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &content,
                false,
            )
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        self.finish_novelty_item(actor, &request, receipt)
            .await
            .map(Some)
    }

    pub(in crate::gameplay_transport) async fn finish_novelty_item(
        &self,
        actor: ExactActorRef,
        request: &ConsumeTransformRequest,
        receipt: ConsumeTransformResult,
    ) -> Result<UseOutcome, ()> {
        let lantern = if receipt.disposition == UseDisposition::Committed
            && receipt.effects["source_item_id"] == GHOSTSILVER_LANTERN_UNLIT
        {
            let states = self.states.spell_states.lock().await;
            states.pending_item_effects.iter().find_map(|memo| {
                (memo.actor == actor && memo.request.command == request.command)
                    .then_some((memo.fence, memo.cue_position))
            })
        } else {
            None
        };
        let clue = if let Some((fence, position)) = lantern {
            self.lantern_feedback(fence, position, false).await
        } else {
            None
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
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
            if let Some(clue) = clue {
                self.states
                    .chat
                    .push_quest_feedback(request.command.game_session_id(), clue);
            }
        }
        Ok(outcome(receipt.disposition))
    }
    /// Crystal 201189f8 data-global/scripts/quests/forgotten_knowledge/actions_lantern.lua:16-28.
    pub(super) async fn lantern_feedback(
        &self,
        fence: CurrentCharacterItemFence,
        at: MovementLocalPosition,
        in_house: bool,
    ) -> Option<&'static str> {
        if at.floor != 10 || at.x.abs_diff(32891) >= 2 || at.y.abs_diff(31619) >= 2 {
            return Some("The lantern begins to glow as you polish it.");
        }
        let inventory = if in_house {
            self.root
                .read_house_carried_inventory(
                    self.character,
                    self.holder,
                    fence,
                    *self.channel_id.as_bytes(),
                )
                .await
                .ok()?
        } else {
            self.root
                .read_item_use_inventory(self.character, self.holder, fence)
                .await
                .ok()?
        };
        let has_key = inventory
            .equipment
            .iter()
            .chain(
                inventory
                    .backpack
                    .iter()
                    .flat_map(|bag| bag.entries.iter().map(|entry| &entry.item)),
            )
            .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
            .any(|item| item.definition.production_key == "oteryn:item.tibia.i23733");
        Some(if has_key {
            "The lantern reveals a hidden door. You have the key to open it."
        } else {
            "The lantern reveals a hidden door. You need its key to open it."
        })
    }
}
