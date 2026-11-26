pub(super) mod peer {
    include!("peer.rs");
}
struct Selected;
impl crate::play::Stepper for Selected {
    async fn step(
        &mut self,
        _: oteryn_session::StepDirection,
    ) -> Result<oteryn_session::StepOutcome, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
    async fn serve(&mut self, _: std::time::Duration) -> Result<(), oteryn_session::SessionError> {
        Ok(())
    }
    fn drain(&mut self) -> Vec<oteryn_session::SessionEvent> {
        Vec::new()
    }
    async fn combat(
        &mut self,
        _: crate::combat_input::CombatCommand,
    ) -> Result<crate::combat_input::CombatReport, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
    async fn logout(
        &mut self,
    ) -> Result<Option<oteryn_session::LogoutResult>, oteryn_session::SessionError> {
        Ok(None)
    }

    fn server_mechanic_selected(&self, _: u32) -> bool {
        true
    }
}
#[test]
fn cp02_0098_quick_loot_scenario() {
    let (link, mut rx, outbox) = crate::play::play_channel();
    assert!(
        !link.quick_loot_action(oteryn_client_ui::quick_loot::Action::Clear(
            oteryn_client_ui::quick_loot::QuickLootFilterMode::Skip
        ))
    );
    outbox.discovery_initialize(&Selected);
    assert!(
        link.quick_loot_action(oteryn_client_ui::quick_loot::Action::Clear(
            oteryn_client_ui::quick_loot::QuickLootFilterMode::Skip
        ))
    );
    assert!(
        matches!(rx.try_recv(), Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) if command.capability == 98 && command.command_type == 114)
    );
}

#[test]
fn cp02_quick_loot_native_session_scenario() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
    use oteryn_client_ui::quick_loot::*;
    peer::scenario(|link, push, wait| {
        let request = QuickLootRequest {
            action: QuickLootAction::FilterAdd,
            corpse: None,
            category: None,
            recipient: None,
            recipient_id: None,
            fallback_to_main: false,
            filter_mode: Some(QuickLootFilterMode::Skip),
            filter_definition_ref: Some(42),
        };
        push(peer::Exchange {
            command: ServerMechanicCommand {
                capability: 98,
                command_type: 114,
                payload: encode_quick_loot_request(&request)
                    .map_err(|error| format!("{error:?}"))?,
            },
            payload: encode_quick_loot_result(&QuickLootResult { items: vec![] })
                .map_err(|error| format!("{error:?}"))?,
            status: oteryn_session::CommandStatus::Accepted,
        });
        assert!(link.discovery_action(98, 2, &["42".into(), "1".into()], 0));
        wait(&|| link.discovery_panel().loot.confirmed.len() == 1);
        assert!(!link.discovery_action(98, 2, &["0".into(), "1".into()], 0));
        assert_eq!(link.discovery_panel().loot.confirmed.len(), 1);
        Ok(())
    })
}

#[test]
fn cp02_clear_uses_selected_filter_list() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut rx, outbox) = crate::play::play_channel();
    outbox.discovery_initialize(&Selected);
    assert!(link.discovery_action(98, 4, &["".into(), "2".into()], 0));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = rx.try_recv()? else {
        return Err("wrong command".into());
    };
    let wire = oteryn_client_ui::quick_loot::decode_quick_loot_request(&command.payload)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        wire.filter_mode,
        Some(oteryn_client_ui::quick_loot::QuickLootFilterMode::Accept)
    );
    Ok(())
}

