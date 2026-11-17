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

#[test]
fn selected_tool_routes_share_a_selector_only_when_storage_guards_are_disjoint()
-> Result<(), String> {
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/world-callbacks/l6-tools.json"
    ))
    .map_err(|e| e.to_string())?;
    let mut a = parse(&source.to_string())?.remove(0);
    a.requires_world_admission = true;
    a.item_on_target.as_mut().ok_or("tool")?.selected_instance = true;
    a.storage[0].minimum = -1;
    a.storage[0].maximum = -1;
    a.storage[0].set = 0;
    let mut b = a.clone();
    b.key.push_str("-next");
    b.storage[0].minimum = 0;
    b.storage[0].maximum = 0;
    b.storage[0].set = 1;
    assert!(validate_registry(&[a.clone(), b.clone()]).is_ok());
    b.storage[0].minimum = -1;
    assert!(
        validate_registry(&[a.clone(), b.clone()]).is_err(),
        "overlapping branches remain ambiguous"
    );
    b.storage[0].minimum = 0;
    b.requires_world_admission = false;
    assert!(
        validate_registry(&[a, b]).is_err(),
        "unfenced branches cannot share a source"
    );
    Ok(())
}
