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
