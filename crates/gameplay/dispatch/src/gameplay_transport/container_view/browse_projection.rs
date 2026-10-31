//! The ephemeral full-tile item projection used by browse views.

use super::browse_rules::{ItemFacts, display_order};
use crate::content::item_ref::ItemDefinitionIndex;
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ReferenceItemField, ReferenceItemSemantics};
use crate::gameplay_transport::item_view::{ItemKey, ViewItem};
use crate::map::overlay::{ChannelOverlay, TilePos};
use crate::map::view::{self, EntrySource, MapFacts};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use std::num::NonZeroU32;

/// The caller proves the actor, current session, pinned generation, tile reach and map binding.
/// Read all items before paging; the ordinary map wire's ten-entry cut is not a browse limit.
pub(in crate::gameplay_transport) fn project(
    overlay: &ChannelOverlay,
    facts: &impl MapFacts,
    native: &NativeGameplayState,
    index: &ItemDefinitionIndex,
    pos: TilePos,
    reset_epoch: u64,
) -> Option<Vec<ViewItem>> {
    let tile = view::compose_tile(overlay, facts, pos).ok()??;
    let base_tile = overlay.base().tile(pos.x, pos.y, pos.floor);
    let ground_ordinal = base_tile
        .as_ref()
        .and_then(|tile| tile.ground())
        .map(|(ordinal, _)| ordinal);
    let mut candidates = Vec::with_capacity(tile.entries.len());
    for entry in tile.entries {
        let MapDefinition::Item(reference) = entry.facts.definition else {
            continue;
        };
        let (unique, key) = match entry.source {
            EntrySource::Base {
                ordinal,
                placement_key,
                ..
            } => {
                // Crystal container.cpp:61 reads the item vector, not the terrain ground.
                if ground_ordinal == Some(ordinal) {
                    continue;
                }
                (
                    base_tile
                        .as_ref()
                        .is_some_and(|tile| tile.unique(ordinal).is_some()),
                    ItemKey::MapBase {
                        bundle_digest: overlay.base().digest(),
                        placement_key,
                        reset_epoch,
                    },
                )
            }
            EntrySource::Added(id) => (false, ItemKey::MapAdded(id.get())),
            EntrySource::Ground(id) => (false, ItemKey::Instance(id)),
        };
        if unique {
            continue;
        }
        let (key_name, revision) = index.definition_at(reference)?;
        let Some(policy) = native.item_policy(key_name, revision) else {
            // The current generation does not prove this item's browse eligibility.
            continue;
        };
        let item_facts = definition_facts(&policy.record().semantics);
        candidates.push((
            ViewItem {
                key,
                item_definition_ref: reference,
                count: NonZeroU32::new(entry.facts.count)?,
                sub_type: entry.facts.sub_type,
            },
            item_facts,
        ));
    }
    Some(
        display_order(&candidates, |(_, facts)| *facts)
            .map(|(item, _)| *item)
            .collect(),
    )
}

