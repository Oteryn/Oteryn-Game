use super::*;
use crate::item_movement::{MoveRequest, RefusalDetail};
use oteryn_protocol_oteryn::{item_view, refusal_detail};

#[test]
fn plain_item_moves_follow_negotiated_destinations() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                const SUPPORTED: &[u32] = &[4, 6, 11, 12, 14, 15, 18];
                let backpack = ItemMoveDestination::MainBackpack;
                let ground = ItemMoveDestination::Ground(item_view::WorldTilePosition {
                    x: 32369,
                    y: 32238,
                    floor: 7,
                });
                let equipment = ItemMoveDestination::Equipment(item_view::EquipmentSlot::Head);
                let container = ItemMoveDestination::Container(handle(10));
                let cases: [(&'static [u32], Vec<ItemMoveDestination>); 4] = [
                    (&[4, 6], vec![backpack]),
                    (&[4, 6, 11, 15, 18], vec![ground, backpack]),
                    (&[4, 6, 12], vec![ground, equipment, backpack]),
                    (
                        &[4, 6, 12, 14],
                        vec![ground, equipment, container, backpack],
                    ),
                ];
                for (selected, permitted) in cases {
                    let (client, mut server) = tokio::io::duplex(64 * 1024);
                    let expected = permitted.clone();
                    let peer = tokio::spawn(async move {
                        let mut domains =
                            vec![inventory_domain(1, &CharacterInventory::default())?];
                        if selected.contains(&14) {
                            domains.push((14, 1, 1, vec![]));
                        }
                        if selected.contains(&18) {
                            use oteryn_protocol_oteryn::world_map::{
                                MapViewHeader, WorldMapViewSnapshot, encode_world_map_snapshot,
                            };
                            let map = encode_world_map_snapshot(&WorldMapViewSnapshot {
                                header: MapViewHeader {
                                    content_generation: CONTENT_GENERATION,
                                    bundle_digest: [0x22; 32],
                                    reset_epoch: 0,
                                    origin: at(0, 0),
                                    first_visible_floor: 0,
                                },
                                tiles: vec![],
                            })
                            .map_err(|e| format!("{e:?}"))?;
                            domains.push((17, 0, 1, map));
                        }
                        join_peer_with(
                            &mut server,
                            SUPPORTED,
                            selected,
                            &[],
                            item_snapshot(vec![])?,
                            &domains,
                        )
                        .await?;
                        for (index, destination) in expected.into_iter().enumerate() {
                            let frame = read_frame(&mut server).await?;
                            let envelope = decode_wire_envelope(&frame)?;
                            let command = envelope.client_command(1)?;
                            assert_eq!(command.command_type, COMMAND_TYPE_ITEM_MOVE_INTENT);
                            // Refused destinations neither send a frame nor consume an ID.
                            assert_eq!(command.command_id, 7 + index as u64);
                            let intent = item_view::decode_item_move_intent_for(
                                command.payload,
                                item_view::ItemMoveSelection {
                                    equip_drop: selected.contains(&12),
                                    container_tree: selected.contains(&14),
                                    world_interaction: selected.contains(&11),
                                },
                            )
                            .map_err(|e| format!("{e:?}"))?;
                            assert_eq!(intent.source, handle(2));
                            assert_eq!(intent.destination, destination);
                            let payload =
                                item_view::encode_item_move_result(ItemMoveOutcome::Moved)
                                    .map_err(|e| format!("{e:?}"))?;
                            write_frame(
                                &mut server,
                                &encode_command_result(
                                    1,
                                    41 + index as u64,
                                    command.command_id,
                                    CommandStatus::Accepted,
                                    &payload,
                                )?,
                            )
                            .await?;
                        }
                        Ok::<(), BoxError>(())
                    });
                    let mut session = match Session::admit(client, admission_with(SUPPORTED)?).await
                    {
                        Ok(session) => session,
                        Err(error) => {
                            let peer_result = peer.await;
                            return Err(format!(
                                "move admission {selected:?}: {error}; peer={peer_result:?}"
                            )
                            .into());
                        }
                    };
                    for destination in [ground, equipment, container] {
                        if !permitted.contains(&destination) {
                            assert!(matches!(
                                session
                                    .move_item(&ItemMoveIntent {
                                        source: handle(2),
                                        destination,
                                    })
                                    .await,
                                Err(SessionError::ItemView(
                                    item_view::ItemViewWireError::Malformed
                                ))
                            ));
                        }
                    }
                    for destination in permitted {
                        let result = session
                            .move_item(&ItemMoveIntent {
                                source: handle(2),
                                destination,
                            })
                            .await?;
                        assert_eq!(result.status, CommandStatus::Accepted);
                        assert_eq!(result.outcome, ItemMoveOutcome::Moved);
                    }
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "plain item move session test panicked")?
        .map_err(BoxError::from)?
}

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
