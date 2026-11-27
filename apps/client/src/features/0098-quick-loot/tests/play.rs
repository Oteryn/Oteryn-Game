pub(super) mod peer {
    include!("peer.rs");
}
struct Selected(bool);
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

    fn server_mechanic_selected(&self, capability: u32) -> bool {
        capability != oteryn_session::quick_loot::CAPABILITY_QUICK_LOOT_DESTINATIONS_V1 || self.0
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
    outbox.discovery_initialize(&Selected(false));
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
        wait(&|| link.quick_loot_panel().filter_mode == Some(QuickLootFilterMode::Accept));
        let panel = link.quick_loot_panel();
        assert_eq!(panel.skipped, std::collections::BTreeSet::from([4]));
        assert_eq!(panel.accepted, std::collections::BTreeSet::from([7]));
        let request = QuickLootRequest {
            action: QuickLootAction::FilterAdd,
            corpse: None,
            selected_item: None,
            review_filters: false,
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
fn selected_item_loot_click_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
    use oteryn_session::{
        ActorPosition, AppliedDelta, CommandStatus, EntityDetail, EntityKind, EntityRef, ItemEntry,
        ItemHandle, OpenContainer, SessionEvent, WorldSpatialEntity,
    };

    let corpse = [7; 16];
    let container = ItemHandle::new(40).ok_or("corpse handle")?;
    let entry = |handle: u64| -> Result<ItemEntry, Box<dyn std::error::Error>> {
        Ok(ItemEntry {
            appearance_id: 0,
            handle: handle.try_into()?,
            item_definition_ref: 42.try_into()?,
            count: 1.try_into()?,
            sub_type: 0,
        })
    };
    let taken = entry(41)?;
    let remaining = entry(42)?;
    let opened = OpenContainer {
        container_handle: Some(container),
        entries: vec![taken, remaining],
    };
    let after = OpenContainer {
        container_handle: Some(container),
        entries: vec![remaining],
    };
    let delta = |revision, value| {
        SessionEvent::OpenContainer(AppliedDelta {
            server_sequence: revision + 10,
            base_revision: revision - 1,
            new_revision: revision,
            value,
        })
    };
    let visible_corpse = WorldSpatialEntity {
        motion: None,
        display_name: None,
        kind: EntityKind::Corpse,
        entity: EntityRef {
            identity: corpse,
            generation: 0,
        },
        position: ActorPosition {
            x: 100,
            y: 100,
            floor: 7,
        },
        detail: EntityDetail::Object {
            item_definition_ref: 43,
            quantity: 1,
            item_handle: Some(container),
        },
    };
    peer::scenario_with_events(
        vec![delta(1, opened.clone())],
        vec![visible_corpse],
        move |link, push, wait| {
            wait(&|| {
                let view = link.corpse_loot_view();
                view.contents == opened && view.identity == Some(corpse)
            });
            let exchange = |item: ItemHandle, payload, status| peer::Exchange {
                command: ServerMechanicCommand {
                    capability: 98,
                    command_type: 114,
                    payload: [
                        &[0x08, 1, 0x12, 16][..],
                        &corpse,
                        &[0x4a, 8],
                        &item.get().to_le_bytes(),
                    ]
                    .concat(),
                },
                payload,
                status,
            };
            push(
                exchange(taken.handle, vec![0x10, 1], CommandStatus::Accepted),
                vec![delta(2, after.clone())],
            );
            assert!(link.quick_loot_corpse_item(container, corpse, taken.handle));
            wait(&|| {
                let view = link.corpse_loot_view();
                view.contents == after && view.feedback == Some("Item moved")
            });
            assert!(!link.quick_loot_corpse_item(container, corpse, taken.handle));

            push(
                exchange(remaining.handle, vec![0x08, 0x01], CommandStatus::Rejected),
                vec![],
            );
            assert!(link.quick_loot_corpse_item(container, corpse, remaining.handle));
            wait(&|| link.corpse_loot_view().feedback == Some("Item is no longer available"));
            let view = link.corpse_loot_view();
            assert_eq!(view.identity, Some(corpse));
            assert_eq!(view.contents, after);
            assert!(link.quick_loot_panel().pending.is_none());
            Ok(())
        },
    )
}

#[test]
fn corpse_loot_click_without_selected_item_is_unchanged() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, mut rx, outbox) = crate::play::play_channel();
    outbox.discovery_initialize(&Selected(false));
    let corpse = [7; 16];
    assert!(link.quick_loot_action(oteryn_client_ui::quick_loot::Action::Corpse(corpse)));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = rx.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!((command.capability, command.command_type), (98, 114));
    assert_eq!(
        command.payload,
        [&[0x08, 1, 0x12, 16][..], &corpse].concat()
    );
    Ok(())
}

