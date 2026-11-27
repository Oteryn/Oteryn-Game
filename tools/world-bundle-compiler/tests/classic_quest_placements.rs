use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use oteryn_world_bundle::{bundle::TILE_LIMITS, sector::Budget};
use oteryn_world_bundle_compiler::{
    b3,
    bundle::{BuildClass, Identity},
    compile::{Input, realize_npcs, realize_spawns},
    source,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn classic_quest_startup_selectors_load_from_committed_regions() -> TestResult {
    let unique = [
        ((32769, 32225, 7), 4240, 6168),
        ((32852, 32332, 7), 2469, 6145),
        ((32176, 32132, 9), 4240, 5014),
        ((32182, 32145, 11), 2772, 30029),
        ((32177, 32148, 11), 5107, 22006),
        ((32175, 32145, 11), 4240, 6162),
        ((32174, 32149, 11), 4240, 6163),
        ((32860, 32516, 11), 2472, 6192),
        ((32154, 31116, 9), 2472, 6219),
        ((32649, 31969, 9), 2472, 6146),
        ((32650, 31969, 9), 2472, 6147),
        ((32651, 31969, 9), 2472, 6148),
        ((32411, 32155, 15), 2434, 5009),
        ((32578, 32195, 14), 2472, 6080),
        ((32575, 32195, 14), 2472, 6081),
        ((32572, 32195, 14), 2472, 6082),
        ((33049, 32399, 10), 1983, 6183),
        ((32130, 32066, 12), 2473, 6120),
        ((32127, 32065, 12), 2473, 6121),
        ((32124, 32064, 12), 2473, 5026),
        ((32576, 32216, 15), 4240, 5016),
    ];
    let action = [
        ((32179, 32149, 10), 5106, 4603),
        ((32393, 32136, 14), 6251, 3620),
        ((32395, 32117, 15), 5097, 3620),
        ((32397, 32114, 15), 5097, 3620),
        ((32400, 32117, 15), 5097, 3620),
        ((32578, 32197, 15), 1631, 3666),
        ((32592, 32102, 14), 5106, 3667),
        ((33032, 32398, 11), 9565, 1060),
        ((33037, 32398, 11), 9565, 1060),
    ];
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let selected: Vec<_> = unique
        .iter()
        .chain(&action)
        .map(|&((x, y, _), _, _)| (x / 256, y / 256))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let project = source::project_regions(&root, &selected)?;
    let expected: BTreeMap<_, _> = unique
        .into_iter()
        .map(|(cell, appearance, tag)| (cell, (appearance, Some(tag), None)))
        .chain(
            action
                .into_iter()
                .map(|(cell, appearance, tag)| (cell, (appearance, None, Some(tag)))),
        )
        .collect();
    let mut observed = BTreeMap::new();
    for bytes in project.regions {
        let region = b3::decode_region(
            &bytes,
            TILE_LIMITS,
            &mut Budget {
                tiles: 65_536,
                entries: 1_000_000,
            },
        )?;
        let z = region.z;
        for (_, _, tiles) in region.sectors {
            for tile in tiles {
                let cell = (tile.x, tile.y, z);
                let Some(&(appearance, _, _)) = expected.get(&cell) else {
                    continue;
                };
                for entry in tile.items {
                    if entry.depth == 0
                        && project.index["palette"][entry.palette as usize]["source_item_id"]
                            == appearance
                    {
                        assert!(
                            observed
                                .insert(cell, (appearance, entry.attrs.unique, entry.attrs.action))
                                .is_none()
                        );
                    }
                }
            }
        }
    }
    assert_eq!(observed, expected);
    Ok(())
}

#[test]
fn classic_quest_wiki_readables_and_pickups_load_with_source_attributes() -> TestResult {
    let expected = BTreeMap::from([
        (
            (32769, 32224, 7, 3506),
            (
                None,
                Some("I suspect Porgol stole your books on necromancy."),
            ),
        ),
        (
            (32855, 32331, 7, 2838),
            (
                None,
                Some("Passage across the Plains of Havoc has become too dangerous."),
            ),
        ),
        ((32575, 32197, 14, 3290), (None, None)),
        ((32575, 32197, 14, 268), (None, None)),
        ((32592, 32098, 14, 236), (Some(2), None)),
        ((32592, 32098, 14, 3276), (None, None)),
        ((32592, 32098, 14, 3352), (None, None)),
        ((32592, 32098, 14, 3358), (None, None)),
    ]);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(127, 125), (128, 125), (128, 126)])?;
    let mut seen = BTreeSet::new();
    for bytes in project.regions {
        let region = b3::decode_region(
            &bytes,
            TILE_LIMITS,
            &mut Budget {
                tiles: 65_536,
                entries: 1_000_000,
            },
        )?;
        for (_, _, tiles) in region.sectors {
            for tile in tiles {
                if !expected
                    .keys()
                    .any(|&(x, y, z, _)| (tile.x, tile.y, region.z) == (x, y, z))
                {
                    continue;
                }
                for entry in tile.items {
                    let palette = &project.index["palette"][entry.palette as usize];
                    let appearance = palette["source_item_id"].as_u64().ok_or("Tibia ID")?;
                    let key = (tile.x, tile.y, region.z, appearance);
                    let Some(&(count, text)) = expected.get(&key) else {
                        continue;
                    };
                    assert!(seen.insert(key));
                    assert_eq!(entry.depth, 0);
                    assert_eq!(tile.house, 0);
                    assert_eq!(palette["provisional"], false);
                    assert_eq!(entry.attrs.count, count);
                    assert_eq!(entry.attrs.text.is_some(), text.is_some());
                    if let Some(text) = text {
                        assert!(
                            entry
                                .attrs
                                .text
                                .as_deref()
                                .is_some_and(|body| body.contains(text))
                        );
                    }
                }
            }
        }
    }
    assert_eq!(seen, expected.into_keys().collect());
    Ok(())
}

#[test]
fn classic_quest_contacts_realize_at_crystal_npc_cells() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let expected = BTreeMap::from([
        ("oteryn:npc.a_prisoner", (32394, 32138, -13)),
        ("oteryn:npc.arito", (33070, 32885, -6)),
        ("oteryn:npc.gregor", (32406, 32201, -6)),
        ("oteryn:npc.sam", (32361, 32198, -7)),
    ]);
    let selected = [(126, 125), (129, 128)];
    let mut project = source::project_regions(&root, &selected)?;
    project
        .families
        .npc_placements
        .retain(|p| expected.contains_key(p.npc.as_str()));
    project.families.routes.clear();
    for spawn in &mut project.families.spawns {
        spawn
            .points
            .retain(|p| selected.contains(&(p.cell.0 / 256, p.cell.1 / 256)));
    }
    project.families.spawns.retain(|s| !s.points.is_empty());
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
    let (npcs, report) = realize_npcs(&input, &registry, &spawns)?;
    assert!(report.held.is_empty(), "{report:#?}");
    let realized: BTreeMap<_, _> = npcs
        .npcs
        .iter()
        .flat_map(|npc| {
            npc.placements
                .iter()
                .map(move |p| (npc.key.as_str(), (p.x, p.y, p.floor)))
        })
        .collect();
    assert_eq!(realized, expected);
    assert_eq!(npcs.placement_count(), expected.len());
    Ok(())
}
