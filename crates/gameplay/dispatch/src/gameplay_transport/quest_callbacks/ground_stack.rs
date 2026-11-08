//! Read the same Channel overlay used by native movement and pickup.
use super::*;
use crate::map::{
    overlay::{AddedItem, ChannelOverlay},
    view::EntrySource,
};
use oteryn_gameplay_quests::quest::ground_stack::GroundStackEntry as NativeGroundStackFact;
use oteryn_gameplay_quests::quest::world_callbacks::{GroundStackEntry, ground_stack_matches_at};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn ground_requirements_match(
        &self,
        runtime: &ChannelRuntimeV1,
        map: &BundleMap,
        callback: &Callback,
    ) -> bool {
        let mut required = callback
            .world_requirements
            .iter()
            .filter(|r| r.ground_stack.is_some());
        if required.clone().next().is_none() {
            return true;
        }
        let Some(ground) = self
            .states
            .loop_ground
            .current_for(runtime, map.overlay.base())
        else {
            return false;
        };
        required.all(|r| {
            r.ground_stack.as_ref().is_some_and(|stack| {
                matches(
                    ground.overlay(),
                    &map.facts,
                    tile(r.position),
                    stack,
                    |id| ground.ground_ordinal(id),
                )
            })
        })
    }
}

pub(super) fn matches(
    overlay: &ChannelOverlay,
    facts: &impl MapFacts,
    pos: TilePos,
    required: &[GroundStackEntry],
    ordinal: impl Fn(&[u8; 16]) -> Option<u64>,
) -> bool {
    // Read base roots separately: BundleFacts intentionally cannot reconstruct
    // portable Item facts from an appearance. Ground records below already carry
    // their canonical definitions from the custody owner.
    let Some(view) = overlay.tile(pos) else {
        return false;
    };
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
        return false;
    }
    ground_stack_matches_at(
        Position {
            x: pos.x,
            y: pos.y,
            floor: pos.floor,
        },
        required,
        view.added().iter().map(|entry| {
            let AddedItem::Ground(ground) = entry.item() else {
                return None;
            };
            (ground.definition.family == "Item"
                && ground.world_id == overlay.world_id()
                && ground.runtime_scope == overlay.runtime_scope())
            .then_some(NativeGroundStackFact {
                instance: ground.item_instance_id,
                item: ground.definition.production_key.clone(),
                revision: ground.definition.revision_ref.clone(),
                quantity: ground.quantity,
                ordinal: ordinal(&ground.item_instance_id)?,
            })
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::durability::item_mint::{GroundItemInstance, GroundPlacement, TypedDefinitionRef};
    use crate::foundation::{ChannelId, WorldId};
    use crate::map::view::EntryFacts;

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
        assert!(!matches(&overlay, &Facts, pos, &required, |id| Some(
            u64::from(id[0])
        )));
        overlay.add_ground(crate_item)?;
        assert!(!matches(&overlay, &Facts, pos, &required, |id| Some(
            u64::from(id[0])
        )));
        let amphora_entry = overlay.add_ground(amphora)?;
        assert!(matches(&overlay, &Facts, pos, &required, |id| Some(
            u64::from(id[0])
        )));
        overlay.remove(pos, amphora_entry)?;
        assert!(!matches(&overlay, &Facts, pos, &required, |id| Some(
            u64::from(id[0])
        )));
        Ok(())
    }
}
