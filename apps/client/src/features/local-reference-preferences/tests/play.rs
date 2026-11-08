use super::*;

#[test]
fn basic_options_navigate_without_changing_preferences() {
    use crate::play::{SettingsPage, SettingsState};
    let mut state = SettingsState::new(None);
    let before = state.preferences.clone();
    assert_eq!(state.page, SettingsPage::Gameplay);
    for page in SettingsPage::ALL {
        state.page = page;
        assert!(!page.label().is_empty());
        assert_eq!(state.preferences, before);
    }
    assert_eq!(SettingsPage::ALL.len(), 5);
}

#[test]
fn named_hotkey_profiles_keep_chat_maps_and_secondary_bindings_separate()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PreferenceInput, PreferenceStore, ReferencePreferences};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let id = "0009-highscores-open";
    let mut preferences = ReferencePreferences::default();
    preferences.rebind(id, "ALT+H")?;
    preferences.rebind_secondary(id, "SUPER+SHIFT+H")?;
    preferences.select_hotkey_context(true)?;
    preferences.rebind(id, "SUPER+SHIFT+J")?;
    preferences.add_hotkey_profile("Hunt")?;
    preferences.rebind(id, "SUPER+SHIFT+K")?;
    preferences.select_hotkey_profile("Default")?;
    let press = |code, modifiers| -> Result<NormalizedInputEvent, Box<dyn std::error::Error>> {
        Ok(NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state: ButtonState::Pressed,
            modifiers,
            repeat: false,
        })
    };
    let alt_h = press(11, Modifiers::one(Modifier::Alt))?;
    let super_shift = Modifiers::one(Modifier::Super).with(Modifier::Shift);
    let secondary = press(11, super_shift)?;
    let chat_on = press(13, super_shift)?;
    let mut input = PreferenceInput::default();
    assert!(oteryn_client_ui::highscores::open_key(&input.route_chat(
        &preferences,
        std::slice::from_ref(&alt_h),
        false
    )?));
    input.route(&preferences, &[NormalizedInputEvent::DeviceLost])?;
    assert!(oteryn_client_ui::highscores::open_key(&input.route_chat(
        &preferences,
        &[secondary],
        false
    )?));
    input.route(&preferences, &[NormalizedInputEvent::DeviceLost])?;
    assert!(!oteryn_client_ui::highscores::open_key(&input.route_chat(
        &preferences,
        &[alt_h],
        true
    )?));
    input.route(&preferences, &[NormalizedInputEvent::DeviceLost])?;
    assert!(oteryn_client_ui::highscores::open_key(&input.route_chat(
        &preferences,
        &[chat_on],
        true
    )?));
    let unchanged = preferences.clone();
    assert!(
        preferences
            .rebind_secondary("local-spell-action-bar-spell-1", "SUPER+SHIFT+J")
            .is_err()
    );
    assert_eq!(preferences, unchanged);
    let path = std::env::temp_dir().join(format!("clientgap-profiles-{}.json", std::process::id()));
    let mut store = PreferenceStore::defaults(path.clone());
    store.apply(preferences.clone())?;
    assert_eq!(
        PreferenceStore::load(path.clone())?.preferences(),
        &preferences
    );
    let (link, _, _) = crate::play::play_channel();
    let mut settings = crate::play::SettingsState::new(None);
    settings.apply(preferences.clone());
    crate::play::mailbox::lock(&link.mailbox).chat.open = true;
    assert!(oteryn_client_ui::highscores::open_key(
        &settings
            .input_from_play(&[press(13, super_shift)?], &link)
            .0
    ));
    settings.input_from_play(&[NormalizedInputEvent::DeviceLost], &link);
    crate::play::mailbox::lock(&link.mailbox).chat.open = false;
    assert!(oteryn_client_ui::highscores::open_key(
        &settings
            .input_from_play(&[press(11, Modifiers::one(Modifier::Alt))?], &link)
            .0
    ));
    preferences.rename_hotkey_profile("Hunt", "Dungeon")?;
    preferences.select_hotkey_profile("Dungeon")?;
    preferences.remove_hotkey_profile("Default")?;
    assert!(preferences.remove_hotkey_profile("Dungeon").is_err());
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn admitted_character_preferences_are_isolated_and_reloaded()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::SettingsState;
    let root = std::env::temp_dir().join(format!("clientgap-scope-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    let path = root.join("preferences.json");
    let mut settings = SettingsState::new(Some(path.clone()));
    let global = settings.preferences.clone();
    let mut play = view()?;
    play.set_preference_character([1; 16]);
    settings.select_character(play.preference_character());
    assert_eq!(settings.scope_label(), "Character settings");
    let mut changed = settings.preferences.clone();
    changed.ui_scale = 120;
    changed.battle_lists[0].name = "Character one".into();
    settings.apply(changed.clone());
    play.set_preference_character([2; 16]);
    settings.select_character(play.preference_character());
    assert_eq!(settings.preferences, global);
    settings.preferences.ui_scale = 130;
    settings.apply(settings.preferences.clone());
    settings.select_character(Some([1; 16]));
    assert_eq!(settings.preferences, changed);
    let mut restarted = SettingsState::new(Some(path));
    restarted.select_character(Some([1; 16]));
    assert_eq!(restarted.preferences, changed);
    restarted.select_character(Some([2; 16]));
    assert_eq!(restarted.preferences.ui_scale, 130);
    restarted.select_character(None);
    assert_eq!(restarted.scope_label(), "Local defaults for new characters");
    assert_eq!(restarted.preferences, global);
    let blocked = root.join("blocked.json");
    std::fs::create_dir(&blocked)?;
    let mut recovered = SettingsState::new(Some(blocked.clone()));
    recovered.apply(changed.clone());
    assert!(recovered.feedback.contains("could not save"));
    std::fs::remove_dir(&blocked)?;
    recovered.reset_defaults()?;
    recovered.restore_backup()?;
    assert_eq!(recovered.preferences, changed);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

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

#[test]
fn help_navigation_opens_existing_report_without_submitting() {
    use crate::play::{HelpTopic, SettingsState, help_text};
    for topic in [HelpTopic::Questions, HelpTopic::Manual, HelpTopic::Client] {
        let text = help_text(topic);
        assert!(text.len() > 200);
        assert!(!text.contains("https://"));
    }
    let mut settings = SettingsState::new(None);
    settings.request_help_report();
    settings.open_help_report(None);
    assert!(settings.feedback.contains("Join a game session"));
    let (link, mut commands, _) = crate::play::play_channel();
    settings.open_help_report(Some(&link));
    assert!(!link.player_report_panel().open);
    settings.request_help_report();
    settings.open_help_report(Some(&link));
    assert!(link.player_report_panel().open);
    assert!(!link.player_report_panel().pending);
    assert!(commands.try_recv().is_err());
}

#[test]
fn general_controls_transform_real_inputs_and_preserve_releases()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{MousePreset, MovementKeys, RotationModifier, SettingsState};
    use oteryn_input_actions::{
        ButtonState, KeyCode, Modifier, Modifiers, MouseButton, NormalizedInputEvent,
    };
    use std::time::Duration;
    let mut settings = SettingsState::new(None);
    let mut prefs = settings.preferences.clone();
    prefs.controls.mouse = MousePreset::LeftHanded;
    prefs.controls.movement = MovementKeys::ArrowsAndWasd;
    prefs.controls.rotation = RotationModifier::Shift;
    prefs.controls.keyboard_delay_ms = 250;
    assert_eq!(prefs.controls.drag_button(true), MouseButton::SECONDARY);
    assert_eq!(prefs.controls.drag_button(false), MouseButton::PRIMARY);
    settings.apply(prefs.clone());
    let key = |code,
               modifiers,
               state,
               repeat|
     -> Result<NormalizedInputEvent, Box<dyn std::error::Error>> {
        Ok(NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            modifiers,
            state,
            repeat,
        })
    };
    let none = Modifiers::NONE;
    let w = key(26, none, ButtonState::Pressed, false)?;
    let mapped = settings.control_events_at(std::slice::from_ref(&w), false, Duration::ZERO);
    assert_eq!(
        crate::input::arrow_step(&mapped),
        Some(crate::input::StepDir::North)
    );
    let repeat = key(26, none, ButtonState::Pressed, true)?;
    assert!(
        settings
            .control_events_at(
                std::slice::from_ref(&repeat),
                false,
                Duration::from_millis(249)
            )
            .is_empty()
    );
    assert_eq!(
        settings
            .control_events_at(&[repeat], false, Duration::from_millis(250))
            .len(),
        1
    );
    prefs.controls.movement = MovementKeys::Arrows;
    settings.apply(prefs.clone());
    let released = key(26, none, ButtonState::Released, false)?;
    // A chat/context/settings change while held cannot strand the translated key.
    assert_eq!(
        settings.control_events_at(&[released], true, Duration::from_millis(251)),
        vec![key(82, none, ButtonState::Released, false)?]
    );
    assert_eq!(
        settings.control_events_at(std::slice::from_ref(&w), true, Duration::from_millis(260)),
        vec![w]
    );
    settings.control_events_at(
        &[NormalizedInputEvent::DeviceLost],
        false,
        Duration::from_millis(270),
    );
    let shift = Modifiers::one(Modifier::Shift);
    let rotation = settings.control_events_at(
        &[key(79, shift, ButtonState::Pressed, false)?],
        false,
        Duration::from_millis(300),
    );
    assert!(
        matches!(rotation.as_slice(),[NormalizedInputEvent::Key { code,modifiers,.. }] if *code == KeyCode::ARROW_RIGHT && *modifiers == Modifiers::one(Modifier::Control))
    );
    let mouse = |state| NormalizedInputEvent::MouseButton {
        button: MouseButton::PRIMARY,
        state,
        modifiers: none,
    };
    assert!(
        matches!(settings.control_events_at(&[mouse(ButtonState::Pressed)],false,Duration::ZERO).as_slice(),[NormalizedInputEvent::MouseButton { button,.. }] if *button == MouseButton::SECONDARY)
    );
    prefs.controls.mouse = MousePreset::Standard;
    settings.apply(prefs.clone());
    assert!(
        matches!(settings.control_events_at(&[mouse(ButtonState::Released)],false,Duration::ZERO).as_slice(),[NormalizedInputEvent::MouseButton { button,.. }] if *button == MouseButton::SECONDARY)
    );
    let good = settings.preferences.clone();
    prefs.controls.keyboard_delay_ms = 1001;
    settings.apply(prefs);
    assert_eq!(settings.preferences, good);
    let path = std::env::temp_dir().join(format!("clientgap-controls-{}.json", std::process::id()));
    crate::play::PreferenceStore::export_to(&good, &path)?;
    assert_eq!(
        crate::play::PreferenceStore::read_transfer(&path)?.controls,
        good.controls
    );
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn shortcut_order_persists_and_buttons_open_real_panels() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::{PreferenceStore, ReferencePreferences, SHORTCUTS};
    let mut prefs = ReferencePreferences::default();
    let panels = prefs.visible_panels.clone();
    let before = prefs.clone();
    assert!(prefs.add_shortcut("unregistered").is_err());
    assert!(prefs.add_shortcut("battle").is_err());
    assert!(prefs.move_shortcut(0, usize::MAX).is_err());
    assert_eq!(prefs, before);
    prefs.add_shortcut("quest-log")?;
    let last = prefs.shortcuts.len() - 1;
    prefs.move_shortcut(last, 0)?;
    assert_eq!(prefs.shortcuts[0], "quest-log");
    prefs.remove_shortcut(1)?;
    assert!(!prefs.shortcuts.iter().any(|id| id == "battle"));
    assert_eq!(prefs.visible_panels, panels);
    let path =
        std::env::temp_dir().join(format!("clientgap-shortcuts-{}.json", std::process::id()));
    let mut store = PreferenceStore::load(path.clone())?;
    store.apply(prefs.clone())?;
    assert_eq!(
        PreferenceStore::load(path.clone())?.preferences().shortcuts,
        prefs.shortcuts
    );
    std::fs::remove_file(path)?;
    let (link, _, _) = crate::play::play_channel();
    assert!(!link.battle_panel().open);
    assert!(link.activate_shortcut("battle"));
    assert!(link.battle_panel().open);
    assert!(link.activate_shortcut("battle"));
    assert!(!link.battle_panel().open);
    assert!(!link.activate_shortcut("unregistered"));
    for &(id, _) in SHORTCUTS {
        assert!(
            link.activate_shortcut(id),
            "shortcut {id} must reach its registered panel entry point"
        );
    }
    let good = prefs.clone();
    prefs.shortcuts.push("unknown".into());
    let mut settings = crate::play::SettingsState::new(None);
    settings.apply(good.clone());
    settings.apply(prefs);
    assert_eq!(settings.preferences, good);
    Ok(())
}

