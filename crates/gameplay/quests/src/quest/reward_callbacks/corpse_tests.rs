use super::*;

#[test]
fn corpse_sampling_requires_bounded_freshness_and_an_exact_species() -> Result<(), String> {
    let mut document: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../content/quests/npc-callbacks/m13-rotworm-sample.json"
    )))
    .map_err(|e| e.to_string())?;
    let row = &mut document["callbacks"][0];
    row["carried_item"] = "oteryn:item.tibia.i17670".into();
    row["creature_target"] = "oteryn:creature.slug".into();
    row["corpse_target"] = serde_json::json!({"appearance":12526,"maximum_age_ms":10000});
    row["take"][0]["item"] = "oteryn:item.tibia.i17670".into();
    row["give"]["item"] = "oteryn:item.tibia.i17672".into();
    let original = parse(&document.to_string())?.remove(0);
    let binding = gated_binding(&original)?.ok_or("corpse terms must bind progress")?;
    document["callbacks"][0]["corpse_target"]["maximum_age_ms"] = 9999.into();
    assert_ne!(
        gated_binding(&parse(&document.to_string())?.remove(0))?,
        Some(binding)
    );
    for mutation in 0..4 {
        let mut invalid = document.clone();
        match mutation {
            0 => invalid["callbacks"][0]["corpse_target"]["maximum_age_ms"] = 0.into(),
            1 => invalid["callbacks"][0]["corpse_target"]["appearance"] = 0.into(),
            2 => invalid["callbacks"][0]["creature_target"] = serde_json::Value::Null,
            _ => invalid["callbacks"][0]["capture_creature"] = true.into(),
        }
        assert!(
            parse(&invalid.to_string()).is_err(),
            "invalid corpse terms {mutation}"
        );
    }
    Ok(())
}
