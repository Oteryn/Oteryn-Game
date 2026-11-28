use super::*;
use oteryn_world_bundle_compiler::source;
use std::{collections::BTreeSet, path::Path};

#[test]
fn shadows_errand_contacts_are_realized_on_the_published_map() -> TestResult {
    let expected = BTreeSet::from([
        ("oteryn:npc.boveas", 32431, 32090, -15),
        ("oteryn:npc.cerdras", 32301, 31815, -6),
        ("oteryn:npc.dreadeye", 32800, 31572, -14),
        ("oteryn:npc.milos", 33258, 31823, -3),
        ("oteryn:npc.scutty", 32574, 31915, -12),
        ("oteryn:npc.telas", 33273, 31788, -11),
        ("oteryn:npc.telas", 33276, 31800, -6),
    ]);
    let regions: BTreeSet<_> = expected
        .iter()
        .map(|&(_, x, y, _)| (x / 256, y / 256))
        .collect();
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let project = source::project_regions(root, &regions.into_iter().collect::<Vec<_>>())?;
    let (resolver, palette) = source::registry(root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: source::world_extent(root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (spawns, _) = realize_spawns(&input, &resolver)?;
    let (npcs, report) = realize_npcs(&input, &resolver, &spawns)?;
    let keys: BTreeSet<_> = expected.iter().map(|&(key, ..)| key).collect();
    let actual: BTreeSet<_> = npcs
        .npcs
        .iter()
        .filter(|npc| keys.contains(npc.key.as_str()))
        .flat_map(|npc| {
            npc.placements
                .iter()
                .map(|p| (npc.key.as_str(), p.x, p.y, p.floor))
        })
        .collect();
    assert_eq!(
        actual,
        expected,
        "held placements: {:?}",
        report
            .held
            .iter()
            .filter(|held| keys.contains(held.npc.as_str()))
            .collect::<Vec<_>>()
    );
    Ok(())
}
