//! Qualify a Quest floor reward from the current native map turn.
use super::*;
use crate::content::{ReferenceItemField, native_gameplay::NativeGameplayState};
use crate::durability::{
    item_mint::{GroundPlacement, TypedDefinitionRef},
    native_map_items_abi::{
        NativeMapInitializationProof, NativeMapItemPlacement, NativeMapOwnerBinding,
        map_initializer_seal,
    },
};
use oteryn_gameplay_quests::quest::reward_callbacks::ItemAmount;

pub(super) struct CurrentQuestFloorReward<'a> {
    runtime: &'a ChannelRuntimeV1,
    map: &'a BundleMap,
    content: &'a NativeGameplayState,
    placement: NativeMapItemPlacement,
}

impl map_initializer_seal::Sealed for CurrentQuestFloorReward<'_> {}

impl NativeMapInitializationProof for CurrentQuestFloorReward<'_> {
    fn current_binding(&self) -> Result<NativeMapOwnerBinding, &'static str> {
        self.runtime
            .owner_fence()
            .map_err(|_| "quest floor reward owner")?;
        let binding = self.runtime.binding();
        let pin = self.runtime.content_pin();
        if self.content.source_digest() != pin.server_artifact_digest()
            || crate::map::overlay::map_revision(self.map.overlay.base())
                != format!("sha256:{}", hex(&pin.map_revision_digest()))
        {
            return Err("quest floor reward content or map mismatch");
        }
        Ok(NativeMapOwnerBinding {
            world: binding.world_id(),
            channel: binding
                .channel_id()
                .ok_or("quest floor reward requires a Channel")?,
            scope_generation: binding.scope_generation().get(),
            content_digest: pin.server_artifact_digest(),
            map_digest: pin.map_revision_digest(),
            frame_digest: pin.frame_binding_digest(),
        })
    }

    fn placements(&self) -> &[NativeMapItemPlacement] {
        std::slice::from_ref(&self.placement)
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    // Crystal 201189f8 data-global/scripts/quests/what_a_foolish_quest/action_cat_basket.lua:32.
    #[allow(
        clippy::too_many_arguments,
        reason = "bound Ground source and retained quest occurrence"
    )]
    pub(super) fn prepare_quest_floor_reward<'a>(
        &'a self,
        runtime: &'a ChannelRuntimeV1,
        actor: ExactActorRef,
        map: &'a BundleMap,
        callback: &Callback,
        position: TilePos,
        command: CommandRef,
        fence: CurrentCharacterGameplayFence,
        reward: &ItemAmount,
    ) -> Option<CurrentQuestFloorReward<'a>> {
        let content = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())?;
        let revision = content.item_index()?.revision(&reward.item)?;
        let record = content.item_policy(&reward.item, revision)?.record();
        let ReferenceItemField::Known(physical) = &record.semantics.physical else {
            return None;
        };
        use crate::content::native_gameplay::NativeItemDestination;
        if !runtime
            .bound_attacker_lease(actor, command.game_session_id())
            .is_ok_and(|lease| {
                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            })
            || !record.admission.materializable
            || !record
                .admission
                .legal_destinations
                .contains(&NativeItemDestination::Ground)
            || record.attributes.blocks_movement != Some(false)
            || record.attributes.blocks_projectile != Some(false)
            || physical.movable != ReferenceItemField::Known(true)
            || physical.pickupable != ReferenceItemField::Known(true)
            || map.facts.house_tile(position)
            || map
                .overlay
                .base()
                .tile(position.x, position.y, position.floor)
                .is_none()
        {
            return None;
        }
        let source = format!(
            "quest-floor-v1/{}/{}",
            hex(command.game_session_id().as_bytes()),
            command.command_id().get()
        );
        let pin = runtime.content_pin();
        Some(CurrentQuestFloorReward {
            runtime,
            map,
            content,
            placement: NativeMapItemPlacement {
                placement_key: source.clone(),
                definition: TypedDefinitionRef {
                    family: record.production_definition.family.clone(),
                    production_key: record.production_definition.production_key.clone(),
                    revision_ref: record.production_definition.revision_ref.clone(),
                },
                ground: GroundPlacement {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            position.x,
                            position.y,
                            position.floor,
                        ),
                    corpse_ref: b"quest-floor-v1".to_vec(),
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision: format!("sha256:{}", hex(&pin.server_artifact_digest())),
                    native_room_placement_context: pin.frame_binding_digest().to_vec(),
                },
                quantity: u32::from(reward.count),
                source_binding: source.into_bytes(),
                source_attributes: serde_json::json!({
                    "kind": "quest_floor_reward",
                    "character": hex(fence.character_id.as_bytes()),
                    "session": hex(command.game_session_id().as_bytes()),
                    "command": command.command_id().get().to_string(),
                    "transition": callback.key,
                }),
                blocks_movement: false,
                blocks_projectile: false,
                immovable_block_solid: false,
                owner_kind: None,
            },
        })
    }
}
