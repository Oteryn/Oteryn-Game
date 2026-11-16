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
    assert_eq!(commands.try_recv()?, PlayCommand::ChatClearOldest);
    // The Session owner clears history; this caller only queues its request.
    assert_eq!(lock(&link.mailbox).chat.lines, vec![line("old"), line("new")]);

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

    let transcript_shown = lock(&link.mailbox).chat.console.show_messages;
    let shown = lock(&link.mailbox).chat.console.show_server_messages;
    assert!(settings.run_utility(&link, UtilityAction::ToggleServerMessages));
    assert_eq!(lock(&link.mailbox).chat.console.show_server_messages, !shown);
    assert_eq!(
        lock(&link.mailbox).chat.console.show_messages,
        transcript_shown
    );

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
    assert_eq!(
        preferences.custom_hotkeys,
        crate::play::default_custom_hotkeys()
    );
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

#[test]
fn chat_channels_utilities_drive_the_console_tabs() -> Result<(), Box<dyn std::error::Error>> {
    use crate::play::UtilityAction;
    let (link, mut commands, outbox) = play_channel();
    let mut settings = bare();
    outbox.chat_initialize(true);
    outbox.chat_public_initialize(true, Some((oteryn_session::ChatRoomSet::default(), Vec::new())));
    assert!(settings.run_utility(&link, UtilityAction::OpenHelpChannel));
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::ChatPublic(oteryn_session::ChatIntent::OpenRoom(
            oteryn_session::ChatRoom::Help
        ))
    ));
    assert!(link.chat_open_conversation("Bryn"));
    assert!(settings.run_utility(&link, UtilityAction::PreviousChannel));
    assert_eq!(lock(&link.mailbox).chat.console.current, None);
    assert!(settings.run_utility(&link, UtilityAction::NextChannel));
    assert!(lock(&link.mailbox).chat.console.current.is_some());
    assert!(settings.run_utility(&link, UtilityAction::CloseCurrentChannel));
    assert!(lock(&link.mailbox).chat.console.channels.is_empty());
    assert!(settings.run_utility(&link, UtilityAction::OpenChannelList));
    assert!(lock(&link.mailbox).chat.console.dialog.open);
    assert!(settings.run_utility(&link, UtilityAction::ToggleLensHelp));
    assert!(link.lens_active());
    assert!(settings.run_utility(&link, UtilityAction::ToggleLensHelp));
    assert!(!link.lens_active());
    Ok(())
}

fn press(code: u16, shift: bool, ctrl: bool) -> oteryn_input_actions::NormalizedInputEvent {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut modifiers = Modifiers::default();
    if ctrl {
        modifiers = modifiers.with(Modifier::Control);
    }
    if shift {
        modifiers = modifiers.with(Modifier::Shift);
    }
    NormalizedInputEvent::Key {
        code: KeyCode::new(code).unwrap_or_else(|_| unreachable!("a fixed key code")),
        state: ButtonState::Pressed,
        modifiers,
        repeat: false,
    }
}

#[test]
fn global_chat_chords_are_the_default_hotkeys_and_conflict_with_nothing()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::UtilityAction::*;
    let defaults = crate::play::default_custom_hotkeys();
    for (chord, action) in [
        ("CTRL+E", CloseCurrentChannel),
        ("TAB", NextChannel),
        ("SHIFT+TAB", PreviousChannel),
        ("CTRL+O", OpenChannelList),
        ("CTRL+T", OpenHelpChannel),
        ("CTRL+H", ToggleLensHelp),
    ] {
        assert_eq!(defaults.get(chord), Some(&CustomAction::Utility { action }));
    }
    let preferences = ReferencePreferences::default();
    // The features that held those chords have no default key; the player can still bind them.
    let bindings = crate::play::preference_bindings()?;
    for id in [
        "0009-highscores-open",
        "0060-player-trade-open",
        "0072-task-board-open",
        "0147-exiva-settings-open",
    ] {
        let binding = bindings.iter().find(|b| b.id == id).ok_or("binding")?;
        assert_eq!(binding.default_chord, "", "{id}");
        let mut rebound = preferences.clone();
        rebound.rebind(id, "CTRL+SHIFT+F9")?;
    }
    // Rebinding a feature onto a Global chord is refused while the chord is taken.
    let mut taken = preferences.clone();
    assert!(taken.rebind("0009-highscores-open", "CTRL+H").is_err());
    // A saved profile without the field gets the defaults; one that emptied it keeps it empty.
    let mut old: serde_json::Value = serde_json::to_value(&preferences)?;
    old.as_object_mut().ok_or("object")?.remove("custom_hotkeys");
    assert_eq!(
        serde_json::from_value::<ReferencePreferences>(old.clone())?.custom_hotkeys,
        defaults
    );
    old["custom_hotkeys"] = serde_json::json!({});
    assert!(
        serde_json::from_value::<ReferencePreferences>(old)?
            .custom_hotkeys
            .is_empty()
    );
    Ok(())
}

