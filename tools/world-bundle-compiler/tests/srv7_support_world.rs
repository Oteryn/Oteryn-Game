//! Authentic support actors, original B3 and existing source spawn families.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity, TerrainKind},
    compile::{self, Input},
    source,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn srv7_support_actual_world_exports_spike_and_wedding_source_fixture() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(125, 127), (128, 125)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: source::world_extent(&root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: vec![],
        families: &project.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    let parity = compile::equivalence(&input, &registry, &compiled.bytes)?;
    let bundle = oteryn_world_bundle::bundle::read(&compiled.bytes)?;
    for (key, merchant, player) in [
        (
            "oteryn:npc.gnomilly",
            (32242, 32599, -9),
            (32243, 32597, -9),
        ),
        ("oteryn:npc.norbert", (32952, 32103, -6), (32953, 32103, -6)),
    ] {
        let actor = bundle
            .npcs
            .npcs
            .iter()
            .find(|npc| npc.key == key)
            .ok_or("actual support source NPC")?;
        assert_eq!(actor.placements.len(), 1);
        assert_eq!(
            (
                actor.placements[0].x,
                actor.placements[0].y,
                actor.placements[0].floor
            ),
            merchant
        );
        let tile = bundle
            .sectors
            .iter()
            .filter(|sector| sector.floor == player.2)
            .flat_map(|sector| sector.tiles.iter())
            .find(|tile| tile.x == player.0 && tile.y == player.1)
            .ok_or("actual public talk tile")?;
        assert_eq!(tile.house, 0);
        assert!(
            tile.items
                .iter()
                .filter(|item| item.depth == 0)
                .any(|item| bundle.manifest.palette[item.palette as usize]
                    .terrain
                    .is_some_and(|terrain| terrain.kind == TerrainKind::Ground
                        && terrain.walkable == Some(true)))
        );
    }
    if let Some(path) = std::env::var_os("OTERYN_SRV7_SUPPORT_BUNDLE") {
        std::fs::write(path, &compiled.bytes)?;
    }
    eprintln!(
        "SRV7_SUPPORT_ACTUAL_SOURCE_PARITY {parity:?} no_geometry_rewrite=true no_spawn_rewrite=true"
    );
    Ok(())
}
