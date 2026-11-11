use super::*;

#[test]
fn world_outfits_require_a_stateful_use_and_a_closed_target() -> Result<(), String> {
    let source = include_str!("../../../../../../content/quests/shared-outfit-callbacks.json");
    let document: serde_json::Value = serde_json::from_str(source).map_err(|e| e.to_string())?;
    assert_eq!(parse(source)?.len(), 2);
    for case in 0..7 {
        let mut row = document["callbacks"][0].clone();
        match case {
            0 => row["trigger"] = "step_in".into(),
            1 => row["storage"] = serde_json::json!([]),
            2 => row["outfit_reward"]["addons"] = 4.into(),
            3 => row["outfit_reward"]["outfit"] = "oteryn:outfit.BAD".into(),
            4 => {
                for storage in row["storage"].as_array_mut().ok_or("storage")? {
                    storage["preserve"] = true.into();
                }
            }
            5 => row["storage"][0]["read_only"] = true.into(),
            _ => row["outfit_reward"]["unknown"] = true.into(),
        }
        assert!(
            parse(&serde_json::json!({"schema":1,"callbacks":[row]}).to_string()).is_err(),
            "case {case}"
        );
    }
    let loaded = load()?;
    let keys: BTreeSet<_> = loaded.iter().map(|c| &c.key).collect();
    assert_eq!(keys.len(), loaded.len());
    Ok(())
}

#[test]
fn world_requirements_reject_unbounded_and_read_only_authority() -> Result<(), String> {
    let source = include_str!("../../../../../../content/quests/questpc7-world-callbacks.json");
    let document: serde_json::Value = serde_json::from_str(source).map_err(|e| e.to_string())?;
    assert_eq!(parse(source)?.len(), 112);
    for case in 0..4 {
        let mut row = document["callbacks"][0].clone();
        match case {
            0 => row["requirements"]["min_level"] = 1_000_001.into(),
            1 => row["requirements"]["completed_quests"] = serde_json::json!(["unknown"]),
            2 => {
                row["requirements"]["owned_cosmetics"] =
                    serde_json::json!([{"outfit":"oteryn:outfit.formal_dress","addons":4}])
            }
            _ => row["storage"][0]["read_only"] = true.into(),
        }
        assert!(
            parse(&serde_json::json!({"schema":1,"callbacks":[row]}).to_string()).is_err(),
            "case {case}"
        );
    }
    Ok(())
}

#[test]
fn world_completion_requires_a_once_only_write_and_refuses_read_only_gates() -> Result<(), String> {
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/questpc7-world-callbacks.json"
    ))
    .map_err(|e| e.to_string())?;
    let control = document["callbacks"]
        .as_array()
        .ok_or("callbacks")?
        .iter()
        .find(|c| c["key"] == "oteryn:quest-callback/questpc7-hoist-control")
        .ok_or("control")?;
    for case in 0..2 {
        let mut bad = control.clone();
        if case == 0 {
            bad["storage"] = serde_json::json!([]);
        } else {
            for row in bad["storage"].as_array_mut().ok_or("storage")? {
                row["preserve"] = true.into();
            }
        }
        assert!(parse(&serde_json::json!({"schema":1,"callbacks":[bad]}).to_string()).is_err());
    }
    let mut source: serde_json::Value =
        serde_json::from_str(EMBEDDED).map_err(|e| e.to_string())?;
    let gate = source["callbacks"]
        .as_array_mut()
        .ok_or("callbacks")?
        .iter_mut()
        .find(|c| c["key"] == "oteryn:quest-callback/formogar-hoist-3059")
        .ok_or("gate")?;
    gate["completes"] = true.into();
    assert!(parse(&source.to_string()).is_err());
    Ok(())
}

#[test]
fn world_item_outputs_are_once_only_bounded_and_have_one_layout() -> Result<(), String> {
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/questpc7-world-callbacks.json"
    ))
    .map_err(|e| e.to_string())?;
    let original = document["callbacks"]
        .as_array()
        .ok_or("callbacks")?
        .iter()
        .find(|r| r["key"] == "oteryn:quest-callback/questpc7-hidden-valuables")
        .ok_or("reward")?;
    for case in 0..11 {
        let mut row = original.clone();
        match case {
            0 => row["trigger"] = "step_in".into(),
            1 => row["storage"] = serde_json::json!([]),
            2 => row["item_reward"]["count"] = 0.into(),
            3 => {
                row["item_reward"]["backpack_outputs"] = serde_json::Value::Array(vec![
                    serde_json::json!({"item":"oteryn:item.tibia.i3030","count":1});
                    20
                ])
            }
            4 => {
                row["item_reward"]["contents"] =
                    serde_json::json!([{"item":"oteryn:item.tibia.i3030","count":1}])
            }
            5 => row["reversible"] = true.into(),
            6 => {
                for storage in row["storage"].as_array_mut().ok_or("storage")? {
                    storage["preserve"] = true.into();
                }
            }
            7 => row["storage"][0]["read_only"] = true.into(),
            8 => row["activation"]["track"] = "oteryn:quest-progress/foreign/track".into(),
            9 => row["activation"]["minimum"] = 0.into(),
            _ => row["activation"]["maximum"] = 0.into(),
        }
        assert!(
            parse(&serde_json::json!({"schema":1,"callbacks":[row]}).to_string()).is_err(),
            "case {case}"
        );
    }
    Ok(())
}

