use super::*;

#[test]
fn mouse_presets_validate_route_and_survive_restart() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PreferenceStore, SettingsState};
    use oteryn_input_actions::{ButtonState, Modifiers, MouseButton, NormalizedInputEvent};
    let path = std::env::temp_dir().join(format!("cgopt-mouse-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    let mut preferences = settings.preferences.clone();
    preferences.set_mouse_preset(1)?;
    preferences.keyboard_delay_ms = 250;
    preferences.rotation_modifier = "SHIFT".into();
    settings.apply(preferences);
    let mut restored = SettingsState::new(Some(path.clone()));
    assert_eq!(
        PreferenceStore::load(path.clone())?.preferences(),
        &settings.preferences
    );
    let press = NormalizedInputEvent::MouseButton {
        button: MouseButton::SECONDARY,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
    };
    assert!(matches!(
        restored.input(std::slice::from_ref(&press)).0.as_slice(),
        [NormalizedInputEvent::MouseButton {
            button: MouseButton::PRIMARY,
            ..
        }]
    ));
    restored.preferences.set_mouse_preset(0)?;
    assert_eq!(restored.input(std::slice::from_ref(&press)).0, vec![press]);
    let before = restored.preferences.clone();
    assert!(restored.preferences.set_mouse_preset(2).is_err());
    assert_eq!(restored.preferences, before);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn basic_options_dispatch_selects_all_categories_without_editing_preferences() {
    use crate::play::{SettingsPage, SettingsState};
    let mut state = SettingsState::new(None);
    assert!(state.request_open());
    let before = state.preferences.clone();
    for (index, page) in SettingsPage::ALL.iter().enumerate() {
        assert!(state.select_page(index));
        assert_eq!(state.page, *page);
        assert_eq!(state.preferences, before);
    }
    let last = state.page;
    assert!(!state.select_page(SettingsPage::ALL.len()));
    assert_eq!(state.page, last);
    for category in [
        SettingsPage::Gameplay,
        SettingsPage::Interface,
        SettingsPage::Graphics,
        SettingsPage::Sound,
    ] {
        assert!(SettingsPage::ALL.contains(&category));
    }
}

#[test]
fn console_font_preference_reaches_the_live_panel_mailbox() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, _, _) = play_channel();
    let mut settings = crate::play::SettingsState::new(None);
    let mut preferences = settings.preferences.clone();
    preferences.console_font_px = 20;
    settings.apply(preferences);
    settings.sync_local_preferences(&link);
    assert_eq!(
        crate::play::mailbox::lock(&link.mailbox)
            .local_preferences
            .console_font_px,
        20
    );
    let mut invalid = settings.preferences.clone();
    invalid.console_font_px = 33;
    assert!(view()?.set_preferences(invalid).is_err());
    Ok(())
}

#[test]
fn compact_status_style_projects_confirmed_vitals_only() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    play.vitals = Some(oteryn_session::ActorVitals {
        health: 50,
        max_health: 100,
        mana: 20,
        max_mana: 40,
        ..Default::default()
    });
    let mut preferences = play.preferences().clone();
    preferences.compact_status = true;
    play.set_preferences(preferences)?;
    assert_eq!(
        play.hud_status(),
        oteryn_client_ui::spell_action_bar::vitals_text(play.actor_vitals())
    );
    Ok(())
}

#[test]
fn hud_arc_geometry_uses_configured_distance_and_size() -> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    play.vitals = Some(oteryn_session::ActorVitals {
        health: 50,
        max_health: 100,
        mana: 20,
        max_mana: 40,
        ..Default::default()
    });
    let mut preferences = play.preferences().clone();
    preferences.health_circle = true;
    preferences.arc_distance = 14;
    preferences.arc_size = 5;
    play.set_preferences(preferences)?;
    let quads = play.scene().sprites().instances();
    assert!(
        quads[quads.len() - 32..]
            .iter()
            .all(|quad| quad.size == [5.0, 5.0])
    );
    let mut invalid = play.preferences().clone();
    invalid.arc_size = 0;
    assert!(play.set_preferences(invalid).is_err());
    Ok(())
}

#[test]
fn hud_name_controls_separate_owner_players_and_creatures() -> Result<(), Box<dyn std::error::Error>>
{
    use oteryn_session::{
        EntityDetail, EntityDisplayName, EntityKind, EntityRef, VisibleAppearance,
        WorldSpatialEntity,
    };
    let mut play = view()?;
    play.set_own_identity([1; 16]);
    let entries = [
        (1, EntityKind::Player),
        (2, EntityKind::Player),
        (3, EntityKind::Creature),
    ]
    .into_iter()
    .map(|(id, kind)| {
        Ok(VisibleAppearance {
            entity: WorldSpatialEntity {
                entity: EntityRef {
                    identity: [id; 16],
                    generation: 1,
                },
                kind,
                position: ActorPosition {
                    x: 100 + i32::from(id),
                    y: 200,
                    floor: 0,
                },
                display_name: Some(EntityDisplayName::new("Actor")?),
                detail: EntityDetail::Actor {
                    direction: StepDirection::South,
                    appearance_ref: 1,
                    health_percent: 100,
                },
            },
            look: None,
            light: None,
            forge_stacks: None,
        })
    })
    .collect::<Result<Vec<_>, oteryn_session::SessionError>>()?;
    play.set_presentations(entries)?;
    assert_eq!(play.scene().name_count(), 3);
    let mut preferences = play.preferences().clone();
    preferences.hud_names = [false, true, false];
    play.set_preferences(preferences)?;
    assert_eq!(play.scene().name_count(), 1);
    Ok(())
}

