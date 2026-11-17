use super::*;

#[test]
fn chat_hotkey_presets_follow_the_live_composer_context() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::SettingsState;
    use oteryn_client_ui::chat::ComposerMode;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let (link, _, _) = play_channel();
    let path = std::env::temp_dir().join(format!("cgopt-chat-context-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    settings.apply_chat_preset(false)?;
    settings.sync_chat_context(&link);
    let saved = std::fs::read(&path)?;
    assert_eq!(
        crate::play::mailbox::lock(&link.mailbox).chat.console.mode,
        ComposerMode::Off
    );
    let press = NormalizedInputEvent::Key {
        code: KeyCode::new(26)?,
        modifiers: Modifiers::NONE,
        state: ButtonState::Pressed,
        repeat: false,
    };
    assert!(matches!(
        settings.input(&[press]).0.as_slice(),
        [NormalizedInputEvent::Key {
            code: KeyCode::ARROW_UP,
            ..
        }]
    ));
    crate::play::mailbox::lock(&link.mailbox).chat.console.mode = ComposerMode::Temporary;
    settings.sync_chat_context(&link);
    assert!(
        !settings
            .preferences
            .keybindings
            .contains_key("0013-paced-movement-move-north")
    );
    assert_eq!(std::fs::read(&path)?, saved);
    crate::play::mailbox::lock(&link.mailbox)
        .chat
        .console
        .sent();
    settings.sync_chat_context(&link);
    assert_eq!(
        settings
            .preferences
            .keybindings
            .get("0013-paced-movement-move-north")
            .map(String::as_str),
        Some("W")
    );
    settings.manual_hotkeys();
    crate::play::mailbox::lock(&link.mailbox).chat.console.mode = ComposerMode::On;
    settings.sync_chat_context(&link);
    assert_eq!(
        settings
            .preferences
            .keybindings
            .get("0013-paced-movement-move-north")
            .map(String::as_str),
        Some("W")
    );
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn loaded_hotkey_profile_preserves_manual_routes_in_live_chat_context()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::SettingsState;
    use oteryn_client_ui::chat::ComposerMode;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let (link, _, _) = play_channel();
    let path =
        std::env::temp_dir().join(format!("cgopt-profile-context-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    let mut manual = settings.preferences.clone();
    manual.general_hotkey_preset(false)?;
    manual.save_hotkey_profile("manual movement")?;
    settings.apply(manual);
    // A pending preset must not survive an explicit profile selection.
    settings.apply_chat_preset(true)?;
    let mut profile = settings.preferences.clone();
    profile.load_hotkey_profile("manual movement")?;
    settings.apply(profile);
    crate::play::mailbox::lock(&link.mailbox).chat.console.mode = ComposerMode::Off;
    settings.sync_chat_context(&link);
    assert_eq!(
        crate::play::mailbox::lock(&link.mailbox).chat.console.mode,
        ComposerMode::Off
    );
    assert!(!settings.preferences.chat_context_hotkeys);
    let forward = KeyCode::new(26)?;
    for mode in [ComposerMode::On, ComposerMode::Temporary, ComposerMode::Off] {
        crate::play::mailbox::lock(&link.mailbox).chat.console.mode = mode;
        settings.sync_chat_context(&link);
        let gesture =
            [ButtonState::Pressed, ButtonState::Released].map(|state| NormalizedInputEvent::Key {
                code: forward,
                modifiers: Modifiers::NONE,
                state,
                repeat: false,
            });
        assert!(matches!(
            settings.input(&gesture).0.first(),
            Some(NormalizedInputEvent::Key {
                code: KeyCode::ARROW_UP,
                ..
            })
        ));
    }
    for open in [false, true] {
        crate::play::mailbox::lock(&link.mailbox).chat.open = open;
        let gesture =
            [ButtonState::Pressed, ButtonState::Released].map(|state| NormalizedInputEvent::Key {
                code: forward,
                modifiers: Modifiers::NONE,
                state,
                repeat: false,
            });
        assert!(matches!(
            settings.input_from_play(&gesture, &link).0.first(),
            Some(NormalizedInputEvent::Key {
                code: KeyCode::ARROW_UP,
                ..
            })
        ));
    }
    let restarted = SettingsState::new(Some(path.clone()));
    assert!(!restarted.preferences.chat_context_hotkeys);
    assert_eq!(
        restarted
            .preferences
            .keybindings
            .get("0013-paced-movement-move-north")
            .map(String::as_str),
        Some("W")
    );
    let mut invalid = settings.preferences.clone();
    let before = invalid.clone();
    assert!(invalid.load_hotkey_profile("missing").is_err());
    assert_eq!(invalid, before);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn docked_chat_mode_controls_the_real_wasd_route() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{MovementKeys, SettingsState};
    use oteryn_client_ui::chat::{ComposerMode, Mode};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let (link, mut commands, _outbox) = play_channel();
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.chat.open = true;
        mailbox.chat.selected = true;
    }
    let mut settings = SettingsState::new(None);
    let mut p = settings.preferences.clone();
    p.controls.movement = MovementKeys::ArrowsAndWasd;
    settings.apply(p);
    let code = KeyCode::new(26)?;
    let gesture =
        [ButtonState::Pressed, ButtonState::Released].map(|state| NormalizedInputEvent::Key {
            code,
            state,
            modifiers: Modifiers::NONE,
            repeat: false,
        });
    let first = |settings: &mut SettingsState| {
        settings.sync_chat_context(&link);
        settings
            .control_events_at(
                &gesture,
                link.preference_chat_open(),
                std::time::Duration::ZERO,
            )
            .first()
            .and_then(|event| match event {
                NormalizedInputEvent::Key { code, .. } => Some(*code),
                _ => None,
            })
    };
    assert_eq!(
        first(&mut settings),
        Some(code),
        "chat On preserves typed W"
    );
    link.chat_console_mode();
    assert_eq!(link.chat_panel().console.mode, ComposerMode::Auto);
    assert!(
        !link.preference_chat_open(),
        "chat Auto leaves the keys with the game until the input is clicked"
    );
    assert_eq!(first(&mut settings), Some(KeyCode::ARROW_UP));
    link.chat_auto_focus(true);
    assert!(link.preference_chat_open());
    assert_eq!(first(&mut settings), Some(code), "a clicked Auto input types W");
    link.chat_auto_focus(false);
    assert_eq!(first(&mut settings), Some(KeyCode::ARROW_UP));
    link.chat_console_mode();
    assert_eq!(link.chat_panel().console.mode, ComposerMode::Off);
    assert!(
        !link.preference_chat_open(),
        "an open docked pane with chat Off is not a text-input context"
    );
    assert_eq!(first(&mut settings), Some(KeyCode::ARROW_UP));
    link.chat_set_mode(ComposerMode::Temporary);
    assert_eq!(first(&mut settings), Some(code));
    link.chat_text("one temporary message".into());
    link.chat_request(Mode::Say, "");
    assert!(
        std::iter::from_fn(|| commands.try_recv().ok())
            .any(|command| matches!(command, crate::combat_input::PlayCommand::ChatPublic(_)))
    );
    assert_eq!(link.chat_panel().console.mode, ComposerMode::Off);
    assert_eq!(
        first(&mut settings),
        Some(KeyCode::ARROW_UP),
        "sending ends temporary chat and restores movement"
    );
    Ok(())
}

#[test]
fn chosen_chat_mode_persists_per_character_and_restores_next_session()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::SettingsState;
    use oteryn_client_ui::chat::ComposerMode;
    let path = std::env::temp_dir().join(format!("cgopt-chat-mode-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let (link, _, _) = play_channel();
    let mut settings = SettingsState::new(Some(path.clone()));
    settings.sync_chat_context(&link);
    assert_eq!(link.chat_panel().console.mode, ComposerMode::On);
    assert!(!path.exists(), "an untouched mode writes nothing");
    link.chat_console_mode();
    settings.sync_chat_context(&link);
    assert_eq!(
        settings.preferences.chat_mode,
        Some(crate::play::feature_local_reference_preferences::settings::ChatMode::Auto)
    );
    // A one-message composer from Chat Off is not a new choice.
    link.chat_console_mode();
    settings.sync_chat_context(&link);
    link.chat_set_mode(ComposerMode::Temporary);
    settings.sync_chat_context(&link);
    let saved = std::fs::read(&path)?;
    link.chat_set_mode(ComposerMode::Off);
    settings.sync_chat_context(&link);
    assert_eq!(std::fs::read(&path)?, saved);
    link.chat_console_mode();
    link.chat_console_mode();
    settings.sync_chat_context(&link);
    assert_eq!(link.chat_panel().console.mode, ComposerMode::Auto);
    // The next session of the character starts in the saved mode.
    let (next, _, _) = play_channel();
    let mut restarted = SettingsState::new(Some(path.clone()));
    restarted.sync_chat_context(&next);
    assert_eq!(next.chat_panel().console.mode, ComposerMode::Auto);
    assert!(!next.chat_panel().console.can_send());
    let _ = std::fs::remove_file(&path);
    Ok(())
}
