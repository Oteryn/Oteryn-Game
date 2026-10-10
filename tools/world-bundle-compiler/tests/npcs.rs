//! NPC-PLACE-1b: the NPC family of the bundle (format v4, decision NPC-PLACE-1 §5, §6, §8):
//! one placement held per reason, one travel destination held per reason, the production stop,
//! the manifest member and the equivalence proof.

use std::error::Error as StdError;

use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::bundle::{
    self, BuildClass, Extent, Family, Identity, Terrain, TerrainKind,
};
use oteryn_world_bundle_compiler::compile::{
    HoldReason, Input, KeyResolver, Resolution, admitted_npcs, compile, equivalence, realize_npcs,
    realize_spawns,
};
use oteryn_world_bundle_compiler::npc;
use oteryn_world_bundle_compiler::project::{
    CreatureFacts, Families, NpcFacts, NpcPlacement, Reference, SpawnPoint, SpawnSource,
    TravelRoute,
};
use oteryn_world_bundle_compiler::sector::{self, Attrs, Item, Tile};
use oteryn_world_bundle_compiler::spawn::{self, Direction};

type TestResult = Result<(), Box<dyn StdError>>;

struct Resolver;

const PALETTE: [&str; 8] = [
    "terrain:grass",
    "terrain:water",
    "item:stairs",
    "item:portal",
    "terrain:mystery",
    "terrain:wall",
    "item:crate",
    "item:rug",
];

impl KeyResolver for Resolver {
    fn resolve(&self, key: &str) -> Resolution {
        match PALETTE.iter().position(|k| *k == key) {
            Some(i) if key.starts_with("terrain:") => {
                Resolution::Resolved(Family::Terrain, i as u32)
            }
            Some(i) => Resolution::Resolved(Family::Item, i as u32),
            None => Resolution::Unknown,
        }
    }

    fn terrain(&self, key: &str) -> Result<Option<Terrain>, Error> {
        let ground = |walkable: bool| Terrain {
            kind: TerrainKind::Ground,
            walkable: Some(walkable),
            ground_speed: Some(if walkable { 150 } else { 0 }),
        };
        match key {
            "terrain:grass" => Ok(Some(ground(true))),
            "terrain:water" => Ok(Some(ground(false))),
            "terrain:wall" => Ok(Some(Terrain {
                kind: TerrainKind::Wall,
                walkable: None,
                ground_speed: None,
            })),
            "terrain:mystery" => Err(Error::Key("unclassified".into())),
            _ => Ok(None),
        }
    }

    fn floor_change(&self, key: &str) -> bool {
        key == "item:stairs"
    }

    fn solid(&self, key: &str) -> bool {
        key == "item:crate"
    }
}

fn palette() -> Vec<String> {
    PALETTE.map(String::from).to_vec()
}

fn item(palette: u32, depth: u8) -> Item {
    Item {
        palette,
        depth,
        attrs: Attrs::default(),
    }
}

fn tile(x: u16, items: Vec<Item>, house: u32) -> Tile {
    Tile {
        x,
        y: 1,
        flags: 0,
        house,
        zones: Vec::new(),
        items,
    }
}

fn region(z: u8, tiles: &[Tile]) -> Result<Vec<u8>, Box<dyn StdError>> {
    let frame = zstd::bulk::compress(&sector::encode(tiles)?, 3)?;
    let mut out = b"OTRB".to_vec();
    out.extend_from_slice(&[1, z]);
    for value in [0u16, 0, 1] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.push(0);
    out.extend_from_slice(&21u32.to_le_bytes());
    out.extend_from_slice(&(frame.len() as u32).to_le_bytes());
    out.extend_from_slice(&frame);
    Ok(out)
}

const GRASS: u32 = 0;

