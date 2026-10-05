//! Native runtime/Content adapter for the actual shared durable Item owner.
use crate::durability::item_mint::{GroundPlacement, TypedDefinitionRef};
pub(crate) use crate::durability::spell_items_abi::*;
impl SpellGroundTarget {
    /// Read-only address for an independently qualified captured cell, including
    /// known source air. This does not prove a placed Ground or mint eligibility.
    pub(crate) fn for_native_tile_read(
        room: &crate::content::QualifiedNativeEntryRoom,
        runtime: &crate::foundation::ChannelRuntimeV1,
        position: crate::foundation::MovementLocalPosition,
    ) -> Result<Self, &'static str> {
        let pin = runtime.content_pin();
        if room.movement_cells().scope().world_id != pin.world_id()
            || room.frame_binding().digest() != pin.frame_binding_digest()
            || room.map_revision_digest() != pin.map_revision_digest()
            || room.compiled().server_digest() != pin.server_artifact_digest()
        {
            return Err("native tile read generation mismatch");
        }
        let cell = crate::content::LogicalCell {
            x: position.x,
            y: position.y,
            z: i32::from(position.floor),
        };
        if let Some(source) = room.source_world() {
            if !source.contains_capture_cell(cell) {
                return Err("tile read is outside qualified source capture");
            }
        } else {
            room.movement_cells()
                .index()
                .lookup(room.movement_cells().scope(), cell)
                .map_err(|_| "tile read is outside qualified placed room cells")?;
        }
        let hex = |bytes: [u8; 32]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let mut spatial_position = Vec::with_capacity(10);
        spatial_position.extend_from_slice(&position.x.to_be_bytes());
        spatial_position.extend_from_slice(&position.y.to_be_bytes());
        spatial_position.extend_from_slice(&position.floor.to_be_bytes());
        Ok(Self {
            spatial_position,
            map_revision: format!("sha256:{}", hex(room.map_revision_digest())),
            content_revision: format!("sha256:{}", hex(pin.server_artifact_digest())),
            placement_context: room.frame_binding().digest().to_vec(),
        })
    }
    /// Reuses the actual combat corpse-placement address codec: signed i32 x,
    /// signed i32 y, signed i16 floor, all big endian. Qualification binds the
    /// native room to this runtime's full activated frame/map/server pin.
    pub(crate) fn from_native_owner(
        room: &crate::content::QualifiedNativeEntryRoom,
        runtime: &crate::foundation::ChannelRuntimeV1,
        position: crate::foundation::MovementLocalPosition,
    ) -> Result<Self, &'static str> {
        let pin = runtime.content_pin();
        let scope = room.movement_cells().scope();
        if scope.world_id != pin.world_id()
            || room.frame_binding().digest() != pin.frame_binding_digest()
            || room.map_revision_digest() != pin.map_revision_digest()
            || room.compiled().server_digest() != pin.server_artifact_digest()
        {
            return Err("native Ground address generation mismatch");
        }
        room.movement_cells()
            .index()
            .lookup(
                scope,
                crate::content::LogicalCell {
                    x: position.x,
                    y: position.y,
                    z: i32::from(position.floor),
                },
            )
            .map_err(|_| "native Ground address is not a qualified placed cell")?;
        let hex =
            |value: [u8; 32]| -> String { value.iter().map(|b| format!("{b:02x}")).collect() };
        let mut spatial_position = Vec::with_capacity(10);
        spatial_position.extend_from_slice(&position.x.to_be_bytes());
        spatial_position.extend_from_slice(&position.y.to_be_bytes());
        spatial_position.extend_from_slice(&position.floor.to_be_bytes());
        Ok(Self {
            spatial_position,
            map_revision: format!("sha256:{}", hex(room.map_revision_digest())),
            content_revision: format!("sha256:{}", hex(pin.server_artifact_digest())),
            placement_context: room.frame_binding().digest().to_vec(),
        })
    }
    pub(crate) fn placement(&self, source_reference: Vec<u8>) -> GroundPlacement {
        GroundPlacement {
            spatial_position: self.spatial_position.clone(),
            corpse_ref: source_reference,
            map_revision: self.map_revision.clone(),
            content_revision: self.content_revision.clone(),
            native_room_placement_context: self.placement_context.clone(),
        }
    }
}
impl QualifiedItemDefinition {
    pub(crate) fn from_native_policy(
        policy: crate::content::native_gameplay::QualifiedItemPolicy<'_>,
    ) -> Result<Self, &'static str> {
        use crate::content::ReferenceItemField as F;
        use crate::content::native_gameplay::{
            NativeItemDestination as D, NativeItemStackClass as S,
        };
        let record = policy.record();
        let item = &record.authoring.item;
        if item.family != crate::content::ProjectV2Family::Item
            || !record.admission.materializable
            || policy.source_digest() == [0; 32]
        {
            return Err("native member is not an admitted materializable Item");
        }
        let movable = match &record.semantics.physical {
            F::Known(physical) => match physical.movable {
                F::Known(value) => value,
                _ => return Err("unknown native Item movable capability"),
            },
            _ => return Err("unknown native Item physical capabilities"),
        };
        let stackable = match &record.semantics.stack {
            F::Known(stack) => match stack.stackable {
                F::Known(value) => value,
                _ => return Err("unknown native Item stack class"),
            },
            _ => return Err("unknown native Item stack capabilities"),
        };
        if stackable != (record.admission.stack_class == S::StackCapable) {
            return Err("native Item stack admission conflicts with source");
        }
        let stack_maximum = match record.admission.stack_class {
            S::NonStackable => 1,
            S::StackCapable => match &record.semantics.stack {
                F::Known(stack) => match stack.stack_max {
                    F::Known(value) if value > 0 => u32::from(value),
                    _ => return Err("unknown native Item stack maximum"),
                },
                _ => return Err("unknown native Item stack capabilities"),
            },
        };
        let decay = match &record.semantics.temporal {
            F::NotApplicable => None,
            F::Known(temporal) => {
                let F::Known(crate::content::ReferenceTemporalMode::DurableAbsoluteDeadline) =
                    temporal.consumption_mode
                else {
                    return Err("native Item lifetime lacks an absolute owner policy");
                };
                let F::Known(crate::content::ReferenceMilliseconds(duration)) = temporal.duration
                else {
                    return Err("unknown native Item duration");
                };
                if !matches!(temporal.stop_duration, F::Known(false)) {
                    return Err("native Item pausable lifetime requires its time-budget owner");
                }
                let target = match &temporal.decay_target {
                    F::NotApplicable => None,
                    F::Known(target) => Some(TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: target.key.clone(),
                        revision_ref: target.revision.clone(),
                    }),
                    _ => return Err("unknown native Item decay target"),
                };
                if duration == 0 {
                    if target.is_some() {
                        return Err("zero-duration Item transform");
                    }
                    None
                } else {
                    if stack_maximum != 1 {
                        return Err("stacked Item lifetime needs split-owner semantics");
                    }
                    Some(QualifiedItemDecay {
                        duration_millis: u32::try_from(duration)
                            .map_err(|_| "Item duration overflow")?,
                        target,
                    })
                }
            }
            _ => return Err("unknown native Item temporal capability"),
        };
        Ok(Self {
            definition: crate::durability::item_mint::TypedDefinitionRef {
                family: record.production_definition.family.clone(),
                production_key: record.production_definition.production_key.clone(),
                revision_ref: record.production_definition.revision_ref.clone(),
            },
            content_generation_digest: policy.source_digest(),
            materializable: true,
            movable,
            stack_maximum,
            container_capacity: match &record.semantics.container {
                F::Known(container) => match container.capacity {
                    F::Known(value) => Some(value),
                    _ => None,
                },
                _ => None,
            },
            inventory_destination: record
                .admission
                .legal_destinations
                .contains(&D::CharacterInventory),
            ground_destination: record.admission.legal_destinations.contains(&D::Ground),
            decay,
            decay_chain: Vec::new(),
        })
    }

    /// Qualification consumes the typed Content definition, not appearance
    /// membership or an inferred capability. The compositor additionally binds
    /// this digest to the currently compatible generation before writer use.
    pub(crate) fn from_content(
        definition: TypedDefinitionRef,
        digest: [u8; 32],
        item: &crate::content::ReferenceItemDefinition,
    ) -> Result<Self, &'static str> {
        use crate::content::{ReferenceItemField as F, ReferenceItemStackClass as S};
        crate::content::validate_item_definition(item).map_err(|_| "invalid Item Content")?;
        if definition.family != "Item" || !item.materializable || digest == [0; 32] {
            return Err("unqualified Item Content");
        }
        let movable = match &item.semantics.physical {
            F::Known(p) => match &p.movable {
                F::Known(value) => *value,
                _ => return Err("unknown movable capability"),
            },
            _ => return Err("unknown movable capability"),
        };
        let stack_maximum = match item.stack_class {
            S::NonStackable => 1,
            S::StackCapable => match &item.semantics.stack {
                F::Known(s) => match &s.stack_max {
                    F::Known(value) if *value > 0 => u32::from(*value),
                    _ => return Err("unknown stack maximum"),
                },
                _ => return Err("unknown stack maximum"),
            },
            S::Unknown => return Err("unknown stack class"),
        };
        Ok(Self {
            definition,
            content_generation_digest: digest,
            materializable: true,
            movable,
            stack_maximum,
            container_capacity: match &item.semantics.container {
                F::Known(c) => match c.capacity {
                    F::Known(value) => Some(value),
                    _ => None,
                },
                _ => None,
            },
            inventory_destination: item
                .legal_destinations
                .contains(&crate::content::ReferenceItemDestination::CharacterInventory),
            // The accepted Reference slice grants inventory custody only.
            // The native candidate has a separate explicit Ground policy.
            ground_destination: false,
            decay: None,
            decay_chain: Vec::new(),
        })
    }
}
impl DurableCorpseReservation {
    pub(crate) fn cell(&self) -> Result<crate::content::LogicalCell, &'static str> {
        decode_ground_cell(&self.placement.spatial_position)
    }
}
pub(crate) fn decode_ground_cell(
    bytes: &[u8],
) -> Result<crate::content::LogicalCell, &'static str> {
    let cell = crate::durability::spell_items_abi::decode_ground_cell(bytes)?;
    Ok(crate::content::LogicalCell {
        x: cell.x,
        y: cell.y,
        z: cell.z,
    })
}

