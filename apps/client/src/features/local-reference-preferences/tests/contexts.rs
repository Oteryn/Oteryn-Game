use super::*;

#[test]
fn chat_hotkey_presets_follow_the_live_composer_context() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::SettingsState;
    use oteryn_client_ui::chat::ComposerMode;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let (link, _, _) = play_channel();
    let path = std::env::temp_dir().join(format!("cgopt-chat-context-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    settings.apply_chat_preset(false)?;
    settings.sync_chat_context(&link);
    let saved = std::fs::read(&path)?;
    assert_eq!(crate::play::mailbox::lock(&link.mailbox).chat.console.mode, ComposerMode::Off);
    let press = NormalizedInputEvent::Key { code: KeyCode::new(26)?, modifiers: Modifiers::NONE, state: ButtonState::Pressed, repeat: false };
    assert!(matches!(settings.input(&[press]).0.as_slice(), [NormalizedInputEvent::Key {code: KeyCode::ARROW_UP, ..}]));
    crate::play::mailbox::lock(&link.mailbox).chat.console.mode = ComposerMode::Temporary;
    settings.sync_chat_context(&link);
    assert!(!settings.preferences.keybindings.contains_key("0013-paced-movement-move-north"));
    assert_eq!(std::fs::read(&path)?, saved);
    crate::play::mailbox::lock(&link.mailbox).chat.console.sent();
    settings.sync_chat_context(&link);
    assert_eq!(settings.preferences.keybindings.get("0013-paced-movement-move-north").map(String::as_str), Some("W"));
    settings.manual_hotkeys();
    crate::play::mailbox::lock(&link.mailbox).chat.console.mode = ComposerMode::On;
    settings.sync_chat_context(&link);
    assert_eq!(settings.preferences.keybindings.get("0013-paced-movement-move-north").map(String::as_str), Some("W"));
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
