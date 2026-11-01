//! W07 uses the existing Item USE/use-with command and the existing Type2 craft owner.
//! The client selects carried handles; pinned content and durable custody select the outcome.

use super::ComposedFreshAdmission;
use super::connection::{UseCommand, UseOutcome};
use super::item_ref_admission::{NativeItemWeights, pinned_index};
use super::item_view::ItemKey;
use crate::durability::item_craft::soulpit::SoulCoreCraftRequest;
use crate::durability::item_mint::GroundPlacement;
use crate::foundation::{CommandId, CommandRef, ExactActorRef, RuntimeScopeRefV1};
use oteryn_gameplay_domain::domain::soulpit::{
    EXALTED_CORE_SOURCE_ID, SOUL_PRISM_SOURCE_ID, is_soul_core,
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
};

mod cadence;
mod entry;
mod handoff;
pub mod runtime;
pub use handoff::{
    BossActivityHandoff, BossActivityHandoffSlot, SoulpitCoreSelection, SoulpitEntryIntent,
};

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn dispatch_soulpit_items(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if let Some(result) = self
            .use_soulpit_obelisk(actor, command, intent, source, target)
            .await?
        {
            return Ok(Some(result));
        }
        self.use_soul_core(actor, command, intent, source, target)
            .await
    }

    /// A historical receipt is checked before resolving handles which a successful craft
    /// may have retired. Every live mutation still proves current actor and item authority.
    pub(super) async fn use_soul_core(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if !matches!(
            intent.target,
            UseTarget::Item(_) | UseTarget::ItemByDefinition(_)
        ) || !matches!(intent.use_with_map_item, Some(MapUseTarget::Item(_)))
        {
            return Ok(None);
        }
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Ok(None);
        };
        let id = CommandId::new(command.command_id).map_err(|_| ())?;
        let permit = self.spell_lane_permit().await.ok_or(())?;
        let mut request = SoulCoreCraftRequest {
            command: CommandRef::new(command.game_session_id, id),
            source: [0; 16],
            target: [0; 16],
            intent: encode_item_use_intent(intent).map_err(|_| ())?,
            ground: None,
        };
        if let Some(result) = self
            .root
            .craft_soul_core(
                &permit,
                self.character,
                self.holder,
                fence,
                request.clone(),
                native,
                &NativeItemWeights(native),
                true,
            )
            .await
            .map_err(|_| ())?
        {
            return Ok(Some(outcome(result)));
        }
        // Historical receipts above remain readable after ownership changes. A new
        // command must reject a stale scope before the fenced inventory reader;
        // otherwise its expected authority refusal would close the connection.
        {
            let runtime = self.runtime.lock().await;
            let binding = runtime.binding();
            if fence.game_session_id != command.game_session_id
                || fence.runtime_scope
                    != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
                || fence.scope_ownership_generation != binding.scope_generation()
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
        }
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        let source = match intent.target {
            UseTarget::Item(_) => match source {
                Some(ItemKey::Instance(id)) => Some(id),
                _ => None,
            },
            UseTarget::ItemByDefinition(wanted) => native
                .item_index()
                .and_then(|index| index.definition_at(wanted))
                .and_then(|(key, revision)| {
                    inventory
                        .first_unreserved(
                            &crate::durability::item_mint::TypedDefinitionRef {
                                family: "Item".into(),
                                production_key: key.into(),
                                revision_ref: revision.into(),
                            },
                            &[],
                        )
                        .ok()
                }),
            _ => None,
        };
        let Some(source) = source else {
            return Ok(None);
        };
        let carried = inventory
            .backpack
            .iter()
            .flat_map(|bag| &bag.entries)
            .chain(inventory.nested.iter().map(|(_, entry)| entry));
        let Some(source_item) = carried
            .clone()
            .find(|entry| entry.item.item_instance_id == source)
        else {
            return Ok(None);
        };
        let source_id = native
            .item_policy(
                &source_item.item.definition.production_key,
                &source_item.item.definition.revision_ref,
            )
            .and_then(|policy| {
                policy
                    .record()
                    .production_binding
                    .external_id
                    .parse::<u32>()
                    .ok()
            });
        if !source_id.is_some_and(|id| {
            is_soul_core(id) || matches!(id, SOUL_PRISM_SOURCE_ID | EXALTED_CORE_SOURCE_ID)
        }) {
            return Ok(None);
        }
        let Some(ItemKey::Instance(target)) = target else {
            return Ok(Some(outcome(UseDisposition::StaleState)));
        };
        if !carried
            .clone()
            .any(|entry| entry.item.item_instance_id == target)
        {
            return Ok(Some(outcome(UseDisposition::StaleState)));
        }
        for item in [source, target] {
            if !self
                .states
                .player_trades
                .physical(command.game_session_id, Some(item))
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
        }
        request.source = source;
        request.target = target;
        request.ground = {
            let mut runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            let binding = runtime.binding();
            if fence.game_session_id != command.game_session_id
                || fence.runtime_scope
                    != RuntimeScopeRefV1::channel(binding.world_id(), binding.channel_id())
                || fence.scope_ownership_generation != binding.scope_generation()
                || runtime.owner_fence().is_err()
                || pinned_index(&runtime, self.active_generation).is_none()
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                || states.is_dead(actor)
                || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return Ok(Some(outcome(UseDisposition::Rejected)));
            }
            let pin = runtime.content_pin();
            let content_revision = format!(
                "sha256:{}",
                pin.server_artifact_digest()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            );
            let native_room_placement_context = pin.frame_binding_digest().to_vec();
            let Some(at) = super::bundle_map_position(&mut runtime, self.movement_cells, actor)
            else {
                return Ok(Some(outcome(UseDisposition::NothingToUse)));
            };
            let mut spatial_position = Vec::with_capacity(10);
            spatial_position.extend_from_slice(&at.at.x.to_be_bytes());
            spatial_position.extend_from_slice(&at.at.y.to_be_bytes());
            spatial_position.extend_from_slice(&at.at.floor.to_be_bytes());
            Some(GroundPlacement {
                spatial_position,
                corpse_ref: Vec::new(),
                map_revision: crate::map::overlay::map_revision(at.map.overlay.base()),
                content_revision,
                native_room_placement_context,
            })
        };
        let result = self
            .root
            .craft_soul_core(
                &permit,
                self.character,
                self.holder,
                fence,
                request,
                native,
                &NativeItemWeights(native),
                false,
            )
            .await
            .map_err(|_| ())?
            .ok_or(())?;
        if result == UseDisposition::Committed {
            self.states.loop_ground.invalidate();
        }
        Ok(Some(outcome(result)))
    }
}
