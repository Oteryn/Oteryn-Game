//! Personal quest occurrences follow the source lifecycle, rather than each save.
use oteryn_gameplay_world_change::counter::{
    CounterError, CounterKind as K, CounterState, Credit, Parameters,
};

fn generation(state: CounterState) -> Result<u64, String> {
    serde_json::to_value(state).map_err(|e| e.to_string())?["occurrence_generation"]
        .as_u64()
        .ok_or_else(|| "missing quest occurrence generation".into())
}
fn saved(state: CounterState, kind: K, epoch: u64, p: &Parameters) -> Result<CounterState, String> {
    state
        .at_save(kind, epoch, false, p, 0)
        .map_err(|e| format!("{e:?}"))
}

#[test]
fn qm55_hive_occurrence_survives_saves_and_forward_stages_then_changes_on_reset()
-> Result<(), String> {
    let p = Parameters::default();
    let mut hive = CounterState::new(K::HiveBorn, 10, false);
    assert_eq!(generation(hive)?, 1);
    for _ in 0..200 {
        hive = hive
            .credit(K::HiveBorn, Credit::Task(0), &p, 0)
            .map_err(|e| format!("{e:?}"))?;
    }
    hive = saved(hive, K::HiveBorn, 11, &p)?;
    assert_eq!(hive.stage, 2);
    assert_eq!(generation(hive)?, 1);
    for _ in 0..200 {
        hive = hive
            .credit(K::HiveBorn, Credit::Task(4), &p, 0)
            .map_err(|e| format!("{e:?}"))?;
    }
    hive = saved(hive, K::HiveBorn, 12, &p)?;
    assert_eq!(hive.stage, 3);
    hive = saved(hive, K::HiveBorn, 13, &p)?;
    assert_eq!(generation(hive)?, 1, "a save is not a new Hive occurrence");
    hive = saved(hive, K::HiveBorn, 17, &p)?;
    assert_eq!(hive.stage, 1);
    assert_eq!(generation(hive)?, 2);
    assert_eq!(
        hive.credit_generation, 1,
        "death-credit generations retain their own semantics"
    );
    assert_eq!(saved(hive, K::HiveBorn, 17, &p)?, hive);
    Ok(())
}

#[test]
fn qm55_deepling_occurrence_changes_only_at_five_day_reset_and_legacy_defaults_are_safe()
-> Result<(), String> {
    let p = Parameters {
        deepling_points: 100,
        ..Parameters::default()
    };
    let mut json = serde_json::to_value(CounterState::new(K::Deeplings, 10, false))
        .map_err(|e| e.to_string())?;
    json["stage"] = 3.into();
    let state: CounterState = serde_json::from_value(json.clone()).map_err(|e| e.to_string())?;
    let same = saved(state, K::Deeplings, 14, &p)?;
    assert_eq!(generation(same)?, 1);
    let reset = saved(same, K::Deeplings, 15, &p)?;
    assert_eq!(reset.stage, 1);
    assert_eq!(generation(reset)?, 2);
    let mut legacy = json.clone();
    legacy
        .as_object_mut()
        .ok_or("counter object")?
        .remove("occurrence_generation");
    assert_eq!(
        generation(serde_json::from_value(legacy).map_err(|e| e.to_string())?)?,
        1
    );
    json["occurrence_generation"] = 0.into();
    let invalid: CounterState = serde_json::from_value(json.clone()).map_err(|e| e.to_string())?;
    assert_eq!(
        invalid.validate(K::Deeplings),
        Err(CounterError::InvalidState)
    );
    json["occurrence_generation"] = u64::MAX.into();
    let exhausted: CounterState = serde_json::from_value(json).map_err(|e| e.to_string())?;
    assert_eq!(
        exhausted.at_save(K::Deeplings, 15, false, &p, 0),
        Err(CounterError::Overflow)
    );
    Ok(())
}
