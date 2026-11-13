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

#[test]
fn every_machine_counterpart_keeps_one_catalogue_scoped_admission_owner() -> Result<(), String> {
    let loaded = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let mut checked = 0;
    for callback in load_for(catalogue)?
        .iter()
        .filter(|c| c.quest == "oteryn:quest.25_years_of_tibia_quest")
    {
        let transition = catalogue
            .transition(&callback.key)
            .ok_or("machine transition")?;
        crate::quest::admission_binding::validate_for_catalogue(catalogue, transition, true)?;
        assert!(
            crate::quest::admission_binding::validate_for_catalogue(catalogue, transition, false)
                .is_err()
        );
        checked += 1;
    }
    assert_eq!(checked, 18);
    Ok(())
}
