#[test]
fn proficiency_visual_selection_uses_named_server_choices_and_exact_track_revision()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::play_channel;
    use oteryn_client_runtime::weapon_proficiency::{
        ModificationOperation, ModificationReply, ModificationRequest, ModificationResult,
        ModifiedSlot, ProficiencyCatalogueEntry, ProficiencyCatalogueLevel,
        ProficiencyCataloguePage,
    };
    use oteryn_client_runtime::weapon_proficiency_state::{Slot, State};
    let (link, mut commands, outbox) = play_channel();
    outbox.weapon_proficiency_initialize(true);
    outbox.weapon_proficiency_state_initialize(true);
    link.weapon_proficiency_request(Intent::Catalogue);
    commands.try_recv()?;
    let catalogue = ProficiencyCataloguePage {
        offset: 0,
        total: 1,
        entries: vec![ProficiencyCatalogueEntry {
            definition: 258,
            weapon: "oteryn:item.test_sword".into(),
            name: "Test sword".into(),
            experience: 90000,
            thresholds: [1; 9],
            levels: vec![ProficiencyCatalogueLevel {
                selected: Some(0),
                perks: vec!["Attack +1".into(), "Defence +2".into()],
            }],
        }],
    };
    outbox.weapon_proficiency_result(&GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: catalogue.encode().ok_or("catalogue encode")?,
    });
    link.weapon_proficiency_select_weapon(0);
    let crate::combat_input::PlayCommand::Feature(query) = commands.try_recv()? else {
        return Err("missing shaping state request".into());
    };
    assert_eq!(
        (query.capability, query.kind, query.payload),
        (448, 732, 258u32.to_be_bytes().to_vec())
    );
    let state = State {
        definition: 258,
        revision: 123,
        slots: [
            Some(Slot {
                level: 0,
                rank: 2,
                current: "Attack +2".into(),
                next: "Attack +3".into(),
                maximum: "Attack +10".into(),
                offers: vec!["Health +3".into(), "Defence +3".into(), "Skill +3".into()],
            }),
            None,
        ],
        modify_dust: [250, 1000],
        rank_dust: [25; 10],
        reshape_dust: 100,
        clear_dust: 10,
        orbs: 1,
    };
    outbox.weapon_proficiency_result(&GatedCommandResult {
        command_id: 2,
        status: CommandStatus::Accepted,
        server_sequence: 2,
        payload: state.encode().ok_or("state encode")?,
    });
    let mut panel = link.weapon_proficiency_panel();
    assert!(native_controls::choose(&mut panel, 2, 1));
    assert!(!native_controls::choose(&mut panel, 2, 2));
    assert!(native_controls::visual_submit(&mut panel, 102, false).is_none());
    assert!(native_controls::choose(&mut panel, 5, 1));
    panel.revision = 999; // A stale draft field cannot override the server observation.
    let command = native_controls::visual_submit(&mut panel, 106, false).ok_or("choose offer")?;
    let decoded =
        ModificationRequest::decode(command.kind, &command.payload).ok_or("decode choose")?;
    assert_eq!(decoded.expected_revision, 123);
    assert_eq!(decoded.definition.get(), 258);
    assert_eq!(
        decoded.operation,
        ModificationOperation::ReshapeChoose { choice: Some(1) }
    );
    assert!(native_controls::visual_submit(&mut panel, 106, false).is_none());
    assert!(apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 3,
            status: CommandStatus::Accepted,
            server_sequence: 3,
            payload: ModificationReply {
                result: ModificationResult::Accepted,
                slot: Some(ModifiedSlot {
                    level: 0,
                    entry_index: 3,
                    rank: 2,
                    pending_offer: None
                })
            }
            .encode()
        }
    ));
    assert!(panel.shaping.is_none());
    assert_eq!(panel.revision, 0);
    assert!(panel.state_refresh_needed);
    assert!(native_controls::visual_submit(&mut panel, 103, false).is_none());
    assert_eq!(
        native_controls::visual_submit(&mut panel, 116, false)
            .ok_or("refresh")?
            .kind,
        732
    );
    Ok(())
}
