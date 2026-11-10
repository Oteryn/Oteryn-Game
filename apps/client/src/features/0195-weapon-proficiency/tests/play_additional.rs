use super::*;

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

#[test]
fn proficiency_unknown_paid_reply_and_scope_change_never_repeat_a_mutation() {
    let mut panel = WeaponProficiencyPanel {
        selected: true,
        state_selected: true,
        weapon: 258,
        pending: true,
        command_kind: 380,
        ..Default::default()
    };
    assert!(!apply_result(
        &mut panel,
        &GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![255]
        }
    ));
    assert!(panel.mutation_uncertain);
    assert_eq!(panel.revision, 0);
    assert!(request(&mut panel, Intent::RankUp).is_none());
    assert!(request(&mut panel, Intent::State).is_some());
    panel.pending = true;
    panel.command_kind = 379;
    panel.mutation_uncertain = false;
    panel.invalidate_observation();
    assert!(panel.mutation_uncertain);
    assert!(!panel.pending);
    assert!(request(&mut panel, Intent::Modify).is_none());
}

#[test]
fn proficiency_catalogue_keeps_all_pages_and_presents_mastery_tree()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::play_channel;
    use oteryn_client_runtime::weapon_proficiency::{
        ProficiencyCatalogueEntry, ProficiencyCatalogueLevel, ProficiencyCataloguePage,
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.weapon_proficiency_initialize(true);
    let entry = |definition, name: &str, experience| ProficiencyCatalogueEntry {
        definition,
        weapon: name.into(),
        name: name.into(),
        experience,
        thresholds: [10, 20, 30, 40, 50, 60, 70, 80, 90],
        levels: vec![ProficiencyCatalogueLevel {
            selected: Some(0),
            perks: vec!["Attack +1".into()],
        }],
    };
    for (offset, entries) in [
        (0u32, vec![entry(258, "Sword", 30)]),
        (1, vec![entry(259, "Axe", 0)]),
    ] {
        if offset == 0 {
            link.weapon_proficiency_request(Intent::Catalogue);
        }
        link.weapon_proficiency_tick(std::time::Instant::now());
        let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("catalogue command".into());
        };
        assert_eq!(command.payload, offset.to_be_bytes());
        outbox.weapon_proficiency_result(&GatedCommandResult {
            command_id: u64::from(offset) + 1,
            status: CommandStatus::Accepted,
            server_sequence: u64::from(offset) + 1,
            payload: ProficiencyCataloguePage {
                offset,
                total: 2,
                entries,
            }
            .encode()
            .ok_or("encode")?,
        });
    }
    let mut panel = link.weapon_proficiency_panel();
    assert_eq!(
        panel.catalogue.as_ref().ok_or("catalogue")?.entries.len(),
        2
    );
    let text = native_controls::display(&panel);
    assert!(text.contains("Mastery"));
    assert!(text.contains("Summary"));
    assert!(text.contains("Perk tree"));
    assert_eq!(panel.visible_weapons(), vec![1, 0]);
    panel.sort_progress = true;
    assert_eq!(panel.visible_weapons(), vec![0, 1]);
    panel.progress_filter = 2;
    assert_eq!(panel.visible_weapons(), vec![0]);
    panel.progress_filter = 3;
    assert_eq!(panel.visible_weapons(), vec![1]);
    panel.progress_filter = 1;
    assert!(panel.visible_weapons().is_empty());
    panel.progress_filter = 0;
    panel.search = "SWO".into();
    assert_eq!(panel.visible_weapons(), vec![0]);
    assert!(native_controls::choose(&mut panel, 0, 0));
    assert!(native_controls::choose(&mut panel, 6, 1));
    crate::play::mailbox::lock(&link.mailbox).weapon_proficiency = panel;
    link.weapon_proficiency_request(Intent::SelectPerk);
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("perk command".into());
    };
    let selected =
        oteryn_client_runtime::weapon_proficiency::SelectPerkRequest::decode(&command.payload)
            .ok_or("perk decode")?;
    assert_eq!(
        (selected.definition.get(), selected.level, selected.perk),
        (258, 0, Some(0))
    );

    Ok(())
}

