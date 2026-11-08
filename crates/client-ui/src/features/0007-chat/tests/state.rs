use super::*;
use oteryn_client_runtime::chat::{ChatIntent, ChatLine, ChatSpeechMode};

#[test]
fn speech_and_named_private_input_are_validated_and_serialized() {
    let mut panel = ChatPanel {
        text: "hello".into(),
        ..ChatPanel::default()
    };
    assert!(request(&mut panel, Mode::Say, "").is_none());
    panel.selected = true;
    for (mode, speech) in [
        (Mode::Say, ChatSpeechMode::Say),
        (Mode::Whisper, ChatSpeechMode::Whisper),
        (Mode::Yell, ChatSpeechMode::Yell),
    ] {
        panel.pending = false;
        assert_eq!(
            request(&mut panel, mode, ""),
            Some(ChatIntent::Say {
                mode: speech,
                text: "hello".into()
            })
        );
        assert!(request(&mut panel, mode, "").is_none());
    }
    panel.pending = false;
    assert!(request(&mut panel, Mode::Private, " ").is_none());
    assert_eq!(
        request(&mut panel, Mode::Private, "Player One"),
        Some(ChatIntent::Private {
            recipient_name: "Player One".into(),
            text: "hello".into()
        })
    );
    panel.pending = false;
    panel.text = "é".repeat(511);
    assert!(request(&mut panel, Mode::Say, "").is_none());
}

#[test]
fn private_lines_and_dropped_delivery_are_visible() {
    let panel = ChatPanel {
        lines: vec![
            ChatLine::Private {
                speaker_name: "Player One".into(),
                text: "hello".into(),
            },
            ChatLine::Dropped,
        ],
        ..ChatPanel::default()
    };
    let text = panel_text(&panel);
    assert!(text.contains("[Private] Player One: hello"));
    assert!(text.contains("dropped"));
}

#[test]
fn open_key_is_an_explicit_non_repeating_play_shortcut() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut event = NormalizedInputEvent::Key {
        code: KeyCode::new(6)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
        repeat: false,
    };
    assert!(open_key(&[event.clone()]));
    if let NormalizedInputEvent::Key { repeat, .. } = &mut event {
        *repeat = true;
    }
    assert!(!open_key(&[event]));
    assert!(!open_key(&[]));
    Ok(())
}

#[test]
fn local_speech_projection_preserves_mode_and_speaker() {
    let panel = ChatPanel {
        lines: vec![ChatLine::Local {
            speaker: oteryn_client_runtime::chat::ChatSpeaker {
                identity: [9; 16],
                generation: std::num::NonZeroU64::MIN,
            },
            speaker_name: "Bob".into(),
            mode: ChatSpeechMode::Yell,
            text: "over here".into(),
            position: oteryn_client_runtime::chat::ActorPosition {
                x: 2,
                y: 1,
                floor: 7,
            },
        }],
        ..ChatPanel::default()
    };
    assert!(panel_text(&panel).contains("[Yell] Bob: over here"));
}

#[test]
fn guild_lines_reach_the_native_chat_projection() {
    let panel = ChatPanel {
        guild: oteryn_client_runtime::chat_guild_room::GuildRoomState::Member {
            guild_id: [1; 16],
            rank: 1,
            motd: "".into(),
        },
        lines: vec![ChatLine::Guild(
            oteryn_client_runtime::chat_guild_room::GuildChatLine {
                guild_id: [1; 16],
                rank: 1,
                speaker_name: "Alice".into(),
                text: "Guild hello".into(),
            },
        )],
        ..Default::default()
    };
    assert!(panel_text(&panel).contains("[Guild] Alice: Guild hello"));
}

