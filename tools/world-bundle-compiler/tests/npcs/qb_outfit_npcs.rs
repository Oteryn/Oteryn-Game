//! QB: outfit and addon hand-in NPCs are admitted into the compiled world at their single
//! agreeing Canary/Crystal promotion-candidate cell, so the source conversations are reachable.
use super::*;

/// (NPC key, x, y, floor) of every outfit hand-in contact this lane placed.
const QB_OUTFIT_NPCS: &[(&str, u16, u16, u8)] = &[
    // Mage and Summoner Outfits Quests: Angelina, Lynda, Myra and Zoltan.
    ("oteryn:npc.angelina", 32633, 32402, 10),
    // Hunter Outfits Quest: Legola (gloves); Elane was already placed.
    ("oteryn:npc.legola", 32298, 31785, 7),
    // Wizard Outfits Quest: Lugri (skull) and The Queen of the Banshees (flowers).
    ("oteryn:npc.lugri", 32389, 32118, 8),
    ("oteryn:npc.lynda", 32331, 32199, 7),
    ("oteryn:npc.myra", 32580, 32751, 6),
    ("oteryn:npc.the_queen_of_the_banshees", 32260, 31866, 14),
    ("oteryn:npc.zoltan", 33266, 31847, 4),
];

/// QB-2: further outfit, addon and reward-use hand-in contacts.
const QB2_OUTFIT_NPCS: &[(&str, u16, u16, u8)] = &[
    // Revenant Outfits.
    ("oteryn:npc.flickering_soul", 33621, 31419, 10),
    // Cave Explorer Outfits.
    ("oteryn:npc.gnommander", 32248, 32604, 8),
    // The Inquisition (Hand of the Inquisition Outfits): the guards of mission 1.
    ("oteryn:npc.grof_the_guard", 32372, 32182, 7),
    ("oteryn:npc.kulag_the_guard", 32287, 32264, 7),
    ("oteryn:npc.miles_the_guard", 32429, 32173, 5),
    ("oteryn:npc.tim_the_guard", 32425, 32223, 6),
    ("oteryn:npc.walter_the_guard", 32338, 32277, 7),
];

/// QB-3: mainland Citizen (Lubo, Hanna) and Warrior (Trisha, Cornelia) addon contacts.
const QB3_OUTFIT_NPCS: &[(&str, u16, u16, u8)] = &[
    ("oteryn:npc.cornelia", 32335, 31794, 7),
    ("oteryn:npc.hanna", 32408, 32220, 7),
    ("oteryn:npc.lubo", 32489, 32119, 7),
    ("oteryn:npc.trisha", 32346, 31749, 7),
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

#[test]
fn qb2_outfit_hand_in_npcs_are_admitted_at_their_source_cells() -> TestResult {
    let refused: Vec<String> = QB2_OUTFIT_NPCS
        .iter()
        .filter_map(|(key, x, y, z)| {
            std::panic::catch_unwind(|| admitted_at(key, *x, *y, *z))
                .map_err(|_| ())
                .and_then(|result| result.map_err(|_| ()))
                .err()
                .map(|()| (*key).to_owned())
        })
        .collect();
    assert!(refused.is_empty(), "QB-2 NPCs refused: {refused:?}");
    Ok(())
}

#[test]
fn qb3_outfit_hand_in_npcs_are_admitted_at_their_source_cells() -> TestResult {
    for (key, x, y, z) in QB3_OUTFIT_NPCS {
        admitted_at(key, *x, *y, *z)?;
    }
    Ok(())
}

#[test]
fn dream_outfit_contacts_compile_at_their_source_cells() -> TestResult {
    for (key, x, y, z) in [
        ("oteryn:npc.eruaran", 33618, 32372, 5),
        ("oteryn:npc.undal", 33710, 32113, 4),
        ("oteryn:npc.vanys", 33685, 32180, 8),
    ] {
        admitted_at(key, x, y, z)?;
    }
    Ok(())
}
