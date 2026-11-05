use super::*;
use oteryn_protocol_oteryn::{bed, item_look};

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
