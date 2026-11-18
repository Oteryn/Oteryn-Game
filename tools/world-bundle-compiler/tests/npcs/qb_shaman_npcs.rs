//! QB Shaman Outfits Quest: Hairycles and Chondur are admitted into the compiled world at their
//! single agreeing Canary/Crystal promotion-candidate cell, so the outfit and both addon
//! conversations are reachable.
//! Reference: TibiaWiki `Shaman_Outfits_Quest/Spoiler` (Hairycles in Banuta, Chondur in Sabrehaven).
use super::*;

/// (NPC key, x, y, floor) of the two Shaman outfit contacts.
const QB_SHAMAN_NPCS: &[(&str, u16, u16, u8)] = &[
    ("oteryn:npc.chondur", 32361, 32550, 7),
    ("oteryn:npc.hairycles", 32826, 32574, 6),
];

#[test]
fn qb_shaman_npcs_are_admitted_at_their_source_cells() -> TestResult {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    for (key, x, y, z) in QB_SHAMAN_NPCS {
        let project =
            oteryn_world_bundle_compiler::source::project_regions(root, &[(x / 256, y / 256)])?;
        let (resolver, palette) = oteryn_world_bundle_compiler::source::registry(root, &project)?;
        assert!(
            project
                .families
                .npc_placements
                .iter()
                .any(|p| p.npc == *key && p.cell == (*x, *y, *z)),
            "QB Shaman NPC {key} is not published at its source cell"
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
            "QB Shaman NPC {key} refused: {:?}",
            report
                .held
                .iter()
                .filter(|held| held.npc == *key)
                .collect::<Vec<_>>()
        );
        let placements = &npc.ok_or("admitted Shaman NPC")?.placements;
        assert_eq!(placements.len(), 1, "{key}");
        assert_eq!(
            (placements[0].x, placements[0].y, placements[0].floor),
            (*x, *y, -(i8::try_from(*z)?)),
            "{key}"
        );
    }
    Ok(())
}
