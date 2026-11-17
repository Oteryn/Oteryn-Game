//! Explicit cross-floor scene syntax; existing authored scenes retain their default.
use oteryn_gameplay_quests::quest::world_callbacks::SceneRelocation;

#[test]
fn lersatio_scene_cross_floor_requires_explicit_admission() -> Result<(), serde_json::Error> {
    let legacy = serde_json::json!({
        "from": [{"x":32963,"y":31456,"floor":-2}],
        "to": {"x":32967,"y":31459,"floor":-1}
    });
    let mut explicit = legacy.clone();
    explicit["cross_floor"] = true.into();
    let admitted: SceneRelocation = serde_json::from_value(explicit)?;
    assert!(admitted.cross_floor);
    let unchanged: SceneRelocation = serde_json::from_value(legacy)?;
    assert!(!unchanged.cross_floor);
    Ok(())
}
