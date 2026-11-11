use crate::play::play_channel;
use oteryn_client_ui::chat::{ConsoleDisplay, panel_text};
use oteryn_session::{ChatLine, ChatRoomSet, CommandStatus, GatedCommandResult};
#[test]
fn console_controls_reach_received_lines_and_current_server_levels()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.chat_initialize(true);
    outbox.chat_public_state(
        ChatRoomSet::default(),
        vec![ChatLine::Private {
            speaker_name: "Bryn".into(),
            text: "Hello".into(),
        }],
    );
    link.console_display(ConsoleDisplay {
        font_px: 20,
        time: true,
        seconds: true,
        levels: true,
    })?;
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("metadata command".into());
    };
    assert_eq!((command.capability, command.kind), (7, 6117));
    assert_eq!(
        oteryn_client_runtime::chat::console::decode_names(&command.payload),
        Some(vec!["Bryn".into()])
    );
    outbox.console_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: oteryn_client_runtime::chat::console::encode_levels(&[Some(42)])
            .ok_or("encode")?,
    });
    let panel = link.chat_panel();
    assert_eq!(panel.console.display.font_px, 20);
    let text = panel_text(&panel);
    assert!(text.contains("Bryn [current level 42]: Hello"));
    assert!(text.lines().any(|line| line.starts_with('[')
        && line.as_bytes().get(3) == Some(&b':')
        && line.as_bytes().get(6) == Some(&b':')));
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let open = NormalizedInputEvent::Key {
        code: KeyCode::new(6)?,
        state: ButtonState::Pressed,
        repeat: false,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
    };
    assert!(link.chat_input(&[open]));
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Feature(command) if command.kind == 6117)
    );
    outbox.console_result(&GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Rejected,
        payload: Vec::new(),
    });
    assert!(panel_text(&link.chat_panel()).contains("[Private] Bryn: Hello"));
    link.console_display(ConsoleDisplay::default())?;
    assert!(panel_text(&link.chat_panel()).contains("[Private] Bryn: Hello"));
    assert!(
        link.console_display(ConsoleDisplay {
            font_px: 200,
            ..Default::default()
        })
        .is_err()
    );
    Ok(())
}

#[test]
fn chat_channels_help_tab_opens_sends_and_closes_through_the_session_callers()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_client_ui::chat::ChannelTab;
    use oteryn_session::{ChatIntent, ChatRoom};
    let (link, mut commands, outbox) = play_channel();
    outbox.chat_initialize(true);
    outbox.chat_public_initialize(true, Some((ChatRoomSet::default(), Vec::new())));

    // OpenHelpChannel asks the server; the tab appears once the room is reported open.
    link.chat_open_help();
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::ChatPublic(ChatIntent::OpenRoom(ChatRoom::Help))
    ));
    assert_eq!(link.chat_panel().console.current, None);
    let mut open = ChatRoomSet::default();
    open.insert(ChatRoom::Help);
    outbox.chat_public_result(
        &ChatIntent::OpenRoom(ChatRoom::Help),
        &oteryn_session::ChatOutcome {
            command_id: 1,
            status: CommandStatus::Accepted,
            disposition: oteryn_session::ChatDisposition::Ok,
            wait_seconds: 0,
            result_server_sequence: 1,
        },
    );
    outbox.chat_public_state(open, Vec::new());
    assert_eq!(
        link.chat_panel().console.current,
        Some(ChannelTab::Room(ChatRoom::Help))
    );

    // The console composer now speaks into Help.
    link.chat_text("hello help".into());
    link.chat_request(oteryn_client_ui::chat::Mode::Say, "");
    // Typing indicators may precede the chat command; only the chat command is asserted.
    let sent = std::iter::from_fn(|| commands.try_recv().ok())
        .find_map(|command| match command {
            PlayCommand::ChatPublic(intent) => Some(intent),
            _ => None,
        })
        .ok_or("no chat command")?;
    assert_eq!(
        sent,
        ChatIntent::Room {
            room: ChatRoom::Help,
            text: "hello help".into()
        }
    );

    // CloseCurrentChannel leaves the room; the tab goes when the server reports it closed.
    link.chat_close_current();
    assert!(matches!(
        commands.try_recv()?,
        PlayCommand::ChatPublic(ChatIntent::CloseRoom(ChatRoom::Help))
    ));
    outbox.chat_public_state(ChatRoomSet::default(), Vec::new());
    assert_eq!(link.chat_panel().console.current, None);
    assert_eq!(link.chat_panel().console.tab_count(), 4);
    Ok(())
}