#[test]
fn cp02_destination_controls_encode_existing_wire_and_reject_invalid_ids()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_ui::quick_loot::*;
    let (link, mut rx, outbox) = crate::play::play_channel();
    outbox.discovery_initialize(&Selected);
    assert!(!link.discovery_action(
        98,
        6,
        &["", "1", "1", "1", "2", "bad"].map(str::to_owned),
        0
    ));
    assert!(rx.try_recv().is_err());
    assert!(link.discovery_action(
        98,
        6,
        &["", "1", "1", "1", "2", "01010101010101010101010101010101"].map(str::to_owned),
        0
    ));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = rx.try_recv()? else {
        return Err("wrong command".into());
    };
    let wire = decode_quick_loot_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(wire.recipient, Some(QuickLootRecipient::GoldPouch));
    assert_eq!(wire.recipient_id, Some([1; 16]));
    assert!(wire.fallback_to_main);
    assert!(link.discovery_panel().loot.confirmed.is_empty());
    outbox.discovery_result(
        98,
        &oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Rejected,
            payload: vec![],
        },
    );
    assert!(link.discovery_panel().loot.pending.is_none());
    assert!(link.discovery_panel().loot.confirmed.is_empty());
    Ok(())
}
#[test]
fn cp02_shortcuts_are_exclusive_and_session_end_clears_all_panels()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    type Open = fn(&[NormalizedInputEvent]) -> bool;
    let opens: [(u16, Open); 4] = [
        (20, oteryn_client_ui::quick_loot::open_key),
        (12, oteryn_client_ui::inspection::open_key),
        (23, oteryn_client_ui::team_finder::open_key),
        (11, oteryn_client_ui::purchase_history::open_key),
    ];
    for (index, (code, _)) in opens.iter().enumerate() {
        let event = NormalizedInputEvent::Key {
            code: KeyCode::new(*code)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control).with(Modifier::Alt),
            repeat: false,
        };
        for (other, (_, open)) in opens.iter().enumerate() {
            assert_eq!(open(std::slice::from_ref(&event)), index == other);
        }
    }
    let (link, rx, outbox) = crate::play::play_channel();
    outbox.discovery_initialize(&Selected);
    assert!(link.quick_loot_action(oteryn_client_ui::quick_loot::Action::LootAll));
    outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    let state = link.discovery_panel();
    assert!(
        !state.loot.selected
            && !state.inspection.selected
            && !state.leader.selected
            && !state.teams.selected
            && !state.history.selected
    );
    assert!(state.loot.pending.is_none());
    outbox.discovery_initialize(&Selected);
    drop(rx);
    assert!(!link.quick_loot_action(oteryn_client_ui::quick_loot::Action::LootAll));
    assert!(link.discovery_panel().loot.pending.is_none());
    assert_eq!(link.discovery_panel().loot.feedback, "Session ended");
    Ok(())
}

#[test]
fn managed_container_reset_uses_the_selected_category_and_existing_wire()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_ui::quick_loot::{Action, QuickLootCategory, QuickLootRecipient};
    let (link, mut commands, _) = crate::play::play_channel();
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.discovery.loot.selected = true;
        mailbox
            .discovery
            .loot
            .destinations
            .push(Action::Destination {
                category: Some(QuickLootCategory::Gold),
                recipient: QuickLootRecipient::Container,
                id: Some([1; 16]),
                fallback: false,
            });
    }
    assert!(!link.quick_loot_control(8, &[], 0));
    assert!(link.quick_loot_control(7, &[], 0));
    assert!(link.quick_loot_control(8, &[], 0));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("wrong command path".into());
    };
    let request = oteryn_client_ui::quick_loot::decode_quick_loot_request(&command.payload)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(request.category, Some(QuickLootCategory::Gold));
    assert_eq!(request.recipient, Some(QuickLootRecipient::MainContainer));
    assert_eq!(request.recipient_id, None);
    assert!(request.fallback_to_main);
    assert!(!link.quick_loot_control(8, &[], 0));
    Ok(())
}

#[test]
fn t056_loot_notice_filter_preserves_received_result() {
    let (link, _, _) = crate::play::play_channel();
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.discovery.loot.selected = true;
        mailbox.discovery.loot.result =
            Some(oteryn_client_ui::quick_loot::QuickLootResult { items: vec![] });
        mailbox.local_preferences.game_window.loot_messages = false;
    }
    assert!(
        !oteryn_client_ui::quick_loot::panel_text(&link.discovery_panel().loot).contains("Looted")
    );
    crate::play::mailbox::lock(&link.mailbox)
        .local_preferences
        .game_window
        .loot_messages = true;
    assert!(
        oteryn_client_ui::quick_loot::panel_text(&link.discovery_panel().loot)
            .contains("Looted 0 of 0")
    );
}

