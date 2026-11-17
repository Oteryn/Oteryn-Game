//! C25: local preferences -> production input -> feature window state, across restart.
use oteryn_client::play::{PlayView, PreferenceStore, SettingsState, play_channel};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_renderer::TileCoord;

#[test]
fn native_settings_restart_rebind_and_visible_play_scenario()
-> Result<(), Box<dyn std::error::Error>> {
    let path =
        std::env::temp_dir().join(format!("oteryn-c25-scenario-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let store = PreferenceStore::load(path.clone())?;
    let settings_key = NormalizedInputEvent::Key {
        code: KeyCode::new(18)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
        repeat: false,
    };
    let mut settings = SettingsState::new(Some(path.clone()));
    let (routed, requested) = settings.input(&[settings_key]);
    assert!(routed.is_empty());
    assert!(requested && settings.open);
    assert!(!settings.request_open());
    let mut preferences = store.preferences().clone();
    assert!(
        preferences
            .rebind("0009-highscores-open", "CTRL+Q")
            .is_err()
    );
    preferences.rebind("0009-highscores-open", "ALT+H")?;
    preferences.ui_scale = 150;
    preferences.audio.master_volume = 35;
    preferences.audio.muted = true;
    settings.apply(preferences);
    assert_eq!(settings.feedback, "Applied and saved");
    drop(settings);
    drop(store);
    let restarted = PreferenceStore::load(path.clone())?;
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    let mut settings = SettingsState::new(Some(path.clone()));
    settings.restore(&mut view, [960, 540])?;
    assert_eq!(view.preferences(), restarted.preferences());
    let (link, _commands, _outbox) = play_channel();
    let (routed, requested) = settings.input(&[NormalizedInputEvent::Key {
        code: KeyCode::new(11)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Alt),
        repeat: false,
    }]);
    assert!(!requested);
    assert!(link.highscores_input(&routed));
    assert!(link.highscores_panel().open);
    assert_eq!(view.scene().view().tile_px(), 49);
    assert_eq!(view.preferences().audio.master_volume, 35);
    assert!(view.preferences().audio.muted);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn settings_sync_keeps_audio_without_storage_and_merges_window_edits()
-> Result<(), Box<dyn std::error::Error>> {
    let mut state = SettingsState::new(None);
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    state.restore(&mut view, [600, 330])?;
    assert_eq!(view.scene().view().tile_px(), 30);
    for code in [67, 68, 69] {
        let (events, requested) = state.input(&[NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        }]);
        assert!(!requested);
        assert!(view.audio_input(&events));
        // A concurrent settings-window edit must survive the audio capture.
        let mut preferences = state.preferences.clone();
        preferences.rebind("0009-highscores-open", "ALT+H")?;
        state.apply(preferences);
        let changed_audio = view.preferences().audio;
        state.capture(&mut view)?;
        state.restore(&mut view, [600, 330])?;
        assert_eq!(view.preferences().audio, changed_audio);
        assert_eq!(
            view.preferences()
                .keybindings
                .get("0009-highscores-open")
                .map(String::as_str),
            Some("ALT+H")
        );
        assert!(state.feedback.contains("could not save"));
    }
    assert!(view.preferences().audio.muted);
    let mut preferences = state.preferences.clone();
    preferences.rebind("local-reference-preferences-open", "ALT+O")?;
    state.apply(preferences);
    let key = |code, modifiers| {
        Ok::<_, oteryn_input_actions::InputError>(NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state: ButtonState::Pressed,
            modifiers,
            repeat: false,
        })
    };
    let (_, requested) = state.input(&[key(
        18,
        Modifiers::one(Modifier::Control).with(Modifier::Shift),
    )?]);
    assert!(!requested && !state.open);
    let (events, requested) = state.input(&[key(18, Modifiers::one(Modifier::Alt))?]);
    // Release the old physical O before pressing its newly bound chord.
    assert!(!requested && events.is_empty());
    state.input(&[NormalizedInputEvent::Key {
        code: KeyCode::new(18)?,
        state: ButtonState::Released,
        modifiers: Modifiers::NONE,
        repeat: false,
    }]);
    let (events, requested) = state.input(&[key(18, Modifiers::one(Modifier::Alt))?]);
    assert!(events.is_empty() && requested && state.open);
    state.open = false;
    state.shutdown = true;
    assert!(!state.request_open());
    Ok(())
}

#[test]
fn settings_sync_keeps_audio_when_file_replacement_fails() -> Result<(), Box<dyn std::error::Error>>
{
    let path = std::env::temp_dir().join(format!("oteryn-c25-blocked-{}", std::process::id()));
    std::fs::create_dir_all(&path)?;
    let mut state = SettingsState::new(Some(path.clone()));
    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
    state.restore(&mut view, [960, 540])?;
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(68)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    let (events, _) = state.input(&[event]);
    let before = view.preferences().audio.master_volume;
    assert!(view.audio_input(&events));
    state.capture(&mut view)?;
    state.restore(&mut view, [960, 540])?;
    assert_eq!(view.preferences().audio.master_volume, before - 5);
    assert!(state.feedback.contains("could not save"));
    assert!(path.is_dir());
    std::fs::remove_dir(path)?;
    Ok(())
}
