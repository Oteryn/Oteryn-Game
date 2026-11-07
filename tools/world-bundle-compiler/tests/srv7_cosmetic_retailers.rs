//! W18: actual production placement input, catalogue admission and original source ground.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{Input, realize_npcs, realize_spawns},
    project::{self, Families},
    resolve::Registry,
    source,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::{fs, path::Path};
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
fn shards(root: &Path, directory: &str, prefix: &str) -> TestResult<Vec<Vec<u8>>> {
    let mut paths = fs::read_dir(root.join(directory))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(prefix) && name.ends_with(".json"))
    });
    paths.sort();
    paths
        .into_iter()
        .map(|path| fs::read(path).map_err(Into::into))
        .collect()
}

#[test]
fn srv7_all_fifty_six_cosmetic_grant_npcs_have_original_retail_placements() -> TestResult {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let points = [
        ("oteryn:npc.emperor_kruzak", (32626, 31923, 3)),
        ("oteryn:npc.zoltan", (33266, 31847, 4)),
        ("oteryn:npc.undal", (33710, 32113, 4)),
        ("oteryn:npc.ezebeth", (33677, 31884, 5)),
        ("oteryn:npc.hugo", (32950, 32102, 5)),
        ("oteryn:npc.irmana", (32952, 32109, 5)),
        ("oteryn:npc.eruaran", (33618, 32372, 5)),
        ("oteryn:npc.cranky_lizard_crone", (33192, 31044, 6)),
        ("oteryn:npc.king_tibianus", (32311, 32172, 6)),
        ("oteryn:npc.ustan", (32580, 32757, 6)),
        ("oteryn:npc.hjaern", (32351, 31026, 7)),
        ("oteryn:npc.yalahari", (32801, 31188, 7)),
        ("oteryn:npc.palimuth", (32801, 31223, 7)),
        ("oteryn:npc.rock_steady", (33453, 31330, 7)),
        ("oteryn:npc.ajax", (32417, 31583, 7)),
        ("oteryn:npc.bron", (32366, 31628, 7)),
        ("oteryn:npc.queen_eloise", (32314, 31754, 7)),
        ("oteryn:npc.avar_tar", (33251, 31763, 7)),
        ("oteryn:npc.legola", (32302, 31782, 7)),
        ("oteryn:npc.percybald", (32380, 31786, 7)),
        ("oteryn:npc.cornelia", (32335, 31794, 7)),
        ("oteryn:npc.sandra", (33257, 31839, 7)),
        ("oteryn:npc.dedoras", (33336, 32014, 7)),
        ("oteryn:npc.lubo", (32489, 32119, 7)),
        ("oteryn:npc.appaloosa", (32852, 32124, 7)),
        ("oteryn:npc.bozo", (32313, 32183, 7)),
        ("oteryn:npc.sam", (32361, 32198, 7)),
        ("oteryn:npc.lynda", (32331, 32199, 7)),
        ("oteryn:npc.walter_jaeger", (32379, 32207, 7)),
        ("oteryn:npc.hanna", (32408, 32220, 7)),
        ("oteryn:npc.palomino", (32450, 32226, 7)),
        ("oteryn:npc.elane", (32342, 32235, 7)),
        ("oteryn:npc.muriel", (32295, 32263, 7)),
        ("oteryn:npc.razan", (33238, 32404, 7)),
        ("oteryn:npc.ishina", (33231, 32423, 7)),
        ("oteryn:npc.habdel", (33225, 32434, 7)),
        ("oteryn:npc.simon_the_beggar", (32186, 32471, 7)),
        ("oteryn:npc.miraia", (33238, 32482, 7)),
        ("oteryn:npc.chondur", (32361, 32550, 7)),
        ("oteryn:npc.raymond_striker", (32350, 32588, 7)),
        ("oteryn:npc.ariella", (32336, 32593, 7)),
        ("oteryn:npc.morgan", (32324, 32599, 7)),
        ("oteryn:npc.vescu", (32565, 32651, 7)),
        ("oteryn:npc.a_majestic_warwolf", (33355, 31991, 8)),
        ("oteryn:npc.lugri", (32389, 32118, 8)),
        ("oteryn:npc.vanys", (33685, 32180, 8)),
        ("oteryn:npc.henricus", (32316, 32267, 8)),
        ("oteryn:npc.gnommander", (32248, 32604, 8)),
        ("oteryn:npc.erayo", (32518, 32911, 8)),
        ("oteryn:npc.navigator", (33639, 31377, 9)),
        ("oteryn:npc.jack_springer", (32326, 32261, 9)),
        ("oteryn:npc.flickering_soul", (33621, 31419, 10)),
        ("oteryn:npc.cael", (33028, 31513, 10)),
        ("oteryn:npc.the_first_dragon", (33617, 31017, 13)),
        ("oteryn:npc.the_queen_of_the_banshees", (32260, 31866, 14)),
        ("oteryn:npc.gnomadness", (33527, 32865, 14)),
    ];
    let keys = points.iter().map(|(key, _)| *key).collect::<BTreeSet<_>>();
    let mut families = Families::default();
    for bytes in shards(&root, "content/npcs/definitions", "npcs-")? {
        families.add_npcs(&bytes)?;
    }
    for bytes in shards(&root, "content/dialogues/definitions", "dialogues-")? {
        families.add_dialogues(&bytes)?;
    }
    // Match source::add_npc_families: service references include promotion,
    // blessings and other admitted types, not only ordinary shop trades.
    let mut service_kinds = fs::read_dir(root.join("content/services"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    service_kinds.sort();
    for kind in service_kinds {
        let kind = kind
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("service kind")?;
        if let Some((_, shards)) = source::indexed(&root, &format!("content/services/{kind}/"))? {
            for bytes in shards {
                families.add_services(&bytes)?;
            }
        }
    }
    for bytes in shards(&root, "content/world/npc-placements", "placements-")? {
        families.add_npc_placements(&bytes)?;
    }
    families
        .npc_placements
        .retain(|p| keys.contains(p.npc.as_str()));
    assert_eq!(
        families.npc_placements.len(),
        56,
        "all56 original cosmetic NPC source placements must reach the real compiler input"
    );
    for (key, cell) in &points {
        assert!(
            families
                .npc_placements
                .iter()
                .any(|p| p.npc == *key && p.cell == *cell),
            "wrong/missing actual retailer source cell for{key}"
        );
    }
    let regions = points
        .iter()
        .map(|(_, (x, y, z))| {
            format!(
                "content/world/placements/region-z{z:02}-x{:03}-y{:03}.b3",
                x / 256,
                y / 256
            )
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|p| fs::read(root.join(p)))
        .collect::<Result<Vec<_>, _>>()?;
    let index: Value =
        serde_json::from_slice(&fs::read(root.join("content/world/placements/index.json"))?)?;
    let mut registry = Registry::default();
    for bytes in shards(&root, "content/items/definitions", "items-")? {
        registry.add_items(&bytes)?;
    }
    registry.add_item_profiles(&fs::read(
        root.join("content/items/definitions/spell-native-profiles.json"),
    )?)?;
    for (family, dir, prefix) in [
        ("Terrain", "content/world/terrain", "terrain-"),
        ("WorldObject", "content/world/objects", "objects-"),
    ] {
        for bytes in shards(&root, dir, prefix)? {
            registry.add_catalogue(family, &bytes)?;
        }
    }
    let palette = index["palette"]
        .as_array()
        .ok_or("palette")?
        .iter()
        .map(|entry| {
            let key = entry["key"].as_str().ok_or("palette key")?.to_owned();
            if entry["provisional"] == true {
                registry.add_provisional(key.clone());
            }
            Ok(key)
        })
        .collect::<TestResult<Vec<_>>>()?;
    registry.seal()?;
    let worlds = shards(&root, "content/world/worlds", "worlds-")?;
    let input = Input {
        regions: &regions,
        palette: &palette,
        identity: Identity::default(),
        world: project::world_extent(worlds.first().ok_or("World shard")?)?,
        build_class: BuildClass::NonProduction,
        draft_areas: vec![],
        families: &families,
    };
    let (spawns, _) = realize_spawns(&input, &registry)?;
    let (npcs, report) = realize_npcs(&input, &registry, &spawns)?;
    assert!(
        report.held.is_empty(),
        "real source NPC/ground admission cannot be bypassed: {report:#?}"
    );
    assert_eq!(npcs.placement_count(), 56);
    assert_eq!(npcs.npcs.len(), 56);
    eprintln!("SRV7_COSMETIC_NPC_SOURCE_PLACEMENTS all56=true geometry_rewritten=false");
    Ok(())
}
