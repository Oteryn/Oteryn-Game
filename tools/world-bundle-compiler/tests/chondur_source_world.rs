//! Original48 Stampor points in their three declared source regions; no spawn/map rewrite.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{self, Input},
    source,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn srv7_chondur_stampor_original_source_materials_and_npc_world_export() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project =
        source::project_regions(&root, &[(125, 121), (129, 121), (129, 122), (126, 127)])?;
    let declared: std::collections::BTreeSet<_> = project
        .families
        .spawns
        .iter()
        .flat_map(|s| s.points.iter())
        .filter(|p| p.creature == "oteryn:creature.stampor")
        .map(|p| (p.cell.0, p.cell.1, p.cell.2, p.respawn_ms))
        .collect();
    assert_eq!(
        declared.len(),
        48,
        "exact existing source Stampor population, not newly invented points"
    );
    let (registry, palette) = source::registry(&root, &project)?;
    let input = Input {
        regions: &project.regions,
        palette: &palette,
        identity: Identity::default(),
        world: source::world_extent(&root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: vec![],
        families: &project.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    let parity = compile::equivalence(&input, &registry, &compiled.bytes)?;
    let bundle = oteryn_world_bundle::bundle::read(&compiled.bytes)?;
    let chondur = bundle
        .npcs
        .npcs
        .iter()
        .find(|npc| npc.key == "oteryn:npc.chondur")
        .ok_or("original source Chondur must be admitted")?;
    assert_eq!(chondur.placements.len(), 1);
    let pose = &chondur.placements[0];
    assert_eq!((pose.x, pose.y, pose.floor), (32361, 32550, -7));
    let mut admitted = std::collections::BTreeSet::new();
    for s in &bundle.spawns.sources {
        for p in &s.points {
            if bundle
                .spawns
                .creatures
                .get(p.creature as usize)
                .is_some_and(|c| c.key == "oteryn:creature.stampor")
            {
                assert!(s.floor < 0);
                admitted.insert((p.x, p.y, u8::try_from(-s.floor)?, p.respawn_ms));
            }
        }
    }
    assert!(
        !admitted.is_empty(),
        "Stampor must have a real source population under accepted fixed-cell policy"
    );
    assert!(
        admitted.is_subset(&declared),
        "no source relocation/respawn rewrite"
    );
    if let Some(path) = std::env::var_os("OTERYN_SRV7_CHONDUR_BUNDLE") {
        std::fs::write(path, &compiled.bytes)?;
    }
    eprintln!(
        "SRV7_STAMPOR_SOURCE_EXPORT declared={} admitted={} refused_under_native_policy={} parity={parity:?}",
        declared.len(),
        admitted.len(),
        declared.len() - admitted.len()
    );
    Ok(())
}
