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

use crate::play::feature_0153_depot_search::view::native;
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

#[test]
fn depot_result_controls_retrieve_all_and_keep_server_outcomes()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cui3::depot_search::{
        self as wire, Outcome, Reply, Request, SearchResult, TierCount,
    };
    use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
    let (link, mut receiver, outbox) = play_channel();
    outbox.depot_search_initialize(true);
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.depot_search.reply = Some(Reply {
            outcome: Outcome::Listed,
            retrieved: 0,
            results: vec![SearchResult {
                client_id: 321,
                depot: 7,
                stash: 0,
                inbox: 0,
                tiers: vec![TierCount { tier: 0, count: 7 }],
            }],
        });
    }
    let inputs = native::Inputs::default();
    native::activate(&link, &inputs, 8);
    let crate::combat_input::PlayCommand::ServerMechanic(command) = receiver.try_recv()? else {
        return Err("missing retrieve-all command".into());
    };
    assert_eq!(
        wire::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
        Request::Retrieve {
            client_id: 321,
            all: true
        }
    );
    native::activate(&link, &inputs, 8);
    assert!(receiver.try_recv().is_err());
    let mut panel = link.depot_search_panel();
    oteryn_client_ui::depot_search::apply_result(
        &mut panel,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: wire::encode_reply(&Reply {
                outcome: Outcome::Retrieved,
                retrieved: 7,
                results: vec![],
            })
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    assert!(native::presentation(&panel).contains("Retrieved: 7"));
    crate::play::mailbox::lock(&link.mailbox).depot_search = panel;
    native::activate(&link, &inputs, 8);
    assert!(receiver.try_recv().is_err());
    native::activate(&link, &inputs, 5);
    let crate::combat_input::PlayCommand::ServerMechanic(command) = receiver.try_recv()? else {
        return Err("missing list command".into());
    };
    assert_eq!(
        wire::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
        Request::Open { after: 0 }
    );
    Ok(())
}

#[test]
fn depot_native_callback_does_not_drop_the_session_owner() -> Result<(), Box<dyn std::error::Error>>
{
    use oteryn_client_runtime::cui3::depot_search::{self as wire, Request};
    let (link, mut receiver, _outbox) = play_channel();
    let observed = std::sync::Arc::clone(&link.mailbox);
    {
        let mut mailbox = crate::play::mailbox::lock(&observed);
        mailbox.depot_search.selected = true;
        mailbox.properties.appearance.selected = true;
        mailbox.properties.appearance.open = true;
        mailbox.wheel.selected = true;
        assert!(!mailbox.quest_shutdown && !mailbox.wheel_shutdown);
    }
    // These are exactly the references held by the native window's callback.
    // Their scope must end without dropping another owning PlayLink.
    {
        let shared = std::sync::Arc::clone(&observed);
        let commands = link.commands.clone();
        native::activate_shared(&shared, &commands, &native::Inputs::default(), 5);
    }
    let crate::combat_input::PlayCommand::ServerMechanic(command) = receiver.try_recv()? else {
        return Err("missing native list command".into());
    };
    assert_eq!(
        wire::decode_request(&command.payload).map_err(|error| format!("{error:?}"))?,
        Request::Open { after: 0 }
    );
    {
        let mailbox = crate::play::mailbox::lock(&observed);
        assert!(mailbox.depot_search.pending);
        assert!(mailbox.properties.appearance.selected && mailbox.properties.appearance.open);
        assert!(mailbox.wheel.selected);
        assert!(!mailbox.quest_shutdown && !mailbox.wheel_shutdown);
    }
    // The original owning link still performs its established cleanup.
    drop(link);
    let mailbox = crate::play::mailbox::lock(&observed);
    assert!(!mailbox.properties.appearance.selected && !mailbox.properties.appearance.open);
    assert!(mailbox.quest_shutdown && mailbox.wheel_shutdown);
    Ok(())
}

#[test]
fn depot_native_retrieve_rejects_a_changed_displayed_reply()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cui3::depot_search::{
        self as wire, Outcome, Reply, Request, SearchResult,
    };
    let (link, mut receiver, outbox) = play_channel();
    outbox.depot_search_initialize(true);
    let displayed = Reply {
        outcome: Outcome::Listed,
        retrieved: 0,
        results: vec![SearchResult {
            client_id: 321,
            depot: 7,
            stash: 0,
            inbox: 0,
            tiers: vec![],
        }],
    };
    let mut current = displayed.clone();
    current.results[0].client_id = 999;
    crate::play::mailbox::lock(&link.mailbox).depot_search.reply = Some(current.clone());
    let inputs = native::Inputs::default();
    native::activate_displayed(&link.mailbox, &link.commands, &inputs, 8, Some(&displayed));
    assert!(receiver.try_recv().is_err());
    assert!(
        link.depot_search_panel()
            .feedback
            .as_deref()
            .is_some_and(|message| message.contains("Results changed"))
    );
    native::activate_displayed(&link.mailbox, &link.commands, &inputs, 8, Some(&current));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = receiver.try_recv()? else {
        return Err("retrieve-all missing".into());
    };
    assert_eq!(
        wire::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
        Request::Retrieve {
            client_id: 999,
            all: true
        }
    );
    Ok(())
}
