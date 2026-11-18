use super::*;
use bevy::ui_widgets::{Activate, ValueChange};
type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn app(
    page: Page,
    path: Option<std::path::PathBuf>,
) -> Result<App, Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(path)?;
    owner.open_graphics();
    let generation = owner.generation();
    let mut app = App::new();
    app.insert_non_send(owner).insert_resource(OptionsState {
        page,
        advanced: true,
        generation: Some(generation),
        ..default()
    });
    Ok(app)
}
#[test]
fn native_checkbox_edits_only_the_draft_and_cancel_discards_it() -> TestResult {
    let mut app = app(Page::Console, None)?;
    let checkbox = app
        .world_mut()
        .spawn(Control::Check("console.local"))
        .observe(toggle)
        .id();
    app.world_mut().trigger(ValueChange {
        source: checkbox,
        value: false,
        is_final: true,
    });
    let owner = app.world().non_send::<HostOwner>();
    assert!(!owner.preferences_draft().ok_or("draft")?.console.visible[0]);
    assert!(owner.settings.preferences.console.visible[0]);
    let cancel = app
        .world_mut()
        .spawn(Control::Cancel)
        .observe(activate)
        .id();
    app.world_mut().trigger(Activate { entity: cancel });
    let owner = app.world().non_send::<HostOwner>();
    assert!(owner.preferences_draft().is_none());
    assert!(owner.settings.preferences.console.visible[0]);
    Ok(())
}
#[test]
fn native_slider_apply_reaches_the_existing_persistence_owner() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("bevy-options.json");
    let result = (|| -> TestResult {
        let mut app = app(Page::Sound, Some(path.clone()))?;
        let slider = app
            .world_mut()
            .spawn(Control::Number("sound.master"))
            .observe(slide)
            .id();
        app.world_mut().trigger(ValueChange {
            source: slider,
            value: 37.0f32,
            is_final: true,
        });
        let owner = app.world().non_send::<HostOwner>();
        assert_eq!(
            owner
                .preferences_draft()
                .ok_or("draft")?
                .audio
                .master_volume,
            37
        );
        assert_eq!(owner.settings.preferences.audio.master_volume, 100);
        let apply = app.world_mut().spawn(Control::Apply).observe(activate).id();
        app.world_mut().trigger(Activate { entity: apply });
        let owner = app.world().non_send::<HostOwner>();
        assert_eq!(owner.settings.preferences.audio.master_volume, 37);
        assert_eq!(
            owner
                .preferences_draft()
                .ok_or("renewed draft")?
                .audio
                .master_volume,
            37
        );
        assert_eq!(owner.settings.feedback, "Applied and saved");
        let restored = crate::play::SettingsState::new(Some(path.clone()));
        assert_eq!(restored.preferences.audio.master_volume, 37);
        Ok(())
    })();
    result
}
#[test]
fn choice_menu_updates_normalized_mouse_controls_and_page_reset_is_scoped() -> TestResult {
    let mut app = app(Page::Controls, None)?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .edit_preferences(|p| p.graphics.fullscreen = true);
    let choice = app
        .world_mut()
        .spawn(Control::Select("controls.mouse", 1))
        .observe(activate)
        .id();
    app.world_mut().trigger(Activate { entity: choice });
    let owner = app.world().non_send::<HostOwner>();
    let draft = owner.preferences_draft().ok_or("draft")?;
    assert_eq!(draft.controls.mouse, crate::play::MousePreset::LeftHanded);
    assert!(
        !draft.swap_mouse_buttons,
        "typed and legacy mappings must not invert twice"
    );
    let reset = app.world_mut().spawn(Control::Reset).observe(activate).id();
    app.world_mut().trigger(Activate { entity: reset });
    let owner = app.world().non_send::<HostOwner>();
    let draft = owner.preferences_draft().ok_or("draft")?;
    assert_eq!(draft.controls.mouse, crate::play::MousePreset::Standard);
    assert!(
        draft.graphics.fullscreen,
        "reset affects the selected page only"
    );
    Ok(())
}
#[test]
fn closed_or_stale_options_controls_do_not_write_into_the_current_owner() -> TestResult {
    let mut app = app(Page::Sound, None)?;
    let generation = app.world().non_send::<HostOwner>().generation();
    app.world_mut().resource_mut::<OptionsState>().generation = Some(generation.saturating_add(1));
    let slider = app
        .world_mut()
        .spawn(Control::Number("sound.master"))
        .observe(slide)
        .id();
    app.world_mut().trigger(ValueChange {
        source: slider,
        value: 0.0f32,
        is_final: true,
    });
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .preferences_draft()
            .ok_or("draft")?
            .audio
            .master_volume,
        100
    );
    app.world_mut().resource_mut::<OptionsState>().generation = Some(generation);
    app.world_mut().non_send_mut::<HostOwner>().cancel();
    app.world_mut().trigger(ValueChange {
        source: slider,
        value: 0.0f32,
        is_final: true,
    });
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .audio
            .master_volume,
        100
    );
    Ok(())
}
#[test]
fn supported_field_ranges_round_trip_without_truncation_and_ids_are_unique() {
    let mut ids = std::collections::BTreeSet::new();
    for page in Page::ADVANCED {
        for row in metadata::rows(page) {
            assert!(ids.insert(row.id), "duplicate editable field {}", row.id);
            let mut p = crate::play::ReferencePreferences::default();
            match row.editor {
                Editor::Number { min, max, .. } => {
                    assert!(row.set(&mut p, Value::Number(max)));
                    assert_eq!(row.value(&p), Value::Number(max), "{}", row.id);
                    assert!(row.set(&mut p, Value::Number(min)));
                    assert_eq!(row.value(&p), Value::Number(min), "{}", row.id);
                    assert!(!row.set(&mut p, Value::Number(max.saturating_add(1))));
                }
                Editor::Choice { values, .. } => {
                    for index in 0..values.len() {
                        assert!(row.set(&mut p, Value::Choice(index)));
                        assert_eq!(row.value(&p), Value::Choice(index));
                    }
                    assert!(!row.set(&mut p, Value::Choice(values.len())));
                }
                Editor::Check { .. } => {
                    assert!(row.set(&mut p, Value::Check(false)));
                    assert_eq!(row.value(&p), Value::Check(false));
                }
            }
        }
    }
}

