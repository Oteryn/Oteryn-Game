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

mod channels {
    use super::*;
    use crate::chat_private_channel::{ChatPanel as PrivatePanel, Intent as PrivateIntent};
    use oteryn_client_runtime::chat_private_channel::{
        ChatChannelList, ChatDisposition, ChatOutcome, InvitedChannel, PrivateChannelIntent,
        PrivateChannelLine,
    };
    use oteryn_client_runtime::chat_public_channels::{ChatRoom, ChatRoomSet};

    fn selected() -> (ChatPanel, ChatPanel, PrivatePanel) {
        let chat = ChatPanel {
            selected: true,
            ..ChatPanel::default()
        };
        let public = ChatPanel {
            selected: true,
            ..ChatPanel::default()
        };
        let private = PrivatePanel {
            selected: true,
            ..PrivatePanel::default()
        };
        (chat, public, private)
    }
    fn rooms(open: &[ChatRoom]) -> ChatRoomSet {
        let mut set = ChatRoomSet::default();
        for room in open {
            set.insert(*room);
        }
        set
    }
    fn outcome(disposition: ChatDisposition) -> ChatOutcome {
        ChatOutcome {
            command_id: 1,
            status: oteryn_client_runtime::CommandStatus::Accepted,
            disposition,
            wait_seconds: 0,
            result_server_sequence: 1,
        }
    }
    fn list(own: Option<u16>, invited: &[(u16, &str)]) -> ChatChannelList {
        ChatChannelList {
            public_rooms: rooms(&[ChatRoom::World, ChatRoom::Help]),
            own_channel: own,
            invited: invited
                .iter()
                .map(|(channel_id, owner)| InvitedChannel {
                    channel_id: *channel_id,
                    owner_name: (*owner).into(),
                })
                .collect(),
        }
    }

    #[test]
    fn chat_channels_help_tab_opens_when_the_server_opens_the_room() {
        let (mut chat, mut public, _) = selected();
        assert_eq!(
            open_help(&mut chat, &mut public),
            Some(ChatIntent::OpenRoom(ChatRoom::Help))
        );
        assert_eq!(chat.console.current, None);
        public.rooms = rooms(&[ChatRoom::Help]);
        chat.console.sync_rooms(public.rooms);
        assert_eq!(chat.console.current, Some(ChannelTab::Room(ChatRoom::Help)));
        assert_eq!(
            chat.console.tab_labels().last().map(String::as_str),
            Some("Help")
        );
        // Already open: no wire call, the tab is only selected again.
        chat.console.select(ConsoleTab::All);
        assert_eq!(open_help(&mut chat, &mut public), None);
        assert_eq!(chat.console.current, Some(ChannelTab::Room(ChatRoom::Help)));
    }

    #[test]
    fn chat_channels_close_current_asks_the_server_and_keeps_fixed_tabs() {
        let (mut chat, mut public, mut private) = selected();
        // The default (fixed) tabs cannot be closed.
        assert_eq!(close_current(&mut chat, &mut public, &mut private), None);
        assert_eq!(chat.console.tab_count(), 4);
        public.rooms = rooms(&[ChatRoom::World, ChatRoom::Help]);
        chat.console.sync_rooms(public.rooms);
        chat.console.select_index(5);
        assert_eq!(chat.console.current, Some(ChannelTab::Room(ChatRoom::Help)));
        assert_eq!(
            close_current(&mut chat, &mut public, &mut private),
            Some(ChatIntent::CloseRoom(ChatRoom::Help))
        );
        public.rooms = rooms(&[ChatRoom::World]);
        chat.console.sync_rooms(public.rooms);
        assert_eq!(
            chat.console.current,
            Some(ChannelTab::Room(ChatRoom::World))
        );
        assert_eq!(chat.console.tab_count(), 5);
    }

