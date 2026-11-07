//! Physical projections join the existing Channel callback fact owner.
use super::*;

/// Apply only after the existing World owner has resolved the current admitted
/// Mage stage in its fenced physical turn. `closed` is stage4, five minutes after
/// the credited Raging Mage death. Reset boots a fresh base map.
impl BundleFacts {
    pub fn project_mage_tower_entry(&self, base: &super::super::WorldBase, closed: bool) -> bool {
        let at = TilePos {
            x: 33143,
            y: 31527,
            floor: -2,
        };
        if self.house_tile(at) {
            return false;
        }
        let Some(tile) = base.tile(at.x, at.y, at.floor) else {
            return false;
        };
        // Retained native B3: 12379 ground followed by10840 at top-level ordinal1.
        // The donor startup table assigns UID13405, but this B3 has no such attr.
        let ordinal = 1;
        let Some((&id, _)) = tile
            .ids()
            .iter()
            .zip(tile.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(usize::from(ordinal))
        else {
            return false;
        };
        let Some(key) = placement_key(at, ordinal) else {
            return false;
        };
        let Some(facts) = self.base_entry(at, ordinal, id) else {
            return false;
        };
        if facts.appearance_id != 10840 {
            return false;
        }
        if self.base_hidden(key) == closed {
            return true;
        }
        // Existing callback owner atomically pairs visibility with this exact
        // source revision. It never changes the10842 exit's independent key.
        self.apply_quest_callback(
            key,
            self.object_revision(key),
            facts,
            &[key],
            &[],
            !closed,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;

    #[test]
    fn worldchg_mage_phase_hides_only_native_entry_and_restores_it() {
        use oteryn_world_bundle::{bundle, sector};
        let (bytes, mut pins) = crate::map::boot::tests::bundle_with_items(
            "oteryn:item.tibia.i10840",
            "oteryn:item.tibia.i10842",
        );
        let mut manifest = bundle::read(&bytes).expect("fixture").manifest;
        manifest.world = bundle::Extent {
            min_x: 32942,
            min_y: 31527,
            max_x: 33144,
            max_y: 31593,
            floors: vec![-8, -2],
        };
        let item = |palette| sector::Item {
            palette,
            depth: 0,
            attrs: Default::default(),
        };
        let mut sectors = vec![
            bundle::Sector {
                floor: -2,
                sx: 33143 / 32,
                sy: 31527 / 32,
                tiles: vec![sector::Tile {
                    x: 33143,
                    y: 31527,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items: vec![item(0), item(3)],
                }],
            },
            bundle::Sector {
                floor: -8,
                sx: 32942 / 32,
                sy: 31592 / 32,
                tiles: vec![sector::Tile {
                    x: 32942,
                    y: 31592,
                    flags: 0,
                    house: 0,
                    zones: vec![],
                    items: vec![item(0), item(0), item(4)],
                }],
            },
        ];
        sectors.sort_by_key(bundle::order);
        let bytes = oteryn_world_bundle::writer::write(&manifest, &sectors, &Default::default())
            .expect("written");
        pins.digest = bundle::read(&bytes).expect("read").digest;
        let base = crate::map::load(&bytes, &pins).expect("base");
        let id = |label| {
            let mut bytes = [label; 16];
            bytes[6] = 0x70;
            bytes[8] = 0x80;
            bytes
        };
        let world = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: TilePos {
                    x: 33143,
                    y: 31527,
                    floor: -2,
                },
            },
            crate::foundation::WorldId::decode(&id(42)).expect("World"),
            crate::foundation::ChannelId::decode(&id(43)).expect("Channel"),
            |key| {
                Some(ItemDefinition {
                    reference: NonZeroU32::new(match key {
                        "oteryn:item.tibia.i10840" => 1,
                        "oteryn:item.tibia.i10842" => 2,
                        _ => return None,
                    })?,
                    solid: Some(false),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            },
        )
        .expect("boot");
        let entry = placement_key(
            TilePos {
                x: 33143,
                y: 31527,
                floor: -2,
            },
            1,
        )
        .expect("entry");
        let exit = placement_key(
            TilePos {
                x: 32942,
                y: 31592,
                floor: -8,
            },
            2,
        )
        .expect("exit");
        let facts = world.facts();
        assert!(!facts.base_hidden(entry));
        assert!(facts.project_mage_tower_entry(world.base(), true));
        assert!(facts.base_hidden(entry));
        assert!(
            !facts.base_hidden(exit),
            "10842 escape always survives tower closure"
        );
        let revision = facts.object_revision(entry);
        assert!(facts.project_mage_tower_entry(world.base(), true));
        assert_eq!(
            facts.object_revision(entry),
            revision,
            "repeated projection is idempotent"
        );
        assert!(facts.project_mage_tower_entry(world.base(), false));
        assert!(!facts.base_hidden(entry));
        assert!(!facts.base_hidden(exit));
        assert!(facts.object_revision(entry) > revision);
    }
}
