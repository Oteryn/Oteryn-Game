//! These tests exercise authored data through the catalogue used by the durable writer.
use super::*;
use crate::quest::{QuestRefusal, cults_of_tibia as cults, unnatural_selection as dance};
use std::collections::BTreeMap;

fn base() -> serde_json::Value {
    serde_json::json!({
        "key":"oteryn:quest-callback/batch4-regression",
        "quest":dance::QUEST,
        "trigger":"step_in",
        "selector":{"position":{"x":32991,"y":31496,"floor":-1},"unique":null,"action":null,"appearance":null},
        "lever":null,"reversible":false,"passages":[],"teleport":null,
        "storage":[],"offerings":[],"item_on_target":null,
        "return_on_failure":false,"failure_message":null,
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/unnatural_selection/movements_mission3_dance_dance_evolution.lua:1-67"]
    })
}

fn callbacks(value: serde_json::Value) -> Result<Vec<Callback>, String> {
    parse(&serde_json::json!({"schema":1,"callbacks":[value]}).to_string())
}

#[test]
fn world_callbacks_source_presentation_rejects_invalid_or_unrelated_effects() -> Result<(), String>
{
    let mut value = base();
    value["teleport"] = serde_json::json!({"x":32968,"y":32631,"floor":-8});
    value["facing"] = 2.into();
    value["effects"] = serde_json::json!([
        {"position":{"x":32968,"y":32626,"floor":-5},"effect":54}
    ]);
    assert!(callbacks(value.clone()).is_ok());
    for (field, invalid) in [
        ("facing", serde_json::json!(4)),
        ("teleport", serde_json::Value::Null),
        (
            "effects",
            serde_json::json!([{"position":{"x":1,"y":2,"floor":1},"effect":54}]),
        ),
        (
            "effects",
            serde_json::json!([{"position":{"x":1,"y":2,"floor":-1},"effect":0}]),
        ),
        (
            "effects",
            serde_json::Value::Array(vec![value["effects"][0].clone(); MAX_EFFECTS + 1]),
        ),
    ] {
        let mut invalid_callback = value.clone();
        invalid_callback[field] = invalid;
        assert!(
            callbacks(invalid_callback).is_err(),
            "accepted invalid {field}"
        );
    }
    let mut native = dance_callback();
    native["effects"] = value["effects"].clone();
    assert!(
        callbacks(native).is_err(),
        "native rule admitted unrelated presentation"
    );
    Ok(())
}

fn catalogue(callbacks: &[Callback]) -> Result<QuestStateCatalogue, String> {
    let lowered = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    extend_callbacks(lowered.catalogue().clone(), callbacks)
}

fn dance_callback() -> serde_json::Value {
    let mut value = base();
    value["native_rule"] = serde_json::json!({"kind":"unnatural_dance","tile":2});
    value["storage"] = serde_json::json!([
        {"track":dance::DANCE_STATUS_TRACK,"minimum":1,"maximum":16,"set":1,"preserve":true},
        {"track":dance::MISSION03_TRACK,"minimum":-1,"maximum":3,"set":3,"preserve":true},
        {"track":dance::QUESTLINE_TRACK,"minimum":-1,"maximum":15,"set":7,"preserve":true}
    ]);
    value
}

#[test]
fn preserve_guard_does_not_reset_later_progress() -> Result<(), String> {
    let mut value = base();
    value["storage"] = serde_json::json!([
        {"track":dance::QUESTLINE_TRACK,"minimum":4,"maximum":15,"set":4,"preserve":true}
    ]);
    value["success_message"] = "Krunus should be pleased.".into();
    let callbacks = callbacks(value)?;
    let catalogue = catalogue(&callbacks)?;
    let transition = catalogue
        .transition(&callbacks[0].key)
        .ok_or("transition")?;
    let stored = BTreeMap::from([(dance::QUESTLINE_TRACK.into(), 12)]);
    let changes = catalogue
        .evaluate(transition, &stored, 0)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!((changes[0].before, changes[0].after), (12, 12));
    Ok(())
}

