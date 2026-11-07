//! W05b/WORLDCHG-Q: apply the reset owner's committed daily decision to the live map.
//!
//! World/event durability owns the decision; this module only projects its admitted effects.

use super::bundle_npcs::NpcBoot;
use super::event;
use crate::foundation::{ChannelRuntimeV1, MovementFacing, NpcPlacement, WorldId};
use crate::map::overlay::TilePos;
use oteryn_gameplay_world_change::daily::DailyWorldChanges;
use sha2::{Digest, Sha256};

#[cfg(test)]
use crate::durability::world_event::changes::utc_month_day;

/// Qualify the boot's optional composition against its resolved reset epoch.
pub(super) fn bind_layers(
    checked: crate::map::boot::CheckedBundle,
    changes: &DailyWorldChanges,
) -> Result<crate::map::boot::CheckedBundle, crate::map::boot::BootRefusal> {
    // A fresh World without an admitted reset retains its ordinary base only.
    if changes.reset_epoch == 0 {
        return Ok(checked);
    }
    checked.compose_epoch_admitted_layers(
        changes.reset_epoch,
        &changes.layers(),
        &changes.exit_layers(),
    )
}

/// The physical draws bind World and committed epoch, never Channel or process lifetime.
fn map_changes(world: WorldId, changes: &DailyWorldChanges) -> crate::map::daily::DailyMap {
    let mut hash = Sha256::new();
    hash.update(b"oteryn.world_change.physical.seed.v1\0");
    hash.update(world.as_bytes());
    hash.update(changes.reset_epoch.to_be_bytes());
    crate::map::daily::DailyMap {
        seed: hash.finalize().into(),
        fungus: changes.their_masters_voice,
        grimvale_full_moon: changes.grimvale_full_moon,
        trader: changes.oriental_trader.map(|town| town.layer()),
        variants: changes.variants,
    }
}

pub(super) fn prepare_map(
    checked: crate::map::boot::CheckedBundle,
    world: WorldId,
    changes: &DailyWorldChanges,
) -> crate::map::boot::CheckedBundle {
    checked.with_daily_map(map_changes(world, changes))
}

/// Places the day's NPCs the loaded catalogue admits; one that cannot stand is logged.
pub(super) fn apply(
    runtime: &mut ChannelRuntimeV1,
    npc_boot: Option<&NpcBoot>,
    changes: &DailyWorldChanges,
    map: Option<&crate::map::boot::BundleWorld>,
) -> usize {
    if changes.reset_epoch == 0 {
        return 0;
    }
    let mut placed = 0;
    let trader = changes
        .oriental_trader
        .filter(|town| {
            map.is_some_and(|map| {
                let (x, y, z) = town.yasir_position();
                map.enterable(crate::map::overlay::TilePos { x, y, floor: -z })
            })
        })
        .map(|town| ("oteryn:npc.yasir", town.yasir_position()));
    use oteryn_gameplay_world_change::{counter::CounterKind as K, owner::counter_admitted};
    // Retained NPC anchors for Orockle/Benevola; adjacent admitted Navigator tile
    // and the audited Seacrest Guardian are explicit native geometry choices.
    let shared_npcs = [
        (K::HiveBorn, "oteryn:npc.orockle", (33448, 31309, 7)),
        (K::Overhunting, "oteryn:npc.benevola", (32596, 31745, 7)),
        (K::Deeplings, "oteryn:npc.navigator", (33638, 31377, 9)),
        // Explicit native barter station, not an asserted donor Yeo coordinate.
        (K::FireFeathered, "oteryn:npc.yeo", (33554, 31624, 14)),
        (
            K::FireFeathered,
            "oteryn:npc.guardian_of_the_deep_seas",
            (33553, 31624, 14),
        ),
    ]
    .into_iter()
    .filter(|(kind, _, _)| changes.world_counters && counter_admitted(*kind))
    .map(|(_, key, at)| (key, at));
    for (key, (x, y, z)) in changes.npcs().chain(trader).chain(shared_npcs) {
        if !npc_boot.is_some_and(|boot| boot.admits(key)) {
            continue;
        }
        let at = NpcPlacement {
            floor: -z,
            y,
            x,
            direction: MovementFacing::South,
        };
        match runtime.place_dynamic_npc(key, at) {
            Ok(_) => placed += 1,
            Err(error) => event(&format!(
                "event=daily_world_change_npc state=refused npc_key={key} reason={error:?}"
            )),
        }
    }
    placed
}

