//! Independent actual-source shop census must have a native dialogue entry.
#[test]
fn srv7_thirty_furniture_shop_sources_have_native_greetings_and_offers()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../.."));
    let capture: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("tools/content-schema/native-gameplay/furniture-rules.json"),
    )?)?;
    let keys = capture["npc_wrapped_offers"]
        .as_array()
        .ok_or("source NPC offers")?
        .iter()
        .filter_map(|r| r["npc_key"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(keys.len(), 30, "closed source identity census");
    let catalogue = crate::content::NpcDataCatalogue::load(
        &root.join("content"),
        std::ffi::OsStr::new("world"),
        crate::content::npc_catalogue_preproduction_limits(),
    )
    .map_err(|e| format!("{e:?}"))?;
    let model = catalogue
        .service_model_for_project()
        .map_err(|e| format!("{e:?}"))?;
    let book = super::NpcTalkBook::from_model(&model);
    for key in keys {
        let entry = book
            .get(key)
            .ok_or_else(|| format!("current furniture shop has no native greeting: {key}"))?;
        assert!(!entry.offers().is_empty(), "{key}");
    }
    Ok(())
}
