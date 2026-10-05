//! SPAWN-CONTENT-1: the spawn family of the bundle (format v3, CREATURE-AI-0 §6.1): cell
//! admission, boss and encounter routing, bounds at their maximum and one past it, and byte
//! reproducibility.

use std::error::Error as StdError;

use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::bundle::{
    self, BuildClass, Extent, Family, Identity, Terrain, TerrainKind,
};
use oteryn_world_bundle_compiler::compile::{
    Input, KeyResolver, Resolution, SpawnReason, compile, equivalence, realize_spawns,
};
use oteryn_world_bundle_compiler::project::{CreatureFacts, Families, SpawnPoint, SpawnSource};
use oteryn_world_bundle_compiler::sector::{self, Attrs, Item, Tile};
use oteryn_world_bundle_compiler::spawn::{self, Direction, Period};

type TestResult = Result<(), Box<dyn StdError>>;

struct Resolver;

impl KeyResolver for Resolver {
    fn resolve(&self, key: &str) -> Resolution {
        match key {
            "terrain:grass" => Resolution::Resolved(Family::Terrain, 1),
            "terrain:water" => Resolution::Resolved(Family::Terrain, 2),
            "item:stairs" => Resolution::Resolved(Family::Item, 3),
            "item:portal" => Resolution::Resolved(Family::Item, 4),
            _ => Resolution::Unknown,
        }
    }

    fn terrain(&self, key: &str) -> Result<Option<Terrain>, Error> {
        let walkable = match key {
            "terrain:grass" => true,
            "terrain:water" => false,
            _ => return Ok(None),
        };
        Ok(Some(Terrain {
            kind: TerrainKind::Ground,
            walkable: Some(walkable),
            ground_speed: walkable.then_some(150).or(Some(0)),
        }))
    }

    fn floor_change(&self, key: &str) -> bool {
        key == "item:stairs"
    }
}

fn palette() -> Vec<String> {
    [
        "terrain:grass",
        "terrain:water",
        "item:stairs",
        "item:portal",
    ]
    .map(String::from)
    .to_vec()
}

fn item(palette: u32) -> Item {
    Item {
        palette,
        depth: 0,
        attrs: Attrs::default(),
    }
}