/// Row y = 1 of floor 7: x=1 grass, 2 water, 3 grass and stairs, 5 grass and a teleport, 6 stairs
/// without ground, 7 no tile, 8 unclassified, 9 grass under a wall, 10 grass under a crate, 11
/// grass with a crate below a rug, 12..=19 grass, 14 and 19 house tiles. A whole compilation
/// refuses the unclassified terrain (`KeyResolver::terrain` fails), so only [`realize_npcs`]
/// alone sees x=8 (`unclassified`).
fn regions(unclassified: bool) -> Result<Vec<Vec<u8>>, Box<dyn StdError>> {
    let mut portal = item(3, 1);
    portal.attrs.teleport = Some((300, 40, 6));
    let mut tiles = vec![
        tile(1, vec![item(GRASS, 0)], 0),
        tile(2, vec![item(1, 0)], 0),
        tile(3, vec![item(GRASS, 0), item(2, 1)], 0),
        tile(5, vec![item(GRASS, 0), portal], 0),
        tile(6, vec![item(2, 0)], 0),
        tile(9, vec![item(GRASS, 0), item(5, 0)], 0),
        tile(10, vec![item(GRASS, 0), item(6, 0)], 0),
        // A crate inside a container (depth 1) is not a top-level entry and does not block.
        tile(11, vec![item(GRASS, 0), item(7, 0), item(6, 1)], 0),
    ];
    if unclassified {
        tiles.push(tile(8, vec![item(GRASS, 0), item(4, 0)], 0));
    }
    for x in 12..=19 {
        tiles.push(tile(x, vec![item(GRASS, 0)], u32::from(x == 14 || x == 19)));
    }
    tiles.sort_by_key(|t| t.x);
    Ok(vec![region(7, &tiles)?])
}

fn reference(family: &str, key: &str) -> Reference {
    Reference {
        family: family.into(),
        key: key.into(),
        revision: "definition-r1".into(),
    }
}

fn placement(key: &str, npc: &str, x: u16) -> NpcPlacement {
    NpcPlacement {
        key: format!("oteryn:npc_placement.{key}"),
        npc: format!("oteryn:npc.{npc}"),
        cell: (x, 1, 7),
        direction: Direction::South,
    }
}

fn route(service: &str, route: &str, x: u16) -> TravelRoute {
    TravelRoute {
        service: format!("oteryn:service.travel.{service}"),
        route: route.into(),
        destination: (x, 1, 7),
    }
}

/// NPCs: `sam` and `tom` admitted, `quiet` with a missing Dialogue, `trader` with a missing
/// Service; `ghost` has no definition.
fn npcs() -> Families {
    let mut families = Families::default();
    let dialogue = reference("Dialogue", "oteryn:dialogue.greeting");
    let service = reference("Service", "oteryn:service.shop.sam");
    families.dialogues.insert(dialogue.clone());
    families.services.insert(service.clone());
    families.teleports.insert((5, 1, 7), (300, 40, 6));
    families.houses.insert(1);
    let facts =
        |dialogue: Option<Reference>, services: Vec<Reference>| NpcFacts { dialogue, services };
    for (name, facts) in [
        ("sam", facts(Some(dialogue.clone()), vec![service.clone()])),
        ("tom", facts(None, Vec::new())),
        (
            "quiet",
            facts(
                Some(reference("Dialogue", "oteryn:dialogue.none")),
                Vec::new(),
            ),
        ),
        (
            "trader",
            facts(None, vec![reference("Service", "oteryn:service.shop.none")]),
        ),
    ] {
        families.npcs.insert(format!("oteryn:npc.{name}"), facts);
    }
    families
}

