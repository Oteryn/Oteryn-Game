//! Original Edron source region for the actual Sandra deposit caller.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{self, Input},
    source,
};
#[test]
fn srv7_sandra_actual_source_world_export() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(129, 124)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let built = compile::compile(
        &Input {
            regions: &project.regions,
            palette: &palette,
            identity: Identity::default(),
            world: source::world_extent(&root)?,
            build_class: BuildClass::NonProduction,
            draft_areas: vec![],
            families: &project.families,
        },
        &registry,
    )?;
    let bundle = oteryn_world_bundle::bundle::read(&built.bytes)?;
    let placements = bundle
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.sandra")
        .ok_or("actual Sandra sourceNPC")?;
    assert!(
        placements
            .placements
            .iter()
            .any(|p| (p.x, p.y, p.floor) == (33257, 31839, -7))
    );
    if let Some(path) = std::env::var_os("OTERYN_SRV7_SANDRA_BUNDLE") {
        std::fs::write(path, &built.bytes)?;
    }
    Ok(())
}
