//! Ordinary USE of HouseInterior items through the existing named-House actor and map view.
#[path = "house_santa_gift_use.rs"]
mod santa_gifts;
#[path = "house_item_side_use.rs"]
pub(super) mod side_effects;
use super::item_view::{ItemKey, SessionItemView};
use super::world_map::{HouseMapEntry, MapUpdate, MapViewError, MapViewSource, SessionMapView};
use super::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, UseCommand, UseOutcome},
};
use crate::durability::consume_transform::{ConsumeTransformRequest, HouseActionContext};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};

#[path = "house_item_cupcakes.rs"]
mod cupcakes;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_house_event_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if let Some(outcome) = self
            .use_house_cupcake(actor, command, intent, source)
            .await?
        {
            return Ok(Some(outcome));
        }
        self.act_house_event_item(actor, command, intent, source, false, None)
            .await
    }

    pub(super) async fn rotate_house_event_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        observed: Option<std::num::NonZeroU32>,
    ) -> Result<Option<UseOutcome>, ()> {
        self.act_house_event_item(actor, command, intent, source, true, observed)
            .await
    }

    async fn act_house_event_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        rotate: bool,
        observed: Option<std::num::NonZeroU32>,
    ) -> Result<Option<UseOutcome>, ()> {
        let refuse = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        if !matches!(intent.target, UseTarget::Item(_))
            || intent.use_with.is_some()
            || intent.use_with_map_item.is_some()
            || intent.use_with_position.is_some()
        {
            return Ok(None);
        }
        let Some(fence) = command.item_fence else {
            return Ok(refuse(UseDisposition::Rejected));
        };
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return Ok(refuse(UseDisposition::NotPossible));
        };
        let _lane = self.spell_lane_permit().await.ok_or(())?;
        let context = {
            let runtime = self.runtime.lock().await;
            if super::item_ref_admission::pinned_index(&runtime, self.active_generation).is_none() {
                return Ok(refuse(UseDisposition::StaleState));
            }
            match HouseActionContext::from_native_owner(
                &runtime,
                actor,
                command.game_session_id,
                fence,
            ) {
                Ok(context) => context,
                Err(_) => return Ok(refuse(UseDisposition::StaleState)),
            }
        };
        let blocked_furniture_cells = self.door.lock().await.blocking_cells().clone();
        let context = context.qualify_furniture_cell(self.movement_cells, &blocked_furniture_cells);
        // Settle a due source clock under the same current named-House fence
        // before selecting its next source action (the regular tick does this too).
        self.root
            .expire_house_furniture(self.holder, fence, context.house().clone())
            .await
            .map_err(|_| ())?;
        let mut bytes =
            oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent).map_err(|_| ())?;
        // A reconnect cannot replay USE as Rotate when its target bytes happen
        // to be identical: the immutable command intent includes its purpose.
        if rotate {
            bytes = super::world_rotate::bound_intent(intent, observed).map_err(|_| ())?;
        }
        let request = ConsumeTransformRequest {
            command: CommandRef::new(
                command.game_session_id,
                CommandId::new(command.command_id).map_err(|_| ())?,
            ),
            // A retired handle may still name an existing immutable same-command receipt.
            source: match source {
                Some(ItemKey::Instance(id)) => id,
                _ => [0; 16],
            },
            target: None,
            intent: bytes,
        };
        let expected = observed
            .and_then(|reference| native.item_index()?.definition_at(reference))
            .map(
                |(key, revision)| crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: key.into(),
                    revision_ref: revision.into(),
                },
            );
        let result = if rotate {
            self.root
                .rotate_in_house(
                    self.holder,
                    context.clone(),
                    request.clone(),
                    native,
                    expected,
                )
                .await
        } else {
            self.root
                .consume_transform_in_house(self.holder, context.clone(), request.clone(), native)
                .await
        }
        .map_err(|_| ())?;
        // A current House source that disappeared or changed custody has a
        // definite refusal. Do not fall through to Channel-only item handlers.
        if result.recipe.is_none() && result.disposition == UseDisposition::NothingToUse {
            if !rotate
                && let Some(outcome) = self
                    .use_house_santa_gift(actor, command, context.clone(), request.clone(), native)
                    .await?
            {
                return Ok(Some(outcome));
            }
            return self
                .use_house_interactive(actor, command, context, request, native)
                .await;
        }
        Ok(refuse(result.disposition))
    }

    /// The existing domain-17 planner reads actual HouseInterior custody; Channel
    /// Ground overlays and their mutation eligibility are not imported into it.
    pub(super) async fn observe_house_item_map(
        &self,
        admitted: AdmittedSession,
        view: &mut SessionMapView,
        items: &mut SessionItemView,
    ) -> Option<Result<Option<MapUpdate>, MapViewError>> {
        let actor = admitted.runtime_actor?;
        let fence = admitted.item_fence?;
        let _lane = self.spell_lane_permit().await?;
        let (context, content_generation, index) = {
            let runtime = self.runtime.lock().await;
            (
                HouseActionContext::from_native_owner(
                    &runtime,
                    actor,
                    admitted.game_session_id,
                    fence,
                )
                .ok()?,
                runtime.content_pin().client_artifact_digest(),
                super::item_ref_admission::pinned_index(&runtime, self.active_generation)?,
            )
        };
        let at = context.position();
        let map = self.bundle_map()?;
        let snapshot = self
            .root
            .read_house_action_items(self.holder, context.clone())
            .await
            .ok()??;
        // The custody read can wait while a remote assignment changes generations.
        // Re-prove the named-House owner before exposing its historical items.
        if !self
            .root
            .house_action_current(self.holder, &context)
            .await
            .ok()?
        {
            return None;
        }
        {
            let runtime = self.runtime.lock().await;
            let current = HouseActionContext::from_native_owner(
                &runtime,
                actor,
                admitted.game_session_id,
                fence,
            )
            .ok()?;
            if current.position() != at
                || super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                    .is_none()
            {
                return None;
            }
        }
        let native = self.active_generation?.native_gameplay()?;
        let mut entries = Vec::with_capacity(snapshot.items.len());
        for item in snapshot.items {
            let [x, y, floor] =
                HouseActionContext::decode_spatial_position(&item.spatial_position)?;
            let position = crate::map::overlay::TilePos {
                x: u16::try_from(x).ok()?,
                y: u16::try_from(y).ok()?,
                floor: i8::try_from(floor).ok()?.checked_neg()?,
            };
            let reference = super::item_ref_admission::definition_ref(index, &item.definition)?;
            let policy = native
                .item_policy(
                    &item.definition.production_key,
                    &item.definition.revision_ref,
                )?
                .record();
            entries.push(HouseMapEntry {
                position,
                item: item.item_instance_id,
                ordinal: item.stack_ordinal,
                facts: crate::map::view::EntryFacts {
                    definition: oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference),
                    terrain_kind: None,
                    appearance_id: crate::map::view::appearance_id(
                        &item.definition.production_key,
                        None,
                    ),
                    blocks_projectile: policy.attributes.blocks_projectile?,
                    pickupable: false,
                    bound: false,
                    count: item.quantity,
                    sub_type: item.key_number,
                },
            });
        }
        // Ordinary-death Ground is a separate exact House namespace. Its immutable
        // receipt proves the root; descendants retain their container parent UUIDs.
        // Keep the original positive native floor in custody, negating only here.
        let death_ground = self
            .root
            .read_house_death_ground(self.holder, &context)
            .await
            .ok()??;
        for entry in death_ground {
            let item = entry.item;
            let cell = crate::durability::spell_items_abi::decode_ground_cell(
                &item.ground.spatial_position,
            )
            .ok()?;
            let position = crate::map::overlay::TilePos {
                x: u16::try_from(cell.x).ok()?,
                y: u16::try_from(cell.y).ok()?,
                floor: i8::try_from(cell.z).ok()?.checked_neg()?,
            };
            let reference = super::item_ref_admission::definition_ref(index, &item.definition)?;
            let policy = native
                .item_policy(
                    &item.definition.production_key,
                    &item.definition.revision_ref,
                )?
                .record();
            entries.push(HouseMapEntry {
                position,
                item: item.item_instance_id,
                ordinal: entry.stack_ordinal,
                facts: crate::map::view::EntryFacts {
                    definition: oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference),
                    terrain_kind: None,
                    appearance_id: crate::map::view::appearance_id(
                        &item.definition.production_key,
                        None,
                    ),
                    blocks_projectile: policy.attributes.blocks_projectile?,
                    pickupable: true,
                    bound: false,
                    count: item.quantity,
                    sub_type: 0,
                },
            });
        }
        let facts = map.facts.base_only();
        let source = MapViewSource {
            overlay: &map.overlay,
            facts: &facts,
            content_generation,
            reset_epoch: self.reset_epoch,
        };
        let actor = oteryn_protocol_oteryn::world_spatial::ActorPosition {
            x: at[0],
            y: at[1],
            floor: i16::try_from(at[2]).ok()?.checked_neg()?,
        };
        Some(view.update_house(items, &source, actor, &entries))
    }
}
