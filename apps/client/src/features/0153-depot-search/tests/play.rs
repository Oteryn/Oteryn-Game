use crate::play::play_channel;
#[test]
fn depot_search_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    outbox.depot_search_initialize(true);
    link.depot_search_request(vec![]);
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
    outbox.depot_search_initialize(true);
    assert!(native::CONTROLS.iter().any(|c| c.0 == "BUTTON"));
    assert_eq!(native::CONTROLS.iter().filter(|c| c.0 == "EDIT").count(), 3);
    let mut inputs = native::Inputs::default();
    inputs.0[0] = "321".into();
    native::submit(&link, &inputs);
    let Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) = rx.try_recv() else {
        panic!("missing command");
    };
    assert_eq!(
        oteryn_client_runtime::cui3::depot_search::decode_request(&command.payload)
            .expect("request"),
        oteryn_client_runtime::cui3::depot_search::Request::Search { client_id: 321 }
    );
    assert!(!native::enabled(&link.depot_search_panel()));
    native::submit(&link, &inputs);
    assert!(rx.try_recv().is_err());
    let mut panel = link.depot_search_panel();
    oteryn_client_ui::depot_search::apply_result(
        &mut panel,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            status: oteryn_client_runtime::server_mechanic::CommandStatus::Accepted,
            server_sequence: 1,
            payload: oteryn_client_runtime::cui3::depot_search::encode_reply(
                &oteryn_client_runtime::cui3::depot_search::Reply {
                    outcome: oteryn_client_runtime::cui3::depot_search::Outcome::Found,
                    retrieved: 0,
                    results: vec![oteryn_client_runtime::cui3::depot_search::SearchResult {
                        client_id: 321,
                        depot: 7,
                        stash: 0,
                        inbox: 0,
                        tiers: vec![oteryn_client_runtime::cui3::depot_search::TierCount {
                            tier: 0,
                            count: 7,
                        }],
                    }],
                },
            )
            .expect("valid reply"),
        },
    );
    assert!(native::presentation(&panel).contains("depot 7"));
    crate::play::mailbox::lock(&link.mailbox).depot_search = panel.clone();
    inputs.0[0] = "999".into();
    native::dispatch(&link, &inputs, true);
    let Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) = rx.try_recv() else {
        panic!("missing detail");
    };
    assert_eq!(
        oteryn_client_runtime::cui3::depot_search::decode_request(&command.payload)
            .expect("detail"),
        oteryn_client_runtime::cui3::depot_search::Request::Detail {
            client_id: 321,
            tier: 0
        }
    );
    panel.pending = true;
    oteryn_client_ui::depot_search::apply_result(
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
