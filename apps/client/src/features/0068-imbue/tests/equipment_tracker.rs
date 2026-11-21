use crate::combat_input::PlayCommand;
use crate::play::{mailbox, play_channel};
use oteryn_client_runtime::equipment::{EquipmentSlot, EquippedItem, ItemEntry};
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
use oteryn_client_ui::inspection::{InspectedItem, Reply, Request, decode_request, encode_reply};
use std::num::{NonZeroU32, NonZeroU64};

#[test]
fn equipment_tracker_reads_real_inspection_commands_and_prunes_unequipped_items()
-> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    {
        let mut m = mailbox::lock(&outbox.0);
        m.discovery.inspection.selected = true;
        m.imbue.tracker_open = true;
        for (slot, number) in [(EquipmentSlot::Head, 0), (EquipmentSlot::Armor, 1)] {
            m.depot.items.inventory.equipment.push(EquippedItem {
                slot,
                item: ItemEntry {
                    appearance_id: 0,
                    handle: NonZeroU64::MIN.saturating_add(number),
                    item_definition_ref: NonZeroU32::MIN.saturating_add(number as u32),
                    count: NonZeroU32::MIN,
                    sub_type: 0,
                },
            });
        }
    }
    for (number, name, time) in [(0, "Helmet", "00:04"), (1, "Plate", "00:10")] {
        assert!(link.imbue_tracker_refresh());
        let Ok(PlayCommand::ServerMechanic(command)) = commands.try_recv() else {
            return Err("missing inspection".into());
        };
        assert_eq!((command.capability, command.command_type), (132, 153));
        assert_eq!(
            decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
            Request::Item(NonZeroU64::MIN.saturating_add(number))
        );
        outbox.discovery_result(
            132,
            &ServerMechanicResponse {
                command_id: number + 1,
                server_sequence: number + 1,
                status: CommandStatus::Accepted,
                payload: encode_reply(&Reply {
                    catalogue: None,
                    status: 1,
                    requests: vec![],
                    items: vec![InspectedItem {
                        slot: 0,
                        definition_ref: number as u32 + 1,
                        count: 1,
                        tier: 0,
                        name: name.into(),
                        attributes: vec![(
                            "Imbuement Slot 1".into(),
                            format!(
                                "Basic Strike (critical damage), lasts {time}h while fighting."
                            ),
                        )],
                    }],
                })
                .map_err(|e| format!("{e:?}"))?,
            },
        );
    }
    let text = oteryn_client_ui::imbue::tracker_text(&link.imbue_panel());
    assert!(text.contains("Helmet"));
    assert!(text.contains("Plate"));
    assert!(text.contains("LOW TIME"));
    mailbox::lock(&outbox.0)
        .depot
        .items
        .inventory
        .equipment
        .retain(|e| e.slot != EquipmentSlot::Head);
    assert!(link.imbue_tracker_refresh());
    let text = oteryn_client_ui::imbue::tracker_text(&link.imbue_panel());
    assert!(!text.contains("Helmet"));
    assert!(!text.contains("LOW TIME"));
    Ok(())
}

