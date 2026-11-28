use super::*;
use oteryn_protocol_oteryn::{bed, item_look};

#[test]
fn acknowledged_world_use_replays_the_same_command() -> Result<(), BoxError> {
    block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let intent = [8, 1];
        let disposition = world_object::encode_use_result(UseDisposition::Committed)
            .map_err(|error| format!("USE result: {error:?}"))?;
        let retained = disposition.clone();
        let peer = tokio::spawn(async move {
            join_peer_with(&mut server, &[], &[], &[], (1, spatial(0)), &[]).await?;
            for sequence in 41..=43 {
                let bytes = read_frame(&mut server).await?;
                let frame = decode_wire_envelope(&bytes)?;
                let command = frame.client_command(1)?;
                assert_eq!(command.command_type, world_object::COMMAND_TYPE_USE_INTENT);
                assert_eq!(command.payload, intent);
                let status = if sequence > 41 && command.command_id == 7 {
                    CommandStatus::DuplicateReplay
                } else {
                    CommandStatus::Accepted
                };
                write_frame(
                    &mut server,
                    &encode_command_result(1, sequence, command.command_id, status, &retained)?,
                )
                .await?;
            }
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, admission_with(&[])?).await?;
        let original = session.world_context_use(&intent).await?;
        assert_eq!(original.command_id, 7);
        assert_eq!(original.status, CommandStatus::Accepted);

        let replay = session
            .replay_acknowledged_world_use(original.command_id, &intent)
            .await?;
        assert_eq!(replay.status, CommandStatus::DuplicateReplay);
        assert_eq!(replay.command_id, original.command_id);
        assert_eq!(replay.payload, disposition);
        assert_eq!(session.next_command_id(), 8);
        assert_eq!(session.last_server_sequence(), 42);

        let next = session.world_context_use(&intent).await?;
        assert_eq!(next.command_id, 8);
        assert_eq!(next.status, CommandStatus::Accepted);
        assert_eq!(next.server_sequence, 43);
        peer.await??;
        Ok(())
    })?
}

#[test]
fn world_context_existing_look_and_bed_commands_cross_the_session() -> Result<(), BoxError> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| block_on(async {
        for selected in [false, true] {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                use oteryn_protocol_oteryn::world_map::{MapViewHeader, WorldMapViewSnapshot, encode_world_map_snapshot};
                let raw = if selected {
                    let map = encode_world_map_snapshot(&WorldMapViewSnapshot {
                        header: MapViewHeader { content_generation: CONTENT_GENERATION, bundle_digest: [2; 32], reset_epoch: 0, origin: at(0, 0), first_visible_floor: 0 }, tiles: vec![],
                    }).map_err(|e| format!("{e:?}"))?;
                    let inventory = item_view::encode_character_inventory(&CharacterInventory::default()).map_err(|e| format!("{e:?}"))?;
                    let container = item_view::encode_open_container(&OpenContainer::default()).map_err(|e| format!("{e:?}"))?;
                    vec![(9, 0, 1, inventory), (11, 0, 1, container), (17, 0, 1, map)]
                } else { vec![] };
                join_peer_with(&mut server, CLIENT_SUPPORTED_CAPABILITIES, if selected { &[4, 6, 11, 15, 18, 28] } else { &[] }, &[], if selected { item_snapshot(vec![])? } else { (1, spatial(0)) }, &raw).await?;
                if selected {
                    for (id, kind) in [(7, 30), (8, 31)] {
                        let frame = read_frame(&mut server).await?;
                        let envelope = decode_wire_envelope(&frame)?;
                        let command = envelope.client_command(1)?;
                        assert_eq!((command.command_type, command.command_id), (kind, id));
                        let payload = if kind == 30 {
                            let intent = item_look::decode_item_look_intent(command.payload).map_err(|e| format!("{e:?}"))?;
                            assert_eq!(intent.target, item_look::ItemLookTarget::Item(std::num::NonZeroU64::MIN));
                            item_look::encode_item_look_result(&item_look::ItemLookResult::Text("You see a bed.".into())).map_err(|e| format!("{e:?}"))?
                        } else {
                            let intent = bed::decode_bed_sleep_intent(command.payload).map_err(|e| format!("{e:?}"))?;
                            assert_eq!((intent.target.get(), intent.skill), (1, 6));
                            world_object::encode_use_result(UseDisposition::BedOccupied).map_err(|e| format!("{e:?}"))?
                        };
                        write_frame(&mut server, &encode_command_result(1, 34 + id, id, CommandStatus::Accepted, &payload)?).await?;
                    }
                }
                Ok::<(), BoxError>(())
            });
            let mut admission = admission()?;
            admission.supported_capabilities = CLIENT_SUPPORTED_CAPABILITIES;
            let mut session = Session::admit(client, admission).await?;
            for (capability, kind, payload) in [(28, 30, vec![8, 1]), (11, 31, vec![10, 2, 8, 1, 16, 6])] {
                let result = session.gated_command(capability, kind, &payload).await;
                if selected { assert!(!result?.payload.is_empty()); }
                else { assert!(matches!(result, Err(SessionError::CapabilityNotSelected { capability: id }) if id == capability)); }
            }
            peer.await??;
        }
        Ok(())
    }))?.join().map_err(|_| "context session thread failed")??
}
