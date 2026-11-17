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
    explicit["players_only"] = true.into();
    let admitted: SceneRelocation = serde_json::from_value(explicit)?;
    assert!(admitted.cross_floor);
    assert!(admitted.players_only);
    let unchanged: SceneRelocation = serde_json::from_value(legacy)?;
    assert!(!unchanged.cross_floor);
    assert!(!unchanged.players_only);
    Ok(())
}

#[test]
fn lersatio_scene_admission_options_reject_mistyped_flags() {
    for flag in ["cross_floor", "players_only"] {
        let mut relocation = serde_json::json!({
            "from": [{"x":32963,"y":31456,"floor":-2}],
            "to": {"x":32967,"y":31459,"floor":-1}
        });
        relocation[flag] = "true".into();
        assert!(serde_json::from_value::<SceneRelocation>(relocation).is_err());
    }
}

#[test]
fn lersatio_encounter_accepts_bounded_current_spectator_input() -> Result<(), serde_json::Error> {
    use oteryn_gameplay_quests::quest::world_callbacks::Encounter;
    let encounter: Encounter = serde_json::from_value(serde_json::json!({
        "minimum_level":0,
        "players":[],
        "monsters":[{"creature":"oteryn:creature.lersatio","position":{"x":32967,"y":31459,"floor":-1}}],
        "room_min":{"x":32963,"y":31456,"floor":-1},
        "room_max":{"x":32972,"y":31465,"floor":-1},
        "spectators":{
            "minimum":{"x":32954,"y":31447,"floor":-2},
            "maximum":{"x":32972,"y":31465,"floor":-2},
            "destination":{"x":32967,"y":31459,"floor":-1}
        }
    }))?;
    assert!(encounter.players.is_empty());
    assert_eq!(encounter.minimum_level, 0);
    let spectators = encounter.spectators.ok_or_else(|| {
        <serde_json::Error as serde::de::Error>::custom("spectator input missing")
    })?;
    assert_eq!(spectators.minimum.floor, -2);
    assert_eq!(spectators.destination.floor, -1);
    Ok(())
}