#[test]
fn a_finished_loot_run_posts_one_loot_notice_when_loot_messages_are_on()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_ui::chat::{ChannelTab, NoticeScope};
    use oteryn_client_ui::quick_loot::{
        Action, QuickLootItemResult, QuickLootResult, encode_quick_loot_result,
    };
    let (link, _commands, outbox) = crate::play::play_channel();
    outbox.discovery_initialize(&Selected);
    let result = || -> Result<oteryn_session::GatedCommandResult, Box<dyn std::error::Error>> {
        Ok(oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: encode_quick_loot_result(&QuickLootResult {
                items: vec![
                    QuickLootItemResult {
                        item: [1; 16],
                        accepted: true,
                    },
                    QuickLootItemResult {
                        item: [2; 16],
                        accepted: false,
                    },
                ],
            })
            .map_err(|e| format!("{e:?}"))?,
        })
    };
    let notices = |link: &crate::play::PlayLink| {
        let mailbox = crate::play::mailbox::lock(&link.mailbox);
        let console = &mailbox.chat.console;
        console
            .notices
            .scoped_text(console.filters, console.display, NoticeScope::LootOnly)
    };
    assert!(link.quick_loot_action(Action::LootAll));
    outbox.discovery_result(98, &result()?);
    assert!(notices(&link).contains("[Loot] Looted 1 of 2 items"));
    // The Loot tab shows it; a filter edit (which is not a loot run) posts nothing more.
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.chat.console.open_channel(ChannelTab::Loot);
    }
    assert!(oteryn_client_ui::chat::panel_text(&link.chat_panel()).contains("Looted 1 of 2 items"));
    assert!(link.quick_loot_action(Action::Clear(
        oteryn_client_ui::quick_loot::QuickLootFilterMode::Skip
    )));
    outbox.discovery_result(98, &result()?);
    assert_eq!(notices(&link).matches("[Loot]").count(), 1);
    // With loot messages off, a loot run leaves no notice.
    crate::play::mailbox::lock(&link.mailbox)
        .local_preferences
        .game_window
        .loot_messages = false;
    assert!(link.quick_loot_action(Action::LootAll));
    outbox.discovery_result(98, &result()?);
    assert_eq!(notices(&link).matches("[Loot]").count(), 1);
    Ok(())
}

#[cfg(bevy_shell)]
impl crate::play::Outbox {
    pub(crate) fn bevy_team_finder_supported(&self) {
        crate::play::mailbox::lock(&self.0).discovery.teams.selected = true;
    }
}