#[test]
fn chat_channels_private_message_tab_cycles_and_closes_locally()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_ui::chat::ChannelTab;
    let (link, mut commands, outbox) = play_channel();
    outbox.chat_initialize(true);
    assert!(link.chat_open_conversation("Bryn"));
    assert_eq!(
        link.chat_panel().console.current,
        Some(ChannelTab::Conversation("Bryn".into()))
    );
    link.chat_channel_cycle(true);
    assert_eq!(link.chat_panel().console.current, None);
    link.chat_channel_cycle(false);
    link.chat_close_current();
    assert!(link.chat_panel().console.channels.is_empty());
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn channel_management_results_preserve_composer_and_submitted_channel()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::mailbox::lock;
    use oteryn_client_runtime::chat_private_channel::PrivateChannelIntent as Private;
    use oteryn_client_ui::chat::ChannelTab;
    use oteryn_session::ChatIntent;
    let (link, _commands, outbox) = play_channel();
    outbox.chat_initialize(true);
    let outcome = oteryn_session::ChatOutcome {
        command_id: 1,
        status: CommandStatus::Accepted,
        disposition: oteryn_session::ChatDisposition::Ok,
        wait_seconds: 0,
        result_server_sequence: 1,
    };
    for join in [true, false] {
        {
            let mut m = lock(&link.mailbox);
            m.chat.pending = true;
            m.chat.text = "pending composer".into();
            m.chat_private.channel = Some(22);
            m.chat_private.pending_intent = Some(if join {
                oteryn_client_ui::chat_private_channel::Intent::Join
            } else {
                oteryn_client_ui::chat_private_channel::Intent::Leave
            });
        }
        let intent = ChatIntent::PrivateChannel(if join {
            Private::Join { channel_id: 11 }
        } else {
            Private::Leave { channel_id: 11 }
        });
        outbox.chat_result(&intent, &outcome);
        let panel = link.chat_panel();
        assert_eq!(
            panel.console.channels.contains(&ChannelTab::Private(11)),
            join
        );
        assert!(!panel.console.channels.contains(&ChannelTab::Private(22)));
        assert!(panel.pending);
        assert_eq!(panel.text, "pending composer");
    }
    outbox.chat_result(
        &ChatIntent::OpenRoom(oteryn_session::ChatRoom::Help),
        &outcome,
    );
    assert!(link.chat_panel().pending);
    outbox.chat_result(
        &ChatIntent::Room {
            room: oteryn_session::ChatRoom::Help,
            text: "pending composer".into(),
        },
        &outcome,
    );
    assert!(!link.chat_panel().pending);
    crate::play::mailbox::lock(&link.mailbox).chat.pending = true;
    outbox.chat_result(
        &ChatIntent::PrivateChannel(Private::Say {
            channel_id: 11,
            text: "pending composer".into(),
        }),
        &outcome,
    );
    assert!(!link.chat_panel().pending);
    Ok(())
}

#[test]
fn legacy_help_opens_and_closes_without_public_channels_capability()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::combat_input::PlayCommand;
    use oteryn_session::{ChatIntent, ChatRoom};
    for room in [
        ChatRoom::World,
        ChatRoom::English,
        ChatRoom::Help,
        ChatRoom::Advertising,
    ] {
        let (link, mut commands, outbox) = play_channel();
        outbox.chat_initialize(true);
        outbox.chat_public_initialize(false, None);
        if room == ChatRoom::Help {
            link.chat_open_help();
        } else {
            link.chat_open_entry(&oteryn_client_ui::chat::ChannelEntry::Room(room));
        }
        assert_eq!(
            commands.try_recv()?,
            PlayCommand::ChatPublic(ChatIntent::OpenRoom(room))
        );
        let mut rooms = ChatRoomSet::default();
        rooms.insert(room);
        outbox.chat_result(
            &ChatIntent::OpenRoom(room),
            &oteryn_session::ChatOutcome {
                command_id: 1,
                status: CommandStatus::Accepted,
                disposition: oteryn_session::ChatDisposition::Ok,
                wait_seconds: 0,
                result_server_sequence: 1,
            },
        );
        outbox.chat_public_state(rooms, Vec::new());
        link.chat_close_current();
        assert_eq!(
            commands.try_recv()?,
            PlayCommand::ChatPublic(ChatIntent::CloseRoom(room))
        );
    }
    let (link, mut commands, outbox) = play_channel();
    outbox.chat_initialize(true);
    link.chat_open_entry(&oteryn_client_ui::chat::ChannelEntry::Room(ChatRoom::Tutor));
    assert!(commands.try_recv().is_err());
    let (link, mut commands, _) = play_channel();
    link.chat_open_help();
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn native_chat_keys_dispatch_saved_bindings_while_composing()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{Modifier, Modifiers};
    let (link, mut commands, outbox) = play_channel();
    outbox.chat_initialize(true);
    outbox.chat_public_initialize(true, Some((ChatRoomSet::default(), Vec::new())));
    link.chat_open_conversation("Ana");
    link.chat_open_conversation("Bryn");
    let mut input = crate::play::PreferenceInput::default();
    let mut key = |vk, mods| {
        let consumed = super::console::native_key(&link, &mut input, vk, mods, true, false, false);
        assert!(super::console::native_key(
            &link, &mut input, vk, mods, false, false, false
        ));
        consumed
    };
    let initial = link.chat_panel().console.current;
    assert!(key(0x09, Modifiers::NONE));
    assert_ne!(link.chat_panel().console.current, initial);
    assert!(key(0x09, Modifiers::one(Modifier::Shift)));
    assert_eq!(link.chat_panel().console.current, initial);
    let ctrl = Modifiers::one(Modifier::Control);
    assert!(key(0x4f, ctrl));
    assert!(link.chat_channel_dialog().0.open);
    assert!(key(0x48, ctrl));
    assert!(link.lens_active());
    assert!(key(0x45, ctrl));
    assert_eq!(link.chat_panel().console.channels.len(), 1);
    assert!(key(0x54, ctrl));
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::ChatPublic(oteryn_session::ChatIntent::OpenRoom(
            oteryn_session::ChatRoom::Help
        ))
    ));
    // A saved rebind replaces Ctrl+O in this HWND too.
    {
        let mut m = crate::play::mailbox::lock(&link.mailbox);
        let action = m
            .local_preferences
            .custom_hotkeys
            .remove("CTRL+O")
            .ok_or("binding")?;
        m.local_preferences
            .custom_hotkeys
            .insert("ALT+O".into(), action);
    }
    assert!(!super::console::native_key(
        &link, &mut input, 0x4f, ctrl, true, false, false
    ));
    assert!(!super::console::native_key(
        &link, &mut input, 0x4f, ctrl, false, false, false
    ));
    assert!(super::console::native_key(
        &link,
        &mut input,
        0x4f,
        Modifiers::one(Modifier::Alt),
        true,
        false,
        false
    ));
    crate::play::mailbox::lock(&link.mailbox)
        .local_preferences
        .custom_hotkeys
        .insert(
            "A".into(),
            crate::play::CustomAction::Utility {
                action: crate::play::UtilityAction::NextChannel,
            },
        );
    assert!(!super::console::native_key(
        &link,
        &mut input,
        0x41,
        Modifiers::NONE,
        true,
        false,
        false
    ));
    Ok(())
}

