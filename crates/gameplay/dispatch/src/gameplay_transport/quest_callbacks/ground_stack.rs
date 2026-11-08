//! Read the same Channel overlay used by native movement and pickup.
use super::*;
use crate::content::item_ref::ItemDefinitionIndex;
use crate::map::{
    overlay::{AddedItem, ChannelOverlay},
    view::EntrySource,
};
use oteryn_gameplay_quests::quest::{
    ground_stack::{
        GroundStackEntry as SnapshotEntry, GroundStackRequirement, GroundStackSnapshot,
        StackItemRequirement, match_readset,
    },
    world_callbacks::{Position, WorldRequirement},
};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn ground_requirements_match(
        &self,
        runtime: &ChannelRuntimeV1,
        map: &BundleMap,
        callback: &Callback,
    ) -> bool {
        if !callback
            .world_requirements
            .iter()
            .any(|r| r.ground_stack.is_some())
        {
            return true;
        }
        let Some(ground) = self
            .states
            .loop_ground
            .current_for(runtime, map.overlay.base())
        else {
            return false;
        };
        let Some(index) =
            super::super::item_ref_admission::pinned_index(runtime, self.active_generation)
        else {
            return false;
        };
        requirements_match(
            ground.overlay(),
            &map.facts,
            &callback.world_requirements,
            index,
        )
    }
}

fn requirements_match(
    overlay: &ChannelOverlay,
    facts: &impl MapFacts,
    requirements: &[WorldRequirement],
    index: &ItemDefinitionIndex,
) -> bool {
    let mut required = Vec::new();
    let mut observed = Vec::new();
    for requirement in requirements {
        let Some(stack) = &requirement.ground_stack else {
            continue;
        };
        let Some(snapshot) = snapshot(overlay, facts, tile(requirement.position)) else {
            return false;
        };
        let mut items = Vec::with_capacity(stack.len());
        for item in stack {
            let Some(revision) = index.revision(&item.item) else {
                return false;
            };
            items.push(StackItemRequirement {
                item: item.item.clone(),
                revision: revision.into(),
                quantity: item.count,
            });
        }
        required.push(GroundStackRequirement {
            position: requirement.position,
            items,
        });
        observed.push(snapshot);
    }
    required.is_empty() || matches!(match_readset(&required, &observed), Ok(Some(_)))
}

fn snapshot(
    overlay: &ChannelOverlay,
    facts: &impl MapFacts,
    pos: TilePos,
) -> Option<GroundStackSnapshot> {
    // Base roots are distinct from portable custody. A movable base root or a
    // volatile overlay object cannot stand in for a durable Item instance.
    let view = overlay.tile(pos)?;
    let mut base = Vec::new();
    if facts.house_tile(pos)
        || crate::map::view::compose_into(overlay.base(), None, facts, pos, &mut base).is_err()
        || base.iter().any(|entry| match entry.source {
            EntrySource::Base {
                ordinal, movable, ..
            } => movable && !view.is_hidden(ordinal),
            _ => true,
        })
    {
        return None;
    }
    let mut entries = Vec::new();
    for (ordinal, entry) in view.added().iter().enumerate() {
        let AddedItem::Ground(ground) = entry.item() else {
            return None;
        };
        if ground.definition.family != "Item"
            || ground.world_id != overlay.world_id()
            || ground.runtime_scope != overlay.runtime_scope()
            || ground.item_instance_id == [0; 16]
        {
            return None;
        }
        entries.push(SnapshotEntry {
            instance: ground.item_instance_id,
            item: ground.definition.production_key.clone(),
            revision: ground.definition.revision_ref.clone(),
            quantity: ground.quantity,
            // ChannelOverlay preserves the Ground owner's append order. Its
            // internal entry ids are not the persisted sequence values.
            ordinal: u64::try_from(ordinal).ok()?.checked_add(1)?,
        });
    }
    Some(GroundStackSnapshot {
        position: Position {
            x: pos.x,
            y: pos.y,
            floor: pos.floor,
        },
        entries,
    })
}

