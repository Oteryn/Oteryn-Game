use crate::combat_input::PlayCommand;
use crate::play::{CustomAction, ReferencePreferences, SettingsState};
use crate::play::{mailbox::lock, play_channel};

fn bare() -> SettingsState {
    SettingsState::new(None)
}

#[test]
fn n022_preference_utilities_toggle_bars_indicator_and_cycle_presets()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::UtilityAction;
    let (link, _, _) = play_channel();
    let mut settings = bare();
    let bars = settings.preferences.creature_bars;
    assert!(settings.run_utility(&link, UtilityAction::ToggleCreatureHuds));
    assert_eq!(settings.preferences.creature_bars, !bars);
    assert!(settings.run_utility(&link, UtilityAction::ToggleFpsLagIndicator));
    assert!(settings.preferences.performance.show_fps);
    assert!(settings.preferences.performance.show_latency);
    assert!(settings.run_utility(&link, UtilityAction::ToggleFpsLagIndicator));
    assert!(!settings.preferences.performance.show_fps);
    assert!(!settings.preferences.performance.show_latency);

    let mut preferences = ReferencePreferences::default();
    preferences.add_hotkey_profile("Zeta")?;
    let first = preferences
        .hotkey_contexts
        .presets
        .keys()
        .next()
        .ok_or("no preset")?
        .clone();
    preferences.select_hotkey_profile(&first)?;
    let second = preferences
        .hotkey_contexts
        .presets
        .keys()
        .nth(1)
        .ok_or("one preset")?
        .clone();
    preferences.next_hotkey_preset()?;
    assert_eq!(preferences.hotkey_contexts.selected, second);
    preferences.next_hotkey_preset()?;
    assert_eq!(preferences.hotkey_contexts.selected, first);
    Ok(())
}

#[test]
fn n022_session_utilities_use_existing_callers() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::UtilityAction;
    let (link, mut commands, _) = play_channel();
    let mut settings = bare();
    assert!(settings.run_utility(&link, UtilityAction::ChangeCharacter));
    assert!(matches!(commands.try_recv()?, PlayCommand::Logout));

    assert!(!settings.run_utility(&link, UtilityAction::ClearOldestMessage));
    let line = |text: &str| {
        oteryn_session::ChatLine::Guild(oteryn_session::GuildChatLine {
            guild_id: [1; 16],
            rank: 1,
            speaker_name: "Player One".into(),
            text: text.into(),
        })
    };
    lock(&link.mailbox).chat.lines = vec![line("old"), line("new")];
    assert!(settings.run_utility(&link, UtilityAction::ClearOldestMessage));
    assert_eq!(lock(&link.mailbox).chat.lines, vec![line("new")]);

    lock(&link.mailbox)
        .chat
        .console
        .communication
        .ignored
        .insert("Pest".into());
    assert!(settings.run_utility(&link, UtilityAction::ShowIgnoreList));
    assert!(
        lock(&link.mailbox)
            .chat
            .feedback
            .as_deref()
            .is_some_and(|text| text.contains("Pest"))
    );

    let shown = lock(&link.mailbox).chat.console.show_messages;
    assert!(settings.run_utility(&link, UtilityAction::ToggleServerMessages));
    assert_eq!(lock(&link.mailbox).chat.console.show_messages, !shown);

    assert!(!settings.run_utility(&link, UtilityAction::QuickLootArea));
    lock(&link.mailbox).discovery.loot.selected = true;
    assert!(settings.run_utility(&link, UtilityAction::QuickLootArea));
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::ServerMechanic(_)
    ));

    assert!(settings.run_utility(&link, UtilityAction::BugReport));
    assert!(lock(&link.mailbox).properties.player_report.bug.open);
    Ok(())
}

#[test]
fn n022_utilities_are_unbound_choices_and_survive_json() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::UtilityAction;
    let (link, _, _) = play_channel();
    let choices = link.custom_hotkey_choices();
    for action in UtilityAction::ALL {
        assert!(
            choices
                .iter()
                .any(|(_, choice)| *choice == CustomAction::Utility { action })
        );
    }
    let mut preferences = ReferencePreferences::default();
    assert!(preferences.custom_hotkeys.is_empty());
    preferences.assign_custom_hotkey(
        "CTRL+SHIFT+F9",
        CustomAction::Utility {
            action: UtilityAction::NextHotkeyPreset,
        },
    )?;
    let text = serde_json::to_string(&preferences)?;
    assert_eq!(
        serde_json::from_str::<ReferencePreferences>(&text)?.custom_hotkeys,
        preferences.custom_hotkeys
    );
    Ok(())
}
