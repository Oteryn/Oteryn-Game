//! Native carried5908 USE_WITH source-qualified static12816 rewardroom body.
use super::*;
use crate::durability::reward_claim_mint::{
    RewardClaimMintError, RewardClaimMintOutcome, RewardClaimMintRequest, RewardClaimRefusal,
    pumpkin_skinning::{self, PumpkinSkinningSource},
};
use crate::gameplay_transport::item_ref_admission::generation_item_facts;
use oteryn_gameplay_items::{pumpkin_skinning as rules, reward_bag::reward_bag_bounded_draw};
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn use_pumpkin_skinning(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if intent.use_with_map_item.is_none() && intent.use_with.is_none() {
            return Ok(None);
        }
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let encoded =
            oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent).map_err(|_| ())?;
        let wire: [u8; 32] = Sha256::digest(&encoded).into();
        if self
            .root
            .probe_pumpkin_skinning(self.character, fence.character_id, command_ref, wire)
            .await
            .map_err(|_| ())?
            .is_some()
        {
            return Ok(Some(pumpkin_outcome(UseDisposition::Committed)));
        }
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Ok(None);
        };
        let index = native.item_index().ok_or(())?;
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        let tool = match intent.target {
            UseTarget::Item(_) => match source {
                Some(ItemKey::Instance(id)) => Some(id),
                _ => None,
            },
            UseTarget::ItemByDefinition(wanted) => {
                index.definition_at(wanted).and_then(|(key, revision)| {
                    inventory
                        .first_unreserved(
                            &TypedDefinitionRef {
                                family: "Item".into(),
                                production_key: key.into(),
                                revision_ref: revision.into(),
                            },
                            &[],
                        )
                        .ok()
                })
            }
            _ => None,
        };
        let Some(knife) = inventory
            .backpack
            .iter()
            .flat_map(|b| &b.entries)
            .chain(inventory.nested.iter().map(|(_, e)| e))
            .find(|e| {
                Some(e.item.item_instance_id) == tool
                    && e.item.definition.production_key == "oteryn:item.tibia.i5908"
            })
        else {
            return Ok(None);
        };
        let knife_id = knife.item.item_instance_id;
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(knife_id))
        {
            return Ok(Some(pumpkin_outcome(UseDisposition::Rejected)));
        }
        let source = {
            let mut runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|f| f.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
                || pinned_index(&runtime, self.active_generation).is_none()
            {
                return Ok(Some(pumpkin_outcome(UseDisposition::Rejected)));
            }
            let Some(at) =
                super::super::bundle_map_position(&mut runtime, self.movement_cells, actor)
            else {
                return Ok(None);
            };
            let facts = at.map.facts.pinned();
            let map = super::super::world_map::MapViewSource {
                overlay: &at.map.overlay,
                facts: &facts,
                content_generation: at.content_generation,
                reset_epoch: self.reset_epoch,
            };
            let resolved = match &intent.use_with_map_item {
                Some(MapUseTarget::Object(object)) => {
                    super::super::world_map::resolve_map_target(&map, object, |_| None::<()>)
                        .ok()
                        .map(|r| (r.pos, r.placement_key))
                }
                Some(MapUseTarget::Item(_)) => target
                    .and_then(|key| super::super::world_map::resolve_map_handle(&map, key).ok())
                    .and_then(|r| match r {
                        super::super::world_map::MapHandleTarget::Base {
                            pos,
                            placement_key,
                            ..
                        } => Some((pos, placement_key)),
                        _ => None,
                    }),
                None => None,
            };
            let Some((pos, key)) = resolved else {
                return Ok(None);
            };
            if (pos.x, pos.y, pos.floor) != (33317, 32414, -15) {
                return Ok(None);
            }
            let tile = view::compose_tile(&at.map.overlay, &facts, pos)
                .map_err(|_| ())?
                .ok_or(())?;
            let before = tile
                .entries
                .iter()
                .find(|e| e.source.key() == MapEntryKey::Base(key))
                .ok_or(())?
                .facts;
            let MapDefinition::Item(reference) = before.definition else {
                return Ok(None);
            };
            let definition =
                super::super::item_look::typed_definition(index, reference).ok_or(())?;
            if definition.production_key != "oteryn:item.tibia.i12816"
                || before.count != 1
                || before.sub_type != 0
                || facts.house_tile(pos)
            {
                return Ok(None);
            }
            if i16::from(pos.floor) != at.at.floor
                || at.at.x.abs_diff(i32::from(pos.x)) > 1
                || at.at.y.abs_diff(i32::from(pos.y)) > 1
            {
                return Ok(Some(pumpkin_outcome(UseDisposition::TooFar)));
            }
            let mut placement = at.map.overlay.base().digest().to_vec();
            placement.extend(key.to_be_bytes());
            (
                placement,
                facts.object_revision(key),
                at.map.overlay.base().digest(),
            )
        };
        let Some(visit) = self
            .root
            .current_pumpkin_reward_visit(
                self.character,
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(|_| ())?,
                self.world_id,
            )
            .await
            .map_err(|_| ())?
        else {
            return Ok(Some(pumpkin_outcome(UseDisposition::Rejected)));
        };
        let source = PumpkinSkinningSource {
            death: visit.death,
            knife: knife_id,
            body_placement: source.0,
            body_revision: source.1,
            map_digest: source.2,
            wire_binding: wire,
        };
        let nonce = visit.draw_nonce();
        let reward = rules::select(5908, 12816, |domain, upper| {
            reward_bag_bounded_draw(visit.source_session, nonce, source.death, domain, upper)
        })
        .ok_or(())?;
        let key = format!("oteryn:item.tibia.i{}", reward.item_id);
        let revision = index.revision(&key).ok_or(())?;
        let item = generation_item_facts(
            native,
            &TypedDefinitionRef {
                family: "Item".into(),
                production_key: key,
                revision_ref: revision.into(),
            },
        )
        .map_err(|_| ())?;
        let Some(bag) = self
            .root
            .read_character_backpack(self.character, fence.character_id)
            .await
            .map_err(|_| ())?
        else {
            return Ok(Some(pumpkin_outcome(UseDisposition::NoRoom)));
        };
        let backpack = generation_item_facts(native, &bag.backpack.definition).map_err(|_| ())?;
        let request = RewardClaimMintRequest {
            command: command_ref,
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: pumpkin_skinning::claim_key(command_ref),
                revision_ref: pumpkin_skinning::CLAIM_REVISION.into(),
            },
            source_placement: pumpkin_skinning::source_key(&source),
            item,
            quantity: reward.count,
            contents: Vec::new(),
            root_text: None,
            backpack,
            content_revision: format!(
                "sha256:{}",
                native
                    .source_digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
            sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
            achievement: None,
            quest_transition: None,
            key_number: None,
        };
        let disposition = match self
            .root
            .claim_pumpkin_skinning(
                self.character,
                self.holder,
                fence,
                source,
                request,
                &NativeItemWeights(native).with_wheel(self.premium_coordinator),
            )
            .await
        {
            Ok(RewardClaimMintOutcome::Committed(_)) => {
                self.states.loop_ground.invalidate();
                let runtime = self.runtime.lock().await;
                if runtime.owner_fence().is_ok()
                    && runtime
                        .player_control_facts(actor, command.game_session_id)
                        .is_ok()
                {
                    self.states.chat.pumpkin_skinning_speech(
                        &runtime,
                        actor,
                        command.game_session_id,
                    );
                    if let Ok(position) = runtime.read_actor_position(actor) {
                        self.states
                            .spell_states
                            .lock()
                            .await
                            .sounds
                            .record_item_effect(&runtime, actor, position.position(), 28, false);
                    }
                }
                UseDisposition::Committed
            }
            Ok(RewardClaimMintOutcome::AlreadyCommitted(_)) => UseDisposition::Committed,
            Err(RewardClaimMintError::Refused(RewardClaimRefusal::AlreadyClaimed)) => {
                self.states.server_messages.pumpkin_skinning_exhausted(
                    command.game_session_id,
                    fence.connection_generation.get(),
                );
                UseDisposition::Exhausted
            }
            Err(RewardClaimMintError::Refused(
                RewardClaimRefusal::MainBackpackFull
                | RewardClaimRefusal::CarryingCapacity
                | RewardClaimRefusal::NoMainBackpack,
            )) => UseDisposition::NoRoom,
            Err(_) => UseDisposition::Rejected,
        };
        Ok(Some(pumpkin_outcome(disposition)))
    }
}
fn pumpkin_outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}
