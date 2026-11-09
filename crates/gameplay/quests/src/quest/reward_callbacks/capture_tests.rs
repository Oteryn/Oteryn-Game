use super::*;

#[test]
fn capture_policy_binds_live_occurrences_and_preserves_legacy_hashes() -> Result<(), String> {
    let live = load()?
        .into_iter()
        .find(|c| c.creature_target.is_some() && !c.fresh_corpse)
        .ok_or("live callback")?;
    let original = gated_binding(&live)?;
    let mut row = serde_json::to_value(&live).map_err(|e| e.to_string())?;
    assert!(row.get("capture_creature").is_none());
    row["capture_creature"] = true.into();
    let document =
        |row: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[row]}).to_string();
    let capture = parse(&document(&row))?.remove(0);
    assert_ne!(gated_binding(&capture)?, original);
    row["fresh_corpse"] = true.into();
    assert!(
        parse(&document(&row)).is_err(),
        "capture cannot consume a corpse"
    );
    row["fresh_corpse"] = false.into();
    row["creature_target"] = serde_json::Value::Null;
    assert!(
        parse(&document(&row)).is_err(),
        "capture needs an exact native creature"
    );
    Ok(())
}

#[test]
fn creature_capture_requires_the_selected_tool_and_pins_capture_semantics() -> Result<(), String> {
    let mut document: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/quests/npc-callbacks/m13-rotworm-sample.json"
    )))
    .map_err(|e| e.to_string())?;
    let original = parse(&document.to_string())?.remove(0);
    document["callbacks"][0]["capture_creature"] = true.into();
    let capture = parse(&document.to_string())?.remove(0);
    assert_ne!(gated_binding(&original)?, gated_binding(&capture)?);
    assert_eq!(
        serde_json::to_value(&capture).map_err(|e| e.to_string())?["capture_creature"],
        true
    );
    for mutation in 0..3 {
        let mut invalid = document.clone();
        match mutation {
            0 => invalid["callbacks"][0]["creature_target"] = serde_json::Value::Null,
            1 => invalid["callbacks"][0]["carried_item"] = serde_json::Value::Null,
            _ => invalid["callbacks"][0]["give"]["count"] = 2.into(),
        }
        assert!(
            parse(&invalid.to_string()).is_err(),
            "invalid capture terms {mutation}"
        );
    }
    Ok(())
}
