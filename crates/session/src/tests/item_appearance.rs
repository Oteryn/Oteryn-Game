use super::*;

#[test]
fn item_appearance_snapshots_and_deltas_require_negotiated_499() -> Result<(), BoxError> {
    block_on(async {
        for appearances in [false, true] {
            let selected: &'static [u32] = if appearances { &[4, 6, 499] } else { &[4, 6] };
            let advertised = if appearances {
                CLIENT_SUPPORTED_CAPABILITIES
            } else {
                ITEM_SELECTED
            };
            let mut item = entry(1, 17, 3);
            item.appearance_id = 3031;
            let inventory = CharacterInventory {
                main_backpack: Some(item),
                ..CharacterInventory::default()
            };
            let encode = move |value| {
                item_view::encode_character_inventory_with_appearances(value, false, appearances)
                    .map_err(|e| format!("{e:?}"))
            };
            let initial = encode(&inventory)?;
            if !appearances {
                assert_eq!(
                    item_view::decode_character_inventory(&initial)
                        .map_err(|e| format!("{e:?}"))?
                        .main_backpack
                        .ok_or("backpack")?
                        .appearance_id,
                    0
                );
            }
            let mut updated = inventory.clone();
            let updated_item = updated.main_backpack.as_mut().ok_or("updated backpack")?;
            updated_item.appearance_id = 2854;
            updated_item.count = 2.try_into()?;
            let delta = encode(&updated)?;
            let (client, mut server) = tokio::io::duplex(256 * 1024);
            let peer = tokio::spawn(async move {
                join_peer_with(
                    &mut server,
                    advertised,
                    selected,
                    &[],
                    item_snapshot(vec![])?,
                    &[(9, 1, 1, initial)],
                )
                .await?;
                write_frame(&mut server, &encode_state_delta(1, 41, 9, 1, 2, 1, &delta)?).await?;
                let mut rest = Vec::new();
                server.read_to_end(&mut rest).await?;
                Ok::<(), BoxError>(())
            });
            let mut admission = item_admission()?;
            admission.supported_capabilities = advertised;
            let mut session = Session::admit(client, admission).await?;
            let expected = if appearances { 3031 } else { 0 };
            assert_eq!(
                session
                    .inventory()
                    .and_then(|view| view.main_backpack)
                    .ok_or("snapshot backpack")?
                    .appearance_id,
                expected
            );
            // Consume the delta through the ordinary session event route.
            session.take_events();
            let events = session
                .wait_for_events(std::time::Duration::from_secs(2))
                .await?;
            assert!(
                events
                    .iter()
                    .any(|event| matches!(event, SessionEvent::Inventory(_)))
            );
            let expected = if appearances { 2854 } else { 0 };
            assert_eq!(
                session
                    .inventory()
                    .and_then(|view| view.main_backpack)
                    .ok_or("delta backpack")?
                    .appearance_id,
                expected
            );
            assert_eq!(
                session
                    .inventory()
                    .and_then(|view| view.main_backpack)
                    .ok_or("delta count")?
                    .count
                    .get(),
                2
            );
            drop(session);
            peer.await??;
        }
        Ok(())
    })?
}
