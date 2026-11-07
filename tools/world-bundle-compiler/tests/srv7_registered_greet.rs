//! Original private registered Greet source region for the actual registered Greet deposit caller.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{self, Input},
    source,
};
#[test]
fn srv7_registered_greet_actual_three_source_world_export() -> Result<(), Box<dyn std::error::Error>>
{
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(126, 123), (126, 125), (130, 124)])?;
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
    for (key, cell) in [
        ("oteryn:npc.bron", (32366, 31628, -7)),
        ("oteryn:npc.bozo", (32313, 32183, -7)),
        ("oteryn:npc.a_majestic_warwolf", (33355, 31991, -8)),
    ] {
        let placements = bundle
            .npcs
            .npcs
            .iter()
            .find(|n| n.key == key)
            .ok_or("actual registered greet sourceNPC")?;
        assert!(
            placements
                .placements
                .iter()
                .any(|p| (p.x, p.y, p.floor) == cell)
        );
    }
    if let Some(path) = std::env::var_os("OTERYN_SRV7_REGISTERED_GREET_BUNDLE") {
        std::fs::write(path, &built.bytes)?;
    }
    Ok(())
}
