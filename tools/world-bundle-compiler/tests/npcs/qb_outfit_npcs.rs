//! QB: outfit and addon hand-in NPCs are admitted into the compiled world at their single
//! agreeing Canary/Crystal promotion-candidate cell, so the source conversations are reachable.
use super::*;

/// (NPC key, x, y, floor) of every outfit hand-in contact this lane placed.
const QB_OUTFIT_NPCS: &[(&str, u16, u16, u8)] = &[
    // Mage and Summoner Outfits Quests: Angelina, Lynda, Myra, Sandra and Zoltan.
    ("oteryn:npc.angelina", 32633, 32402, 10),
    // Assassin Outfits Quest: Vescu (base), Erayo (first addon), Atrad (second addon).
    ("oteryn:npc.atrad", 32077, 32533, 9),
    ("oteryn:npc.erayo", 32518, 32911, 8),
    // Beggar Outfits Quest: Hugo (base), Simon the Beggar (addons).
    ("oteryn:npc.hugo", 32950, 32102, 5),
    // Hunter Outfits Quest: Legola (gloves); Elane was already placed.
    ("oteryn:npc.legola", 32298, 31785, 7),
    // Wizard Outfits Quest: Lugri (skull) and The Queen of the Banshees (flowers).
    ("oteryn:npc.lugri", 32389, 32118, 8),
    ("oteryn:npc.lynda", 32331, 32199, 7),
    ("oteryn:npc.myra", 32580, 32751, 6),
    ("oteryn:npc.sandra", 33257, 31839, 7),
    ("oteryn:npc.simon_the_beggar", 32186, 32471, 7),
    ("oteryn:npc.the_queen_of_the_banshees", 32260, 31866, 14),
    ("oteryn:npc.vescu", 32565, 32651, 7),
    ("oteryn:npc.zoltan", 33266, 31847, 4),
];

fn admitted_at(key: &str, x: u16, y: u16, z: u8) -> TestResult {
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
        "QB outfit NPC {key} is not published at its source cell"
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
    let npc = npcs.npcs.iter().find(|npc| npc.key == key);
    assert!(
        npc.is_some(),
        "QB outfit NPC {key} refused: {:?}",
        report
            .held
            .iter()
            .filter(|held| held.npc == key)
            .collect::<Vec<_>>()
    );
    let placements = &npc.ok_or("admitted outfit NPC")?.placements;
    assert_eq!(placements.len(), 1, "{key}");
    assert_eq!(
        (placements[0].x, placements[0].y, placements[0].floor),
        (x, y, -(i8::try_from(z)?)),
        "{key}"
    );
    Ok(())
}

#[test]
fn qb_outfit_hand_in_npcs_are_admitted_at_their_source_cells() -> TestResult {
    for (key, x, y, z) in QB_OUTFIT_NPCS {
        admitted_at(key, *x, *y, *z)?;
    }
    Ok(())
}
