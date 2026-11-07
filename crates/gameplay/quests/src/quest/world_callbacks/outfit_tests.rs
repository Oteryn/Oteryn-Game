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
    assert_eq!(parse(source)?.len(), 2);
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
