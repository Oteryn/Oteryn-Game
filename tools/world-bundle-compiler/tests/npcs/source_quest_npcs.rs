//! Native world admission for quest contacts at their donor NPC XML cells.
use super::*;

#[test]
fn sams_old_backpack_kroox_is_admitted_at_the_source_cell() -> TestResult {
    admitted_at("oteryn:npc.kroox", (32651, 31887, 9))
}

#[test]
fn druid_outfit_contacts_are_admitted_at_the_source_cells() -> TestResult {
    admitted_at("oteryn:npc.ustan", (32580, 32757, 6))?;
    admitted_at("oteryn:npc.ceiron", (32665, 31717, 7))?;
    admitted_at("oteryn:npc.a_majestic_warwolf", (33355, 31991, 8))
}

#[test]
fn nightmare_exchange_contact_is_admitted_at_the_source_cell() -> TestResult {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let candidates: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json"),
    )?)?;
    let key = "oteryn:npc.the_dream_master";
    let candidate = candidates["candidates"]
        .as_array()
        .ok_or("NPC candidates")?
        .iter()
        .find(|candidate| candidate["identity"]["key"] == key)
        .ok_or("Dream Master placement candidate")?;
    let [placement] = candidate["placements"]
        .as_array()
        .ok_or("Dream Master source placements")?
        .as_slice()
    else {
        return Err("expected one Dream Master source placement".into());
    };
    let cell: SourceCell = serde_json::from_value(placement["position"].clone())?;
    admitted_at(key, (cell.x, cell.y, cell.z))
}

#[derive(serde::Deserialize)]
struct SourceCell {
    x: u16,
    y: u16,
    z: u8,
}

fn admitted_at(key: &str, cell: (u16, u16, u8)) -> TestResult {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    let project = oteryn_world_bundle_compiler::source::project_regions(
        root,
        &[(cell.0 / 256, cell.1 / 256)],
    )?;
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
    let (spawns, _) = realize_spawns(&input, &resolver)?;
    let (npcs, report) = realize_npcs(&input, &resolver, &spawns)?;
    let npc = npcs.npcs.iter().find(|npc| npc.key == key).ok_or_else(|| {
        format!(
            "{key} not admitted at {cell:?}: {:?}",
            report.held.iter().find(|held| held.npc == key)
        )
    })?;
    assert_eq!(npc.placements.len(), 1);
    let placement = &npc.placements[0];
    assert_eq!(
        (placement.x, placement.y, placement.floor),
        (cell.0, cell.1, -i8::try_from(cell.2)?)
    );
    Ok(())
}
