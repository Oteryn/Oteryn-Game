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

#[path = "../native.rs"]
mod native;
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
