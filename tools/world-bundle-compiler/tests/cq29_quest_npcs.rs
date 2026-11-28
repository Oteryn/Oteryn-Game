use oteryn_world_bundle_compiler::bundle::{BuildClass, Identity};
use oteryn_world_bundle_compiler::compile::{Input, realize_npcs, realize_spawns};
use oteryn_world_bundle_compiler::source;
use std::path::Path;

#[test]
fn quest_contacts_compile_at_crystal_cells() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let expected = [
        ("ahmet", 33126, 32810, -6),
        ("bertram", 32355, 32797, -7),
        ("black_bert", 32340, 32206, -6),
        ("carina", 33015, 32048, -6),
        ("chantalle", 32363, 32792, -7),
        ("costello", 32177, 31936, -7),
        ("dalbrect", 32206, 31756, -6),
        ("dorian", 32318, 32209, -8),
        ("herbert", 32332, 32839, -7),
        ("nurik", 32931, 32071, -8),
        ("oswald", 32381, 32220, -7),
        ("sarina", 32336, 31807, -7),
        ("servant_sentry", 33312, 31836, -7),
        ("snake_eye", 32657, 32189, -8),
    ];
    let mut regions: Vec<_> = expected
        .iter()
        .map(|(_, x, y, _)| (x / 256, y / 256))
        .collect();
    regions.sort_unstable();
    regions.dedup();
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
    for (slug, x, y, floor) in expected {
        let key = format!("oteryn:npc.{slug}");
        assert!(
            npcs.npcs.iter().any(|npc| npc.key == key
                && npc
                    .placements
                    .iter()
                    .any(|p| (p.x, p.y, p.floor) == (x, y, floor))),
            "missing {key} at ({x}, {y}, {floor}): {:?}",
            report
                .held
                .iter()
                .filter(|row| row.npc == key)
                .collect::<Vec<_>>(),
        );
    }
    Ok(())
}
