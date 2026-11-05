//! SRVGAP-C4 dedicated USE leaf. Historical receipts precede current item handles.
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_view::ItemKey,
};
use crate::durability::item_use::novelty_appearance::{ActionPremium, ActionRequest};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_items::usable_actions;
use oteryn_protocol_oteryn::world_object::{UseIntent, UseTarget};

impl ActionPremium for super::spell_premium_coordinator::RegisteredSpellAccessOwner {
    fn current(&self, account: [u8; 16]) -> bool {
        use super::spell_access_facts::CurrentSpellAccessOwner;
        self.account_id() == Some(account) && self.premium_gate().premium_current()
    }
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_usable_action(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        visible_target: bool,
        direct: bool,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(native) = self.active_generation.and_then(|a| a.native_gameplay()) else {
            return Ok(None);
        };
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        // This leaf composes the Channel item owner; scoped House/Instance owners keep routing.
        if !matches!(
            fence.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        ) {
            return Ok(None);
        }
        if intent.use_with_map_item.is_some() || intent.use_with_position.is_some() {
            return Ok(None);
        }
        let mut encoded =
            oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent).map_err(|_| ())?;
        let source = match source {
            Some(ItemKey::Instance(id)) => id,
            _ => [0; 16],
        };
        if direct {
            encoded.extend_from_slice(&source);
        }
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let pending = {
            let states = self.states.spell_states.lock().await;
            states.c4_request(actor, command_ref, &encoded)
        };
        let mut request = pending.unwrap_or(ActionRequest {
            command: command_ref,
            source,
            intent: encoded,
            player_target: false,
        });
        let premium = self
            .refresh_spell_access(actor, command.game_session_id)
            .await;
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let content = super::item_ref_admission::NativeItemWeights(native)
            .with_wheel(self.premium_coordinator);
        if let Some(receipt) = self
            .root
            .use_novelty_appearance(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &content,
                premium.as_ref(),
                true,
            )
            .await
            .map_err(|_| ())?
        {
            return self
                .finish_usable_action(actor, command, &request, &receipt)
                .await
                .map(Some);
        }

