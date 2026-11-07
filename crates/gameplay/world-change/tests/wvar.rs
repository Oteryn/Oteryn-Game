use oteryn_gameplay_world_change::{daily::DailyWorldChanges, variants};

#[test]
fn wvar_selection_is_stored_and_old_decisions_never_activate_variants()
-> Result<(), Box<dyn std::error::Error>> {
    let definitions = variants::definitions()?;
    assert!(definitions.iter().any(|d| d.key == "fury_gates"));
    let first = DailyWorldChanges::roll(8, 5, 1, |_| 0);
    assert_eq!(first.variant("fury_gates"), Some(0));
    assert_eq!(first.variant("nightmare_isles"), Some(0));
    let bytes = serde_json::to_vec(&first)?;
    let snapshot: serde_json::Value = serde_json::from_slice(&bytes)?;
    assert!(snapshot.get("counter_stages").is_none());
    assert!(snapshot.get("counter_generations").is_none());
    let restored: DailyWorldChanges = serde_json::from_slice(&bytes)?;
    assert_eq!(serde_json::to_value(restored)?, snapshot);
    assert_eq!(restored.counter_stages, [0; 8]);
    assert_eq!(restored.counter_generations, [0; 8]);
    let mut persisted = first;
    persisted.counter_stages = [0; 8];
    persisted.counter_generations = [0; 8];
    assert_eq!(restored, persisted);
    let mut legacy = serde_json::to_value(first)?;
    legacy.as_object_mut().ok_or("object")?.remove("variants");
    let legacy: DailyWorldChanges = serde_json::from_value(legacy)?;
    assert_eq!(legacy.variant("fury_gates"), None);
    assert_eq!(
        DailyWorldChanges::roll(9, 5, 1, |_| u64::MAX).variant("fury_gates"),
        None
    );
    assert_eq!(
        DailyWorldChanges::roll(9, 5, 1, |_| u64::MAX).variant("nightmare_isles"),
        Some(2)
    );
    Ok(())
}

#[test]
fn wvar_portal_pairs_preserve_return_markers_and_eligibility()
-> Result<(), Box<dyn std::error::Error>> {
    let definitions = variants::definitions()?;
    for (key, count, action, unique) in [
        ("fury_gates", 10, Some(9715), None),
        ("nightmare_isles", 3, None, Some(35020)),
    ] {
        let definition = definitions
            .iter()
            .find(|d| d.key == key)
            .ok_or("definition")?;
        assert_eq!(definition.variants.len(), count);
        for variant in &definition.variants {
            assert_eq!(variant.portals.len(), 2);
            assert_eq!(variant.tags.len(), 1);
            assert_eq!(variant.tags[0].action, action);
            assert_eq!(variant.tags[0].unique, unique);
            let entry = &variant.portals[0];
            assert_eq!(entry.premium, key == "fury_gates");
            assert_eq!(entry.promoted, key == "fury_gates");
            assert_eq!(entry.min_level, if key == "fury_gates" { 60 } else { 0 });
            assert!(!variant.portals[1].premium);
        }
    }
    Ok(())
}

#[test]
fn wvar_closed_change_keeps_only_its_safe_return() -> Result<(), Box<dyn std::error::Error>> {
    let selected = variants::Selected::default();
    assert!(selected.passage((32680, 31719, 7)).is_none());
    assert_eq!(
        selected.passage((33290, 31785, 13)).map(|p| p.destination),
        Some((32680, 31720, 7))
    );
    assert!(selected.passage((32516, 32018, 9)).is_none());
    assert_eq!(
        selected.passage((32516, 32018, 10)).map(|p| p.destination),
        Some((32515, 32018, 9))
    );
    Ok(())
}