#[test]
fn native_chat_tabs_refresh_renamed_captions_with_the_same_count() {
    let mut console = oteryn_client_ui::chat::Console::default();
    console.open_channel(oteryn_client_ui::chat::ChannelTab::Private(11));
    let mut held = console.tab_labels();
    let list = oteryn_session::ChatChannelList {
        own_channel: Some(11),
        ..Default::default()
    };
    console.sync_participants("Ana", Some(&list), &[]);
    let labels = console.tab_labels();
    assert_eq!(labels.len(), held.len());
    assert!(super::console::update_tab_labels(&mut held, &labels));
    assert_eq!(held, labels);
    assert!(!super::console::update_tab_labels(&mut held, &labels));
}

#[test]
fn native_chat_keys_dispatch_saved_editing_and_keypad_bindings()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{Modifier, Modifiers};
    for (chord, vk, extended) in [
        ("ENTER", 0x0d, false),
        ("ESCAPE", 0x1b, false),
        ("KEYPADENTER", 0x0d, true),
        ("ARROWLEFT", 0x25, true),
        ("ARROWUP", 0x26, true),
        ("ARROWRIGHT", 0x27, true),
        ("ARROWDOWN", 0x28, true),
        ("KEYPAD1", 0x61, false),
        ("KEYPAD3", 0x63, false),
        ("KEYPAD7", 0x67, false),
        ("KEYPAD9", 0x69, false),
        ("KEYPAD1", 0x23, false),
        ("KEYPAD3", 0x22, false),
        ("KEYPAD7", 0x24, false),
        ("KEYPAD9", 0x21, false),
    ] {
        let (link, _, outbox) = play_channel();
        outbox.chat_initialize(true);
        {
            let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
            let action = mailbox
                .local_preferences
                .custom_hotkeys
                .remove("CTRL+O")
                .ok_or("binding")?;
            mailbox
                .local_preferences
                .custom_hotkeys
                .insert(format!("ALT+{chord}"), action);
        }
        let mut input = crate::play::PreferenceInput::default();
        let alt = Modifiers::one(Modifier::Alt);
        assert!(
            !super::console::native_key(
                &link,
                &mut input,
                vk,
                Modifiers::NONE,
                true,
                false,
                extended
            ),
            "{chord}"
        );
        assert!(!link.chat_channel_dialog().0.open);
        assert!(!super::console::native_key(
            &link,
            &mut input,
            vk,
            Modifiers::NONE,
            false,
            false,
            extended
        ));
        assert!(
            super::console::native_key(&link, &mut input, vk, alt, true, false, extended),
            "{chord}"
        );
        assert!(link.chat_channel_dialog().0.open, "{chord}");
        assert!(
            super::console::native_key(&link, &mut input, vk, alt, false, false, extended),
            "{chord}"
        );
    }
    Ok(())
}
