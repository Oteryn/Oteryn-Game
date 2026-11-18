//! QB Barbarian Outfits Quest: Bron and Ajax are admitted into the compiled world at their single
//! agreeing Canary/Crystal promotion-candidate cell, so both addon conversations are reachable.
//! Reference: TibiaWiki `Barbarian_Outfits_Quest/Spoiler` (Bron and Ajax near Northport).
use super::*;

/// (NPC key, x, y, floor) of the two Barbarian addon contacts.
const QB_BARBARIAN_NPCS: &[(&str, u16, u16, u8)] = &[
    ("oteryn:npc.ajax", 32417, 31583, 7),
    ("oteryn:npc.bron", 32366, 31628, 7),
];

#[test]
fn qb_barbarian_npcs_are_admitted_at_their_source_cells() -> TestResult {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    for (key, x, y, z) in QB_BARBARIAN_NPCS {
        let project =
            oteryn_world_bundle_compiler::source::project_regions(root, &[(x / 256, y / 256)])?;
        let (resolver, palette) = oteryn_world_bundle_compiler::source::registry(root, &project)?;
        assert!(
            project
                .families
                .npc_placements
                .iter()
                .any(|p| p.npc == *key && p.cell == (*x, *y, *z)),
            "QB Barbarian NPC {key} is not published at its source cell"
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
        let npc = npcs.npcs.iter().find(|npc| npc.key == *key);
        assert!(
            npc.is_some(),
            "QB Barbarian NPC {key} refused: {:?}",
            report
                .held
                .iter()
                .filter(|held| held.npc == *key)
                .collect::<Vec<_>>()
        );
        let placements = &npc.ok_or("admitted Barbarian NPC")?.placements;
        assert_eq!(placements.len(), 1, "{key}");
        assert_eq!(
            (placements[0].x, placements[0].y, placements[0].floor),
            (*x, *y, -(i8::try_from(*z)?)),
            "{key}"
        );
    }
    Ok(())
}
