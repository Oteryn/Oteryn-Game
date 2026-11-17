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

#[cfg(test)]
pub(super) fn roll_calendar(
    world: WorldId,
    reset_epoch: u64,
    month: u8,
    day: u8,
) -> DailyWorldChanges {
    oteryn_gameplay_world_change::daily::roll_calendar(world.as_bytes(), reset_epoch, month, day)
}

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
        items: changes.items().collect(),
        trader: changes.oriental_trader.map(|town| town.layer()),
    }
}

pub(super) fn prepare_map(
    checked: crate::map::boot::CheckedBundle,
    world: WorldId,
    changes: &DailyWorldChanges,
) -> crate::map::boot::CheckedBundle {
    if changes.reset_epoch == 0 {
        return checked;
    }
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

/// Counts the reset day's Items projected into the base map before its owners were built.
/// This does not add a second callback copy, which the physical harvest owner cannot consume.
pub(super) fn apply_items(
    map: Option<&crate::map::boot::BundleWorld>,
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
        use crate::map::view::MapFacts;
        let present = map.base().tile(x, y, -z).is_some_and(|tile| {
            tile.ids().iter().enumerate().any(|(ordinal, id)| {
                u8::try_from(ordinal)
                    .ok()
                    .and_then(|ordinal| map.facts().base_entry(pos, ordinal, *id))
                    .is_some_and(|facts| facts.appearance_id == appearance)
            })
        });
        if present {
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
pub(crate) mod tests {
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
        let mut appearances: BTreeSet<u16> = (12059..=12065).chain([432, 486]).collect();
        let grounds: BTreeSet<u16> = (12059..=12065).chain([408, 432, 464, 486, 494]).collect();
        for layer in layers["layers"].as_array().expect("layer array") {
            if !layer["key"]
                .as_str()
                .expect("key")
                .starts_with("oriental_trader/")
            {
                continue;
            }
            for tile in layer["tiles"].as_array().expect("tiles") {
                for entry in tile["entries"].as_array().expect("entries") {
                    let id = u16::try_from(entry[0].as_u64().expect("id")).expect("u16");
                    appearances.insert(id);
                }
            }
        }
        // The base bundle omits daily-only variants and decorations. The real boot must
        // resolve their references from the active Item catalogue, not a test-only palette.
        let palette: Vec<_> = appearances
            .into_iter()
            .filter(|id| grounds.contains(id) && !(12059..=12063).contains(id))
            .map(|id| PaletteEntry {
                family: Family::Item,
                id: u32::from(id),
                key: format!("oteryn:item.tibia.i{id}"),
                terrain: grounds.contains(&id).then_some(Terrain {
                    kind: TerrainKind::Ground,
                    walkable: Some(id != 486),
                    ground_speed: Some(150),
                    blocks_projectile: None,
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
        for (x, y, z) in oteryn_gameplay_world_change::daily::THAWING_ICE_FLOWERS {
            let floor = -z;
            let index = palette.iter().position(|p| p.id == 432).expect("ground") as u32;
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
        for sector in sectors.values_mut() {
            sector.tiles.sort_by_key(|t| (t.y, t.x));
        }
        let manifest = Manifest {
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
                ..Identity::default()
            },
            world: Extent {
                min_x: 32000,
                min_y: 31000,
                max_x: 34000,
                max_y: 33000,
                floors: (-9..=-1).collect(),
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
        prepare_map(physical_bundle(), world(), changes)
            .boot(
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
            .expect("physical boot")
    }

    #[test]
    fn sp02_fungus_boot_changes_only_the_source_rectangle_and_rebuilds_the_same_day() {
        exercise_fungus(41);
    }

    fn exercise_fungus(epoch: u64) {
        exercise_fungus_snapshot(DailyWorldChanges::roll(epoch, 1, 1, |_| 0));
    }

    fn exercise_fungus_snapshot(day: DailyWorldChanges) {
        use crate::map::view::MapFacts;
        let epoch = day.reset_epoch;
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
        if day.their_masters_voice {
            assert!((1200..=1600).contains(&changed), "changed {changed}");
        } else {
            assert_eq!(changed, 0);
        }
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
        for day in [12, 13, 14] {
            exercise_grimvale_snapshot(DailyWorldChanges::roll(epoch, 1, day, |_| u64::MAX));
        }
    }

    fn exercise_grimvale_snapshot(changes: DailyWorldChanges) {
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
        let map = boot_physical(&changes);
        let sources = map
            .activation_spawns(|_| Some((100, false)))
            .expect("activation sources");
        let points: Vec<_> = sources.iter().flat_map(|(points, _)| points).collect();
        assert!(!points.is_empty(), "Grimvale sources");
        assert!(points.len() <= 21);
        let allowed = if changes.grimvale_full_moon {
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
            NodeId::decode(&[1, 0, 0, 0, 0, 3, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 3]).expect("node"),
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

    pub(crate) fn qworld_physical_case() -> crate::bestiary_postgres_harness::TestResult {
        use crate::bestiary_postgres_harness::{Harness, configured_admin, runtime};
        let Some(admin) = configured_admin() else {
            return Ok(());
        };
        std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
            let run = || -> crate::bestiary_postgres_harness::TestResult {
                runtime()?.block_on(async {
                    let h = Harness::create(admin, "qworld_physical", false).await?;
                    let world = world();
                    let active = (1..=128).find(|epoch| {
                        let day = roll_calendar(world, *epoch, 6, 13);
                        day.their_masters_voice && day.fury_gates.is_some()
                    }).ok_or("active source occurrence")?;
                    let inactive = (active + 1..=active + 128).find(|epoch| {
                        let day = roll_calendar(world, *epoch, 6, 14);
                        !day.their_masters_voice && day.fury_gates.is_none()
                    }).ok_or("inactive source occurrence")?;
                    for (epoch, day, enabled) in [(active, 13, true), (inactive, 14, false)] {
                        let timestamp = format!("2026-06-{day:02} 06:00:00+00");
                        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'qworld',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,$4::text::timestamptz,'2026-06')")
                            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind([7_u8;32].as_slice()).bind(timestamp).execute(&h.pool).await?;
                        let announcement = oteryn_gameplay_world_change::scheduled::announcement_for_epoch(epoch).unwrap_or_default();
                        sqlx::query("INSERT INTO game_world_event_epochs(world_id,reset_epoch,active,announcement) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4)")
                            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(!announcement.is_empty()).bind(announcement).execute(&h.pool).await?;
                        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch")
                            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
                        assert_eq!(h.root.world_change_calendar(world, epoch).await?, (6, day));
                        let snapshot = h.root.world_change_snapshot(world, epoch).await?;
                        assert_eq!(snapshot.their_masters_voice, enabled);
                        assert_eq!(snapshot.grimvale_full_moon, enabled);
                        assert_eq!(snapshot.fury_gates.is_some(), enabled);
                        assert_eq!(h.root.world_change_snapshot(world, epoch).await?, snapshot);
                        // Separate native runtime: the spawn owner drives its own Tokio clock.
                        std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
                            exercise_fungus_snapshot(snapshot);
                            exercise_grimvale_snapshot(snapshot);
                            exercise_fury_snapshot(snapshot).map_err(|error| error.to_string())
                        })?.join().map_err(|_| "QWORLD physical owner thread failed")??;
                    }
                    assert!(h.root.world_change_snapshot(world, active).await.is_err());
                    h.cleanup().await
                })
            };
            run().map_err(|error| error.to_string())
        })?.join().map_err(|_| "QWORLD PostgreSQL thread failed")?.map_err(Into::into)
    }

    fn exercise_fury_snapshot(
        changes: DailyWorldChanges,
    ) -> crate::bestiary_postgres_harness::TestResult {
        use compile::{KeyResolver, Resolution};
        use oteryn_world_bundle_compiler::{compile, source};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let layers: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../content/encounters/world-changes/layers.json"
        ))?;
        let source_layer = layers["layers"]
            .as_array()
            .ok_or("source layers")?
            .iter()
            .find(|layer| layer["key"] == changes.fury_exit)
            .ok_or("selected Fury source layer")?;
        let tiles = source_layer["tiles"].as_array().ok_or("source tiles")?;
        let regions: std::collections::BTreeSet<_> = tiles
            .iter()
            .map(|tile| {
                let x = tile["position"][0].as_u64().ok_or("source x")? as u16;
                let y = tile["position"][1].as_u64().ok_or("source y")? as u16;
                Ok::<_, &'static str>((x / 256, y / 256))
            })
            .collect::<Result<_, _>>()?;
        let project = source::project_regions(&root, &regions.into_iter().collect::<Vec<_>>())?;
        let (registry, palette) = source::registry(&root, &project)?;
        let input = compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity: oteryn_world_bundle::bundle::Identity {
                project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
                world_schema_version: "world-schema-1".into(),
                content_revision: "qworld".into(),
                ..Default::default()
            },
            world: source::world_extent(&root)?,
            build_class: oteryn_world_bundle::bundle::BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &project.families,
        };
        let compiled = compile::compile(&input, &registry)?;
        let pins = crate::map::BundlePins {
            digest: compiled.digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "qworld".into(),
            production: false,
        };
        let base = crate::map::load(&compiled.bytes, &pins)?;
        let decoded = oteryn_world_bundle::bundle::read(&compiled.bytes)?;
        let (floor, x, y) = decoded
            .sectors
            .iter()
            .filter(|sector| sector.floor == -7)
            .flat_map(|sector| sector.tiles.iter().map(move |tile| (sector.floor, tile)))
            .find(|(floor, tile)| {
                base.tile(tile.x, tile.y, *floor)
                    .is_some_and(|entry| entry.walkable())
                    && tile.items.iter().all(|item| {
                        let entry = &decoded.manifest.palette[item.palette as usize];
                        entry.family != oteryn_world_bundle::bundle::Family::Item
                            || !registry.solid(&entry.key)
                            || (!registry.solid_stated(&entry.key)
                                && entry.terrain.is_some_and(|terrain| {
                                    terrain.kind == oteryn_world_bundle::bundle::TerrainKind::Ground
                                }))
                    })
                    && !tiles.iter().any(|source| {
                        source["position"][0].as_u64() == Some(u64::from(tile.x))
                            && source["position"][1].as_u64() == Some(u64::from(tile.y))
                    })
            })
            .map(|(floor, tile)| (floor, tile.x, tile.y))
            .ok_or("enterable source entry outside Fury overlay")?;
        let checked = crate::map::boot::check(
            compiled.bytes,
            crate::map::boot::BootPins {
                bundle: pins,
                map_revision: format!(
                    "sha256:{}",
                    compiled
                        .digest
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<String>()
                ),
                start: TilePos { x, y, floor },
            },
        )?;
        let projected = bind_layers(checked, &changes)?.boot(
            world(),
            crate::foundation::ChannelId::decode(&[
                1, 0, 0, 0, 0, 2, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2,
            ])?,
            |key| {
                let Resolution::Resolved(oteryn_world_bundle::bundle::Family::Item, id) =
                    registry.resolve(key)
                else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: std::num::NonZeroU32::new(id + 1)?,
                    solid: registry.solid_stated(key).then(|| registry.solid(key)),
                    blocks_projectile: registry.solid(key),
                    pickupable: false,
                    door: false,
                })
            },
        )?;
        assert_eq!(projected.base().layer_epoch(), Some(changes.reset_epoch));
        assert_eq!(
            projected
                .base()
                .active_layers()
                .iter()
                .any(|key| key == changes.fury_exit),
            changes.fury_gates.is_some()
        );
        assert!(
            projected
                .base()
                .exit_layers()
                .iter()
                .any(|key| key == changes.fury_exit)
        );
        if changes.fury_gates.is_some() {
            let mut different = 0;
            for tile in tiles {
                let x = tile["position"][0].as_u64().ok_or("source x")? as u16;
                let y = tile["position"][1].as_u64().ok_or("source y")? as u16;
                let floor = -(tile["position"][2].as_i64().ok_or("source z")? as i8);
                let active = projected
                    .base()
                    .tile(x, y, floor)
                    .ok_or("active Fury tile")?;
                different += usize::from(
                    base.tile(x, y, floor)
                        .is_none_or(|old| old.ids() != active.ids()),
                );
            }
            assert!(
                different > 0,
                "source Fury layer must change physical tiles"
            );
        }
        Ok(())
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
    fn thawing_boot_installs_harvestable_source_placements() {
        use crate::map::view::MapFacts;
        let active = DailyWorldChanges::roll(41, 1, 1, |_| 0);
        let day = boot_physical(&active);
        let restarted = boot_physical(&active);
        let inactive = boot_physical(&DailyWorldChanges::roll(42, 1, 1, |_| u64::MAX));
        for (appearance, (x, y, z)) in active.items() {
            let pos = TilePos { x, y, floor: -z };
            let read = |world: &crate::map::boot::BundleWorld| {
                let tile = world.base().tile(x, y, -z).expect("source ground");
                tile.ids()
                    .iter()
                    .enumerate()
                    .filter_map(|(ordinal, id)| {
                        world
                            .facts()
                            .base_entry(pos, u8::try_from(ordinal).ok()?, *id)
                    })
                    .map(|facts| facts.appearance_id)
                    .collect::<Vec<_>>()
            };
            assert_eq!(read(&day), vec![432, appearance], "{pos:?}");
            assert_eq!(read(&day), read(&restarted));
            assert_eq!(read(&inactive), vec![432]);
            assert!(day.facts().movement_items(pos).is_empty());
        }
        assert_eq!(active.items().count(), 14);
        assert_eq!(apply_items(Some(&day), &active), 14);
        for (_, (x, y, z)) in active.items() {
            assert!(
                day.facts()
                    .movement_items(TilePos { x, y, floor: -z })
                    .is_empty()
            );
        }
    }

    #[test]
    fn w05b_apply_items_without_a_bundle_places_nothing() {
        let day = DailyWorldChanges::roll(1, 1, 1, |_| 0);
        assert!(day.thawing);
        assert_eq!(apply_items(None, &day), 0);
    }
}
