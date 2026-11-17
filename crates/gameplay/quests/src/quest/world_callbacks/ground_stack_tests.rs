use super::*;

#[test]
fn m02_durable_recheck_uses_shared_ground_stack_wire() -> Result<(), String> {
    // Nomads Land, chosen recipe s4: the window is obstructed by actual carried
    // crates/amphorae, not an immovable appearance projection.
    let mut callback = load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/l6/waterskin-of-mead")
        .ok_or("native personal callback")?;
    callback.effects = Default::default();
    callback.requires_world_admission = true;
    let mut callback = serde_json::to_value(callback).map_err(|e| e.to_string())?;
    callback["world_requirements"] = serde_json::json!([{
        "position":{"x":100,"y":200,"floor":-7},"appearance":100,
        "ground_stack":[{"item":"oteryn:item.tibia.i3031","count":2},
                        {"item":"oteryn:item.tibia.i3035","count":1}]
    }]);
    let wire = |c: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
    let parsed = parse(&wire(&callback))?;
    let roundtrip = serde_json::to_value(&parsed[0]).map_err(|e| e.to_string())?;
    assert_eq!(
        roundtrip["world_requirements"],
        callback["world_requirements"]
    );
    for stack in [
        serde_json::json!([]),
        serde_json::json!([{"item":"","count":1}]),
        serde_json::json!([{"item":"oteryn:item.tibia.i3031","count":0}]),
        serde_json::json!([{"item":"oteryn:item.tibia.i3031","count":1,"ignored":true}]),
    ] {
        let mut bad = callback.clone();
        bad["world_requirements"][0]["ground_stack"] = stack;
        assert!(parse(&wire(&bad)).is_err());
    }
    let mut competing = callback.clone();
    competing["world_requirements"][0]["ground_stack"][0] =
        serde_json::json!({"item":"oteryn:item.tibia.i3031","quantity":2});
    assert!(
        parse(&wire(&competing)).is_err(),
        "only SRV7 count schema is accepted"
    );
    callback["actor_kind"] = "monster".into();
    assert!(parse(&wire(&callback)).is_err());
    Ok(())
}
