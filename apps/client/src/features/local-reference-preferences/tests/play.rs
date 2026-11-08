use super::*;

#[test]
fn settings_transfer_backup_and_defaults_survive_restart() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::{PreferenceStore, ReferencePreferences, SettingsState};
    let root = std::env::temp_dir().join(format!("clientgap-transfer-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    let path = root.join("settings.json");
    let export = root.join("export.json");
    let mut state = SettingsState::new(Some(path.clone()));
    let mut changed = state.preferences.clone();
    changed.ui_scale = 125;
    changed.audio.master_volume = 37;
    state.apply(changed.clone());
    state.export_to(&export)?;
    assert!(state.export_to(&export).is_err()); // Never overwrite an existing export.
    state.reset_defaults()?;
    assert_eq!(state.preferences, ReferencePreferences::default());
    assert_eq!(
        PreferenceStore::load(path.clone())?.preferences(),
        &state.preferences
    );
    state.restore_backup()?;
    assert_eq!(state.preferences, changed);
    state.reset_defaults()?;
    state.import_from(&export)?;
    assert_eq!(SettingsState::new(Some(path.clone())).preferences, changed);
    std::fs::write(root.join("bad.json"), br#"{"ui_scale":181}"#)?;
    let before = std::fs::read(&path)?;
    assert!(state.import_from(&root.join("bad.json")).is_err());
    assert_eq!(state.preferences, changed);
    assert_eq!(std::fs::read(&path)?, before);
    std::fs::write(root.join("huge.json"), vec![b' '; (1 << 20) + 1])?;
    assert!(state.import_from(&root.join("huge.json")).is_err());
    // Backup failure must prevent a destructive replacement.
    std::fs::remove_file(path.with_extension("backup.json"))?;
    std::fs::create_dir(path.with_extension("backup.json"))?;
    assert!(state.reset_defaults().is_err());
    assert_eq!(state.preferences, changed);
    assert_eq!(std::fs::read(&path)?, before);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn reference_preferences_are_local_and_reject_unknown_panel_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    assert_eq!(play.preferences().ui_scale, 100);
    assert!(play.preferences().panel_visible("minimap"));

    let mut changed = play.preferences().clone();
    changed.ui_scale = 125;
    changed.visible_panels.retain(|panel| panel != "minimap");
    play.set_preferences(changed)?;
    assert_eq!(play.preferences().ui_scale, 125);
    assert!(!play.preferences().panel_visible("minimap"));

    let mut invalid = play.preferences().clone();
    invalid.visible_panels.push("unregistered-panel".to_owned());
    assert!(play.set_preferences(invalid).is_err());
    Ok(())
}

#[test]
fn preferences_are_restored_when_scene_rebuild_fails() -> Result<(), BatchError> {
    let mut play = view()?;
    let old = play.preferences().clone();
    let mut changed = old.clone();
    changed.ui_scale = 125;
    play.local.set_world(Arc::new(World::builtin_with([(
        (100, 200),
        crate::map::MapTile {
            under: vec![crate::map::Draw {
                cell: u16::MAX,
                offset: [0, 0],
            }],
            ..crate::map::MapTile::default()
        },
    )])?));
    play.map_floor = Some(play.floor);
    assert!(play.set_preferences(changed).is_err());
    assert_eq!(play.preferences(), &old);
    Ok(())
}

#[test]
fn settings_rebind_conflicts_routes_and_survives_restart() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::{PreferenceStore, preference_bindings};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let path = std::env::temp_dir().join(format!("oteryn-c25-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut store = PreferenceStore::load(path.clone())?;
    let bindings = preference_bindings()?;
    let highscores = "0009-highscores-open";
    assert!(bindings.iter().any(|binding| binding.id == highscores));
    let mut changed = store.preferences().clone();
    assert!(changed.rebind(highscores, "Ctrl+L").is_err());
    assert!(changed.rebind(highscores, "F10").is_err());
    changed.rebind(highscores, "Alt+H")?;
    changed.ui_scale = 125;
    store.apply(changed)?;
    let restored = PreferenceStore::load(path.clone())?;
    assert_eq!(restored.preferences(), store.preferences());
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(11)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(oteryn_input_actions::Modifier::Alt),
        repeat: false,
    };
    let routed = restored.preferences().route_input(&[event])?;
    assert!(oteryn_client_ui::highscores::open_key(&routed));
    let old = NormalizedInputEvent::Key {
        code: KeyCode::new(11)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(oteryn_input_actions::Modifier::Control),
        repeat: false,
    };
    assert!(restored.preferences().route_input(&[old])?.is_empty());
    let mut play = view()?;
    play.set_preferences(restored.preferences().clone())?;
    assert_eq!(play.scene().view().tile_px(), 49);
    play.set_world_viewport([600, 330])?;
    assert_eq!(play.scene().view().tile_px(), 30);
    assert_eq!(play.preferences().ui_scale, 125);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn every_registered_binding_conflicts_and_invalid_saves_leave_preferences_unchanged()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PreferenceStore, ReferencePreferences, preference_bindings};
    let original = ReferencePreferences::default();
    for binding in preference_bindings()? {
        if binding.id == "0009-highscores-open" {
            continue;
        }
        let mut preferences = original.clone();
        assert!(
            preferences
                .rebind("0009-highscores-open", binding.default_chord)
                .is_err(),
            "{}",
            binding.id
        );
        assert_eq!(preferences, original);
    }
    let mut preferences = original.clone();
    assert!(preferences.rebind("unknown", "ALT+H").is_err());
    assert!(
        preferences
            .rebind("0009-highscores-open", "CTRL+CTRL+H")
            .is_err()
    );
    let path = std::env::temp_dir().join(format!("oteryn-c25-invalid-{}", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut store = PreferenceStore::load(path.clone())?;
    preferences.ui_scale = 181;
    assert!(store.apply(preferences).is_err());
    assert_eq!(store.preferences(), &original);
    assert!(!path.exists());
    std::fs::write(&path, b"invalid-json")?;
    assert!(PreferenceStore::load(path.clone()).is_err());
    assert_eq!(std::fs::read(&path)?, b"invalid-json");
    std::fs::remove_file(&path)?;
    std::fs::create_dir(&path)?;
    let mut blocked = PreferenceStore::defaults(path.clone());
    let mut changed = original.clone();
    changed.audio.master_volume = 35;
    assert!(blocked.apply(changed).is_err());
    assert_eq!(blocked.preferences(), &original);
    assert!(!path.with_extension("json.tmp").exists());
    std::fs::remove_dir(path)?;
    Ok(())
}

#[test]
fn rebound_spell_releases_after_modifier_and_preference_changes()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PreferenceInput, ReferencePreferences};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut preferences = ReferencePreferences::default();
    preferences.rebind("local-spell-action-bar-spell-1", "ALT+H")?;
    let code = KeyCode::new(11)?;
    let key = |state, modifiers| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat: false,
    };
    let mut input = PreferenceInput::default();
    let mut spells = crate::spell::SpellHotkeys::new()?;
    let press = input.route(
        &preferences,
        &[key(ButtonState::Pressed, Modifiers::one(Modifier::Alt))],
    )?;
    assert_eq!(spells.route(&press).len(), 1);
    preferences.rebind("local-spell-action-bar-spell-1", "ALT+J")?;
    let release = input.route(&preferences, &[key(ButtonState::Released, Modifiers::NONE)])?;
    spells.route(&release);
    let press = input.route(
        &preferences,
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(13)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Alt),
            repeat: false,
        }],
    )?;
    assert_eq!(spells.route(&press).len(), 1);
    Ok(())
}

