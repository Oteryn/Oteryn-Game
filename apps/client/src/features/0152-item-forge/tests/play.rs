use crate::play::play_channel;
#[test]
fn item_forge_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    outbox.item_forge_initialize(true);
    link.item_forge_request(vec![]);
    assert!(matches!(
        rx.try_recv(),
        Ok(crate::combat_input::PlayCommand::ServerMechanic(_))
    ));
}

#[test]
fn forge_native_flow_decodes_result_and_history() {
    use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
    let (link, _, outbox) = play_channel();
    outbox.item_forge_initialize(true);
    let receipt = br#"{"operation":"fusion","succeeded":false,"applied":true,"reason":null,"bonus":0,"result":{"first_item":2,"first_tier":1,"second_item":3,"second_tier":null,"dust":50,"dust_limit":100,"gold_spent":25000,"cores_spent":1,"slivers_spent":0,"slivers_created":0,"cores_created":0}}"#;
    let mut response = ServerMechanicResponse {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: receipt.to_vec(),
    };
    outbox.item_forge_result(&response);
    let text = oteryn_client_ui::item_forge::panel_text(&link.item_forge_panel());
    assert!(text.contains("Fusion: failed"), "{text}");
    assert!(text.contains("Dust: 50 / 100"), "{text}");
    response.payload = [b"[{\"ordinal\":7,\"receipt\":".as_slice(), receipt, b"}]"].concat();
    outbox.item_forge_result(&response);
    let text = oteryn_client_ui::item_forge::panel_text(&link.item_forge_panel());
    assert!(text.contains("History #7"), "{text}");
    response.payload = vec![1];
    outbox.item_forge_result(&response);
    let text = oteryn_client_ui::item_forge::panel_text(&link.item_forge_panel());
    assert!(
        text.contains("History #7") && text.contains("Invalid forge reply"),
        "{text}"
    );
}

use crate::play::feature_0152_item_forge::view::native;
#[test]
#[allow(clippy::expect_used, clippy::panic)]
fn native_controls_submit_and_restore() {
    let (link, mut rx, outbox) = play_channel();
    outbox.item_forge_initialize(true);
    assert!(native::CONTROLS.iter().any(|c| c.0 == "BUTTON"));
    assert_eq!(native::CONTROLS.iter().filter(|c| c.0 == "EDIT").count(), 7);
    let mut inputs = native::Inputs::default();
    inputs.0[0] = "1".into();
    inputs.0[1] = "22".into();
    inputs.0[2] = "33".into();
    inputs.0[3] = "01010101010101010101010101010101".into();
    native::submit(&link, &inputs);
    let Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) = rx.try_recv() else {
        panic!("missing command");
    };
    let Some(oteryn_client_runtime::cui3::item_forge::Request {
        first_item: Some(first),
        ..
    }) = oteryn_client_runtime::cui3::item_forge::Request::decode(&command.payload)
    else {
        panic!("forge request");
    };
    assert_eq!(first.get(), 22);
    assert!(!native::enabled(&link.item_forge_panel()));
    native::submit(&link, &inputs);
    assert!(rx.try_recv().is_err());
    let mut panel = link.item_forge_panel();
    oteryn_client_ui::item_forge::apply_result(
        &mut panel,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            status: oteryn_client_runtime::server_mechanic::CommandStatus::Accepted,
            server_sequence: 1,
            payload: br#"{"outcome":"Applied","dust":42}"#.to_vec(),
        },
    );
    assert!(native::presentation(&panel).contains("42"));
    panel.pending = true;
    oteryn_client_ui::item_forge::apply_result(
        &mut panel,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            status: oteryn_client_runtime::server_mechanic::CommandStatus::Rejected,
            server_sequence: 1,
            payload: vec![],
        },
    );
    assert!(native::enabled(&panel));
    assert!(native::presentation(&panel).contains("refused"));
}

#[test]
fn nx8_current_dust_is_displayed_without_history() {
    use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
    let (link, _, outbox) = play_channel();
    outbox.item_forge_initialize(true);
    outbox.item_forge_result(&ServerMechanicResponse {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: br#"{"current_dust":73,"dust_limit":125}"#.to_vec(),
    });
    let text = oteryn_client_ui::item_forge::panel_text(&link.item_forge_panel());
    assert!(text.contains("Current dust: 73 / 125"), "{text}");
}

#[test]
#[allow(clippy::panic)] // A missing command is the behaviour this play test diagnoses.
fn nx8_native_current_dust_control_uses_query_without_mutation_fields() {
    let (link, mut rx, outbox) = play_channel();
    outbox.item_forge_initialize(true);
    let mut inputs = native::Inputs::default();
    inputs.0[0] = "8".into();
    inputs.0[3] = "not a mutation occurrence".into();
    native::submit(&link, &inputs);
    let Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) = rx.try_recv() else {
        panic!("missing current dust command");
    };
    assert_eq!(command.command_type, 668);
    assert!(command.payload.is_empty());
}

#[path = "preview_play.rs"]
mod preview_play;
#[path = "preview_session.rs"]
mod preview_session;
