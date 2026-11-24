//! Source-qualified mainland and island NPCs must survive the World loader and NPC compiler.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{HoldReason, Input, realize_npcs, realize_spawns},
    source,
    spawn::Direction,
};
use serde::Deserialize;

const CONTACTS: &str = include_str!("../../content-schema/npc-authoring/mainland_contacts.json");

#[derive(Deserialize)]
struct Contact {
    npc: String,
    cell: (u16, u16, i8),
    direction: Option<String>,
    nearby: Option<Nearby>,
}

#[derive(Deserialize)]
struct Nearby {
    cell: (u16, u16, i8),
}

// Additional contacts for the same NPCs from the other NPC content lanes.
const PACKAGE_CONTACTS: &[(&str, (u16, u16, i8), Direction)] = &[
    ("oteryn:npc.barney", (32142, 32926, -7), Direction::North),
    (
        "oteryn:npc.dr_merlay",
        (33579, 31868, -12),
        Direction::North,
    ),
    ("oteryn:npc.dregor", (33539, 31950, -14), Direction::North),
    ("oteryn:npc.eathar", (33536, 31948, -13), Direction::North),
    ("oteryn:npc.eranth", (32121, 32953, -7), Direction::North),
    (
        "oteryn:npc.gnomfurry",
        (33529, 32902, -14),
        Direction::North,
    ),
    ("oteryn:npc.gnominer", (33520, 32881, -14), Direction::North),
    ("oteryn:npc.hexel", (33558, 31945, -14), Direction::North),
    ("oteryn:npc.jossel", (33566, 31948, -13), Direction::North),
    ("oteryn:npc.klesar", (33542, 31947, -14), Direction::North),
    (
        "oteryn:npc.lesser_messenger_of_heaven",
        (33194, 31644, -7),
        Direction::North,
    ),
    ("oteryn:npc.maeryn", (33330, 31686, -7), Direction::North),
    (
        "oteryn:npc.mr_brandon",
        (33548, 31864, -7),
        Direction::North,
    ),
    ("oteryn:npc.nicholas", (33640, 31898, -5), Direction::North),
    ("oteryn:npc.normod", (33550, 31939, -14), Direction::North),
    ("oteryn:npc.paolo", (32159, 32965, -7), Direction::North),
    ("oteryn:npc.peremin", (33550, 31963, -14), Direction::North),
];

#[test]
fn mainland_and_island_npcs_compile_at_qualified_cells() -> Result<(), Box<dyn std::error::Error>> {
    let contacts: BTreeMap<String, Vec<Contact>> = serde_json::from_str(CONTACTS)?;
    let mut expected = BTreeMap::new();
    let mut cells = BTreeSet::new();
    for contact in contacts.values().flatten() {
        let (x, y, z) = contact
            .nearby
            .as_ref()
            .map_or(contact.cell, |near| near.cell);
        let key = format!("oteryn:npc.{}", contact.npc);
        let direction = match contact.direction.as_deref() {
            None | Some("north") => Direction::North,
            Some("east") => Direction::East,
            Some("south") => Direction::South,
            Some("west") => Direction::West,
            Some(_) => return Err("unsupported source direction".into()),
        };
        assert!(expected.insert((key, (x, y, -z)), direction).is_none());
        assert!(cells.insert((x, y, z.unsigned_abs())));
    }
    assert_eq!(expected.len(), 975);
    for (key, cell, direction) in [
        ("a_beggar", (33597, 31953, -7), Direction::North),
        ("fadil", (33219, 32396, -7), Direction::North),
        ("gewen", (32588, 31942, 0), Direction::North),
        ("gnominus", (32804, 31758, -10), Direction::North),
        ("iptar_sin", (33924, 31476, -5), Direction::North),
        ("maeryn", (33332, 31686, -6), Direction::North),
        ("petaloid", (33825, 31982, -6), Direction::South),
    ] {
        assert_eq!(
            expected.get(&(format!("oteryn:npc.{key}"), cell)),
            Some(&direction)
        );
    }

    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let regions: Vec<_> = cells
        .iter()
        .map(|(x, y, _)| (x / 256, y / 256))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let project = source::project_regions(root, &regions)?;
    let (registry, palette) = source::registry(root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: source::world_extent(root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: Vec::new(),
        families: &project.families,
    };
    let (spawns, _) = realize_spawns(&input, &registry)?;
    let (table, report) = realize_npcs(&input, &registry, &spawns)?;
    let keys: BTreeSet<_> = expected.keys().map(|(key, _)| key.as_str()).collect();
    let realized: BTreeMap<_, _> = table
        .npcs
        .iter()
        .filter(|npc| keys.contains(npc.key.as_str()))
        .flat_map(|npc| {
            npc.placements
                .iter()
                .map(move |p| ((npc.key.clone(), (p.x, p.y, p.floor)), p.direction))
        })
        .collect();
    let missing: Vec<_> = expected
        .iter()
        .filter(|(entry, direction)| realized.get(*entry) != Some(*direction))
        .collect();
    assert!(
        missing.is_empty(),
        "{} missing or misoriented contacts (first 12: {:?}); held contacts: {:?}",
        missing.len(),
        &missing[..missing.len().min(12)],
        report
            .held
            .iter()
            .filter(|hold| keys.contains(hold.npc.as_str()))
            .collect::<Vec<_>>(),
    );
    for &(key, cell, direction) in PACKAGE_CONTACTS {
        assert!(expected.insert((key.to_owned(), cell), direction).is_none());
    }
    assert_eq!(realized, expected);
    assert!(
        !report
            .routes_held
            .iter()
            .any(|route| route.reason == HoldReason::NpcPlacement
                && cells.contains(&route.destination)),
        "the new NPCs must keep every existing travel arrival clear"
    );
    Ok(())
}