#[test]
fn interface_scale_changes_hud_geometry_and_rejects_invalid_values()
-> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    let mut preferences = play.preferences().clone();
    preferences.interface_scale = 150;
    preferences.cursor_crosshair = true;
    play.set_preferences(preferences)?;
    assert_eq!(play.preferences().scaled_hud_bounds(), [12, 12, 1350, 60]);
    let mut invalid = play.preferences().clone();
    invalid.interface_scale = 201;
    assert!(play.set_preferences(invalid).is_err());
    Ok(())
}

#[test]
fn hotkey_profiles_preserve_routes_across_restart() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{PreferenceStore, ReferencePreferences};
    let path = std::env::temp_dir().join(format!("cgopt-profiles-{}.json", std::process::id()));
    let mut preferences = ReferencePreferences::default();
    preferences.save_hotkey_profile("arrows")?;
    preferences.general_hotkey_preset(false)?;
    preferences.save_hotkey_profile("wasd")?;
    let mut store = PreferenceStore::load(path.clone())?;
    store.apply(preferences)?;
    let mut restored = PreferenceStore::load(path.clone())?.preferences().clone();
    restored.load_hotkey_profile("arrows")?;
    assert!(
        !restored
            .keybindings
            .contains_key("0013-paced-movement-move-north")
    );
    restored.load_hotkey_profile("wasd")?;
    assert_eq!(
        restored
            .keybindings
            .get("0013-paced-movement-move-north")
            .map(String::as_str),
        Some("W")
    );
    let before = restored.clone();
    assert!(restored.load_hotkey_profile("missing").is_err());
    assert!(restored.save_hotkey_profile("").is_err());
    assert_eq!(restored, before);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn action_bar_hotkeys_route_every_supported_slot() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::ReferencePreferences;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let mut preferences = ReferencePreferences::default();
    let chords = ["F5", "F6", "F7", "F8"];
    preferences.set_action_bar_hotkeys(&chords)?;
    let mut router = crate::spell::SpellHotkeys::new()?;
    for slot in 0..4 {
        let event = NormalizedInputEvent::Key {
            code: KeyCode::new(62 + slot)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        let routed = preferences.route_input(&[event])?;
        assert_eq!(
            router.route(&routed).first().map(|slot| slot.get()),
            Some(u32::from(slot + 1))
        );
        let release = NormalizedInputEvent::Key {
            code: KeyCode::new(62 + slot)?,
            state: ButtonState::Released,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        router.route(&preferences.route_input(&[release])?);
    }
    let before = preferences.clone();
    assert!(
        preferences
            .set_action_bar_hotkeys(&["F5", "F5", "F7", "F8"])
            .is_err()
    );
    assert_eq!(preferences, before);
    Ok(())
}

#[test]
fn general_hotkey_presets_route_movement_and_preserve_other_bindings()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{ReferencePreferences, SettingsState};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let mut preferences = ReferencePreferences::default();
    preferences.rebind("0009-highscores-open", "ALT+H")?;
    preferences.general_hotkey_preset(false)?;
    let mut state = SettingsState::new(None);
    state.preferences = preferences;
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(26)?,
        modifiers: Modifiers::NONE,
        state: ButtonState::Pressed,
        repeat: false,
    };
    assert!(matches!(
        state.input(&[event]).0.as_slice(),
        [NormalizedInputEvent::Key {
            code: KeyCode::ARROW_UP,
            ..
        }]
    ));
    state.preferences.general_hotkey_preset(true)?;
    assert_eq!(
        state
            .preferences
            .keybindings
            .get("0009-highscores-open")
            .map(String::as_str),
        Some("ALT+H")
    );
    assert!(
        !state
            .preferences
            .keybindings
            .contains_key("0013-paced-movement-move-north")
    );
    Ok(())
}

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
    assert_eq!(SettingsPage::ALL.len(), 18);
    for added in [
        SettingsPage::Screenshots,
        SettingsPage::HelpGuide,
        SettingsPage::MinimapTransfer,
        SettingsPage::ConsoleFilters,
        SettingsPage::ActionBarLayout,
        SettingsPage::Language,
    ] {
        assert!(SettingsPage::ALL.contains(&added));
    }
}
