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
