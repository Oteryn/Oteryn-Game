//! Original Edron source region for the actual Hugo deposit caller.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{self, Input},
    source,
};
#[test]
fn srv7_simon_lifetime_actual_source_world_export() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(
        &root,
        &[
            (125, 125),
            (126, 125),
            (127, 125),
            (128, 125),
            (125, 126),
            (126, 126),
            (127, 126),
            (128, 126),
        ],
    )?;
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
        .find(|npc| npc.key == "oteryn:npc.hugo")
        .ok_or("actual Hugo sourceNPC")?;
    assert!(
        placements
            .placements
            .iter()
            .any(|p| (p.x, p.y, p.floor) == (32950, 32102, -5))
    );
    assert!(bundle.npcs.npcs.iter().any(|npc| {
        npc.key == "oteryn:npc.simon_the_beggar"
            && npc
                .placements
                .iter()
                .any(|p| (p.x, p.y, p.floor) == (32186, 32471, -7))
    }));
    assert!(
        bundle.npcs.npcs.iter().any(|npc| {
            npc.key == "oteryn:npc.captain_fearless"
                && npc
                    .placements
                    .iter()
                    .any(|p| (p.x, p.y, p.floor) == (32957, 32021, -6))
        }),
        "the existing eight source regions include the genuine Venore captain"
    );
    let boat_talk = bundle
        .sectors
        .iter()
        .filter(|s| s.floor == -6)
        .flat_map(|s| s.tiles.iter())
        .find(|t| (t.x, t.y) == (32956, 32021))
        .ok_or("actual captain neighbor")?;
    assert_eq!(boat_talk.items.len(), 1);
    let boat_ground = &bundle.manifest.palette[boat_talk.items[0].palette as usize];
    assert_eq!(boat_ground.key, "oteryn:item.tibia.i408");
    assert!(boat_ground.terrain.is_some_and(|t| t.kind
        == oteryn_world_bundle_compiler::bundle::TerrainKind::Ground
        && t.walkable == Some(true)));
    let talk = bundle
        .sectors
        .iter()
        .filter(|sector| sector.floor == -5)
        .flat_map(|sector| sector.tiles.iter())
        .find(|tile| (tile.x, tile.y) == (32951, 32103))
        .ok_or("actual Hugo talk tile")?;
    assert_eq!(
        talk.items.len(),
        1,
        "source neighbor is untouched single Ground, no unknown decorative collision"
    );
    let ground = &bundle.manifest.palette[talk.items[0].palette as usize];
    assert_eq!(ground.key, "oteryn:item.tibia.i17516");
    assert!(ground.terrain.is_some_and(|terrain| terrain.kind
        == oteryn_world_bundle_compiler::bundle::TerrainKind::Ground
        && terrain.walkable == Some(true)));
    for (x, y, z, ground_id) in [
        (32208, 32370, 10, 354),
        (32198, 32367, 9, 351),
        (32179, 32367, 8, 353),
    ] {
        let tile = bundle
            .sectors
            .iter()
            .filter(|s| s.floor == -z)
            .flat_map(|s| s.tiles.iter())
            .find(|t| (t.x, t.y) == (x, y))
            .ok_or("actual tunnel ladder source cell")?;
        let keys: Vec<_> = tile
            .items
            .iter()
            .filter(|i| i.depth == 0)
            .map(|i| bundle.manifest.palette[i.palette as usize].key.as_str())
            .collect();
        assert_eq!(
            keys,
            vec![
                format!("oteryn:item.tibia.i{ground_id}").as_str(),
                "oteryn:item.tibia.i1948"
            ]
        );
        let destination = bundle
            .sectors
            .iter()
            .filter(|s| s.floor == -(z - 1))
            .flat_map(|s| s.tiles.iter())
            .find(|t| (t.x, t.y) == (x, y + 1))
            .ok_or("actual preferred south ladder destination")?;
        let ground = &bundle.manifest.palette[destination.items[0].palette as usize];
        assert!(ground.terrain.is_some_and(|t| t.kind
            == oteryn_world_bundle_compiler::bundle::TerrainKind::Ground
            && t.walkable == Some(true)));
    }
    if let Some(path) = std::env::var_os("OTERYN_SRV7_SIMON_LIFETIME_BUNDLE") {
        std::fs::write(path, &built.bytes)?;
    }
    Ok(())
}