fn families() -> Families {
    let mut families = npcs();
    families.creatures.insert(
        "oteryn:creature.rat".into(),
        CreatureFacts {
            period: "All".into(),
            boss: false,
            encounter_bound: false,
        },
    );
    families.spawns = vec![SpawnSource {
        key: "oteryn:spawn.a".into(),
        centre: (12, 1, 7),
        points: vec![SpawnPoint {
            creature: "oteryn:creature.rat".into(),
            cell: (12, 1, 7),
            direction: Direction::South,
            respawn_ms: 60_000,
        }],
    }];
    // Authored out of key order on purpose: the bundle sorts by NPC, then position.
    families.npc_placements = vec![
        placement("tom-2", "tom", 15),
        placement("sam-1", "sam", 1),
        placement("ghost", "ghost", 16),
        placement("quiet", "quiet", 17),
        placement("trader", "trader", 18),
        placement("water", "tom", 2),
        placement("stairs", "tom", 3),
        placement("portal", "tom", 5),
        placement("no-ground", "tom", 6),
        placement("no-tile", "tom", 7),
        placement("mystery", "tom", 8),
        placement("wall", "tom", 9),
        placement("crate", "tom", 10),
        placement("rug", "tom", 11),
        placement("spawn", "tom", 12),
        placement("shared-a", "tom", 13),
        placement("shared-b", "sam", 13),
        placement("house", "sam", 14),
    ];
    families.routes = vec![
        route("boat", "to-sam", 1),
        route("boat", "to-house", 19),
        route("boat", "to-spawn", 12),
        route("boat", "to-wall", 9),
        route("boat", "to-crate", 10),
        route("boat", "to-water", 2),
        route("boat", "to-ghost", 16),
        // Two Services with a route of the same key: only the held pair is listed.
        route("cart", "same", 17),
        route("ship", "same", 6),
    ];
    families
}

fn world() -> Extent {
    Extent {
        min_x: 0,
        min_y: 0,
        max_x: 512,
        max_y: 256,
        floors: vec![-7, -6],
    }
}

fn input<'a>(
    regions: &'a [Vec<u8>],
    palette: &'a [String],
    families: &'a Families,
    build_class: BuildClass,
) -> Input<'a> {
    Input {
        regions,
        palette,
        identity: Identity::default(),
        world: world(),
        build_class,
        draft_areas: Vec::new(),
        families,
    }
}

fn realize(
    input: &Input<'_>,
) -> Result<(npc::Table, oteryn_world_bundle_compiler::compile::NpcReport), Error> {
    let (spawns, _) = realize_spawns(input, &Resolver)?;
    realize_npcs(input, &Resolver, &spawns)
}

