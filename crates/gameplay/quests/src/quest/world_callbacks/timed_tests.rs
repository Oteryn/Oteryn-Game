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
fn wave_source() -> Result<serde_json::Value, String> {
    let mut value = source()?;
    value["effects"]["encounter"]["timing"]["cleanup_spawned_only"] = true.into();
    value["effects"]["encounter"]["timing"]["exclusive_room"] = true.into();
    value["effects"]["encounter"]["waves"] = serde_json::json!([
        {"after_ms":4000,"monsters":[{"position":{"x":32479,"y":31900,"floor":-5},"creature":"oteryn:creature.rat"}]},
        {"after_ms":6000,"monsters":[{"position":{"x":32479,"y":31900,"floor":-5},"creature":"oteryn:creature.ghoul"}]}
    ]);
    Ok(value)
}
#[test]
fn timed_waves_admit_bounded_source_owned_sequences() -> Result<(), String> {
    let value = wave_source()?;
    let admitted = author(value.clone())?;
    assert_eq!(
        serde_json::to_value(&admitted[0]).map_err(|e| e.to_string())?,
        value
    );
    Ok(())
}
#[test]
fn timed_waves_refuse_unowned_unordered_and_unbounded_plans() -> Result<(), String> {
    let value = wave_source()?;
    for field in ["cleanup_spawned_only", "exclusive_room"] {
        let mut bad = value.clone();
        bad["effects"]["encounter"]["timing"][field] = false.into();
        assert!(author(bad).is_err(), "waves require {field}");
    }
    for deadline in [0, 2000, 6000, 100000] {
        let mut bad = value.clone();
        bad["effects"]["encounter"]["waves"][0]["after_ms"] = deadline.into();
        assert!(author(bad).is_err(), "invalid wave deadline {deadline}");
    }
    for count in [0, 9] {
        let mut bad = value.clone();
        let monster = bad["effects"]["encounter"]["waves"][0]["monsters"][0].clone();
        bad["effects"]["encounter"]["waves"][0]["monsters"] = (0..count)
            .map(|index| {
                let mut next = monster.clone();
                next["position"]["x"] = (32476 + index % 6).into();
                next["position"]["y"] = (31900 + index / 6).into();
                next
            })
            .collect::<Vec<_>>()
            .into();
        assert!(author(bad).is_err(), "invalid wave population {count}");
    }
    let mut bad = value.clone();
    bad["effects"]["encounter"]["waves"][0]["monsters"][0]["position"]["x"] = 1.into();
    assert!(author(bad).is_err(), "spawns stay in the bounded room");
    let mut maximum = value.clone();
    let wave = value["effects"]["encounter"]["waves"][0].clone();
    maximum["effects"]["encounter"]["waves"] = (0..7)
        .map(|index| {
            let mut next = wave.clone();
            next["after_ms"] = (4000 + index * 2000).into();
            next
        })
        .collect::<Vec<_>>()
        .into();
    author(maximum.clone())?;
    maximum["effects"]["encounter"]["waves"]
        .as_array_mut()
        .ok_or("waves")?
        .push({
            let mut excess = wave;
            excess["after_ms"] = 18000.into();
            excess
        });
    assert!(
        author(maximum).is_err(),
        "at most eight waves including the initial wave"
    );
    Ok(())
}
#[test]
fn exclusive_room_is_explicit_and_preserves_legacy_receipts() -> Result<(), String> {
    let legacy = source()?;
    let mut exclusive = legacy.clone();
    exclusive["effects"]["encounter"]["timing"]["exclusive_room"] = true.into();
    let callbacks = author(exclusive)?;
    assert_eq!(
        serde_json::to_value(&callbacks[0]).map_err(|e| e.to_string())?["effects"]["encounter"]["timing"]
            ["exclusive_room"],
        true
    );
    assert_eq!(
        serde_json::to_value(&author(legacy.clone())?[0]).map_err(|e| e.to_string())?,
        legacy
    );
    Ok(())
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
