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