fn definition_facts(semantics: &ReferenceItemSemantics) -> ItemFacts {
    let movable = matches!(&semantics.physical, ReferenceItemField::Known(physical)
        if physical.movable == ReferenceItemField::Known(true));
    let container = matches!(&semantics.container, ReferenceItemField::Known(container)
        if matches!(container.capacity, ReferenceItemField::Known(_)));
    ItemFacts {
        movable,
        container,
        // Current admitted item profiles do not expose both source wrapable and BLOCKPATH.
        // A Wrap transform alone does not prove Crystal item.hpp:517-518 isWrapable().
        // Keep the unqualified furniture branch out; pickupable is not a movable fallback.
        wrappable: false,
        blocks_path: true,
        unique: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::native_gameplay::test_support::activated_with_loot_tables;
    use crate::content::{ReferenceItemContainer, ReferenceItemPhysical};
    use crate::foundation::{ChannelId, WorldId};
    use crate::map::boot::{BootPins, boot, tests::bundle_with_item_ids};
    use crate::map::facts::ItemDefinition;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn id(tag: u8) -> [u8; 16] {
        [1, 0x90, 0, 0, 0, tag, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, tag]
    }

    #[test]
    fn browse_projection_requires_movable_evidence_even_when_pickupable() {
        let semantics = ReferenceItemSemantics {
            physical: ReferenceItemField::Known(ReferenceItemPhysical {
                weight: ReferenceItemField::Unknown,
                movable: ReferenceItemField::Unknown,
                pickupable: ReferenceItemField::Known(true),
            }),
            ..ReferenceItemSemantics::default()
        };
        assert!(!super::super::browse_rules::eligible(definition_facts(
            &semantics
        )));
        // Native item admission identifies a container by a proven capacity, including zero.
        // Browse eligibility does not require that the displayed item can accept an insertion.
        for capacity in [0, 8] {
            let with_container = ReferenceItemSemantics {
                container: ReferenceItemField::Known(ReferenceItemContainer {
                    capacity: ReferenceItemField::Known(capacity),
                }),
                ..semantics.clone()
            };
            assert!(super::super::browse_rules::eligible(definition_facts(
                &with_container
            )));
        }
    }

    #[test]
    fn browse_projection_shows_full_stack_and_excludes_unique_without_hiding_other_bindings()
    -> TestResult {
        let world = WorldId::decode(&id(70)).map_err(|error| format!("{error:?}"))?;
        let channel = ChannelId::decode(&id(71)).map_err(|error| format!("{error:?}"))?;
        let (controller, _) = activated_with_loot_tables(world, None);
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native generation")?;
        let index = native.item_index().ok_or("item index")?;
        let bag = "oteryn:item.tibia.i2854";
        let coin = "oteryn:item.tibia.i3031";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, mut pins) = bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag ref")?.get() - 1),
            (coin, reference(coin).ok_or("coin ref")?.get() - 1),
        );
        let mut decoded = oteryn_world_bundle::bundle::read(&bytes)?;
        let source = decoded.sectors[0]
            .tiles
            .iter_mut()
            .find(|tile| tile.x == 7)
            .ok_or("coin tile")?;
        let coin_item = source.items.last().ok_or("coin item")?.clone();
        source.items.extend(std::iter::repeat_n(coin_item, 39));
        source.items[1].attrs.unique = Some(9001);
        source.items[2].attrs.action = Some(9002);
        source.items[3].attrs.text = Some("bound but browseable".into());
        let bytes = oteryn_world_bundle_compiler::bundle::write(
            &decoded.manifest,
            &decoded.sectors,
            &decoded.spawns,
        )?;
        pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
        let map_revision = format!(
            "sha256:{}",
            pins.digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        let map = boot(
            &bytes,
            &BootPins {
                bundle: pins,
                map_revision,
                start: TilePos {
                    x: 2,
                    y: 0,
                    floor: -7,
                },
            },
            world,
            channel,
            |key| {
                let record = native.item_policy(key, "definition-r1")?.record();
                Some(ItemDefinition {
                    reference: reference(key)?,
                    solid: record.attributes.blocks_movement,
                    blocks_projectile: record.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: key == coin,
                    door: false,
                })
            },
        )?;
        let pos = TilePos {
            x: 7,
            y: 0,
            floor: -7,
        };
        let entries = project(map.overlay(), &map.facts().pinned(), native, index, pos, 4)
            .ok_or("browse projection")?;
        assert_eq!(entries.len(), 39);
        let ordinal = |entry: &ViewItem| match entry.key {
            ItemKey::MapBase {
                placement_key,
                reset_epoch: 4,
                ..
            } => Some((placement_key & 0xFF) as u8),
            _ => None,
        };
        assert_eq!(entries.first().and_then(ordinal), Some(40));
        assert_eq!(entries.last().and_then(ordinal), Some(2));
        assert!(entries.iter().any(|entry| ordinal(entry) == Some(3)));
        let coin_reference = reference(coin).ok_or("coin ref")?;
        assert!(
            entries
                .iter()
                .all(|entry| entry.item_definition_ref == coin_reference)
        );
        let empty = project(
            map.overlay(),
            &map.facts().pinned(),
            native,
            index,
            TilePos {
                x: 2,
                y: 0,
                floor: -7,
            },
            4,
        )
        .ok_or("empty browse")?;
        assert!(empty.is_empty());
        Ok(())
    }
}
