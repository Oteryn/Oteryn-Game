use std::{collections::BTreeSet, path::Path};

use oteryn_world_bundle_compiler::source;
use serde::Deserialize;

use super::*;

#[derive(Deserialize)]
struct RegionalPlacements {
    placements: Vec<SourcePlacement>,
}

#[derive(Deserialize)]
struct SourcePlacement {
    npc: String,
    cell: (u16, u16, u8),
    adjacent: Option<AdjacentContact>,
    #[serde(default)]
    direction: u8,
}

#[derive(Deserialize)]
struct AdjacentContact {
    cell: (u16, u16, u8),
}

#[test]
fn regional_npcs_are_loaded_at_every_qualified_source_anchor() -> TestResult {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let selection: RegionalPlacements = serde_json::from_str(include_str!(
        "../../../content-schema/npc-authoring/regional_placements.json"
    ))?;
    assert_eq!(selection.placements.len(), 400);

    let project = source::project(root)?;
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
    let loaded: BTreeSet<_> = table
        .npcs
        .iter()
        .flat_map(|npc| {
            npc.placements.iter().map(|p| {
                (
                    npc.key.as_str(),
                    (p.x, p.y, p.floor.unsigned_abs()),
                    p.direction as u8,
                )
            })
        })
        .collect();
    let missing: Vec<_> = selection
        .placements
        .iter()
        .map(|p| {
            (
                format!("oteryn:npc.{}", p.npc),
                p.adjacent.as_ref().map_or(p.cell, |contact| contact.cell),
                p.direction,
            )
        })
        .filter(|(key, cell, direction)| !loaded.contains(&(key.as_str(), *cell, *direction)))
        .collect();
    assert!(
        missing.is_empty(),
        "{} missing placements, first: {:?}. Compiler holds: {:?}",
        missing.len(),
        missing.first(),
        report.held
    );
    let blocked_routes: Vec<_> = report
        .routes_held
        .iter()
        .filter(|hold| hold.reason == HoldReason::NpcPlacement)
        .collect();
    assert!(
        blocked_routes.is_empty(),
        "NPCs obstruct travel arrivals: {blocked_routes:?}"
    );
    Ok(())
}