#[test]
fn foreground_game_modal_blocks_background_options_value_changes() -> TestResult {
    let mut app = app(Page::Sound, None)?;
    app.world_mut()
        .spawn((GameModal, GlobalZIndex(200), Node::default()));
    let slider = app
        .world_mut()
        .spawn(Control::Number("sound.master"))
        .observe(slide)
        .id();
    app.world_mut().trigger(ValueChange {
        source: slider,
        value: 0.0f32,
        is_final: true,
    });
    let owner = app.world().non_send::<HostOwner>();
    assert_eq!(
        owner
            .preferences_draft()
            .ok_or("draft")?
            .audio
            .master_volume,
        100
    );
    assert_eq!(owner.settings.preferences.audio.master_volume, 100);
    Ok(())
}

#[test]
fn active_options_escape_does_not_require_a_focused_child() -> TestResult {
    use bevy::input::keyboard::{Key, KeyboardInput};
    let mut app = app(Page::Sound, None)?;
    app.init_resource::<InputFocus>()
        .add_message::<KeyboardInput>()
        .add_systems(Update, keyboard);
    app.world_mut()
        .spawn((OptionsDialog, GameModal, GlobalZIndex(100), Node::default()));
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: Key::Escape,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .preferences_draft()
            .is_none()
    );
    Ok(())
}

#[test]
fn ask_before_exit_checkbox_round_trips_through_the_existing_settings_owner() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("preferences.json");
    let mut app = app(Page::Misc, Some(path.clone()))?;
    let checkbox = app
        .world_mut()
        .spawn(Control::Check("misc.exit"))
        .observe(toggle)
        .id();
    app.world_mut().trigger(ValueChange {
        source: checkbox,
        value: false,
        is_final: true,
    });
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .preferences_draft()
            .ok_or("draft")?
            .miscellaneous
            .confirm_exit
    );
    let ok = app.world_mut().spawn(Control::Ok).observe(activate).id();
    app.world_mut().trigger(Activate { entity: ok });
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .preferences_draft()
            .is_none()
    );
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .miscellaneous
            .confirm_exit
    );
    assert!(
        !crate::play::SettingsState::new(Some(path))
            .preferences
            .miscellaneous
            .confirm_exit
    );
    Ok(())
}

