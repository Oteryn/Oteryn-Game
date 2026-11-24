use std::collections::BTreeSet;
use std::path::Path;

use oteryn_world_bundle_compiler::source;

use super::*;

#[test]
fn crystal_travel_origins_are_admitted_to_the_bundle() -> TestResult {
    let expected = [
        ("captain_seahorse", (33175, 31763, 6)),
        ("carlson", (32022, 31692, 7)),
        ("cornell", (33304, 31720, 7)),
        ("cornell", (33342, 31691, 7)),
        ("gewen", (32588, 31942, 0)),
        ("gurbasch", (33311, 31988, 15)),
        ("imbul", (32558, 32781, 7)),
        ("iyad", (32251, 31097, 4)),
        ("jack_fate", (32286, 32891, 6)),
        ("junkar_mines", (32556, 32069, 10)),
        ("junkar_robsons", (32525, 32038, 14)),
        ("junkar_thais", (32368, 32181, 13)),
        ("kendra_thais", (32261, 32262, 7)),
        ("lorek", (32679, 32775, 7)),
        ("maris", (32648, 31292, 6)),
        ("maris_fenrock", (32564, 31313, 7)),
    ];
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let regions: BTreeSet<_> = expected
        .iter()
        .map(|(_, (x, y, _))| (x / 256, y / 256))
        .collect();
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
    let (table, report) = realize_npcs(&input, &resolver, &spawns)?;
    let missing: Vec<_> = expected
        .into_iter()
        .filter(|(name, (x, y, z))| {
            let key = format!("oteryn:npc.{name}");
            !table.npcs.iter().any(|npc| {
                npc.key == key
                    && npc.placements.iter().any(|p| {
                        (p.x, p.y, p.floor, p.direction) == (*x, *y, -*z, Direction::North)
                    })
            })
        })
        .collect();
    assert!(
        missing.is_empty(),
        "missing Crystal origins: {missing:?}; compiler holds: {:?}",
        report
            .held
            .iter()
            .filter(|hold| {
                missing
                    .iter()
                    .any(|(name, _)| hold.npc.strip_prefix("oteryn:npc.") == Some(*name))
            })
            .collect::<Vec<_>>()
    );
    Ok(())
}