#[test]
fn cp02_clear_uses_selected_filter_list() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut rx, outbox) = crate::play::play_channel();
    outbox.discovery_initialize(&Selected(false));
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
    outbox.discovery_initialize(&Selected(false));
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
fn check_managed_destination_category(
    category: oteryn_session::quick_loot::QuickLootCategory,
) -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::quick_loot::*;
    let (link, mut rx, outbox) = crate::play::play_channel();
    let fields = [
        String::new(),
        String::new(),
        (category as u8).to_string(),
        "0".into(),
        "3".into(),
    ];
    outbox.discovery_initialize(&Selected(false));
    assert!(!link.discovery_action(98, 6, &fields, 0));
    assert!(rx.try_recv().is_err());

    outbox.discovery_initialize(&Selected(true));
    assert!(link.discovery_action(98, 6, &fields, 0), "{category:?}");
    let crate::combat_input::PlayCommand::ServerMechanic(command) = rx.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!(command.capability, CAPABILITY_QUICK_LOOT_DESTINATIONS_V1);
    assert_eq!(command.command_type, COMMAND_TYPE_QUICK_LOOT_DESTINATIONS);
    let wire = decode_quick_loot_request(&command.payload).map_err(|error| format!("{error:?}"))?;
    assert_eq!(wire.action, QuickLootAction::AssignLoot);
    assert_eq!(wire.category, Some(category));
    assert_eq!(wire.recipient, Some(QuickLootRecipient::MainContainer));
    Ok(())
}

#[test]
fn managed_destination_category_armor() -> Result<(), Box<dyn std::error::Error>> {
    check_managed_destination_category(oteryn_session::quick_loot::QuickLootCategory::Armor)
}

