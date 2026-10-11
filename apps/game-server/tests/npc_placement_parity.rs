//! NPC-PLACE-1b: the compiler's NPC placement checks agree with the game server. Its admitted
//! NPCs are the server's NPC service model, a written placement is a cell the booted World lets
//! a creature enter (and a cell-held one is not), and the bundle carries the catalogue digest the
//! server's reader computes (decision NPC-PLACE-1 §3.1, §5, §7).
#![cfg(target_os = "linux")]

use std::collections::BTreeSet;
use std::error::Error as StdError;
use std::ffi::OsStr;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use oteryn_game_server::content::{
    NpcDataCatalogue, capture_world_project, npc_catalogue_preproduction_limits,
};
use oteryn_game_server::foundation::{ChannelId, WorldId};
use oteryn_game_server::map::BundlePins;
use oteryn_game_server::map::boot::{BootPins, boot};
use oteryn_game_server::map::facts::ItemDefinition;
use oteryn_game_server::map::overlay::TilePos;
use oteryn_world_bundle_compiler::Error;
use oteryn_world_bundle_compiler::bundle::{
    self, BuildClass, Extent, Family, Identity, Terrain, TerrainKind,
};
use oteryn_world_bundle_compiler::compile::{
    HoldReason, Input, KeyResolver, Resolution, admitted_npcs, compile,
};
use oteryn_world_bundle_compiler::project::{Families, NpcFacts, NpcPlacement};
use oteryn_world_bundle_compiler::sector::{self, Attrs, Item, Tile};
use oteryn_world_bundle_compiler::spawn::Direction;
use serde_json::Value;

type TestResult = Result<(), Box<dyn StdError>>;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The shards of the `OTERYN_FAMILY_INDEX/v1` index of `directory`, in index order, as the
/// compiler reads them; none for another schema (a tree directory marker).
fn shards(root: &Path, directory: &str) -> Result<Vec<Vec<u8>>, Box<dyn StdError>> {
    let index: Value =
        serde_json::from_slice(&std::fs::read(root.join(directory).join("index.json"))?)?;
    let mut out = Vec::new();
    if index["schema"] != "OTERYN_FAMILY_INDEX/v1" {
        return Ok(out);
    }
    for shard in index["shards"].as_array().ok_or("index has no shards")? {
        out.push(std::fs::read(
            root.join(shard.as_str().ok_or("shard path")?),
        )?);
    }
    Ok(out)
}

/// The repository's NPC, Dialogue and Service families, read as the compiler reads them.
fn repository_families() -> Result<Families, Box<dyn StdError>> {
    let root = root();
    let mut families = Families::default();
    for shard in shards(&root, "content/npcs/definitions/")? {
        families.add_npcs(&shard)?;
    }
    for shard in shards(&root, "content/dialogues/definitions/")? {
        families.add_dialogues(&shard)?;
    }
    let mut kinds: Vec<_> = std::fs::read_dir(root.join("content/services"))?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()?;
    kinds.sort();
    for kind in kinds {
        if root
            .join("content/services")
            .join(&kind)
            .join("index.json")
            .is_file()
        {
            for shard in shards(&root, &format!("content/services/{kind}/"))? {
                families.add_services(&shard)?;
            }
        }
    }
    Ok(families)
}

#[test]
fn the_compiler_admits_exactly_the_npcs_of_the_server_service_model() -> TestResult {
    let world = root().join("content/world");
    let project = capture_world_project(
        world.parent().ok_or("content")?,
        OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )?;
    let catalogue =
        NpcDataCatalogue::from_project(project, npc_catalogue_preproduction_limits().project)?;
    let model = catalogue.service_model_for_project()?;
    let server: BTreeSet<&str> = model.npcs().keys().map(|npc| npc.key.as_str()).collect();
    let families = repository_families()?;
    let compiler: BTreeSet<&str> = admitted_npcs(&families).into_iter().collect();
    assert!(!server.is_empty());
    assert_eq!(compiler, server);
    Ok(())
}

const PALETTE: [&str; 5] = [
    "terrain:grass",
    "terrain:lava",
    "terrain:wall",
    "item:box",
    "item:coin",
];

/// The served definitions of the fixture's Items; the §1.6 reference is 1 + the palette `id`.
fn item(key: &str) -> Option<ItemDefinition> {
    let (id, solid) = match key {
        "item:box" => (3, true),
        "item:coin" => (4, false),
        _ => return None,
    };
    Some(ItemDefinition {
        reference: NonZeroU32::new(id + 1)?,
        solid: Some(solid),
        blocks_projectile: solid,
        pickupable: !solid,
    })
}

