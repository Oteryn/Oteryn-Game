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

#[test]
fn carried_input_sets_are_bounded_unique_and_keep_legacy_hashes() -> Result<(), String> {
    let mut value: serde_json::Value = serde_json::from_str::<serde_json::Value>(include_str!(
        "../../../../../../content/quests/authored-world-callbacks.json"
    ))
    .map_err(|e| e.to_string())?["callbacks"][0]
        .clone();
    value["trigger"] = "use".into();
    value["effects"] = serde_json::json!({"transform":null,"facing":null,"magic":null,"portals":null,"item_cycle":null,"encounter":null,"carried_inputs":[
        {"item":"oteryn:item.tibia.i50336","count":1},
        {"item":"oteryn:item.tibia.i50337","count":1},
        {"item":"oteryn:item.tibia.i50338","count":1}
    ]});
    let doc = |c: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
    assert!(parse_with_provenance(&doc(&value), true).is_ok());
    for case in 0..8 {
        let mut bad = value.clone();
        match case {
            0=>bad["effects"]["carried_inputs"][0]["count"]=0.into(),
            1=>bad["effects"]["carried_inputs"][0]["count"]=101.into(),
            2=>bad["effects"]["carried_inputs"][0]["item"]="bad".into(),
            3=>bad["effects"]["carried_inputs"][1]=bad["effects"]["carried_inputs"][0].clone(),
            4=>bad["trigger"]="step_in".into(),
            5=>bad["storage"]=serde_json::json!([]),
            6=>bad["item_on_target"]=serde_json::json!({"item":"oteryn:item.tibia.i50336","count":1,"replacement":null,"target_replacement":null}),
            _=>bad["effects"]["carried_inputs"] = serde_json::Value::Array((1..=9).map(|id|serde_json::json!({"item":format!("oteryn:item.tibia.i{id}"),"count":1})).collect()),
        }
        assert!(
            parse_with_provenance(&doc(&bad), true).is_err(),
            "carried input case {case}"
        );
    }
    let legacy = CallbackEffects::default();
    assert!(
        serde_json::to_value(legacy)
            .map_err(|e| e.to_string())?
            .get("carried_inputs")
            .is_none()
    );
    Ok(())
}

#[test]
fn plain_carried_tool_and_reward_share_the_bounded_input_owner() -> Result<(), String> {
    let original = load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/authored-mystery-assembly")
        .ok_or("assembly")?;
    let value = serde_json::to_value(original).map_err(|e| e.to_string())?;
    let doc = |v: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[v]}).to_string();
    assert!(parse_with_provenance(&doc(&value), true).is_ok());
    for case in 0..4 {
        let mut bad = value.clone();
        match case {
            0 => bad["item_on_target"]["reusable"] = true.into(),
            1 => bad["item_on_target"]["replacement"] = "oteryn:item.tibia.i50336".into(),
            2 => bad["item_on_target"]["target_replacement"] = 231.into(),
            _ => bad["item_on_target"]["count"] = 0.into(),
        }
        assert!(
            parse_with_provenance(&doc(&bad), true).is_err(),
            "invalid crafting input {case}"
        );
    }
    Ok(())
}

#[test]
fn declared_tool_reach_is_bounded_same_floor_and_legacy_stable() -> Result<(), String> {
    let source = include_str!("../../../../../../content/quests/authored-world-callbacks.json");
    let mut row: serde_json::Value = serde_json::from_str::<serde_json::Value>(source)
        .map_err(|e| e.to_string())?["callbacks"]
        .as_array()
        .ok_or("callbacks")?
        .iter()
        .find(|c| c["key"] == "oteryn:quest-callback/authored-venore-machine-1")
        .ok_or("machine")?
        .clone();
    let doc = |r: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[r]}).to_string();
    let parsed = parse_with_provenance(&doc(&row), true)?;
    let callback = parsed.first().ok_or("callback")?;
    let target = Position {
        x: 10,
        y: 10,
        floor: -6,
    };
    assert!(callback.reaches(
        Position {
            x: 12,
            y: 11,
            floor: -6
        },
        target
    ));
    assert!(!callback.reaches(
        Position {
            x: 13,
            y: 10,
            floor: -6
        },
        target
    ));
    assert!(!callback.reaches(
        Position {
            x: 10,
            y: 10,
            floor: -7
        },
        target
    ));
    for case in 0..5 {
        let mut invalid = row.clone();
        match case {
            0 => invalid["use_range"] = 0.into(),
            1 => invalid["use_range"] = 3.into(),
            2 => invalid["item_on_target"] = serde_json::Value::Null,
            3 => {
                invalid["selector"]["position"] = serde_json::Value::Null;
                invalid["selector"]["unique"] = 1.into();
            }
            _ => invalid["storage"] = serde_json::json!([]),
        }
        assert!(
            parse_with_provenance(&doc(&invalid), true).is_err(),
            "bounded tool reach case {case}"
        );
    }
    row["use_range"] = 1.into();
    let explicit = parse_with_provenance(&doc(&row), true)?;
    row.as_object_mut().ok_or("row")?.remove("use_range");
    let legacy = parse_with_provenance(&doc(&row), true)?;
    assert_eq!(explicit, legacy);
    assert!(
        serde_json::to_value(&legacy[0])
            .map_err(|e| e.to_string())?
            .get("use_range")
            .is_none()
    );
    assert!(!legacy[0].reaches(
        Position {
            x: 12,
            y: 10,
            floor: -6
        },
        target
    ));
    Ok(())
}
