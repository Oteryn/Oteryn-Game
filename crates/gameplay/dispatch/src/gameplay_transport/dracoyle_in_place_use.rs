//! Actual gem USE WITH an inactive Ground or named-House statue.
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_view::ItemKey,
};
use crate::durability::consume_transform::{
    ConsumeTransformRequest, GroundActionContext, HouseActionContext,
};
use crate::foundation::{
    CommandId, CommandRef, ExactActorRef, MovementLocalPosition, RuntimeScopeRefV1,
};
use oteryn_gameplay_items::novelty_items::{GroundEffect, NoveltyCue, NoveltyEffects, NoveltyKind};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent};

fn effects(id: u32, position: MovementLocalPosition) -> NoveltyEffects {
    NoveltyEffects{kind:NoveltyKind::Transform,source_item_id:id,
  donor:"crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/actions/other/enchanting.lua:107-112".into(),
  ground:Some(GroundEffect{x:position.x,y:position.y,floor:position.floor}),
  cues:vec![NoveltyCue{binding:"appearance:effect/magic_green".into(),effect_id:18}],
  speech:None,private_speech:false,require_head:false,require_player_target:false,dice_value:None,costume:None,
  damage:None,achievement:None,achievement_progress:vec![],timed_output:None}
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_dracoyle_in_place(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let Some(native) = self.active_generation.and_then(|a| a.native_gameplay()) else {
            return Ok(None);
        };
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let mut request = ConsumeTransformRequest {
            command: CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).map_err(|_| ())?,
            ),
            source: [0; 16],
            target: Some([0; 16]),
            intent: oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent)
                .map_err(|_| ())?,
        };
        let house = if matches!(fence.runtime_scope, RuntimeScopeRefV1::Instance { .. }) {
            let runtime = self.runtime.lock().await;
            Some(
                HouseActionContext::from_native_owner(
                    &runtime,
                    actor,
                    command.game_session_id,
                    fence,
                )
                .map_err(|_| ())?,
            )
        } else {
            None
        };
        // A retained House receipt outlives the source view handles. A zero identity
        // can only reach this read-only lookup; the fresh locked source lookup refuses it.
        if let Some(context) = &house {
            let replay = self
                .root
                .enchant_dracoyle_in_house(
                    self.holder,
                    context.clone(),
                    request.clone(),
                    native,
                    &super::item_ref_admission::NativeItemWeights(native)
                        .with_wheel(self.premium_coordinator),
                    crate::durability::item_mint::TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: "oteryn:item.tibia.i9035".into(),
                        revision_ref: "definition-r1".into(),
                    },
                    0,
                )
                .await
                .map_err(|_| ())?;
            if replay.recipe.as_deref() == Some("oteryn:seasonal.dracoyle") {
                return Ok(outcome(replay.disposition));
            }
        }
        let (Some(ItemKey::Instance(gem)), Some(ItemKey::Instance(statue))) = (source, target)
        else {
            return Ok(None);
        };
        let inventory = if house.is_some() {
            self.root
                .read_hireling_use_inventory(
                    self.character,
                    self.holder,
                    fence,
                    *self.channel_id.as_bytes(),
                )
                .await
                .map_err(|_| ())?
        } else {
            self.root
                .read_item_use_inventory(self.character, self.holder, fence)
                .await
                .map_err(|_| ())?
        };
        let gem_present = inventory
            .backpack
            .iter()
            .flat_map(|bag| bag.entries.iter().map(|e| &e.item))
            .chain(inventory.nested.iter().map(|(_, e)| &e.item))
            .any(|i| {
                i.item_instance_id == gem && i.definition.production_key == "oteryn:item.tibia.i677"
            });
        if !gem_present {
            return Ok(None);
        }
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(gem))
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(statue))
        {
            return Ok(outcome(UseDisposition::Rejected));
        }
        request.source = statue;
        request.target = Some(gem);
        let _permit = self.spell_lane_permit().await.ok_or(())?;
        if let Some(context) = house {
            {
                let runtime = self.runtime.lock().await;
                let states = self.states.spell_states.lock().await;
                if states.is_dead(actor)
                    || states.has_pending_spell_commit(actor, command.game_session_id)
                    || runtime
                        .assert_house_actor_spell_unreserved(actor, command.game_session_id)
                        .is_err()
                {
                    return Ok(outcome(UseDisposition::Rejected));
                }
            }
            let Some(item) = self
                .root
                .read_house_interactive_source(self.holder, context.clone(), statue)
                .await
                .map_err(|_| ())?
            else {
                return Ok(None);
            };
            let id = match item.definition.production_key.as_str() {
                "oteryn:item.tibia.i9035" => 9035,
                "oteryn:item.tibia.i9040" => 9040,
                _ => return Ok(None),
            };
            let at = MovementLocalPosition {
                x: item.position[0],
                y: item.position[1],
                floor: i16::try_from(item.position[2]).map_err(|_| ())?,
            };
            let reservation = self
                .prepare_house_item_cues(actor, command, at, &effects(id, at))
                .await?;
            let receipt = self
                .root
                .enchant_dracoyle_in_house(
                    self.holder,
                    context,
                    request,
                    native,
                    &super::item_ref_admission::NativeItemWeights(native)
                        .with_wheel(self.premium_coordinator),
                    item.definition,
                    item.state_revision,
                )
                .await
                .map_err(|_| ())?;
            self.finish_house_item_cues(actor, command, reservation, &receipt)
                .await?;
            return Ok(outcome(receipt.disposition));
        }
        let Some(source) = self
            .root
            .read_ground_action_source(self.character, fence, statue)
            .await
            .map_err(|_| ())?
        else {
            return Ok(None);
        };
        let id = match source.definition().production_key.as_str() {
            "oteryn:item.tibia.i9035" => 9035,
            "oteryn:item.tibia.i9040" => 9040,
            _ => return Ok(None),
        };
        let at = source.position();
        let Some(room) = self.rooms.gate_ground_item_action() else {
            return Ok(outcome(UseDisposition::NotPossible));
        };
        let context = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || runtime.owner_fence().is_err()
            {
                return Ok(outcome(UseDisposition::Rejected));
            }
            GroundActionContext::from_native_owner(
                room,
                &runtime,
                actor,
                command.game_session_id,
                source,
            )
            .map_err(|_| ())?
        };
        let receipt = self
            .root
            .consume_transform_on_ground(
                &_permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &super::item_ref_admission::NativeItemWeights(native)
                    .with_wheel(self.premium_coordinator),
                &context,
            )
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        if receipt.disposition == UseDisposition::Committed && receipt.fresh {
            self.states.loop_ground.invalidate();
            let runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            for cue in &effects(id, at).cues {
                states
                    .sounds
                    .record_item_effect(&runtime, actor, at, cue.effect_id, false);
            }
        }
        Ok(outcome(receipt.disposition))
    }
}