#[test]
#[ignore = "requires this run's I07 node-boot capture"]
fn e2e_i07_real_replies_replay_through_play_outbox() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_ui::chat::NoticeScope;
    use oteryn_client_ui::quick_loot::*;
    use oteryn_session::{CommandStatus, GatedCommandResult};
    use std::collections::BTreeMap;

    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Expected {
        skip: u32,
        gold: u32,
        listed: [u8; 16],
        backpack: [u8; 16],
        accept: u32,
        pouch: [u8; 16],
        corpse: [u8; 16],
        coin: [u8; 16],
        armor: [u8; 16],
        filtered: [u8; 16],
        other: [u8; 16],
    }

    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Reply {
        command_id: u64,
        game_session_id: String,
        payload: Vec<u8>,
        server_sequence: u64,
        status: u32,
    }

    let directory = std::path::PathBuf::from(
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("I07 replay requires a fresh capture")?,
    );
    let expected: Expected =
        serde_json::from_slice(&std::fs::read(directory.join("i07-expected.txt"))?)?;
    let route = |category, recipient, id, fallback| Action::Destination {
        category,
        recipient,
        id,
        fallback,
    };
    let gold = route(
        Some(QuickLootCategory::Gold),
        QuickLootRecipient::GoldPouch,
        Some(expected.pouch),
        false,
    );
    let armor = |fallback| {
        route(
            Some(QuickLootCategory::Equipment),
            QuickLootRecipient::GoldPouch,
            Some(expected.pouch),
            fallback,
        )
    };
    let main = route(None, QuickLootRecipient::MainContainer, None, false);
    let click = Action::Corpse(expected.corpse);
    let skip = QuickLootFilterMode::Skip;
    let accept = QuickLootFilterMode::Accept;
    let gold_main = route(
        Some(QuickLootCategory::Gold),
        QuickLootRecipient::MainContainer,
        None,
        false,
    );
    let steps: [(Action, &[([u8; 16], bool)], Option<&str>); 18] = [
        (
            Action::Add {
                mode: skip,
                definition: expected.skip,
            },
            &[],
            None,
        ),
        (
            Action::Add {
                mode: accept,
                definition: expected.gold,
            },
            &[],
            None,
        ),
        (main.clone(), &[], None),
        (gold, &[], None),
        (
            Action::LootAll,
            &[
                (expected.coin, true),
                (expected.armor, false),
                (expected.filtered, false),
                (expected.other, false),
                (expected.listed, false),
            ],
            Some("Looted 1 of 5 items"),
        ),
        (gold_main.clone(), &[], None),
        (armor(false), &[], None),
        (
            Action::Add {
                mode: accept,
                definition: expected.accept,
            },
            &[],
            None,
        ),
        (
            Action::LootAll,
            &[
                (expected.armor, false),
                (expected.filtered, false),
                (expected.other, false),
                (expected.listed, false),
            ],
            Some("Looted 0 of 4 items"),
        ),
        (armor(true), &[], None),
        (
            Action::LootAll,
            &[
                (expected.armor, true),
                (expected.filtered, false),
                (expected.other, false),
                (expected.listed, false),
            ],
            Some("Looted 1 of 4 items"),
        ),
        (
            click,
            &[(expected.filtered, true)],
            Some("Looted 1 of 1 items"),
        ),
        (Action::Select(skip), &[], None),
        (
            route(
                None,
                QuickLootRecipient::GoldPouch,
                Some(expected.pouch),
                false,
            ),
            &[],
            None,
        ),
        (
            Action::LootAll,
            &[(expected.other, false), (expected.listed, false)],
            Some("Looted 0 of 2 items"),
        ),
        (main.clone(), &[], None),
        (
            Action::LootAll,
            &[(expected.other, true), (expected.listed, false)],
            Some("Looted 1 of 2 items"),
        ),
        (
            route(
                None,
                QuickLootRecipient::GoldPouch,
                Some(expected.backpack),
                true,
            ),
            &[],
            None,
        ),
    ];
    let (link, mut commands, outbox) = crate::play::play_channel();
    crate::play::mailbox::lock(&link.mailbox)
        .local_preferences
        .game_window
        .loot_messages = true;
    outbox.discovery_initialize(&Selected);
    let mut sessions = [String::new(), String::new()];
    let mut previous = (0, 0);
    for (index, (action, expected_items, notice)) in steps.into_iter().enumerate() {
        let key = index + 1;
        let session_index = usize::from(key >= 11);
        if key == 11 {
            outbox.discovery_initialize(&Selected);
            previous = (0, 0);
            let panel = link.discovery_panel().loot;
            assert!(panel.confirmed.is_empty() && panel.destinations.is_empty());
        }
        let reply: Reply =
            serde_json::from_slice(&std::fs::read(directory.join(format!("i07-{key}.json")))?)?;
        let status = if key == 18 {
            CommandStatus::Rejected
        } else {
            CommandStatus::Accepted
        };
        assert_eq!(reply.status, status as u32, "I07 probe {key}");
        assert!(
            reply.command_id > previous.0 && reply.server_sequence > previous.1,
            "I07 probe {key}"
        );
        previous = (reply.command_id, reply.server_sequence);
        if key == 1 || key == 11 {
            sessions[session_index].clone_from(&reply.game_session_id);
        }
        assert_eq!(
            reply.game_session_id, sessions[session_index],
            "I07 probe {key}"
        );
        let mode = |mode| (mode as u8).to_string();
        let dispatched = match action {
            Action::LootAll => link.discovery_action(98, 1, &[], 0),
            Action::Add {
                mode: filter,
                definition,
            } => link.discovery_action(98, 2, &[definition.to_string(), mode(filter)], 0),
            Action::Select(filter) => {
                link.discovery_action(98, 5, &[String::new(), mode(filter)], 0)
            }
            Action::Destination {
                category,
                recipient,
                id,
                fallback,
            } => {
                let id = id
                    .map(|id| {
                        id.iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>()
                    })
                    .unwrap_or_default();
                link.discovery_action(
                    98,
                    6,
                    &[
                        String::new(),
                        String::new(),
                        category.map_or(0, |category| category as u8).to_string(),
                        u8::from(fallback).to_string(),
                        (recipient as u8).to_string(),
                        id,
                    ],
                    0,
                )
            }
            Action::Corpse(_) => link.quick_loot_action(action),
            _ => return Err("unexpected I07 replay action".into()),
        };
        assert!(dispatched, "I07 probe {key}");
        let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
            return Err("I07 client queued the wrong command".into());
        };
        assert_eq!((command.capability, command.command_type), (98, 114));
        assert_eq!(
            command.payload,
            std::fs::read(directory.join(format!("i07-{key}-request.bin")))?,
            "I07 probe {key}"
        );
        let notices = || {
            let mailbox = crate::play::mailbox::lock(&link.mailbox);
            let console = &mailbox.chat.console;
            console
                .notices
                .scoped_text(console.filters, console.display, NoticeScope::LootOnly)
        };
        let before = notices();
        let before_panel = link.discovery_panel().loot;
        outbox.discovery_result(
            98,
            &GatedCommandResult {
                command_id: reply.command_id,
                server_sequence: reply.server_sequence,
                status,
                payload: reply.payload,
            },
        );
        let panel = link.discovery_panel().loot;
        assert_eq!(panel.pending, None);
        if key == 18 {
            assert_eq!(panel.destinations, before_panel.destinations);
            assert_eq!(panel.result, before_panel.result);
            assert_eq!(notices(), before);
            assert_eq!(
                panel.feedback,
                "Loot request refused or invalid reply; try again"
            );
            continue;
        }
        assert_eq!(panel.feedback, "Confirmed by the server");
        let items = &panel
            .result
            .ok_or("I07 client omitted the loot result")?
            .items;
        assert_eq!(items.len(), expected_items.len());
        assert_eq!(
            items
                .iter()
                .map(|item| (item.item, item.accepted))
                .collect::<BTreeMap<_, _>>(),
            expected_items.iter().copied().collect(),
            "I07 probe {key}"
        );
        let after = notices();
        assert_eq!(
            after.matches("[Loot]").count(),
            before.matches("[Loot]").count() + usize::from(notice.is_some())
        );
        if let Some(notice) = notice {
            assert!(
                after
                    .lines()
                    .last()
                    .is_some_and(|line| line.ends_with(notice))
            );
        }
        if key == 10 {
            assert_eq!(panel.skipped, [expected.skip].into_iter().collect());
            assert_eq!(
                panel.accepted,
                [expected.accept, expected.gold].into_iter().collect()
            );
            assert_eq!(
                panel.destinations,
                [main.clone(), gold_main.clone(), armor(true)]
            );
        }
    }
    assert_ne!(sessions[0], sessions[1]);
    println!(
        "I07_NATIVE_REPLAY session={} relog={}",
        sessions[0], sessions[1]
    );
    Ok(())
}
