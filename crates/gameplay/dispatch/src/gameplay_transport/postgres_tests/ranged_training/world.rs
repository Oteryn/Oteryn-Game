use super::*;
use crate::content::item_ref::ItemDefinitionIndex;
use crate::map::boot::{BootPins, BundleWorld, boot};
use crate::map::facts::ItemDefinition;
use crate::map::overlay::TilePos;
use oteryn_world_bundle::writer::*;

pub(super) fn training_world(
    index: &ItemDefinitionIndex,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
) -> TestResult<BundleWorld> {
    use oteryn_world_bundle::sector::{Attrs, Item, Tile};
    let keys = [
        "oteryn:item.tibia.i100",
        "oteryn:item.tibia.i16198",
        "oteryn:item.tibia.i28558",
    ];
    let manifest = Manifest {
        subareas: Default::default(),
        format: FORMAT.into(),
        min_reader_version: VERSION,
        projection_class: "server".into(),
        compiler_version: "loop-training-test".into(),
        build_class: BuildClass::Production,
        identity: Identity {
            project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
            world_schema_version: "world-schema-1".into(),
            content_revision: "content-1".into(),
            ..Identity::default()
        },
        world: Extent {
            min_x: 0,
            min_y: 0,
            max_x: 32,
            max_y: 32,
            floors: vec![-7],
        },
        palette: keys
            .iter()
            .enumerate()
            .map(|(i, key)| {
                Ok(PaletteEntry {
                    key: (*key).into(),
                    family: Family::Item,
                    id: index
                        .definition_ref("Item", key, "definition-r1")
                        .ok_or("training item ref")?
                        .get()
                        - 1,
                    terrain: (i == 0).then_some(Terrain {
                        kind: TerrainKind::Ground,
                        walkable: Some(true),
                        ground_speed: Some(150),
                    }),
                })
            })
            .collect::<TestResult<Vec<_>>>()?,
        draft_areas: vec![],
        skipped_provisional_keys: vec![],
        dropped_teleports: vec![],
        spawns: Default::default(),
        npcs: Default::default(),
    };
    let tile = |x, items: Vec<u32>| Tile {
        x,
        y: 0,
        flags: 1,
        house: 0,
        zones: vec![],
        items: items
            .into_iter()
            .map(|palette| Item {
                palette,
                depth: 0,
                attrs: Attrs::default(),
            })
            .collect(),
    };
    let bytes = write(
        &manifest,
        &[Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles: vec![tile(1, vec![0]), tile(2, vec![0]), tile(3, vec![0]), tile(4, vec![0]), tile(5, vec![0]), tile(6, vec![0, 1, 2])],
        }],
        &Default::default(),
    )
    .map_err(debug)?;
    let digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    let pins = crate::map::BundlePins {
        digest,
        project_format_version: "OTERYN_WORLD_PROJECT/v2".into(),
        world_schema_version: "world-schema-1".into(),
        content_revision: "content-1".into(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    boot(
        &bytes,
        &BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: TilePos {
                x: 1,
                y: 0,
                floor: -7,
            },
        },
        admitted.world_id,
        admitted.channel_id,
        |key| {
            Some(ItemDefinition {
                reference: index.definition_ref("Item", key, "definition-r1")?,
                solid: Some(false),
                blocks_projectile: false,
                pickupable: false,
                door: false,
            })
        },
    )
    .map_err(debug)
}
