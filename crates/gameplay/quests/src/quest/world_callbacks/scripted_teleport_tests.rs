use super::*;

#[test]
fn scripted_teleport_is_explicit_native_player_content_and_legacy_defaults_are_identical()
-> Result<(), String> {
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/authored-bindings/m03-25-years-time-machine.json"
    ))
    .map_err(|e| e.to_string())?;
    let mut value = document["quests"][0]["stages"][0]["trigger"]["callback"].clone();
    value["effects"] = serde_json::json!({"scripted_teleport":true});
    let parse_one = |value| parse(&serde_json::json!({"schema":1,"callbacks":[value]}).to_string());
    parse_one(value.clone())?;
    value["teleport"] = serde_json::Value::Null;
    assert!(
        parse_one(value.clone()).is_err(),
        "scripted landing requires a destination"
    );
    value = document["quests"][0]["stages"][0]["trigger"]["callback"].clone();
    value["effects"] = serde_json::json!({"scripted_teleport":true});
    value["selector"]["native"] = serde_json::Value::Null;
    assert!(
        parse_one(value.clone()).is_err(),
        "no unbound scripted placement"
    );
    value = document["quests"][0]["stages"][0]["trigger"]["callback"].clone();
    value["effects"] = serde_json::json!({"scripted_teleport":false});
    let explicit = parse_one(value.clone())?;
    value
        .as_object_mut()
        .ok_or("callback object")?
        .remove("effects");
    let legacy = parse_one(value)?;
    assert_eq!(
        explicit, legacy,
        "omitting the opt-in retains legacy callback hashes"
    );
    Ok(())
}