#[cfg(test)]
fn matches(
    overlay: &ChannelOverlay,
    facts: &impl MapFacts,
    pos: TilePos,
    required: &[oteryn_gameplay_quests::quest::world_callbacks::GroundStackEntry],
) -> bool {
    let mut records: Vec<_> = required
        .iter()
        .map(|item| [&*item.item, "definition-r1"])
        .collect();
    records.sort_unstable();
    records.dedup();
    let Ok(index) = ItemDefinitionIndex::decode(
        &serde_json::to_vec(&serde_json::json!({
            "schema": "OTERYN_NATIVE_ITEM_KEYS/v1", "records": records,
        }))
        .unwrap_or_default(),
    ) else {
        return false;
    };
    requirements_match(
        overlay,
        facts,
        &[WorldRequirement {
            position: Position {
                x: pos.x,
                y: pos.y,
                floor: pos.floor,
            },
            appearance: 100,
            ground_stack: Some(required.to_vec()),
        }],
        &index,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::durability::item_mint::{GroundItemInstance, GroundPlacement, TypedDefinitionRef};
    use crate::foundation::{ChannelId, WorldId};
    use crate::map::view::EntryFacts;
    use oteryn_gameplay_quests::quest::world_callbacks::GroundStackEntry;

    struct Facts;
    impl MapFacts for Facts {
        fn base_entry(&self, _: TilePos, _: u8, _: u32) -> Option<EntryFacts> {
            Some(EntryFacts {
                definition: oteryn_protocol_oteryn::world_map::MapDefinition::Item(
                    std::num::NonZeroU32::MIN,
                ),
                terrain_kind: None,
                appearance_id: 100,
                blocks_projectile: false,
                pickupable: false,
                bound: false,
                count: 1,
                sub_type: 0,
            })
        }
        fn added_entry(&self, _: &AddedItem) -> Option<EntryFacts> {
            None
        }
        fn house_tile(&self, _: TilePos) -> bool {
            false
        }
        fn base_text(&self, _: u64) -> Option<&str> {
            None
        }
        fn object_revision(&self, _: u64) -> u64 {
            0
        }
    }

    #[test]
    fn srv7_m02_native_overlay_ground_stack_tracks_moves_and_distinct_roots()
    -> Result<(), Box<dyn std::error::Error>> {
        let (bytes, pins) = crate::map::boot::tests::bundle();
        let base = std::sync::Arc::new(crate::map::load(&bytes, &pins)?);
        let mut world_bytes = [0; 16];
        world_bytes[6] = 0x70;
        world_bytes[8] = 0x80;
        world_bytes[15] = 1;
        let mut channel_bytes = world_bytes;
        channel_bytes[15] = 2;
        let world = WorldId::decode(&world_bytes)?;
        let channel = ChannelId::decode(&channel_bytes)?;
        let mut overlay = ChannelOverlay::new(base, world, channel);
        let pos = TilePos {
            x: 2,
            y: 0,
            floor: -7,
        };
        let required = [
            GroundStackEntry {
                item: "oteryn:item.crate".into(),
                count: 1,
            },
            GroundStackEntry {
                item: "oteryn:item.amphora".into(),
                count: 1,
            },
        ];
        let ground = |id, item: &str| GroundItemInstance {
            item_instance_id: [id; 16],
            world_id: world,
            runtime_scope: overlay.runtime_scope(),
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: item.into(),
                revision_ref: "definition-r1".into(),
            },
            quantity: 1,
            runtime_scope_ownership_generation: 1,
            ground: GroundPlacement {
                spatial_position: [
                    2_i32.to_be_bytes().as_slice(),
                    0_i32.to_be_bytes().as_slice(),
                    (-7_i16).to_be_bytes().as_slice(),
                ]
                .concat(),
                corpse_ref: Vec::new(),
                map_revision: crate::map::overlay::map_revision(overlay.base()),
                content_revision: "fixture".into(),
                native_room_placement_context: Vec::new(),
            },
            minted_transaction_id: [id; 16],
        };
        let crate_item = ground(3, "oteryn:item.crate");
        let amphora = ground(4, "oteryn:item.amphora");
        assert!(!matches(&overlay, &Facts, pos, &required));
        overlay.add_ground(crate_item)?;
        assert!(!matches(&overlay, &Facts, pos, &required));
        let amphora_entry = overlay.add_ground(amphora.clone())?;
        assert!(matches(&overlay, &Facts, pos, &required));
        overlay.remove(pos, amphora_entry)?;
        assert!(!matches(&overlay, &Facts, pos, &required));
        let mut wrong_revision = amphora;
        wrong_revision.definition.revision_ref = "definition-r2".into();
        overlay.add_ground(wrong_revision)?;
        assert!(!matches(&overlay, &Facts, pos, &required));
        Ok(())
    }
}