#[test]
fn proficiency_clear_requires_explicit_confirmation() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::weapon_proficiency::{
        ProficiencyCatalogueEntry, ProficiencyCatalogueLevel, ProficiencyCataloguePage,
    };
    use oteryn_client_runtime::weapon_proficiency_state::{Slot, State};
    let mut panel = WeaponProficiencyPanel {
        selected: true,
        state_selected: true,
        weapon: 258,
        slot: 1,
        catalogue: Some(ProficiencyCataloguePage {
            offset: 0,
            total: 1,
            entries: vec![ProficiencyCatalogueEntry {
                definition: 258,
                weapon: "Sword".into(),
                name: "Sword".into(),
                experience: 30,
                thresholds: [10, 20, 30, 40, 50, 60, 70, 80, 90],
                levels: vec![ProficiencyCatalogueLevel {
                    selected: Some(0),
                    perks: vec!["Attack +1".into()],
                }],
            }],
        }),
        shaping: Some(State {
            definition: 258,
            revision: 123,
            slots: [
                Some(Slot {
                    level: 0,
                    rank: 2,
                    current: "Attack +2".into(),
                    next: "Attack +3".into(),
                    maximum: "Attack +10".into(),
                    offers: vec![],
                }),
                None,
            ],
            modify_dust: [250, 1000],
            rank_dust: [25; 10],
            reshape_dust: 100,
            clear_dust: 10,
            orbs: 1,
        }),
        ..Default::default()
    };
    assert!(native_controls::visual_submit(&mut panel, 108, false).is_none());
    assert!(native_controls::display(&panel).contains("╔"));
    assert!(!panel.pending);
    assert!(panel.feedback.is_some_and(|text| text.contains("Confirm")));
    let (link, mut commands, _) = crate::play::play_channel();
    crate::play::mailbox::lock(&link.mailbox).weapon_proficiency = panel;
    link.weapon_proficiency_request(Intent::CancelClear);
    link.weapon_proficiency_request(Intent::ConfirmClear);
    assert!(commands.try_recv().is_err());
    link.weapon_proficiency_request(Intent::Clear);
    link.weapon_proficiency_tick(std::time::Instant::now() + std::time::Duration::from_secs(10));
    assert!(commands.try_recv().is_err()); // Polling cannot dismiss a confirmation.
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        if let Some(state) = &mut mailbox.weapon_proficiency.shaping {
            state.revision = 124;
        }
    }
    link.weapon_proficiency_request(Intent::ConfirmClear);
    assert!(commands.try_recv().is_err());
    link.weapon_proficiency_request(Intent::Clear);
    link.weapon_proficiency_request(Intent::ConfirmClear);
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("clear command".into());
    };
    let clear = oteryn_client_runtime::weapon_proficiency::ModificationRequest::decode(
        command.kind,
        &command.payload,
    )
    .ok_or("clear decode")?;
    assert_eq!(
        (clear.definition.get(), clear.slot, clear.expected_revision),
        (258, 1, 124)
    );
    assert_eq!(
        clear.operation,
        oteryn_client_runtime::weapon_proficiency::ModificationOperation::Clear
    );
    link.weapon_proficiency_request(Intent::ConfirmClear);
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn proficiency_advancement_is_detected_from_real_play_results()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::play::play_channel;
    use oteryn_client_runtime::weapon_proficiency::{
        ProficiencyCatalogueEntry, ProficiencyCatalogueLevel, ProficiencyCataloguePage,
        ProficiencyPage, ProficiencyRow,
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.weapon_proficiency_initialize(true);
    outbox.weapon_proficiency_state_initialize(true);
    link.weapon_proficiency_request(Intent::Catalogue);
    commands.try_recv()?;
    outbox.weapon_proficiency_result(&GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: ProficiencyCataloguePage {
            offset: 0,
            total: 1,
            entries: vec![ProficiencyCatalogueEntry {
                definition: 258,
                weapon: "Sword".into(),
                name: "Sword".into(),
                experience: 0,
                thresholds: [10, 20, 30, 40, 50, 60, 70, 80, 90],
                levels: vec![ProficiencyCatalogueLevel {
                    selected: None,
                    perks: vec!["Attack +1".into()],
                }],
            }],
        }
        .encode()
        .ok_or("catalogue")?,
    });
    link.weapon_proficiency_request(Intent::Browse);
    commands.try_recv()?;
    outbox.weapon_proficiency_result(&GatedCommandResult {
        command_id: 2,
        status: CommandStatus::Accepted,
        server_sequence: 2,
        payload: ProficiencyPage {
            offset: 0,
            total: 1,
            rows: vec![ProficiencyRow {
                definition: std::num::NonZeroU32::new(258).ok_or("definition")?,
                name: "Sword".into(),
                experience: 20,
                level: 2,
                selections: vec![None],
            }],
        }
        .encode()
        .ok_or("page")?,
    });
    assert!(native_controls::display(&link.weapon_proficiency_panel()).contains("Advancement"));
    link.weapon_proficiency_open_track(true);
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("notification open".into());
    };
    assert_eq!(
        (command.capability, command.kind, command.payload),
        (448, 732, 258u32.to_be_bytes().to_vec())
    );
    let panel = link.weapon_proficiency_panel();
    assert!(panel.open);
    assert_eq!(panel.weapon, 258);
    assert!(panel.advancements.is_empty());
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.weapon_proficiency.pending = false;
    }
    link.weapon_proficiency_open_track(false);
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("progress bar open".into());
    };
    assert_eq!(
        (command.kind, command.payload),
        (732, 258u32.to_be_bytes().to_vec())
    );

    Ok(())
}