#[test]
fn carried_source_requirements_reject_unbounded_and_ambiguous_inputs() -> Result<(), String> {
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/questpc7-world-callbacks.json"
    ))
    .map_err(|e| e.to_string())?;
    let original = source["callbacks"]
        .as_array()
        .ok_or("callbacks")?
        .iter()
        .find(|r| r["key"] == "oteryn:quest-callback/questpc7-hydra-nest")
        .ok_or("nest")?;
    for case in 0..8 {
        let mut row = original.clone();
        match case {
            0 => row["requirements"]["owned_items"][0]["count"] = 0.into(),
            1 => row["requirements"]["owned_items"][0]["count"] = 101.into(),
            2 => row["requirements"]["owned_items"][0]["item"] = "candidate:item/4839".into(),
            3 => {
                row["requirements"]["owned_items"][0]["map_source"] = serde_json::json!(1_u64 << 48)
            }
            4 => {
                row["requirements"]["owned_items"][0]["map_source"] = serde_json::json!(16_u64 << 8)
            }
            5 => {
                row["requirements"]["owned_items"] =
                    serde_json::Value::Array(vec![row["requirements"]["owned_items"][0].clone(); 2])
            }
            6 => {
                row["requirements"]["owned_items"] =
                    serde_json::Value::Array(vec![row["requirements"]["owned_items"][0].clone(); 9])
            }
            _ => row["requirements"]["owned_items"][0]["unknown"] = true.into(),
        }
        assert!(
            parse(&serde_json::json!({"schema":1,"callbacks":[row]}).to_string()).is_err(),
            "case {case}"
        );
    }
    Ok(())
}

#[test]
fn world_reward_bundle_is_explicit_atomic_and_once_only() -> Result<(), String> {
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../content/quests/questpc7-world-callbacks.json"
    ))
    .map_err(|e| e.to_string())?;
    let mut row = document["callbacks"]
        .as_array()
        .ok_or("callbacks")?
        .iter()
        .find(|c| c["key"] == "oteryn:quest-callback/questpc7-hidden-valuables")
        .ok_or("valuables")?
        .clone();
    row["effects"]["outfit_reward"] =
        serde_json::json!({"outfit":"oteryn:outfit.rift_warrior","addons":3});
    row["reward_bundle"] = true.into();
    row["achievements"] = serde_json::json!(["oteryn:achievement/hat_hunter"]);
    let encode =
        |row: serde_json::Value| serde_json::json!({"schema":1,"callbacks":[row]}).to_string();
    assert_eq!(
        parse(&encode(row.clone()))?.len(),
        1,
        "explicit complete reward bundle is supported"
    );
    for case in 0..10 {
        let mut bad = row.clone();
        match case {
            0 => bad["reward_bundle"] = false.into(),
            1 => bad["completes"] = false.into(),
            2 => bad["trigger"] = "step_in".into(),
            3 => {
                bad["achievements"] = serde_json::json!([
                    "oteryn:achievement/hat_hunter",
                    "oteryn:achievement/hat_hunter"
                ])
            }
            4 => bad["achievements"] = serde_json::json!(["oteryn:achievement/BAD"]),
            5 => {
                bad["achievements"] = serde_json::json!(
                    (0..9)
                        .map(|n| format!("oteryn:achievement/test_{n}"))
                        .collect::<Vec<_>>()
                )
            }
            6 => bad["item_reward"] = serde_json::Value::Null,
            7 => bad["effects"]["outfit_reward"] = serde_json::Value::Null,
            8 => {
                for s in bad["storage"].as_array_mut().ok_or("storage")? {
                    s["preserve"] = true.into();
                }
            }
            _ => {
                for s in bad["storage"].as_array_mut().ok_or("storage")? {
                    s["read_only"] = true.into();
                }
            }
        }
        assert!(parse(&encode(bad)).is_err(), "case {case}");
    }
    Ok(())
}
