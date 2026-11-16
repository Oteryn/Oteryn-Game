use super::*;
use bevy::ui_widgets::{Activate, ValueChange};
use std::sync::atomic::{AtomicU64, Ordering};
type TestResult = Result<(), Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);

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
    let path = std::env::temp_dir().join(format!(
        "bevy-options-{}-{}.json",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
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
    let _ = std::fs::remove_file(&path);
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
    struct Directory(std::path::PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let directory = Directory(std::env::temp_dir().join(format!(
        "bevy-options-exit-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    std::fs::create_dir_all(&directory.0)?;
    let path = directory.0.join("preferences.json");
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
    struct Directory(std::path::PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let directory = Directory(std::env::temp_dir().join(format!(
        "bevy-options-remember-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )));
    std::fs::create_dir_all(&directory.0)?;
    let path = directory.0.join("preferences.json");
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
    let path = std::env::temp_dir().join(format!(
        "bevy-options-resource-hud-{}.json",
        std::process::id()
    ));
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
    let _ = std::fs::remove_file(&path);
    result
}
