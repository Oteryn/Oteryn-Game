use super::fixture::debug;

use super::{Dependencies, TestResult};

use oteryn_world_bundle_compiler::{Error, bundle, compile, project::Families};

struct Ground(u32);

impl compile::KeyResolver for Ground {
    fn resolve(&self, _: &str) -> compile::Resolution {
        compile::Resolution::Resolved(bundle::Family::Item, self.0)
    }
    fn terrain(&self, _: &str) -> Result<Option<bundle::Terrain>, Error> {
        Ok(Some(bundle::Terrain {
            kind: bundle::TerrainKind::Ground,
            walkable: Some(true),
            ground_speed: Some(150),
            blocks_projectile: None,
        }))
    }
    fn floor_change(&self, _: &str) -> bool {
        false
    }
}

pub(super) fn world(
    deps: &Dependencies,
    generation: &crate::content::ActiveGeneration,
) -> TestResult<crate::map::boot::BundleWorld> {
    let index = generation
        .native_gameplay()
        .and_then(|n| n.item_index())
        .ok_or("item index")?;
    let key = "oteryn:item.tibia.i100";
    let ground = index
        .definition_ref("Item", key, "definition-r1")
        .ok_or("ground ref")?;
    let (_, mut pins) = crate::map::boot::tests::bundle();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let families = Families {
        subareas: compile::subareas::Source::load(&root)?,
        ..Families::default()
    };
    let input = compile::Input {
        regions: &[include_bytes!(
            "../../../../../../../tools/world-bundle-compiler/tests/fixtures/subarea-z07-x126-y121.b3"
        )
        .to_vec()],
        palette: &[key.into()],
        identity: bundle::Identity {
            project_format_version: pins.project_format_version.clone(),
            world_schema_version: pins.world_schema_version.clone(),
            content_revision: pins.content_revision.clone(),
            ..bundle::Identity::default()
        },
        world: bundle::Extent {
            min_x: 32000,
            min_y: 31000,
            max_x: 33000,
            max_y: 32000,
            floors: vec![-7],
        },
        build_class: bundle::BuildClass::Production,
        draft_areas: vec![],
        families: &families,
    };
    let compiled = compile::compile(&input, &Ground(ground.get() - 1))?;
    pins.digest = compiled.digest;
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    assert_eq!(
        base.subareas.resolve(32348, 31010, -7),
        None,
        "inaccessible mask pixel was filtered before boot"
    );
    crate::map::boot::boot(
        &compiled.bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: 32362,
                y: 31009,
                floor: -7,
            },
        },
        deps.admitted.world_id,
        deps.admitted.channel_id,
        |key| {
            Some(crate::map::facts::ItemDefinition {
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