#[test]
fn preserve_guard_never_widens_an_existing_track_domain() -> Result<(), String> {
    let original = crate::quest::loader::load_embedded_quest_state("content-1")
        .map_err(|e| format!("{e:?}"))?;
    let original_track = original
        .catalogue()
        .track(dance::QUESTLINE_TRACK)
        .ok_or("track")?;
    let mut value = base();
    value["storage"] = serde_json::json!([
        {"track":dance::QUESTLINE_TRACK,"minimum":4,"maximum":9223372036854775807_i64,
         "set":9223372036854775807_i64,"preserve":true}
    ]);
    let callbacks = callbacks(value)?;
    let catalogue = catalogue(&callbacks)?;
    assert_eq!(
        catalogue.track(dance::QUESTLINE_TRACK),
        Some(original_track)
    );
    Ok(())
}

#[test]
fn dance_final_step_uses_one_complete_locked_snapshot() -> Result<(), String> {
    let callbacks = callbacks(dance_callback())?;
    let catalogue = catalogue(&callbacks)?;
    let transition = catalogue
        .transition(&callbacks[0].key)
        .ok_or("transition")?;
    let stored = BTreeMap::from([
        (dance::DANCE_STATUS_TRACK.into(), 16),
        (dance::MISSION03_TRACK.into(), 0),
        (dance::QUESTLINE_TRACK.into(), 15),
    ]);
    let changes = catalogue
        .evaluate(transition, &stored, 0)
        .map_err(|e| format!("{e:?}"))?;
    let after: BTreeMap<_, _> = changes
        .into_iter()
        .map(|change| (change.track, change.after))
        .collect();
    assert_eq!(
        after,
        BTreeMap::from([
            (dance::DANCE_STATUS_TRACK.into(), 17),
            (dance::MISSION03_TRACK.into(), 3),
            (dance::QUESTLINE_TRACK.into(), 7),
        ])
    );
    assert_eq!(
        catalogue.evaluate(transition, &after, 0),
        Err(QuestRefusal::StageMismatch)
    );
    Ok(())
}

#[test]
fn cults_tenth_symbol_and_repeat_use_the_catalogue_rule() -> Result<(), String> {
    let mut value = base();
    value["quest"] = cults::QUEST.into();
    value["trigger"] = "use".into();
    value["selector"] = serde_json::json!({"position":{"x":32683,"y":31537,"floor":-9},"unique":null,"action":5535,"appearance":null});
    value["sources"] = serde_json::json!([
        "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/cults_of_tibia/actions_cult_symbol.lua:1-51"
    ]);
    value["native_rule"] = serde_json::json!({"kind":"cults_symbol","symbol":9});
    value["storage"] = serde_json::json!([
        {"track":cults::MISSION,"minimum":0,"maximum":7,"set":0,"preserve":true},
        {"track":cults::TEMP,"minimum":0,"maximum":2147483647,"set":0,"preserve":true},
        {"track":cults::OBJECTS,"minimum":0,"maximum":2147483647,"set":0,"preserve":true}
    ]);
    let callbacks = callbacks(value)?;
    let catalogue = catalogue(&callbacks)?;
    let transition = catalogue
        .transition(&callbacks[0].key)
        .ok_or("transition")?;
    let stored = BTreeMap::from([
        (cults::MISSION.into(), 7),
        (cults::TEMP.into(), 511),
        (cults::OBJECTS.into(), 9),
    ]);
    let changes = catalogue
        .evaluate(transition, &stored, 0)
        .map_err(|e| format!("{e:?}"))?;
    let after: BTreeMap<_, _> = changes
        .into_iter()
        .map(|change| (change.track, change.after))
        .collect();
    assert_eq!(
        after,
        BTreeMap::from([
            (cults::MISSION.into(), 4),
            (cults::TEMP.into(), 1023),
            (cults::OBJECTS.into(), 10),
        ])
    );
    assert_eq!(
        catalogue.evaluate(transition, &after, 0),
        Err(QuestRefusal::StageMismatch)
    );
    Ok(())
}

