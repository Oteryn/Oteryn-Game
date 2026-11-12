//! Crystal201189f data-global/scripts/systems/hazard_primal.lua:23-81:
//! a pod is a real, immovable, scope-ephemeral Item until stepped on or hatched.
//! Source geometry remains the qualified Source world; no BundleMap, durable
//! Ground mint, ItemInstance or new wire capability is manufactured here.
use super::ComposedFreshAdmission;
use super::world_spatial::{ActorPosition, ChannelEntity, VisibleKind};
use crate::content::{CollisionClass, LogicalCell, ReferenceItemField};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition, RuntimeScopeRefV1};
use crate::map::facts::BundleFacts;
use crate::map::overlay::TilePos;
use crate::map::view::{EntryFacts, EntrySource, MapFacts};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

const POD: u16 = 39176;
const POD_KEY: &str = "oteryn:item.tibia.i39176";

/// Expected physical projection identity only. This never proves a current
/// actor, session, assignment or content activation: each producer checks its
/// actual runtime owner before using it.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Binding {
    scope: RuntimeScopeRefV1,
    generation: u64,
    server: [u8; 32],
    frame: [u8; 32],
    map: [u8; 32],
}
impl Binding {
    fn current(runtime: &ChannelRuntimeV1) -> Option<Self> {
        runtime.owner_fence().ok()?;
        let pin = runtime.content_pin();
        Some(Self {
            scope: runtime.binding().runtime_scope(),
            generation: runtime.binding().scope_generation().get(),
            server: pin.server_artifact_digest(),
            frame: pin.frame_binding_digest(),
            map: runtime.map_revision_digest(),
        })
    }
    fn identity(self, entry: u64) -> [u8; 16] {
        let mut hash = Sha256::new();
        hash.update(b"oteryn:source-volatile-item:v1");
        hash.update(self.scope.world_id().as_bytes());
        match self.scope {
            RuntimeScopeRefV1::Channel { channel_id, .. } => {
                hash.update([1]);
                hash.update(channel_id.as_bytes());
            }
            RuntimeScopeRefV1::Instance { instance_id, .. } => {
                hash.update([2]);
                hash.update(instance_id);
            }
        }
        hash.update(self.generation.to_be_bytes());
        for digest in [self.server, self.frame, self.map] {
            hash.update(digest);
        }
        hash.update(entry.to_be_bytes());
        let digest = hash.finalize();
        let mut id = [0; 16];
        id.copy_from_slice(&digest[..16]);
        id
    }
}

#[derive(Default)]
pub(super) struct SourceVolatileItems(Mutex<Option<Arc<SourceVolatileProjection>>>);

/// A retained handle owns its exact old physical projection as well as its
/// binding. Cleanup of a replaced owner can never remove the same numeric entry
/// from a new scope/generation's independent volatile facts.
pub(super) struct SourceVolatileProjection {
    binding: Binding,
    facts: BundleFacts,
}

fn tile(position: MovementLocalPosition) -> Option<TilePos> {
    Some(TilePos {
        x: u16::try_from(position.x).ok()?,
        y: u16::try_from(position.y).ok()?,
        floor: i8::try_from(position.floor.checked_neg()?).ok()?,
    })
}

