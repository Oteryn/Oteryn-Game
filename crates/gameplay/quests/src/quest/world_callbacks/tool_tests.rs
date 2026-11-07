use super::*;

#[test]
fn carried_tool_presence_keeps_legacy_plans_and_rejects_replacement() -> Result<(), String> {
    let mut callback: serde_json::Value = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../../../../../content/quests/authored-world-callbacks.json"
    ))
    .map_err(|e| e.to_string())?["callbacks"][0]
        .clone();
    callback["key"] = "oteryn:quest-callback/authored-tool-test".into();
    callback["trigger"] = "use".into();
    callback["selector"]["appearance"] = 355.into();
    callback["selector"]["ordinal"] = 0.into();
    callback["item_on_target"] = serde_json::json!({"item":"oteryn:item.tibia.i3456","count":1,"replacement":null,"target_replacement":432,"reusable":true});
    let doc = |c: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
    assert!(
        parse_with_provenance(&doc(&callback), true).is_ok(),
        "owned tool presence must parse"
    );
    callback["item_on_target"]["replacement"] = "oteryn:item.tibia.i3456".into();
    assert!(
        parse_with_provenance(&doc(&callback), true).is_err(),
        "presence cannot burn/remint the same tool"
    );
    callback["item_on_target"]["replacement"] = serde_json::Value::Null;
    for count in [0, 101] {
        callback["item_on_target"]["count"] = count.into();
        assert!(parse_with_provenance(&doc(&callback), true).is_err());
    }
    let old:ItemOnTarget=serde_json::from_value(serde_json::json!({"item":"oteryn:item.tibia.i3456","count":1,"replacement":null,"target_replacement":432})).map_err(|e|e.to_string())?;
    assert!(
        serde_json::to_value(old)
            .map_err(|e| e.to_string())?
            .get("reusable")
            .is_none(),
        "old definition hashes stay stable"
    );
    Ok(())
}

#[test]
fn carried_prerequisites_and_arrived_travel_are_bounded_and_legacy_stable() -> Result<(), String> {
    let mut callback: serde_json::Value = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../../../../../content/quests/authored-world-callbacks.json"
    ))
    .map_err(|e| e.to_string())?["callbacks"][0]
        .clone();
    callback["key"] = "oteryn:quest-callback/authored-presence-test".into();
    let document =
        |c: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
    let original = parse_with_provenance(&document(&callback), true)?;
    let encoded = serde_json::to_value(&original[0]).map_err(|e| e.to_string())?;
    assert!(encoded.get("present").is_none());
    assert!(encoded.get("required_travel").is_none());
    callback["present"] = serde_json::json!([{"item":"oteryn:item.tibia.i9205","count":1}]);
    callback["required_travel"] =
        serde_json::json!([{"route":"oteryn:service.travel.captain_waverider/treasure_island"}]);
    assert!(parse_with_provenance(&document(&callback), true).is_ok());
    let valid = callback.clone();
    callback["present"][0]["count"] = 0.into();
    assert!(parse_with_provenance(&document(&callback), true).is_err());
    callback = valid.clone();
    callback["present"] = serde_json::json!([valid["present"][0], valid["present"][0]]);
    assert!(parse_with_provenance(&document(&callback), true).is_err());
    callback = valid.clone();
    callback["required_travel"] = serde_json::json!([{"route":"oteryn:npc.captain_waverider"}]);
    assert!(parse_with_provenance(&document(&callback), true).is_err());
    callback = valid;
    callback["storage"] = serde_json::json!([]);
    assert!(parse_with_provenance(&document(&callback), true).is_err());
    Ok(())
}

#[test]
fn read_only_native_step_entry_has_no_unchecked_value_or_premium_effects() -> Result<(), String> {
    let mut callback: serde_json::Value = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../../../../../content/quests/world-callbacks.json"
    ))
    .map_err(|e| e.to_string())?["callbacks"]
        .as_array()
        .ok_or("callbacks")?
        .iter()
        .find(|row| row["key"] == "oteryn:quest-callback/secret-library-main-descent")
        .ok_or("library gate")?
        .clone();
    let document =
        |c: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
    assert!(parse(&document(&callback)).is_ok());
    callback["trigger"] = "step_out".into();
    assert!(parse(&document(&callback)).is_err());
    callback["trigger"] = "step_in".into();
    callback["premium_required"] = true.into();
    assert!(parse(&document(&callback)).is_err());
    callback["premium_required"] = false.into();
    callback["storage"][0]["read_only"] = false.into();
    assert!(
        parse(&document(&callback)).is_err(),
        "mixed writable predicates cannot bypass the writer"
    );
    Ok(())
}