#[test]
fn global_chat_chords_drive_channels_the_list_and_the_lens() -> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = play_channel();
    let play = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 7, &[])?;
    outbox.chat_initialize(true);
    outbox.chat_public_initialize(true, Some((oteryn_session::ChatRoomSet::default(), Vec::new())));
    assert!(link.chat_open_conversation("Bryn"));
    assert!(link.chat_open_conversation("Ana"));
    let mut settings = bare();
    let mut fire = |chat_on: bool, code: u16, shift: bool, ctrl: bool| {
        settings.preferences.hotkey_contexts.chat_on = chat_on;
        let _ = settings.input(&[press(code, shift, ctrl)]);
        let actions = settings.take_custom_actions();
        let mut release = press(code, shift, ctrl);
        if let oteryn_input_actions::NormalizedInputEvent::Key { state, .. } = &mut release {
            *state = oteryn_input_actions::ButtonState::Released;
        }
        let _ = settings.input(&[release]);
        actions.len() == 1 && link.custom_hotkey_action(&play, &actions[0])
    };
    let current = || lock(&link.mailbox).chat.console.current.clone();
    // Tab and Shift+Tab switch channels, also while chat is on and composing.
    for chat_on in [false, true] {
        let first = current();
        assert!(fire(chat_on, 43, false, false));
        assert_ne!(current(), first);
        assert!(fire(chat_on, 43, true, false));
        assert_eq!(current(), first);
    }
    // Ctrl+O lists the channels, Ctrl+H toggles the lens, Ctrl+E closes the current channel.
    assert!(fire(false, 18, false, true));
    assert!(lock(&link.mailbox).chat.console.dialog.open);
    assert!(fire(false, 11, false, true));
    assert!(link.lens_active());
    assert!(fire(false, 11, false, true));
    assert!(!link.lens_active());
    let tabs = lock(&link.mailbox).chat.console.channels.len();
    assert!(fire(false, 8, false, true));
    assert_eq!(lock(&link.mailbox).chat.console.channels.len(), tabs - 1);
    // Ctrl+T opens the Help channel.
    assert!(fire(false, 23, false, true));
    Ok(())
}

#[test]
fn clear_oldest_message_keeps_private_echo_recipients_aligned() {
    use crate::play::UtilityAction;
    use oteryn_client_ui::chat::{ChatExtras, panel_text};
    let (link, mut commands, outbox) = play_channel();
    outbox.chat_initialize(true);
    outbox.chat_public_state(
        oteryn_session::ChatRoomSet::default(),
        vec![
            oteryn_session::ChatLine::Private {
                speaker_name: "Ari".into(),
                text: "old to Bo".into(),
            },
            oteryn_session::ChatLine::Private {
                speaker_name: "Ari".into(),
                text: "new to Cy".into(),
            },
        ],
    );
    lock(&link.mailbox).chat.apply_extras(
        "Ari",
        None,
        ChatExtras {
            recipients: Some(vec![Some("Bo".into()), Some("Cy".into())]),
            ..Default::default()
        },
    );
    assert!(link.chat_open_conversation("Bo"));
    assert!(panel_text(&link.chat_panel()).contains("old to Bo"));
    assert!(bare().run_utility(&link, UtilityAction::ClearOldestMessage));
    assert_eq!(commands.try_recv(), Ok(PlayCommand::ChatClearOldest));
    // Only the Session command changes history. Its next publication replaces every view.
    outbox.chat_public_state(
        oteryn_session::ChatRoomSet::default(),
        vec![oteryn_session::ChatLine::Private {
            speaker_name: "Ari".into(),
            text: "new to Cy".into(),
        }],
    );
    lock(&link.mailbox).chat.apply_extras(
        "Ari",
        None,
        ChatExtras {
            recipients: Some(vec![Some("Cy".into())]),
            ..Default::default()
        },
    );
    assert!(!panel_text(&link.chat_panel()).contains("old to Bo"));
    assert!(!panel_text(&link.chat_panel()).contains("new to Cy"));
    assert!(link.chat_open_conversation("Cy"));
    assert!(panel_text(&link.chat_panel()).contains("To Cy: new to Cy"));
    assert_eq!(link.chat_panel().recipients, vec![Some("Cy".into())]);
    assert!(bare().run_utility(&link, UtilityAction::ClearOldestMessage));
    assert_eq!(commands.try_recv(), Ok(PlayCommand::ChatClearOldest));
    outbox.chat_public_state(oteryn_session::ChatRoomSet::default(), Vec::new());
    lock(&link.mailbox)
        .chat
        .apply_extras("Ari", None, ChatExtras::default());
    assert!(link.chat_panel().recipients.is_empty());
    assert!(!bare().run_utility(&link, UtilityAction::ClearOldestMessage));
}

