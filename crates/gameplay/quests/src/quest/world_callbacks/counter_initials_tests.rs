use super::*;

#[test]
fn declared_counter_initial_is_used_without_changing_legacy_defaults() -> Result<(), String> {
    let callback = load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/qm03/pits")
        .ok_or("Pits callback")?;
    let track = callback.storage[0].track.clone();
    let mut value = serde_json::to_value(callback).map_err(|e| e.to_string())?;
    value["track_initials"] = serde_json::json!({track.clone(): 0});
    let parse_one = |value| parse(&serde_json::json!({"schema":1,"callbacks":[value]}).to_string());
    let callbacks = parse_one(value.clone())?;
    let empty =
        || QuestStateCatalogue::new("counter-test", vec![], vec![]).map_err(|e| format!("{e:?}"));
    let catalogue = extend_callbacks(empty()?, &callbacks)?;
    let declared = catalogue.track(&track).ok_or("declared track")?;
    assert_eq!((declared.initial, declared.min, declared.max), (0, 0, 1));
    value
        .as_object_mut()
        .ok_or("object")?
        .remove("track_initials");
    let legacy = extend_callbacks(empty()?, &parse_one(value.clone())?)?;
    assert_eq!(legacy.track(&track).ok_or("legacy track")?.initial, -1);
    assert!(
        extend_callbacks(legacy, &callbacks).is_err(),
        "an explicit default must not rewrite an existing declaration"
    );
    value["track_initials"] = serde_json::json!({"foreign-track":0});
    assert!(parse_one(value.clone()).is_err());
    value["track_initials"] = serde_json::json!({track.clone():2});
    assert!(parse_one(value.clone()).is_err());
    value["track_initials"] = serde_json::json!({track:0});
    value["storage"][0]["at_least"] = false.into();
    value["storage"][0]["preserve"] = true.into();
    assert!(parse_one(value).is_err());
    Ok(())
}
