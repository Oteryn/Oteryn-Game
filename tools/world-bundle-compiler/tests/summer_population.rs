//! SUMMER: all admitted static Summer points and Mayor Pocaro on their real ground.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use oteryn_world_bundle_compiler::bundle::{BuildClass, Identity};
use oteryn_world_bundle_compiler::compile::{Input, realize_npcs, realize_spawns};
use oteryn_world_bundle_compiler::project::{self, Families};
use oteryn_world_bundle_compiler::resolve::Registry;
use serde_json::Value;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn shards(root: &Path, directory: &str, prefix: &str) -> TestResult<Vec<Vec<u8>>> {
    let mut paths = fs::read_dir(root.join(directory))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".json"))
    });
    paths.sort();
    paths
        .into_iter()
        .map(|path| fs::read(path).map_err(Into::into))
        .collect()
}

#[test]
fn summer_population_all_static_species_and_mayor_bind_to_real_map_cells() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // Crystal's summer and current maps agree on these populations. This is source-map
    // qualification, not independent proof of Global spawn coordinates or attack balance.
    let expected: BTreeMap<String, usize> = [
        ("cult_initiate", 1),
        ("iceplume_strider", 29),
        ("moonsilver_custodian", 1),
        ("moonspawn_blightspitter", 13),
        ("moonspawn_oozecrown", 15),
        ("moonstone_excavator", 55),
        ("moonstone_overseer", 36),
        ("radiant_acolyte", 51),
        ("radiant_inquisitor", 56),
        ("radiant_paragon", 84),
        ("radiant_templar", 64),
        ("radiant_warden", 78),
        ("radiant_zealot", 65),
        // Ordinary points only: the thirty-second Sentinel is a same-cell weighted
        // alternative, qualified separately below without guaranteeing another actor.
        ("silverfrost_sentinel", 31),
        ("winged_jaracal", 30),
    ]
    .into_iter()
    .map(|(name, count)| (format!("oteryn:creature.{name}"), count))
    .collect();
    let mut families = Families::default();
    for bytes in shards(&root, "content/creatures/definitions", "creatures-")? {
        families.add_creatures(&bytes)?;
    }
    for bytes in shards(&root, "content/world/spawns", "spawns-")? {
        families.add_spawns(&bytes)?;
    }
    // Keep the authored points and the real region bytes; no synthetic ground or resolver.
    for source in &mut families.spawns {
        source
            .points
            .retain(|point| expected.contains_key(&point.creature));
    }
    families.spawns.retain(|source| !source.points.is_empty());
    let mut authored = BTreeMap::new();
    let mut regions = BTreeSet::new();
    for source in &families.spawns {
        for point in &source.points {
            *authored.entry(point.creature.clone()).or_insert(0) += 1;
            let (x, y, z) = point.cell;
            regions.insert(format!(
                "content/world/placements/region-z{z:02}-x{:03}-y{:03}.b3",
                x / 256,
                y / 256
            ));
        }
    }
    assert_eq!(authored, expected);
    // Summer's existing ingress and return paths are map-authored Transition records,
    // not new unrestricted travel services. Qualify both ends against the real B3
    // teleport attributes, using the same parity caller as production compilation.
    let entrances = [
        ((33679, 32684, 15), (33975, 32636, 1)),
        ((33973, 32635, 1), (33681, 32685, 15)),
        ((33973, 32636, 1), (33681, 32685, 15)),
        ((34091, 32372, 8), (34107, 32619, 7)),
        ((34107, 32621, 7), (34090, 32370, 8)),
        // Explicit paired destinations in the pinned15.30 werepanther map.
        ((33039, 32964, 8), (33045, 32988, 8)),
        ((33043, 32986, 8), (33040, 32962, 8)),
    ];
    for bytes in shards(&root, "content/world/transitions", "teleports-")? {
        families.add_teleports(&bytes)?;
    }
    for (from, to) in entrances {
        assert_eq!(families.teleports.get(&from), Some(&to));
        for (x, y, z) in [from, to] {
            regions.insert(format!(
                "content/world/placements/region-z{z:02}-x{:03}-y{:03}.b3",
                x / 256,
                y / 256,
            ));
        }
    }
    let regions = regions
        .into_iter()
        .map(|path| fs::read(root.join(path)))
        .collect::<Result<Vec<_>, _>>()?;
    let parity = oteryn_world_bundle_compiler::compile::parity(&regions, &families)?;
    assert!(parity.no_record.is_empty(), "{parity:#?}");
    assert!(parity.mismatched.is_empty(), "{parity:#?}");
    for (from, _) in entrances {
        assert!(
            !parity.records_without_attribute.contains(&from),
            "Summer entrance {from:?} must be present in the shipped map"
        );
    }
    let index: Value =
        serde_json::from_slice(&fs::read(root.join("content/world/placements/index.json"))?)?;
    let mut registry = Registry::default();
    for bytes in shards(&root, "content/items/definitions", "items-")? {
        registry.add_items(&bytes)?;
    }
    registry.add_item_profiles(&fs::read(
        root.join("content/items/definitions/spell-native-profiles.json"),
    )?)?;
    for (family, directory, prefix) in [
        ("Terrain", "content/world/terrain", "terrain-"),
        ("WorldObject", "content/world/objects", "objects-"),
    ] {
        for bytes in shards(&root, directory, prefix)? {
            registry.add_catalogue(family, &bytes)?;
        }
    }
    let palette = index["palette"]
        .as_array()
        .ok_or("palette")?
        .iter()
        .map(|entry| {
            let key = entry["key"].as_str().ok_or("palette key")?.to_owned();
            if entry["provisional"] == true {
                registry.add_provisional(key.clone());
            }
            Ok(key)
        })
        .collect::<TestResult<Vec<_>>>()?;
    registry.seal()?;
    let worlds = shards(&root, "content/world/worlds", "worlds-")?;
    let input = Input {
        regions: &regions,
        palette: &palette,
        identity: Identity::default(),
        world: project::world_extent(worlds.first().ok_or("World shard")?)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &families,
    };
    let (table, report) = realize_spawns(&input, &registry)?;
    // Decision 21 admits the exact appearance-only ground through Terrain.
    // The donor places one Excavator on official 15.30 appearance 53854:
    // bank + unpass, speed 1. Preserve that blocked cell instead of changing
    // current-client collision facts or relocating the donor creature.
    assert_eq!(
        report.dropped,
        vec![oteryn_world_bundle_compiler::compile::DroppedPoint {
            source: "oteryn:spawn.x34161_y32684_z6".into(),
            creature: "oteryn:creature.moonstone_excavator".into(),
            cell: (34161, 32684, 6),
            reason: oteryn_world_bundle_compiler::compile::SpawnReason::NoGround,
        }],
    );
    let mut realized = BTreeMap::new();
    for source in &table.sources {
        for point in &source.points {
            *realized
                .entry(table.creatures[point.creature as usize].key.clone())
                .or_insert(0) += 1;
        }
    }
    let mut admitted = expected;
    *admitted
        .get_mut("oteryn:creature.moonstone_excavator")
        .ok_or("Excavator expected count")? -= 1;
    assert_eq!(realized, admitted);
    assert_eq!(table.point_count(), 608);

    // The production travel-admission caller uses the same landing-cell checks as
    // native bundle boot. A matching teleport attribute alone would not detect a
    // target mapped to unclassified ground or a solid object.
    let entry_families = Families {
        routes: entrances
            .iter()
            .enumerate()
            .map(|(ordinal, (_, destination))| project::TravelRoute {
                service: "oteryn:service.summer_entry_validation".into(),
                route: format!("entry_{ordinal}"),
                destination: *destination,
            })
            .collect(),
        ..Families::default()
    };
    let (_, entries) = realize_npcs(
        &Input {
            regions: &regions,
            palette: &palette,
            identity: Identity::default(),
            world: input.world.clone(),
            build_class: BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &entry_families,
        },
        &registry,
        &table,
    )?;
    assert!(entries.routes_held.is_empty(), "{entries:#?}");
    assert_eq!(entries.routes, entrances.len());

    let world = input.world.clone();
    for (directory, prefix) in [
        ("content/services/trade", "trade-"),
        ("content/services/travel", "travel-"),
    ] {
        for bytes in shards(&root, directory, prefix)? {
            families.add_services(&bytes)?;
        }
    }
    for bytes in shards(&root, "content/npcs/definitions", "npcs-")? {
        families.add_npcs(&bytes)?;
    }
    for bytes in shards(&root, "content/dialogues/definitions", "dialogues-")? {
        families.add_dialogues(&bytes)?;
    }
    for bytes in shards(&root, "content/world/npc-placements", "placements-")? {
        families.add_npc_placements(&bytes)?;
    }
    let source: Value = serde_json::from_slice(&fs::read(
        root.join("tools/content-migration/summer_npcs.json"),
    )?)?;
    let positions = source["positions"].as_array().ok_or("Summer positions")?;
    let names: BTreeSet<_> = positions
        .iter()
        .filter_map(|row| row["npc"].as_str())
        .collect();
    assert_eq!(names.len(), 19);
    families
        .npc_placements
        .retain(|placement| names.contains(placement.npc.as_str()));
    assert_eq!(families.npc_placements.len(), 20);
    // Only the Summer source captain's real route belongs to this bounded census.
    families
        .routes
        .retain(|route| route.service == "oteryn:service.travel.captain_corsarah");
    let mut npc_regions = BTreeSet::new();
    for route in &families.routes {
        let (x, y, z) = route.destination;
        npc_regions.insert(format!(
            "content/world/placements/region-z{z:02}-x{:03}-y{:03}.b3",
            x / 256,
            y / 256
        ));
    }
    for placement in &families.npc_placements {
        let (x, y, z) = placement.cell;
        assert!(positions.iter().any(|row| row["npc"] == placement.npc
            && row["cell"]["x"] == x
            && row["cell"]["y"] == y
            && row["cell"]["floor"] == z));
        npc_regions.insert(format!(
            "content/world/placements/region-z{z:02}-x{:03}-y{:03}.b3",
            x / 256,
            y / 256
        ));
    }
    let npc_regions = npc_regions
        .into_iter()
        .map(|path| fs::read(root.join(path)))
        .collect::<Result<Vec<_>, _>>()?;
    // Doctor is stationary in the source. Qualify a real adjacent southern talk
    // approach independently of the NPC standing cell; no synthetic floor or relocation.
    families.routes.push(project::TravelRoute {
        service: "oteryn:service.summer_doctor_approach_validation".into(),
        route: "southern_adjacent_cell".into(),
        destination: (34010, 32642, 6),
    });
    let input = Input {
        regions: &npc_regions,
        palette: &palette,
        identity: Identity::default(),
        world,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &families,
    };
    let (npcs, report) = realize_npcs(&input, &registry, &table)?;
    assert!(report.held.is_empty(), "{report:#?}");
    assert_eq!(npcs.placement_count(), 20);
    assert_eq!(npcs.npcs.len(), 19);
    assert!(
        report.routes_held.is_empty(),
        "Doctor approach: {report:#?}"
    );
    // Captain Corsarah's retained source route and Doctor's adjacent approach.
    assert_eq!(report.routes, 2);
    Ok(())
}