#[test]
fn remember_device_checkbox_keeps_the_session_and_persists_the_login_choices() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("preferences.json");
    let mut app = app(Page::Misc, Some(path.clone()))?;
    let set = |app: &mut App, id: &'static str, value: bool| {
        let checkbox = app
            .world_mut()
            .spawn(Control::Check(id))
            .observe(toggle)
            .id();
        app.world_mut().trigger(ValueChange {
            source: checkbox,
            value,
            is_final: true,
        });
    };
    set(&mut app, "misc.remember_device", true);
    set(&mut app, "misc.quick_login", true);
    set(&mut app, "misc.stay_logged_in", false);
    let draft = app
        .world()
        .non_send::<HostOwner>()
        .preferences_draft()
        .ok_or("draft")?
        .miscellaneous;
    assert!(draft.remember_device && draft.stay_logged_in && draft.quick_login);
    let ok = app.world_mut().spawn(Control::Ok).observe(activate).id();
    app.world_mut().trigger(Activate { entity: ok });
    let saved = crate::play::SettingsState::new(Some(path.clone()))
        .preferences
        .miscellaneous;
    assert!(saved.remember_device && saved.stay_logged_in && saved.quick_login);
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    set(&mut app, "misc.remember_device", false);
    set(&mut app, "misc.stay_logged_in", false);
    let ok = app.world_mut().spawn(Control::Ok).observe(activate).id();
    app.world_mut().trigger(Activate { entity: ok });
    let saved = crate::play::SettingsState::new(Some(path))
        .preferences
        .miscellaneous;
    assert!(!saved.remember_device && !saved.stay_logged_in && saved.quick_login);
    Ok(())
}
#[test]
fn page_reset_restores_the_section_editors_of_its_page() -> TestResult {
    let mut app = app(Page::GeneralHotkeys, None)?;
    let defaults = crate::play::ReferencePreferences::default();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .edit_preferences(|p| {
            p.keybindings
                .insert("0165-hireling-open".into(), "CTRL+ALT+SHIFT+F11".into());
            p.secondary_keybindings
                .insert("0165-hireling-open".into(), "CTRL+ALT+SHIFT+F10".into());
            p.action_keys.0.insert(35, "CTRL+ALT+SHIFT+F9".into());
            p.hotkey_contexts.chat_on = !p.hotkey_contexts.chat_on;
            p.audio_device = "Headset".into();
            p.condition_hud_hidden = vec!["Soul War".into()];
            p.condition_status_hidden = vec!["Resting Area".into()];
            p.move_special_condition(0, true);
            p.move_status_entry(true, 0, true);
            p.set_language(oteryn_client_ui::localization::Language::Sv);
            p.resource_hud.soul = false;
        });
    let reset = app.world_mut().spawn(Control::Reset).observe(activate).id();
    for page in [
        Page::GeneralHotkeys,
        Page::Sound,
        Page::Hud,
        Page::Interface,
    ] {
        app.world_mut().resource_mut::<OptionsState>().page = page;
        let revision = app.world().resource::<OptionsState>().revision;
        app.world_mut().trigger(Activate { entity: reset });
        assert_ne!(
            app.world().resource::<OptionsState>().revision,
            revision,
            "{page:?} rebuilds its sections"
        );
    }
    let owner = app.world().non_send::<HostOwner>();
    let draft = owner.preferences_draft().ok_or("draft")?;
    assert_eq!(draft.keybindings, defaults.keybindings);
    assert_eq!(draft.secondary_keybindings, defaults.secondary_keybindings);
    assert_eq!(draft.action_keys, defaults.action_keys);
    assert_eq!(draft.hotkey_contexts, defaults.hotkey_contexts);
    assert_eq!(draft.audio_device, defaults.audio_device);
    assert_eq!(draft.condition_hud_hidden, defaults.condition_hud_hidden);
    assert_eq!(
        draft.condition_status_hidden,
        defaults.condition_status_hidden
    );
    assert_eq!(
        draft.ordered_condition_keys(),
        defaults.ordered_condition_keys()
    );
    assert_eq!(draft.skill_order, defaults.skill_order);
    assert_eq!(draft.language(), defaults.language());
    assert!(draft.resource_hud.soul);
    Ok(())
}
#[test]
fn hud_resource_toggles_edit_apply_and_save_each_display() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("bevy-options-resource-hud.json");
    let result = (|| -> TestResult {
        let mut app = app(Page::Hud, Some(path.clone()))?;
        for (id, value) in [
            ("hud.resource_values", false),
            ("hud.resource_soul", false),
            ("hud.resource_stamina", false),
            ("hud.resource_conditions", false),
            ("hud.resource_rest", false),
            ("hud.resource_combo", true),
        ] {
            let checkbox = app
                .world_mut()
                .spawn(Control::Check(id))
                .observe(toggle)
                .id();
            app.world_mut().trigger(ValueChange {
                source: checkbox,
                value,
                is_final: true,
            });
        }
        app.world_mut().non_send_mut::<HostOwner>().apply(true);
        let stored = crate::play::SettingsState::new(Some(path.clone())).preferences;
        assert_eq!(
            stored.resource_hud,
            crate::play::ResourceHudOptions {
                combo: true,
                values: false,
                soul: false,
                stamina: false,
                conditions: false,
                rest: false,
            }
        );
        Ok(())
    })();
    result
}