#[test]
fn managed_destination_category_highest() -> Result<(), Box<dyn std::error::Error>> {
    let category = oteryn_session::quick_loot::QuickLootCategory::ALL
        .last()
        .copied()
        .ok_or("empty protocol category range")?;
    check_managed_destination_category(category)
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
    outbox.discovery_initialize(&Selected(false));
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
    outbox.discovery_initialize(&Selected(false));
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
    outbox.discovery_initialize(&Selected(false));
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

#[cfg(bevy_shell)]
#[test]
#[ignore = "requires this run's I07 node-boot capture"]
fn e2e_i07_real_replies_replay_through_play_outbox() -> Result<(), Box<dyn std::error::Error>> {
    use crate::bevy_shell::quick_loot_replay as ui;
    use oteryn_client_ui::chat::NoticeScope;
    use oteryn_client_ui::quick_loot::*;
    use oteryn_session::{CommandStatus, GatedCommandResult};

    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct ContainerDelta {
        server_sequence: u64,
        base_revision: u64,
        new_revision: u64,
        payload: Vec<u8>,
    }

    impl ContainerDelta {
        fn decode(
            self,
        ) -> Result<
            oteryn_session::AppliedDelta<oteryn_session::OpenContainer>,
            Box<dyn std::error::Error>,
        > {
            Ok(oteryn_session::AppliedDelta {
                server_sequence: self.server_sequence,
                base_revision: self.base_revision,
                new_revision: self.new_revision,
                value: oteryn_protocol_oteryn::item_view::decode_open_container_with_appearances(
                    &self.payload,
                    true,
                )
                .map_err(|error| format!("I07 captured corpse view: {error:?}"))?,
            })
        }
    }

    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Expected {
        final_session: String,
        skip: u32,
        gold: u32,
        listed: [u8; 16],
        backpack: [u8; 16],
        accept: u32,
        pouch: [u8; 16],
        east_top: [u8; 16],
        foreign_view: ContainerDelta,
        fulfilment: [u8; 16],
        selected_handle: u64,
        corpse: [u8; 16],
        corpse_generation: u64,
        corpse_kind: u8,
        corpse_position: [i32; 3],
        corpse_definition: u32,
        opened: ContainerDelta,
        after_click: ContainerDelta,
        coin: [u8; 16],
        armor: [u8; 16],
        filtered: [u8; 16],
        spare: [u8; 16],
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
    let east_outcomes = |move_food| [(expected.listed, false), (expected.other, move_food)];
    let east_refused = east_outcomes(false);
    let east_moved = east_outcomes(true);
    let gold = route(
        Some(QuickLootCategory::Gold),
        QuickLootRecipient::GoldPouch,
        Some(expected.fulfilment),
        false,
    );
    let full = directory.file_name().is_some_and(|name| name == "i07-full");
    let armor_route = |id, fallback| {
        route(
            Some(QuickLootCategory::Armor),
            QuickLootRecipient::GoldPouch,
            Some(id),
            fallback,
        )
    };
    let armor = |fallback| armor_route(expected.fulfilment, fallback);
    let main = route(None, QuickLootRecipient::MainContainer, None, false);
    let selected = oteryn_session::ItemHandle::new(expected.selected_handle)
        .ok_or("I07 selected handle must be nonzero")?;
    let click = Action::CorpseItem {
        corpse: expected.corpse,
        item: selected,
    };
    let read_reply = |key| -> Result<Reply, Box<dyn std::error::Error>> {
        Ok(serde_json::from_slice(&std::fs::read(
            directory.join(format!("i07-{key}.json")),
        )?)?)
    };
    let moved = read_reply(13)?;
    let stale = read_reply(20)?;
    let first = read_reply(1)?;
    let relog = read_reply(12)?;
    assert_ne!(first.game_session_id, relog.game_session_id);
    assert_eq!(moved.game_session_id, relog.game_session_id);
    assert_eq!(stale.game_session_id, relog.game_session_id);
    assert!(moved.command_id > relog.command_id && stale.command_id > moved.command_id);
    assert_eq!(moved.status, CommandStatus::Accepted as u32);
    assert_eq!(stale.status, CommandStatus::Rejected as u32);
    let opened = expected.opened.decode()?;
    let after_click = expected.after_click.decode()?;
    assert!(opened.server_sequence > relog.server_sequence);
    assert!(opened.server_sequence < moved.server_sequence);
    assert!(moved.server_sequence < after_click.server_sequence);
    assert!(after_click.server_sequence < stale.server_sequence);
    assert_eq!(after_click.base_revision, opened.new_revision);
    let container = opened
        .value
        .container_handle
        .ok_or("I07 open corpse handle missing")?;
    assert_eq!(after_click.value.container_handle, Some(container));
    let [taken, sibling] = opened.value.entries.as_slice() else {
        return Err("I07 captured corpse requires exactly two entries".into());
    };
    assert_eq!(taken.handle, selected);
    assert_eq!(after_click.value.entries, vec![*sibling]);
    let mut initial = vec![selected, sibling.handle];
    initial.sort_unstable();
    let sibling = sibling.handle;
    let (menu_link, mut menu_commands, menu_outbox) = crate::play::play_channel();
    menu_outbox.bevy_quick_loot_selected(true);
    let [x, y, floor] = expected.corpse_position;
    assert_eq!(
        expected.corpse_kind,
        oteryn_session::EntityKind::Corpse as u8
    );
    menu_outbox.bevy_quick_loot_corpse(oteryn_session::WorldSpatialEntity {
        motion: None,
        display_name: None,
        kind: oteryn_session::EntityKind::Corpse,
        entity: oteryn_session::EntityRef {
            identity: expected.corpse,
            generation: expected.corpse_generation,
        },
        position: oteryn_session::ActorPosition {
            x,
            y,
            floor: floor.try_into()?,
        },
        detail: oteryn_session::EntityDetail::Object {
            item_definition_ref: expected.corpse_definition,
            quantity: 1,
            item_handle: Some(container),
        },
    });
    menu_outbox.push(vec![oteryn_session::SessionEvent::OpenContainer(opened)]);
    use crate::bevy_shell::corpse_menu_replay as menu;
    let mut menu_app = menu::corpse_menu_app(menu_link)?;
    assert_eq!(menu::rendered_corpse_items(&mut menu_app), initial);
    menu::click_corpse_loot(&mut menu_app, selected)?;
    let crate::combat_input::PlayCommand::ServerMechanic(command) = menu_commands.try_recv()?
    else {
        return Err("I07 corpse menu dispatched a different command".into());
    };
    assert_eq!((command.capability, command.command_type), (98, 114));
    assert_eq!(
        command.payload,
        std::fs::read(directory.join("i07-13-request.bin"))?
    );
    crate::play::session_loop::route_server_mechanic_reply(
        &menu_outbox,
        &command,
        &GatedCommandResult {
            command_id: moved.command_id,
            server_sequence: moved.server_sequence,
            status: CommandStatus::Accepted,
            payload: moved.payload,
        },
    );
    assert_eq!(
        menu::corpse_menu_link(&menu_app)?
            .corpse_loot_view()
            .feedback,
        Some("Item moved")
    );
    menu_outbox.push(vec![oteryn_session::SessionEvent::OpenContainer(
        after_click,
    )]);
    menu_app.update();
    let foreign_view = expected.foreign_view.decode()?;
    let [foreign_item] = foreign_view.value.entries.as_slice() else {
        return Err("I07 foreign corpse capture must expose its live item".into());
    };
    let foreign = read_reply(25)?;
    let last_assignment = read_reply(24)?;
    assert_eq!(foreign.game_session_id, relog.game_session_id);
    assert!(foreign.command_id > last_assignment.command_id);
    assert!(last_assignment.server_sequence < foreign_view.server_sequence);
    assert!(foreign_view.server_sequence < foreign.server_sequence);
    assert_eq!(foreign.status, CommandStatus::Rejected as u32);
    assert_ne!(foreign_view.value.container_handle, Some(container));
    assert_ne!(foreign_item.handle, selected);
    assert_ne!(foreign_item.handle, sibling);
    assert_eq!(foreign_item.item_definition_ref.get(), expected.skip);
    assert!(
        menu::corpse_menu_link(&menu_app)?.quick_loot_action(Action::CorpseItem {
            corpse: expected.corpse,
            item: foreign_item.handle,
        })
    );
    let crate::combat_input::PlayCommand::ServerMechanic(command) = menu_commands.try_recv()?
    else {
        return Err("I07 foreign-handle replay dispatched a different command".into());
    };
    assert_eq!((command.capability, command.command_type), (98, 114));
    assert_eq!(
        command.payload,
        std::fs::read(directory.join("i07-25-request.bin"))?
    );
    crate::play::session_loop::route_server_mechanic_reply(
        &menu_outbox,
        &command,
        &GatedCommandResult {
            command_id: foreign.command_id,
            server_sequence: foreign.server_sequence,
            status: CommandStatus::Rejected,
            payload: foreign.payload,
        },
    );
    assert_eq!(
        menu::corpse_menu_link(&menu_app)?
            .corpse_loot_view()
            .feedback,
        Some("Item is no longer available")
    );
    assert_eq!(menu::rendered_corpse_items(&mut menu_app), vec![sibling]);
    assert!(!menu::corpse_menu_link(&menu_app)?.quick_loot_corpse_item(
        container,
        expected.corpse,
        selected
    ));

    // Replay the captured stale receipt against the obsolete action. The rendered
    // menu has already removed that row and correctly refuses to submit it again.
    assert!(menu::corpse_menu_link(&menu_app)?.quick_loot_action(click.clone()));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = menu_commands.try_recv()?
    else {
        return Err("I07 stale replay dispatched a different command".into());
    };
    assert_eq!((command.capability, command.command_type), (98, 114));
    assert_eq!(
        command.payload,
        std::fs::read(directory.join("i07-20-request.bin"))?
    );
    crate::play::session_loop::route_server_mechanic_reply(
        &menu_outbox,
        &command,
        &GatedCommandResult {
            command_id: stale.command_id,
            server_sequence: stale.server_sequence,
            status: CommandStatus::Rejected,
            payload: stale.payload,
        },
    );
    assert_eq!(
        menu::corpse_menu_link(&menu_app)?
            .corpse_loot_view()
            .feedback,
        Some("Item is no longer available")
    );
    menu_app.update();
    assert_eq!(menu::rendered_corpse_items(&mut menu_app), vec![sibling]);
    assert!(menu_commands.try_recv().is_err());
    let skip = QuickLootFilterMode::Skip;
    let accept = QuickLootFilterMode::Accept;
    let gold_main = route(
        Some(QuickLootCategory::Gold),
        QuickLootRecipient::MainContainer,
        None,
        false,
    );
    type ReplayStep<'a> = (Action, &'a [([u8; 16], bool)], Option<&'a str>);
    let unassigned_pouch = route(
        None,
        QuickLootRecipient::GoldPouch,
        Some(expected.fulfilment),
        false,
    );
    let steps: [ReplayStep<'_>; 20] = [
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
        (unassigned_pouch.clone(), &[], None),
        (gold, &[], None),
        (main.clone(), &[], None),
        (
            Action::LootAll,
            &[
                (expected.coin, true),
                (expected.armor, false),
                (expected.filtered, false),
                (expected.spare, false),
                (expected.listed, false),
                (expected.other, false),
            ],
            Some("Looted 1 of 6 items"),
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
                (expected.spare, false),
            ],
            Some("Looted 0 of 3 items"),
        ),
        (armor(true), &[], None),
        (
            Action::LootAll,
            &[
                (expected.armor, true),
                (expected.filtered, false),
                (expected.spare, false),
            ],
            Some("Looted 1 of 3 items"),
        ),
        (click.clone(), &[], None),
        (Action::Select(skip), &[], None),
        (
            if full {
                route(
                    Some(QuickLootCategory::Food),
                    QuickLootRecipient::GoldPouch,
                    Some(expected.fulfilment),
                    false,
                )
            } else {
                unassigned_pouch
            },
            &[],
            None,
        ),
        (Action::LootAll, &east_refused, Some("Looted 0 of 2 items")),
        (
            if full {
                route(
                    Some(QuickLootCategory::Food),
                    QuickLootRecipient::MainContainer,
                    None,
                    false,
                )
            } else {
                main.clone()
            },
            &[],
            None,
        ),
        (Action::LootAll, &east_moved, Some("Looted 1 of 2 items")),
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
        (click, &[], None),
    ];
    let (mut app, mut commands, outbox) = ui::fixture(true)?;
    app.add_plugins(bevy::ui_widgets::ButtonPlugin);
    let link = ui::link(&app)?;
    crate::play::mailbox::lock(&link.mailbox)
        .local_preferences
        .game_window
        .loot_messages = true;
    outbox.discovery_initialize(&Selected(true));
    let picker_inventory = oteryn_session::item_view::decode_character_inventory_with_appearances(
        &std::fs::read(directory.join("i07-picker-inventory.bin"))?,
        true,
        true,
    )
    .map_err(|error| format!("I07 captured picker inventory: {error:?}"))?;
    ui::prepare_filter_picker(&mut app, &outbox, picker_inventory.clone());
    ui::open(&mut app)?;
    use oteryn_client_runtime::cui3::store as store_wire;
    outbox.bevy_store_initialize();
    ui::click_inbox_refresh(&mut app)?;
    let crate::combat_input::PlayCommand::ServerMechanic(inbox_command) = commands.try_recv()?
    else {
        return Err("I07 client queued the wrong Store Inbox command".into());
    };
    assert_eq!(inbox_command.capability, store_wire::CAPABILITY_STORE_V1);
    assert_eq!(
        inbox_command.command_type,
        store_wire::COMMAND_TYPE_STORE_INBOX_CLAIM
    );
    assert_eq!(
        inbox_command.payload,
        std::fs::read(directory.join("i07-inbox-request.bin"))?
    );
    let inbox: Reply = serde_json::from_slice(&std::fs::read(directory.join("i07-inbox.json"))?)?;
    assert_eq!(inbox.game_session_id, first.game_session_id);
    assert_eq!(inbox.status, CommandStatus::Accepted as u32);
    crate::play::session_loop::route_server_mechanic_reply(
        &outbox,
        &inbox_command,
        &GatedCommandResult {
            command_id: inbox.command_id,
            server_sequence: inbox.server_sequence,
            status: CommandStatus::Accepted,
            payload: inbox.payload,
        },
    );
    assert_eq!(
        ui::link(&app)?.quick_loot_inbox().0,
        Some(expected.fulfilment)
    );
    let mut sessions = [String::new(), String::new()];
    let mut previous = (inbox.command_id, inbox.server_sequence);
    let mut probes: Vec<_> = (1..=20).zip(steps).collect();
    if full {
        probes.splice(
            14..14,
            [
                (
                    21,
                    (
                        route(
                            None,
                            QuickLootRecipient::GoldPouch,
                            Some(expected.fulfilment),
                            false,
                        ),
                        &[][..],
                        None,
                    ),
                ),
                (
                    22,
                    (
                        Action::LootAll,
                        &east_refused[..],
                        Some("Looted 0 of 2 items"),
                    ),
                ),
                (23, (main.clone(), &[][..], None)),
            ],
        );
    }
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct LootTile {
        game_session_id: String,
        command_id: u64,
        server_sequence: u64,
        actor_x: i32,
        x: i32,
        y: i32,
        floor: i16,
        corpses: Vec<(i32, [u8; 16], u64)>,
    }
    let gold_pouch = route(
        Some(QuickLootCategory::Gold),
        QuickLootRecipient::GoldPouch,
        Some(expected.pouch),
        false,
    );
    probes.push((
        24,
        (
            route(
                Some(QuickLootCategory::Gold),
                QuickLootRecipient::GoldPouch,
                Some(expected.fulfilment),
                false,
            ),
            &[][..],
            None,
        ),
    ));
    for (key, (action, expected_items, notice)) in probes {
        let session_index = usize::from(key >= 12);
        let reply: Reply =
            serde_json::from_slice(&std::fs::read(directory.join(format!("i07-{key}.json")))?)?;
        if key == 12 {
            let restore: Reply = serde_json::from_slice(
                &std::fs::read(directory.join("i07-relog-restore.json")).map_err(|error| {
                    format!("I07 probe 12 requires the live relog restore capture: {error}")
                })?,
            )?;
            assert_eq!(restore.game_session_id, reply.game_session_id);
            assert_ne!(restore.game_session_id, sessions[0]);
            assert_eq!(restore.status, CommandStatus::Accepted as u32);
            previous = (restore.command_id, restore.server_sequence);
            peer::restore_active_capture(
                &outbox,
                peer::Exchange {
                    command: oteryn_client_runtime::server_mechanic::ServerMechanicCommand {
                        capability: CAPABILITY_QUICK_LOOT_DESTINATIONS_V1,
                        command_type: COMMAND_TYPE_QUICK_LOOT_DESTINATIONS,
                        payload: std::fs::read(directory.join("i07-relog-restore-request.bin"))?,
                    },
                    payload: restore.payload,
                    status: CommandStatus::Accepted,
                },
                previous,
            )?;
            let panel = ui::link(&app)?.quick_loot_panel();
            assert_eq!(panel.filter_mode, Some(accept));
            assert_eq!(panel.skipped, [expected.skip].into_iter().collect());
            assert_eq!(
                panel.accepted,
                [expected.gold, expected.accept].into_iter().collect()
            );
            assert_eq!(
                panel.destinations,
                [
                    route(None, QuickLootRecipient::MainContainer, None, true),
                    route(
                        Some(QuickLootCategory::Gold),
                        QuickLootRecipient::MainContainer,
                        None,
                        true
                    ),
                    armor_route(expected.pouch, true),
                ]
            );
            ui::open(&mut app)?;
        }
        let status = if matches!(key, 19 | 20) {
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
        if key == 1 || key == 12 {
            sessions[session_index].clone_from(&reply.game_session_id);
        }
        assert_eq!(
            reply.game_session_id, sessions[session_index],
            "I07 probe {key}"
        );
        if matches!(key, 13 | 20) {
            continue;
        }
        let dispatched = if key == 6 {
            ui::click_action(&mut app, Action::LootAll)?;
            true
        } else if action == Action::LootAll {
            let tile: LootTile = serde_json::from_slice(
                &std::fs::read(directory.join(format!("i07-{key}-tile.json"))).map_err(
                    |error| format!("I07 probe {key} requires the live clicked tile: {error}"),
                )?,
            )?;
            assert_eq!(
                tile.corpses.iter().filter(|(x, _, _)| *x == tile.x).count(),
                if key >= 15 { 2 } else { 1 },
                "I07 probe {key}"
            );
            assert_eq!(
                tile.game_session_id, reply.game_session_id,
                "I07 probe {key} tile session"
            );
            assert_eq!(
                (tile.command_id, tile.server_sequence),
                (reply.command_id, reply.server_sequence),
                "I07 probe {key} tile command"
            );
            assert_eq!(
                tile.corpses.len(),
                3,
                "I07 probe {key} competing corpse omitted"
            );
            if key >= 15 {
                let top = tile
                    .corpses
                    .iter()
                    .rfind(|(x, _, _)| *x == tile.x)
                    .ok_or("I07 missing east top corpse")?;
                assert_eq!(
                    top.1, expected.east_top,
                    "I07 probe {key} visible top corpse"
                );
            }
            outbox.entities(
                tile.corpses
                    .into_iter()
                    .map(
                        |(x, identity, generation)| oteryn_session::WorldSpatialEntity {
                            motion: None,
                            display_name: None,
                            kind: oteryn_session::EntityKind::Corpse,
                            entity: oteryn_session::EntityRef {
                                identity,
                                generation,
                            },
                            position: oteryn_session::ActorPosition {
                                x,
                                y: tile.y,
                                floor: tile.floor,
                            },
                            detail: oteryn_session::EntityDetail::Object {
                                item_definition_ref: expected.corpse_definition,
                                quantity: 1,
                                item_handle: None,
                            },
                        },
                    )
                    .collect(),
            );
            let actor = oteryn_renderer::TileCoord::new(tile.actor_x, tile.y);
            let click = oteryn_renderer::TileCoord::new(tile.x, tile.y);
            ui::click_corpse_tile(&mut app, actor, click, tile.floor)?;
            true
        } else if matches!(&action, Action::Select(_) | Action::Destination { .. }) && key != 19 {
            let control = format!("{action:?}");
            ui::click_action(&mut app, action)
                .map_err(|error| format!("I07 probe {key} control {control}: {error}"))?;
            true
        } else {
            let link = ui::link(&app)?;
            match action {
                Action::Add { mode, definition } => {
                    let handle = picker_inventory
                        .entries
                        .iter()
                        .find(|item| item.item_definition_ref.get() == definition)
                        .ok_or("I07 filter definition omitted from captured picker inventory")?
                        .handle;
                    ui::add_filter_item(&mut app, mode == skip, handle)?;
                    true
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
                _ => return Err("unexpected I07 replay action".into()),
            }
        };
        assert!(dispatched, "I07 probe {key}");
        let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
            return Err("I07 client queued the wrong command".into());
        };
        let captured = std::fs::read(directory.join(format!("i07-{key}-request.bin")))?;
        let wire = decode_quick_loot_request(&captured)
            .map_err(|error| format!("I07 probe {key} invalid captured request: {error:?}"))?;
        let command_type = wire.action.command_type();
        let capability = if command_type == COMMAND_TYPE_QUICK_LOOT_DESTINATIONS {
            CAPABILITY_QUICK_LOOT_DESTINATIONS_V1
        } else {
            CAPABILITY_QUICK_LOOT_V1
        };
        assert_eq!(
            (command.capability, command.command_type),
            (capability, command_type),
            "I07 probe {key}"
        );
        assert_eq!(command.payload, captured, "I07 probe {key}");
        let link = ui::link(&app)?;
        let notices = || {
            let mailbox = crate::play::mailbox::lock(&link.mailbox);
            let console = &mailbox.chat.console;
            console
                .notices
                .scoped_text(console.filters, console.display, NoticeScope::LootOnly)
        };
        let before = notices();
        let before_panel = link.discovery_panel().loot;
        crate::play::session_loop::route_server_mechanic_reply(
            &outbox,
            &command,
            &GatedCommandResult {
                command_id: reply.command_id,
                server_sequence: reply.server_sequence,
                status,
                payload: reply.payload,
            },
        );
        let panel = link.discovery_panel().loot;
        assert_eq!(panel.pending, None);
        if matches!(key, 19 | 20) {
            assert_eq!(panel.destinations, before_panel.destinations);
            assert_eq!(
                panel.filter_mode, before_panel.filter_mode,
                "I07 probe {key}"
            );
            assert_eq!(panel.skipped, before_panel.skipped, "I07 probe {key}");
            assert_eq!(panel.accepted, before_panel.accepted, "I07 probe {key}");
            assert_eq!(panel.result, before_panel.result);
            assert_eq!(notices(), before);
            assert_eq!(panel.feedback, "The server refused the request");
            continue;
        }
        assert_eq!(panel.feedback, "Confirmed by the server");
        let items = &panel
            .result
            .ok_or("I07 client omitted the loot result")?
            .items;
        assert_eq!(items.len(), expected_items.len());
        let mut actual: Vec<_> = items
            .iter()
            .map(|item| (item.item, item.accepted))
            .collect();
        let mut expected_items = expected_items.to_vec();
        if key == 6 {
            actual.sort_unstable();
            expected_items.sort_unstable();
        }
        assert_eq!(actual, expected_items, "I07 probe {key}");
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
        if key == 11 {
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
    let restored: Reply =
        serde_json::from_slice(&std::fs::read(directory.join("i07-restore.json"))?)?;
    assert_eq!(restored.game_session_id, expected.final_session);
    assert!(!sessions.contains(&restored.game_session_id));
    assert_ne!(restored.command_id, 0);
    assert_ne!(restored.server_sequence, 0);
    assert_eq!(restored.status, CommandStatus::Accepted as u32);
    let panel = peer::restore_capture(
        peer::Exchange {
            command: oteryn_client_runtime::server_mechanic::ServerMechanicCommand {
                capability: CAPABILITY_QUICK_LOOT_DESTINATIONS_V1,
                command_type: COMMAND_TYPE_QUICK_LOOT_DESTINATIONS,
                payload: std::fs::read(directory.join("i07-restore-request.bin"))?,
            },
            payload: restored.payload,
            status: CommandStatus::Accepted,
        },
        (restored.command_id, restored.server_sequence),
    )?;
    assert_eq!(panel.filter_mode, Some(skip));
    assert_eq!(panel.skipped, [expected.skip].into_iter().collect());
    assert_eq!(
        panel.accepted,
        [expected.accept, expected.gold].into_iter().collect()
    );
    let mut routes = vec![main.clone(), gold_pouch, armor_route(expected.pouch, false)];
    if full {
        routes.push(route(
            Some(QuickLootCategory::Food),
            QuickLootRecipient::MainContainer,
            None,
            false,
        ));
    }
    assert_eq!(panel.destinations, routes);
    println!(
        "I07_NATIVE_REPLAY session={} relog={} final={}",
        sessions[0], sessions[1], expected.final_session
    );
    Ok(())
}
