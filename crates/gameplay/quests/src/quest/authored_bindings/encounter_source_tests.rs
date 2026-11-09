//! Minimal source-roster fixture; no production quest content or admission authority.
use super::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn fixture() -> Result<Value, String> {
    let mut value: Value = serde_json::from_slice(include_bytes!("fixtures/kill.json"))
        .map_err(|error| error.to_string())?;
    let quest = &mut value["quests"][0];
    quest["stages"][1]["completes"] = json!(false);
    let counter = &mut quest["stages"][1]["trigger"]["counter"];
    counter["maximum"] = json!(2);
    counter
        .as_object_mut()
        .ok_or("counter")?
        .remove("advance_gate_to");
    counter["encounter_source"] = json!("oteryn:quest-callback/engenc-source-fixture");
    let mut callback = super::super::world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-callback/behemoth")
        .ok_or("template")?;
    callback.key = "oteryn:quest-callback/engenc-source-fixture".into();
    callback.quest = quest["quest"].as_str().ok_or("quest")?.into();
    callback.passages.clear();
    callback.selector.position = Some(super::super::world_callbacks::Position {
        x: 100,
        y: 99,
        floor: -7,
    });
    callback.selector.unique = None;
    callback.selector.action = None;
    callback.selector.native = None;
    callback.storage.clear();
    callback.effects.encounter = Some(serde_json::from_value(json!({
        "minimum_level":1,
        "players":[{"from":{"x":99,"y":99,"floor":-7},"to":{"x":100,"y":100,"floor":-7}}],
        "monsters":[{"position":{"x":101,"y":100,"floor":-7},"creature":"oteryn:creature.rat"},{"position":{"x":102,"y":100,"floor":-7},"creature":"oteryn:creature.rat"}],
        "room_min":{"x":100,"y":100,"floor":-7},"room_max":{"x":102,"y":100,"floor":-7},"timing":null
    })).map_err(|error| error.to_string())?);
    quest["stages"].as_array_mut().ok_or("stages")?.push(json!({"key":"source-entry","completes":false,"trigger":{"kind":"map_use","callback":callback}}));
    Ok(value)
}
fn parse(value: &Value) -> Result<Bindings, String> {
    Bindings::parse(&serde_json::to_vec(value).map_err(|error| error.to_string())?)
}
#[test]
fn engenc_counter_source_is_owned_species_bounded_and_preserves_phase_revision()
-> Result<(), String> {
    let value = fixture()?;
    let bindings = parse(&value)?;
    let base = QuestStateCatalogue::empty("engenc-source").ok_or("catalogue")?;
    let catalogue = bindings.attach(&base)?;
    let key = value["quests"][0]["stages"][1]["trigger"]["counter"]["key"]
        .as_str()
        .ok_or("key")?;
    let counter = super::super::native_kills::counter_for(&catalogue, key).ok_or("counter")?;
    assert_eq!(
        counter.encounter_source,
        Some("oteryn:quest-callback/engenc-source-fixture")
    );
    let transition = catalogue.transition(key).ok_or("transition")?;
    let mut state = BTreeMap::from([(counter.gate.into(), 4), (counter.track.into(), 0)]);
    for expected in 1..=2 {
        let changes = catalogue
            .evaluate(transition, &state, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(changes.len(), 1, "death must not rewrite the mission epoch");
        assert_eq!(changes[0].track, counter.track);
        assert_eq!(changes[0].after, expected);
        state.insert(counter.track.into(), expected);
    }
    assert!(catalogue.evaluate(transition, &state, 0).is_err());
    let mut rebound = value.clone();
    rebound["quests"][0]["stages"][1]["trigger"]["counter"]["encounter_source"] =
        json!("oteryn:quest-callback/engenc-other-source");
    rebound["quests"][0]["stages"][2]["trigger"]["callback"]["key"] =
        json!("oteryn:quest-callback/engenc-other-source");
    let rebound = parse(&rebound)?.attach(&base)?;
    assert_ne!(
        catalogue.definition_hash(counter.quest),
        rebound.definition_hash(counter.quest)
    );
    let mut early_completion = value.clone();
    early_completion["quests"][0]["stages"][1]["completes"] = json!(true);
    assert!(parse(&early_completion).is_err());
    for (field, invalid) in [
        ("encounter_source", json!("oteryn:quest-callback/foreign")),
        ("maximum", json!(3)),
        ("maximum", json!(1)),
        ("creature", json!("oteryn:creature.dragon")),
        ("advance_gate_to", json!(99)),
    ] {
        let mut bad = value.clone();
        bad["quests"][0]["stages"][1]["trigger"]["counter"][field] = invalid;
        assert!(parse(&bad).is_err(), "{field}");
    }
    Ok(())
}

#[test]
fn engenc_source_threshold_changes_phase_only_at_the_goal() -> Result<(), String> {
    let mut value = fixture()?;
    value["quests"][0]["stages"][1]["trigger"]["counter"]["advance_gate_to"] = json!(5);
    let bindings = parse(&value)?;
    let base = QuestStateCatalogue::empty("engenc-threshold").ok_or("base")?;
    let catalogue = bindings.attach(&base)?;
    let key = value["quests"][0]["stages"][1]["trigger"]["counter"]["key"]
        .as_str()
        .ok_or("key")?;
    let transition = catalogue.transition(key).ok_or("transition")?;
    let counter = super::super::native_kills::counter_for(&catalogue, key).ok_or("counter")?;
    let mut state = BTreeMap::from([(counter.gate.into(), 4), (counter.track.into(), 0)]);
    let first = catalogue
        .evaluate(transition, &state, 0)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        first.len(),
        2,
        "retain one evaluation result per declared effect"
    );
    let phase = first
        .iter()
        .find(|change| change.track == counter.gate)
        .ok_or("phase")?;
    assert_eq!(
        phase.before, phase.after,
        "a nonterminal phase is a read-only gate"
    );
    for change in first {
        state.insert(change.track, change.after);
    }
    let final_changes = catalogue
        .evaluate(transition, &state, 0)
        .map_err(|error| format!("{error:?}"))?;
    for change in final_changes {
        state.insert(change.track, change.after);
    }
    assert_eq!(state[counter.track], 2);
    assert_eq!(state[counter.gate], 5);
    assert!(catalogue.evaluate(transition, &state, 0).is_err());
    Ok(())
}
//! Minimal source-roster fixture; no production quest content or admission authority.
use super::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn fixture() -> Result<Value, String> {
    let mut value: Value = serde_json::from_slice(include_bytes!("fixtures/kill.json"))
        .map_err(|error| error.to_string())?;
    let quest = &mut value["quests"][0];
    quest["stages"][1]["completes"] = json!(false);
    let counter = &mut quest["stages"][1]["trigger"]["counter"];
    counter["maximum"] = json!(2);
    counter
        .as_object_mut()
        .ok_or("counter")?
        .remove("advance_gate_to");
    counter["encounter_source"] = json!("oteryn:quest-callback/engenc-source-fixture");
    let mut callback = super::super::world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == "oteryn:quest-callback/behemoth")
        .ok_or("template")?;
    callback.key = "oteryn:quest-callback/engenc-source-fixture".into();
    callback.quest = quest["quest"].as_str().ok_or("quest")?.into();
    callback.passages.clear();
    callback.selector.position = Some(super::super::world_callbacks::Position {
        x: 100,
        y: 99,
        floor: -7,
    });
    callback.selector.unique = None;
    callback.selector.action = None;
    callback.selector.native = None;
    callback.storage.clear();
    callback.effects.encounter = Some(serde_json::from_value(json!({
        "minimum_level":1,
        "players":[{"from":{"x":99,"y":99,"floor":-7},"to":{"x":100,"y":100,"floor":-7}}],
        "monsters":[{"position":{"x":101,"y":100,"floor":-7},"creature":"oteryn:creature.rat"},{"position":{"x":102,"y":100,"floor":-7},"creature":"oteryn:creature.rat"}],
        "room_min":{"x":100,"y":100,"floor":-7},"room_max":{"x":102,"y":100,"floor":-7},"timing":null
    })).map_err(|error| error.to_string())?);
    quest["stages"].as_array_mut().ok_or("stages")?.push(json!({"key":"source-entry","completes":false,"trigger":{"kind":"map_use","callback":callback}}));
    Ok(value)
}
fn parse(value: &Value) -> Result<Bindings, String> {
    Bindings::parse(&serde_json::to_vec(value).map_err(|error| error.to_string())?)
}
#[test]
fn engenc_counter_source_is_owned_species_bounded_and_preserves_phase_revision()
-> Result<(), String> {
    let value = fixture()?;
    let bindings = parse(&value)?;
    let base = QuestStateCatalogue::empty("engenc-source").ok_or("catalogue")?;
    let catalogue = bindings.attach(&base)?;
    let key = value["quests"][0]["stages"][1]["trigger"]["counter"]["key"]
        .as_str()
        .ok_or("key")?;
    let counter = super::super::native_kills::counter_for(&catalogue, key).ok_or("counter")?;
    assert_eq!(
        counter.encounter_source,
        Some("oteryn:quest-callback/engenc-source-fixture")
    );
    let transition = catalogue.transition(key).ok_or("transition")?;
    let mut state = BTreeMap::from([(counter.gate.into(), 4), (counter.track.into(), 0)]);
    for expected in 1..=2 {
        let changes = catalogue
            .evaluate(transition, &state, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(changes.len(), 1, "death must not rewrite the mission epoch");
        assert_eq!(changes[0].track, counter.track);
        assert_eq!(changes[0].after, expected);
        state.insert(counter.track.into(), expected);
    }
    assert!(catalogue.evaluate(transition, &state, 0).is_err());
    let mut rebound = value.clone();
    rebound["quests"][0]["stages"][1]["trigger"]["counter"]["encounter_source"] =
        json!("oteryn:quest-callback/engenc-other-source");
    rebound["quests"][0]["stages"][2]["trigger"]["callback"]["key"] =
        json!("oteryn:quest-callback/engenc-other-source");
    let rebound = parse(&rebound)?.attach(&base)?;
    assert_ne!(
        catalogue.definition_hash(counter.quest),
        rebound.definition_hash(counter.quest)
    );
    let mut early_completion = value.clone();
    early_completion["quests"][0]["stages"][1]["completes"] = json!(true);
    assert!(parse(&early_completion).is_err());
    for (field, invalid) in [
        ("encounter_source", json!("oteryn:quest-callback/foreign")),
        ("maximum", json!(3)),
        ("maximum", json!(1)),
        ("creature", json!("oteryn:creature.dragon")),
        ("advance_gate_to", json!(5)),
    ] {
        let mut bad = value.clone();
        bad["quests"][0]["stages"][1]["trigger"]["counter"][field] = invalid;
        assert!(parse(&bad).is_err(), "{field}");
    }
    Ok(())
}
