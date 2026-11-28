use super::*;

#[test]
fn imbuement_activity_handoff_discards_old_item_and_material_reply() -> Result<(), Error> {
    use oteryn_client_runtime::{imbuement_details as wire, raw_feature::GatedCommandResult};
    let (link, _receiver, _outbox) = play_channel();
    let item = NonZeroU64::new(7).ok_or("item")?;
    let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
    mailbox.imbue.selected = true;
    mailbox.imbue.details_selected = true;
    mailbox.imbue.item = Some(item);
    mailbox.imbue.window = Some(imbuement::ImbuementWindow {
        slot_count: 1,
        applied: vec![],
        offers: vec![],
        clear_gold: 15000,
    });
    let command = oteryn_client_ui::imbue::details::request(&mut mailbox.imbue).ok_or("query")?;
    mailbox.cancel_activity_requests();
    assert!(mailbox.imbue.selected && mailbox.imbue.details_selected);
    assert!(mailbox.imbue.item.is_none() && mailbox.imbue.window.is_none());
    assert!(mailbox.imbue.details_pending.is_none());
    oteryn_client_ui::imbue::details::apply(
        &mut mailbox.imbue,
        &command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_client_runtime::raw_feature::CommandStatus::Accepted,
            payload: wire::encode_reply(&wire::Reply {
                item,
                quotes: vec![],
            })
            .map_err(|e| format!("{e:?}"))?,
        },
    );
    assert!(mailbox.imbue.details.is_none());
    drop(mailbox);
    Ok(())
}

/// Exercises the production live picker and the actual session-loop Feature reply route.
#[test]
fn imbuement_live_picker_and_material_reply_reach_run_session() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let requests = Arc::new(Mutex::new(Vec::new()));
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer {
                        requests: requests.clone(),
                        details: true,
                        catalogue: false,
                    },
                    incoming,
                    outbox,
                ));
                ready(&link, &requests, 0).await?;
                tokio::time::timeout(Duration::from_secs(3), async {
                    while crate::play::feature_0068_imbue::input::carried_items(
                        &crate::play::mailbox::lock(&link.mailbox),
                    )
                    .is_empty()
                    {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(!link.imbue_carried_item(NonZeroU64::new(999).ok_or("unknown item")?));
                assert!(
                    requests
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .is_empty()
                );
                assert!(link.imbue_carried_item(NonZeroU64::new(42).ok_or("carried item")?));
                ready(&link, &requests, 1).await?;
                assert_eq!(
                    requests.lock().unwrap_or_else(PoisonError::into_inner)[0]
                        .item
                        .get(),
                    42
                );
                assert!(link.imbue_panel().details_selected);
                link.imbue_details_request();
                tokio::time::timeout(Duration::from_secs(3), async {
                    while link.imbue_panel().details.is_none() {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(link.imbue_slot(1));
                let panel = link.imbue_panel();
                assert!(panel.details_pending.is_none());
                assert!(panel.can_apply());
                let text = oteryn_client_ui::imbue::selected_detail_text(&panel);
                assert!(text.contains("25"));
                assert!(text.contains("i11466"));
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Imbuement live route test thread panicked")?
}

fn verify_p21_query(command: &ImbueCommand, item: NonZeroU64, expected: &str) -> Result<(), Error> {
    if command.0.item != item || command.0.action != imbuement::ImbuementAction::Query {
        return Err("shrine selection did not query the captured Sword handle".into());
    }
    let bytes =
        imbuement::encode_imbuement_request(&command.0).map_err(|error| format!("{error:?}"))?;
    let encoded: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    if encoded != expected {
        return Err("shrine query does not match the live request bytes".into());
    }
    Ok(())
}

#[test]
fn p21_query_verifier_rejects_wrong_target() -> Result<(), Error> {
    let item = NonZeroU64::MIN;
    let request = ImbueCommand(imbuement::ImbuementRequest {
        item,
        action: imbuement::ImbuementAction::Query,
    });
    let bytes =
        imbuement::encode_imbuement_request(&request.0).map_err(|error| format!("{error:?}"))?;
    let expected: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    verify_p21_query(&request, item, &expected)?;
    let wrong = ImbueCommand(imbuement::ImbuementRequest {
        item: NonZeroU64::MAX,
        action: imbuement::ImbuementAction::Query,
    });
    assert!(verify_p21_query(&wrong, item, &expected).is_err());
    assert!(verify_p21_query(&request, item, "").is_err());
    Ok(())
}

/// Replays the carried-item click in the production shrine panel against the live request.
#[test]
#[ignore = "requires a live P21 inventory and request capture"]
fn p21_live_shrine_selection_replays_exact_query() -> Result<(), Error> {
    use crate::combat_input::PlayCommand;
    use crate::play::feature_0068_imbue::input::ImbueViewAction;
    use oteryn_protocol_oteryn::item_view::decode_character_inventory_with_appearances;

    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("P21 live capture directory missing")?;
    let metadata = std::fs::read_to_string(std::path::Path::new(&root).join("p21-expected.txt"))?;
    let field = |key: &str| {
        metadata
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once('=')?;
                (name == key).then_some(value)
            })
            .ok_or_else(|| format!("P21 live metadata missing {key}"))
    };
    let item: NonZeroU64 = field("item_handle")?.parse()?;
    let definition = field("item_definition")?.parse()?;
    let expected = field("query_hex")?;
    let inventory_hex = field("inventory_hex")?;
    let mut pairs = inventory_hex.as_bytes().chunks_exact(2);
    let bytes = pairs
        .by_ref()
        .map(|pair| Ok::<_, Error>(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect::<Result<Vec<_>, _>>()?;
    assert!(pairs.remainder().is_empty());
    let inventory = decode_character_inventory_with_appearances(&bytes, true, true)
        .map_err(|error| format!("P21 inventory decode: {error:?}"))?;
    let mut sword = inventory
        .entries
        .iter()
        .filter(|entry| entry.item_definition_ref == definition);
    let entry = sword
        .next()
        .ok_or("P21 Sword absent from captured inventory")?;
    assert_eq!((entry.handle, entry.count.get()), (item, 1));
    assert!(sword.next().is_none());
    let (link, mut commands, _outbox) = play_channel();
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.imbue.selected = true;
        mailbox.depot.items.inventory = inventory;
    }
    link.open_imbue();
    let panel = link.imbue_panel();
    assert!(panel.open && panel.item.is_none() && panel.window.is_none());
    assert!(matches!(
        commands.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));

    let absent = NonZeroU64::MAX;
    assert_ne!(item, absent);
    assert!(!link.imbue_dispatch_bound(&panel, ImbueViewAction::Item(absent)));
    assert_eq!(link.imbue_panel(), panel);
    assert!(matches!(
        commands.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));

    assert!(link.imbue_dispatch_bound(&panel, ImbueViewAction::Item(item)));
    let PlayCommand::Imbue(command) = commands.try_recv()? else {
        return Err("shrine selection did not enqueue an imbuement command".into());
    };
    verify_p21_query(&command, item, expected)?;
    assert!(matches!(
        commands.try_recv(),
        Err(tokio::sync::mpsc::error::TryRecvError::Empty)
    ));
    Ok(())
}