/// Places the day's Items on the bundle map's callback layer; one that cannot be placed is
/// logged. A World without a bundle map places nothing.
pub(super) fn apply_items(
    map: Option<&crate::map::facts::BundleFacts>,
    changes: &DailyWorldChanges,
) -> usize {
    if changes.reset_epoch == 0 {
        return 0;
    }
    let Some(map) = map else {
        return 0;
    };
    let mut placed = 0;
    for (appearance, (x, y, z)) in changes.items() {
        let pos = TilePos { x, y, floor: -z };
        if map.create_world_change_item(pos, appearance) {
            placed += 1;
        } else {
            event(&format!(
                "event=daily_world_change_item state=refused appearance={appearance} x={x} y={y} z={z}"
            ));
        }
    }
    placed
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures")]
mod tests {
    use super::*;
    use crate::foundation::{ChannelId, NodeId, NpcPlacementTable};
    use crate::gameplay_transport::NpcTalkBook;
    use std::sync::Arc;

    fn roll(world: WorldId, reset_epoch: u64, unix_ms: i64) -> DailyWorldChanges {
        crate::durability::world_event::changes::roll(world, reset_epoch, unix_ms)
            .expect("authored daily candidates")
    }

    fn world() -> WorldId {
        WorldId::decode(&[1, 0, 0, 0, 0, 1, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 1]).expect("world")
    }

    fn channel_runtime() -> ChannelRuntimeV1 {
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
            .expect("channel");
        let node =
            NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node");
        ChannelRuntimeV1::from_committed_assignment(
            world(),
            channel,
            node,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::test(world()),
        )
        .expect("runtime")
    }

    fn boot(keys: &[&str]) -> NpcBoot {
        NpcBoot::Place {
            table: NpcPlacementTable::default(),
            catalogue_sha256: String::new(),
            admitted: keys.iter().map(|key| (*key).to_owned()).collect(),
            production: true,
            talk: Arc::new(NpcTalkBook::default()),
        }
    }

    fn npc_keys(runtime: &ChannelRuntimeV1) -> Vec<String> {
        runtime
            .visible_entities()
            .npcs
            .iter()
            .filter_map(|npc| runtime.npc_key(npc.actor).map(str::to_owned))
            .collect()
    }

    /// 2031-06-13T12:00:00Z, 2031-07-01T00:00:00Z and 2031-05-31T23:59:59.999Z.
    const JUNE_13: i64 = 1_939_118_400_000;
    const JULY_1: i64 = 1_940_630_400_000;
    const MAY_31_LAST: i64 = 1_938_038_399_999;

    #[test]
    fn world_src_zero_epoch_boot_keeps_base_without_daily_grants()
    -> Result<(), Box<dyn std::error::Error>> {
        let (bytes, pins) = crate::map::boot::tests::bundle();
        let boot_pins = crate::map::boot::BootPins {
            map_revision: format!(
                "sha256:{}",
                pins.digest
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            ),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 3,
                y: 0,
                floor: -7,
            },
        };
        let checked = crate::map::boot::check(bytes.clone(), boot_pins.clone())?;
        let unqualified = roll(world(), 0, JUNE_13);
        let bound = prepare_map(bind_layers(checked, &unqualified)?, world(), &unqualified);
        let channel = ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])?;
        let booted = bound.boot(world(), channel, crate::map::boot::tests::items)?;
        assert!(booted.base().active_layers().is_empty());
        assert!(booted.base().exit_layers().is_empty());
        assert_eq!(booted.base().layer_epoch(), None);
        assert!(
            crate::map::boot::check(bytes, boot_pins)?
                .compose_epoch_layers(0, &[], &[])
                .is_err()
        );
        let mut runtime = channel_runtime();
        assert_eq!(
            apply(
                &mut runtime,
                Some(&boot(&["oteryn:npc.rosemarie"])),
                &unqualified,
                None
            ),
            0
        );
        Ok(())
    }

    #[test]
    fn w05b_the_boot_roll_is_fixed_per_reset_epoch_and_follows_the_calendar() {
        assert_eq!(utc_month_day(JUNE_13), (6, 13));
        assert_eq!(utc_month_day(JULY_1), (7, 1));
        assert_eq!(utc_month_day(MAY_31_LAST), (5, 31));
        // A restart within the day rolls the same day again.
        assert_eq!(
            roll(world(), 41, JUNE_13),
            roll(world(), 41, JUNE_13 + 3_600_000)
        );
        let june = roll(world(), 41, JUNE_13);
        assert!(june.flower_month && june.grimvale_full_moon);
        let july = roll(world(), 41, JULY_1);
        assert!(!july.flower_month && !july.grimvale_full_moon);
        // Across many epochs the chance rolls land near the donor rates (10/20/33%).
        let days: Vec<_> = (1..=3_000)
            .map(|epoch| roll(world(), epoch, JULY_1))
            .collect();
        let rate = |hit: fn(&DailyWorldChanges) -> bool| days.iter().filter(|d| hit(d)).count();
        assert!((240..=360).contains(&rate(|d| d.thawing)), "thawing");
        assert!(
            (510..=690).contains(&rate(|d| d.their_masters_voice)),
            "voice"
        );
        assert!(
            (880..=1_100).contains(&rate(|d| d.oriental_trader.is_some())),
            "trader"
        );
    }

    #[test]
    fn w05b_rosemarie_stands_on_the_live_map_only_in_june() {
        let admitted = boot(&["oteryn:npc.rosemarie"]);
        let mut runtime = channel_runtime();
        let june = roll(world(), 41, JUNE_13);
        assert_eq!(apply(&mut runtime, Some(&admitted), &june, None), 1);
        assert_eq!(npc_keys(&runtime), ["oteryn:npc.rosemarie"]);
        let position = runtime
            .visible_entities()
            .npcs
            .first()
            .map(|npc| (npc.position.x, npc.position.y, npc.position.floor));
        assert_eq!(position, Some((32_534, 32_827, 7)));
        for unix_ms in [MAY_31_LAST, JULY_1] {
            let mut runtime = channel_runtime();
            let day = roll(world(), 41, unix_ms);
            assert_eq!(apply(&mut runtime, Some(&admitted), &day, None), 0);
            assert!(npc_keys(&runtime).is_empty());
        }
        // A catalogue without Rosemarie places nobody.
        let mut runtime = channel_runtime();
        assert_eq!(apply(&mut runtime, Some(&boot(&[])), &june, None), 0);
        assert_eq!(apply(&mut runtime, None, &june, None), 0);
        assert!(npc_keys(&runtime).is_empty());
    }
    #[test]
    fn sp02_yasir_requires_a_ship_map() {
        let mut runtime = channel_runtime();
        let admitted = boot(&["oteryn:npc.yasir"]);
        let changes = DailyWorldChanges::roll(1, 1, 1, |_| 0);
        assert_eq!(apply(&mut runtime, Some(&admitted), &changes, None), 0);
        assert!(npc_keys(&runtime).is_empty());
    }

    fn physical_bundle() -> crate::map::boot::CheckedBundle {
        physical_bundle_omitting(None)
    }

    fn physical_bundle_omitting(
        omitted: Option<(i8, u16, u16)>,
    ) -> crate::map::boot::CheckedBundle {
        use oteryn_world_bundle::sector::{Attrs, Item, Tile};
        use oteryn_world_bundle::writer::{
            self as bundle, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector,
            Terrain, TerrainKind,
        };
        use std::collections::{BTreeMap, BTreeSet};
        let layers: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/encounters/world-changes/layers.json"
        ))
        .expect("layers");
        let mut appearances: BTreeSet<u16> =
            (12059..=12065).chain([231, 432, 486, 776, 10840]).collect();
        let mut grounds: BTreeSet<u16> = (12059..=12065)
            .chain([231, 408, 432, 464, 486, 494])
            .collect();
        for layer in layers["layers"].as_array().expect("layer array") {
            for tile in layer["tiles"].as_array().expect("tiles") {
                if let Some(first) = tile["entries"].as_array().expect("entries").first() {
                    grounds.insert(u16::try_from(first[0].as_u64().expect("ground")).expect("u16"));
                }
                for entry in tile["entries"].as_array().expect("entries") {
                    let id = u16::try_from(entry[0].as_u64().expect("id")).expect("u16");
                    appearances.insert(id);
                }
            }
        }
        for tile in oteryn_gameplay_world_change::variants::definitions()
            .expect("recipes")
            .iter()
            .flat_map(|d| {
                d.inactive_tiles
                    .iter()
                    .chain(d.variants.iter().flat_map(|v| &v.tiles))
            })
        {
            if let Some((ground, _, _)) = tile.entries.first() {
                grounds.insert(*ground);
                appearances.insert(*ground);
            }
        }
        // The base bundle omits daily-only variants and decorations. The real boot must
        // resolve their references from the active Item catalogue, not a test-only palette.
        let palette: Vec<_> = appearances
            .into_iter()
            .filter(|id| {
                (grounds.contains(id) || [776, 10840].contains(id)) && !(12059..=12063).contains(id)
            })
            .map(|id| PaletteEntry {
                family: Family::Item,
                id: u32::from(id),
                key: format!("oteryn:item.tibia.i{id}"),
                terrain: grounds.contains(&id).then_some(Terrain {
                    kind: TerrainKind::Ground,
                    walkable: Some(!matches!(id, 486 | 622 | 4597..=4602 | 13988..=13989)),
                    ground_speed: Some(150),
                }),
            })
            .collect();
        let mut sectors = BTreeMap::<(i8, u16, u16), Sector>::new();
        for (x0, x1, y0, y1, floor, appearance) in [
            (33306, 33369, 31847, 31919, -9, 12065),
            (33330, 33350, 31670, 31690, -7, 432),
            (33305, 33305, 31847, 31847, -9, 12065),
            (33370, 33370, 31847, 31847, -9, 12065),
            (33306, 33306, 31847, 31847, -8, 12065),
        ] {
            let index = palette
                .iter()
                .position(|p| p.id == appearance)
                .expect("palette") as u32;
            for y in y0..=y1 {
                for x in x0..=x1 {
                    sectors
                        .entry((floor, y / 32, x / 32))
                        .or_insert_with(|| Sector {
                            floor,
                            sy: y / 32,
                            sx: x / 32,
                            tiles: Vec::new(),
                        })
                        .tiles
                        .push(Tile {
                            x,
                            y,
                            flags: 0,
                            house: 0,
                            zones: Vec::new(),
                            items: vec![Item {
                                palette: index,
                                depth: 0,
                                attrs: Attrs::default(),
                            }],
                        });
                }
            }
        }
        for (x, y, z, appearance) in [(33290u16, 31785u16, 13i8, 776u16), (33498, 32613, 8, 10840)]
        {
            let ground = palette.iter().position(|p| p.id == 432).expect("ground") as u32;
            let marker = palette
                .iter()
                .position(|p| p.id == u32::from(appearance))
                .expect("marker") as u32;
            sectors
                .entry((-z, y / 32, x / 32))
                .or_insert_with(|| Sector {
                    floor: -z,
                    sy: y / 32,
                    sx: x / 32,
                    tiles: Vec::new(),
                })
                .tiles
                .push(Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: Vec::new(),
                    items: vec![
                        Item {
                            palette: ground,
                            depth: 0,
                            attrs: Attrs::default(),
                        },
                        Item {
                            palette: marker,
                            depth: 0,
                            attrs: Attrs::default(),
                        },
                    ],
                });
        }
        for (x, y, z) in oteryn_gameplay_world_change::variants::definitions()
            .expect("recipes")
            .iter()
            .flat_map(|d| &d.variants)
            .flat_map(|v| &v.portals)
            .flat_map(|p| [p.position, p.destination])
        {
            let ground = palette.iter().position(|p| p.id == 432).expect("ground") as u32;
            let sector = sectors
                .entry((-z, y / 32, x / 32))
                .or_insert_with(|| Sector {
                    floor: -z,
                    sy: y / 32,
                    sx: x / 32,
                    tiles: Vec::new(),
                });
            if !sector.tiles.iter().any(|t| t.x == x && t.y == y) {
                sector.tiles.push(Tile {
                    x,
                    y,
                    flags: 0,
                    house: 0,
                    zones: Vec::new(),
                    items: vec![Item {
                        palette: ground,
                        depth: 0,
                        attrs: Attrs::default(),
                    }],
                });
            }
        }
        for sector in sectors.values_mut() {
            sector.tiles.sort_by_key(|t| (t.y, t.x));
        }
        if let Some(sector) = omitted {
            sectors.remove(&sector);
        }
        let manifest = Manifest {
            world_changes: Some(oteryn_world_bundle::world_changes::Package::new(
                oteryn_gameplay_world_change::variants::RECIPES.into(),
                oteryn_gameplay_world_change::variants::LAYERS.into(),
                oteryn_gameplay_world_change::counter::BINDINGS_JSON.into(),
            )),
            subareas: Default::default(),
            format: bundle::FORMAT.into(),
            min_reader_version: bundle::VERSION,
            projection_class: "server".into(),
            compiler_version: "sp02-fixture".into(),
            build_class: BuildClass::Production,
            identity: Identity {
                project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
                world_schema_version: "world-schema-1".into(),
                content_revision: "sp02".into(),
                required_capabilities: vec![oteryn_world_bundle::world_changes::CAPABILITY.into()],
                ..Identity::default()
            },
            world: Extent {
                min_x: 32000,
                min_y: 31000,
                max_x: 34000,
                max_y: 33000,
                floors: (-15..=-1).collect(),
            },
            palette,
            draft_areas: Vec::new(),
            skipped_provisional_keys: Vec::new(),
            dropped_teleports: Vec::new(),
            spawns: Default::default(),
            npcs: Default::default(),
        };
        let bytes = bundle::write(
            &manifest,
            &sectors.into_values().collect::<Vec<_>>(),
            &Default::default(),
        )
        .expect("bundle");
        let digest = bundle::read(&bytes).expect("verified").digest;
        let pins = crate::map::BundlePins {
            digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "sp02".into(),
            production: true,
        };
        crate::map::boot::check(
            bytes,
            crate::map::boot::BootPins {
                map_revision: format!(
                    "sha256:{}",
                    digest
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                ),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 33330,
                    y: 31670,
                    floor: -7,
                },
            },
        )
        .expect("checked")
    }

    fn boot_physical(changes: &DailyWorldChanges) -> crate::map::boot::BundleWorld {
        try_boot_physical(physical_bundle(), changes).expect("physical boot")
    }

    fn try_boot_physical(
        bundle: crate::map::boot::CheckedBundle,
        changes: &DailyWorldChanges,
    ) -> Result<crate::map::boot::BundleWorld, crate::map::boot::BootRefusal> {
        prepare_map(bundle, world(), changes).boot(
            world(),
            ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
                .expect("channel"),
            |key| {
                let id: u32 = key.strip_prefix("oteryn:item.tibia.i")?.parse().ok()?;
                Some(crate::map::facts::ItemDefinition {
                    reference: std::num::NonZeroU32::new(id + 1)?,
                    solid: Some(false),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            },
        )
    }

    #[test]
    fn wvar_spider_web_barrier_is_explicit_and_return_lands_beside_it() {
        let definition = oteryn_gameplay_world_change::variants::definitions()
            .expect("definitions")
            .iter()
            .find(|d| d.key == "spider_nest")
            .expect("spider");
        let mut day = DailyWorldChanges::roll(7, 1, 1, |_| u64::MAX);
        day.variants = Default::default();
        day.variants.0[definition.slot] = Some(1);
        day.variants.1 = oteryn_gameplay_world_change::variants::recipe_digest();
        let closed = boot_physical(&day);
        let entrance = TilePos {
            x: 32817,
            y: 32280,
            floor: -7,
        };
        assert!(
            closed
                .base()
                .tile(entrance.x, entrance.y, entrance.floor)
                .expect("ground")
                .walkable(),
            "the Item is decorative; the source barrier supplies collision"
        );
        assert!(!closed.enterable(entrance));
        let return_landing = TilePos {
            x: 32818,
            y: 32280,
            floor: -7,
        };
        assert!(closed.enterable(return_landing));
        day.variants.0[definition.slot] = Some(0);
        let opened = boot_physical(&day);
        assert!(opened.enterable(entrance));
        let landing = definition.variants[0].portals[0].destination;
        assert_ne!(landing, definition.variants[0].spawns[0].position);
    }

    #[test]
    fn wvar_closed_entrances_keep_walkable_exit_landings() {
        let mut day = DailyWorldChanges::roll(7, 1, 1, |_| u64::MAX);
        day.variants = Default::default();
        let map = boot_physical(&day);
        for position in [(32516, 32018, 10), (32589, 32303, 8)] {
            let route = day.variants.passage(position).expect("permanent exit");
            let (x, y, z) = route.destination;
            assert!(
                map.base()
                    .tile(x, y, -z)
                    .is_some_and(|tile| tile.walkable())
            );
        }
        let definition = oteryn_gameplay_world_change::variants::definitions()
            .expect("definitions")
            .iter()
            .find(|d| d.key == "awash")
            .expect("awash");
        day.variants.0[definition.slot] = Some(1);
        day.variants.1 = oteryn_gameplay_world_change::variants::recipe_digest();
        let flooded = boot_physical(&day);
        assert!(
            !flooded
                .base()
                .tile(32516, 32018, -9)
                .expect("closed entrance")
                .walkable()
        );
        assert!(
            flooded
                .base()
                .tile(32515, 32018, -9)
                .expect("safe landing")
                .walkable()
        );
    }

    #[test]
    fn wvar_missing_portal_anchor_refuses_boot() {
        let mut day = DailyWorldChanges::roll(7, 1, 1, |_| u64::MAX);
        day.variants = Default::default();
        let bundle = physical_bundle_omitting(Some((-13, 31785 / 32, 33290 / 32)));
        assert!(try_boot_physical(bundle, &day).is_err());
    }

    #[test]
    fn wvar_layer_only_sector_binds_persistent_return() {
        use crate::map::view::{MapFacts, placement_key};
        let mut day = DailyWorldChanges::roll(7, 1, 1, |_| u64::MAX);
        day.variants = Default::default();
        let bundle = physical_bundle_omitting(Some((-8, 32303 / 32, 32589 / 32)));
        let map = try_boot_physical(bundle, &day).expect("layer-only return");
        let at = TilePos {
            x: 32589,
            y: 32303,
            floor: -8,
        };
        let tile = map.base().tile(at.x, at.y, at.floor).expect("return tile");
        assert!(tile.ids().iter().enumerate().any(|(ordinal, id)| {
            map.facts()
                .base_entry(at, ordinal as u8, *id)
                .is_some_and(|entry| entry.appearance_id == 1949)
                && placement_key(at, ordinal as u8)
                    .is_some_and(|key| map.facts().quest_callback_matches(key, None, None, None))
        }));
    }

    #[test]
    fn wvar_maps_restore_portal_markers_select_one_camp_and_revert() {
        use crate::map::view::{MapFacts, placement_key};
        use oteryn_gameplay_world_change::variants;
        let mut day = DailyWorldChanges::roll(7, 1, 1, |_| u64::MAX);
        day.variants = Default::default();
        let inactive = boot_physical(&day);
        for definition in variants::definitions().expect("definitions") {
            for (index, variant) in definition.variants.iter().enumerate() {
                day.variants = Default::default();
                day.variants.0[definition.slot] = Some(index as u8);
                day.variants.1 = variants::recipe_digest();
                let map = boot_physical(&day);
                let restart = boot_physical(&day);
                for portal in &variant.portals {
                    let (dx, dy, dz) = portal.destination;
                    assert!(
                        map.base()
                            .tile(dx, dy, -dz)
                            .is_some_and(|tile| tile.walkable()),
                        "passage destination must have walkable ground: {} {} {:?}",
                        definition.key,
                        variant.key,
                        portal.destination
                    );
                    let (x, y, z) = portal.position;
                    let at = TilePos { x, y, floor: -z };
                    let tile = map.base().tile(x, y, -z).expect("portal tile");
                    assert!(
                        tile.ids()
                            .iter()
                            .zip(tile.depths())
                            .filter(|(_, d)| **d == 0)
                            .enumerate()
                            .any(|(o, (id, _))| map
                                .facts()
                                .base_entry(at, o as u8, *id)
                                .is_some_and(|e| e.appearance_id == portal.appearance))
                    );
                    assert!(
                        map.floors().teleport(at).is_none(),
                        "raw teleport cannot bypass current passage admission"
                    );
                }
                for tag in &variant.tags {
                    let (x, y, z) = tag.position;
                    let pos = TilePos { x, y, floor: -z };
                    assert!(
                        (0..8).any(|ordinal| placement_key(pos, ordinal).is_some_and(|key| map
                            .facts()
                            .quest_callback_matches(
                                key,
                                tag.unique,
                                tag.action,
                                Some(tag.appearance)
                            )))
                    );
                    assert_eq!(
                        map.base().tile(x, y, -z).map(|t| t.ids().to_vec()),
                        restart.base().tile(x, y, -z).map(|t| t.ids().to_vec())
                    );
                }
                if definition.key == "nomads" {
                    let sources = map
                        .activation_spawns(|_| Some((100, false)))
                        .expect("spawns");
                    let nomads: Vec<_> = sources
                        .iter()
                        .flat_map(|s| &s.0)
                        .filter(|p| p.2.contains("nomad") || p.2 == "oteryn:creature.dog")
                        .collect();
                    assert_eq!(nomads.len(), variant.spawns.len());
                    assert!(
                        inactive
                            .activation_spawns(|_| Some((100, false)))
                            .expect("inactive")
                            .iter()
                            .flat_map(|s| &s.0)
                            .all(|p| !p.2.contains("nomad") && p.2 != "oteryn:creature.dog")
                    );
                }
            }
        }
        day.variants = Default::default();
        let reverted = boot_physical(&day);
        assert_eq!(reverted, inactive);
    }

    #[test]
    fn sp02_fungus_boot_changes_only_the_source_rectangle_and_rebuilds_the_same_day() {
        exercise_fungus(41);
    }

    fn exercise_fungus(epoch: u64) {
        use crate::map::view::MapFacts;
        let day = DailyWorldChanges::roll(epoch, 1, 1, |_| 0);
        let world = boot_physical(&day);
        let restarted = boot_physical(&day);
        let mut changed = 0;
        for y in 31847..=31919 {
            for x in 33306..=33369 {
                let pos = crate::map::overlay::TilePos { x, y, floor: -9 };
                let tile = world.base().tile(x, y, -9).expect("fungus tile");
                assert_eq!(
                    tile.ids(),
                    restarted.base().tile(x, y, -9).expect("restart tile").ids()
                );
                let fact = world
                    .facts()
                    .base_entry(pos, 0, tile.ids()[0])
                    .expect("ground fact");
                assert_eq!(
                    fact.terrain_kind,
                    Some(oteryn_world_bundle::bundle::TerrainKind::Ground)
                );
                assert!(
                    fact.appearance_id == 12065 || (12059..=12063).contains(&fact.appearance_id)
                );
                changed += usize::from(fact.appearance_id != 12065);
            }
        }
        assert!((1200..=1600).contains(&changed), "changed {changed}");
        assert_eq!(world, restarted);
        for (x, y, floor) in [(33305, 31847, -9), (33370, 31847, -9), (33306, 31847, -8)] {
            assert_eq!(
                world.base().tile(x, y, floor).expect("outside tile").ids(),
                &[12065]
            );
        }
        let none = boot_physical(&DailyWorldChanges::roll(epoch, 1, 1, |_| u64::MAX));
        assert_eq!(
            none.base().tile(33306, 31847, -9).expect("ground").ids(),
            &[12065]
        );
    }

    #[test]
    fn sp02_trader_boot_loads_each_ship_before_placing_yasir() {
        exercise_trader(41);
    }

    fn exercise_trader(epoch: u64) {
        for (town, at) in [
            (
                oteryn_gameplay_world_change::daily::OrientalTraderTown::Ankrahmun,
                (33102, 32884),
            ),
            (
                oteryn_gameplay_world_change::daily::OrientalTraderTown::Carlin,
                (32400, 31815),
            ),
            (
                oteryn_gameplay_world_change::daily::OrientalTraderTown::LibertyBay,
                (32314, 32895),
            ),
        ] {
            let mut day = DailyWorldChanges::roll(epoch, 1, 1, |_| u64::MAX);
            day.oriental_trader = Some(town);
            let map = boot_physical(&day);
            let pos = crate::map::overlay::TilePos {
                x: at.0,
                y: at.1,
                floor: -6,
            };
            assert!(map.base().tile(at.0, at.1, -6).is_some(), "ship tile");
            assert!(map.enterable(pos), "Yasir deck is walkable");
            let layers: serde_json::Value = serde_json::from_str(include_str!(
                "../../../../../content/encounters/world-changes/layers.json"
            ))
            .expect("layers");
            let source = layers["layers"]
                .as_array()
                .expect("layers")
                .iter()
                .find(|l| l["key"].as_str() == Some(town.layer()))
                .expect("selected layer");
            for tile in source["tiles"].as_array().expect("tiles") {
                let x = tile["position"][0].as_u64().expect("x") as u16;
                let y = tile["position"][1].as_u64().expect("y") as u16;
                let floor = -(tile["position"][2].as_i64().expect("z") as i8);
                let tile_pos = crate::map::overlay::TilePos { x, y, floor };
                let actual =
                    crate::map::view::compose_tile(map.overlay(), &map.facts().pinned(), tile_pos)
                        .expect("composed source tile")
                        .expect("ship stack");
                let expected: Vec<_> = tile["entries"]
                    .as_array()
                    .expect("entries")
                    .iter()
                    .map(|e| e[0].as_u64().expect("appearance") as u16)
                    .collect();
                assert_eq!(
                    actual
                        .entries
                        .iter()
                        .map(|e| e.facts.appearance_id)
                        .collect::<Vec<_>>(),
                    expected
                );
                let flags = tile["flags"].as_u64().unwrap_or(0);
                let base = map.base().tile(x, y, floor).expect("ship base");
                assert_eq!(base.protection_zone(), flags & 1 != 0);
                assert_eq!(base.no_logout(), flags & 8 != 0);
            }
            let mut runtime = channel_runtime();
            assert_eq!(
                apply(
                    &mut runtime,
                    Some(&boot(&["oteryn:npc.yasir"])),
                    &day,
                    Some(&map)
                ),
                1
            );
            assert_eq!(npc_keys(&runtime), ["oteryn:npc.yasir"]);
        }
    }
    #[test]
    fn sp02_grimvale_boot_activates_only_the_days_creatures() {
        exercise_grimvale(41);
    }

    fn exercise_grimvale(epoch: u64) {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || exercise_grimvale_inner(epoch))
            .expect("Grimvale thread")
            .join()
            .expect("Grimvale owner case");
    }

    fn exercise_grimvale_inner(epoch: u64) {
        use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                world(),
                None,
            );
        let active = controller.active().expect("active native generation");
        let native = active.native_gameplay().expect("native");
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("executor");
        for day in [12, 13, 14] {
            let changes = DailyWorldChanges::roll(epoch, 1, day, |_| u64::MAX);
            let map = boot_physical(&changes);
            let sources = map
                .activation_spawns(|_| Some((100, false)))
                .expect("activation sources");
            let points: Vec<_> = sources.iter().flat_map(|(points, _)| points).collect();
            assert!(!points.is_empty(), "Grimvale sources");
            assert!(points.len() <= 21);
            let allowed = if day == 13 {
                &["oteryn:creature.wereboar", "oteryn:creature.werebadger"][..]
            } else {
                &[
                    "oteryn:creature.bandit",
                    "oteryn:creature.badger",
                    "oteryn:creature.blue_butterfly",
                    "oteryn:creature.yellow_butterfly",
                ][..]
            };
            let count = points.len();
            for (position, delay, creature, health, _) in points {
                assert!((33330..=33350).contains(&position.0));
                assert!((31670..=31690).contains(&position.1));
                assert_eq!(position.2, 7);
                assert!(allowed.contains(&creature.as_str()));
                assert_eq!(*health, 100);
                assert_eq!(*delay, 90_000);
            }
            let channel_pin = oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::from_activation(
                world(),pin.activation_sequence(),pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),map.frame_binding_digest(world()),
                map.map_revision_digest(),(33330,31670,7));
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                world(),
                ChannelId::decode(&[1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2])
                    .expect("channel"),
                NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3])
                    .expect("node"),
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                64,
                channel_pin,
            )
            .expect("runtime");
            native
                .install_companion_policies(&mut runtime)
                .expect("served policies");
            let realized = executor
                .block_on(super::super::realize_bundle_spawns(
                    &mut runtime,
                    &map,
                    &oteryn_foundation::CancellationToken::new(),
                ))
                .expect("activation");
            assert_eq!(realized, Some((count, 1)));
            for (_, _, actor) in runtime.spawn_point_creatures() {
                let actor = actor.expect("live Grimvale creature");
                let snapshot = runtime.companion_snapshot(actor).expect("served creature");
                assert!(snapshot.health > 0);
                assert_eq!(snapshot.health, snapshot.maximum_health);
            }
            let (point, _, actor) = runtime.spawn_point_creatures()[0];
            let actor = actor.expect("first creature");
            let health = runtime.companion_snapshot(actor).expect("health").health;
            runtime
                .commit_monster_lab_damage(actor, b"sp02-lethal-hit", health)
                .expect("lethal hit");
            crate::gameplay_transport::drive_owner_spawns(&mut runtime, 0);
            let due = runtime
                .pending_respawn(point)
                .expect("pending respawn")
                .due_micros;
            assert_eq!(due, 90_000_000);
            crate::gameplay_transport::drive_owner_spawns(&mut runtime, due);
            crate::gameplay_transport::drive_owner_spawns(&mut runtime, due + 4_200_000);
            assert!(runtime.spawn_point_creatures()[0].2.is_some());
            assert_ne!(runtime.spawn_point_creatures()[0].2, Some(actor));
        }
    }

    fn pg_physical(
        unit: &'static str,
        exercise: fn(u64),
    ) -> crate::bestiary_postgres_harness::TestResult {
        use crate::bestiary_postgres_harness::{Harness, configured_admin, runtime};
        let Some(admin) = configured_admin() else {
            return Ok(());
        };
        std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
            let run = || -> crate::bestiary_postgres_harness::TestResult {
                runtime()?.block_on(async {
                    let h = Harness::create(admin,unit,false).await?;
                    let world = world();
                    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,41,$2,'sp02',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
                        .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(&h.pool).await?;
                    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,41)")
                        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
                    // Readiness refuses an unbound nonzero event epoch; adding its committed
                    // event binding is the same durable boundary the actual Channel boot reads.
                    assert!(h.root.active_world_event_epoch(world).await.is_err());
                    sqlx::query("INSERT INTO game_world_event_epochs VALUES(encode($1,'hex')::uuid,41,false,'')")
                        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
                    let epoch = h.root.active_world_event_epoch(world).await?.ok_or("missing epoch")?;
                    assert_eq!(epoch,41);
                    exercise(epoch);
                    let restarted = h.root.active_world_event_epoch(world).await?.ok_or("restart epoch")?;
                    exercise(restarted);
                    eprintln!("SP02 POSTGRES_CASE_EXECUTED {unit} epoch={epoch} restart={restarted}");
                    h.cleanup().await
                })
            };
            run().map_err(|e|e.to_string())
        })?.join().map_err(|_|"SP02 PostgreSQL thread failed")?
            .map_err(Into::into)
    }

    #[test]
    fn sp02_sv05_postgres_epoch_and_restart() -> crate::bestiary_postgres_harness::TestResult {
        pg_physical("sp02_sv05", exercise_fungus)
    }
    #[test]
    fn sp02_sv06_postgres_epoch_and_restart() -> crate::bestiary_postgres_harness::TestResult {
        pg_physical("sp02_sv06", exercise_trader)
    }
    #[test]
    fn sp02_sv07_postgres_epoch_and_restart() -> crate::bestiary_postgres_harness::TestResult {
        pg_physical("sp02_sv07", exercise_grimvale)
    }

    #[test]
    fn w05b_apply_items_without_a_bundle_places_nothing() {
        let day = DailyWorldChanges::roll(1, 1, 1, |_| 0);
        assert!(day.thawing);
        assert_eq!(apply_items(None, &day), 0);
    }
}
