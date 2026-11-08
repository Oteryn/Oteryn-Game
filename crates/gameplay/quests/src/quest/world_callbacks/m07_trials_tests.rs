//! M07: a physical world predicate and a personal stage write share native admission.
use super::*;

fn trial() -> Result<Callback, String> {
    let mut c = load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/l6/waterskin-of-mead")
        .ok_or("native sample")?;
    c.key = "oteryn:quest-callback/m07/test-native-trial".into();
    c.effects = Default::default();
    c.requires_world_admission = true;
    c.world_requirements = vec![WorldRequirement {
        position: c.selector.position.ok_or("sample position")?,
        appearance: 2523,
        ground_stack: None,
    }];
    Ok(c)
}
fn encode(c: &Callback) -> String {
    serde_json::json!({"schema":1,"callbacks":[c]}).to_string()
}

#[test]
fn m07_native_world_trial_can_guard_and_commit_a_personal_stage() -> Result<(), String> {
    let c = trial()?;
    parse(&encode(&c))?;
    assert!(gated_binding(&c)?.is_some());
    let mut ungated = c;
    ungated.requires_world_admission = false;
    assert!(
        parse(&encode(&ungated)).is_err(),
        "world predicates cannot authorize an unbound personal write"
    );
    Ok(())
}

#[test]
fn m07_native_world_trial_can_complete_only_with_its_physical_binding() -> Result<(), String> {
    let c = trial()?;
    let mut value = serde_json::to_value(&c).map_err(|e| e.to_string())?;
    value["effects"]["completes_quest"] = true.into();
    let parsed = parse(&serde_json::json!({"schema":1,"callbacks":[value.clone()]}).to_string())?;
    assert!(primary_transition(&parsed[0])?.completes);
    value["requires_world_admission"] = false.into();
    assert!(parse(&serde_json::json!({"schema":1,"callbacks":[value]}).to_string()).is_err());
    Ok(())
}

#[test]
fn m07_trial_binding_covers_world_requirement_and_completion() -> Result<(), String> {
    let mut c = trial()?;
    let bound = gated_binding(&c)?;
    c.world_requirements[0].appearance += 1;
    assert_ne!(bound, gated_binding(&c)?);
    let bound = gated_binding(&c)?;
    c.effects.completes_quest = true;
    assert_ne!(bound, gated_binding(&c)?);
    c.reversible = true;
    assert!(parse(&encode(&c)).is_err());
    c.reversible = false;
    for state in &mut c.storage {
        state.preserve = true;
    }
    assert!(parse(&encode(&c)).is_err());
    Ok(())
}

#[test]
fn m07_clear_area_requires_native_personal_stage_and_bounded_same_floor() -> Result<(), String> {
    let mut c = trial()?;
    let mut v = serde_json::to_value(&c).map_err(|e| e.to_string())?;
    v["effects"]["clear_area"] = serde_json::json!({
        "minimum":{"x":100,"y":100,"floor":-3},
        "maximum":{"x":110,"y":110,"floor":-3},
        "creatures":["oteryn:creature.dragon"]
    });
    let parsed = parse(&serde_json::json!({"schema":1,"callbacks":[v.clone()]}).to_string())?;
    assert_ne!(gated_binding(&c)?, gated_binding(&parsed[0])?);
    for mutation in 0..7 {
        let mut bad = v.clone();
        match mutation {
            0 => bad["requires_world_admission"] = false.into(),
            1 => bad["effects"]["clear_area"]["maximum"]["floor"] = (-4).into(),
            2 => bad["effects"]["clear_area"]["maximum"]["x"] = 65535.into(),
            3 => bad["effects"]["clear_area"]["minimum"]["x"] = 111.into(),
            4 => bad["effects"]["clear_area"]["creatures"] = serde_json::json!([]),
            5 => {
                bad["effects"]["clear_area"]["creatures"] =
                    serde_json::json!(["oteryn:creature.dragon", "oteryn:creature.dragon"])
            }
            _ => {
                bad["effects"]["clear_area"]["creatures"] = serde_json::json!(["oteryn:creature."])
            }
        }
        assert!(parse(&serde_json::json!({"schema":1,"callbacks":[bad]}).to_string()).is_err());
    }
    c.effects = parsed[0].effects.clone();
    let mut unstarted = c.clone();
    unstarted.storage.retain(|state| !state.preserve);
    assert!(
        parse(&encode(&unstarted)).is_err(),
        "an initially empty room is not a started trial"
    );
    let started = unstarted.storage.first_mut().ok_or("trial stage")?;
    started.minimum = 1;
    started.maximum = 1;
    started.set = 2;
    parse(&encode(&unstarted))?;
    for state in &mut c.storage {
        state.preserve = true;
    }
    assert!(parse(&encode(&c)).is_err());
    Ok(())
}
