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
