use super::*;

#[test]
fn console_and_action_bar_settings_reach_received_chat_and_cast_caller()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::{SettingsState, play_channel};
    use oteryn_session::{ChatLine, ChatRoomSet};
    let path = std::env::temp_dir().join(format!("o006-{}.json", std::process::id()));
    let mut state = SettingsState::new(Some(path.clone()));
    let (link, mut commands, outbox) = play_channel();
    let mut preferences = state.preferences.clone();
    preferences.console.visible[1] = false;
    preferences.action_bar.columns = 2;
    preferences.action_bar.origin = [20, 120];
    preferences.rebind("local-spell-action-bar-spell-1", "ALT+1")?;
    state.apply(preferences);
    state.sync_presentation(&link);
    outbox.chat_public_state(
        ChatRoomSet::default(),
        vec![ChatLine::Private {
            speaker_name: "Player One".into(),
            text: "A received message".into(),
        }],
    );
    let panel = link.chat_panel();
    assert!(!oteryn_client_ui::chat::panel_text(&panel).contains("A received message"));
    assert_eq!(panel.lines.len(), 1);
    let mut preferences = state.preferences.clone();
    preferences.console.visible[1] = true;
    state.apply(preferences);
    state.sync_presentation(&link);
    assert!(oteryn_client_ui::chat::panel_text(&link.chat_panel()).contains("A received message"));
    let bounds = state.preferences.action_bar.rects([960, 540]);
    assert_eq!(bounds[0][1], bounds[1][1]);
    assert!(bounds[2][1] > bounds[0][1]);
    assert!(link.spell_bar_bind(&["9".into(), "10".into(), String::new(), String::new()]));
    assert!(state.queue_bar_slot(0));
    assert!(state.queue_bar_slot(1));
    let events = state.take_window_events();
    let (events, opened) = state.input(&events);
    assert!(!opened);
    let mut hotkeys = crate::spell::SpellHotkeys::new()?;
    let mut play = view()?;
    for slot in hotkeys.route(&events) {
        play.cast(slot);
    }
    play.tick(&link).map_err(|_| "tick")?;
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(spell)) if spell.get() == 9)
    );
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(spell)) if spell.get() == 10)
    );
    assert!(commands.try_recv().is_err());
    let restored = SettingsState::new(Some(path.clone()));
    assert_eq!(restored.preferences, state.preferences);
    assert!(!state.queue_bar_slot(4));
    std::fs::remove_file(path)?;
    Ok(())
}
