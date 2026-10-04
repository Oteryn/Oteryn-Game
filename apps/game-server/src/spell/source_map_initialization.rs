//! Actual active source-map placement producer. No player-derived mint cause.
//! Only the compatible native Item policy can turn a source alias into an Item
//! definition. Beds/teleports/doors retain their required owner kind; a durable
//! row is not an assertion that those interaction semantics are implemented.
use crate::content::{QualifiedNativeEntryRoom, native_gameplay::NativeGameplayState};
use crate::durability::native_map_items_abi::*;
use crate::foundation::ChannelRuntimeV1;

pub(crate) struct CurrentNativeMapInitialization<'owner> {
    runtime: &'owner ChannelRuntimeV1,
    room: &'owner QualifiedNativeEntryRoom,
    content: &'owner NativeGameplayState,
    placements: Vec<NativeMapItemPlacement>,
}
impl crate::durability::native_map_items_abi::map_initializer_seal::Sealed
    for CurrentNativeMapInitialization<'_>
{
}
impl NativeMapInitializationProof for CurrentNativeMapInitialization<'_> {
    fn current_binding(&self) -> Result<NativeMapOwnerBinding, &'static str> {
        Self::qualify_current_binding(self.runtime, self.room, self.content)
    }

    fn placements(&self) -> &[NativeMapItemPlacement] {
        &self.placements
    }
}
impl<'owner> CurrentNativeMapInitialization<'owner> {
    pub(crate) fn qualify_current_binding(
        runtime: &ChannelRuntimeV1,
        room: &QualifiedNativeEntryRoom,
        content: &NativeGameplayState,
    ) -> Result<NativeMapOwnerBinding, &'static str> {
        runtime
            .owner_fence()
            .map_err(|_| "current native map owner unavailable")?;
        let pin = runtime.content_pin();
        let scope = room.movement_cells().scope();
        if content.source_digest() != pin.server_artifact_digest()
            || room.compiled().server_digest() != pin.server_artifact_digest()
            || room.compiled().client_digest() != pin.client_artifact_digest()
            || scope.world_id != runtime.binding().world_id()
            || scope.generation_digest != pin.server_artifact_digest()
            || room.frame_binding().digest() != pin.frame_binding_digest()
            || room.map_revision_digest() != pin.map_revision_digest()
        {
            return Err("source map current artifact/scope mismatch");
        }
        let source = room
            .source_world()
            .ok_or("source map initializer requires actual source world")?;
        if source.scope() != scope {
            return Err("source map scope substitution");
        }
        Ok(NativeMapOwnerBinding {
            world: runtime.binding().world_id(),
            channel: runtime.binding().channel_id(),
            scope_generation: runtime.binding().scope_generation().get(),
            content_digest: pin.server_artifact_digest(),
            map_digest: pin.map_revision_digest(),
            frame_digest: pin.frame_binding_digest(),
        })
    }
    pub(crate) fn from_current_owner(
        runtime: &'owner ChannelRuntimeV1,
        room: &'owner QualifiedNativeEntryRoom,
        content: &'owner NativeGameplayState,
    ) -> Result<Self, &'static str> {
        let mut proof = Self {
            runtime,
            room,
            content,
            placements: Vec::new(),
        };
        proof.current_binding()?;
        let source = room.source_world().ok_or("missing source map")?;
        if source.pending_mutable_placements().len() > 128 {
            return Err("source map mutable placement budget exceeded");
        }
        for placement in source.pending_mutable_placements() {
            let alias = placement.binding();
            let id = alias
                .external_id
                .parse::<u32>()
                .map_err(|_| "source map alias is not canonical")?;
            let policy = content
                .item_policy_for_source_id(id)
                .ok_or("source map Item definition not active")?;
            let record = policy.record();
            let active_alias = &record.production_binding;
            if policy.source_digest() != content.source_digest()
                || source.native_item_binding(id) != Some(active_alias)
                || active_alias.target != alias.target
            {
                return Err("source map alias does not bind active native Item");
            }
            let item =
                super::world_items_execution::QualifiedItemDefinition::from_native_policy(policy)?;
            if !item.ground_destination || item.decay.is_some() {
                return Err("source map Item needs unavailable Ground/temporal owner");
            }
            if record.attributes.blocks_movement != Some(placement.block_solid())
                || record.attributes.blocks_projectile != Some(placement.block_projectile())
                || record.attributes.immovable_block_solid
                    != Some(placement.block_solid() && !placement.movable())
            {
                return Err("source map current Item flags lack matching qualified policy");
            }
            // Canary Item::count starts at1; stackable OTBM ATTR_COUNT overrides
            // it. Nonstackable subtype attributes remain exact pending state;
            // they never become a zero quantity or guessed charge count.
            let quantity = if item.stack_maximum > 1 {
                match placement.attributes().get("15") {
                    // Canary/Crystal MapCache creates a stack with one Item and
                    // only applies a positive ATTR_COUNT; raw zero stays in the
                    // source attributes but materializes as quantity one.
                    Some(value) => u32::try_from(value.as_u64().ok_or("source stack count shape")?)
                        .map_err(|_| "source stack count overflow")?
                        .max(1),
                    None => 1,
                }
            } else {
                1
            };
            if quantity == 0 || quantity > item.stack_maximum {
                return Err("source stack count outside qualified Item limit");
            }
            let cell = placement.cell();
            let floor = i16::try_from(cell.z).map_err(|_| "source floor overflow")?;
            let target = super::world_items_execution::SpellGroundTarget::for_native_tile_read(
                room,
                runtime,
                crate::foundation::MovementLocalPosition {
                    x: cell.x,
                    y: cell.y,
                    floor,
                },
            )?;
            let source_binding =
                serde_json::to_vec(alias).map_err(|_| "source alias serialization")?;
            proof.placements.push(NativeMapItemPlacement {
                placement_key: format!(
                    "{}:cell({},{},{}):ordinal{}",
                    crate::content::native_spell_world::SOURCE_WORLD_MAP,
                    cell.x,
                    cell.y,
                    cell.z,
                    placement.ordinal()
                ),
                definition: item.definition,
                quantity,
                ground: crate::durability::item_mint::GroundPlacement {
                    spatial_position: target.spatial_position,
                    corpse_ref: b"native-source-map-init-r21".to_vec(),
                    map_revision: target.map_revision,
                    content_revision: target.content_revision,
                    native_room_placement_context: target.placement_context,
                },
                source_binding,
                source_attributes: serde_json::to_value(placement.attributes())
                    .map_err(|_| "source attributes serialization")?,
                blocks_movement: placement.block_solid(),
                blocks_projectile: placement.block_projectile(),
                immovable_block_solid: placement.block_solid() && !placement.movable(),
                owner_kind: placement.source_kind().map(str::to_owned),
            });
        }
        Ok(proof)
    }
}
