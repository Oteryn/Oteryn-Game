use super::*;

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
