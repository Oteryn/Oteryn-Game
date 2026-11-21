use super::Bindings;
use std::collections::BTreeMap;

#[test]
fn production_draconia_binds_native_key_3012_without_completing_the_route() -> Result<(), String> {
    let bindings = Bindings::parse(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/quests/authored-bindings/qmech-m01c-draconia.json"
    )))?;
    bindings.verify_embedded_recipe_evidence()?;
    let mut completion = bindings.clone();
    completion.quests[0].stages[0].completes = true;
    assert!(completion.verify_embedded_recipe_evidence().is_err());
    let catalogue = bindings
        .attach(&crate::quest::QuestStateCatalogue::empty("draconia-native").ok_or("catalogue")?)?;
    for (x, y) in [(32675_u64, 31649_u64), (32676, 31671)] {
        let mut placement = vec![19; 32];
        placement.extend_from_slice(&((x << 32) | (y << 16) | (246 << 8) | 1).to_be_bytes());
        let candidates =
            bindings.door_use_candidates(&catalogue, &BTreeMap::new(), &placement, 3012);
        assert_eq!(candidates.len(), 1);
        assert!(
            bindings
                .door_use_candidates(&catalogue, &BTreeMap::new(), &placement, 3013)
                .is_empty()
        );
        let transition = catalogue.transition(&candidates[0]).ok_or("transition")?;
        let changes = catalogue
            .evaluate(transition, &BTreeMap::new(), 0)
            .map_err(|error| format!("{error:?}"))?;
        assert!(!catalogue.completes_after(transition, &changes));
        let progressed = changes
            .into_iter()
            .map(|change| (change.track, change.after))
            .collect();
        assert!(
            bindings
                .door_use_candidates(&catalogue, &progressed, &placement, 3012)
                .is_empty()
        );
    }
    Ok(())
}

#[test]
fn door_use_binding_requires_exact_native_placement_key_number_and_active_phase()
-> Result<(), String> {
    let bindings = Bindings::parse(include_bytes!("fixtures/door_use.json"))?;
    let catalogue = bindings
        .attach(&crate::quest::QuestStateCatalogue::empty("door-fixture").ok_or("catalogue")?)?;
    let mut placement = vec![19; 32];
    placement.extend_from_slice(&7_u64.to_be_bytes());
    let key = "oteryn:quest-door-use/m01c-fixture";
    assert_eq!(
        bindings.door_use_candidates(&catalogue, &BTreeMap::new(), &placement, 3520),
        vec![key]
    );
    assert!(
        bindings
            .door_use_candidates(&catalogue, &BTreeMap::new(), &placement, 3519)
            .is_empty()
    );
    placement[39] = 8;
    assert!(
        bindings
            .door_use_candidates(&catalogue, &BTreeMap::new(), &placement, 3520)
            .is_empty()
    );
    placement[39] = 7;
    assert!(
        bindings
            .door_use_candidates(&catalogue, &BTreeMap::new(), &placement[32..], 3520)
            .is_empty()
    );
    let transition = catalogue.transition(key).ok_or("transition")?;
    let changes = catalogue
        .evaluate(transition, &BTreeMap::new(), 0)
        .map_err(|e| format!("{e:?}"))?;
    assert!(!catalogue.completes_after(transition, &changes));
    let state = changes.into_iter().map(|c| (c.track, c.after)).collect();
    assert!(
        bindings
            .door_use_candidates(&catalogue, &state, &placement, 3520)
            .is_empty()
    );
    Ok(())
}

#[test]
fn door_use_binding_rejects_ambiguous_unbounded_or_foreign_guards() -> Result<(), String> {
    for field in ["key_number", "native_placement_key", "storage"] {
        let mut value =
            serde_json::from_slice::<serde_json::Value>(include_bytes!("fixtures/door_use.json"))
                .map_err(|e| e.to_string())?;
        value["quests"][0]["stages"][0]["trigger"]["usage"]
            .as_object_mut()
            .ok_or("usage")?
            .remove(field);
        assert!(Bindings::parse(&serde_json::to_vec(&value).map_err(|e| e.to_string())?).is_err());
    }
    for (field, value) in [
        ("key_number", serde_json::json!(0)),
        ("native_placement_key", serde_json::json!(u64::MAX)),
        ("storage", serde_json::json!([])),
    ] {
        let mut doc =
            serde_json::from_slice::<serde_json::Value>(include_bytes!("fixtures/door_use.json"))
                .map_err(|e| e.to_string())?;
        doc["quests"][0]["stages"][0]["trigger"]["usage"][field] = value;
        assert!(Bindings::parse(&serde_json::to_vec(&doc).map_err(|e| e.to_string())?).is_err());
    }
    let mut doc =
        serde_json::from_slice::<serde_json::Value>(include_bytes!("fixtures/door_use.json"))
            .map_err(|e| e.to_string())?;
    doc["quests"][0]["stages"][0]["trigger"]["usage"]["storage"][0]["track"] =
        serde_json::json!("oteryn:quest-progress/foreign");
    assert!(Bindings::parse(&serde_json::to_vec(&doc).map_err(|e| e.to_string())?).is_err());
    Ok(())
}
