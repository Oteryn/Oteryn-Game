//! MAP-CUTOVER-1a: a world bundle boots into a Channel World or is refused, and a restart with the
//! same pins rebuilds an equal World, against a fixture bundle each test assembles.

use std::error::Error as StdError;
use std::num::NonZeroU32;

use oteryn_game_server::foundation::{ChannelId, WorldId};
use oteryn_game_server::map::boot::{BootPins, BootRefusal, BundleWorld, boot};
use oteryn_game_server::map::facts::ItemDefinition;
use oteryn_game_server::map::overlay::TilePos;
use oteryn_game_server::map::{BundlePins, LoadError};
use oteryn_world_bundle_compiler::bundle::{
    self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
    TerrainKind,
};
use oteryn_world_bundle_compiler::sector::{Attrs, Item, Tile};

type TestResult = Result<(), Box<dyn StdError>>;

fn id(n: u8) -> [u8; 16] {
    [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n]
}

/// One row at native floor -7: grass at x 1..=3, lava at x 4, a wall at x 5, a solid box at x 6
/// and a loose coin at x 7.
fn bundle() -> Result<(Vec<u8>, BundlePins), Box<dyn StdError>> {
    let terrain = |key: &str, id, kind, walkable: Option<bool>| PaletteEntry {
        key: key.into(),
        family: Family::Terrain,
        id,
        terrain: Some(Terrain {
            kind,
            walkable,
            ground_speed: walkable.map(|_| 150),
        }),
    };
    let item = |key: &str, id| PaletteEntry {
        key: key.into(),
        family: Family::Item,
        id,
        terrain: None,
    };
    let manifest = Manifest {
        format: bundle::FORMAT.into(),
        min_reader_version: bundle::VERSION,
        projection_class: "server".into(),
        compiler_version: "engineering".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "rev-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 32,
            max_y: 32,
            floors: vec![-7],
        },
        palette: vec![
            terrain("terrain:grass", 1, TerrainKind::Ground, Some(true)),
            terrain("terrain:lava", 2, TerrainKind::Ground, Some(false)),
            terrain("terrain:wall", 3, TerrainKind::Wall, None),
            item("item:box", 1),
            item("item:coin", 2),
        ],
        draft_areas: Vec::new(),
        skipped_provisional_keys: Vec::new(),
        dropped_teleports: Vec::new(),
        spawns: Default::default(),
    };
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
    let sector = Sector {
        floor: -7,
        sx: 0,
        sy: 0,
        tiles: vec![
            tile(1, &[0]),
            tile(2, &[0]),
            tile(3, &[0]),
            tile(4, &[1]),
            tile(5, &[0, 2]),
            tile(6, &[0, 3]),
            tile(7, &[0, 4]),
        ],
    };
    let bytes = bundle::write(&manifest, &[sector], &Default::default())?;
    let digest = bundle::read(&bytes)?.digest;
    let pins = BundlePins {
        digest,
        project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
        world_schema_version: "world-schema-1".into(),
        content_revision: "rev-1".into(),
        production: false,
    };
    Ok((bytes, pins))
}

/// The served definitions of the fixture's Items; the §1.6 reference is 1 + the palette `id`.
fn item(key: &str) -> Option<ItemDefinition> {
    let (id, solid) = match key {
        "item:box" => (1, true),
        "item:coin" => (2, false),
        _ => return None,
    };
    Some(ItemDefinition {
        reference: NonZeroU32::new(id + 1)?,
        solid: Some(solid),
        blocks_projectile: solid,
        pickupable: !solid,
    })
}

fn pins(bundle: &BundlePins, x: u16) -> BootPins {
    let mut revision = String::from("sha256:");
    for byte in bundle.digest {
        revision.push_str(&format!("{byte:02x}"));
    }
    BootPins {
        bundle: bundle.clone(),
        map_revision: revision,
        start: TilePos { x, y: 0, floor: -7 },
    }
}

fn boot_at(bytes: &[u8], pins: &BootPins) -> Result<BundleWorld, BootRefusal> {
    let world = WorldId::decode(&id(1)).map_err(|_| BootRefusal::MapRevision)?;
    let channel = ChannelId::decode(&id(2)).map_err(|_| BootRefusal::MapRevision)?;
    boot(bytes, pins, world, channel, item)
}

#[test]
fn map_cutover_a_bundle_boots_and_marks_enterable_ground() -> TestResult {
    let (bytes, load) = bundle()?;
    let world = boot_at(&bytes, &pins(&load, 2))?;
    assert_eq!(
        world.start(),
        TilePos {
            x: 2,
            y: 0,
            floor: -7
        }
    );
    assert_eq!(world.base().tile_count(), 7);
    let at = |x| world.enterable(TilePos { x, y: 0, floor: -7 });
    assert!(at(1) && at(2) && at(3) && at(7));
    // Lava, a wall, a solid item and a missing tile are not enterable.
    assert!(!at(4) && !at(5) && !at(6) && !at(0) && !at(8));
    Ok(())
}

#[test]
fn map_cutover_each_boot_check_refuses() -> TestResult {
    let (bytes, load) = bundle()?;
    let good = pins(&load, 2);

    let mut digest = good.clone();
    digest.bundle.digest[0] ^= 1;
    assert_eq!(
        boot_at(&bytes, &digest).err(),
        Some(BootRefusal::Load(LoadError::Digest))
    );

    let mut schema = good.clone();
    schema.bundle.world_schema_version = "world-schema-2".into();
    assert_eq!(
        boot_at(&bytes, &schema).err(),
        Some(BootRefusal::Load(LoadError::SchemaVersion))
    );

    let mut revision = good.clone();
    revision.bundle.content_revision = "rev-2".into();
    assert_eq!(
        boot_at(&bytes, &revision).err(),
        Some(BootRefusal::Load(LoadError::ContentRevision))
    );

    let mut map_revision = good.clone();
    map_revision.map_revision = "sha256:00".into();
    assert_eq!(
        boot_at(&bytes, &map_revision).err(),
        Some(BootRefusal::MapRevision)
    );

    // Lava, a wall, a solid item, a missing tile and a missing floor are not enterable starts.
    for x in [4, 5, 6, 0, 9] {
        assert_eq!(
            boot_at(&bytes, &pins(&load, x)).err(),
            Some(BootRefusal::StartNotEnterable),
            "start x {x}"
        );
    }
    let mut floor = good;
    floor.start.floor = -6;
    assert_eq!(
        boot_at(&bytes, &floor).err(),
        Some(BootRefusal::StartNotEnterable)
    );
    Ok(())
}

#[test]
fn map_cutover_a_restart_with_the_same_pins_rebuilds_an_equal_world() -> TestResult {
    let (bytes, load) = bundle()?;
    let first = boot_at(&bytes, &pins(&load, 2))?;
    let restart = boot_at(&bytes, &pins(&load, 2))?;
    assert_eq!(first, restart);
    assert_ne!(first, boot_at(&bytes, &pins(&load, 3))?);
    Ok(())
}
