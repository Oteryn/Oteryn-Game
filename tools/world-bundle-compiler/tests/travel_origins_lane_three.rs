use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{Input, realize_npcs, realize_spawns},
    source,
};
use std::{collections::BTreeSet, error::Error, path::Path};

#[test]
fn lane_three_travel_origins_are_realized_from_the_world_project() -> Result<(), Box<dyn Error>> {
    type Cell = (u16, u16, i8);
    let origins: &[(&str, &[Cell])] = &[
        ("melian", &[(32983, 31539, -1)]),
        ("nielson", &[(32233, 31675, -7)]),
        ("old_adall", &[(32629, 32771, -7)]),
        ("pemaret", &[(33286, 31955, -6)]),
        ("pino", &[(33193, 31782, -3)]),
        ("sebastian", &[(32349, 32624, -7)]),
        ("sebastian_nargor", &[(32027, 32814, -7)]),
        ("svenson", &[(32045, 31580, -7)]),
        ("tanyt", &[(33957, 31513, 0)]),
        (
            "thorgrin",
            &[
                (33025, 31553, -10),
                (33024, 31554, -12),
                (33024, 31554, -14),
            ],
        ),
        ("tonar", &[(33181, 32883, -7)]),
        ("tonar_oskayaat", &[(33067, 32917, -7)]),
        ("urks_the_mute", &[(33517, 32856, -14)]),
        ("uzon", &[(32536, 31836, -4)]),
        ("uzon_back", &[(32661, 31915, 0)]),
        ("ziyad", &[(33805, 32768, -2)]),
    ];
    let destinations = [
        ("melian", "marapur", (33805, 32766, 2)),
        ("pino", "marapur", (33805, 32766, 2)),
        ("sebastian", "nargor", (32024, 32813, 7)),
        ("tanyt", "marapur", (33805, 32766, 2)),
        ("tonar", "passage", (33070, 32916, 7)),
        ("uzon", "kazor", (32588, 31941, 0)),
        ("uzon", "kazordoon", (32588, 31941, 0)),
    ];
    let regions: BTreeSet<_> = origins
        .iter()
        .flat_map(|(_, cells)| cells.iter().map(|&(x, y, _)| (x / 256, y / 256)))
        .chain(
            destinations
                .iter()
                .map(|&(_, _, (x, y, _))| (x / 256, y / 256)),
        )
        .collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &regions.into_iter().collect::<Vec<_>>())?;
    let (registry, palette) = source::registry(&root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: source::world_extent(&root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (spawns, _) = realize_spawns(&input, &registry)?;
    let (table, report) = realize_npcs(&input, &registry, &spawns)?;
    for (stem, route, destination) in destinations {
        let service = format!("oteryn:service.travel.{stem}");
        let record = project
            .families
            .routes
            .iter()
            .find(|row| row.service == service && row.route == route)
            .ok_or_else(|| format!("missing route {service}/{route}"))?;
        assert_eq!(record.destination, destination, "{service}/{route}");
        assert!(
            !report
                .routes_held
                .iter()
                .any(|hold| hold.service == service && hold.route == route),
            "destination held for {service}/{route}",
        );
    }
    for (stem, cells) in origins {
        let key = format!("oteryn:npc.{stem}");
        let npc = table
            .npcs
            .iter()
            .find(|row| row.key == key)
            .ok_or_else(|| {
                format!(
                    "{key} absent from the bundle NPC table. Holds: {:?}",
                    report
                        .held
                        .iter()
                        .filter(|row| row.npc == key)
                        .collect::<Vec<_>>()
                )
            })?;
        let service = format!("oteryn:service.travel.{stem}");
        assert!(
            project.families.npcs[&key]
                .services
                .iter()
                .any(|reference| reference.key == service),
            "{key} does not reference {service}",
        );
        let actual: BTreeSet<_> = npc.placements.iter().map(|p| (p.x, p.y, p.floor)).collect();
        assert_eq!(actual, cells.iter().copied().collect(), "{key}");
    }
    Ok(())
}
