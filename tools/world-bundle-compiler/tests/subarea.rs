//! Real 15.30 masks -> synthetic served map -> pinned native membership.
use std::path::Path;

use bundle::{BuildClass, Extent, Family, Identity, Terrain, TerrainKind};
use compile::{Input, KeyResolver, Resolution, subareas::Source};
use oteryn_world_bundle_compiler::{Error, bundle, compile, project::Families};

struct Ground;
impl KeyResolver for Ground {
    fn resolve(&self, _: &str) -> Resolution {
        Resolution::Resolved(Family::Terrain, 0)
    }
    fn terrain(&self, _: &str) -> Result<Option<Terrain>, Error> {
        Ok(Some(Terrain {
            kind: TerrainKind::Ground,
            walkable: Some(true),
            ground_speed: Some(150),
        }))
    }
    fn floor_change(&self, _: &str) -> bool {
        false
    }
}

#[test]
fn subarea_real_masks_compile_deterministically_and_drop_inaccessible_tiles()
-> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut families = Families {
        subareas: Source::load(&root)?,
        ..Families::default()
    };
    let regions = vec![include_bytes!("fixtures/subarea-z07-x126-y121.b3").to_vec()];
    let palette = vec!["oteryn:terrain.grass".into()];
    let make = |families| Input {
        regions: &regions,
        palette: &palette,
        identity: Identity::default(),
        world: Extent {
            min_x: 32000,
            min_y: 31000,
            max_x: 33000,
            max_y: 32000,
            floors: vec![-7],
        },
        build_class: BuildClass::NonProduction,
        draft_areas: vec![],
        families,
    };
    let first = compile::compile(&make(&families), &Ground)?;
    let second = compile::compile(&make(&families), &Ground)?;
    assert_eq!(first.bytes, second.bytes);
    compile::equivalence(&make(&families), &Ground, &first.bytes)?;
    let read = bundle::read(&first.bytes)?;
    let table = &read.manifest.subareas;
    assert_eq!(table.areas.len(), 465);
    assert_eq!(table.masks.len(), 209);
    assert_eq!(
        table
            .masks
            .iter()
            .map(|m| m.source_tiles as usize)
            .sum::<usize>(),
        1_901_667
    );
    assert_eq!(table.resolve(32362, 31009, -7), Some((23, 109)));
    assert_eq!(table.resolve(32363, 31009, -7), Some((23, 109)));
    assert_eq!(
        table.resolve(32361, 31009, -7),
        None,
        "walkable neighbour outside the mask"
    );
    assert_eq!(
        table.resolve(32362, 31009, -6),
        None,
        "never project a mask onto another floor"
    );
    assert_eq!(
        table.resolve(32348, 31010, -7),
        None,
        "mask pixel on isolated walkable ground is inaccessible"
    );
    assert_eq!(table.masks.iter().map(|m| m.admitted_tiles).sum::<u32>(), 2);
    // A declared travel entry makes the otherwise isolated component reachable.
    families
        .routes
        .push(oteryn_world_bundle_compiler::project::TravelRoute {
            service: "oteryn:service.travel".into(),
            route: "island".into(),
            destination: (32348, 31010, 7),
        });
    let entered = compile::compile(
        &Input {
            regions: &regions,
            palette: &palette,
            identity: Identity::default(),
            world: Extent {
                min_x: 32000,
                min_y: 31000,
                max_x: 33000,
                max_y: 32000,
                floors: vec![-7],
            },
            build_class: BuildClass::NonProduction,
            draft_areas: vec![],
            families: &families,
        },
        &Ground,
    )?;
    assert_eq!(
        bundle::read(&entered.bytes)?
            .manifest
            .subareas
            .resolve(32348, 31010, -7),
        Some((23, 109))
    );
    assert_ne!(first.digest, entered.digest);
    Ok(())
}