#[test]
fn each_placement_is_written_or_held_with_its_reason() -> TestResult {
    let (regions, palette, families) = (regions(true)?, palette(), families());
    let (table, report) = realize(&input(
        &regions,
        &palette,
        &families,
        BuildClass::NonProduction,
    ))?;
    let held: Vec<_> = report
        .held
        .iter()
        .map(|h| {
            (
                h.key.trim_start_matches("oteryn:npc_placement."),
                h.cell.0,
                h.reason,
            )
        })
        .collect();
    assert_eq!(
        held,
        [
            ("ghost", 16, HoldReason::UnboundNpc),
            ("quiet", 17, HoldReason::UnresolvedDialogue),
            ("trader", 18, HoldReason::UnresolvedService),
            ("water", 2, HoldReason::NotWalkable),
            ("stairs", 3, HoldReason::FloorChange),
            ("portal", 5, HoldReason::Teleport),
            ("no-ground", 6, HoldReason::NoGround),
            ("no-tile", 7, HoldReason::NoTile),
            ("mystery", 8, HoldReason::UnclassifiedTerrain),
            ("wall", 9, HoldReason::Wall),
            ("crate", 10, HoldReason::BlockSolid),
            ("spawn", 12, HoldReason::SpawnPoint),
            ("shared-a", 13, HoldReason::SharedCell),
            ("shared-b", 13, HoldReason::SharedCell),
        ]
    );
    assert_eq!((report.placements, report.written), (18, 4));
    // Sorted by NPC key, then (floor, y, x); a house tile is no placement reason.
    let written: Vec<_> = table
        .npcs
        .iter()
        .map(|n| {
            (
                n.key.as_str(),
                n.placements
                    .iter()
                    .map(|p| (p.floor, p.x))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(
        written,
        [
            ("oteryn:npc.sam", vec![(-7, 1), (-7, 14)]),
            ("oteryn:npc.tom", vec![(-7, 11), (-7, 15)]),
        ]
    );
    assert_eq!(
        admitted_npcs(&families),
        ["oteryn:npc.sam", "oteryn:npc.tom"]
    );
    Ok(())
}

#[test]
fn each_travel_destination_is_held_with_its_reason_by_service_and_route() -> TestResult {
    let (regions, palette, families) = (regions(true)?, palette(), families());
    let (_, report) = realize(&input(
        &regions,
        &palette,
        &families,
        BuildClass::NonProduction,
    ))?;
    let held: Vec<_> = report
        .routes_held
        .iter()
        .map(|r| {
            (
                r.service.trim_start_matches("oteryn:service.travel."),
                r.route.as_str(),
                r.reason,
            )
        })
        .collect();
    assert_eq!(
        held,
        [
            ("boat", "to-sam", HoldReason::NpcPlacement),
            ("boat", "to-house", HoldReason::HouseTile),
            ("boat", "to-spawn", HoldReason::SpawnPoint),
            ("boat", "to-wall", HoldReason::Wall),
            ("boat", "to-crate", HoldReason::BlockSolid),
            ("boat", "to-water", HoldReason::NotWalkable),
            ("ship", "same", HoldReason::NoGround),
        ]
    );
    // A held placement creates no actor and is not a reason: `to-ghost` (x=16) and `cart/same`
    // (x=17) stand on held placements and are admitted.
    assert_eq!(report.routes, 9);
    Ok(())
}

#[test]
fn the_manifest_lists_held_records_and_the_bundle_proves_equivalent() -> TestResult {
    let (regions, palette, families) = (regions(false)?, palette(), families());
    let input = input(&regions, &palette, &families, BuildClass::NonProduction);
    let first = compile(&input, &Resolver)?;
    assert_eq!(first.bytes, compile(&input, &Resolver)?.bytes);
    let read = bundle::read(&first.bytes)?;
    let counts = &read.manifest.npcs;
    assert_eq!((counts.npcs, counts.placements), (2, 4));
    assert_eq!(read.npcs.placement_count(), 4);
    assert_eq!(counts.held.len(), 14);
    assert!(counts.held.windows(2).all(|p| p[0] < p[1]));
    let pair = |s: &str, r: &str| [format!("oteryn:service.travel.{s}"), r.to_owned()];
    assert_eq!(
        counts.routes_held,
        [
            pair("boat", "to-crate"),
            pair("boat", "to-house"),
            pair("boat", "to-sam"),
            pair("boat", "to-spawn"),
            pair("boat", "to-wall"),
            pair("boat", "to-water"),
            pair("ship", "same"),
        ]
    );
    // No catalogue directory was read: the empty catalogue's digest.
    assert_eq!(
        counts.catalogue_sha256,
        npc::catalogue_digest::<&str, &[u8]>(&[])
    );
    equivalence(&input, &Resolver, &first.bytes)?;
    // Another placement family gives another bundle and no longer proves equivalent.
    let mut other = families.clone();
    other.npc_placements.retain(|p| !p.key.ends_with("rug"));
    let changed = compile(
        &self::input(&regions, &palette, &other, BuildClass::NonProduction),
        &Resolver,
    )?;
    assert_ne!(changed.bytes, first.bytes);
    assert!(equivalence(&input, &Resolver, &changed.bytes).is_err());
    // So does another catalogue digest.
    let mut digest = families.clone();
    digest.catalogue_sha256 = Some("ab".repeat(32));
    let changed = compile(
        &self::input(&regions, &palette, &digest, BuildClass::NonProduction),
        &Resolver,
    )?;
    assert_eq!(
        bundle::read(&changed.bytes)?.manifest.npcs.catalogue_sha256,
        "ab".repeat(32)
    );
    assert!(equivalence(&input, &Resolver, &changed.bytes).is_err());
    Ok(())
}

#[test]
fn a_production_build_stops_on_a_held_placement_or_route() -> TestResult {
    let (regions, palette) = (regions(false)?, palette());
    let mut clean = npcs();
    clean.npc_placements = vec![placement("sam-1", "sam", 1)];
    clean.routes = vec![route("boat", "ok", 15)];
    let built = compile(
        &input(&regions, &palette, &clean, BuildClass::Production),
        &Resolver,
    )?;
    let read = bundle::read(&built.bytes)?;
    assert_eq!(read.manifest.npcs.placements, 1);
    assert!(read.manifest.npcs.held.is_empty() && read.manifest.npcs.routes_held.is_empty());
    let mut service = clean.clone();
    service
        .npc_placements
        .push(placement("trader", "trader", 18));
    let result = compile(
        &input(&regions, &palette, &service, BuildClass::Production),
        &Resolver,
    );
    assert!(matches!(result, Err(Error::Family(_))));
    assert!(
        compile(
            &input(&regions, &palette, &service, BuildClass::NonProduction),
            &Resolver
        )
        .is_ok()
    );
    let mut route = clean.clone();
    route.routes.push(self::route("boat", "onto-sam", 1));
    let result = compile(
        &input(&regions, &palette, &route, BuildClass::Production),
        &Resolver,
    );
    assert!(matches!(result, Err(Error::Family(_))));
    Ok(())
}

#[test]
fn the_reference_packet_cases_hold() -> TestResult {
    let (regions, palette) = (regions(false)?, palette());
    let mut families = npcs();
    families.npc_placements = vec![
        // An exact admitted NPC key is written.
        placement("exact", "sam", 1),
        // Another or no NPC is held `UnboundNpc`.
        placement("other", "Sam", 12),
        placement("none", "", 15),
        // Two placements on one cell are held `SharedCell`.
        placement("one", "tom", 13),
        placement("two", "sam", 13),
    ];
    let (table, report) = realize_npcs(
        &input(&regions, &palette, &families, BuildClass::NonProduction),
        &Resolver,
        &spawn::Table::default(),
    )?;
    assert_eq!(table.placement_count(), 1);
    assert_eq!(table.npcs[0].key, "oteryn:npc.sam");
    let reasons: Vec<_> = report.held.iter().map(|h| h.reason).collect();
    assert_eq!(
        reasons,
        [
            HoldReason::UnboundNpc,
            HoldReason::UnboundNpc,
            HoldReason::SharedCell,
            HoldReason::SharedCell,
        ]
    );
    Ok(())
}

#[test]
fn positions_outside_the_world_and_duplicate_keys_fail_compilation() -> TestResult {
    let (regions, palette) = (regions(false)?, palette());
    let fails = |families: &Families| {
        realize_npcs(
            &input(&regions, &palette, families, BuildClass::NonProduction),
            &Resolver,
            &spawn::Table::default(),
        )
        .err()
    };
    let mut families = npcs();
    families.npc_placements = vec![placement("far", "sam", 600)];
    assert!(matches!(fails(&families), Some(Error::Bounds(_))));
    families.npc_placements = vec![placement("high", "sam", 1)];
    families.npc_placements[0].cell.2 = 5;
    assert!(matches!(fails(&families), Some(Error::Bounds(_))));
    families.npc_placements = Vec::new();
    families.routes = vec![route("boat", "far", 600)];
    assert!(matches!(fails(&families), Some(Error::Bounds(_))));
    families.routes = Vec::new();
    families.npc_placements = vec![placement("a", "sam", 1), placement("a", "tom", 15)];
    assert!(matches!(fails(&families), Some(Error::Format(_))));
    Ok(())
}