#[test]
fn server_message_utility_preserves_player_lines_across_console_tabs() {
    use crate::play::UtilityAction;
    use oteryn_client_ui::chat::{ChannelTab, ConsoleTab, panel_text};
    use oteryn_session::{AppliedDelta, ChatLine, ChatRoomSet, ServerMessage, SessionEvent};
    let (link, _, outbox) = play_channel();
    outbox.chat_initialize(true);
    outbox.chat_public_state(
        ChatRoomSet::default(),
        vec![ChatLine::Private {
            speaker_name: "Bryn".into(),
            text: "Meet at the bridge.".into(),
        }],
    );
    outbox.push(vec![SessionEvent::ServerMessage(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: ServerMessage::StatusText("You are in a safe area.".into()),
    })]);
    assert!(link.chat_open_conversation("Bryn"));
    for channel in [None, Some(ChannelTab::Conversation("Bryn".into()))] {
        link.chat_console_tab(ConsoleTab::All);
        lock(&link.mailbox).chat.console.current = channel.clone();
        assert!(panel_text(&link.chat_panel()).contains("Meet at the bridge."));
        assert!(bare().run_utility(&link, UtilityAction::ToggleServerMessages));
        assert!(panel_text(&link.chat_panel()).contains("Meet at the bridge."));
        assert!(!panel_text(&link.chat_panel()).contains("You are in a safe area."));
        assert!(link.chat_panel().console.show_messages);
        assert!(bare().run_utility(&link, UtilityAction::ToggleServerMessages));
        if channel.is_none() {
            assert!(panel_text(&link.chat_panel()).contains("You are in a safe area."));
        }
    }
}

/// Tibia Global binds Ctrl+M to ToggleShowServermessagesInCurrentChannel
/// (docs/reference/tibia-manual/controls.md:93, and the Global client options file) and opens the
/// Market only from the depot locker (docs/reference/tibia-manual/controls_trading.md:29).
#[test]
fn global_ctrl_m_toggles_server_messages_and_the_market_has_no_default_chord()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::UtilityAction::ToggleServerMessages;
    assert_eq!(
        crate::play::default_custom_hotkeys().get("CTRL+M"),
        Some(&CustomAction::Utility {
            action: ToggleServerMessages
        })
    );
    let bindings = crate::play::preference_bindings()?;
    let market = bindings
        .iter()
        .find(|b| b.id == "0063-market-open")
        .ok_or("market binding")?;
    assert_eq!(market.default_chord, "");
    // The player can still give the Market a free chord.
    ReferencePreferences::default().rebind("0063-market-open", "CTRL+SHIFT+F9")?;

    let (link, _commands, _outbox) = play_channel();
    let play = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 7, &[])?;
    let mut settings = bare();
    let (routed, _) = settings.input(&[press(16, false, true)]);
    let actions = settings.take_custom_actions();
    assert_eq!(
        actions,
        vec![CustomAction::Utility {
            action: ToggleServerMessages
        }]
    );
    assert!(!link.market_input(&routed));
    let shown = lock(&link.mailbox).chat.console.show_server_messages;
    assert!(link.custom_hotkey_action(&play, &actions[0]));
    assert_eq!(lock(&link.mailbox).chat.console.show_server_messages, !shown);
    assert!(!link.market_panel().open);
    Ok(())
}

