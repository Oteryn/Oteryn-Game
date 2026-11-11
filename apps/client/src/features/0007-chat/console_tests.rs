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
    outbox.chat_public_result(&oteryn_session::ChatOutcome {
        command_id: 1,
        status: CommandStatus::Accepted,
        disposition: oteryn_session::ChatDisposition::Ok,
        wait_seconds: 0,
        result_server_sequence: 1,
    });
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
