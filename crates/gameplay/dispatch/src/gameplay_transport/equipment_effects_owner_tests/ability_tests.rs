use super::*;

#[test]
fn equipment_effects_owner_pinned_production_speed_matches_all_raw_attributes() -> TestResult {
    let input = input()?;
    let profiles: ItemProfilesDocument =
        serde_json::from_slice(&input.item_profiles.as_ref().ok_or("profiles")?.bytes)?;
    assert_eq!(profiles.records.len(), 413);
    let content = native(&input)?;
    for record in profiles.records {
        let abilities = item_ability_profile(&record.semantics)?;
        let sum = abilities.abilities.iter().try_fold(0_i32, |sum, ability| {
            sum.checked_add(match ability {
                ItemAbility::Speed(value) => *value,
                _ => 0,
            })
        });
        assert_eq!(
            sum, record.attributes.speed_bonus,
            "{}",
            record.production_definition.production_key
        );
        let equipment = snapshot(&content, &record.production_definition.production_key, 10);
        assert_eq!(speed_term(&equipment, &content), Some(0));
    }
    Ok(())
}

#[test]
fn equipment_effects_owner_rejects_mismatch_unknown_policy_and_invalid_slots() -> TestResult {
    let content = native(&input()?)?;
    let mut equipment = snapshot(&content, "oteryn:item.tibia.i2854", 9);
    assert!(equipment_effects(&equipment, &content).is_some());
    equipment.content_digest = [0; 32];
    assert!(equipment_effects(&equipment, &content).is_none());
    equipment.content_digest = content.source_digest();
    equipment.items[0].slot = 11;
    assert!(equipment_effects(&equipment, &content).is_none());
    equipment.items[0].slot = 9;
    equipment.items[0].definition.revision_ref = "unknown".into();
    assert!(speed_term(&equipment, &content).is_none());
    equipment.items.clear();
    assert_eq!(speed_term(&equipment, &content), Some(0));
    Ok(())
}

#[test]
fn equipment_effects_owner_uses_primary_slot_and_extra_never_suppresses() -> TestResult {
    let input = fixture_input(
        serde_json::json!([
            {"kind":"SPEED","parameter":{"state":"KNOWN","value":{"kind":"SIGNED_POINTS","value":30}},"evaluation_phase":{"state":"UNKNOWN"},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}},
            {"kind":"SUPPRESS_DROWN","parameter":{"state":"KNOWN","value":{"kind":"BOOLEAN","value":true}},"evaluation_phase":{"state":"UNKNOWN"},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}}
        ]),
        Some(30),
    )?;
    let content = native(&input)?;
    let mut equipment = snapshot(&content, "oteryn:item.tibia.i2854", 9);
    assert_eq!(speed_term(&equipment, &content), Some(30));
    assert_eq!(
        equipment_effects(&equipment, &content)
            .ok_or("effects")?
            .suppressions,
        [AbilitySuppression::Drown].into_iter().collect()
    );
    for slot in [1, 2, 3, 4, 5, 6, 7, 8, 10] {
        equipment.items[0].slot = slot;
        let effects = equipment_effects(&equipment, &content).ok_or("effects")?;
        assert_eq!(effects.speed, 0);
        assert!(effects.suppressions.is_empty());
    }
    Ok(())
}

#[test]
fn equipment_effects_owner_preserves_unknown_speed_and_rejects_ability_errors() -> TestResult {
    let content = native(&fixture_input(serde_json::json!([]), None)?)?;
    let equipment = snapshot(&content, "oteryn:item.tibia.i2854", 9);
    assert!(speed_term(&equipment, &content).is_none());
    let content = native(&fixture_input(
        serde_json::json!([
            {"kind":"SPEED","parameter":{"state":"UNKNOWN"},"evaluation_phase":{"state":"UNKNOWN"},"priority":{"state":"UNKNOWN"},"target_domain":{"state":"UNKNOWN"}}
        ]),
        Some(0),
    )?)?;
    let equipment = snapshot(&content, "oteryn:item.tibia.i2854", 9);
    assert!(equipment_effects(&equipment, &content).is_none());
    Ok(())
}
