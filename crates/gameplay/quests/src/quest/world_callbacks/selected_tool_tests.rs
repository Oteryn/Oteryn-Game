use super::*;

#[test]
fn selected_world_tool_requires_fenced_admission_without_changing_legacy_bindings()
-> Result<(), String> {
    let mut source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/world-callbacks/l6-tools.json"
    ))
    .map_err(|e| e.to_string())?;
    let legacy = parse(&source.to_string())?;
    let encoded = serde_json::to_value(&legacy[0]).map_err(|e| e.to_string())?;
    assert!(encoded["item_on_target"].get("selected_instance").is_none());
    source["callbacks"][0]["requires_world_admission"] = true.into();
    source["callbacks"][0]["item_on_target"]["selected_instance"] = true.into();
    let selected = parse(&source.to_string())?;
    assert_eq!(
        serde_json::to_value(&selected[0]).map_err(|e| e.to_string())?["item_on_target"]["selected_instance"],
        true
    );
    source["callbacks"][0]["requires_world_admission"] = false.into();
    assert!(
        parse(&source.to_string()).is_err(),
        "selected custody needs the fenced World owner"
    );
    source["callbacks"][0]["item_on_target"]["selected_instance"] = false.into();
    assert!(
        parse(&source.to_string()).is_ok(),
        "legacy retained tools keep their admission"
    );
    Ok(())
}
