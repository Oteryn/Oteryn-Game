use oteryn_world_bundle_compiler::{
    bundle::{self, BuildClass, Bundle, Identity},
    compile::{self, Input},
    source,
};
use serde::Deserialize;
use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Deserialize)]
struct Position {
    x: u16,
    y: u16,
    floor: i8,
}

#[derive(Deserialize)]
struct Selector {
    position: Option<Position>,
    #[serde(default)]
    position_aliases: Vec<Position>,
    #[serde(flatten)]
    object: Object,
    native: Option<NativeSelector>,
}

#[derive(Clone, Copy, Default, Deserialize)]
struct Object {
    appearance: Option<u16>,
    ordinal: Option<usize>,
}

#[derive(Deserialize)]
struct NativeSelector {
    position: Position,
    appearance: u16,
    ordinal: usize,
}

#[derive(Deserialize)]
struct Document {
    #[serde(default)]
    callbacks: Vec<Callback>,
}

#[derive(Deserialize)]
struct Callback {
    key: String,
    selector: Selector,
}

fn compiled_world(
    root: &Path,
    positions: impl IntoIterator<Item = Position>,
) -> Result<Bundle, Box<dyn Error>> {
    let regions: BTreeSet<_> = positions
        .into_iter()
        .map(|at| (at.x / 256, at.y / 256))
        .collect();
    let project = source::project_regions(root, &regions.into_iter().collect::<Vec<_>>())?;
    let (registry, palette) = source::registry(root, &project)?;
    let compiled = compile::compile(
        &Input {
            regions: &project.regions,
            palette: &palette,
            identity: Identity::default(),
            world: source::world_extent(root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: vec![],
            families: &project.families,
        },
        &registry,
    )?;
    Ok(bundle::read(&compiled.bytes)?)
}

fn contains_object(world: &Bundle, at: Position, object: Object) -> bool {
    world
        .sectors
        .iter()
        .filter(|sector| {
            sector.floor == at.floor && (sector.sx, sector.sy) == (at.x / 32, at.y / 32)
        })
        .flat_map(|sector| &sector.tiles)
        .filter(|tile| (tile.x, tile.y) == (at.x, at.y))
        .flat_map(|tile| tile.items.iter().filter(|item| item.depth == 0).enumerate())
        .any(|(ordinal, item)| {
            let key = &world.manifest.palette[item.palette as usize].key;
            let appearance = key
                .rsplit_once(".tibia.i")
                .and_then(|(_, id)| id.parse().ok());
            object.appearance.is_none_or(|id| Some(id) == appearance)
                && object.ordinal.is_none_or(|expected| expected == ordinal)
        })
}

fn world_documents(authoring: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut directories = Vec::new();
    for batch in fs::read_dir(authoring)? {
        let batch = batch?.path();
        directories.extend([batch.join("world"), batch.join("world-callbacks")]);
    }
    for batch in fs::read_dir(authoring.join("batches"))? {
        directories.push(batch?.path());
    }
    let mut files = Vec::new();
    for directory in directories.into_iter().filter(|path| path.is_dir()) {
        for file in fs::read_dir(directory)? {
            let path = file?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

#[test]
fn every_positioned_authored_quest_object_is_in_the_compiled_world() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let authoring = root.join("tools/content-schema/quest-authoring");
    let mut required = Vec::new();
    for path in world_documents(&authoring)? {
        let document: Document = serde_json::from_slice(&fs::read(&path)?)?;
        for Callback { key, selector } in document.callbacks {
            if let Some(native) = selector.native {
                required.push((
                    path.clone(),
                    key.clone(),
                    native.position,
                    Object {
                        appearance: Some(native.appearance),
                        ordinal: Some(native.ordinal),
                    },
                ));
            }
            let object = selector.object;
            // Positional triggers and AID/UID registrations alone name no item appearance.
            if object.appearance.is_some() || object.ordinal.is_some() {
                for at in selector
                    .position
                    .into_iter()
                    .chain(selector.position_aliases)
                {
                    required.push((path.clone(), key.clone(), at, object));
                }
            }
        }
    }
    let world = compiled_world(&root, required.iter().map(|(_, _, at, _)| *at))?;
    let missing: Vec<_> = required
        .iter()
        .filter(|(_, _, at, object)| !contains_object(&world, *at, *object))
        .map(|(path, key, at, _)| format!("{} {key} at {at:?}", path.display()))
        .collect();
    assert!(
        missing.is_empty(),
        "missing quest objects:\n{}",
        missing.join("\n")
    );
    Ok(())
}

#[test]
fn guarded_crystal_sphere_is_at_the_blue_valley_return() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let at = Position {
        x: 33605,
        y: 31468,
        floor: -7,
    };
    let world = compiled_world(&root, [at])?;
    let object = Object {
        appearance: Some(50227),
        ..Object::default()
    };
    assert!(
        contains_object(&world, at, object),
        "Guarded Crystal Sphere at {at:?}"
    );
    Ok(())
}