#[test]
fn native_callback_rejects_incomplete_or_foreign_authoring() {
    for case in 0..6 {
        let mut value = dance_callback();
        match case {
            0 => {
                value["storage"].as_array_mut().map(Vec::pop);
            }
            1 => value["quest"] = cults::QUEST.into(),
            2 => value["native_rule"]["tile"] = 10.into(),
            3 => value["storage"][0]["preserve"] = false.into(),
            4 => value["trigger"] = "use".into(),
            _ => value["selector"]["position"]["x"] = 1.into(),
        }
        assert!(callbacks(value).is_err(), "case {case}");
    }
}

#[test]
fn triangle_tower_wall_preserves_source_remove_and_create_ids() -> Result<(), String> {
    let callback = load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/triangle-tower-lever")
        .ok_or("Triangle Tower UID30034 has no admitted source callback")?;
    assert_eq!(callback.selector.unique, Some(30034));
    assert_eq!(
        callback.selector.position,
        Some(Position {
            x: 32573,
            y: 32121,
            floor: -7
        })
    );
    assert_eq!(callback.lever, Some([2772, 2773]));
    assert!(callback.reversible);
    let value = serde_json::to_value(callback).map_err(|e| e.to_string())?;
    assert_eq!(
        value["effects"]["item_cycle"],
        serde_json::json!({"position":{"x":32566,"y":32119,"floor":-7},"remove":1270,"create":1025})
    );
    Ok(())
}

#[test]
fn asymmetric_wall_rejects_effects_it_cannot_commit() -> Result<(), String> {
    // Main's Triangle uses item_cycle now. The retained source packet still
    // exercises the separate asymmetric-passage validator without registering
    // a second production owner for that same selector.
    let packet: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../tools/content-schema/quest-authoring/batches/quests4/triangle_tower_quest.json"
    )))
    .map_err(|error| error.to_string())?;
    let value = packet["callbacks"][0].clone();
    callbacks(value.clone())?;
    for (field, invalid) in [
        (
            "effects",
            serde_json::json!([{"position":{"x":32566,"y":32119,"floor":-7},"effect":54}]),
        ),
        ("success_message", serde_json::json!("unhandled message")),
        ("experience", serde_json::json!(1)),
    ] {
        let mut value = value.clone();
        value[field] = invalid;
        assert!(callbacks(value).is_err(), "accepted unsupported {field}");
    }
    Ok(())
}

#[test]
fn m02_ground_stack_requires_exact_ordered_bounded_items_and_durable_admission()
-> Result<(), String> {
    let mut value = base();
    value["requires_world_admission"] = true.into();
    value["storage"] = serde_json::json!([
        {"track":dance::QUESTLINE_TRACK,"minimum":-1,"maximum":15,"set":1}
    ]);
    value["world_requirements"] = serde_json::json!([{
        "position":{"x":32991,"y":31496,"floor":-1}, "appearance":0,
        "ground_stack":[{"item":"oteryn:item.tibia.i2471","count":1},
                        {"item":"oteryn:item.tibia.i2904","count":1}]
    }]);
    assert!(
        callbacks(value.clone()).is_ok(),
        "portable stacks need a durable read-set"
    );
    for case in 0..6 {
        let mut bad = value.clone();
        match case {
            0 => bad["storage"] = serde_json::json!([]),
            1 => bad["world_requirements"][0]["ground_stack"][0]["count"] = 0.into(),
            2 => bad["world_requirements"][0]["ground_stack"][0]["item"] = "*".into(),
            3 => bad["world_requirements"][0]["ground_stack"][0]["count"] = 101.into(),
            4 => bad["world_requirements"][0]["ground_stack"] = serde_json::json!([]),
            _ => {
                bad["world_requirements"][0]["ground_stack"] = serde_json::json!(vec![
                        value["world_requirements"][0]["ground_stack"][0]
                            .clone();
                        MAX_EFFECTS + 1
                    ])
            }
        }
        assert!(
            callbacks(bad).is_err(),
            "accepted unsafe Ground prerequisite {case}"
        );
    }
    Ok(())
}