/// Global's hotkey manager offers ShowPrey, ToggleVipWidget and ToggleSkillsWidget as actions.
#[test]
fn n021_prey_vip_and_skills_are_assignable_hotkey_actions() -> Result<(), Box<dyn std::error::Error>>
{
    use crate::play::UtilityAction::{ShowPrey, ToggleSkillsWidget, ToggleVipWidget};
    let (link, mut commands, _outbox) = play_channel();
    let play = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 7, &[])?;
    let mut settings = bare();
    let mut preferences = settings.preferences.clone();
    for (chord, action) in [
        ("CTRL+SHIFT+F5", ShowPrey),
        ("CTRL+SHIFT+F6", ToggleVipWidget),
        ("CTRL+SHIFT+F7", ToggleSkillsWidget),
    ] {
        preferences.assign_custom_hotkey(chord, CustomAction::Utility { action })?;
    }
    settings.apply(preferences);
    lock(&link.mailbox).prey.selected = true;
    lock(&link.mailbox).vip.selected = true;
    let mut fire = |code: u16| {
        let mut event = press(code, true, true);
        let _ = settings.input(std::slice::from_ref(&event));
        let actions = settings.take_custom_actions();
        if let oteryn_input_actions::NormalizedInputEvent::Key { state, .. } = &mut event {
            *state = oteryn_input_actions::ButtonState::Released;
        }
        let _ = settings.input(&[event]);
        actions.len() == 1 && link.custom_hotkey_action(&play, &actions[0])
    };
    // F5..F7 are key codes 62..64.
    assert!(fire(62));
    assert!(lock(&link.mailbox).prey.open);
    assert!(matches!(commands.try_recv()?, PlayCommand::Prey(_)));
    assert!(fire(63));
    assert!(lock(&link.mailbox).vip.open);
    assert!(matches!(commands.try_recv()?, PlayCommand::Vip(_)));
    assert!(fire(63));
    assert!(!lock(&link.mailbox).vip.open);
    assert!(fire(64));
    assert!(lock(&link.mailbox).loyalty_skills.open);
    assert!(fire(64));
    assert!(!lock(&link.mailbox).loyalty_skills.open);
    Ok(())
}

/// The installed Tibia Global client options (%LOCALAPPDATA%/Tibia/packages/Tibia/conf/
/// clientoptions.json, every hotkey set, chat on and chat off) bind Ctrl+Y to ShowPrey, Ctrl+P to
/// ToggleVipWidget, Alt+S to ToggleSkillsWidget and Ctrl+K to ShowOptionsHotkeys; they bind
/// nothing to Ctrl+V and give the boss slots no chord.
#[test]
fn global_default_chords_for_prey_vip_skills_and_hotkey_options()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::UtilityAction::{
        ShowOptionsHotkeys, ShowPrey, ToggleSkillsWidget, ToggleVipWidget,
    };
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let defaults = crate::play::default_custom_hotkeys();
    for (chord, action) in [
        ("CTRL+Y", ShowPrey),
        ("CTRL+P", ToggleVipWidget),
        ("ALT+S", ToggleSkillsWidget),
        ("CTRL+K", ShowOptionsHotkeys),
    ] {
        assert_eq!(
            defaults.get(chord),
            Some(&CustomAction::Utility { action }),
            "{chord}"
        );
    }
    assert!(!defaults.contains_key("CTRL+V"));
    let bindings = crate::play::preference_bindings()?;
    for id in [
        "0043-vip-open",
        "0140-prey-open",
        "0225-boss-difficulty-open",
        "0274-loyalty-skills-open",
    ] {
        let binding = bindings.iter().find(|b| b.id == id).ok_or("binding")?;
        assert_eq!(binding.default_chord, "", "{id}");
    }

    let (link, mut commands, _outbox) = play_channel();
    lock(&link.mailbox).prey.selected = true;
    lock(&link.mailbox).vip.selected = true;
    let mut settings = bare();
    let mut fire = |code: u16, modifier: Modifier| {
        let mut event = NormalizedInputEvent::Key {
            code: KeyCode::new(code).unwrap_or_else(|_| unreachable!("a fixed key code")),
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(modifier),
            repeat: false,
        };
        let (routed, _) = settings.input(std::slice::from_ref(&event));
        let actions = settings.take_custom_actions();
        if let NormalizedInputEvent::Key { state, .. } = &mut event {
            *state = ButtonState::Released;
        }
        let _ = settings.input(&[event]);
        let ran = actions.iter().all(|action| match action {
            CustomAction::Utility { action } => settings.run_utility(&link, *action),
            _ => false,
        });
        (routed, actions, ran)
    };
    // Y, P, S, K and V are key codes 28, 19, 22, 14 and 25.
    let (routed, actions, ran) = fire(28, Modifier::Control);
    assert_eq!(actions, vec![CustomAction::Utility { action: ShowPrey }]);
    assert!(ran && lock(&link.mailbox).prey.open);
    assert!(matches!(commands.try_recv()?, PlayCommand::Prey(_)));
    assert!(!link.boss_difficulty_input(&routed));
    assert!(!lock(&link.mailbox).boss_difficulty.open);
    let (_, actions, ran) = fire(19, Modifier::Control);
    assert_eq!(
        actions,
        vec![CustomAction::Utility {
            action: ToggleVipWidget
        }]
    );
    assert!(ran && lock(&link.mailbox).vip.open);
    let (_, actions, ran) = fire(22, Modifier::Alt);
    assert_eq!(
        actions,
        vec![CustomAction::Utility {
            action: ToggleSkillsWidget
        }]
    );
    assert!(ran && lock(&link.mailbox).loyalty_skills.open);
    let (_, actions, _) = fire(14, Modifier::Control);
    assert_eq!(
        actions,
        vec![CustomAction::Utility {
            action: ShowOptionsHotkeys
        }]
    );
    lock(&link.mailbox).vip.open = false;
    let (routed, actions, _) = fire(25, Modifier::Control);
    assert!(actions.is_empty());
    assert!(!link.vip_input(&routed));
    assert!(!lock(&link.mailbox).vip.open);
    Ok(())
}

