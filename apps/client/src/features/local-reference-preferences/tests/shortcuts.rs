use crate::play::{SettingsState, ShortcutEdit, WINDOW_ACTIONS};
#[test]
fn t059_settings_selector_persists_order_and_routes_the_selected_shortcut()
-> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("t059-shortcuts-{}.json", std::process::id()));
    let mut state = SettingsState::new(Some(path.clone()));
    let id = WINDOW_ACTIONS[0].0;
    assert!(state.edit_shortcut(id, ShortcutEdit::Remove));
    assert!(
        state
            .shortcut_choices(false)
            .iter()
            .any(|(key, _)| *key == id)
    );
    assert!(state.edit_shortcut(id, ShortcutEdit::Add));
    assert_eq!(
        state.preferences.shortcut_order.last().map(String::as_str),
        Some(id)
    );
    assert!(state.edit_shortcut(id, ShortcutEdit::Up));
    let position = state
        .preferences
        .shortcut_order
        .iter()
        .position(|key| key == id)
        .ok_or("position")?;
    assert_eq!(position, WINDOW_ACTIONS.len() - 2);
    assert!(state.edit_shortcut(id, ShortcutEdit::Down));
    assert!(!state.edit_shortcut(id, ShortcutEdit::Add));
    let before = state.preferences.clone();
    assert!(!state.edit_shortcut("unknown", ShortcutEdit::Add));
    assert_eq!(state.preferences, before);
    let mut restored = SettingsState::new(Some(path.clone()));
    assert_eq!(
        restored.preferences.shortcut_order,
        state.preferences.shortcut_order
    );
    assert_eq!(
        restored.shortcut_choices(true).last().map(|(key, _)| *key),
        Some(id)
    );
    assert!(restored.queue_window(id));
    assert!(oteryn_client_ui::cyclopedia_character::open_key(
        &restored.take_window_events()
    ));
    std::fs::remove_file(path)?;
    Ok(())
}
