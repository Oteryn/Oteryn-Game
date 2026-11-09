use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{Input, realize_npcs, realize_spawns},
    project::TravelRoute,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn m38_anna_targuna_landing_uses_the_existing_real_source_admission() -> TestResult {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let destination = (31938, 31928, 7);
    let mut project = oteryn_world_bundle_compiler::source::project_regions(
        root,
        &[(124, 124), (127, 126), (127, 127)],
    )?;
    project.families.routes = vec![TravelRoute {
        service: "oteryn:service.travel.anna".into(),
        route: "targuna".into(),
        destination,
    }];
    let (resolver, palette) = oteryn_world_bundle_compiler::source::registry(root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: oteryn_world_bundle_compiler::source::world_extent(root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let compiled = oteryn_world_bundle_compiler::compile::compile(&input, &resolver)?;
    let omitted: Vec<_> = compiled
        .diagnostics
        .iter()
        .filter(|d| {
            d.floor == -7
                && [(32552_u16, 32497_u16), (31938, 31928), (32534, 32513)]
                    .iter()
                    .any(|(x, y)| d.x.abs_diff(*x) <= 3 && d.y.abs_diff(*y) <= 3)
        })
        .collect();
    assert!(
        omitted.is_empty(),
        "travel-anchor geometry omitted: {omitted:?}"
    );
    let (spawns, _) = realize_spawns(&input, &resolver)?;
    let (npcs, report) = realize_npcs(&input, &resolver, &spawns)?;
    let anna = npcs.npcs.iter().find(|npc| npc.key == "oteryn:npc.anna");
    assert!(
        anna.is_some(),
        "Anna source anchor refused: {:?}",
        report
            .held
            .iter()
            .filter(|held| held.npc == "oteryn:npc.anna")
            .collect::<Vec<_>>()
    );
    assert!(
        anna.ok_or("Anna")?
            .placements
            .iter()
            .any(|p| (p.x, p.y, p.floor) == (32552, 32497, -7))
    );
    assert_eq!(report.routes, 1);
    assert!(
        report.routes_held.is_empty(),
        "source Targuna landing refused: {:?}",
        report.routes_held
    );
    Ok(())
}