fn tile(x: u16, items: Vec<Item>, flags: u32) -> Tile {
    Tile {
        x,
        y: 1,
        flags,
        house: 0,
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

/// Row y = 1 of floor 7: x=1 grass, 2 water, 3 grass and stairs, 4 grass in a protection zone,
/// 5 grass and a teleport, 6 stairs without ground, 7 no tile, 8..=12 grass.
fn regions() -> Result<Vec<Vec<u8>>, Box<dyn StdError>> {
    let mut portal = item(3);
    portal.attrs.teleport = Some((300, 40, 6));
    let tiles = vec![
        tile(1, vec![item(0)], 0),
        tile(2, vec![item(1)], 0),
        tile(3, vec![item(0), item(2)], 0),
        tile(4, vec![item(0)], 1),
        tile(5, vec![item(0), portal], 0),
        tile(6, vec![item(2)], 0),
        tile(8, vec![item(0)], 0),
        tile(9, vec![item(0)], 0),
        tile(10, vec![item(0)], 0),
        tile(11, vec![item(0)], 0),
        tile(12, vec![item(0)], 0),
    ];
    Ok(vec![region(7, &tiles)?])
}

fn creature(period: &str, boss: bool, encounter_bound: bool) -> CreatureFacts {
    CreatureFacts {
        period: period.into(),
        boss,
        encounter_bound,
    }
}

fn point(creature: &str, x: u16) -> SpawnPoint {
    SpawnPoint {
        creature: format!("oteryn:creature.{creature}"),
        cell: (x, 1, 7),
        direction: Direction::South,
        respawn_ms: 60_000,
    }
}

fn families() -> Families {
    let mut families = Families::default();
    families.teleports.insert((5, 1, 7), (300, 40, 6));
    for (name, facts) in [
        ("rat", creature("All", false, false)),
        ("bat", creature("Night", false, false)),
        ("dragonlord", creature("All", true, false)),
        ("guard", creature("All", false, true)),
    ] {
        families
            .creatures
            .insert(format!("oteryn:creature.{name}"), facts);
    }
    let source = |key: &str, centre: u16, points: Vec<SpawnPoint>| SpawnSource {
        key: format!("oteryn:spawn.{key}"),
        centre: (centre, 1, 7),
        points,
    };
    // Authored out of key order on purpose: the bundle sorts by key.
    families.spawns = vec![
        source(
            "b",
            9,
            vec![
                point("dragonlord", 8),
                point("guard", 9),
                point("ghost", 10),
                point("bat", 11),
                point("rat", 12),
            ],
        ),
        source("c", 2, vec![point("rat", 2), point("rat", 7)]),
        source("a", 3, (1..=7).map(|x| point("rat", x)).collect::<Vec<_>>()),
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

fn input<'a>(regions: &'a [Vec<u8>], palette: &'a [String], families: &'a Families) -> Input<'a> {
    Input {
        regions,
        palette,
        identity: Identity::default(),
        world: world(),
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families,
    }
}

#[test]
fn points_are_admitted_dropped_or_routed_by_their_cell_and_creature() -> TestResult {
    let (regions, palette, families) = (regions()?, palette(), families());
    let (table, report) = realize_spawns(&input(&regions, &palette, &families), &Resolver)?;
    let dropped: Vec<_> = report
        .dropped
        .iter()
        .map(|d| (d.source.as_str(), d.cell.0, d.reason))
        .collect();
    assert_eq!(
        dropped,
        [
            ("oteryn:spawn.b", 8, SpawnReason::Boss),
            ("oteryn:spawn.b", 9, SpawnReason::EncounterBound),
            ("oteryn:spawn.b", 10, SpawnReason::UnboundCreature),
            ("oteryn:spawn.c", 2, SpawnReason::NotWalkable),
            ("oteryn:spawn.c", 7, SpawnReason::NoTile),
            ("oteryn:spawn.a", 2, SpawnReason::NotWalkable),
            ("oteryn:spawn.a", 3, SpawnReason::FloorChange),
            ("oteryn:spawn.a", 4, SpawnReason::ProtectionZone),
            ("oteryn:spawn.a", 5, SpawnReason::Teleport),
            ("oteryn:spawn.a", 6, SpawnReason::NoGround),
            ("oteryn:spawn.a", 7, SpawnReason::NoTile),
        ]
    );
    assert_eq!((report.sources, report.points), (3, 14));
    // Source c has no point left and is not written; the rest is sorted by key.
    assert_eq!((report.realized_sources, report.realized_points), (2, 3));
    assert_eq!(report.inactive_points, 1);
    let keys: Vec<_> = table.sources.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["oteryn:spawn.a", "oteryn:spawn.b"]);
    let creatures: Vec<_> = table
        .creatures
        .iter()
        .map(|c| (&c.key[..], c.period))
        .collect();
    assert_eq!(
        creatures,
        [
            ("oteryn:creature.bat", Period::Night),
            ("oteryn:creature.rat", Period::All)
        ]
    );
    // Source a keeps the one rat on x=1; source b keeps its bat and its rat, in order.
    let b = &table.sources[1];
    assert_eq!(b.floor, -7);
    assert_eq!(
        b.points
            .iter()
            .map(|p| (p.creature, p.x))
            .collect::<Vec<_>>(),
        [(0, 11), (1, 12)]
    );
    assert_eq!(report.count(SpawnReason::Boss), 1);
    Ok(())
}

#[test]
fn the_bundle_holds_the_family_byte_reproducibly_and_proves_equivalent() -> TestResult {
    let (regions, palette, families) = (regions()?, palette(), families());
    let input = input(&regions, &palette, &families);
    let first = compile(&input, &Resolver)?;
    assert_eq!(first.bytes, compile(&input, &Resolver)?.bytes);
    assert_eq!(first.spawns.realized_points, 3);
    let read = bundle::read(&first.bytes)?;
    assert_eq!(
        (read.manifest.spawns.sources, read.manifest.spawns.points),
        (2, 3)
    );
    assert_eq!(read.spawns.point_count(), 3);
    equivalence(&input, &Resolver, &first.bytes)?;
    // Another family gives another bundle and no longer proves equivalent.
    let mut other = families.clone();
    other.spawns.pop();
    let changed = compile(&self::input(&regions, &palette, &other), &Resolver)?;
    assert_ne!(changed.bytes, first.bytes);
    assert!(equivalence(&input, &Resolver, &changed.bytes).is_err());
    // The writer refuses counts that are not the table's.
    let mut manifest = read.manifest.clone();
    manifest.spawns.points += 1;
    assert!(bundle::write(&manifest, &read.sectors, &read.spawns).is_err());
    Ok(())
}

#[test]
fn a_duplicate_source_key_fails_compilation() -> TestResult {
    let (regions, palette, mut families) = (regions()?, palette(), families());
    families.spawns.push(families.spawns[0].clone());
    let result = realize_spawns(&input(&regions, &palette, &families), &Resolver);
    assert!(matches!(result, Err(Error::Format(_))));
    Ok(())
}

fn table(sources: usize, points: usize) -> spawn::Table {
    spawn::Table {
        creatures: vec![spawn::Creature {
            key: "oteryn:creature.rat".into(),
            period: Period::All,
        }],
        sources: (0..sources)
            .map(|i| spawn::Source {
                key: format!("s{i:06}"),
                floor: -7,
                x: 10,
                y: 10,
                points: (0..points)
                    .map(|j| spawn::Point {
                        creature: 0,
                        x: 10 + (j % 8) as u16,
                        y: 10,
                        direction: Direction::North,
                        respawn_ms: spawn::MIN_RESPAWN_MS,
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn limit(table: &spawn::Table) -> bool {
    matches!(spawn::validate(table, &world()), Err(Error::Limit(_)))
}

#[test]
fn bounds_hold_at_their_maximum_and_refuse_one_more() -> TestResult {
    // CREATUREAI0-RL-03: points per source.
    assert!(spawn::validate(&table(1, spawn::MAX_POINTS_PER_SOURCE), &world()).is_ok());
    assert!(limit(&table(1, spawn::MAX_POINTS_PER_SOURCE + 1)));
    // CREATUREAI0-RL-02: sources.
    let at_max = table(spawn::MAX_SOURCES, 1);
    assert!(spawn::validate(&at_max, &world()).is_ok());
    let mut over = at_max.clone();
    over.sources.push(spawn::Source {
        key: "z".into(),
        ..at_max.sources[0].clone()
    });
    assert!(limit(&over));
    // CREATUREAI0-RL-01: points, with the full-size table round-tripping inside the raw cap.
    let full = table(
        spawn::MAX_POINTS / spawn::MAX_POINTS_PER_SOURCE,
        spawn::MAX_POINTS_PER_SOURCE,
    );
    assert_eq!(full.point_count(), spawn::MAX_POINTS);
    assert!(spawn::validate(&full, &world()).is_ok());
    let raw = spawn::encode(&full);
    assert!(raw.len() <= spawn::MAX_RAW_BYTES);
    assert_eq!(spawn::decode(&raw, &world())?, full);
    let mut over = full.clone();
    over.sources.push(spawn::Source {
        key: "z".into(),
        points: full.sources[0].points[..1].to_vec(),
        ..full.sources[0].clone()
    });
    assert!(limit(&over));
    assert!(matches!(
        spawn::decode(&spawn::encode(&over), &world()),
        Err(Error::Limit(_))
    ));
    // CREATUREAI0-RL-13: respawn delay.
    for (ms, ok) in [
        (spawn::MIN_RESPAWN_MS - 1, false),
        (spawn::MIN_RESPAWN_MS, true),
        (spawn::MAX_RESPAWN_MS, true),
        (spawn::MAX_RESPAWN_MS + 1, false),
    ] {
        let mut t = table(1, 1);
        t.sources[0].points[0].respawn_ms = ms;
        assert_eq!(spawn::validate(&t, &world()).is_ok(), ok, "{ms}");
    }
    Ok(())
}

fn shard(points: usize, respawn_ms: u32) -> Vec<u8> {
    let points: Vec<_> = (0..points)
        .map(|_| {
            serde_json::json!({
                "cell": {"floor": 7, "x": 1, "y": 1},
                "creature": "oteryn:creature.rat",
                "direction": "north",
                "respawn_ms": respawn_ms,
            })
        })
        .collect();
    serde_json::json!({
        "coordinate_frame": "global-target-2026-09-27",
        "family": "Spawn.Source",
        "records": [{"declaration": {
            "centre": {"floor": 7, "x": 1, "y": 1},
            "identity": {"key": "oteryn:spawn.x"},
            "points": points,
        }}],
    })
    .to_string()
    .into_bytes()
}

#[test]
fn the_source_shard_checks_its_bounds_before_it_holds_anything() {
    let add = |points, ms| Families::default().add_spawns(&shard(points, ms));
    assert!(add(spawn::MAX_POINTS_PER_SOURCE, 1_000).is_ok());
    assert!(matches!(
        add(spawn::MAX_POINTS_PER_SOURCE + 1, 1_000),
        Err(Error::Limit(_))
    ));
    assert!(add(1, spawn::MAX_RESPAWN_MS).is_ok());
    assert!(matches!(
        add(1, spawn::MAX_RESPAWN_MS + 1),
        Err(Error::Limit(_))
    ));
    assert!(matches!(add(1, 999), Err(Error::Limit(_))));
    assert!(add(0, 1_000).is_err());
}
