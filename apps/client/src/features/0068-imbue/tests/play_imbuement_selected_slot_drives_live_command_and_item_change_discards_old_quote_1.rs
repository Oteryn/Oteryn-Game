#[test]
fn imbuement_selected_slot_drives_live_command_and_item_change_discards_old_quote()
-> Result<(), Error> {
    let (link, mut receiver, outbox) = play_channel();
    outbox.imbue_initialize(true);
    let item = NonZeroU64::new(42).ok_or("item")?;
    link.imbue_item(item);
    outbox.imbue_result(&ImbueResponse {
        status: CommandStatus::Accepted,
        reply: Some(imbuement::ImbuementReply {
            result: imbuement::ImbuementResult::Ok,
            window: Some(imbuement::ImbuementWindow {
                slot_count: 2,
                applied: vec![imbuement::AppliedImbuement {
                    slot: 0,
                    family: 2,
                    tier: 1,
                    remaining_ms: 5000,
                }],
                offers: vec![imbuement::ImbuementOffer {
                    family: 3,
                    tier: 2,
                    gold: 400,
                    requires_premium: true,
                }],
                clear_gold: 75,
            }),
        }),
    });
    let loaded = link
        .imbue_panel()
        .window
        .clone()
        .ok_or("loaded shrine window")?;
    crate::play::mailbox::lock(&link.mailbox)
        .imbue
        .details_selected = true;
    link.imbue_details_request();
    link.imbue_details_request();
    let crate::combat_input::PlayCommand::Feature(details_command) = receiver.try_recv()? else {
        return Err("missing material query".into());
    };
    assert!(receiver.try_recv().is_err());
    use oteryn_client_runtime::imbuement_details as details;
    assert_eq!(
        details::decode_query(&details_command.payload).map_err(|e| format!("{e:?}"))?,
        item
    );
    let detail_reply = details::Reply {
        item,
        quotes: vec![details::Quote {
            family: 3,
            tier: 2,
            gold: 400,
            requires_premium: true,
            recipe_available: true,
            success_percent: 100,
            materials: vec![
                details::Material {
                    definition_key: "oteryn:item.tibia.i11466".into(),
                    quantity: 25,
                },
                details::Material {
                    definition_key: "oteryn:item.tibia.i22007".into(),
                    quantity: 20,
                },
            ],
        }],
    };
    outbox.properties_result(
        &details_command,
        &oteryn_client_runtime::raw_feature::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: details::encode_reply(&detail_reply).map_err(|e| format!("{e:?}"))?,
        },
    );
    assert!(link.imbue_slot(1));
    assert!(link.imbue_offer(0));
    assert!(
        oteryn_client_ui::imbue::selected_detail_text(&link.imbue_panel())
            .contains("Success chance: 100%")
    );
    assert!(
        oteryn_client_ui::imbue::selected_detail_text(&link.imbue_panel())
            .contains("Apply cost: 400 gold")
    );
    assert!(
        oteryn_client_ui::imbue::selected_detail_text(&link.imbue_panel())
            .contains("25 × oteryn:item.tibia.i11466")
    );
    assert!(link.imbue_panel().can_apply());
    assert_eq!(
        oteryn_client_ui::imbue::slot_label(&link.imbue_panel(), 1),
        "Slot 2: empty"
    );
    link.imbue_request(Intent::Apply);
    let crate::combat_input::PlayCommand::Imbue(command) = receiver.try_recv()? else {
        return Err("missing apply command".into());
    };
    assert_eq!(
        command.0,
        imbuement::ImbuementRequest {
            item,
            action: imbuement::ImbuementAction::Apply {
                slot: 1,
                imbuement: 0
            }
        }
    );
    assert!(!link.imbue_slot(0));
    outbox.imbue_result(&ImbueResponse {
        status: CommandStatus::Accepted,
        reply: Some(imbuement::ImbuementReply {
            result: imbuement::ImbuementResult::Ok,
            window: None,
        }),
    });
    assert!(link.imbue_panel().window.is_none());
    assert!(link.imbue_panel().refresh_required);
    link.imbue_request(Intent::Refresh);
    let crate::combat_input::PlayCommand::Imbue(refresh) = receiver.try_recv()? else {
        return Err("missing post-mutation refresh".into());
    };
    assert_eq!(refresh.0.action, imbuement::ImbuementAction::Query);
    outbox.imbue_result(&ImbueResponse {
        status: CommandStatus::Accepted,
        reply: Some(imbuement::ImbuementReply {
            result: imbuement::ImbuementResult::Ok,
            window: Some(loaded),
        }),
    });
    assert!(link.imbue_panel().window.is_some());
    link.imbue_item(NonZeroU64::new(43).ok_or("new item")?);
    assert!(link.imbue_panel().window.is_none());
    link.imbue_request(Intent::Apply);
    assert!(receiver.try_recv().is_err());
    outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    assert!(!link.imbue_panel().selected);
    assert!(link.imbue_panel().details.is_none());
    link.imbue_request(Intent::Refresh);
    assert!(receiver.try_recv().is_err());
    Ok(())
}