impl SourceVolatileProjection {
    fn position(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        runtime: &ChannelRuntimeV1,
        position: MovementLocalPosition,
    ) -> Option<TilePos> {
        if Binding::current(runtime)? != self.binding
            || owner.source_projection_binding(runtime)? != self.binding
        {
            return None;
        }
        let cell = LogicalCell {
            x: position.x,
            y: position.y,
            // Source cells already use native_floor=-source_z (activity_template.rs:373-390).
            // TilePos below is the existing presentation intermediate, whose wire
            // encoder negates its floor; collision lookup must use runtime.floor.
            z: i32::from(position.floor),
        };
        let cells = &owner.movement_cells;
        if cells.index().lookup(cells.scope(), cell).ok()? != CollisionClass::Walkable {
            return None;
        }
        tile(position)
    }
    pub(super) fn create_pod(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        runtime: &ChannelRuntimeV1,
        position: MovementLocalPosition,
    ) -> Option<u64> {
        let pos = self.position(owner, runtime, position)?;
        if !self.facts.create_movement_item(pos, POD, 0, || true) {
            return None;
        }
        self.facts
            .movement_items(pos)
            .last()
            .and_then(|entry| match entry.source {
                EntrySource::Added(id) if entry.facts.appearance_id == POD => Some(id.get()),
                _ => None,
            })
    }
    pub(super) fn remove_pod(&self, position: MovementLocalPosition, key: u64) -> bool {
        tile(position).is_some_and(|pos| self.facts.remove_movement_item(pos, key, POD))
    }
    pub(super) fn emit_effect(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        runtime: &ChannelRuntimeV1,
        position: MovementLocalPosition,
        effect: u16,
    ) -> bool {
        self.position(owner, runtime, position)
            .is_some_and(|pos| self.facts.emit_movement_effect(pos, effect))
    }
    fn entities(&self) -> Vec<ChannelEntity> {
        self.facts
            .movement_item_entries()
            .into_iter()
            .filter_map(|(pos, entry)| {
                let EntrySource::Added(id) = entry.source else {
                    return None;
                };
                let MapDefinition::Item(item_definition_ref) = entry.facts.definition else {
                    return None;
                };
                Some(ChannelEntity {
                    display_name: None,
                    health_percent: 100,
                    direction: StepDirection::South,
                    kind: VisibleKind::GroundItem {
                        item_definition_ref,
                    },
                    identity: self.binding.identity(id.get()),
                    generation: 0,
                    position: ActorPosition {
                        x: i32::from(pos.x),
                        y: i32::from(pos.y),
                        floor: -i16::from(pos.floor),
                    },
                    revision: 1,
                })
            })
            .collect()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    fn source_projection_binding(&self, runtime: &ChannelRuntimeV1) -> Option<Binding> {
        if !matches!(
            self.movement_cells.index(),
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Source(_)
        ) {
            return None;
        }
        let binding = Binding::current(runtime)?;
        let scope = self.movement_cells.scope();
        if scope.world_id != binding.scope.world_id() || scope.generation_digest != binding.server {
            return None;
        }
        super::item_ref_admission::pinned_index(runtime, self.active_generation)?;
        Some(binding)
    }
    pub(super) fn source_volatile_projection(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Option<Arc<SourceVolatileProjection>> {
        let binding = self.source_projection_binding(runtime)?;
        let index = super::item_ref_admission::pinned_index(runtime, self.active_generation)?;
        let revision = index.revision(POD_KEY)?;
        let native = self.active_generation?.native_gameplay()?;
        let profile = native.item_policy(POD_KEY, revision)?.record();
        let ReferenceItemField::Known(physical) = &profile.semantics.physical else {
            return None;
        };
        // XML items.xml:71991 names the pod; its pinned appearances.dat39176
        // unmove/avoid flags make it immovable but walkable (the step-in owner
        // must still run). Require actual admitted native facts, never defaults.
        if physical.movable != ReferenceItemField::Known(false)
            || physical.pickupable != ReferenceItemField::Known(false)
            || profile.attributes.blocks_movement != Some(false)
            || profile.attributes.blocks_projectile != Some(false)
        {
            return None;
        }
        let facts = EntryFacts {
            definition: MapDefinition::Item(index.definition_ref("Item", POD_KEY, revision)?),
            terrain_kind: None,
            appearance_id: POD,
            blocks_projectile: false,
            pickupable: false,
            bound: true,
            count: 1,
            sub_type: 0,
        };
        let mut current = self.states.source_volatile_items.0.lock().ok()?;
        if let Some(value) = current.as_ref().filter(|value| value.binding == binding) {
            return Some(Arc::clone(value));
        }
        let value = Arc::new(SourceVolatileProjection {
            binding,
            facts: BundleFacts::for_volatile_item(facts)?,
        });
        *current = Some(Arc::clone(&value));
        Some(value)
    }
    pub(super) fn source_volatile_entities(
        &self,
        runtime: &ChannelRuntimeV1,
    ) -> Vec<ChannelEntity> {
        let Some(binding) = self.source_projection_binding(runtime) else {
            return Vec::new();
        };
        self.states
            .source_volatile_items
            .0
            .lock()
            .ok()
            .and_then(|value| {
                value
                    .as_ref()
                    .filter(|value| value.binding == binding)
                    .map(|value| value.entities())
            })
            .unwrap_or_default()
    }
    pub(super) fn source_volatile_effects(&self, after: u64) -> Vec<(u64, TilePos, u16)> {
        let Ok(runtime) = self.runtime.try_lock() else {
            return Vec::new();
        };
        let Some(binding) = self.source_projection_binding(&runtime) else {
            return Vec::new();
        };
        self.states
            .source_volatile_items
            .0
            .lock()
            .ok()
            .and_then(|value| {
                value
                    .as_ref()
                    .filter(|value| value.binding == binding)
                    .map(|value| value.facts.movement_effects_since(after))
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::super::world_spatial::{self, ChannelEntities, SessionVisibility, VisibilityUpdate};
    use super::*;
    use oteryn_protocol_oteryn::world_spatial_entities::{EntityDetail, EntityKind, EntityLights};
    use std::num::NonZeroU32;

    fn projection(generation: u64) -> Result<Arc<SourceVolatileProjection>, String> {
        let uuid = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];
        let world = crate::foundation::WorldId::decode(&uuid).map_err(|e| format!("{e:?}"))?;
        let scope = RuntimeScopeRefV1::instance(world, uuid).map_err(|e| format!("{e:?}"))?;
        let facts = BundleFacts::for_volatile_item(EntryFacts {
            definition: MapDefinition::Item(NonZeroU32::new(7).ok_or("Item reference")?),
            terrain_kind: None,
            appearance_id: POD,
            blocks_projectile: false,
            pickupable: false,
            bound: true,
            count: 1,
            sub_type: 0,
        })
        .ok_or("immovable pod projection")?;
        Ok(Arc::new(SourceVolatileProjection {
            binding: Binding {
                scope,
                generation,
                server: [3; 32],
                frame: [4; 32],
                map: [5; 32],
            },
            facts,
        }))
    }
    fn channel(projection: &SourceVolatileProjection) -> ChannelEntities {
        let observer = [9; 16];
        let mut entities = vec![ChannelEntity {
            display_name: None,
            health_percent: 100,
            direction: StepDirection::South,
            kind: VisibleKind::Player,
            identity: observer,
            generation: 1,
            position: ActorPosition {
                x: 100,
                y: 100,
                floor: -7,
            },
            revision: 1,
        }];
        entities.extend(projection.entities());
        ChannelEntities {
            content_generation: [7; 32],
            observer,
            entities,
            lights: EntityLights::new(),
            observed_at_ms: 0,
        }
    }
    fn add(projection: &SourceVolatileProjection) -> Result<u64, String> {
        let pos = TilePos {
            x: 100,
            y: 100,
            floor: 7,
        };
        if !projection.facts.create_movement_item(pos, POD, 0, || true) {
            return Err("real volatile pod creation refused".into());
        }
        projection
            .facts
            .movement_items(pos)
            .last()
            .and_then(|entry| match entry.source {
                EntrySource::Added(id) => Some(id.get()),
                _ => None,
            })
            .ok_or("physical pod identity".into())
    }

    #[test]
    fn source_pod_native_ground_item_wire_capability_and_consumption_delta() -> Result<(), String> {
        let projection = projection(1)?;
        let key = add(&projection)?;
        let mut visibility = SessionVisibility::with_objects(true);
        let mut items = super::super::item_view::SessionItemView::resume(
            super::super::item_view::ItemViewContinuity::default(),
        );
        let mut attach = |entities: &mut [world_spatial::WorldSpatialEntity]| {
            items
                .attach_spatial_handles(entities)
                .map_err(|_| world_spatial::WorldSpatialError::LimitExceeded)
        };
        let snapshot = visibility
            .snapshot(&channel(&projection), &mut attach)
            .map_err(|e| format!("{e:?}"))?;
        let (_, payload) =
            world_spatial::encode_visibility_snapshot_lit(&[4, 6], &snapshot, &EntityLights::new())
                .map_err(|e| format!("{e:?}"))?;
        let decoded =
            oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_snapshot_with_item_handles(
                &payload,
            )
            .map_err(|e| format!("{e:?}"))?;
        let pod = decoded
            .entities
            .iter()
            .find(|entry| entry.kind == EntityKind::GroundItem)
            .ok_or("native pod object missing")?;
        assert_eq!(pod.entity.generation, 0);
        let EntityDetail::Object {
            item_definition_ref,
            quantity,
            item_handle,
        } = pod.detail
        else {
            return Err("native pod must decode as an actual item object".into());
        };
        assert_eq!(item_definition_ref, 7);
        assert_eq!(quantity, 1);
        assert!(
            item_handle.is_some(),
            "native item view assigns a real session handle"
        );
        assert_eq!(
            pod.position,
            ActorPosition {
                x: 100,
                y: 100,
                floor: -7
            }
        );
        let without_items = SessionVisibility::with_objects(false)
            .snapshot(&channel(&projection), &mut attach)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            without_items.entities.len(),
            1,
            "capability4 gates the actual GroundItem object"
        );
        let position = MovementLocalPosition {
            x: 100,
            y: 100,
            floor: -7,
        };
        assert!(!projection.remove_pod(MovementLocalPosition { x: 101, ..position }, key));
        assert!(projection.remove_pod(position, key));
        let VisibilityUpdate::Delta(delta) = visibility
            .refresh(&channel(&projection), &mut attach)
            .map_err(|e| format!("{e:?}"))?
        else {
            return Err("consumed pod must leave the native view".into());
        };
        assert_eq!(delta.leave, vec![pod.entity]);
        let (_, payload) =
            world_spatial::encode_visibility_delta_lit(&[4, 6], &delta, &EntityLights::new())
                .map_err(|e| format!("{e:?}"))?;
        let decoded =
            oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_entities_delta_with_item_handles(
                &payload,
            )
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(decoded.leave, vec![pod.entity]);
        Ok(())
    }

    #[test]
    fn source_pod_retained_handle_cleanup_never_removes_replacement_projection()
    -> Result<(), String> {
        let old = projection(1)?;
        let new = projection(2)?;
        let old_key = add(&old)?;
        let new_key = add(&new)?;
        assert_eq!(
            old_key, new_key,
            "independent owners can issue the same local entry number"
        );
        assert_ne!(old.entities()[0].identity, new.entities()[0].identity);
        let position = MovementLocalPosition {
            x: 100,
            y: 100,
            floor: -7,
        };
        assert!(old.remove_pod(position, old_key));
        assert!(old.entities().is_empty());
        assert_eq!(
            new.entities().len(),
            1,
            "old retained cleanup cannot touch a new generation's real pod"
        );
        assert!(new.remove_pod(position, new_key));
        Ok(())
    }
}
