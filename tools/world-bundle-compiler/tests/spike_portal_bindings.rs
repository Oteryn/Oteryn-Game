//! Real source startup attributes, not fixture-created transport or relocated map objects.
use oteryn_world_bundle_compiler::{
    bundle::{BuildClass, Identity},
    compile::{self, Input},
    source,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn srv7_spike_existing17_source_portals_retain_actions_and_real_handlers() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let capture: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("tools/content-schema/world-authoring/samples/spike-portal-bindings.json"),
    )?)?;
    let rows = capture["bindings"].as_array().ok_or("source bindings")?;
    assert_eq!(rows.len(), 17);
    let project = source::project_regions(&root, &[(125, 127)])?;
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
    let bundle = oteryn_world_bundle::bundle::read(&compiled.bytes)?;
    let mut stamps = Vec::new();
    for sector in &bundle.sectors {
        for tile in &sector.tiles {
            for item in &tile.items {
                if item.depth == 0
                    && item
                        .attrs
                        .action
                        .is_some_and(|a| (4227..=4243).contains(&a))
                {
                    stamps.push((
                        i64::from(tile.x),
                        i64::from(tile.y),
                        -i64::from(sector.floor),
                        i64::from(item.attrs.action.ok_or("source action")?),
                        bundle.manifest.palette[item.palette as usize].key.clone(),
                    ));
                }
            }
        }
    }
    let mut expected = Vec::new();
    for row in rows {
        let p = &row["position"];
        expected.push((
            p[0].as_i64().ok_or("source x")?,
            p[1].as_i64().ok_or("source y")?,
            p[2].as_i64().ok_or("source z")?,
            row["action"].as_i64().ok_or("source action")?,
            format!(
                "oteryn:item.tibia.i{}",
                row["item"].as_u64().ok_or("source item")?
            ),
        ));
    }
    stamps.sort();
    expected.sort();
    assert_eq!(
        stamps, expected,
        "all17existing source objects need real startup action attributes"
    );
    let callbacks: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/quests/world-callbacks.json"),
    )?)?;
    for row in rows {
        let key = format!(
            "oteryn:quest-callback/spike-portal-{}",
            row["action"].as_u64().ok_or("action")?
        );
        assert_eq!(
            callbacks["callbacks"]
                .as_array()
                .ok_or("actual handlers")?
                .iter()
                .filter(|c| c["key"] == key
                    && c["trigger"] == "step_in"
                    && c["selector"]["action"] == row["action"]
                    && c["teleport"].is_object())
                .count(),
            1
        );
    }
    eprintln!(
        "SRV7_SPIKE_REAL_SOURCE_PORTAL_STAMPS count={} no_new_objects=true",
        stamps.len()
    );
    Ok(())
}
