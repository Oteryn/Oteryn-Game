use super::*;
use oteryn_world_bundle_compiler::source;
use std::collections::BTreeSet;
use std::path::Path;

#[test]
fn crystal_npc_contacts_are_admitted_at_their_world_anchors() -> TestResult {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let expected = [
        ("a_sleeping_dragon", (33242, 31213, 10)),
        ("brom", (32703, 31832, 12)),
        ("brutus", (33153, 32984, 8)),
        ("death_npc", (32570, 32574, 11)),
        ("doctor_marrow", (34010, 32641, 6)),
        ("ellipses", (33449, 32805, 11)),
        ("eustacios_spy_rat", (33857, 31334, 9)),
        ("habdel", (33225, 32434, 7)),
        ("harlow", (32836, 31364, 7)),
        ("kepar", (32193, 32297, 6)),
        ("palimuth", (32801, 31223, 7)),
        ("rafzan", (33052, 32095, 6)),
        ("raymond_striker", (32350, 32588, 7)),
        ("roughington", (33535, 31832, 9)),
        ("shadowpunch", (33377, 31832, 9)),
        ("sinatuki", (32425, 31085, 7)),
        ("skullfrost", (32549, 32622, 11)),
        ("the_first_dragon", (33617, 31017, 13)),
        ("victor", (32255, 32216, 8)),
        ("wyrdins_apprentice", (32434, 31985, 7)),
        ("zalamon", (33354, 31410, 8)),
    ];
    let regions = expected
        .iter()
        .map(|&(_, (x, y, _))| (x / 256, y / 256))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let project = source::project_regions(root, &regions)?;
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
    let mut missing = Vec::new();
    for (name, (x, y, z)) in expected {
        let key = format!("oteryn:npc.{name}");
        if !npcs.npcs.iter().any(|npc| {
            npc.key == key
                && npc
                    .placements
                    .iter()
                    .any(|p| (p.x, p.y, p.floor) == (x, y, -(z as i8)))
        }) {
            missing.push((key, (x, y, z)));
        }
    }
    assert!(missing.is_empty(), "missing Crystal contacts: {missing:?}");
    assert!(
        !report
            .held
            .iter()
            .any(|hold| { hold.npc == "oteryn:npc.habdel" && hold.cell == (12072, 12043, 7) })
    );
    Ok(())
}
