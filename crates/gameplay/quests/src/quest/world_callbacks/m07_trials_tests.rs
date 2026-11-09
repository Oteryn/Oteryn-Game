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
fn m07_native_trial_level_guard_composes_with_exact_world_requirements() -> Result<(), String> {
    let mut callback = load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/m07/realm-trial")
        .ok_or("native trial")?;
    callback.minimum_level = Some(50);
    parse(&encode(&callback))?;
    for invalid in 0..4 {
        let mut bad = callback.clone();
        match invalid {
            0 => bad.selector.native = None,
            1 => bad.requires_world_admission = false,
            2 => bad.reversible = true,
            _ => bad.trigger = Trigger::StepIn,
        }
        assert!(
            parse(&encode(&bad)).is_err(),
            "unbound level/world composition {invalid}"
        );
    }
    Ok(())
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

#[test]
fn m07_remote_passage_reset_has_explicit_targets_and_bounded_deadline() -> Result<(), String> {
    let callback = trial()?;
    let mut value = serde_json::to_value(callback).map_err(|error| error.to_string())?;
    value["passages"] = serde_json::json!([{
        "position":{"x":33134,"y":32289,"floor":-8},
        "appearance":2074,
        "replacement":2075,
        "create_on_close":null
    }]);
    value["effects"]["passages_reset_after_ms"] = 15000.into();
    parse(&serde_json::json!({"schema":1,"callbacks":[value.clone()]}).to_string())?;
    for invalid in 0..3 {
        let mut bad = value.clone();
        match invalid {
            0 => bad["passages"] = serde_json::json!([]),
            1 => bad["effects"]["passages_reset_after_ms"] = 0.into(),
            _ => bad["effects"]["passages_reset_after_ms"] = 3600001.into(),
        }
        assert!(parse(&serde_json::json!({"schema":1,"callbacks":[bad]}).to_string()).is_err());
    }
    Ok(())
}

#[test]
fn m07_dreamer_levers_bind_four_native_controls_and_all_source_wall_objects() -> Result<(), String>
{
    let callbacks: Vec<_> = load()?
        .into_iter()
        .filter(|c| {
            c.key
                .starts_with("oteryn:quest-callback/m07/dreamer-lever-")
        })
        .collect();
    assert_eq!(
        callbacks.len(),
        4,
        "all four physical Dreamer controls must be wired"
    );
    let mut wall_count = 0;
    for callback in &callbacks {
        assert_eq!(callback.quest, "oteryn:quest.dreamer_s_challenge_quest");
        assert_eq!(callback.lever, Some([2772, 2773]));
        assert!(!callback.reversible);
        assert_eq!(callback.effects.reset_after_ms, Some(8000));
        assert_eq!(callback.effects.passages_reset_after_ms, Some(7000));
        let native = callback
            .selector
            .native
            .as_ref()
            .ok_or("physical lever counterpart")?;
        assert_eq!(native.ordinal, 1);
        assert_eq!(native.appearance, 2772);
        assert_eq!(native.position.floor, -14);
        assert_eq!(
            callback.selector.unique, None,
            "the captured map has no donor UID tags"
        );
        wall_count += callback.passages.len();
        let mut duplicate = (*callback).clone();
        duplicate
            .passages
            .push(duplicate.passages.first().ok_or("walls")?.clone());
        assert!(
            parse(&encode(&duplicate)).is_err(),
            "reject duplicate exact wall identity"
        );
    }
    assert_eq!(wall_count, 17);
    let north = callbacks
        .iter()
        .find(|c| c.key.ends_with("-north"))
        .ok_or("north control")?;
    let corner: std::collections::BTreeSet<_> = north
        .passages
        .iter()
        .filter(|p| p.position.x == 32760 && p.position.y == 32288)
        .map(|p| p.appearance)
        .collect();
    assert_eq!(
        corner,
        [1027, 1272].into_iter().collect(),
        "retain both source creates at the same coordinate"
    );
    Ok(())
}

#[test]
fn m07_native_offering_can_bind_an_exact_input_and_physical_world_predicate() -> Result<(), String>
{
    let mut callback = trial()?;
    callback.item_on_target = Some(
        serde_json::from_value(serde_json::json!({
            "item":"oteryn:item.tibia.i9634","count":1,
            "replacement":null,"target_replacement":null
        }))
        .map_err(|e| e.to_string())?,
    );
    parse(&encode(&callback))?;
    let binding = gated_binding(&callback)?;
    callback.item_on_target.as_mut().ok_or("offering")?.count = 2;
    assert_ne!(
        gated_binding(&callback)?,
        binding,
        "the exact charge is part of admission"
    );
    callback.requires_world_admission = false;
    assert!(
        parse(&encode(&callback)).is_err(),
        "paid physical predicates must remain natively bound"
    );
    Ok(())
}

#[test]
fn m07_lion_strength_owns_exact_input_and_reuses_original_tales_counter() -> Result<(), String> {
    let callback = load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/m07/lion-strength")
        .ok_or("Lion Strength native binding missing")?;
    assert_eq!(callback.quest, "oteryn:quest.tibia_tales");
    assert!(callback.requires_world_admission);
    let input = callback.item_on_target.as_ref().ok_or("cobra tongue")?;
    assert_eq!(
        (input.item.as_str(), input.count, input.reusable),
        ("oteryn:item.tibia.i9634", 1, false)
    );
    assert_eq!(callback.effects.passages_reset_after_ms, Some(15000));
    assert_eq!(callback.passages.len(), 1);
    assert_eq!(
        (
            callback.passages[0].appearance,
            callback.passages[0].replacement
        ),
        (2074, Some(2075))
    );
    assert_eq!(callback.state_ops.len(), 1);
    assert_eq!(
        callback.state_ops[0].track,
        "oteryn:quest-progress/quest/u10_70/lions_rock/questline"
    );
    let catalogue = crate::quest::loader::load_embedded_quest_state("m07-native-lion")
        .map_err(|e| format!("{e:?}"))?;
    let flag = callback.storage.first().ok_or("source flag")?;
    let track = catalogue
        .catalogue()
        .track(&flag.track)
        .ok_or("flag track")?;
    assert_eq!(track.quest, "oteryn:quest.tibia_tales");
    assert_eq!(track.initial, -1);
    Ok(())
}

#[test]
fn m07_clear_area_extra_floors_are_explicit_bounded_and_bound_to_admission() -> Result<(), String> {
    let c = trial()?;
    let mut v = serde_json::to_value(&c).map_err(|e| e.to_string())?;
    v["effects"]["clear_area"] = serde_json::json!({
        "minimum":{"x":33074,"y":32580,"floor":-3},
        "maximum":{"x":33096,"y":32601,"floor":-3},
        "additional_floors":[-6,-5,-4],
        "creatures":["oteryn:creature.dragon","oteryn:creature.dragon_lord","oteryn:creature.fire_elemental"]
    });
    let document =
        |v: &serde_json::Value| serde_json::json!({"schema":1,"callbacks":[v]}).to_string();
    let parsed = parse(&document(&v))?;
    let area = parsed[0]
        .effects
        .clear_area
        .as_ref()
        .ok_or("stacked guardian area")?;
    for floor in [-6, -5, -4, -3] {
        assert!(area.contains(Position {
            x: 33080,
            y: 32590,
            floor
        }));
    }
    assert!(!area.contains(Position {
        x: 33080,
        y: 32590,
        floor: -2
    }));
    assert!(!area.contains(Position {
        x: 33097,
        y: 32590,
        floor: -4
    }));
    let mut single = v.clone();
    single["effects"]["clear_area"]
        .as_object_mut()
        .ok_or("area object")?
        .remove("additional_floors");
    let legacy = parse(&document(&single))?;
    assert!(
        !legacy[0]
            .effects
            .clear_area
            .as_ref()
            .ok_or("single-floor area")?
            .contains(Position {
                x: 33080,
                y: 32590,
                floor: -4
            })
    );
    assert_ne!(gated_binding(&parsed[0])?, gated_binding(&legacy[0])?);
    for floors in [
        serde_json::json!([-3]),
        serde_json::json!([-4, -4]),
        serde_json::json!([-16]),
        serde_json::json!([1]),
        serde_json::json!([-7, -6, -5, -4]),
    ] {
        let mut bad = v.clone();
        bad["effects"]["clear_area"]["additional_floors"] = floors;
        assert!(parse(&document(&bad)).is_err(), "invalid extra-floor scope");
    }
    Ok(())
}
