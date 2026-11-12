use crate::play::play_channel;
use oteryn_client_runtime::{
    cast::Message,
    server_mechanic::{CommandStatus, ServerMechanicResponse},
};
#[test]
fn s17a_native_caster_panel_sends_start_and_confirmed_stop()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut rx, outbox) = play_channel();
    outbox.cast_control_initialize(true);
    link.cast_control_open();
    let crate::combat_input::PlayCommand::ServerMechanic(query) = rx.try_recv()? else {
        return Err("cast query".into());
    };
    assert_eq!(
        Message::decode(&query.payload).map_err(|e| format!("{e:?}"))?,
        Message::ControlQuery
    );
    outbox.cast_control_result(&ServerMechanicResponse {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: Message::End.encode().map_err(|e| format!("{e:?}"))?,
    });
    link.cast_control_request(0);
    let crate::combat_input::PlayCommand::ServerMechanic(request) = rx.try_recv()? else {
        return Err("cast command".into());
    };
    assert_eq!((request.capability, request.command_type), (290, 502));
    assert!(matches!(
        Message::decode(&request.payload).map_err(|e| format!("{e:?}"))?,
        Message::Start { revision: 0, .. }
    ));
    link.cast_control_request(0);
    assert!(
        rx.try_recv().is_err(),
        "pending request disables duplicate submission"
    );
    let response = Message::CasterState {
        cast_id: [1; 16],
        revision: 7,
        epoch: 1,
        delay: 0,
        count: 3,
    };
    outbox.cast_control_result(&ServerMechanicResponse {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: response.encode().map_err(|e| format!("{e:?}"))?,
    });
    assert!(link.cast_control_panel().text().contains("3 viewers"));
    link.cast_control_request(1);
    let crate::combat_input::PlayCommand::ServerMechanic(request) = rx.try_recv()? else {
        return Err("stop command".into());
    };
    assert_eq!(
        Message::decode(&request.payload).map_err(|e| format!("{e:?}"))?,
        Message::Stop { revision: 7 }
    );
    outbox.logged_out();
    assert!(!link.cast_control_panel().active);
    assert!(!link.cast_control_panel().selected);
    Ok(())
}

#[test]
fn s17a_native_consent_reads_and_uses_the_confirmed_character_revision()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut rx, outbox) = play_channel();
    outbox.cast_control_initialize(false);
    outbox.cast_consent_initialize(true);
    link.cast_control_open();
    let crate::combat_input::PlayCommand::ServerMechanic(query) = rx.try_recv()? else {
        return Err("consent query".into());
    };
    assert_eq!((query.capability, query.command_type), (292, 503));
    assert_eq!(
        Message::decode(&query.payload).map_err(|e| format!("{e:?}"))?,
        Message::ConsentQuery
    );
    outbox.cast_consent_result(&ServerMechanicResponse {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: Message::ConsentState {
            revision: 5,
            allowed: false,
        }
        .encode()
        .map_err(|e| format!("{e:?}"))?,
    });
    link.cast_control_request(3);
    let crate::combat_input::PlayCommand::ServerMechanic(toggle) = rx.try_recv()? else {
        return Err("consent toggle".into());
    };
    assert_eq!(
        Message::decode(&toggle.payload).map_err(|e| format!("{e:?}"))?,
        Message::ConsentSet {
            revision: 5,
            allowed: true
        }
    );
    assert!(!link.cast_control_panel().allow_shown);
    outbox.logged_out();
    assert!(!link.cast_control_panel().consent_known);
    Ok(())
}
