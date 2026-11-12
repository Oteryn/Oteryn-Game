use super::*;
fn inventory(count: u32) -> Result<Vec<u8>, Error> {
    let entry = |handle, item, count| -> Result<ItemEntry, Error> {
        Ok(ItemEntry {
            handle: std::num::NonZeroU64::new(handle).ok_or("handle")?,
            item_definition_ref: std::num::NonZeroU32::new(item).ok_or("item")?,
            count: std::num::NonZeroU32::new(count).ok_or("count")?,
            sub_type: 0,
        })
    };
    let mut entries = Vec::new();
    if count > 0 {
        entries.push(entry(55, 42, count.min(100))?);
    }
    if count > 100 {
        entries.push(entry(56, 42, count - 100)?);
    }
    Ok(item_view::encode_character_inventory(&CharacterInventory {
        main_backpack: Some(entry(54, 43, 1)?),
        entries,
        ..Default::default()
    })
    .map_err(|e| format!("inventory: {e:?}"))?)
}
fn window(revision: u64) -> Result<Vec<u8>, Error> {
    let mut offers = vec![
        NpcOffer {
            item_definition_ref: 42,
            side: NpcTradeSide::Buy,
            count: 1,
            unit_price: 17,
        },
        NpcOffer {
            item_definition_ref: 42,
            side: NpcTradeSide::Sell,
            count: 1,
            unit_price: 4,
        },
    ];
    if revision == 20 {
        offers.reverse();
    }
    Ok(npc_service::encode_npc_trade_window(&NpcTradeWindow {
        open: Some((npc().entity, revision)),
        offers,
    })
    .map_err(|e| format!("window: {e:?}"))?)
}
async fn delta(
    stream: &mut DuplexStream,
    seq: &mut u64,
    domain: u32,
    base: u64,
    payload: &[u8],
) -> Result<(), Error> {
    *seq += 1;
    write_frame(
        stream,
        &encode_state_delta(1, *seq, domain, base, base + 1, 1, payload)?,
    )
    .await?;
    Ok(())
}
async fn result(
    stream: &mut DuplexStream,
    seq: &mut u64,
    id: u64,
    payload: &[u8],
) -> Result<(), Error> {
    *seq += 1;
    write_frame(
        stream,
        &encode_command_result(1, *seq, id, CommandStatus::Accepted, payload)?,
    )
    .await?;
    Ok(())
}
async fn resources(
    stream: &mut DuplexStream,
    seq: &mut u64,
    gold: bool,
    query: u64,
) -> Result<(), Error> {
    if !gold {
        return Ok(());
    }
    use oteryn_protocol_oteryn::atelier::resources;
    let bytes = read_frame(stream).await?;
    let command = decode_wire_envelope(&bytes)?.client_command(1)?;
    assert_eq!(command.command_type, 484);
    assert!(command.payload.is_empty());
    result(
        stream,
        seq,
        command.command_id,
        &resources::encode_reply(&resources::ResourceBalances {
            carried_gold: 100 + query,
            bank_gold: 5000,
            ..Default::default()
        }),
    )
    .await
}
pub(super) async fn peer(
    mut stream: DuplexStream,
    finished: oneshot::Receiver<()>,
    gold: bool,
) -> Result<(), Error> {
    let bootstrap = read_frame(&mut stream).await?;
    assert!(
        decode_wire_envelope(&bootstrap)?
            .client_bootstrap()?
            .supported_capabilities
            .contains(&272)
    );
    write_frame(
        &mut stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&identity(1))?,
            world_id: WorldId::decode(&identity(2))?,
            channel_id: ChannelId::decode(&identity(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: if gold {
                &[3, 4, 6, 12, 13, 14, 81, 148, 272]
            } else {
                &[3, 4, 6, 12, 13, 14, 81]
            },
        })?,
    )
    .await?;
    let spatial = encode_world_spatial_entities_snapshot(&WorldSpatialEntitiesSnapshot {
        content_generation: [1; 32],
        actor_position: position(100),
        own_identity: [7; 16],
        entities: vec![actor(100), npc()],
    })
    .map_err(|e| format!("spatial: {e:?}"))?;
    let initial_inventory = inventory(7)?;
    for frame in encode_single_chunk_snapshot(
        1,
        1,
        40,
        &[
            DomainSnapshot {
                domain_id: 1,
                revision: 5,
                snapshot_type: 2,
                payload: &spatial,
            },
            DomainSnapshot {
                domain_id: 2,
                revision: 2,
                snapshot_type: 1,
                payload: &[],
            },
            DomainSnapshot {
                domain_id: 7,
                revision: 0,
                snapshot_type: 1,
                payload: &[],
            },
            DomainSnapshot {
                domain_id: 8,
                revision: 0,
                snapshot_type: 1,
                payload: &[],
            },
            DomainSnapshot {
                domain_id: 9,
                revision: 0,
                snapshot_type: 1,
                payload: &initial_inventory,
            },
            DomainSnapshot {
                domain_id: 11,
                revision: 0,
                snapshot_type: 1,
                payload: &[],
            },
            DomainSnapshot {
                domain_id: 14,
                revision: 0,
                snapshot_type: 1,
                payload: &[],
            },
        ],
    )? {
        write_frame(&mut stream, &frame).await?;
    }
    let mut seq = 40;
    for (base, word) in [(0, "hi"), (1, "trade")] {
        talk(&mut stream, &mut seq, word, base).await?;
    }
    delta(&mut stream, &mut seq, 8, 0, &window(10)?).await?;
    use oteryn_protocol_oteryn::container_tree::{self, ContainerView, ContainerViews};
    let views = container_tree::encode_container_views(&ContainerViews {
        views: vec![ContainerView {
            view_id: 0,
            container_handle: std::num::NonZeroU64::new(54).ok_or("backpack")?,
            parent_handle: None,
            capacity: 8,
            entries: vec![],
            browse: None,
        }],
    })
    .map_err(|e| format!("container: {e:?}"))?;
    delta(&mut stream, &mut seq, 14, 0, &views).await?;
    resources(&mut stream, &mut seq, gold, 0).await?;
    // No talk commands are allowed on success; the next command must be the next trade.
    let mut inventory_revision = 0;
    for (query, side, amount, count, disposition) in [
        (
            1,
            NpcTradeSide::Buy,
            100,
            Some(107),
            NpcIntentDisposition::Ok,
        ),
        (
            2,
            NpcTradeSide::Sell,
            100,
            Some(7),
            NpcIntentDisposition::Ok,
        ),
        (
            3,
            NpcTradeSide::Sell,
            3,
            None,
            NpcIntentDisposition::Rejected,
        ),
        (4, NpcTradeSide::Sell, 1, None, NpcIntentDisposition::Stale),
        (5, NpcTradeSide::Sell, 3, Some(4), NpcIntentDisposition::Ok),
    ] {
        let bytes = read_frame(&mut stream).await?;
        let command = decode_wire_envelope(&bytes)?.client_command(1)?;
        assert_eq!(command.command_type, 8);
        let intent = npc_service::decode_npc_trade_intent(command.payload)
            .map_err(|e| format!("trade: {e:?}"))?;
        assert_eq!(intent.npc_actor, npc().entity);
        assert_eq!(intent.catalogue_revision, if query == 5 { 20 } else { 10 });
        assert_eq!(
            intent.offer_index,
            if side == NpcTradeSide::Buy || query == 5 {
                0
            } else {
                1
            }
        );
        assert_eq!(intent.quantity, amount);
        assert_eq!(intent.side, side);
        result(
            &mut stream,
            &mut seq,
            command.command_id,
            &npc_service::encode_npc_intent_result(disposition),
        )
        .await?;
        if let Some(count) = count {
            delta(
                &mut stream,
                &mut seq,
                9,
                inventory_revision,
                &inventory(count)?,
            )
            .await?;
            inventory_revision += 1;
        }
        if disposition == NpcIntentDisposition::Stale {
            talk(&mut stream, &mut seq, "trade", 2).await?;
            delta(&mut stream, &mut seq, 8, 1, &window(20)?).await?;
        }
        resources(&mut stream, &mut seq, gold, query).await?;
    }
    let bytes = read_frame(&mut stream).await?;
    let command = decode_wire_envelope(&bytes)?.client_command(1)?;
    assert_eq!(command.command_type, 97);
    let intent = oteryn_protocol_oteryn::npc_shop::decode_npc_shop_intent(command.payload)
        .map_err(|e| format!("sell all: {e:?}"))?;
    assert_eq!(
        intent.action,
        oteryn_protocol_oteryn::npc_shop::NpcShopAction::SellAll
    );
    assert_eq!(intent.catalogue_revision, 20);
    assert_eq!(intent.offer_index, 0);
    result(
        &mut stream,
        &mut seq,
        command.command_id,
        &oteryn_protocol_oteryn::npc_shop::encode_npc_shop_result(
            &oteryn_protocol_oteryn::npc_shop::NpcShopResult {
                disposition: NpcIntentDisposition::Ok,
                look_text: String::new(),
                sold_units: 4,
                total_gold_units: 16,
            },
        )
        .map_err(|e| format!("sold: {e:?}"))?,
    )
    .await?;
    delta(&mut stream, &mut seq, 9, inventory_revision, &inventory(0)?).await?;
    resources(&mut stream, &mut seq, gold, 6).await?;
    finished.await?;
    stream.shutdown().await?;
    let mut extra = [0];
    assert_eq!(stream.read(&mut extra).await?, 0);
    Ok(())
}
async fn talk(
    stream: &mut DuplexStream,
    seq: &mut u64,
    text: &str,
    base: u64,
) -> Result<(), Error> {
    let bytes = read_frame(stream).await?;
    let command = decode_wire_envelope(&bytes)?.client_command(1)?;
    assert_eq!(command.command_type, 7);
    let intent =
        npc_service::decode_npc_talk_intent(command.payload).map_err(|e| format!("talk: {e:?}"))?;
    assert_eq!(intent.text, text);
    result(
        stream,
        seq,
        command.command_id,
        &npc_service::encode_npc_intent_result(NpcIntentDisposition::Ok),
    )
    .await?;
    let conversation = npc_service::encode_npc_conversation(&NpcConversation {
        open: Some((npc().entity, NpcConversationState::Open)),
        lines: vec!["Ask for {trade}.".into()],
    })
    .map_err(|e| format!("conversation: {e:?}"))?;
    delta(stream, seq, 7, base, &conversation).await
}