    #[test]
    fn chat_channels_cycle_wraps_across_fixed_and_opened_tabs() {
        let mut console = Console::default();
        console.open_channel(ChannelTab::Conversation("Bryn".into()));
        console.open_channel(ChannelTab::Private(101));
        console.select(ConsoleTab::Guild);
        console.cycle(true);
        assert_eq!(
            console.current,
            Some(ChannelTab::Conversation("Bryn".into()))
        );
        console.cycle(true);
        assert_eq!(console.current, Some(ChannelTab::Private(101)));
        console.cycle(true);
        assert_eq!(
            (console.current.clone(), console.tab),
            (None, ConsoleTab::All)
        );
        console.cycle(false);
        assert_eq!(console.current, Some(ChannelTab::Private(101)));
        assert_eq!(
            console.tab_labels(),
            ["All", "Local", "Private", "Guild", "Bryn", "Channel 101"]
        );
        // Closing the left-most opened tab falls back to the fixed tab.
        console.select_index(4);
        assert_eq!(
            console.close_current(),
            Some(ChannelTab::Conversation("Bryn".into()))
        );
        assert_eq!(console.current, None);
    }

    #[test]
    fn chat_channels_list_offers_rooms_own_channel_and_invitations() {
        let (mut chat, mut public, mut private) = selected();
        let channels = list(None, &[(205, "Ana")]);
        let entries = channel_entries(Some(&channels), ChatRoomSet::default());
        assert_eq!(
            entries,
            [
                ChannelEntry::Room(ChatRoom::World),
                ChannelEntry::Room(ChatRoom::Help),
                ChannelEntry::CreateOwn,
                ChannelEntry::Invitation {
                    channel_id: 205,
                    owner_name: "Ana".into()
                },
            ]
        );
        let text = channel_list_text(&entries, &chat.console);
        assert!(text.contains("World: closed") && text.contains("Invitation: Ana (205)"));
        // Joining an invitation opens its tab only after the server accepted the join.
        assert_eq!(
            open_entry(&mut chat, &mut public, &mut private, &entries[3]),
            Some(ChatIntent::PrivateChannel(PrivateChannelIntent::Join {
                channel_id: 205
            }))
        );
        assert!(chat.console.channels.is_empty());
        private_result(
            &mut chat,
            private.pending_intent,
            private.channel,
            &outcome(ChatDisposition::Ok),
        );
        assert_eq!(chat.console.current, Some(ChannelTab::Private(205)));
        assert!(channel_list_text(&entries, &chat.console).contains("World: closed"));
        // The invitation withdrawn: the tab goes away with the list.
        chat.console.sync_private(Some(&list(None, &[])));
        assert_eq!(chat.console.current, None);
        assert!(chat.console.channels.is_empty());
    }

    #[test]
    fn chat_channels_create_own_channel_selects_it_when_the_list_arrives() {
        let (mut chat, mut public, mut private) = selected();
        assert_eq!(
            open_entry(
                &mut chat,
                &mut public,
                &mut private,
                &ChannelEntry::CreateOwn
            ),
            Some(ChatIntent::PrivateChannel(PrivateChannelIntent::Create))
        );
        private_result(
            &mut chat,
            private.pending_intent,
            private.channel,
            &outcome(ChatDisposition::Ok),
        );
        chat.console.sync_private(Some(&list(Some(100), &[])));
        assert_eq!(chat.console.current, Some(ChannelTab::Private(100)));
        // Refusal (no Premium): no tab, no stale focus.
        let (mut chat, mut public, mut private) = selected();
        assert!(
            open_entry(
                &mut chat,
                &mut public,
                &mut private,
                &ChannelEntry::CreateOwn
            )
            .is_some()
        );
        private_result(
            &mut chat,
            private.pending_intent,
            private.channel,
            &outcome(ChatDisposition::NotAllowed),
        );
        chat.console.sync_private(Some(&list(Some(100), &[])));
        assert_eq!(chat.console.current, None);
        // Closing the own channel leaves it on the wire.
        chat.console.open_channel(ChannelTab::Private(100));
        private.pending = false;
        assert_eq!(
            close_current(&mut chat, &mut public, &mut private),
            Some(ChatIntent::PrivateChannel(PrivateChannelIntent::Leave {
                channel_id: 100
            }))
        );
        private_result(
            &mut chat,
            Some(PrivateIntent::Leave),
            Some(100),
            &outcome(ChatDisposition::Ok),
        );
        assert!(chat.console.channels.is_empty());
    }

