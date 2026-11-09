use super::Bindings;
use serde_json::{Value, json};

pub(super) fn document() -> Value {
    json!({"schema":1,"quests":[{
        "quest":"oteryn:quest.m01c_terrain_fixture","recipe_sha256":"a".repeat(64),
        "tracks":[{"key":"oteryn:quest-progress/m01c-terrain","initial":0,"minimum":0,"maximum":2}],
        "stages":[{"key":"s1","completes":false,"trigger":{"kind":"terrain_use","usage":{
            "key":"oteryn:quest-terrain-use/m01c-fixture",
            "tools":[{"key":"oteryn:item.tibia.i3457","revision":"definition-r1"}],
            "placement":vec![49_u8;40],"result":469,
            "storage":[{"track":"oteryn:quest-progress/m01c-terrain","minimum":0,"maximum":0,"set":1,"preserve":false}],
            "sources":["M01c engine fixture; no production quest qualification"]
        }}}],"sources":["M01c engine fixture"]
    }]})
}

#[test]
fn terrain_use_accepts_an_exact_finite_native_receipt_binding() -> Result<(), String> {
    let bindings = Bindings::parse(&serde_json::to_vec(&document()).map_err(|e| e.to_string())?)?;
    let encoded = serde_json::to_value(bindings).map_err(|e| e.to_string())?;
    assert_eq!(
        encoded["quests"][0]["stages"][0]["trigger"]["kind"],
        "terrain_use"
    );
    Ok(())
}

#[test]
fn terrain_use_rejects_missing_ambiguous_or_unbounded_receipt_inputs() -> Result<(), String> {
    for (field, value) in [
        ("placement", json!(vec![49_u8; 39])),
        ("result", json!(0)),
        ("tools", json!([])),
        (
            "tools",
            json!([{"key":"oteryn:item.tibia.i3457","revision":""}]),
        ),
        (
            "tools",
            json!([{"key":"oteryn:item.tibia.i3457","revision":"definition-r1"},{"key":"oteryn:item.tibia.i3457","revision":"definition-r1"}]),
        ),
        (
            "storage",
            json!([{"track":"oteryn:quest-progress/foreign","minimum":0,"maximum":0,"set":1,"preserve":false}]),
        ),
        ("sources", json!([])),
    ] {
        let mut value_document = document();
        value_document["quests"][0]["stages"][0]["trigger"]["usage"][field] = value;
        assert!(
            Bindings::parse(&serde_json::to_vec(&value_document).map_err(|e| e.to_string())?)
                .is_err(),
            "accepted invalid {field}"
        );
    }
    Ok(())
}

#[test]
fn terrain_use_candidates_match_tool_revision_target_result_and_initial_storage()
-> Result<(), String> {
    use super::terrain_use::ToolDefinition;
    use std::collections::BTreeMap;
    let bindings = Bindings::parse(&serde_json::to_vec(&document()).map_err(|e| e.to_string())?)?;
    let base = crate::quest::QuestStateCatalogue::empty("content-1").ok_or("revision")?;
    let catalogue = bindings.attach(&base)?;
    let tool = ToolDefinition {
        key: "oteryn:item.tibia.i3457".into(),
        revision: "definition-r1".into(),
    };
    let initial = BTreeMap::new();
    assert_eq!(
        bindings.terrain_use_candidates(&catalogue, &initial, &tool, &[49; 40], 469),
        vec!["oteryn:quest-terrain-use/m01c-fixture"]
    );
    let changed = ToolDefinition {
        revision: "definition-r2".into(),
        ..tool.clone()
    };
    assert!(
        bindings
            .terrain_use_candidates(&catalogue, &initial, &changed, &[49; 40], 469)
            .is_empty()
    );
    assert!(
        bindings
            .terrain_use_candidates(&catalogue, &initial, &tool, &[50; 40], 469)
            .is_empty()
    );
    assert!(
        bindings
            .terrain_use_candidates(&catalogue, &initial, &tool, &[49; 40], 470)
            .is_empty()
    );
    let finished = BTreeMap::from([("oteryn:quest-progress/m01c-terrain".into(), 1)]);
    assert!(
        bindings
            .terrain_use_candidates(&catalogue, &finished, &tool, &[49; 40], 469)
            .is_empty()
    );
    Ok(())
}

#[test]
fn terrain_use_position_key_survives_map_repacking_but_keeps_tile_and_ordinal_exact()
-> Result<(), String> {
    use super::terrain_use::ToolDefinition;
    use std::collections::BTreeMap;
    let mut value = document();
    let usage = &mut value["quests"][0]["stages"][0]["trigger"]["usage"];
    usage.as_object_mut().ok_or("usage")?.remove("placement");
    let key = (32493_u64 << 32) | (32259_u64 << 16) | (7_u64 << 8);
    usage["placement_key"] = key.into();
    let bindings = Bindings::parse(&serde_json::to_vec(&value).map_err(|e| e.to_string())?)?;
    let catalogue = bindings
        .attach(&crate::quest::QuestStateCatalogue::empty("content-1").ok_or("revision")?)?;
    let tool = ToolDefinition {
        key: "oteryn:item.tibia.i3457".into(),
        revision: "definition-r1".into(),
    };
    for digest in [1_u8, 2] {
        let mut placement = vec![digest; 32];
        placement.extend_from_slice(&key.to_be_bytes());
        assert_eq!(
            bindings
                .terrain_use_candidates(&catalogue, &BTreeMap::new(), &tool, &placement, 469)
                .len(),
            1
        );
        placement[39] = 1;
        assert!(
            bindings
                .terrain_use_candidates(&catalogue, &BTreeMap::new(), &tool, &placement, 469)
                .is_empty()
        );
    }
    value["quests"][0]["stages"][0]["trigger"]["usage"]["placement"] =
        serde_json::json!(vec![49_u8; 40]);
    assert!(Bindings::parse(&serde_json::to_vec(&value).map_err(|e| e.to_string())?).is_err());
    Ok(())
}