/// Tibia Global's ToggleManualSortMode (docs/reference/tibia-manual/controls.md:79; Shift+S in
/// every hotkey set of the installed Global client options): the hotkey toggles the session's
/// mode, and while it is on a drop on a container slot sends that slot (capability 510).
#[test]
fn shift_s_toggles_manual_sort_mode_and_a_container_drop_sends_its_slot()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::UtilityAction::ToggleManualSortMode;
    use oteryn_client_ui::inventory::movement::DragItem;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    use oteryn_session::ItemMoveDestination;
    assert_eq!(
        crate::play::default_custom_hotkeys().get("SHIFT+S"),
        Some(&CustomAction::Utility {
            action: ToggleManualSortMode
        })
    );
    for selected in [false, true] {
        let (link, mut commands, outbox) = play_channel();
        outbox.inventory_initialize(None, false);
        outbox.container_manual_sort_initialize(selected);
        let mut settings = bare();
        let mut shift_s = || {
            let mut event = NormalizedInputEvent::Key {
                code: KeyCode::new(22).unwrap_or_else(|_| unreachable!("a fixed key code")),
                state: ButtonState::Pressed,
                modifiers: Modifiers::one(Modifier::Shift),
                repeat: false,
            };
            let _ = settings.input(std::slice::from_ref(&event));
            let actions = settings.take_custom_actions();
            if let NormalizedInputEvent::Key { state, .. } = &mut event {
                *state = ButtonState::Released;
            }
            let _ = settings.input(&[event]);
            assert_eq!(
                actions,
                vec![CustomAction::Utility {
                    action: ToggleManualSortMode
                }]
            );
            settings.run_utility(&link, ToggleManualSortMode)
        };
        let drop = |commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>| {
            assert!(link.item_drag_begin(DragItem {
                equipped: false,
                handle: 5.try_into().unwrap_or_else(|_| unreachable!("non-zero")),
                count: 1,
            }));
            link.item_drag_drop(Some(ItemMoveDestination::Container(
                9.try_into().unwrap_or_else(|_| unreachable!("non-zero")),
                Some(2),
            )));
            match commands.try_recv() {
                Ok(PlayCommand::MoveItem(request)) => Some(request.destination),
                _ => None,
            }
        };
        let container = 9.try_into()?;
        assert!(!link.container_manual_sort_active());
        assert_eq!(
            drop(&mut commands),
            Some(ItemMoveDestination::Container(container, None))
        );
        lock(&link.mailbox).item_movement.pending = false;
        assert_eq!(shift_s(), selected, "510 negotiated: {selected}");
        assert_eq!(link.container_manual_sort_active(), selected);
        let slot = selected.then_some(2);
        assert_eq!(
            drop(&mut commands),
            Some(ItemMoveDestination::Container(container, slot))
        );
        lock(&link.mailbox).item_movement.pending = false;
        assert_eq!(shift_s(), selected);
        assert!(!link.container_manual_sort_active());
        assert_eq!(
            drop(&mut commands),
            Some(ItemMoveDestination::Container(container, None))
        );
    }
    Ok(())
}
