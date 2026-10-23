//! Native bundle admission and the typed depot exchange on the actual Session stream.
use super::*;
use oteryn_protocol_oteryn::depot::*;
use oteryn_protocol_oteryn::world_map::*;

#[test]
fn depot_client_admits_bundle_browses_and_waits_for_committed_inventory() -> Result<(), BoxError> {
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
                        selected_capabilities: &[4, 6, 18, 37],
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
                    entries: vec![entry(2, 5, 100)],
                    equipment: vec![],
                })
                .map_err(|e| format!("{e:?}"))?;
                let container = item_view::encode_open_container(&OpenContainer::default())
                    .map_err(|e| format!("{e:?}"))?;
                let domains = [
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
                    let request =
                        decode_depot_intent(command.payload).map_err(|e| format!("{e:?}"))?;
                    if id == 8 {
                        assert_eq!(request.action, DepotAction::Deposit(handle(2)));
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
                        write_frame(&mut server, &encode_state_delta(1, 43, 9, 2, 3, 1, &inv)?)
                            .await?;
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
                .depot(DepotIntent {
                    town: Some(0),
                    action: DepotAction::Deposit(handle(2)),
                    ..browse
                })
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