#[test]
fn guild_lines_require_current_matching_membership_and_preserve_order() {
    use oteryn_client_runtime::chat_guild_room::{GuildChatLine, GuildRoomState};
    let mut panel = ChatPanel {
        lines: vec![
            ChatLine::Private {
                speaker_name: "Bob".into(),
                text: "first".into(),
            },
            ChatLine::Guild(GuildChatLine {
                guild_id: [1; 16],
                rank: 1,
                speaker_name: "Alice".into(),
                text: "second".into(),
            }),
            ChatLine::Private {
                speaker_name: "Bob".into(),
                text: "third".into(),
            },
        ],
        ..Default::default()
    };
    for state in [
        GuildRoomState::Absent,
        GuildRoomState::Unavailable,
        GuildRoomState::Member {
            guild_id: [2; 16],
            rank: 1,
            motd: "".into(),
        },
    ] {
        panel.guild = state;
        assert!(!panel_text(&panel).contains("second"));
    }
    panel.guild = GuildRoomState::Member {
        guild_id: [1; 16],
        rank: 1,
        motd: "".into(),
    };
    let text = panel_text(&panel);
    let positions: Vec<_> = ["first", "second", "third"]
        .iter()
        .map(|word| text.find(word))
        .collect();
    assert!(positions.iter().all(Option::is_some));
    assert!(positions.windows(2).all(|p| p[0] < p[1]));
}

#[test]
fn console_tabs_keep_independent_filters_and_gate_composition() {
    use oteryn_client_runtime::chat_public_channels::ChatRoom;
    let mut panel = ChatPanel {
        selected: true,
        text: "hello".into(),
        ..Default::default()
    };
    panel.lines = vec![
        ChatLine::Private {
            speaker_name: "Alice".into(),
            text: "private hello".into(),
        },
        ChatLine::Room {
            room: ChatRoom::World,
            speaker_name: "Bob".into(),
            text: "world hello".into(),
        },
    ];
    panel.console.tab = ConsoleTab::Private;
    panel.console.filter_mut().contains = "Alice".into();
    assert!(panel_text(&panel).contains("private hello"));
    assert!(!panel_text(&panel).contains("world hello"));
    panel.console.tab = ConsoleTab::Public(ChatRoom::World);
    assert!(panel.console.filter_mut().contains.is_empty());
    assert!(panel_text(&panel).contains("world hello"));
    panel.console.tab = ConsoleTab::Private;
    assert_eq!(panel.console.filter_mut().contains, "Alice");
    panel.console.mode = ComposerMode::Off;
    assert!(request(&mut panel, Mode::Say, "").is_none());
    panel.console.mode = ComposerMode::Temporary;
    assert!(request(&mut panel, Mode::Say, "Alice").is_some());
    assert_eq!(panel.console.mode, ComposerMode::Off);
}

#[test]
fn console_tab_sends_to_the_selected_server_room() {
    use oteryn_client_runtime::chat_public_channels::ChatRoom;
    let mut panel = ChatPanel {
        selected: true,
        text: "hello".into(),
        ..Default::default()
    };
    panel.console.tab = ConsoleTab::Public(ChatRoom::Help);
    assert_eq!(
        request(&mut panel, Mode::Say, ""),
        Some(ChatIntent::Room {
            room: ChatRoom::Help,
            text: "hello".into()
        })
    );
    panel.pending = false;
    panel.console.tab = ConsoleTab::Party;
    assert_eq!(
        request(&mut panel, Mode::Say, ""),
        Some(ChatIntent::Party {
            text: "hello".into()
        })
    );
}

#[test]
fn communication_lists_filter_without_destroying_confirmed_history() -> Result<(), &'static str> {
    let mut panel = ChatPanel {
        lines: vec![ChatLine::Private {
            speaker_name: "Alice".into(),
            text: "keep history".into(),
        }],
        ..Default::default()
    };
    panel.console.communication = CommunicationRules::parse(" alice ", "", "", true, false)?;
    assert!(!panel_text(&panel).contains("keep history"));
    assert_eq!(panel.lines.len(), 1);
    panel.console.communication = CommunicationRules::parse("ALICE", "Alice", "alice", true, true)?;
    assert!(panel_text(&panel).contains("keep history"));
    panel.console.communication = CommunicationRules::parse("", "", "ALICE", false, false)?;
    assert!(!panel_text(&panel).contains("keep history"));
    assert!(CommunicationRules::parse(&"x".repeat(121), "", "", false, false).is_err());
    Ok(())
}
