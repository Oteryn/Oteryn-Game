use crate::play::play_channel;
use oteryn_client_runtime::{
    CastCasterLine,
    cast::Message,
    server_mechanic::{CommandStatus, ServerMechanicResponse},
};
use oteryn_session::{AppliedDelta, SessionEvent};
#[test]
fn s17c_caster_receives_live_text_at_delay_300_and_sends_moderation()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut rx, outbox) = play_channel();
    outbox.cast_control_initialize(true);
    outbox.cast_chat_initialize(true);
    outbox.cast_control_result(&ServerMechanicResponse {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: Message::CasterState {
            cast_id: [1; 16],
            revision: 9,
            epoch: 1,
            delay: 300,
            count: 1,
        }
        .encode()
        .map_err(|e| format!("{e:?}"))?,
    });
    outbox.push(vec![
        SessionEvent::CastRoster(AppliedDelta {
            server_sequence: 2,
            base_revision: 1,
            new_revision: 2,
            value: oteryn_client_runtime::CastRoster {
                revision: 9,
                subjects: vec![[4; 16]],
            },
        }),
        SessionEvent::CastCasterLine(AppliedDelta {
            server_sequence: 2,
            base_revision: 1,
            new_revision: 2,
            value: CastCasterLine {
                handle: [4; 16],
                text: "Viewer-1: live at D=300".into(),
            },
        }),
    ]);
    assert!(
        oteryn_client_ui::cast_chat::panel_text(&link.cast_chat_panel()).contains("live at D=300")
    );
    link.cast_chat_request("", &"04".repeat(16), 1);
    let crate::combat_input::PlayCommand::ServerMechanic(command) = rx.try_recv()? else {
        return Err("moderate request".into());
    };
    assert_eq!((command.capability, command.command_type), (290, 505));
    assert!(
        matches!(Message::decode(&command.payload).map_err(|e|format!("{e:?}"))?,Message::Moderate{revision:9,subject,action:1,..} if subject==[4;16])
    );
    outbox.cast_control_result(&ServerMechanicResponse {
        command_id: 3,
        status: CommandStatus::Accepted,
        server_sequence: 3,
        payload: Message::End.encode().map_err(|e| format!("{e:?}"))?,
    });
    outbox.push(vec![SessionEvent::CastCasterLine(AppliedDelta {
        server_sequence: 4,
        base_revision: 3,
        new_revision: 4,
        value: CastCasterLine {
            handle: [4; 16],
            text: "Held after stop".into(),
        },
    })]);
    assert!(link.cast_chat_panel().lines.is_empty());
    outbox.logged_out();
    assert!(link.cast_chat_panel().lines.is_empty());
    assert!(!link.cast_chat_panel().selected);
    Ok(())
}
