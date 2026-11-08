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
