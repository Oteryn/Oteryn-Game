use super::*;

fn source() -> Result<serde_json::Value, String> {
    let callback = load()?
        .into_iter()
        .find(|callback| callback.key.ends_with("/paradox-ghoul-switch"))
        .ok_or("source timed switch")?;
    serde_json::to_value(callback).map_err(|error| error.to_string())
}
fn author(callback: serde_json::Value) -> Result<Vec<Callback>, String> {
    parse(&serde_json::json!({"schema":1,"callbacks":[callback]}).to_string())
}
#[test]
fn spawned_only_cleanup_is_explicit_and_preserves_legacy_receipts() -> Result<(), String> {
    let legacy = source()?;
    let mut owned = legacy.clone();
    owned["effects"]["encounter"]["timing"]["cleanup_spawned_only"] = true.into();
    let callbacks = author(owned)?;
    let encoded = serde_json::to_value(&callbacks[0]).map_err(|error| error.to_string())?;
    assert_eq!(
        encoded["effects"]["encounter"]["timing"]["cleanup_spawned_only"],
        true
    );
    let callbacks = author(legacy.clone())?;
    assert_eq!(
        serde_json::to_value(&callbacks[0]).map_err(|error| error.to_string())?,
        legacy
    );
    Ok(())
}
#[test]
fn timed_spawn_does_not_relax_ordinary_party_entry() -> Result<(), String> {
    let mut callback = source()?;
    author(callback.clone())?;
    callback["effects"]["encounter"]
        .as_object_mut()
        .ok_or("encounter")?
        .remove("timing");
    assert!(
        author(callback.clone()).is_err(),
        "zero-party spawning requires an explicit timing owner"
    );
    callback["effects"]["encounter"]["players"] = serde_json::json!([{"from":{"x":32481,"y":31904,"floor":-5},"to":{"x":32478,"y":31901,"floor":-5}}]);
    let party = author(callback)?;
    assert!(
        party[0]
            .effects
            .encounter
            .as_ref()
            .ok_or("party")?
            .timing
            .is_none()
    );
    let encoded = serde_json::to_value(&party[0]).map_err(|error| error.to_string())?;
    assert!(
        encoded["effects"]["encounter"].get("timing").is_none(),
        "legacy party serialization omits the new descriptor"
    );
    Ok(())
}
#[test]
fn timed_authoring_refuses_unbounded_delays_and_mixed_party_authority() -> Result<(), String> {
    for case in 0..4 {
        let mut callback = source()?;
        match case {
            0 => {
                callback["effects"]["encounter"]["timing"]["reset_after_ms"] =
                    serde_json::json!(2000)
            }
            1 => {
                callback["effects"]["encounter"]["timing"]["spawn_after_ms"] =
                    serde_json::json!(60001)
            }
            2 => {
                callback["effects"]["encounter"]["room_min"] =
                    serde_json::json!({"x":0,"y":0,"floor":-5});
                callback["effects"]["encounter"]["room_max"] =
                    serde_json::json!({"x":65535,"y":65535,"floor":-5});
            }
            _ => {
                callback["effects"]["encounter"]["players"] = serde_json::json!([{"from":{"x":32481,"y":31904,"floor":-5},"to":{"x":32478,"y":31901,"floor":-5}}])
            }
        }
        assert!(
            author(callback).is_err(),
            "unqualified timed descriptor {case}"
        );
    }
    Ok(())
}