        // Main's event/component owner may already have committed this wire command.
        // Defer before inspecting a remaining source unit so a replay cannot spend it again.
        if !direct
            && self
                .root
                .consume_transform_detailed(
                    &permit,
                    self.character,
                    self.holder,
                    fence,
                    crate::durability::consume_transform::ConsumeTransformRequest {
                        command: command_ref,
                        source: request.source,
                        target: None,
                        intent: request.intent.clone(),
                    },
                    native,
                    &content,
                    true,
                )
                .await
                .map_err(|_| ())?
                .is_some()
        {
            return Ok(None);
        }

        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        if request.source == [0; 16]
            && let UseTarget::ItemByDefinition(reference) = intent.target
            && let Some(definition) = native
                .item_index()
                .and_then(|index| super::item_look::typed_definition(index, reference))
            && usable_actions::supported(&definition.production_key)
        {
            request.source = inventory
                .first_unreserved(&definition, &[])
                .map_err(|_| ())?;
        }
        let item = inventory
            .backpack
            .iter()
            .flat_map(|b| b.entries.iter().map(|e| &e.item))
            .chain(inventory.nested.iter().map(|(_, e)| &e.item))
            .chain(inventory.equipment.iter())
            .find(|i| i.item_instance_id == request.source);
        // Probe after a source retires; unrelated live items continue through their existing owners.
        if let Some(item) = item {
            if !usable_actions::supported(&item.definition.production_key) {
                return Ok(None);
            }
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let mut player_target = false;
            let target = intent.use_with.and_then(|target| {
                if !visible_target {
                    return None;
                }
                runtime
                    .positioned_melee_census()
                    .ok()?
                    .into_iter()
                    .find(|(a, _, _)| {
                        a.placement_identity() == target.identity
                            && a.actor_local_generation() == target.generation
                    })
                    .map(|(actor, _, session)| {
                        player_target = session.is_some();
                        actor
                    })
            });
            if intent.use_with.is_some() && target.is_none() {
                return Ok(Some(UseOutcome::rejected()));
            }
            // Crystal src/lua/creature/actions.cpp:269-280,688-693: default Action reach.
            if let Some(target) = target {
                let from = runtime
                    .read_actor_position(actor)
                    .map_err(|_| ())?
                    .position();
                let to = runtime
                    .read_actor_position(target)
                    .map_err(|_| ())?
                    .position();
                if !oteryn_gameplay_items::reach::within_reach(
                    (from.x, from.y, i32::from(from.floor)),
                    (to.x, to.y, i32::from(to.floor)),
                ) {
                    return Ok(Some(UseOutcome {
                        disposition: oteryn_protocol_oteryn::world_object::UseDisposition::TooFar,
                        ..UseOutcome::rejected()
                    }));
                }
            }
            request.player_target = player_target;
            if let Err(disposition) = states.prepare_c4(
                &runtime,
                actor,
                &request,
                usable_actions::item_id(&item.definition.production_key).ok_or(())?,
                self.owner_now().get(),
                target,
            ) {
                return Ok(Some(UseOutcome {
                    disposition,
                    ..UseOutcome::rejected()
                }));
            }
        }
        if item.is_some()
            && !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(request.source))
        {
            self.states
                .spell_states
                .lock()
                .await
                .release_c4(actor, command_ref);
            return Ok(Some(UseOutcome::rejected()));
        }
        let receipt = self
            .root
            .use_novelty_appearance(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &content,
                premium.as_ref(),
                false,
            )
            .await
            .map_err(|_| ())?;
        let Some(receipt) = receipt else {
            self.states
                .spell_states
                .lock()
                .await
                .release_c4(actor, command_ref);
            return Ok(None);
        };
        self.finish_usable_action(actor, command, &request, &receipt)
            .await
            .map(Some)
    }
    async fn finish_usable_action(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        request: &ActionRequest,
        receipt: &crate::durability::item_use::novelty_appearance::ActionReceipt,
    ) -> Result<UseOutcome, ()> {
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if let Some((action, target)) = states.finish_c4(&runtime, actor, request, receipt)? {
            if let Some(text) = &action.text {
                self.states.chat.publish_item_action(
                    &runtime,
                    actor,
                    command.game_session_id,
                    text,
                    action.private_text,
                );
                self.states.tutorial_sessions.notice(
                    command.game_session_id,
                    oteryn_protocol_oteryn::tutorial_notices::TutorialNotice::StatusText(
                        text.clone(),
                    ),
                );
            }
            if let Some(presentations) = states.presentations.as_mut() {
                // Presentation is best effort; a full outbox never refuses a committed grant.
                let _ = presentations.publish_item_action(
                    &runtime,
                    actor,
                    request.command,
                    action.item_id,
                    &action.effects,
                    if action.item_id == 3002 { target } else { None },
                );
            }
        }
        Ok(UseOutcome {
            disposition: receipt.disposition,
            ..UseOutcome::rejected()
        })
    }
    pub(super) async fn use_usable_action_instance(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        item: ItemKey,
    ) -> Option<Option<UseOutcome>> {
        let ItemKey::Instance(id) = item else {
            return None;
        };
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1)?),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        };
        // oteryn-choice: direct owner callers bind the instance in addition to synthetic USE bytes;
        // production wire callers bind the original bounded USE bytes before handle resolution.
        let result = self
            .use_usable_action(
                actor,
                command,
                &intent,
                Some(ItemKey::Instance(id)),
                false,
                true,
            )
            .await;
        match result {
            Ok(Some(outcome)) => Some(Some(outcome)),
            Ok(None) => None,
            Err(()) => Some(None),
        }
    }
}
