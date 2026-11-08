use super::*;

#[test]
fn configured_rotation_modifier_sends_existing_turn_command()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (link, mut commands, outbox) = play_channel();
    outbox.target_move_initialize(true);
    let mut view = view()?;
    let mut preferences = view.preferences().clone();
    preferences.rotation_modifier = "ALT".into();
    view.set_preferences(preferences)?;
    assert!(view.target_move_input(
        &link,
        &[NormalizedInputEvent::Key {
            code: KeyCode::ARROW_UP,
            modifiers: Modifiers::one(Modifier::Alt),
            state: ButtonState::Pressed,
            repeat: false,
        }]
    ));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("missing turn command".into());
    };
    assert_eq!(command.payload, [26, 2, 8, 1]);
    Ok(())
}

#[test]
fn mouse_preset_changes_actual_click_routing() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::SettingsState;
    use oteryn_input_actions::{ButtonState, Modifiers, MouseButton, NormalizedInputEvent};
    let mut settings = SettingsState::new(None);
    settings.preferences.swap_mouse_buttons = true;
    let press = NormalizedInputEvent::MouseButton {
        button: MouseButton::SECONDARY,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
    };
    let (events, _) = settings.input(&[press]);
    assert!(matches!(
        events.as_slice(),
        [NormalizedInputEvent::MouseButton {
            button: MouseButton::PRIMARY,
            ..
        }]
    ));
    settings.preferences.keyboard_delay_ms = 1000;
    let key = NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(82)?,
        modifiers: Modifiers::NONE,
        state: ButtonState::Pressed,
        repeat: false,
    };
    assert_eq!(settings.input(std::slice::from_ref(&key)).0.len(), 1);
    let mut repeat = key.clone();
    if let NormalizedInputEvent::Key { repeat, .. } = &mut repeat {
        *repeat = true;
    }
    assert!(settings.input(&[repeat]).0.is_empty());
    settings.input(&[NormalizedInputEvent::FocusChanged { focused: false }]);
    assert_eq!(settings.input(&[key]).0.len(), 1);
    settings.preferences.keyboard_delay_ms = 1001;
    assert!(view()?.set_preferences(settings.preferences).is_err());
    Ok(())
}

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

include!("play_settings_transfer_backup_and_defaults_survive_restart_1.rs");

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

include!("play_settings_rebind_conflicts_routes_and_survives_restart_1.rs");

include!(
    "play_every_registered_binding_conflicts_and_invalid_saves_leave_preferences_unchanged_1.rs"
);

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
fn t162_monk_hud_has_independent_visibility() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    play.vitals = Some(oteryn_session::ActorVitals {
        harmony: 3,
        serene: true,
        ..Default::default()
    });
    play.rebuild()?;
    let shown = play.scene().sprites().len();
    let mut json = serde_json::to_value(play.preferences())?;
    json["show_harmony"] = serde_json::json!(false);
    play.set_preferences(serde_json::from_value(json.clone())?)?;
    assert_eq!(play.scene().sprites().len() + 5, shown);
    json["show_serene"] = serde_json::json!(false);
    play.set_preferences(serde_json::from_value(json)?)?;
    assert_eq!(play.scene().sprites().len() + 6, shown);
    Ok(())
}

#[test]
fn o010_menu_uses_live_window_inputs_after_rebinding() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = crate::play::SettingsState::new(None);
    state
        .preferences
        .rebind("0287-actor-spell-catalog-open", "Alt+U")?;
    assert!(state.queue_window("0287-actor-spell-catalog-open"));
    assert!(!state.queue_window("unknown"));
    let raw = state.take_window_events();
    let (events, _) = state.input(&raw);
    let (link, _, _) = crate::play::play_channel();
    assert!(link.spell_catalog_input(&events));
    assert!(link.spell_catalog_panel().open);
    assert!(state.take_window_events().is_empty());
    Ok(())
}

include!("play_o021_live_health_and_party_marks_project_at_entity_position_1.rs");

#[test]
fn o025_health_circle_tracks_confirmed_resources_and_hides_without_vitals()
-> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    let baseline = play.scene().sprites().len();
    let mut json = serde_json::to_value(play.preferences())?;
    json["health_circle"] = serde_json::json!(true);
    play.set_preferences(serde_json::from_value(json)?)?;
    assert_eq!(play.scene().sprites().len(), baseline);
    play.vitals = Some(oteryn_session::ActorVitals {
        health: 50,
        max_health: 100,
        mana: 0,
        max_mana: 0,
        ..Default::default()
    });
    play.rebuild()?;
    // 32 circle segments, four HP/MP quads and six monk indicators.
    assert_eq!(play.scene().sprites().len(), baseline + 42);
    let circle = &play.scene().sprites().instances()[baseline + 10..];
    assert_eq!(circle.len(), 32);
    assert!(circle.iter().any(|segment| segment.uv != circle[31].uv));
    Ok(())
}

#[test]
fn o037_shortcut_panel_collapse_and_order_survive_restart() -> Result<(), Box<dyn std::error::Error>>
{
    let path = std::env::temp_dir().join(format!("cg-mainpanel-{}.json", std::process::id()));
    let mut state = crate::play::SettingsState::new(Some(path.clone()));
    let mut json = serde_json::to_value(&state.preferences)?;
    json["shortcuts_collapsed"] = serde_json::json!(true);
    json["shortcut_order"] =
        serde_json::json!(["0287-actor-spell-catalog-open", "0279-skill-progress-open"]);
    state.apply(serde_json::from_value(json.clone())?);
    let restored = crate::play::SettingsState::new(Some(path.clone()));
    assert_eq!(
        serde_json::to_value(&restored.preferences)?["shortcut_order"],
        json["shortcut_order"]
    );
    assert_eq!(
        serde_json::to_value(&restored.preferences)?["shortcuts_collapsed"],
        true
    );
    json["shortcut_order"] = serde_json::json!(["unknown"]);
    assert!(
        view()?
            .set_preferences(serde_json::from_value(json)?)
            .is_err()
    );
    std::fs::remove_file(path)?;
    Ok(())
}

include!("health_circle.rs");
