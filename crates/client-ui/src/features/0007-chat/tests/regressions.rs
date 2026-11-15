use crate::chat::*;
use oteryn_client_runtime::chat::{ChatIntent, ChatLine, ChatSpeechMode};

#[test]
fn tibia_prefixes_override_fixed_and_channel_destinations() {
    use oteryn_client_runtime::chat_public_channels::ChatRoom;
    for channel in [
        None,
        Some(ChannelTab::Room(ChatRoom::Help)),
        Some(ChannelTab::Private(100)),
        Some(ChannelTab::Conversation("Bob".into())),
        Some(ChannelTab::Loot),
        Some(ChannelTab::ServerLog),
        Some(ChannelTab::Npc),
    ] {
        for (text, expected) in [
            (
                "*Alice* secret",
                ChatIntent::Private {
                    recipient_name: "Alice".into(),
                    text: "secret".into(),
                },
            ),
            (
                "@Alice@ secret",
                ChatIntent::Private {
                    recipient_name: "Alice".into(),
                    text: "secret".into(),
                },
            ),
            (
                "#w quiet",
                ChatIntent::Say {
                    mode: ChatSpeechMode::Whisper,
                    text: "quiet".into(),
                },
            ),
            (
                "#y loud",
                ChatIntent::Say {
                    mode: ChatSpeechMode::Yell,
                    text: "loud".into(),
                },
            ),
            (
                "#s hello",
                ChatIntent::Say {
                    mode: ChatSpeechMode::Say,
                    text: "hello".into(),
                },
            ),
        ] {
            let mut panel = ChatPanel {
                selected: true,
                text: text.into(),
                ..Default::default()
            };
            panel.console.current = channel.clone();
            let actual = if channel.is_some() {
                channel_request(&mut panel)
            } else {
                request(&mut panel, Mode::Say, "")
            };
            assert_eq!(actual, Some(expected), "{text} in {channel:?}");
            assert!(panel.pending);
        }
    }
}

#[test]
fn incomplete_private_and_empty_speech_prefixes_never_broadcast() {
    for text in [
        "*Alice",
        "@Alice",
        "** secret",
        "@@ secret",
        "*Alice*  ",
        "#w ",
        "#y",
        "#s ",
    ] {
        for channel in [false, true] {
            let mut panel = ChatPanel {
                selected: true,
                text: text.into(),
                ..Default::default()
            };
            if channel {
                panel.console.open_channel(ChannelTab::Private(100));
            }
            let intent = if channel {
                channel_request(&mut panel)
            } else {
                request(&mut panel, Mode::Say, "")
            };
            assert_eq!(intent, None, "{text}");
            assert!(!panel.pending);
        }
    }
}

#[test]
fn opened_tabs_keep_independent_message_visibility() {
    use oteryn_client_runtime::chat_public_channels::ChatRoom;
    let mut panel = ChatPanel {
        lines: vec![
            ChatLine::Room {
                room: ChatRoom::Help,
                speaker_name: "Tutor".into(),
                text: "help message".into(),
            },
            ChatLine::Private {
                speaker_name: "Alice".into(),
                text: "private message".into(),
            },
        ],
        ..Default::default()
    };
    panel.lines.push(ChatLine::PrivateChannel(
        oteryn_client_runtime::chat_private_channel::PrivateChannelLine {
            channel_id: 100,
            speaker_name: "Bob".into(),
            text: "channel message".into(),
        },
    ));
    panel
        .console
        .notices
        .push(NoticeClass::Loot, "loot message", 1);
    panel
        .console
        .notices
        .push(NoticeClass::Status, "server message", 2);
    panel.console.npc_lines = vec!["npc message".into()];
    let tabs = [
        ChannelTab::Room(ChatRoom::Help),
        ChannelTab::Conversation("Alice".into()),
        ChannelTab::Npc,
        ChannelTab::Private(100),
        ChannelTab::Loot,
        ChannelTab::ServerLog,
    ];
    for (index, tab) in tabs.iter().enumerate() {
        panel.console.open_channel(tab.clone());
        assert!(panel.console.show_messages);
        panel.console.show_messages = false;
        assert!(!panel_text(&panel).contains("message"));
        panel.console.select(ConsoleTab::Private);
        assert!(panel.console.show_messages);
        panel.console.select_index(4 + index);
        assert!(!panel.console.show_messages);
        assert!(!panel_text(&panel).contains("message"));
        panel.console.select(ConsoleTab::All);
        assert!(panel.console.show_messages);
    }
    panel.console.select(ConsoleTab::Private);
    panel.console.show_messages = false;
    for tab in tabs {
        panel.console.open_channel(tab);
        panel.console.show_messages = true;
        assert!(panel_text(&panel).contains("message"));
        panel.console.select(ConsoleTab::Private);
        assert!(!panel.console.show_messages);
    }
}

#[test]
fn channel_visibility_survives_focus_close_cycle_and_mirror() {
    use oteryn_client_runtime::chat_public_channels::{ChatRoom, ChatRoomSet};
    let mut console = Console::default();
    console.show_messages = false;
    console.open_channel(ChannelTab::Conversation("Alice".into()));
    assert!(console.show_messages);
    console.show_messages = false;
    console.open_channel(ChannelTab::Room(ChatRoom::Help));
    assert!(console.show_messages);
    console.show_messages = false;
    console.focus = Some(PendingFocus::Tab(ChannelTab::Room(ChatRoom::World)));
    let mut rooms = ChatRoomSet::default();
    rooms.insert(ChatRoom::Help);
    rooms.insert(ChatRoom::World);
    console.sync_rooms(rooms);
    assert!(console.show_messages);
    console.cycle(false);
    assert_eq!(console.current, Some(ChannelTab::Room(ChatRoom::Help)));
    assert!(!console.show_messages);
    console.close_current();
    assert_eq!(
        console.current,
        Some(ChannelTab::Conversation("Alice".into()))
    );
    assert!(!console.show_messages);
    console.open_channel(ChannelTab::Conversation("ALICE".into()));
    assert!(!console.show_messages);
    assert!(!console.shows_messages(ConsoleTab::All));
    console.close_current();
    assert_eq!(console.current, None);
    assert!(!console.show_messages);
    console.open_channel(ChannelTab::Conversation("Alice".into()));
    assert!(console.show_messages);
    console.mirror = Some(ConsoleTab::All);
    let panel = ChatPanel {
        open: true,
        console,
        lines: vec![ChatLine::Private {
            speaker_name: "Alice".into(),
            text: "mirror message".into(),
        }],
        ..Default::default()
    };
    assert!(panel_text(&panel).contains("mirror message"));
    assert!(mirror_text(&panel).is_some_and(|text| !text.contains("mirror message")));
}
