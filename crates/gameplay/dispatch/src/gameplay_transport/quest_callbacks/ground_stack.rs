//! Read the same Channel overlay used by native movement and pickup.
use super::*;
use crate::content::item_ref::ItemDefinitionIndex;
#[cfg(test)]
use crate::map::overlay::AddedItem;
use crate::map::overlay::{ChannelOverlay, quest_ground::snapshot};
#[cfg(test)]
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_gameplay_quests::quest::{
    ground_stack::{
        GroundStackRequirement, GroundStackSnapshot, StackItemRequirement, match_readset,
    },
    world_callbacks::{WorldRequirement, ground::GroundStackObserved},
};

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Advice for a fresh ambiguous USE, never a replacement for either durable
    /// Ground participant. All candidates see one immutable, currently pinned read.
    pub(super) async fn retain_ground_candidates(
        &self,
        permit: &SpellLanePermit,
        map: &BundleMap,
        candidates: &mut Vec<&Callback>,
    ) -> bool {
        let index = {
            let runtime = self.runtime.lock().await;
            if permit
                .check_scope(runtime.binding().runtime_scope())
                .is_err()
            {
                return false;
            }
            let Some(index) =
                super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)
            else {
                return false;
            };
            index
        };
        // Only the lane is retained across the bounded owner read, not a runtime
        // or spell-state guard. The caller keeps that lane through final admission.
        let Some(ground) =
            super::super::loop_move::loop_ground(self, map.overlay.base(), Some(index)).await
        else {
            return false;
        };
        let runtime = self.runtime.lock().await;
        if permit
            .check_scope(runtime.binding().runtime_scope())
            .is_err()
            || !self
                .states
                .loop_ground
                .current_for(&runtime, map.overlay.base())
                .is_some_and(|current| std::sync::Arc::ptr_eq(&current, &ground))
        {
            return false;
        }
        let Some(index) =
            super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)
        else {
            return false;
        };
        candidates.retain(|callback| {
            advisory_requirements_match(&callback.world_requirements, index, &|position, count| {
                snapshot(ground.overlay(), &map.facts, tile(position), count, &|id| {
                    ground.ground_ordinal(id)
                })
            })
        });
        true
    }

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
            &|id| ground.ground_ordinal(id),
        )
    }
}

fn requirements_match(
    overlay: &ChannelOverlay,
    facts: &impl MapFacts,
    requirements: &[WorldRequirement],
    index: &ItemDefinitionIndex,
    ordinal: &impl Fn(&[u8; 16]) -> Option<u64>,
) -> bool {
    ordered_requirements_match(requirements, index, &|position, count| {
        snapshot(overlay, facts, tile(position), count, ordinal)
    })
}

