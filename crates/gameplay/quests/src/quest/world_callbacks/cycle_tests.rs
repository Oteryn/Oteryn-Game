use super::*;

#[test]
fn cycle_reward_requires_a_preserved_nonnegative_local_epoch() -> Result<(), String> {
    let mut callback = load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/l2-djinn-tear-fountain")
        .ok_or("existing World reward")?;
    assert!(
        serde_json::to_value(&callback.effects)
            .map_err(|e| e.to_string())?
            .get("item_reward_cycle")
            .is_none(),
        "legacy wire bytes omit the new optional field"
    );
    let epoch = "oteryn:quest-progress/cycle-fixture/epoch";
    callback.effects.item_reward_cycle = Some(epoch.into());
    callback.storage.push(Storage {
        set_if: None,
        add: None,
        read_only: false,
        track: epoch.into(),
        minimum: 0,
        maximum: 99,
        set: 0,
        preserve: true,
        increment: false,
        at_least: false,
    });
    let document = |c: &Callback| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
    parse(&document(&callback))?;
    for mutation in 0..4 {
        let mut changed = callback.clone();
        match mutation {
            0 => changed.effects.item_reward = None,
            1 => changed.effects.item_reward_cycle = Some("oteryn:quest-progress/missing".into()),
            2 => changed.storage.last_mut().ok_or("epoch")?.preserve = false,
            _ => changed.storage.last_mut().ok_or("epoch")?.minimum = -1,
        }
        assert!(
            parse(&document(&changed)).is_err(),
            "invalid cycle source {mutation}"
        );
    }
    Ok(())
}

#[test]
fn oramond_five_native_roots_reopen_only_on_daily_epoch() -> Result<(), String> {
    let callbacks: Vec<_> = load()?
        .into_iter()
        .filter(|c| c.quest == "oteryn:quest.oramond_quest")
        .collect();
    assert_eq!(callbacks.len(), 5);
    let lowered = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    let catalogue = lowered.catalogue();
    let clock = catalogue
        .track("oteryn:quest-progress/oramond/native-roots/last-start")
        .ok_or("daily clock")?;
    assert_eq!((clock.initial, clock.min, clock.max), (0, 0, i64::MAX));

    let mut stored = std::collections::BTreeMap::new();
    for day in 0..2 {
        let now = 1_800_000_000 + day * 86_400;
        let restart = catalogue
            .transition(if day == 0 {
                "oteryn:quest-reward/oramond/native-roots-start"
            } else {
                "oteryn:quest-reward/oramond/native-roots-restart"
            })
            .ok_or("daily restart")?;
        if day == 1 {
            assert!(
                catalogue.evaluate(restart, &stored, now).is_err(),
                "unreported roots cannot be erased"
            );
            let report = catalogue
                .transition("oteryn:quest-reward/oramond/native-roots-report")
                .ok_or("root hand-in")?;
            for change in catalogue
                .evaluate(report, &stored, now)
                .map_err(|e| format!("{e:?}"))?
            {
                stored.insert(change.track, change.after);
            }
        }
        let changes = catalogue
            .evaluate(restart, &stored, now)
            .map_err(|e| format!("{e:?}"))?;
        for change in changes {
            stored.insert(change.track, change.after);
        }
        assert!(catalogue.evaluate(restart, &stored, now + 86_399).is_err());
        for callback in &callbacks {
            assert!(callback.selector.native.is_some());
            assert_eq!(
                callback
                    .effects
                    .item_reward
                    .as_ref()
                    .ok_or("root reward")?
                    .item,
                "oteryn:item.tibia.i21291"
            );
            let epoch = callback.effects.item_reward_cycle.as_ref().ok_or("epoch")?;
            assert_eq!(stored.get(epoch), Some(&(day + 1)));
            let transition = catalogue.transition(&callback.key).ok_or("harvest")?;
            let changes = catalogue
                .evaluate(transition, &stored, now)
                .map_err(|e| format!("{e:?}"))?;
            for change in changes {
                stored.insert(change.track, change.after);
            }
            assert!(catalogue.evaluate(transition, &stored, now).is_err());
        }
    }
    Ok(())
}

#[test]
fn native_lion_well_per_use_reward_keeps_its_physical_admission() -> Result<(), String> {
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/world-callbacks/m07-lion-offerings.json"
    ))
    .map_err(|error| error.to_string())?;
    let well = document["callbacks"]
        .as_array()
        .ok_or("callbacks")?
        .iter()
        .find(|row| row["key"] == "oteryn:quest-callback/m07/lion-well")
        .ok_or("native well")?;
    let encode =
        |row: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[row]}).to_string();
    parse(&encode(well))?;
    for field in ["requires_world_admission", "effects"] {
        let mut changed = well.clone();
        if field == "effects" {
            changed[field]["item_reward_per_use"] = false.into();
        } else {
            changed[field] = false.into();
        }
        assert!(parse(&encode(&changed)).is_err(), "missing {field}");
    }
    Ok(())
}

#[test]
fn native_per_use_rewards_require_exact_physical_admission() -> Result<(), String> {
    let callbacks = load()?;
    let source = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-callback/m07/lion-strength")
        .ok_or("native source")?;
    let reward = callbacks
        .iter()
        .find_map(|c| c.effects.item_reward.clone())
        .ok_or("existing reward")?;
    let mut value = serde_json::to_value(source).map_err(|e| e.to_string())?;
    value["item_on_target"] = serde_json::Value::Null;
    value["passages"] = serde_json::json!([]);
    value["world_requirements"] = serde_json::json!([]);
    value["state_ops"] = serde_json::json!([]);
    value["storage"] = serde_json::json!([{
        "track":"oteryn:quest-progress/m07/native-producer-start","minimum":-1,
        "maximum":100,"set":1,"at_least":true
    }]);
    value["effects"] = serde_json::json!({"item_reward":reward,"item_reward_per_use":true});
    let document =
        |v: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[v]}).to_string();
    let parsed = parse(&document(&value))?;
    let serialized = serde_json::to_value(&parsed[0].effects).map_err(|e| e.to_string())?;
    assert_eq!(
        serialized["item_reward_per_use"], true,
        "explicit repeat mode survives lowering"
    );
    for mutation in 0..5 {
        let mut changed = value.clone();
        match mutation {
            0 => changed["requires_world_admission"] = false.into(),
            1 => changed["selector"]["native"] = serde_json::Value::Null,
            2 => changed["reversible"] = true.into(),
            3 => changed["effects"]["item_reward"] = serde_json::Value::Null,
            _ => changed["effects"]["item_reward_cycle"] = "oteryn:quest-progress/m07/epoch".into(),
        }
        assert!(
            parse(&document(&changed)).is_err(),
            "unsafe per-use mode {mutation}"
        );
    }
    Ok(())
}

#[test]
fn native_per_use_producer_accepts_a_current_world_guard_without_widening_legacy_rewards()
-> Result<(), String> {
    let mut callback = load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/m07/lion-flower-west")
        .ok_or("native flower producer")?;
    let document = |c: &Callback| serde_json::json!({"schema":1,"callbacks":[c]}).to_string();
    assert!(!callback.world_requirements.is_empty());
    parse(&document(&callback))?;
    callback.effects.item_reward_per_use = false;
    let flag = callback.storage.first_mut().ok_or("source start flag")?;
    flag.maximum = 0;
    flag.at_least = false;
    assert!(
        parse(&document(&callback)).is_err(),
        "legacy reward cannot borrow a physical guard"
    );
    callback.world_requirements.clear();
    parse(&document(&callback))?;
    Ok(())
}
