//! Ordinary USE of HouseInterior items through the existing named-House actor and map view.
use super::item_view::{ItemKey, SessionItemView};
use super::world_map::{HouseMapEntry, MapUpdate, MapViewError, MapViewSource, SessionMapView};
use super::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, UseCommand, UseOutcome},
};
use crate::durability::consume_transform::{ConsumeTransformRequest, HouseActionContext};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent, UseTarget};

#[path = "house_item_use/presentation.rs"]
mod presentation;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_house_event_item(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        let refuse = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        if !matches!(intent.target, UseTarget::Item(_))
            || intent.use_with.is_some()
            || matches!(
                intent.use_with_map_item.as_ref(),
                Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Item(_))
            )
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
        if intent.use_with_map_item.is_none()
            && let Some(item @ ItemKey::Instance(_)) = source
            && let Some(outcome) = self
                .try_use_house_rotten_essence(actor, command, item)
                .await
        {
            return Ok(Some(outcome));
        }
        let _lane = self.spell_lane_permit().await.ok_or(())?;
        let mut context = {
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
            intent: oteryn_protocol_oteryn::world_object::encode_item_use_intent(intent)
                .map_err(|_| ())?,
        };
        if let Some(oteryn_protocol_oteryn::world_object::MapUseTarget::Object(target)) =
            intent.use_with_map_item.as_ref()
        {
            // A successful bed use consumed its source kit and changed both bed
            // revisions. Probe history before resolving either current handle.
            if let Some(replay) = self
                .root
                .replay_house_item_action(self.holder, &context, &request)
                .await
                .map_err(|_| ())?
            {
                return Ok(refuse(replay.disposition));
            }
            if !self
                .root
                .house_source_is_bed_kit(self.holder, &context, request.source)
                .await
                .map_err(|_| ())?
            {
                return Ok(None);
            }
            let Some(target) = self.prepare_house_bed_changes(&context, target).await else {
                return Ok(refuse(UseDisposition::NotPossible));
            };
            context = context.with_bed_target(target);
        }
        // Crystal carpets.lua:185-201 uses the actual tile stack. Capture only
        // native base entries; Channel-created items cannot become House facts.
        if let Some(map) = self.bundle_map() {
            let changes = self
                .house_base_decoration_facts(&context, map.overlay.base().digest())
                .await
                .ok_or(())?;
            let base = map.facts.base_only();
            let facts = super::world_map::DecoratedFacts {
                base: &base,
                changes: &changes,
            };
            let index = native.item_index().ok_or(())?;
            let at = context.position();
            let mut base_items = Vec::new();
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let Some(x) = at[0].checked_add(dx).and_then(|x| u16::try_from(x).ok()) else {
                        continue;
                    };
                    let Some(y) = at[1].checked_add(dy).and_then(|y| u16::try_from(y).ok()) else {
                        continue;
                    };
                    let Some(floor) = i8::try_from(at[2]).ok().and_then(i8::checked_neg) else {
                        continue;
                    };
                    let position = crate::map::overlay::TilePos { x, y, floor };
                    let mut entries = Vec::new();
                    crate::map::view::compose_into(
                        map.overlay.base(),
                        None,
                        &facts,
                        position,
                        &mut entries,
                    )
                    .map_err(|_| ())?;
                    for (ordinal, entry) in entries.iter().enumerate() {
                        if entry.facts.terrain_kind.is_some() {
                            continue;
                        }
                        let oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference) =
                            entry.facts.definition
                        else {
                            continue;
                        };
                        let definition =
                            super::item_look::typed_definition(index, reference).ok_or(())?;
                        base_items.push(
                            crate::durability::consume_transform::HouseDecorationBaseItem {
                                position: [i32::from(x), i32::from(y), at[2]],
                                ordinal: ordinal as u64,
                                definition,
                            },
                        );
                    }
                }
            }
            context = context.with_decoration_base_items(base_items);
        }
        let publication_house = context.house().clone();
        let result = self
            .root
            .consume_transform_in_house(self.holder, context, request, native)
            .await
            .map_err(|_| ())?;
        if result.fresh
            && result
                .effects
                .get("kind")
                .and_then(serde_json::Value::as_str)
                == Some("house_decoration")
        {
            presentation::publish(
                &self.runtime,
                &self.states.spell_states,
                &self.states.chat,
                presentation::Publication {
                    actor,
                    fence,
                    house: &publication_house,
                    effects: &result.effects,
                },
            )
            .await;
        }
        // A current House source that disappeared or changed custody has a
        // definite refusal. Do not fall through to Channel-only item handlers.
        if result.recipe.is_none() && result.disposition == UseDisposition::NothingToUse {
            return Ok(None);
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
        if !self
            .root
            .settle_house_timed_decorations(self.holder, fence, context.house().clone())
            .await
            .ok()?
        {
            return None;
        }
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
        let base_changes = self
            .house_base_decoration_facts(&context, map.overlay.base().digest())
            .await?;
        let base_facts = map.facts.base_only();
        let facts = super::world_map::DecoratedFacts {
            base: &base_facts,
            changes: &base_changes,
        };
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