    #[test]
    fn chat_channels_each_tab_shows_only_its_own_lines_and_sends_to_it() {
        let (mut chat, _, _) = selected();
        chat.lines = vec![
            ChatLine::Room {
                room: ChatRoom::Help,
                speaker_name: "Tutor".into(),
                text: "welcome".into(),
            },
            ChatLine::Room {
                room: ChatRoom::World,
                speaker_name: "Ana".into(),
                text: "worldline".into(),
            },
            ChatLine::PrivateChannel(PrivateChannelLine {
                channel_id: 100,
                speaker_name: "Cleo".into(),
                text: "secret".into(),
            }),
            ChatLine::Private {
                speaker_name: "Bryn".into(),
                text: "psst".into(),
            },
        ];
        let mut shown = |tab: ChannelTab| {
            chat.console.open_channel(tab);
            panel_text(&chat)
        };
        let help = shown(ChannelTab::Room(ChatRoom::Help));
        assert!(help.contains("welcome") && !help.contains("worldline") && !help.contains("psst"));
        let channel = shown(ChannelTab::Private(100));
        assert!(channel.contains("secret") && !channel.contains("welcome"));
        let pm = shown(ChannelTab::Conversation("bryn".into()));
        assert!(pm.contains("psst") && !pm.contains("secret"));
        assert!(!pm.contains("Tutor"));

        chat.text = "hello".into();
        assert_eq!(
            channel_request(&mut chat),
            Some(ChatIntent::Private {
                recipient_name: "bryn".into(),
                text: "hello".into()
            })
        );
        chat.pending = false;
        chat.console.open_channel(ChannelTab::Private(100));
        assert_eq!(
            channel_request(&mut chat),
            Some(ChatIntent::PrivateChannel(PrivateChannelIntent::Say {
                channel_id: 100,
                text: "hello".into()
            }))
        );
        chat.pending = false;
        chat.console.open_channel(ChannelTab::Room(ChatRoom::Help));
        assert_eq!(
            channel_request(&mut chat),
            Some(ChatIntent::Room {
                room: ChatRoom::Help,
                text: "hello".into()
            })
        );
        chat.pending = false;
        chat.console.select(ConsoleTab::All);
        assert_eq!(channel_request(&mut chat), None);
    }

    #[test]
    fn chat_channels_conversation_tab_opens_locally_and_closes_without_wire() {
        let (mut chat, mut public, mut private) = selected();
        assert!(!open_conversation(&mut chat, "  "));
        assert!(open_conversation(&mut chat, " Bryn "));
        assert_eq!(
            chat.console.current,
            Some(ChannelTab::Conversation("Bryn".into()))
        );
        assert_eq!(close_current(&mut chat, &mut public, &mut private), None);
        assert!(chat.console.channels.is_empty());
    }

    #[test]
    fn chat_local_tabs_open_from_the_dialog_rows_and_close_without_wire() {
        let (mut chat, mut public, mut private) = selected();
        let entries = dialog_entries(Some(&list(None, &[])), ChatRoomSet::default());
        let local: Vec<_> = entries
            .iter()
            .filter(|entry| matches!(entry, ChannelEntry::Local(_)))
            .cloned()
            .collect();
        assert_eq!(
            local,
            [
                ChannelEntry::Local(ChannelTab::Loot),
                ChannelEntry::Local(ChannelTab::ServerLog),
                ChannelEntry::Local(ChannelTab::Npc),
            ]
        );
        for entry in &local {
            assert_eq!(
                open_entry(&mut chat, &mut public, &mut private, entry),
                None
            );
        }
        assert_eq!(
            chat.console.tab_labels()[4..],
            ["Loot", "Server Log", "NPC"]
        );
        assert_eq!(chat.console.current, Some(ChannelTab::Npc));
        // Typing in a local tab speaks locally; nothing is sent to a channel.
        chat.text = "hi".into();
        assert!(matches!(
            channel_request(&mut chat),
            Some(ChatIntent::Say { .. })
        ));
        assert_eq!(close_current(&mut chat, &mut public, &mut private), None);
        assert_eq!(chat.console.channels.len(), 2);
        assert!(dialog_rows(&entries, &chat.console)[entries.len() - 1].contains("closed"));
    }

