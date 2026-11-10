use super::*;

#[test]
fn source_stationary_swan_is_admitted_at_its_exact_water_anchor() -> TestResult {
    exact_contact("oteryn:npc.a_swan", (33185, 31732, 7), false)
}

#[test]
fn source_yselda_is_admitted_only_at_its_retained_page_bound_contact() -> TestResult {
    exact_contact("oteryn:npc.yselda", (32619, 32488, 12), true)
}

#[test]
fn source_spy_rat_is_admitted_only_at_its_retained_page_bound_contact() -> TestResult {
    exact_contact("oteryn:npc.eustacios_spy_rat", (33857, 31333, 9), false)
}

fn exact_contact(key: &str, (x, y, z): (u16, u16, u8), requires_opt_in: bool) -> TestResult {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let project =
        oteryn_world_bundle_compiler::source::project_regions(root, &[(x / 256, y / 256)])?;
    let (resolver, palette) = oteryn_world_bundle_compiler::source::registry(root, &project)?;
    assert!(
        project
            .families
            .npc_placements
            .iter()
            .any(|p| p.npc == key && p.cell == (x, y, z)),
        "M38 source contact is not published"
    );
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: oteryn_world_bundle_compiler::source::world_extent(root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (spawns, _) = realize_spawns(&input, &resolver)?;
    let (npcs, report) = realize_npcs(&input, &resolver, &spawns)?;
    let swan = npcs.npcs.iter().find(|npc| npc.key == key);
    assert!(
        swan.is_some(),
        "source contact refused: {:?}",
        report
            .held
            .iter()
            .filter(|held| held.npc == key)
            .collect::<Vec<_>>()
    );
    let placements = &swan.ok_or("admitted source contact")?.placements;
    assert_eq!(placements.len(), 1);
    assert_eq!(
        (placements[0].x, placements[0].y, placements[0].floor),
        (x, y, -(z as i8))
    );
    if requires_opt_in {
        let mut ordinary_families = project.families.clone();
        ordinary_families.npc_non_walkable_contacts.clear();
        let ordinary = Input {
            families: &ordinary_families,
            ..input
        };
        let (ordinary_npcs, report) = realize_npcs(&ordinary, &resolver, &spawns)?;
        assert!(!ordinary_npcs.npcs.iter().any(|npc| npc.key == key));
        assert!(
            report
                .held
                .iter()
                .any(|held| held.npc == key && held.reason == HoldReason::NotWalkable)
        );
    }
    Ok(())
}
