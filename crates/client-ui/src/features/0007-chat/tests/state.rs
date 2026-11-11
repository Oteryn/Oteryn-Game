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
fn console_tabs_have_independent_filters_and_temporary_chat_restores_off() {
    let mut panel = ChatPanel::default();
    panel.console.select(ConsoleTab::Private);
    panel.console.show_messages = false;
    panel.console.select(ConsoleTab::Local);
    assert!(panel.console.show_messages);
    panel.console.select(ConsoleTab::Private);
    assert!(!panel.console.show_messages);
    panel.console.mode = ComposerMode::Off;
    panel.console.temporary_on();
    assert!(panel.console.can_send());
    panel.console.sent();
    assert!(!panel.console.can_send());
    panel.selected = true;
    panel.text = "hello".into();
    assert!(request(&mut panel, Mode::Say, "").is_none());
}

#[test]
fn console_filter_changes_projection_without_removing_history() {
    let mut panel = ChatPanel {
        lines: vec![ChatLine::Private {
            speaker_name: "Alice".into(),
            text: "secret".into(),
        }],
        ..Default::default()
    };
    panel.console.select(ConsoleTab::Local);
    assert!(!panel_text(&panel).contains("secret"));
    panel.console.select(ConsoleTab::Private);
    assert!(panel_text(&panel).contains("secret"));
    panel.console.show_messages = false;
    assert!(!panel_text(&panel).contains("secret"));
    assert_eq!(panel.lines.len(), 1);
}

#[test]
fn console_metadata_query_is_bounded() {
    let mut panel = ChatPanel {
        selected: true,
        ..Default::default()
    };
    panel.console.display.levels = true;
    panel.lines = (0..40)
        .map(|i| ChatLine::Private {
            speaker_name: format!("Speaker {i}"),
            text: "Hello".into(),
        })
        .collect();
    let payload = console_request(&mut panel);
    assert_eq!(
        payload
            .as_ref()
            .and_then(|bytes| oteryn_client_runtime::chat::console::decode_names(bytes))
            .map(|names| names.len()),
        Some(32)
    );
    assert!(console_request(&mut panel).is_none());
    panel.console.metadata.pending = None;
    let next = console_request(&mut panel)
        .and_then(|bytes| oteryn_client_runtime::chat::console::decode_names(&bytes));
    assert!(next.is_some_and(|names| names.iter().any(|name| name == "Speaker 0")));
}

#[test]
fn console_receipt_times_are_stable_and_seconds_are_optional() {
    let lines = vec![ChatLine::Private {
        speaker_name: "Bryn".into(),
        text: "Hello".into(),
    }];
    let mut metadata = Metadata::default();
    metadata.observe(&lines, 3661);
    metadata.observe(&lines, 7200);
    let mut display = ConsoleDisplay {
        time: true,
        ..Default::default()
    };
    assert_eq!(metadata.prefix(display, 0), "[01:01 UTC] ");
    display.seconds = true;
    assert_eq!(metadata.prefix(display, 0), "[01:01:01 UTC] ");
    display.time = false;
    assert_eq!(metadata.prefix(display, 0), "");
}

#[test]
fn npc_speech_never_inherits_a_players_current_level() -> Result<(), &'static str> {
    let mut panel = ChatPanel::default();
    panel.console.display.levels = true;
    panel
        .console
        .metadata
        .levels
        .insert("Forest crier".into(), 99);
    panel.lines.push(ChatLine::Local {
        speaker: oteryn_client_runtime::chat::ChatSpeaker {
            identity: [1; 16],
            generation: std::num::NonZeroU64::new(1).ok_or("generation")?,
        },
        speaker_name: "Forest crier".into(),
        mode: ChatSpeechMode::MonsterSay,
        text: "The paths are open.".into(),
        position: oteryn_client_runtime::chat::ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
    });
    assert!(panel_text(&panel).contains("Forest crier: The paths are open."));
    assert!(!panel_text(&panel).contains("current level"));
    assert!(console_request(&mut panel).is_none());
    Ok(())
}

#[test]
fn identical_messages_at_log_capacity_keep_new_receipt_times() {
    let line = ChatLine::Private {
        speaker_name: "Bryn".into(),
        text: "Hello".into(),
    };
    let max = oteryn_client_runtime::chat::MAX_CHAT_LOG_LINES;
    let mut metadata = Metadata::default();
    for _ in 0..max {
        metadata.received(&line, 10);
    }
    let lines = vec![line.clone(); max];
    metadata.observe(&lines, 99);
    metadata.received(&line, 20);
    metadata.observe(&lines, 100);
    let display = ConsoleDisplay {
        time: true,
        seconds: true,
        ..Default::default()
    };
    assert_eq!(metadata.prefix(display, 0), "[00:00:10 UTC] ");
    assert_eq!(metadata.prefix(display, max - 1), "[00:00:20 UTC] ");
}

#[test]
fn n021_default_channel_and_temporary_mode_keys() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let key =
        |code: u16, modifiers| -> Result<[NormalizedInputEvent; 1], Box<dyn std::error::Error>> {
            Ok([NormalizedInputEvent::Key {
                code: KeyCode::new(code)?,
                state: ButtonState::Pressed,
                modifiers,
                repeat: false,
            }])
        };
    assert!(crate::chat::default_channel_key(&key(
        7,
        Modifiers::one(Modifier::Alt)
    )?));
    assert!(!crate::chat::default_channel_key(&key(7, Modifiers::NONE)?));
    assert!(crate::chat::temporary_mode_key(&key(88, Modifiers::NONE)?));
    assert!(!crate::chat::temporary_mode_key(&key(
        88,
        Modifiers::one(Modifier::Alt)
    )?));
    Ok(())
}