fn srv7_weighted_registry(root: &Path) -> TestResult<(Registry, Vec<String>)> {
    let index: Value =
        serde_json::from_slice(&fs::read(root.join("content/world/placements/index.json"))?)?;
    let mut registry = Registry::default();
    for bytes in shards(root, "content/items/definitions", "items-")? {
        registry.add_items(&bytes)?;
    }
    registry.add_item_profiles(&fs::read(
        root.join("content/items/definitions/spell-native-profiles.json"),
    )?)?;
    for (family, directory, prefix) in [
        ("Terrain", "content/world/terrain", "terrain-"),
        ("WorldObject", "content/world/objects", "objects-"),
    ] {
        for bytes in shards(root, directory, prefix)? {
            registry.add_catalogue(family, &bytes)?;
        }
    }
    let palette = index["palette"]
        .as_array()
        .ok_or("palette")?
        .iter()
        .map(|entry| {
            let key = entry["key"].as_str().ok_or("palette key")?.to_owned();
            if entry["provisional"] == true {
                registry.add_provisional(key.clone());
            }
            Ok(key)
        })
        .collect::<TestResult<Vec<_>>>()?;
    registry.seal()?;
    Ok((registry, palette))
}

#[test]
fn srv7_summer_weighted_sentinel_and_asura_compile_on_real_source_cells() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut families = Families::default();
    for bytes in shards(&root, "content/creatures/definitions", "creatures-")? {
        families.add_creatures(&bytes)?;
    }
    for bytes in shards(&root, "content/world/spawns", "spawns-")? {
        families.add_spawns(&bytes)?;
    }
    families
        .spawns
        .retain(|source| source.key == "oteryn:spawn.x33866_y32698_z9");
    assert_eq!(
        families.spawns.len(),
        1,
        "the held source group must have one production caller"
    );
    assert_eq!(
        families.spawns[0].points.len(),
        2,
        "one weighted cell plus independent south cell"
    );
    let regions = vec![fs::read(
        root.join("content/world/placements/region-z09-x132-y127.b3"),
    )?];
    let (registry, palette) = srv7_weighted_registry(&root)?;
    let worlds = shards(&root, "content/world/worlds", "worlds-")?;
    let input = Input {
        regions: &regions,
        palette: &palette,
        identity: Identity::default(),
        world: project::world_extent(worlds.first().ok_or("World shard")?)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &families,
    };
    let (table, report) = realize_spawns(&input, &registry)?;
    assert!(report.dropped.is_empty(), "{report:#?}");
    assert_eq!(table.sources.len(), 1);
    assert_eq!(table.point_count(), 2);
    assert_eq!(table.weighted.len(), 1);
    let source = &table.sources[0];
    assert_eq!(source.floor, -9);
    assert_eq!(
        (
            source.points[0].x,
            source.points[0].y,
            source.points[0].respawn_ms
        ),
        (33866, 32698, 47_104_000)
    );
    assert_eq!(
        (
            source.points[1].x,
            source.points[1].y,
            source.points[1].respawn_ms
        ),
        (33866, 32699, 60_000)
    );
    let choices: Vec<_> = table.weighted[0]
        .choices
        .iter()
        .map(|choice| {
            (
                table.creatures[choice.creature as usize].key.as_str(),
                choice.weight,
            )
        })
        .collect();
    assert_eq!(
        choices,
        vec![
            ("oteryn:creature.silverfrost_sentinel", 25),
            ("oteryn:creature.true_frost_flower_asura", 25)
        ]
    );
    assert_eq!(
        oteryn_world_bundle_compiler::spawn::decode(
            &oteryn_world_bundle_compiler::spawn::encode(&table),
            &input.world
        )?,
        table
    );
    Ok(())
}
