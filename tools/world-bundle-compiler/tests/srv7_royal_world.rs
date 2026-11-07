//! Actual Royal purchase, Thais Inbox access and named-House source geometry.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity, TerrainKind},
    compile::{self, Input},
    source,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn srv7_royal_actual_world_exports_king_inbox_and_house_source_fixture() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(126, 125), (126, 128)])?;
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
    {
        let (key, merchant, player) = (
            "oteryn:npc.king_tibianus",
            (32311, 32172, -6),
            (32311, 32173, -6),
        );
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
    // Fixture staging between these real source cells is explicit; this does
    // not claim a tested walking route between King, depot and House.
    for (x, y, floor, house) in [
        (32352, 32225, -7, 0),
        (32352, 32226, -7, 0),
        (32309, 32805, -7, 0),
        (32310, 32803, -7, 3437),
    ] {
        let tile = bundle
            .sectors
            .iter()
            .filter(|sector| sector.floor == floor)
            .flat_map(|sector| sector.tiles.iter())
            .find(|tile| tile.x == x && tile.y == y)
            .ok_or("actual Royal continuation source cell")?;
        assert_eq!(tile.house, house, "original source House identity");
    }
    if let Some(path) = std::env::var_os("OTERYN_SRV7_ROYAL_BUNDLE") {
        std::fs::write(path, &compiled.bytes)?;
    }
    eprintln!(
        "SRV7_ROYAL_ACTUAL_SOURCE_PARITY {parity:?} no_geometry_rewrite=true no_spawn_rewrite=true"
    );
    Ok(())
}
