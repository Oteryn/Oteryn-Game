//! MAP-CUTOVER-1b: the production map facts of a booted bundle World, through the public boot
//! API: §1.6 references, placement bindings, counts and charges, house tiles, and the refusal of
//! a palette Item the item index disagrees with.

use std::error::Error as StdError;
use std::num::NonZeroU32;

use oteryn_game_server::foundation::{ChannelId, WorldId};
use oteryn_game_server::map::BundlePins;
use oteryn_game_server::map::boot::{BootPins, BootRefusal, BundleWorld, boot};
use oteryn_game_server::map::facts::ItemDefinition;
use oteryn_game_server::map::overlay::TilePos;
use oteryn_game_server::map::view::{MapFacts, placement_key};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_world_bundle_compiler::bundle::{
    self, BuildClass, Extent, Family, Identity, Manifest, PaletteEntry, Sector, Terrain,
    TerrainKind,
};
use oteryn_world_bundle_compiler::sector::{Attrs, Item, Tile};

type TestResult = Result<(), Box<dyn StdError>>;

fn id(n: u8) -> [u8; 16] {
    [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n]
}

const fn at(x: u16) -> TilePos {
    TilePos { x, y: 0, floor: -7 }
}

/// One row at native floor -7, all on grass (Terrain id 0): a plain coin at x 1, a coin stack of
/// 5 at x 2, a lever with an action and 3 charges at x 3, a coin on a house tile at x 4 and a
/// coin dropped as a teleport at x 5. The coin is Item id 0 and the lever Item id 1.
fn bundle() -> Result<(Vec<u8>, BundlePins), Box<dyn StdError>> {
    let entry = |key: &str, family, id, terrain| PaletteEntry {
        key: key.into(),
        family,
        id,
        terrain,
    };
    let coin = |attrs| Item {
        palette: 1,
        depth: 0,
        attrs,
    };
    let tile = |x, house, item: Item| Tile {
        x,
        y: 0,
        flags: 0,
        house,
        zones: Vec::new(),
        items: vec![
            Item {
                palette: 0,
                depth: 0,
                attrs: Attrs::default(),
            },
            item,
        ],
    };
    let dropped = placement_key(at(5), 1).ok_or("placement key")?;
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
            entry(
                "terrain:grass",
                Family::Terrain,
                0,
                Some(Terrain {
                    kind: TerrainKind::Ground,
                    walkable: Some(true),
                    ground_speed: Some(150),
                }),
            ),
            entry("item:coin", Family::Item, 0, None),
            entry("item:lever", Family::Item, 1, None),
        ],
        draft_areas: Vec::new(),
        skipped_provisional_keys: Vec::new(),
        dropped_teleports: vec![dropped],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let sector = Sector {
        floor: -7,
        sx: 0,
        sy: 0,
        tiles: vec![
            tile(1, 0, coin(Attrs::default())),
            tile(
                2,
                0,
                coin(Attrs {
                    count: Some(5),
                    ..Attrs::default()
                }),
            ),
            tile(
                3,
                0,
                Item {
                    palette: 2,
                    depth: 0,
                    attrs: Attrs {
                        action: Some(1000),
                        charges: Some(3),
                        ..Attrs::default()
                    },
                },
            ),
            tile(4, 9, coin(Attrs::default())),
            tile(5, 0, coin(Attrs::default())),
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

/// The served definitions; the §1.6 reference is 1 + the palette `id`.
fn item(key: &str) -> Option<ItemDefinition> {
    let (id, pickupable) = match key {
        "item:coin" => (0, true),
        "item:lever" => (1, false),
        _ => return None,
    };
    Some(ItemDefinition {
        reference: NonZeroU32::new(id + 1)?,
        solid: Some(false),
        blocks_projectile: false,
        pickupable,
    })
}

fn boot_with(
    item: impl Fn(&str) -> Option<ItemDefinition>,
) -> Result<BundleWorld, Box<dyn StdError>> {
    let (bytes, bundle) = bundle()?;
    let mut map_revision = String::from("sha256:");
    for byte in bundle.digest {
        map_revision.push_str(&format!("{byte:02x}"));
    }
    let pins = BootPins {
        bundle,
        map_revision,
        start: at(1),
    };
    Ok(boot(
        &bytes,
        &pins,
        WorldId::decode(&id(1))?,
        ChannelId::decode(&id(2))?,
        item,
    )?)
}

#[test]
fn map_cutover_bundle_facts_carry_references_bindings_counts_and_houses() -> TestResult {
    let world = boot_with(item)?;
    let facts = world.facts();
    let one = NonZeroU32::MIN;
    let two = one.saturating_add(1);

    let grass = facts.base_entry(at(1), 0, 0).ok_or("grass")?;
    assert_eq!(grass.definition, MapDefinition::Terrain(one));
    let plain = facts.base_entry(at(1), 1, 0).ok_or("coin")?;
    assert_eq!(plain.definition, MapDefinition::Item(one));
    assert!(plain.pickupable && !plain.bound);
    assert_eq!((plain.count, plain.sub_type), (1, 0));

    let stack = facts.base_entry(at(2), 1, 0).ok_or("stack")?;
    assert_eq!((stack.count, stack.bound), (5, false));

    let lever = facts.base_entry(at(3), 1, 1).ok_or("lever")?;
    assert_eq!(lever.definition, MapDefinition::Item(two));
    assert!(lever.bound && !lever.pickupable);
    assert_eq!(lever.sub_type, 3);

    assert!(facts.house_tile(at(4)) && !facts.house_tile(at(1)));
    assert!(facts.base_entry(at(5), 1, 0).ok_or("dropped")?.bound);

    // No overlay object has moved, and a compact id that is not the placement's has no facts.
    let key = placement_key(at(3), 1).ok_or("placement key")?;
    assert_eq!(facts.object_revision(key), 0);
    assert_eq!(facts.base_entry(at(3), 1, 0), None);
    Ok(())
}

#[test]
fn map_cutover_an_item_the_index_lacks_or_places_elsewhere_refuses_boot() -> TestResult {
    let missing = |key: &str| item(key).filter(|_| key != "item:lever");
    assert_eq!(
        boot_with(missing).err().map(|error| error.to_string()),
        Some(BootRefusal::ItemReference.to_string())
    );
    let moved = |key: &str| {
        item(key).map(|definition| ItemDefinition {
            reference: definition.reference.saturating_add(5),
            ..definition
        })
    };
    assert_eq!(
        boot_with(moved).err().map(|error| error.to_string()),
        Some(BootRefusal::ItemReference.to_string())
    );
    Ok(())
}
