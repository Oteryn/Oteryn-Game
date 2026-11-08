use super::*;

#[test]
fn cycle_reward_requires_a_preserved_nonnegative_local_epoch() -> Result<(), String> {
    let mut callback = load()?
        .into_iter()
        .find(|c| c.effects.item_reward.is_some())
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