#[test]
fn equipped_imbuement_widget_shortcut_and_time_filters_use_the_confirmed_feed() -> Result<(), String>
{
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (link, mut commands, outbox) = play_channel();
    {
        let mut m = mailbox::lock(&outbox.0);
        m.discovery.inspection.selected = true;
        for (slot, number) in [(EquipmentSlot::Head, 0), (EquipmentSlot::Armor, 1)] {
            m.depot.items.inventory.equipment.push(EquippedItem {
                slot,
                item: ItemEntry {
                    appearance_id: 0,
                    handle: NonZeroU64::MIN.saturating_add(number),
                    item_definition_ref: NonZeroU32::MIN.saturating_add(number as u32),
                    count: NonZeroU32::MIN,
                    sub_type: 0,
                },
            });
        }
    }
    assert!(
        link.imbue_input(&[NormalizedInputEvent::Key {
            code: KeyCode::new(12).map_err(|e| format!("{e:?}"))?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE
                .with(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift),
            repeat: false,
        }])
    );
    assert!(link.imbue_panel().tracker_open);
    assert!(!link.imbue_panel().open);
    for (number, name, time) in [(0, "Helmet", "00:04"), (1, "Plate", "00:10")] {
        if number != 0 {
            assert!(link.imbue_tracker_refresh());
        }
        let Ok(PlayCommand::ServerMechanic(command)) = commands.try_recv() else {
            return Err("inspection query".into());
        };
        assert_eq!(
            decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
            Request::Item(NonZeroU64::MIN.saturating_add(number))
        );
        outbox.discovery_result(
            132,
            &ServerMechanicResponse {
                command_id: number + 1,
                server_sequence: number + 1,
                status: CommandStatus::Accepted,
                payload: encode_reply(&Reply {
                    catalogue: None,
                    status: 1,
                    requests: vec![],
                    items: vec![InspectedItem {
                        slot: 0,
                        definition_ref: number as u32 + 1,
                        count: 1,
                        tier: 0,
                        name: name.into(),
                        attributes: vec![(
                            "Imbuement Slot 1".into(),
                            format!(
                                "Basic Strike (critical damage), lasts {time}h while fighting."
                            ),
                        )],
                    }],
                })
                .map_err(|e| format!("{e:?}"))?,
            },
        );
    }
    assert!(link.imbue_tracker_filter(" 5 "));
    let text = oteryn_client_ui::imbue::tracker_text(&link.imbue_panel());
    assert!(text.contains("Helmet"));
    assert!(!text.contains("Plate"));
    assert!(link.imbue_tracker_filter("0"));
    assert!(oteryn_client_ui::imbue::tracker_text(&link.imbue_panel()).contains("Plate"));
    assert!(!link.imbue_tracker_filter("bad"));
    assert!(oteryn_client_ui::imbue::tracker_text(&link.imbue_panel()).contains("Enter minutes"));
    assert_eq!(link.imbue_panel().equipment.records.len(), 2);
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn equipment_tracker_discards_an_inflight_reply_after_unequip() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    {
        let mut m = mailbox::lock(&outbox.0);
        m.discovery.inspection.selected = true;
        m.imbue.tracker_open = true;
        m.depot.items.inventory.equipment.push(EquippedItem {
            slot: EquipmentSlot::Head,
            item: ItemEntry {
                appearance_id: 0,
                handle: NonZeroU64::MIN,
                item_definition_ref: NonZeroU32::MIN,
                count: NonZeroU32::MIN,
                sub_type: 0,
            },
        });
    }
    assert!(link.imbue_tracker_refresh());
    assert!(matches!(
        commands.try_recv(),
        Ok(PlayCommand::ServerMechanic(_))
    ));
    mailbox::lock(&outbox.0)
        .depot
        .items
        .inventory
        .equipment
        .clear();
    outbox.discovery_result(
        132,
        &ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: encode_reply(&Reply {
                catalogue: None,
                status: 1,
                requests: vec![],
                items: vec![InspectedItem {
                    slot: 0,
                    definition_ref: 1,
                    count: 1,
                    tier: 0,
                    name: "Old helmet".into(),
                    attributes: vec![(
                        "Imbuement Slot 1".into(),
                        "Basic Strike (critical damage), lasts 00:01h while fighting.".into(),
                    )],
                }],
            })
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    let p = link.imbue_panel();
    assert!(p.equipment.records.is_empty());
    assert!(p.equipment.pending.is_none());
    assert!(!oteryn_client_ui::imbue::tracker_text(&p).contains("Old helmet"));
    Ok(())
}

#[test]
fn equipment_tracker_filters_each_slot_through_inspection_and_playlink() -> Result<(), String> {
    let (link, mut commands, outbox) = play_channel();
    {
        let mut m = mailbox::lock(&outbox.0);
        m.discovery.inspection.selected = true;
        m.imbue.tracker_open = true;
        for (slot, number) in [(EquipmentSlot::Head, 0), (EquipmentSlot::Armor, 1)] {
            m.depot.items.inventory.equipment.push(EquippedItem {
                slot,
                item: ItemEntry {
                    appearance_id: 0,
                    handle: NonZeroU64::MIN.saturating_add(number),
                    item_definition_ref: NonZeroU32::MIN.saturating_add(number as u32),
                    count: NonZeroU32::MIN,
                    sub_type: 0,
                },
            });
        }
    }
    for (number, name, time) in [(0, "Helmet", "00:04"), (1, "Plate", "00:10")] {
        assert!(link.imbue_tracker_refresh());
        let Ok(PlayCommand::ServerMechanic(command)) = commands.try_recv() else {
            return Err("missing inspection".into());
        };
        assert_eq!((command.capability, command.command_type), (132, 153));
        assert_eq!(
            decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
            Request::Item(NonZeroU64::MIN.saturating_add(number))
        );
        outbox.discovery_result(
            132,
            &ServerMechanicResponse {
                command_id: number + 1,
                server_sequence: number + 1,
                status: CommandStatus::Accepted,
                payload: encode_reply(&Reply {
                    catalogue: None,
                    status: 1,
                    requests: vec![],
                    items: vec![InspectedItem {
                        slot: 0,
                        definition_ref: number as u32 + 1,
                        count: 1,
                        tier: 0,
                        name: name.into(),
                        attributes: vec![
                            (
                                "Imbuement Slot 2".into(),
                                "Powerful Void, lasts 04:00h while fighting.".into(),
                            ),
                            ("Imbuement Slot 3".into(), "empty".into()),
                            (
                                "Imbuement Slot 1".into(),
                                format!(
                                    "Basic Strike (critical damage), lasts {time}h while fighting."
                                ),
                            ),
                        ],
                    }],
                })
                .map_err(|e| format!("{e:?}"))?,
            },
        );
    }
    for group in [0, 1, 2, 3] {
        link.imbue_tracker_toggle_group(group);
        let text = oteryn_client_ui::imbue::tracker_text(&link.imbue_panel());
        assert_eq!(text.contains("10 minutes"), group != 0, "group {group}");
        assert_eq!(text.contains("240 minutes"), group != 2, "group {group}");
        assert_eq!(
            text.contains("unused imbuement slot"),
            group != 3,
            "group {group}"
        );
        link.imbue_tracker_toggle_group(group);
    }
    Ok(())
}
