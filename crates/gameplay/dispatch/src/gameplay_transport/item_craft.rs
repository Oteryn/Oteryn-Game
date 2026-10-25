//! CRAFT-1: existing bounded USE/use-with → current map station → atomic item owner.
use super::ComposedFreshAdmission;
use super::connection::{FreshAdmissionAuthority, UseCommand, UseOutcome};
use super::item_ref_admission::{NativeItemWeights, pinned_index};
use super::item_view::{ItemKey, SessionItemView};
use crate::durability::item_craft::ItemCraftRequest;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::map::view::{self, MapEntryKey};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseDisposition, UseIntent, UseTarget};

mod rope;

pub(super) async fn dispatch<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    command: UseCommand,
    intent: UseIntent,
    view: Option<&SessionItemView>,
) -> Option<UseOutcome> {
    let source = match intent.target {
        UseTarget::Item(handle) => view.and_then(|v| v.resolve(handle)),
        _ => None,
    };
    let target = match &intent.use_with_map_item {
        Some(MapUseTarget::Item(handle)) => view.and_then(|v| v.resolve(*handle)),
        _ => None,
    };
    authority
        .use_craft(actor, command, intent, source, target)
        .await
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_cooking_conversion(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Option<UseOutcome> {
        match self
            .use_rope_tool(actor, command, &intent, source, target)
            .await
        {
            Ok(Some(outcome)) => return Some(outcome),
            Ok(None) => {}
            Err(()) => return None,
        }
        let disposition = |d| {
            Some(UseOutcome {
                disposition: d,
                ..UseOutcome::rejected()
            })
        };
        let fence = command.item_fence?;
        let id = CommandId::new(command.command_id).ok()?;
        let native = self.active_generation?.native_gameplay()?;
        let _permit = self.spell_lane_permit().await?;
        let mut request = ItemCraftRequest {
            command: CommandRef::new(command.game_session_id, id),
            source: [0; 16],
            station: TypedDefinitionRef {
                family: String::new(),
                production_key: String::new(),
                revision_ref: String::new(),
            },
            intent: oteryn_protocol_oteryn::world_object::encode_item_use_intent(&intent).ok()?,
        };
        // Replay precedes current handles and custody, including input retirement and reconnect.
        if let Some(result) = self
            .root
            .craft_item(
                &_permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &NativeItemWeights(native),
                true,
            )
            .await
            .ok()?
        {
            return disposition(result);
        }
        let Some(ItemKey::Instance(source)) = source else {
            return disposition(UseDisposition::StaleState);
        };
        request.source = source;
        request.station = {
            let mut runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return disposition(UseDisposition::Rejected);
            }
            let index = pinned_index(&runtime, self.active_generation)?;
            let at = super::bundle_map_position(&mut runtime, self.movement_cells, actor)?;
            let facts = at.map.facts.pinned();
            let map = super::world_map::MapViewSource {
                overlay: &at.map.overlay,
                facts: &facts,
                content_generation: at.content_generation,
                reset_epoch: self.reset_epoch,
            };
            let resolved = match &intent.use_with_map_item {
                Some(MapUseTarget::Object(object)) => {
                    super::world_map::resolve_map_target(&map, object, |_| None::<()>)
                        .ok()
                        .map(|t| (t.pos, MapEntryKey::Base(t.placement_key)))
                }
                Some(MapUseTarget::Item(_)) => target
                    .and_then(|key| super::world_map::resolve_map_handle(&map, key).ok())
                    .and_then(|t| match t {
                        super::world_map::MapHandleTarget::Base {
                            pos, placement_key, ..
                        } => Some((pos, MapEntryKey::Base(placement_key))),
                        _ => None,
                    }),
                None => None,
            };
            let Some((pos, key)) = resolved else {
                return disposition(UseDisposition::StaleState);
            };
            if i16::from(pos.floor) != at.at.floor
                || at.at.x.abs_diff(i32::from(pos.x)) > 1
                || at.at.y.abs_diff(i32::from(pos.y)) > 1
            {
                return disposition(UseDisposition::TooFar);
            }
            let tile = view::compose_tile(&at.map.overlay, &facts, pos).ok()??;
            let entry = tile.entries.iter().find(|e| e.source.key() == key)?;
            let MapDefinition::Item(reference) = entry.facts.definition else {
                return disposition(UseDisposition::NothingToUse);
            };
            super::item_look::typed_definition(index, reference)?
        };
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(source))
        {
            return disposition(UseDisposition::Rejected);
        }
        let result = self
            .root
            .craft_item(
                &_permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &NativeItemWeights(native),
                false,
            )
            .await
            .ok()??;
        disposition(result)
    }
}
