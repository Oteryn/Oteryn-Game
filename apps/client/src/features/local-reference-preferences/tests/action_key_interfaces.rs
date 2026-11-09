use super::*;

#[test]
fn options_key_edit_uses_action_keys_and_executes_the_live_mixed_slot()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{BarAction, SettingsState};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let path =
        std::env::temp_dir().join(format!("cgopt-key-interface-{}.json", std::process::id()));
    let mut settings = SettingsState::new(Some(path.clone()));
    let mut preferences = settings.preferences.clone();
    preferences.rebind("local-spell-action-bar-spell-36", "SUPER+ALT+F12")?;
    assert_eq!(
        preferences.action_keys.0.get(&35).map(String::as_str),
        Some("SUPER+ALT+F12")
    );
    settings.apply(preferences);
    let (link, mut commands, outbox) = play_channel();
    outbox.chat_initialize(true);
    settings.sync_bar_actions(&link);
    assert!(link.assign_bar_action(
        35,
        BarAction::Text {
            text: "Last row greeting".into()
        }
    ));
    settings.sync_bar_actions(&link);
    settings = SettingsState::new(Some(path.clone()));
    settings.sync_bar_actions(&link);
    let play = view()?;
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(69)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Super).with(Modifier::Alt),
        repeat: false,
    };
    assert!(
        settings
            .input_in_context(std::slice::from_ref(&event), false)
            .0
            .is_empty()
    );
    let actions = settings.take_custom_actions();
    assert_eq!(actions.len(), 1);
    assert!(link.custom_hotkey_action(&play, &actions[0]));
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::ChatPublic(oteryn_session::ChatIntent::Say { text, .. }) if text == "Last row greeting")
    );
    settings.input_in_context(&[event], false);
    assert!(settings.take_custom_actions().is_empty());
    assert!(commands.try_recv().is_err());
    assert!(link.bind_action_key(35, "SUPER+ALT+F11"));
    settings.sync_bar_actions(&link);
    let binding = crate::play::preference_bindings()?
        .into_iter()
        .find(|binding| binding.id == "local-spell-action-bar-spell-36")
        .ok_or("slot")?;
    assert_eq!(
        settings.preferences.binding_chord(&binding),
        "SUPER+ALT+F11"
    );
    settings
        .preferences
        .rebind("local-spell-action-bar-spell-36", "A")?;
    let letter = NormalizedInputEvent::Key {
        code: KeyCode::new(4)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    settings.input_in_context(&[letter], true);
    assert!(settings.take_custom_actions().is_empty());
    std::fs::remove_file(path)?;
    Ok(())
}
