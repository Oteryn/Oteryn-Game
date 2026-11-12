use super::*;
use crate::item_movement::{MoveRequest, RefusalDetail};
use oteryn_protocol_oteryn::{item_view, refusal_detail};

#[test]
fn counted_moves_use_live_selection_and_preserve_refusals_and_updates() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let selected = &[4, 6, 12, 14, 61, 66];
                let inventory = CharacterInventory {
                    main_backpack: Some(entry(1, 2854, 1)),
                    entries: vec![entry(2, 3031, 20)],
                    ..Default::default()
                };
                let encoded =
                    item_view::encode_character_inventory_with_equip_drop(&inventory, true)
                        .map_err(|e| format!("{e:?}"))?;
                let mut updated = inventory.clone();
                updated.entries[0].count = 13.try_into()?;
                let delta = item_view::encode_character_inventory_with_equip_drop(&updated, true)
                    .map_err(|e| format!("{e:?}"))?;
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        selected,
                        selected,
                        &[],
                        item_snapshot(vec![])?,
                        &[(9, 1, 1, encoded), (14, 1, 1, vec![])],
                    )
                    .await?;
                    for (i, destination) in [
                        ItemMoveDestination::MainBackpack,
                        ItemMoveDestination::Container(handle(10)),
                        ItemMoveDestination::Ground(item_view::WorldTilePosition {
                            x: 1,
                            y: 0,
                            floor: 0,
                        }),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let frame = read_frame(&mut server).await?;
                        let envelope = decode_wire_envelope(&frame)?;
                        let command = envelope.client_command(1)?;
                        assert_eq!(command.command_type, 9);
                        let (intent, count) = item_view::decode_item_move_intent_with_count_for(
                            command.payload,
                            item_view::ItemMoveSelection {
                                equip_drop: true,
                                container_tree: true,
                                world_interaction: false,
                            },
                        )
                        .map_err(|e| format!("{e:?}"))?;
                        assert_eq!(intent.source, handle(2));
                        assert_eq!(intent.destination, destination);
                        assert_eq!(count, Some(7));
                        let outcome = if i == 0 {
                            ItemMoveOutcome::Moved
                        } else {
                            ItemMoveOutcome::NoRoom
                        };
                        let mut payload = item_view::encode_item_move_result(outcome)
                            .map_err(|e| format!("{e:?}"))?;
                        refusal_detail::append_refusal_detail(
                            &mut payload,
                            selected,
                            if i == 0 {
                                None
                            } else {
                                Some(RefusalDetail::NotEnoughCapacity)
                            },
                        );
                        if i == 0 {
                            write_frame(
                                &mut server,
                                &encode_state_delta(1, 41, 9, 1, 2, 1, &delta)?,
                            )
                            .await?;
                        }
                        write_frame(
                            &mut server,
                            &encode_command_result(
                                1,
                                42 + i as u64,
                                command.command_id,
                                CommandStatus::Accepted,
                                &payload,
                            )?,
                        )
                        .await?;
                    }
                    Ok::<(), BoxError>(())
                });
                let mut admission = item_admission()?;
                admission.supported_capabilities = selected;
                let mut session = Session::admit(client, admission).await?;
                for destination in [
                    ItemMoveDestination::MainBackpack,
                    ItemMoveDestination::Container(handle(10)),
                    ItemMoveDestination::Ground(item_view::WorldTilePosition {
                        x: 1,
                        y: 0,
                        floor: 0,
                    }),
                ] {
                    let response = session
                        .move_item_counted(MoveRequest {
                            source: handle(2),
                            destination,
                            count: Some(7),
                        })
                        .await?;
                    if destination == ItemMoveDestination::MainBackpack {
                        assert_eq!(response.outcome, ItemMoveOutcome::Moved);
                    } else {
                        assert_eq!(response.detail, Some(RefusalDetail::NotEnoughCapacity));
                    }
                    // Only sequenced domain-9 data changes the projected count.
                    assert_eq!(session.inventory(), Some(&updated));
                }
                peer.await??;
                Ok::<(), BoxError>(())
            })
        })?
        .join()
        .map_err(|_| "move session thread failed")??
}

#[test]
fn empty_rejected_move_fails_closed() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let selected = &[4, 6, 12, 14, 61, 66];
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        selected,
                        selected,
                        &[],
                        item_snapshot(vec![])?,
                        &[(14, 1, 1, vec![])],
                    )
                    .await?;
                    let frame = read_frame(&mut server).await?;
                    let envelope = decode_wire_envelope(&frame)?;
                    let command = envelope.client_command(1)?;
                    write_frame(
                        &mut server,
                        &encode_command_result(
                            1,
                            41,
                            command.command_id,
                            CommandStatus::Rejected,
                            &[],
                        )?,
                    )
                    .await?;
                    Ok::<(), BoxError>(())
                });
                let mut admission = item_admission()?;
                admission.supported_capabilities = selected;
                let mut session = Session::admit(client, admission).await?;
                let request = MoveRequest {
                    source: handle(2),
                    destination: ItemMoveDestination::MainBackpack,
                    count: None,
                };
                assert!(matches!(
                    session.move_item_counted(request).await,
                    Err(SessionError::ItemView(_))
                ));
                assert!(matches!(
                    session.move_item_counted(request).await,
                    Err(SessionError::SessionUnusable)
                ));
                peer.await??;
                Ok::<(), BoxError>(())
            })
        })?
        .join()
        .map_err(|_| "move rejection thread failed")??
}
