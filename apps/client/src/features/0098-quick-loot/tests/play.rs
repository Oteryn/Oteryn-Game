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
fn managed_container_reset_uses_the_selected_category_and_existing_wire() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_ui::quick_loot::{Action, QuickLootCategory, QuickLootRecipient};
    let (link, mut commands, _) = crate::play::play_channel();
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.discovery.loot.selected = true;
        mailbox.discovery.loot.destinations.push(Action::Destination {
            category: Some(QuickLootCategory::Gold), recipient: QuickLootRecipient::Container,
            id: Some([1; 16]), fallback: false,
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
