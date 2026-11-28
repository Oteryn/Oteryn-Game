//! Native bundle admission and the typed depot exchange on the actual Session stream.
use super::*;
use oteryn_protocol_oteryn::depot::*;
use oteryn_protocol_oteryn::world_map::*;

#[test]
fn depot_client_admits_bundle_browses_and_waits_for_committed_inventory() -> Result<(), BoxError> {
    depot_exchange(false)
}

#[test]
fn depot_container_deposit_waits_for_container_delta_without_inventory_change()
-> Result<(), BoxError> {
    depot_exchange(true)
}

fn depot_exchange(container_only: bool) -> Result<(), BoxError> {
    block_on(async {
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let peer = tokio::spawn(async move {
            let result: Result<(), BoxError> = async {
                read_frame(&mut server).await?;
                write_frame(
                    &mut server,
                    &encode_server_accepted(&ServerAcceptedValue {
                        game_session_id: GameSessionId::decode(&uuid_v7(1))?,
                        world_id: WorldId::decode(&uuid_v7(2))?,
                        channel_id: ChannelId::decode(&uuid_v7(3))?,
                        connection_generation: 1,
                        current_server_sequence: 0,
                        next_command_id: 7,
                        schema_revision: 1,
                        selected_capabilities: if container_only { &[4, 6, 12, 14, 18, 37, 510] } else { &[4, 6, 18, 37] },
                    })?,
                )
                .await?;
                let header = MapViewHeader {
                    content_generation: CONTENT_GENERATION,
                    bundle_digest: [0x22; 32],
                    reset_epoch: 0,
                    origin: at(0, 0),
                    first_visible_floor: 0,
                };
                let map = encode_world_map_snapshot(&WorldMapViewSnapshot {
                    header,
                    tiles: vec![],
                })
                .map_err(|e| format!("{e:?}"))?;
                let spatial = item_snapshot(vec![])?.1;
                let inv = item_view::encode_character_inventory(&CharacterInventory {
                    main_backpack: Some(entry(1, 4, 1)),
                    entries: if container_only { vec![] } else { vec![entry(2, 5, 100)] },
                    equipment: vec![],
                })
                .map_err(|e| format!("{e:?}"))?;
                let container = item_view::encode_open_container(&OpenContainer::default())
                    .map_err(|e| format!("{e:?}"))?;
                let views = oteryn_protocol_oteryn::container_tree::encode_container_views(
                    &oteryn_protocol_oteryn::container_tree::ContainerViews { views: if container_only {
                        vec![oteryn_protocol_oteryn::container_tree::ContainerView { view_id: 0, container_handle: handle(1), parent_handle: None, capacity: 20, entries: vec![entry(2, 5, 100)], browse: None }]
                    } else { vec![] } }).map_err(|e| format!("{e:?}"))?;
                let mut domains = vec![
                    DomainSnapshot {
                        domain_id: 1,
                        revision: 5,
                        snapshot_type: 2,
                        payload: &spatial,
                    },
                    DomainSnapshot {
                        domain_id: 9,
                        revision: 2,
                        snapshot_type: 1,
                        payload: &inv,
                    },
                    DomainSnapshot {
                        domain_id: 11,
                        revision: 1,
                        snapshot_type: 1,
                        payload: &container,
                    },
                    DomainSnapshot {
                        domain_id: 17,
                        revision: 1,
                        snapshot_type: 1,
                        payload: &map,
                    },
                ];
                if container_only { domains.push(DomainSnapshot { domain_id: 14, revision: 1, snapshot_type: 1, payload: &views }); }
                for frame in encode_single_chunk_snapshot(1, 1, 40, &domains)? {
                    write_frame(&mut server, &frame).await?;
                }
                for (id, sequence, outcome) in [
                    (7, 41, DepotOutcome::Opened),
                    (8, 42, DepotOutcome::Moved),
                    (9, 45, DepotOutcome::Opened),
                ] {
                    let frame = read_frame(&mut server).await?;
                    let envelope = decode_wire_envelope(&frame)?;
                    let command = envelope.client_command(1)?;
                    assert_eq!(command.command_type, COMMAND_TYPE_DEPOT_INTENT);
                    assert_eq!(command.command_id, id);
                    // Manual Sort Mode's slot reaches the wire only with 510 (the container
                    // variant); otherwise the session drops it and the 37-only decoder accepts.
                    let (request, slot) = decode_depot_intent_at(command.payload, container_only)
                        .map_err(|e| format!("{e:?}"))?;
                    if id == 8 {
                        assert_eq!(request.action, DepotAction::Deposit(handle(2)));
                        assert_eq!(slot, container_only.then_some(4));
                    } else {
                        assert_eq!(slot, None);
                    }
                    let reply = encode_depot_reply(&DepotReply {
                        outcome,
                        town: Some(0),
                        box_number: 1,
                        page: 0,
                        total: 0,
                        entries: vec![],
                    })
                    .map_err(|e| format!("{e:?}"))?;
                    write_frame(
                        &mut server,
                        &encode_command_result(1, sequence, id, CommandStatus::Accepted, &reply)?,
                    )
                    .await?;
                    if id == 8 {
                        let inv = item_view::encode_character_inventory(&CharacterInventory {
                            main_backpack: Some(entry(1, 4, 1)),
                            entries: vec![],
                            equipment: vec![],
                        })
                        .map_err(|e| format!("{e:?}"))?;
                        let delta = if container_only {
                            let empty = oteryn_protocol_oteryn::container_tree::encode_container_views(&oteryn_protocol_oteryn::container_tree::ContainerViews::default()).map_err(|e| format!("{e:?}"))?;
                            encode_state_delta(1, 43, 14, 1, 2, 1, &empty)?
                        } else { encode_state_delta(1, 43, 9, 2, 3, 1, &inv)? };
                        write_frame(&mut server, &delta).await?;
                        let delta = encode_world_map_delta(
                            &header,
                            &WorldMapViewDelta {
                                header: MapViewHeader {
                                    origin: at(1, 0),
                                    ..header
                                },
                                tiles: vec![],
                                cleared: vec![],
                            },
                        )
                        .map_err(|e| format!("{e:?}"))?;
                        write_frame(
                            &mut server,
                            &encode_state_delta(1, 44, 17, 1, 2, 1, &delta)?,
                        )
                        .await?;
                    }
                }
                let mut remaining = Vec::new();
                server.read_to_end(&mut remaining).await?;
                Ok::<_, BoxError>(())
            }
            .await;
            if let Err(error) = &result {
                eprintln!("depot peer: {error}");
            }
            result
        });
        let mut admission = item_admission()?;
        admission.supported_capabilities = CLIENT_SUPPORTED_CAPABILITIES;
        let mut session = Session::admit(client, admission).await?;
        assert!(session.is_selected(18));
        let map = session.world_map_snapshot().ok_or("join map")?;
        assert_eq!(map.header.origin, at(0, 0));
        assert_eq!(map.header.bundle_digest, [0x22; 32]);
        assert!(map.tiles.is_empty());
        assert!(session.world_object_overlay().is_empty());
        let browse = DepotIntent {
            town: None,
            box_number: 1,
            page: 0,
            action: DepotAction::Browse,
        };
        assert_eq!(session.depot(browse).await?.ok_or("reply")?.town, Some(0));
        assert_eq!(
            session
                .depot_at(
                    DepotIntent {
                        town: Some(0),
                        action: DepotAction::Deposit(handle(2)),
                        ..browse
                    },
                    Some(4),
                )
                .await?
                .ok_or("reply")?
                .outcome,
            DepotOutcome::Moved
        );
        assert!(session.inventory().ok_or("inventory")?.entries.is_empty());
        assert_eq!(
            session.depot(browse).await?.ok_or("reply")?.outcome,
            DepotOutcome::Opened
        );
        assert!(
            session
                .take_events()
                .iter()
                .any(|event| matches!(event, SessionEvent::WorldMap(_)))
        );
        // The map path needed by depot access cannot accept a new reset binding as a delta.
        let binding = session.map_header.ok_or("map binding")?;
        let changed = MapViewHeader {
            reset_epoch: binding.reset_epoch + 1,
            origin: at(2, 0),
            ..binding
        };
        let payload = encode_world_map_delta(
            &changed,
            &WorldMapViewDelta {
                header: changed,
                tiles: vec![],
                cleared: vec![at(2, 0)],
            },
        )
        .map_err(|e| format!("{e:?}"))?;
        let wire = encode_state_delta(1, 46, 17, 2, 3, 1, &payload)?;
        assert!(session.apply_state_delta(46, &wire).is_err());
        assert_eq!(session.map_header, Some(binding));
        drop(session);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn depot_client_refuses_unselected_capability_without_sending() -> Result<(), BoxError> {
    block_on(async {
        let (mut session, _peer) = vitals_pair().await?;
        assert_eq!(
            session
                .depot(DepotIntent {
                    town: None,
                    box_number: 1,
                    page: 0,
                    action: DepotAction::Browse
                })
                .await?,
            None
        );
        assert_eq!(session.next_command_id(), 7);
        Ok(())
    })?
}

#[path = "depot_open_tests.rs"]
mod open_tests;

#[test]
fn depot_at_encodes_selected_town_and_preserves_legacy_bytes() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for negotiated in [true, false] {
                    let selected: &[u32] = if negotiated {
                        &[4, 6, CAPABILITY_DEPOT_V1, CAPABILITY_CROSS_TOWN_DEPOT_V1]
                    } else {
                        &[4, 6, CAPABILITY_DEPOT_V1]
                    };
                    let (client, mut server) = tokio::io::duplex(64 * 1024);
                    let peer = tokio::spawn(async move {
                        let inventory =
                            item_view::encode_character_inventory(&CharacterInventory::default())
                                .map_err(|error| format!("{error:?}"))?;
                        let container = item_view::encode_open_container(&OpenContainer::default())
                            .map_err(|error| format!("{error:?}"))?;
                        join_peer_with(
                            &mut server,
                            CLIENT_SUPPORTED_CAPABILITIES,
                            selected,
                            &[],
                            item_snapshot(vec![])?,
                            &[(9, 1, 1, inventory), (11, 1, 1, container)],
                        )
                        .await?;
                        let local = [8, 1, 16, 1, 24, 0, 40, 0];
                        let named = [8, 1, 16, 1, 24, 0, 40, 0, 32, 6];
                        let remote = [8, 1, 16, 1, 24, 0, 40, 0, 32, 6, 56, 1];
                        let expected = if negotiated {
                            remote.as_slice()
                        } else {
                            named.as_slice()
                        };
                        for (sequence, payload) in [(41, local.as_slice()), (42, expected)] {
                            let frame = read_frame(&mut server).await?;
                            let envelope = decode_wire_envelope(&frame)?;
                            let command = envelope.client_command(1)?;
                            assert_eq!(command.command_type, COMMAND_TYPE_DEPOT_INTENT);
                            assert_eq!(command.payload, payload);
                            let reply = encode_depot_reply(&DepotReply {
                                outcome: DepotOutcome::Opened,
                                town: Some(6),
                                box_number: 1,
                                page: 0,
                                total: 0,
                                entries: vec![],
                            })
                            .map_err(|error| format!("{error:?}"))?;
                            write_frame(
                                &mut server,
                                &encode_command_result(
                                    1,
                                    sequence,
                                    command.command_id,
                                    CommandStatus::Accepted,
                                    &reply,
                                )?,
                            )
                            .await?;
                        }
                        Ok::<(), BoxError>(())
                    });
                    let exchange = async {
                        let mut admission = item_admission()?;
                        admission.supported_capabilities = CLIENT_SUPPORTED_CAPABILITIES;
                        let mut session = Session::admit(client, admission).await?;
                        for town in [None, Some(6)] {
                            let reply = session
                                .depot_at(
                                    DepotIntent {
                                        town,
                                        box_number: 1,
                                        page: 0,
                                        action: DepotAction::Browse,
                                    },
                                    None,
                                )
                                .await?;
                            assert_eq!(
                                reply.map(|reply| reply.outcome),
                                Some(DepotOutcome::Opened)
                            );
                        }
                        Ok::<(), BoxError>(())
                    };
                    let result = exchange.await;
                    peer.await??;
                    result?;
                }
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "depot exchange thread panicked")?
}