#[test]
fn action_bar_options_show_one_bar_at_a_time() -> TestResult {
    let mut app = app(Page::ActionBars, None)?;
    let checkbox = app
        .world_mut()
        .spawn(Control::Check("action_bars.left_1"))
        .observe(toggle)
        .id();
    app.world_mut().trigger(ValueChange {
        source: checkbox,
        value: true,
        is_final: true,
    });
    let ok = app.world_mut().spawn(Control::Ok).observe(activate).id();
    app.world_mut().trigger(Activate { entity: ok });
    let (link, _, _) = crate::play::play_channel();
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    owner.settings.sync_bar_actions(&link);
    let enabled = link.bar_layout().rows.map(|row| row.enabled);
    assert_eq!(
        enabled,
        [true, true, false, false, false, false, false, false, false],
        "only the first left bar joins the default bottom bar"
    );
    let rows = metadata::rows(Page::ActionBars);
    for (id, index) in [
        ("action_bars.bottom_1", 0),
        ("action_bars.bottom_2", 3),
        ("action_bars.bottom_3", 4),
        ("action_bars.left_2", 5),
        ("action_bars.left_3", 6),
        ("action_bars.right_1", 2),
        ("action_bars.right_2", 7),
        ("action_bars.right_3", 8),
    ] {
        let row = rows.iter().find(|row| row.id == id).ok_or(id)?;
        let mut p = owner.settings.preferences.clone();
        assert!(row.set(&mut p, Value::Check(!enabled[index])));
        let changed: Vec<_> = (0..9)
            .filter(|&i| p.bar_layout.rows[i].enabled != enabled[i])
            .collect();
        assert_eq!(changed, vec![index], "{id} toggles exactly one bar");
    }
    for (id, side) in [
        ("action_bars.bottom_all", [0, 3, 4]),
        ("action_bars.left_all", [1, 5, 6]),
        ("action_bars.right_all", [2, 7, 8]),
    ] {
        let row = rows.iter().find(|row| row.id == id).ok_or(id)?;
        let mut p = owner.settings.preferences.clone();
        assert!(row.set(&mut p, Value::Check(true)));
        assert_eq!(row.value(&p), Value::Check(true));
        let on: Vec<_> = (0..9).filter(|&i| p.bar_layout.rows[i].enabled).collect();
        let mut expected: Vec<usize> = side.to_vec();
        expected.extend([0, 1].into_iter().filter(|i| !side.contains(i)));
        expected.sort_unstable();
        assert_eq!(on, expected, "{id} shows the whole side");
        assert!(row.set(&mut p, Value::Check(false)));
        assert!(side.iter().all(|&i| !p.bar_layout.rows[i].enabled));
    }
    Ok(())
}
