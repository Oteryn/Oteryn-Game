use super::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

fn fixture() -> Result<Value, String> {
    let mut value: Value = serde_json::from_slice(include_bytes!("fixtures/kill.json"))
        .map_err(|error| error.to_string())?;
    value["quests"][0]["stages"][1]["completes"] = json!(false);
    value["quests"][0]["stages"][1]["trigger"] = json!({
        "kind":"hit", "objective":{
            "key":"oteryn:quest-transition/native-hits/training",
            "creature":"oteryn:creature.rat",
            "source":{"kind":"weapon","item":"oteryn:item.spear"},
            "track":"oteryn:quest-progress/authored-kills/count",
            "gate":"oteryn:quest-progress/authored-kills/phase",
            "gate_value":4,"advance_gate_to":5
        }
    });
    Ok(value)
}
fn parse(value: &Value) -> Result<Bindings, String> {
    Bindings::parse(&serde_json::to_vec(value).map_err(|error| error.to_string())?)
}

#[test]
fn engenc_authored_hit_is_sequential_one_shot_and_hash_bound() -> Result<(), String> {
    let value = fixture()?;
    let bindings = parse(&value)?;
    let base = QuestStateCatalogue::empty("engenc-hit").ok_or("base")?;
    let catalogue = bindings.attach(&base)?;
    let transition = catalogue
        .transition("oteryn:quest-transition/native-hits/training")
        .ok_or("hit transition")?;
    let phase = "oteryn:quest-progress/authored-kills/phase";
    let count = "oteryn:quest-progress/authored-kills/count";
    assert!(
        catalogue
            .evaluate(
                transition,
                &BTreeMap::from([(phase.into(), 3), (count.into(), 0)]),
                0
            )
            .is_err()
    );
    let mut state = BTreeMap::from([(phase.into(), 4), (count.into(), 0)]);
    for change in catalogue
        .evaluate(transition, &state, 0)
        .map_err(|error| format!("{error:?}"))?
    {
        state.insert(change.track, change.after);
    }
    assert_eq!(state[phase], 5);
    assert_eq!(state[count], 1);
    assert!(catalogue.evaluate(transition, &state, 0).is_err());
    let mut changed = value.clone();
    changed["quests"][0]["stages"][1]["trigger"]["objective"]["source"] = json!({
        "kind":"rune","item":"oteryn:item.training_rune","spell":"candidate:spell/rune/training"
    });
    let changed = parse(&changed)?.attach(&base)?;
    assert_ne!(
        catalogue.definition_hash(&bindings.quests[0].quest),
        changed.definition_hash(&bindings.quests[0].quest)
    );
    for (field, invalid) in [
        ("advance_gate_to", json!(4)),
        ("gate", json!(count)),
        ("creature", json!("oteryn:item.spear")),
        ("key", json!("oteryn:quest-transition/native-kills/forged")),
    ] {
        let mut bad = value.clone();
        bad["quests"][0]["stages"][1]["trigger"]["objective"][field] = invalid;
        assert!(parse(&bad).is_err(), "{field}");
    }
    for source in [
        json!({"kind":"weapon","item":"oteryn:creature.rat"}),
        json!({"kind":"rune","item":"oteryn:item.training_rune"}),
        json!({"kind":"rune","item":"oteryn:item.training_rune","spell":"anything"}),
    ] {
        let mut bad = value.clone();
        bad["quests"][0]["stages"][1]["trigger"]["objective"]["source"] = source;
        assert!(parse(&bad).is_err());
    }
    Ok(())
}

#[test]
fn engenc_hit_selectors_require_exact_weapon_rune_and_live_phase() -> Result<(), String> {
    let mut value = fixture()?;
    let base = QuestStateCatalogue::empty("engenc-hit-selector").ok_or("base")?;
    let catalogue = parse(&value)?.attach(&base)?;
    let (quest, objective) =
        hits::objective_for(&catalogue, "oteryn:quest-transition/native-hits/training")
            .ok_or("objective")?;
    assert_eq!(
        quest,
        catalogue
            .transition(&objective.key)
            .ok_or("transition")?
            .quest
    );
    assert!(objective.matches("oteryn:creature.rat", "oteryn:item.spear", None));
    assert!(!objective.matches("oteryn:creature.dragon", "oteryn:item.spear", None));
    assert!(!objective.matches("oteryn:creature.rat", "oteryn:item.sword", None));
    assert!(!objective.matches(
        "oteryn:creature.rat",
        "oteryn:item.spear",
        Some("candidate:spell/rune/training")
    ));
    assert!(!objective.active(&catalogue, &BTreeMap::new()));
    let mut state = BTreeMap::from([(objective.gate.clone(), 4)]);
    assert!(objective.active(&catalogue, &state));
    state.insert(objective.track.clone(), 1);
    assert!(!objective.active(&catalogue, &state));
    value["quests"][0]["stages"][1]["trigger"]["objective"]["source"] = json!({"kind":"rune","item":"oteryn:item.training_rune","spell":"candidate:spell/rune/training"});
    let catalogue = parse(&value)?.attach(&base)?;
    let (_, objective) =
        hits::objective_for(&catalogue, "oteryn:quest-transition/native-hits/training")
            .ok_or("rune")?;
    assert!(objective.matches(
        "oteryn:creature.rat",
        "oteryn:item.training_rune",
        Some("candidate:spell/rune/training")
    ));
    assert!(!objective.matches("oteryn:creature.rat", "oteryn:item.training_rune", None));
    assert!(!objective.matches(
        "oteryn:creature.rat",
        "oteryn:item.training_rune",
        Some("candidate:spell/rune/other")
    ));
    Ok(())
}