#[test]
fn actor_placement_does_not_select_a_character_preference_file()
-> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    play.set_own_identity([31; 16]);
    assert_eq!(
        play.preference_character(),
        None,
        "runtime placement is not a durable character id"
    );
    play.set_preference_character([7; 16]);
    play.set_own_identity([32; 16]);
    assert_eq!(play.preference_character(), Some([7; 16]));
    play = play.with_entities(vec![])?;
    assert_eq!(
        play.preference_character(),
        Some([7; 16]),
        "scene rebuilding retains the admitted character"
    );
    let root = std::env::temp_dir().join(format!("stable-character-scope-{}", std::process::id()));
    let path = root.join("preferences.json");
    let mut settings = crate::play::SettingsState::new(Some(path.clone()));
    settings.select_character(play.preference_character());
    let mut prefs = settings.preferences.clone();
    prefs.ui_scale = 140;
    settings.apply(prefs);
    let mut next_admission = view()?;
    next_admission.set_own_identity([33; 16]);
    next_admission.set_preference_character([7; 16]);
    let mut restarted = crate::play::SettingsState::new(Some(path));
    restarted.select_character(next_admission.preference_character());
    assert_eq!(restarted.preferences.ui_scale, 140);
    next_admission.set_preference_character([8; 16]);
    restarted.select_character(next_admission.preference_character());
    assert_eq!(restarted.preferences.ui_scale, 100);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn hud_arcs_project_server_vitals_and_persist_geometry_controls()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _, outbox) = crate::play::play_channel();
    let mut play = view()?;
    let mut prefs = play.preferences().clone();
    prefs.arcs = crate::play::ArcPreferences {
        health: true,
        mana: true,
        distance: 25,
        size: 60,
        opacity: 50,
    };
    play.set_preferences(prefs.clone())?;
    assert!(play.hud_arc_layout().is_none());
    outbox.vitals(Some(oteryn_session::ActorVitals {
        health: 50,
        max_health: 100,
        mana: 25,
        max_mana: 100,
        ..Default::default()
    }));
    play.tick(&link).map_err(|_| "tick")?;
    let layout = play.hud_arc_layout().ok_or("arc layout")?;
    assert_eq!(layout.alpha, 127);
    assert_eq!((layout.width, layout.height), (186, 136));
    assert_eq!(layout.strokes.len(), 2);
    assert_eq!(layout.strokes[0].filled_segments, 40);
    assert_eq!(layout.strokes[1].filled_segments, 20);
    assert!(
        layout
            .strokes
            .iter()
            .flat_map(|stroke| &stroke.points)
            .all(|p| p[0] >= 0 && p[1] >= 0 && p[0] <= layout.width && p[1] <= layout.height)
    );
    let path = std::env::temp_dir().join(format!("arc-preferences-{}.json", std::process::id()));
    let mut settings = crate::play::SettingsState::new(Some(path.clone()));
    settings.apply(prefs.clone());
    assert_eq!(
        crate::play::SettingsState::new(Some(path.clone()))
            .preferences
            .arcs,
        prefs.arcs
    );
    let mut invalid = prefs.clone();
    invalid.arcs.opacity = 101;
    settings.apply(invalid);
    assert_eq!(settings.preferences, prefs);
    prefs.arcs.health = false;
    prefs.arcs.mana = false;
    play.set_preferences(prefs)?;
    assert!(play.hud_arc_layout().is_none());
    std::fs::remove_file(path)?;
    Ok(())
}