/// The compiler's view of the same facts: its solidity is the server's served definition.
struct Resolver;

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
        let ground = |walkable| Terrain {
            kind: TerrainKind::Ground,
            walkable: Some(walkable),
            ground_speed: Some(150),
        };
        Ok(match key {
            "terrain:grass" => Some(ground(true)),
            "terrain:lava" => Some(ground(false)),
            "terrain:wall" => Some(Terrain {
                kind: TerrainKind::Wall,
                walkable: None,
                ground_speed: None,
            }),
            _ => None,
        })
    }

    fn floor_change(&self, _: &str) -> bool {
        false
    }

    fn solid(&self, key: &str) -> bool {
        item(key).and_then(|item| item.solid).unwrap_or(true)
    }
}

/// One row y = 0 of legacy floor 7 (native -7): grass at x 1, lava at 2, grass under a wall at
/// 3, grass under a box at 4, grass under a coin at 5, no tile at 6, grass at 7.
fn region() -> Result<Vec<u8>, Box<dyn StdError>> {
    let tile = |x, palettes: &[u32]| Tile {
        x,
        y: 0,
        flags: 0,
        house: 0,
        zones: Vec::new(),
        items: palettes
            .iter()
            .map(|palette| Item {
                palette: *palette,
                depth: 0,
                attrs: Attrs::default(),
            })
            .collect(),
    };
    let tiles = [
        tile(1, &[0]),
        tile(2, &[1]),
        tile(3, &[0, 2]),
        tile(4, &[0, 3]),
        tile(5, &[0, 4]),
        tile(7, &[0]),
    ];
    let frame = zstd::bulk::compress(&sector::encode(&tiles)?, 3)?;
    let mut out = b"OTRB".to_vec();
    out.extend_from_slice(&[1, 7]);
    for value in [0u16, 0, 1] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.push(0);
    out.extend_from_slice(&21u32.to_le_bytes());
    out.extend_from_slice(&(frame.len() as u32).to_le_bytes());
    out.extend_from_slice(&frame);
    Ok(out)
}

fn identity() -> Identity {
    Identity {
        project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
        world_schema_version: "world-schema-1".into(),
        content_revision: "rev-1".into(),
        ..Identity::default()
    }
}

fn id(n: u8) -> [u8; 16] {
    [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n]
}

#[test]
fn a_written_placement_is_a_cell_the_booted_world_lets_a_creature_enter() -> TestResult {
    let mut families = Families::default();
    families.npcs.insert(
        "oteryn:npc.a".into(),
        NpcFacts {
            dialogue: None,
            services: Vec::new(),
        },
    );
    families.npc_placements = (1..=7)
        .map(|x| NpcPlacement {
            key: format!("oteryn:npc_placement.a.x{x}_y0_z7"),
            npc: "oteryn:npc.a".into(),
            cell: (x, 0, 7),
            direction: Direction::South,
        })
        .collect();
    let catalogue = oteryn_world_bundle::npc::catalogue_sha256(&root().join("content"))?;
    families.catalogue_sha256 = Some(catalogue.clone());
    let (regions, palette) = (vec![region()?], PALETTE.map(String::from).to_vec());
    let compiled = compile(
        &Input {
            regions: &regions,
            palette: &palette,
            identity: identity(),
            world: Extent {
                min_x: 0,
                min_y: 0,
                max_x: 32,
                max_y: 32,
                floors: vec![-7],
            },
            build_class: BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &families,
        },
        &Resolver,
    )?;
    let held: Vec<_> = compiled
        .npcs
        .held
        .iter()
        .map(|held| (held.cell.0, held.reason))
        .collect();
    assert_eq!(
        held,
        [
            (2, HoldReason::NotWalkable),
            (3, HoldReason::Wall),
            (4, HoldReason::BlockSolid),
            (6, HoldReason::NoTile),
        ]
    );
    // The server's reader sees the digest the compiler wrote.
    let read = bundle::read(&compiled.bytes)?;
    assert_eq!(read.manifest.npcs.catalogue_sha256, catalogue);
    let mut revision = String::from("sha256:");
    for byte in compiled.digest {
        revision.push_str(&format!("{byte:02x}"));
    }
    let pins = BootPins {
        bundle: BundlePins {
            digest: compiled.digest,
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            production: false,
        },
        map_revision: revision,
        start: TilePos {
            x: 1,
            y: 0,
            floor: -7,
        },
    };
    let world = boot(
        &compiled.bytes,
        &pins,
        WorldId::decode(&id(1))?,
        ChannelId::decode(&id(2))?,
        item,
    )?;
    let written: BTreeSet<_> = read
        .npcs
        .npcs
        .iter()
        .flat_map(|npc| npc.placements.iter().map(|p| (p.x, p.y, p.floor)))
        .collect();
    assert_eq!(written.len(), 3);
    for x in 1..=7 {
        let enterable = world.enterable(TilePos { x, y: 0, floor: -7 });
        assert_eq!(written.contains(&(x, 0, -7)), enterable, "x = {x}");
    }
    Ok(())
}