#[test]
fn unregistered_and_sibling_chords_are_reserved() -> Result<(), Box<dyn std::error::Error>> {
    for chord in [
        "CTRL+G",
        "ENTER",
        "KEYPADENTER",
        "CTRL+SHIFT+G",
        "CTRL+SHIFT+L",
        "CTRL+P",
    ] {
        let mut preferences = crate::play::ReferencePreferences::default();
        assert!(
            preferences.rebind("0009-highscores-open", chord).is_err(),
            "{chord}"
        );
    }
    Ok(())
}

#[test]
fn moved_logout_suppresses_all_original_modifier_variants() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::ReferencePreferences;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut preferences = ReferencePreferences::default();
    preferences.rebind("0021-logout-logout-l", "ALT+L")?;
    for bits in 0..16 {
        let modifiers = Modifiers::from_bits(bits)?;
        if !modifiers.contains(Modifier::Control) {
            continue;
        }
        let routed = preferences.route_input(&[NormalizedInputEvent::Key {
            code: KeyCode::new(15)?,
            state: ButtonState::Pressed,
            modifiers,
            repeat: false,
        }])?;
        assert!(!crate::input::logout_key(&routed), "modifier bits {bits}");
    }
    Ok(())
}

#[test]
fn t010_hud_layout_is_validated_and_restored() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    let mut json = serde_json::to_value(play.preferences())?;
    json["hud_bounds"] = serde_json::json!([20, 80, 640, 60]);
    play.set_preferences(serde_json::from_value(json.clone())?)?;
    assert_eq!(
        serde_json::to_value(play.preferences())?["hud_bounds"],
        json["hud_bounds"]
    );
    json["hud_bounds"] = serde_json::json!([20, 80, 0, 60]);
    assert!(play.set_preferences(serde_json::from_value(json)?).is_err());
    Ok(())
}

#[test]
fn t011_vitals_bars_reach_live_scene_and_can_be_hidden() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    play.vitals = Some(oteryn_session::ActorVitals {
        health: 50,
        max_health: 100,
        mana: 20,
        max_mana: 40,
        ..Default::default()
    });
    play.rebuild()?;
    let shown = play.scene().sprites().instances().len();
    let mut json = serde_json::to_value(play.preferences())?;
    json["hud_bars"] = serde_json::json!(false);
    play.set_preferences(serde_json::from_value(json)?)?;
    assert_eq!(play.scene().sprites().instances().len() + 4, shown);
    Ok(())
}

#[test]
fn t012_visual_preferences_control_live_effect_projection() -> Result<(), Box<dyn std::error::Error>>
{
    let mut play = view()?;
    let now = std::time::Instant::now();
    let baseline = play.scene().sprites().len();
    play.receive_presentation(
        &[oteryn_session::PresentationEventKind::MagicEffect {
            position: oteryn_session::ActorPosition {
                x: play.own.x,
                y: play.own.y,
                floor: play.floor,
            },
            effect_id: 29,
            source: oteryn_session::PresentationSource::Own,
        }],
        now,
    );
    play.project_presentation(now)?;
    assert!(play.scene().sprites().len() > baseline);
    let mut json = serde_json::to_value(play.preferences())?;
    json["show_effects"] = serde_json::json!(false);
    json["show_lights"] = serde_json::json!(false);
    play.set_preferences(serde_json::from_value(json)?)?;
    assert_eq!(play.scene().sprites().len(), baseline);
    assert!(play.scene().sprites().lighting.sources.is_empty());
    Ok(())
}
