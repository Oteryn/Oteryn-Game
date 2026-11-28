//! Crystal 201189f8 placements through the repository compiler and native bundle loader.

use crate::content::native_gameplay::{NativeGameplayInput, test_support::try_activate_input};
use crate::foundation::{ChannelId, WorldId};
use crate::map::boot::{BootPins, BundleWorld, boot, native_item_definition};
use crate::map::floor::{Cell, Ground, is_ladder_appearance, resolve_landing};
use crate::map::overlay::TilePos;
use crate::movement::CardinalStep;
use oteryn_protocol_oteryn::world_map::MAX_BASE_ORDINAL;
use oteryn_world_bundle_compiler::{bundle, compile, project, source};
use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

const FLOORS: &[(Cell, u16, Cell)] = &[
    ((32085, 32017, 8), 166, (32083, 32017, 9)),
    ((32951, 31460, 9), 166, (32950, 31460, 10)),
    ((33012, 32457, 11), 167, (33012, 32455, 12)),
    ((33109, 32407, 13), 166, (33109, 32407, 14)),
    ((33850, 32722, 10), 53441, (33850, 32723, 9)),
    ((33851, 32722, 10), 53441, (33851, 32723, 9)),
    ((33852, 32722, 10), 53441, (33852, 32723, 9)),
    ((33861, 32113, 5), 51875, (33861, 32112, 4)),
    ((33870, 32112, 3), 51875, (33870, 32111, 2)),
    ((33872, 32115, 6), 51875, (33872, 32114, 5)),
    ((33873, 32117, 2), 51875, (33873, 32116, 1)),
    ((33875, 32113, 4), 51877, (33874, 32113, 3)),
    ((33876, 32126, 3), 51875, (33876, 32125, 2)),
    ((34111, 32540, 8), 54581, (34110, 32540, 7)),
    ((34114, 32543, 7), 54578, (34113, 32543, 6)),
    ((34114, 32544, 7), 54581, (34113, 32544, 6)),
];
const TELEPORTS: &[(Cell, u16, Cell)] = &[
    ((32867, 32826, 1), 775, (32866, 32831, 7)),
    ((33006, 32396, 15), 34111, (34106, 32052, 13)),
    ((33044, 32409, 15), 34111, (34106, 32052, 13)),
    ((33965, 31979, 9), 34111, (33175, 31764, 6)),
];
// Current Edron geometry has rock here and Crystal supplies no tile on floor 11.
const EDRON_REWORK_ROCK: Cell = (33378, 31806, 10);
const LADDER: Cell = (32582, 32558, 11);
const LADDER_LANDING: Cell = (32582, 32559, 10);

fn position((x, y, z): Cell) -> Option<TilePos> {
    Some(TilePos {
        x: x.try_into().ok()?,
        y: y.try_into().ok()?,
        floor: (-z).try_into().ok()?,
    })
}

fn world() -> Result<BundleWorld, Box<dyn Error + Send + Sync>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let regions: BTreeSet<_> = FLOORS
        .iter()
        .chain(TELEPORTS)
        .flat_map(|&(from, _, to)| [from, to])
        .chain([LADDER, LADDER_LANDING, EDRON_REWORK_ROCK])
        .filter_map(position)
        .map(|pos| (pos.x / 256, pos.y / 256))
        .collect();
    let project = source::project_regions(&root, &regions.into_iter().collect::<Vec<_>>())?;
    let (registry, palette) = source::registry(&root, &project)?;
    let identity: bundle::Identity = serde_json::from_slice(&std::fs::read(
        root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let pins = crate::map::BundlePins {
        digest: [0; 32],
        project_format_version: identity.project_format_version.clone(),
        world_schema_version: identity.world_schema_version.clone(),
        content_revision: identity.content_revision.clone(),
        production: false,
    };
    let artifact = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity,
            world: source::world_extent(&root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: project::draft_areas(&project.index)?,
            families: &project.families,
        },
        &registry,
    )?;
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = 42;
    let world = WorldId::decode(&id).map_err(|error| format!("{error:?}"))?;
    id[15] = 43;
    let channel = ChannelId::decode(&id).map_err(|error| format!("{error:?}"))?;
    let input = NativeGameplayInput::from_manifest(&root.join("content/spells.manifest.json"))
        .map_err(|error| format!("{error:?}"))?;
    let (controller, _) =
        try_activate_input(world, &input).map_err(|error| format!("{error:?}"))?;
    let generation = controller.active().ok_or("active native generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let digest = artifact.digest;
    let map_revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    boot(
        &artifact.bytes,
        &BootPins {
            bundle: crate::map::BundlePins { digest, ..pins },
            map_revision,
            start: position(TELEPORTS[0].2).ok_or("entry tile")?,
        },
        world,
        channel,
        |key| native_item_definition(Some(native), key),
    )
    .map_err(|error| format!("{error:?}").into())
}

#[test]
fn crystal_static_traversal_loads_donor_routes() -> Result<(), Box<dyn Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(check_routes)?
        .join()
        .map_err(|_| "Crystal traversal test thread panicked")?
}

fn check_routes() -> Result<(), Box<dyn Error + Send + Sync>> {
    let world = world()?;
    let appearance = |cell: Cell, ordinal: u8| {
        let pos = position(cell)?;
        world
            .base()
            .tile(pos.x, pos.y, pos.floor)?
            .appearance(ordinal)
    };
    let ground = |cell| {
        let Some(pos) = position(cell) else {
            return Ground::Absent;
        };
        if world.base().tile(pos.x, pos.y, pos.floor).is_none() {
            Ground::Absent
        } else if world.enterable(pos) {
            Ground::Walkable
        } else {
            Ground::Blocked
        }
    };
    let mut failures = Vec::new();
    for &(from, id, to) in FLOORS.iter().chain(TELEPORTS) {
        let (x, y, z) = from;
        let actual = resolve_landing(
            world.floors(),
            ground,
            |_| true,
            (x - 1, y, z),
            CardinalStep::East,
        );
        if actual != Ok(to)
            || !(0..=MAX_BASE_ORDINAL).any(|ordinal| appearance(from, ordinal) == Some(id))
        {
            failures.push(format!(
                "{id} at {from:?}: expected {to:?}, landed {actual:?}"
            ));
        }
    }
    if !(0..=MAX_BASE_ORDINAL)
        .filter_map(|ordinal| appearance(LADDER, ordinal))
        .any(|id| id == 50564 && is_ladder_appearance(id))
        || ground(LADDER_LANDING) != Ground::Walkable
    {
        failures.push(format!(
            "50564 at {LADDER:?}: missing USE selector or landing"
        ));
    }
    assert_eq!(ground(EDRON_REWORK_ROCK), Ground::Blocked);
    assert_eq!(ground((33378, 31806, 11)), Ground::Absent);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}
