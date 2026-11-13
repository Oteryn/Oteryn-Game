use super::*;

fn repair() -> Result<serde_json::Value, String> {
    let mut document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/world-callbacks/m07-machinery-burn.json"
    ))
    .map_err(|e| e.to_string())?;
    let mut callback = document["callbacks"][0].clone();
    callback["effects"] = serde_json::json!({"transform":1271});
    callback["item_on_target"]["reusable"] = false.into();
    callback["item_on_target"]["count"] = 7.into();
    let rear = callback["world_requirements"][1].clone();
    callback["passages"] = serde_json::json!([{
        "position":rear["position"],"appearance":rear["appearance"],
        "replacement":1271,"replacement_passable":false
    }]);
    let source = callback["selector"]["native"].clone();
    callback["passages"]
        .as_array_mut()
        .ok_or("passage list")?
        .push(serde_json::json!({
            "position":source["position"],"appearance":source["appearance"],
            "replacement":1271,"replacement_passable":false
        }));
    document["callbacks"] = serde_json::json!([callback]);
    Ok(document)
}

#[test]
fn m07_solid_repairs_admit_exact_fenced_native_consuming_trials() -> Result<(), String> {
    let callbacks = parse(&repair()?.to_string())?;
    let encoded = serde_json::to_value(&callbacks[0]).map_err(|e| e.to_string())?;
    assert_eq!(encoded["passages"][0]["replacement_passable"], false);
    Ok(())
}

#[test]
fn m07_solid_repairs_reject_unbounded_or_legacy_collision_changes() -> Result<(), String> {
    for variant in 0..8 {
        let mut document = repair()?;
        let callback = &mut document["callbacks"][0];
        match variant {
            0 => callback["item_on_target"]["reusable"] = true.into(),
            1 => callback["selector"]["native"] = serde_json::Value::Null,
            2 => callback["passages"][0]["replacement_passable"] = true.into(),
            3 => callback["passages"][0]["replacement"] = serde_json::Value::Null,
            4 => callback["passages"][0]["if_present"] = true.into(),
            5 => callback["world_requirements"] = serde_json::json!([]),
            6 => callback["reversible"] = true.into(),
            _ => {
                callback["passages"].as_array_mut().ok_or("passages")?.pop();
            }
        }
        assert!(parse(&document.to_string()).is_err(), "variant {variant}");
    }
    Ok(())
}
