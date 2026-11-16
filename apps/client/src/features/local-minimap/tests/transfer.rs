use super::*;
use crate::play::{PlayView, SettingsState};

#[test]
fn minimap_settings_transfer_is_bound_validated_and_persistent()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("t074-{}", std::process::id()));
    let mut map = Minimap::default();
    map.root = Some(root.join("maps"));
    map.bind_world("11111111-1111-4111-8111-111111111111");
    map.binding = Some(([1; 32], [2; 32], 1));
    map.cells.insert((-7, 500, 500), 55);
    map.edit_marker(500, 500, -7, "Home".into(), 1)?;
    let active = map.path().ok_or("active map")?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), -7, &[])?;
    view.minimap = std::sync::Arc::new(std::sync::Mutex::new(map));
    let mut settings = SettingsState::new(Some(root.join("settings.json")));
    let export = root.join("portable.json");
    settings.request_minimap_transfer(false, export.clone());
    assert!(settings.apply_minimap_transfer(&mut view)?);
    settings.request_minimap_transfer(false, export.clone());
    assert!(settings.apply_minimap_transfer(&mut view).is_err());
    lock(&view.minimap).edit_marker(500, 500, -7, "Changed".into(), 2)?;
    settings.request_minimap_transfer(true, export.clone());
    assert!(settings.apply_minimap_transfer(&mut view)?);
    assert_eq!(lock(&view.minimap).markers[0].label, "Home");
    let bytes = std::fs::read(&active)?;
    let mut saved: serde_json::Value = serde_json::from_slice(&std::fs::read(&export)?)?;
    let mut invalid = saved.clone();
    invalid["saved"]["cells"][0][0] = serde_json::json!(100);
    std::fs::write(root.join("invalid.json"), serde_json::to_vec(&invalid)?)?;
    settings.request_minimap_transfer(true, root.join("invalid.json"));
    assert!(settings.apply_minimap_transfer(&mut view).is_err());
    assert_eq!(std::fs::read(&active)?, bytes);
    let mut other_epoch = saved.clone();
    other_epoch["binding"][2] = serde_json::json!(2);
    std::fs::write(root.join("epoch.json"), serde_json::to_vec(&other_epoch)?)?;
    settings.request_minimap_transfer(true, root.join("epoch.json"));
    assert!(settings.apply_minimap_transfer(&mut view).is_err());
    assert_eq!(std::fs::read(&active)?, bytes);
    saved["world"] = serde_json::json!("22222222-2222-4222-8222-222222222222");
    std::fs::write(root.join("wrong.json"), serde_json::to_vec(&saved)?)?;
    settings.request_minimap_transfer(true, root.join("wrong.json"));
    assert!(settings.apply_minimap_transfer(&mut view).is_err());
    assert_eq!(std::fs::read(&active)?, bytes);
    std::fs::create_dir(active.with_extension("tmp"))?;
    settings.request_minimap_transfer(true, export);
    assert!(settings.apply_minimap_transfer(&mut view).is_err());
    assert_eq!(lock(&view.minimap).markers[0].label, "Home");
    assert_eq!(std::fs::read(&active)?, bytes);
    std::fs::remove_dir(active.with_extension("tmp"))?;
    let mut reopened = Minimap::default();
    reopened.root = Some(root.join("maps"));
    reopened.bind_world("11111111-1111-4111-8111-111111111111");
    reopened.binding = Some(([1; 32], [2; 32], 1));
    reopened.load()?;
    assert_eq!(reopened.markers[0].label, "Home");
    assert!(reopened.cells.contains_key(&(-7, 500, 500)));
    drop(view);
    drop(reopened);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn old_format_positive_floors_import_as_native() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("floorframe-old-import-{}", std::process::id()));
    let mut map = Minimap::default();
    map.root = Some(root.clone());
    map.bind_world("11111111-1111-4111-8111-111111111111");
    let binding = ([1u8; 32], [2u8; 32], 1u64);
    map.binding = Some(binding);
    std::fs::create_dir_all(&root)?;
    let portable = root.join("old.json");
    std::fs::write(
        &portable,
        serde_json::to_vec(&serde_json::json!({
            "world": "11111111-1111-4111-8111-111111111111", "binding": binding,
            "saved": { "version": 1, "cells": [[7, 100, 200, 55]],
                "markers": [{ "x": 100, "y": 200, "floor": 7, "label": "Old Home", "color": 1 }] }
        }))?,
    )?;
    map.import_from(&portable)?;
    assert_eq!(map.cells.get(&(-7, 100, 200)), Some(&55));
    assert_eq!(map.markers[0].floor, -7);
    let canonical: Saved =
        serde_json::from_slice(&std::fs::read(map.path().ok_or("active path")?)?)?;
    assert_eq!(canonical.cells, vec![[-7, 100, 200, 55]]);
    assert_eq!(canonical.markers[0].floor, -7);
    drop(map);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
