//! Exact Channel Ground observations for ordered quest predicates.
use super::{AddedItem, ChannelOverlay, TilePos};
use crate::map::view::{EntrySource, MapFacts};
use oteryn_gameplay_quests::quest::{
    ground_stack::{GroundStackEntry as SnapshotEntry, GroundStackSnapshot, MAX_STACK_ITEMS},
    world_callbacks::Position,
};

/// Read typed Ground facts, preserving durable instance identities and ordinals.
/// The caller retains current scope/custody authority and protects the readset.
pub fn snapshot(
    overlay: &ChannelOverlay,
    facts: &impl MapFacts,
    pos: TilePos,
    expected_entries: usize,
    ordinal: &impl Fn(&[u8; 16]) -> Option<u64>,
) -> Option<GroundStackSnapshot> {
    // Base roots are distinct from portable custody. A movable base root or a
    // volatile overlay object cannot stand in for a durable Item instance.
    let view = overlay.tile(pos)?;
    if expected_entries > MAX_STACK_ITEMS || view.added().len() != expected_entries {
        return None;
    }
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
    for entry in view.added() {
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
            // Preserve the durable append ordinal retained by the custody owner.
            ordinal: ordinal(&ground.item_instance_id)?,
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
    fn m07_native_ground_snapshot_bounds_cells_before_identity_resolution()
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
        overlay.add_ground(crate_item)?;
        overlay.add_ground(amphora)?;
        let calls = std::cell::Cell::new(0);
        let ordinal = |id: &[u8; 16]| {
            calls.set(calls.get() + 1);
            Some(u64::from(id[0]))
        };
        let observed = snapshot(&overlay, &Facts, pos, required.len(), &ordinal)
            .ok_or("two typed Ground entries")?;
        assert_eq!(observed.entries.len(), 2);
        assert_eq!(observed.entries[0].instance, [3; 16]);
        assert_eq!(observed.entries[1].instance, [4; 16]);
        assert_eq!(calls.get(), 2);
        calls.set(0);
        assert!(snapshot(&overlay, &Facts, pos, 1, &ordinal).is_none());
        assert_eq!(
            calls.get(),
            0,
            "overfull cells must be rejected before resolving custody ordinals"
        );
        Ok(())
    }
}