fn ordered_requirements_match(
    requirements: &[WorldRequirement],
    index: &ItemDefinitionIndex,
    observe: &impl Fn(Position, usize) -> Option<GroundStackSnapshot>,
) -> bool {
    let mut required = Vec::new();
    let mut observed = Vec::new();
    for requirement in requirements {
        let Some(stack) = &requirement.ground_stack else {
            continue;
        };
        let Some(snapshot) = observe(requirement.position, stack.len()) else {
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

fn advisory_requirements_match(
    requirements: &[WorldRequirement],
    index: &ItemDefinitionIndex,
    observe: &impl Fn(Position, usize) -> Option<GroundStackSnapshot>,
) -> bool {
    ordered_requirements_match(requirements, index, observe)
        && requirements.iter().all(|requirement| {
            let Some(required) = &requirement.ground else {
                return true;
            };
            let Some(observed) = observe(requirement.position, required.stacks.len()) else {
                return false;
            };
            if observed.position != requirement.position {
                return false;
            }
            let observed: Vec<_> = observed
                .entries
                .iter()
                .map(|entry| GroundStackObserved {
                    instance: entry.instance,
                    item: &entry.item,
                    revision: &entry.revision,
                    quantity: u64::from(entry.quantity),
                    ordinal: Some(entry.ordinal),
                })
                .collect();
            required.matches(&observed)
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
            ground: None,
            position: Position {
                x: pos.x,
                y: pos.y,
                floor: pos.floor,
            },
            appearance: 100,
            ground_stack: Some(required.to_vec()),
        }],
        &index,
        &|id| Some(u64::from(id[0])),
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
    fn nomads_advice_keeps_both_actual_ground_contracts_and_requires_unique_selection()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_gameplay_quests::quest::ground_stack::GroundStackEntry as ObservedEntry;
        let loaded = world_callbacks::load()?;
        let callbacks = [
            loaded
                .iter()
                .find(|c| c.key == "oteryn:quest-callback/engenc2/nomads-land-window")
                .ok_or("personal window")?,
            loaded
                .iter()
                .find(|c| c.key == "oteryn:quest-callback/m02/nomads-land/window")
                .ok_or("stateless window")?,
        ];
        for callback in callbacks {
            let native = callback.selector.native.as_ref().ok_or("native window")?;
            assert_eq!(
                native.position,
                Position {
                    x: 33216,
                    y: 32855,
                    floor: -9
                }
            );
            assert_eq!((native.ordinal, native.appearance), (1, 1744));
        }
        let tracks = std::collections::BTreeMap::new();
        assert!(
            unique_eligible(callbacks, &tracks).is_none(),
            "two eligible callbacks refuse without physical advice"
        );
        assert!(unique_eligible(std::iter::empty(), &tracks).is_none());
        let index = ItemDefinitionIndex::decode(&serde_json::to_vec(&serde_json::json!({
            "schema": "OTERYN_NATIVE_ITEM_KEYS/v1",
            "records": [
                ["oteryn:item.tibia.i2471", "definition-r1"],
                ["oteryn:item.tibia.i2904", "definition-r1"]
            ]
        }))?)?;
        let layout = |cells: &[(u16, &[u32])]| {
            let mut id = 0_u8;
            cells
                .iter()
                .map(|(x, items)| GroundStackSnapshot {
                    position: Position {
                        x: *x,
                        y: 32854,
                        floor: -9,
                    },
                    entries: items
                        .iter()
                        .map(|item| {
                            id += 1;
                            ObservedEntry {
                                instance: [id; 16],
                                item: format!("oteryn:item.tibia.i{item}"),
                                revision: "definition-r1".into(),
                                quantity: 1,
                                ordinal: u64::from(id),
                            }
                        })
                        .collect(),
                })
                .collect::<Vec<_>>()
        };
        let eligible = |snapshots: &[GroundStackSnapshot]| {
            callbacks
                .into_iter()
                .filter(|callback| {
                    advisory_requirements_match(
                        &callback.world_requirements,
                        &index,
                        &|position, count| {
                            snapshots
                                .iter()
                                .find(|snapshot| {
                                    snapshot.position == position && snapshot.entries.len() == count
                                })
                                .cloned()
                        },
                    )
                })
                .collect::<Vec<_>>()
        };
        let personal = layout(&[(33215, &[2471]), (33216, &[2471, 2904]), (33217, &[2471])]);
        let stateless = layout(&[(33215, &[2471, 2471, 2904]), (33216, &[2471, 2471, 2904])]);
        assert_eq!(
            unique_eligible(eligible(&personal), &tracks).map(|c| c.key.as_str()),
            Some(callbacks[0].key.as_str())
        );
        assert_eq!(
            unique_eligible(eligible(&stateless), &tracks).map(|c| c.key.as_str()),
            Some(callbacks[1].key.as_str())
        );
        assert!(eligible(&[]).is_empty());
        assert!(
            eligible(&personal[..2]).is_empty(),
            "missing third personal pile"
        );
        for original in [&personal, &stateless] {
            let mut wrong_order = (*original).clone();
            let last = wrong_order[1].entries.len() - 1;
            wrong_order[1].entries.swap(0, last);
            for (offset, entry) in wrong_order[1].entries.iter_mut().enumerate() {
                entry.ordinal = u64::try_from(offset)? + 10;
            }
            assert!(eligible(&wrong_order).is_empty());
            let mut wrong_revision = (*original).clone();
            wrong_revision[0].entries[0].revision = "definition-r2".into();
            assert!(eligible(&wrong_revision).is_empty());
            let mut wrong_quantity = (*original).clone();
            wrong_quantity[0].entries[0].quantity = 2;
            assert!(eligible(&wrong_quantity).is_empty());
            let mut extra = (*original).clone();
            let mut item = extra[0].entries[0].clone();
            item.instance = [99; 16];
            item.ordinal = 99;
            extra[0].entries.push(item);
            assert!(eligible(&extra).is_empty());
        }
        let mut duplicate = stateless.clone();
        duplicate[1].entries[0].instance = duplicate[0].entries[0].instance;
        assert!(
            eligible(&duplicate).is_empty(),
            "distinct tiles cannot reuse a custody root"
        );
        let mut retired = personal;
        retired[1].entries.pop();
        assert!(
            eligible(&retired).is_empty(),
            "a later observation cannot reuse the accepted layout"
        );
        Ok(())
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