    #[test]
    fn chat_loot_tab_takes_loot_notices_out_of_the_server_log_and_all_tabs() {
        let mut chat = ChatPanel::default();
        chat.console
            .notices
            .push(NoticeClass::Loot, "Looted 2 of 3 items", 1);
        chat.console
            .notices
            .push(NoticeClass::Status, "Server is saving", 2);
        let log = |chat: &ChatPanel| {
            let mut shown = chat.clone();
            shown.console.current = Some(ChannelTab::ServerLog);
            panel_text(&shown)
        };
        // Without a Loot tab the loot line stays in the combined log.
        chat.console.open_channel(ChannelTab::ServerLog);
        assert!(log(&chat).contains("Looted 2 of 3 items") && log(&chat).contains("saving"));
        chat.console.open_channel(ChannelTab::Loot);
        let loot = panel_text(&chat);
        assert!(loot.contains("Looted 2 of 3 items") && !loot.contains("saving"));
        let log = log(&chat);
        assert!(!log.contains("Looted") && log.contains("Server is saving"));
        chat.console.select(ConsoleTab::All);
        assert!(!panel_text(&chat).contains("Looted"));
    }

    #[test]
    fn chat_npc_tab_keeps_the_last_conversation_lines() {
        let mut chat = ChatPanel::default();
        chat.console.open_channel(ChannelTab::Npc);
        chat.console
            .sync_npc(&["Hello, traveller.".into(), "Buy?".into()]);
        assert_eq!(panel_text(&chat), "Hello, traveller.\nBuy?\n");
        chat.console.sync_npc(&[]);
        assert!(panel_text(&chat).contains("Buy?"));
        chat.console.sync_npc(&["Farewell.".into()]);
        assert_eq!(panel_text(&chat), "Farewell.\n");
    }

    #[test]
    fn chat_dialog_selects_steps_and_confirms_rows_or_a_typed_name() {
        let channels = list(Some(100), &[(205, "Ana")]);
        let entries = dialog_entries(Some(&channels), ChatRoomSet::default());
        let console = Console::default();
        let rows = dialog_rows(&entries, &console);
        assert_eq!(rows.len(), entries.len());
        assert!(rows[0].starts_with("World") && rows.iter().any(|row| row.contains("Ana")));
        let mut dialog = ChannelDialog::default();
        dialog.show();
        assert!(dialog.open && dialog.selected == 0);
        assert!(!dialog.select(entries.len(), entries.len()));
        dialog.step(false, entries.len());
        assert_eq!(dialog.selected, entries.len() - 1);
        dialog.step(true, entries.len());
        assert_eq!(dialog.selected, 0);
        dialog.step(true, 0);
        assert!(dialog.select(1, entries.len()));
        assert_eq!(
            dialog.confirm(&entries),
            Some(DialogAction::Entry(ChannelEntry::Room(ChatRoom::Help)))
        );
        assert!(!dialog.open);
        // A typed name wins over the selection and opens a conversation.
        dialog.show();
        dialog.set_name("  Bryn ");
        assert_eq!(
            dialog.confirm(&entries),
            Some(DialogAction::Conversation("Bryn".into()))
        );
        // A hidden dialog confirms nothing; no rows and no name: nothing to confirm either.
        assert_eq!(dialog.confirm(&entries), None);
        dialog.show();
        assert_eq!(dialog.confirm(&[]), None);
        // The create row is captioned as the owner asked.
        let create = dialog_rows(&[ChannelEntry::CreateOwn], &console);
        assert_eq!(create, ["Create own private channel"]);
    }
}
