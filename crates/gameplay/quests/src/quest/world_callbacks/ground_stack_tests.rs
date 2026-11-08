use super::*;

#[test]
fn m02_ground_stack_predicate_preserves_order_and_requires_a_durable_player_callback()
-> Result<(), String> {
    // Nomads Land, chosen recipe s4: the window is obstructed by actual carried
    // crates/amphorae, not an immovable appearance projection.
    let callback = load()?
        .into_iter()
        .find(|c| {
            c.actor_kind == ActorKind::Player
                && c.has_personal_state()
                && c.native_rule.is_none()
                && c.state_ops.is_empty()
                && !c.opening_only
                && !c.reversible
                && c.trigger != Trigger::StepOut
                && c.world_requirements.is_empty()
        })
        .ok_or("personal callback")?;
    let mut callback = serde_json::to_value(callback).map_err(|e| e.to_string())?;
    callback["world_requirements"] = serde_json::json!([{
        "position":{"x":100,"y":200,"floor":-7},"appearance":100,
        "ground_stack":[{"item":"oteryn:item.tibia.i3031","quantity":2},
                        {"item":"oteryn:item.tibia.i3035","quantity":1}]
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
        serde_json::json!([{"item":"","quantity":1}]),
        serde_json::json!([{"item":"oteryn:item.tibia.i3031","quantity":0}]),
        serde_json::json!([{"item":"oteryn:item.tibia.i3031","quantity":1,"ignored":true}]),
    ] {
        let mut bad = callback.clone();
        bad["world_requirements"][0]["ground_stack"] = stack;
        assert!(parse(&wire(&bad)).is_err());
    }
    let mut reversible = callback.clone();
    reversible["reversible"] = true.into();
    assert!(parse(&wire(&reversible)).is_err());
    callback["actor_kind"] = "monster".into();
    assert!(parse(&wire(&callback)).is_err());
    Ok(())
}