impl QualifiedRuneDefinition {
    /// The active importer has established Rune classification and its exact
    /// production alias. Numeric carrier IDs are compared only with that alias;
    /// they never construct an Item DefinitionRef.
    pub(crate) fn from_native_policy(
        policy: crate::content::native_gameplay::QualifiedItemPolicy<'_>,
        carrier_item: u32,
        carrier_charges: u32,
    ) -> Result<Self, &'static str> {
        use crate::content::{ReferenceItemField as F, ReferenceItemType};
        let record = policy.record();
        let F::Known(classification) = &record.semantics.classification else {
            return Err("unknown Rune classification");
        };
        if record.attributes.rune_consumption
            != Some(crate::content::native_gameplay::NativeRuneConsumption::ItemCountOne)
        {
            return Err("Rune ItemCount consumption source policy unavailable");
        }
        if classification.item_type != F::Known(ReferenceItemType::Rune)
            || carrier_item == 0
            || carrier_charges == 0
            || record.production_binding.external_id != carrier_item.to_string()
        {
            return Err("Rune carrier/active production binding mismatch");
        }
        let item = QualifiedItemDefinition::from_native_policy(policy)?;
        if !item.movable
            || !item.inventory_destination
            || item.container_capacity.is_some()
            || item.stack_maximum <= 1
            || item.decay.is_some()
        {
            return Err("Rune ItemCount source policy unsupported");
        }
        Ok(Self {
            item,
            source_item_id: carrier_item,
        })
    }
}
