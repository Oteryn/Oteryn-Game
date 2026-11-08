//! World ingress must carry the same native time operations as NPC ingress.
use super::*;

#[test]
fn world_callback_clock_operations_are_lowered_and_bounded() -> Result<(), String> {
    let mut value = serde_json::to_value(
        load()?
            .into_iter()
            .find(|c| !c.storage.is_empty() && !c.reversible && c.native_rule.is_none())
            .ok_or("world fixture")?,
    )
    .map_err(|e| e.to_string())?;
    value["state_ops"] = serde_json::json!([{
        "track":"oteryn:quest-progress/clock-test/last-use",
        "from":{"kind":"elapsed_at_least","seconds":3600},
        "effect":{"kind":"set_now"}
    }]);
    let callbacks = parse(&serde_json::json!({"schema":1,"callbacks":[value]}).to_string())?;
    let transition = primary_transition(&callbacks[0])?;
    assert!(transition.effects.iter().any(|effect| effect.track
        == "oteryn:quest-progress/clock-test/last-use"
        && effect.from == QuestComparison::ElapsedAtLeast(3600)
        && effect.effect == QuestEffectKind::SetNow));
    let callback = &callbacks[0];
    let empty =
        QuestStateCatalogue::new("clock-test", vec![], vec![]).map_err(|e| format!("{e:?}"))?;
    let catalogue = extend_callbacks(empty, &callbacks)?;
    let now = 1_700_000_000;
    let stored = std::collections::BTreeMap::from_iter(
        callback
            .storage
            .iter()
            .map(|s| (s.track.clone(), s.minimum))
            .chain([(
                "oteryn:quest-progress/clock-test/last-use".into(),
                now - 3599,
            )]),
    );
    assert_eq!(
        catalogue.evaluate(&transition, &stored, now),
        Err(crate::quest::QuestRefusal::StageMismatch)
    );
    let changes = catalogue
        .evaluate(&transition, &stored, now + 1)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(changes.last().ok_or("clock change")?.after, now + 1);
    let mut bad = serde_json::to_value(callback).map_err(|e| e.to_string())?;
    bad["state_ops"][0]["track"] = serde_json::Value::String(callback.storage[0].track.clone());
    assert!(parse(&serde_json::json!({"schema":1,"callbacks":[bad]}).to_string()).is_err());
    let mut clock_only = callback.clone();
    clock_only.storage.clear();
    clock_only.requires_world_admission = false;
    clock_only.minimum_level = None;
    clock_only.offerings.clear();
    clock_only.item_on_target = None;
    clock_only.monk_reward = None;
    clock_only.effects = Default::default();
    parse(&serde_json::json!({"schema":1,"callbacks":[clock_only]}).to_string())?;
    assert!(clock_only.has_personal_state());
    assert_eq!(primary_transition(&clock_only)?.effects.len(), 1);
    Ok(())
}
